use crate::workflow::{
    errors::{Result, SwissPipeError},
    models::{Node, Workflow, WorkflowEvent, InputMergeStrategy},
    input_sync::InputSyncService,
};
use std::{collections::{HashMap, HashSet}, sync::Arc};
use tokio::task::JoinSet;
use uuid::Uuid;

use super::node_executor::NodeExecutor;

pub struct DagExecutor {
    node_executor: Arc<NodeExecutor>,
    _input_sync_service: Arc<InputSyncService>,
}

struct ExecutionContext<'a> {
    predecessors: &'a HashMap<String, Vec<String>>,
    successors: &'a HashMap<String, Vec<(String, Option<bool>)>>,
    completed_nodes: &'a HashSet<String>,
    node_outputs: &'a HashMap<String, WorkflowEvent>,
    pending_executions: &'a mut JoinSet<Result<(String, WorkflowEvent)>>,
    execution_id: &'a str,
}

impl DagExecutor {
    pub fn new(
        node_executor: Arc<NodeExecutor>,
        input_sync_service: Arc<InputSyncService>,
    ) -> Self {
        Self {
            node_executor,
            _input_sync_service: input_sync_service,
        }
    }

    /// Execute workflow using DAG traversal matching the original implementation
    pub async fn execute_workflow(&self, workflow: &Workflow, event: WorkflowEvent) -> Result<WorkflowEvent> {
        let execution_id = Uuid::new_v4().to_string();
        tracing::info!("Starting DAG execution for workflow '{}' with execution_id '{}'", workflow.name, execution_id);

        // Build graph structures for DAG traversal
        let node_map: HashMap<String, &Node> = workflow.nodes
            .iter()
            .map(|node| (node.id.clone(), node))
            .collect();

        let predecessors = self.build_predecessor_map(workflow);
        let successors = self.build_successor_map(workflow);

        // Initialize execution state
        let mut completed_nodes: HashSet<String> = HashSet::new();
        let mut node_outputs: HashMap<String, WorkflowEvent> = HashMap::new();
        let mut pending_executions: JoinSet<Result<(String, WorkflowEvent)>> = JoinSet::new();

        // Start with the trigger node
        let start_node_id = workflow.start_node_id.as_ref()
            .ok_or_else(|| SwissPipeError::Config("Workflow missing start_node_id".to_string()))?;
        let start_node = node_map.get(start_node_id)
            .ok_or_else(|| SwissPipeError::NodeNotFound(start_node_id.clone()))?;

        // Execute trigger node first
        let trigger_result = self.execute_single_node(start_node, vec![event], &execution_id).await?;
        completed_nodes.insert(start_node_id.clone());
        node_outputs.insert(start_node_id.clone(), trigger_result.clone());

        tracing::info!("Completed trigger node '{}' (id: {}), looking for next nodes", start_node.name, start_node_id);

        // Schedule immediately ready nodes for concurrent execution
        let mut execution_context = ExecutionContext {
            predecessors: &predecessors,
            successors: &successors,
            completed_nodes: &completed_nodes,
            node_outputs: &node_outputs,
            pending_executions: &mut pending_executions,
            execution_id: &execution_id,
        };
        self.schedule_ready_nodes(workflow, &mut execution_context).await?;

        // Main execution loop for DAG traversal
        while !pending_executions.is_empty() {
            // Wait for any node to complete
            if let Some(result) = pending_executions.join_next().await {
                match result {
                    Ok(Ok((node_id, output))) => {
                        tracing::info!("Node '{}' completed successfully", node_id);
                        completed_nodes.insert(node_id.clone());
                        node_outputs.insert(node_id.clone(), output);

                        // Schedule any newly ready nodes
                        let mut execution_context = ExecutionContext {
                            predecessors: &predecessors,
                            successors: &successors,
                            completed_nodes: &completed_nodes,
                            node_outputs: &node_outputs,
                            pending_executions: &mut pending_executions,
                            execution_id: &execution_id,
                        };
                        self.schedule_ready_nodes(workflow, &mut execution_context).await?;
                    }
                    Ok(Err(e)) => {
                        // Node execution failed, cancel remaining tasks
                        pending_executions.abort_all();
                        return Err(e);
                    }
                    Err(join_error) => {
                        // Task join failed
                        pending_executions.abort_all();
                        return Err(SwissPipeError::Generic(format!("Task join error: {join_error}")));
                    }
                }
            }
        }

        tracing::info!("DAG execution completed successfully for execution_id '{}'", execution_id);

        // Return the final output (last completed node or aggregate)
        self.get_final_output(workflow, &completed_nodes, &node_outputs)
    }

    fn build_predecessor_map(&self, workflow: &Workflow) -> HashMap<String, Vec<String>> {
        let mut predecessors: HashMap<String, Vec<String>> = HashMap::new();

        for edge in &workflow.edges {
            let from_id = &edge.from_node_id;
            let to_id = &edge.to_node_id;
            predecessors
                .entry(to_id.clone())
                .or_default()
                .push(from_id.clone());
        }

        predecessors
    }

    fn build_successor_map(&self, workflow: &Workflow) -> HashMap<String, Vec<(String, Option<bool>)>> {
        let mut successors: HashMap<String, Vec<(String, Option<bool>)>> = HashMap::new();

        for edge in &workflow.edges {
            let from_id = &edge.from_node_id;
            let to_id = &edge.to_node_id;
            successors
                .entry(from_id.clone())
                .or_default()
                .push((to_id.clone(), edge.condition_result));
        }

        successors
    }

    async fn schedule_ready_nodes(
        &self,
        workflow: &Workflow,
        execution_context: &mut ExecutionContext<'_>,
    ) -> Result<()> {
        // Find all nodes that are ready to execute
        for node in &workflow.nodes {
            if execution_context.completed_nodes.contains(&node.id) {
                continue; // Already completed
            }

            // Check if all predecessors are completed
            let node_predecessors = execution_context.predecessors.get(&node.id).cloned().unwrap_or_default();
            let all_predecessors_ready = node_predecessors.iter().all(|pred| execution_context.completed_nodes.contains(pred));

            if all_predecessors_ready {
                tracing::info!("Node '{}' (id: {}) is ready for execution", node.name, node.id);

                // Collect inputs from predecessors
                let inputs = self.collect_node_inputs(
                    &node.id,
                    &node_predecessors,
                    execution_context.successors,
                    execution_context.node_outputs,
                    execution_context.completed_nodes,
                )?;

                // Clone necessary data for the async task
                let node_clone = node.clone();
                let execution_id = execution_context.execution_id.to_string();
                let node_executor = self.node_executor.clone();

                // Spawn async execution
                execution_context.pending_executions.spawn(async move {
                    let result = Self::execute_single_node_static(
                        node_executor,
                        &node_clone,
                        inputs,
                        &execution_id,
                    ).await;
                    match result {
                        Ok(event) => Ok((node_clone.id, event)),
                        Err(e) => Err(e),
                    }
                });
            }
        }

        Ok(())
    }

    fn collect_node_inputs(
        &self,
        node_id: &str,
        predecessors: &[String],
        successors: &HashMap<String, Vec<(String, Option<bool>)>>,
        node_outputs: &HashMap<String, WorkflowEvent>,
        _completed_nodes: &HashSet<String>,
    ) -> Result<Vec<WorkflowEvent>> {
        let mut inputs = Vec::new();

        for pred_id in predecessors {
            if let Some(pred_output) = node_outputs.get(pred_id) {
                // Check if this edge should be followed based on conditions
                if let Some(pred_successors) = successors.get(pred_id) {
                    for (succ_id, condition_result) in pred_successors {
                        if succ_id == node_id {
                            if let Some(expected_result) = condition_result {
                                // Check if condition matches - use node ID as key
                                let actual_result = pred_output.condition_results
                                    .get(pred_id)
                                    .copied()
                                    .unwrap_or(false);

                                tracing::debug!(
                                    "Condition edge: pred_id='{}', expected={}, actual={}, follow={}",
                                    pred_id, expected_result, actual_result, actual_result == *expected_result
                                );

                                if actual_result == *expected_result {
                                    inputs.push(pred_output.clone());
                                }
                            } else {
                                // Unconditional edge
                                inputs.push(pred_output.clone());
                            }
                        }
                    }
                }
            }
        }

        Ok(inputs)
    }

    /// Execute a single node with input merging
    async fn execute_single_node(
        &self,
        node: &Node,
        inputs: Vec<WorkflowEvent>,
        execution_id: &str,
    ) -> Result<WorkflowEvent> {
        let merged_input = self.merge_inputs(inputs, node)?;
        self.node_executor.execute_node(node, merged_input, execution_id).await
    }

    /// Static version for use in spawned tasks
    async fn execute_single_node_static(
        node_executor: Arc<NodeExecutor>,
        node: &Node,
        inputs: Vec<WorkflowEvent>,
        execution_id: &str,
    ) -> Result<WorkflowEvent> {
        let merged_input = Self::merge_inputs_static(inputs, node)?;
        node_executor.execute_node(node, merged_input, execution_id).await
    }

    /// Merge multiple inputs based on node's input merge strategy
    fn merge_inputs(&self, inputs: Vec<WorkflowEvent>, node: &Node) -> Result<WorkflowEvent> {
        Self::merge_inputs_static(inputs, node)
    }

    /// Static version of merge_inputs for use in spawned tasks
    fn merge_inputs_static(inputs: Vec<WorkflowEvent>, node: &Node) -> Result<WorkflowEvent> {
        tracing::info!("Executing node '{}' with {} inputs", node.name, inputs.len());

        // Merge inputs based on strategy
        let input_event = if inputs.len() <= 1 {
            inputs.into_iter().next().unwrap_or_else(|| WorkflowEvent {
                data: serde_json::Value::Object(serde_json::Map::new()),
                metadata: HashMap::new(),
                headers: HashMap::new(),
                condition_results: HashMap::new(),
            })
        } else {
            let merge_strategy = node.input_merge_strategy
                .as_ref()
                .unwrap_or(&InputMergeStrategy::WaitForAll);

            InputSyncService::merge_inputs(inputs, merge_strategy)?
        };

        Ok(input_event)
    }

    /// Determine the final output of the workflow - matches original logic
    fn get_final_output(
        &self,
        workflow: &Workflow,
        completed_nodes: &HashSet<String>,
        node_outputs: &HashMap<String, WorkflowEvent>,
    ) -> Result<WorkflowEvent> {
        // Find leaf nodes (nodes with no successors) - matches original logic
        let successors = self.build_successor_map(workflow);
        let mut leaf_nodes = Vec::new();

        for node in &workflow.nodes {
            if completed_nodes.contains(&node.id)
                && (!successors.contains_key(&node.id) || successors[&node.id].is_empty()) {
                    leaf_nodes.push(node.id.clone());
                }
        }

        if leaf_nodes.len() == 1 {
            // Single leaf node - return its output
            let leaf_id = &leaf_nodes[0];
            if let Some(output) = node_outputs.get(leaf_id) {
                Ok(output.clone())
            } else {
                Err(SwissPipeError::Generic(
                    format!("No output found for final node '{leaf_id}'")
                ))
            }
        } else if leaf_nodes.len() > 1 {
            // Multiple leaf nodes - merge their outputs
            let leaf_outputs: Vec<WorkflowEvent> = leaf_nodes.iter()
                .filter_map(|name| node_outputs.get(name).cloned())
                .collect();

            InputSyncService::merge_inputs(leaf_outputs, &InputMergeStrategy::WaitForAll)
        } else {
            // No leaf nodes found - this shouldn't happen in a valid DAG
            Err(SwissPipeError::Generic("No leaf nodes found in completed workflow".to_string()))
        }
    }
}