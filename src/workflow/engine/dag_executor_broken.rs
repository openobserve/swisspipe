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

    /// Execute workflow using DAG traversal
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

        // Schedule nodes that are ready to execute
        self.schedule_ready_nodes(
            workflow,
            &node_map,
            &predecessors,
            &completed_nodes,
            &node_outputs,
            &mut pending_executions,
            &execution_id,
        ).await;

        // Process nodes as they complete
        while let Some(result) = pending_executions.join_next().await {
            match result.map_err(|e| SwissPipeError::Generic(format!("Task join error: {e}")))? {
                Ok((node_id, event_output)) => {
                    completed_nodes.insert(node_id.clone());
                    node_outputs.insert(node_id.clone(), event_output);
                    tracing::debug!("Node '{}' completed successfully", node_id);

                    // Schedule any newly ready nodes
                    self.schedule_ready_nodes(
                        workflow,
                        &node_map,
                        &predecessors,
                        &completed_nodes,
                        &node_outputs,
                        &mut pending_executions,
                        &execution_id,
                    ).await;
                }
                Err(e) => {
                    tracing::error!("Node failed: {}", e);
                    // Cancel remaining tasks and return error
                    pending_executions.abort_all();
                    return Err(e);
                }
            }
        }

        // Get final output from the last completed node
        self.get_final_output(workflow, &node_outputs, &successors)
    }

    /// Build predecessor map for efficient DAG traversal
    fn build_predecessor_map(&self, workflow: &Workflow) -> HashMap<String, Vec<String>> {
        let mut predecessors: HashMap<String, Vec<String>> = HashMap::new();

        for edge in &workflow.edges {
            predecessors
                .entry(edge.to_node_id.clone())
                .or_insert_with(Vec::new)
                .push(edge.from_node_id.clone());
        }

        predecessors
    }

    /// Build successor map for efficient DAG traversal
    fn build_successor_map(&self, workflow: &Workflow) -> HashMap<String, Vec<(String, Option<bool>)>> {
        let mut successors: HashMap<String, Vec<(String, Option<bool>)>> = HashMap::new();

        for edge in &workflow.edges {
            successors
                .entry(edge.from_node_id.clone())
                .or_insert_with(Vec::new)
                .push((edge.to_node_id.clone(), edge.condition_result));
        }

        successors
    }

    /// Schedule nodes that are ready to execute
    async fn schedule_ready_nodes(
        &self,
        workflow: &Workflow,
        _node_map: &HashMap<String, &Node>,
        predecessors: &HashMap<String, Vec<String>>,
        completed_nodes: &HashSet<String>,
        node_outputs: &HashMap<String, WorkflowEvent>,
        pending_executions: &mut JoinSet<Result<(String, WorkflowEvent)>>,
        execution_id: &str,
    ) {
        for node in &workflow.nodes {
            let node_id = &node.id;

            // Skip if already completed or already scheduled
            if completed_nodes.contains(node_id) || self.is_node_scheduled(pending_executions, node_id) {
                continue;
            }

            // Check if all predecessors are completed
            let node_predecessors = predecessors.get(node_id).cloned().unwrap_or_default();
            if node_predecessors.iter().all(|pred_id| completed_nodes.contains(pred_id)) {
                // All predecessors completed, check if we should execute based on conditions
                if self.should_execute_node(workflow, node, node_outputs) {
                    let inputs = self.collect_node_inputs(node, &node_predecessors, node_outputs, workflow);

                    // Clone necessary data for the spawned task
                    let node_clone = node.clone();
                    let inputs_clone = inputs;
                    let execution_id_clone = execution_id.to_string();
                    let node_executor = self.node_executor.clone();

                    pending_executions.spawn(async move {
                        let result = Self::execute_single_node_static(
                            node_executor,
                            &node_clone,
                            inputs_clone,
                            &execution_id_clone,
                        ).await;
                        result.map(|event| (node_clone.id, event))
                    });

                    tracing::debug!("Scheduled node '{}' for execution", node_id);
                }
            }
        }
    }

    /// Check if node is already scheduled in pending executions
    fn is_node_scheduled(&self, _pending_executions: &JoinSet<Result<(String, WorkflowEvent)>>, _node_id: &str) -> bool {
        // Since JoinSet doesn't provide direct access to check contents,
        // we rely on the caller to not double-schedule
        // This is a simplified implementation - in production you might want
        // to track scheduled nodes separately
        false // Simplified for now
    }

    /// Determine if a node should execute based on conditional edges
    fn should_execute_node(
        &self,
        workflow: &Workflow,
        node: &Node,
        node_outputs: &HashMap<String, WorkflowEvent>,
    ) -> bool {
        // Check if any incoming edge has a condition that blocks execution
        for edge in &workflow.edges {
            if edge.to_node_id == node.id {
                if let Some(expected_result) = edge.condition_result {
                    // This is a conditional edge - check if condition passes
                    if let Some(from_output) = node_outputs.get(&edge.from_node_id) {
                        let actual_result = from_output.condition_results
                            .get(&edge.from_node_id)
                            .copied()
                            .unwrap_or(false);

                        if actual_result != expected_result {
                            tracing::debug!(
                                "Node '{}' blocked by conditional edge: expected={}, actual={}",
                                node.id, expected_result, actual_result
                            );
                            return false;
                        }
                    }
                }
            }
        }

        true
    }

    /// Collect inputs for a node from its predecessors
    fn collect_node_inputs(
        &self,
        node: &Node,
        predecessors: &[String],
        node_outputs: &HashMap<String, WorkflowEvent>,
        workflow: &Workflow,
    ) -> Vec<WorkflowEvent> {
        let mut inputs = Vec::new();

        if predecessors.is_empty() {
            // No predecessors - this shouldn't happen for non-trigger nodes
            tracing::warn!("Node '{}' has no predecessors", node.id);
            return inputs;
        }

        // Collect outputs from all predecessors that have valid edges
        for pred_id in predecessors {
            // Check if there's a valid edge from this predecessor
            let has_valid_edge = workflow.edges.iter().any(|edge| {
                edge.from_node_id == *pred_id && edge.to_node_id == node.id && {
                    if let Some(expected) = edge.condition_result {
                        // Conditional edge - check if condition passes
                        if let Some(pred_output) = node_outputs.get(pred_id) {
                            let actual = pred_output.condition_results
                                .get(pred_id)
                                .copied()
                                .unwrap_or(false);
                            actual == expected
                        } else {
                            false
                        }
                    } else {
                        // Unconditional edge
                        true
                    }
                }
            });

            if has_valid_edge {
                if let Some(pred_output) = node_outputs.get(pred_id) {
                    inputs.push(pred_output.clone());
                }
            }
        }

        inputs
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
        if inputs.is_empty() {
            return Err(SwissPipeError::Generic("No inputs to merge".to_string()));
        }

        if inputs.len() == 1 {
            return Ok(inputs.into_iter().next().unwrap());
        }

        // Use node's input merge strategy, defaulting to FirstWins
        let strategy = node.input_merge_strategy
            .as_ref()
            .unwrap_or(&InputMergeStrategy::FirstWins);

        match strategy {
            InputMergeStrategy::FirstWins => Ok(inputs[0].clone()),
            InputMergeStrategy::WaitForAll => {
                // Merge all inputs into an array structure
                let mut merged_metadata = std::collections::HashMap::new();
                let mut merged_headers = std::collections::HashMap::new();
                let mut merged_condition_results = std::collections::HashMap::new();

                // Create array of input data values
                let input_data_array: Vec<serde_json::Value> = inputs.iter()
                    .map(|input| input.data.clone())
                    .collect();

                // Merge metadata, headers, and condition results with input prefix
                for (index, input) in inputs.iter().enumerate() {
                    let input_key = format!("input_{index}");

                    for (key, value) in &input.metadata {
                        merged_metadata.insert(format!("{input_key}_{key}"), value.clone());
                    }

                    for (key, value) in &input.headers {
                        merged_headers.insert(format!("{input_key}_{key}"), value.clone());
                    }

                    for (key, value) in &input.condition_results {
                        merged_condition_results.insert(format!("{input_key}_{key}"), *value);
                    }
                }

                Ok(WorkflowEvent {
                    data: serde_json::Value::Array(input_data_array),
                    metadata: merged_metadata,
                    headers: merged_headers,
                    condition_results: merged_condition_results,
                })
            }
            InputMergeStrategy::TimeoutBased(_) => {
                // For now, treat timeout-based like WaitForAll
                // In a full implementation, this would involve async coordination
                Self::merge_inputs_static(inputs, &Node {
                    input_merge_strategy: Some(InputMergeStrategy::WaitForAll),
                    ..node.clone()
                })
            }
        }
    }

    /// Determine the final output of the workflow
    fn get_final_output(
        &self,
        workflow: &Workflow,
        node_outputs: &HashMap<String, WorkflowEvent>,
        successors: &HashMap<String, Vec<(String, Option<bool>)>>,
    ) -> Result<WorkflowEvent> {
        // Find nodes with no successors (leaf nodes)
        let leaf_nodes: Vec<&Node> = workflow.nodes
            .iter()
            .filter(|node| {
                !successors.contains_key(&node.id) ||
                successors.get(&node.id).unwrap().is_empty()
            })
            .collect();

        if leaf_nodes.is_empty() {
            return Err(SwissPipeError::Generic("No leaf nodes found in workflow".to_string()));
        }

        // Use the output from the first leaf node
        let final_node = leaf_nodes[0];
        node_outputs.get(&final_node.id)
            .cloned()
            .ok_or_else(|| SwissPipeError::Generic(
                format!("No output found for final node '{}'", final_node.id)
            ))
    }
}