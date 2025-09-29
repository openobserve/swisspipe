use crate::workflow::{
    errors::{Result, SwissPipeError},
    models::{Node, Workflow, WorkflowEvent, InputMergeStrategy, NodeOutput, HilMultiPathResult, NodeType},
    input_sync::InputSyncService,
};
use std::{collections::{HashMap, HashSet}, sync::Arc};
use tokio::task::JoinSet;

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
    pending_executions: &'a mut JoinSet<Result<(String, NodeOutput)>>,
    execution_id: &'a str,
    executed_hil_handles: &'a HashMap<String, HashSet<String>>, // HIL node_id -> set of executed handles
    multipath_scheduled_nodes: &'a HashSet<String>, // Nodes already scheduled by MultiPath execution
    pending_nodes: &'a mut HashSet<String>, // Nodes currently pending execution
}

struct HilExecutionParams<'a> {
    workflow: &'a Workflow,
    hil_node_id: &'a str,
    hil_result: HilMultiPathResult,
    _completed_nodes: &'a mut HashSet<String>,
    _node_outputs: &'a mut HashMap<String, WorkflowEvent>,
    pending_executions: &'a mut JoinSet<Result<(String, NodeOutput)>>,
    execution_id: &'a str,
    executed_hil_handles: &'a mut HashMap<String, HashSet<String>>,
    multipath_scheduled_nodes: &'a mut HashSet<String>,
    pending_nodes: &'a mut HashSet<String>,
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
    pub async fn execute_workflow(&self, workflow: &Workflow, event: WorkflowEvent, execution_id: &str) -> Result<WorkflowEvent> {
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
        let mut pending_executions: JoinSet<Result<(String, NodeOutput)>> = JoinSet::new();
        let mut executed_hil_handles: HashMap<String, HashSet<String>> = HashMap::new(); // Track executed HIL handles
        let mut multipath_scheduled_nodes: HashSet<String> = HashSet::new(); // Track nodes scheduled by MultiPath execution
        let mut pending_nodes: HashSet<String> = HashSet::new(); // Track nodes currently pending execution

        // Start with the trigger node
        let start_node_id = workflow.start_node_id.as_ref()
            .ok_or_else(|| SwissPipeError::Config("Workflow missing start_node_id".to_string()))?;
        let start_node = node_map.get(start_node_id)
            .ok_or_else(|| SwissPipeError::NodeNotFound(start_node_id.clone()))?;

        // Execute trigger node first
        let trigger_output = self.execute_single_node(start_node, vec![event], execution_id).await?;
        let trigger_event = match trigger_output {
            NodeOutput::Continue(event) => event,
            NodeOutput::Complete => {
                tracing::info!("Trigger node completed workflow immediately");
                return self.get_final_output(workflow, &completed_nodes, &node_outputs);
            },
            NodeOutput::MultiPath(_) => {
                return Err(SwissPipeError::Generic("Trigger nodes cannot return MultiPath".to_string()));
            },
            NodeOutput::AsyncPending(event) => {
                tracing::info!("Trigger node returned AsyncPending - workflow execution will continue via job queue");
                return Ok(event); // Return the event, async processing will handle the rest
            }
        };
        completed_nodes.insert(start_node_id.clone());
        node_outputs.insert(start_node_id.clone(), trigger_event.clone());

        tracing::info!("Completed trigger node '{}' (id: {}), looking for next nodes", start_node.name, start_node_id);

        // Schedule immediately ready nodes for concurrent execution
        let mut execution_context = ExecutionContext {
            predecessors: &predecessors,
            successors: &successors,
            completed_nodes: &completed_nodes,
            node_outputs: &node_outputs,
            pending_executions: &mut pending_executions,
            execution_id,
            executed_hil_handles: &executed_hil_handles,
            multipath_scheduled_nodes: &multipath_scheduled_nodes,
            pending_nodes: &mut pending_nodes,
        };
        self.schedule_ready_nodes(workflow, &mut execution_context).await?;

        // Main execution loop for DAG traversal
        while !pending_executions.is_empty() {
            // Wait for any node to complete
            if let Some(result) = pending_executions.join_next().await {
                match result {
                    Ok(Ok((node_id, output))) => {
                        tracing::info!("Node '{}' completed successfully", node_id);

                        // Remove node from pending set since it's now completed
                        pending_nodes.remove(&node_id);
                        tracing::debug!("Removed node '{}' from pending set", node_id);

                        match output {
                            NodeOutput::Continue(event) => {
                                // Standard single-path continuation
                                completed_nodes.insert(node_id.clone());
                                node_outputs.insert(node_id.clone(), event);
                            },
                            NodeOutput::MultiPath(hil_result) => {
                                // HIL 3-path execution
                                tracing::info!("Processing HIL MultiPath result for node '{}'", node_id);

                                // Save hil_task_id before moving hil_result
                                let hil_task_id = hil_result.hil_task_id.clone();

                                let hil_params = HilExecutionParams {
                                    workflow,
                                    hil_node_id: &node_id,
                                    hil_result: *hil_result,
                                    _completed_nodes: &mut completed_nodes,
                                    _node_outputs: &mut node_outputs,
                                    pending_executions: &mut pending_executions,
                                    execution_id,
                                    executed_hil_handles: &mut executed_hil_handles,
                                    multipath_scheduled_nodes: &mut multipath_scheduled_nodes,
                                    pending_nodes: &mut pending_nodes,
                                };
                                self.handle_multipath_execution(hil_params).await?;

                                // After HIL MultiPath, check if workflow should be marked as blocked for human input
                                // If no non-HIL nodes are pending and notification path has been executed,
                                // the workflow is blocked waiting for human response
                                if self.is_workflow_blocked_for_human_input(workflow, &completed_nodes, &executed_hil_handles, &mut pending_executions).await? {
                                    tracing::info!("Workflow blocked for human input after HIL node '{}' - exiting execution loop", node_id);

                                    // Return a special event indicating the workflow is pending human input
                                    let blocking_event = WorkflowEvent {
                                        data: serde_json::json!({
                                            "status": "pending_human_input",
                                            "hil_node_id": node_id,
                                            "hil_task_id": hil_task_id,
                                            "message": "Workflow is blocked pending human response"
                                        }),
                                        metadata: HashMap::new(),
                                        headers: HashMap::new(),
                                        condition_results: HashMap::new(),
        hil_task: None,
                                    };
                                    return Ok(blocking_event);
                                }
                            },
                            NodeOutput::Complete => {
                                // Node completed workflow execution
                                completed_nodes.insert(node_id.clone());
                                tracing::info!("Node '{}' completed workflow execution", node_id);
                            },
                            NodeOutput::AsyncPending(event) => {
                                // Node requires async processing - will be handled by job queue
                                completed_nodes.insert(node_id.clone());
                                node_outputs.insert(node_id.clone(), event);
                                tracing::info!("Node '{}' returned AsyncPending - will be processed asynchronously", node_id);
                            }
                        }

                        // Schedule any newly ready nodes
                        let mut execution_context = ExecutionContext {
                            predecessors: &predecessors,
                            successors: &successors,
                            completed_nodes: &completed_nodes,
                            node_outputs: &node_outputs,
                            pending_executions: &mut pending_executions,
                            execution_id,
                            executed_hil_handles: &executed_hil_handles,
                            multipath_scheduled_nodes: &multipath_scheduled_nodes,
                            pending_nodes: &mut pending_nodes,
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

            // Skip nodes already scheduled by MultiPath execution to prevent duplicates
            if execution_context.multipath_scheduled_nodes.contains(&node.id) {
                tracing::debug!("Skipping node '{}' - already scheduled by MultiPath execution", node.name);
                continue;
            }

            // Skip nodes already pending execution to prevent duplicates
            if execution_context.pending_nodes.contains(&node.id) {
                tracing::debug!("Skipping node '{}' - already pending execution", node.name);
                continue;
            }

            // Check if all predecessors are completed and HIL handles are executed
            let node_predecessors = execution_context.predecessors.get(&node.id).cloned().unwrap_or_default();
            let all_predecessors_ready = self.check_predecessors_ready(
                workflow,
                &node.id,
                &node_predecessors,
                execution_context.completed_nodes,
                execution_context.executed_hil_handles,
            )?;

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

                // Only spawn if node has valid inputs after condition filtering
                if !inputs.is_empty() {
                    // Mark node as pending to prevent duplicate scheduling
                    execution_context.pending_nodes.insert(node.id.clone());
                    tracing::debug!("Added node '{}' to pending set", node.name);

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
                            Ok(output) => Ok((node_clone.id, output)),
                            Err(e) => Err(e),
                        }
                    });
                } else {
                    tracing::debug!("Skipping node '{}' - no valid inputs after condition filtering", node.name);
                }
            }
        }

        Ok(())
    }

    /// Handle HIL MultiPath execution with 3-handle routing
    async fn handle_multipath_execution(&self, params: HilExecutionParams<'_>) -> Result<()> {
        // Execute notification path immediately (blue handle)
        tracing::info!("Executing HIL notification path for node '{}'", params.hil_node_id);
        let notification_successors = self.find_handle_successors(params.workflow, params.hil_node_id, "notification")?;

        // IMMEDIATELY mark notification successors as scheduled by MultiPath to prevent duplicate scheduling
        for successor_id in &notification_successors {
            params.multipath_scheduled_nodes.insert(successor_id.clone());
            tracing::debug!("Marked node '{}' as scheduled by MultiPath execution", successor_id);
        }
        let notification_count = notification_successors.len();

        for successor_id in notification_successors {
            if let Some(successor_node) = params.workflow.nodes.iter().find(|n| n.id == successor_id) {
                // Mark node as pending to prevent duplicate scheduling
                params.pending_nodes.insert(successor_id.clone());
                tracing::debug!("Added HIL notification successor '{}' to pending set", successor_node.name);

                let node_clone = successor_node.clone();
                let event_clone = params.hil_result.notification_path.event.clone();
                let execution_id_clone = params.execution_id.to_string();
                let node_executor = self.node_executor.clone();

                tracing::info!("Scheduling notification path successor: {}", successor_node.name);
                params.pending_executions.spawn(async move {
                    let result = Self::execute_single_node_static(
                        node_executor,
                        &node_clone,
                        vec![event_clone],
                        &execution_id_clone,
                    ).await;
                    match result {
                        Ok(output) => Ok((node_clone.id, output)),
                        Err(e) => Err(e),
                    }
                });
            }
        }

        // Log approved and denied handle successors for debugging
        let approved_successors = self.find_handle_successors(params.workflow, params.hil_node_id, "approved")?;
        let denied_successors = self.find_handle_successors(params.workflow, params.hil_node_id, "denied")?;

        tracing::info!(
            "HIL node '{}' handle routing - notification: {} nodes, approved: {} nodes, denied: {} nodes",
            params.hil_node_id,
            notification_count,
            approved_successors.len(),
            denied_successors.len()
        );

        // Do NOT mark HIL node as completed - it should remain pending until human response
        // Only mark specific handle as executed in the tracking map
        tracing::debug!(
            "HIL node '{}' notification path executed but node remains pending until human response",
            params.hil_node_id
        );

        // Track that notification handle has been executed
        params.executed_hil_handles
            .entry(params.hil_node_id.to_string())
            .or_default()
            .insert("notification".to_string());

        tracing::info!(
            "HIL node '{}' notification handle executed. Approved/denied handles remain blocked pending human response.",
            params.hil_node_id
        );

        // Store pending execution context for approved/denied paths
        // These will be resumed when human responds via HIL service using the handle-specific routing
        tracing::info!(
            "HIL node '{}' blocked for human response. Task ID: {}, Node Execution ID: {}",
            params.hil_node_id,
            params.hil_result.hil_task_id,
            params.hil_result.node_execution_id
        );

        Ok(())
    }

    /// Find successor nodes for a specific handle
    fn find_handle_successors(
        &self,
        workflow: &Workflow,
        node_id: &str,
        handle_id: &str,
    ) -> Result<Vec<String>> {
        tracing::debug!("Finding successors for node '{}' with handle '{}'", node_id, handle_id);

        let mut successors = Vec::new();

        // Filter edges by source_handle_id
        for edge in &workflow.edges {
            if edge.from_node_id == node_id {
                // Check if this edge matches the requested handle
                let edge_handle = edge.source_handle_id.as_deref().unwrap_or("default");

                // Match handle-specific routing
                let matches_handle = match handle_id {
                    "notification" => edge_handle == "notification" || edge_handle == "default",
                    "approved" => edge_handle == "approved",
                    "denied" => edge_handle == "denied",
                    "default" => edge_handle == "default" || edge.source_handle_id.is_none(),
                    _ => edge_handle == handle_id,
                };

                if matches_handle {
                    tracing::debug!("Found edge from '{}' to '{}' for handle '{}'",
                        edge.from_node_id, edge.to_node_id, handle_id);
                    successors.push(edge.to_node_id.clone());
                }
            }
        }

        tracing::debug!("Found {} successors for node '{}' with handle '{}': {:?}",
            successors.len(), node_id, handle_id, successors);

        Ok(successors)
    }

    /// Check if all predecessors are ready, considering HIL handle routing
    fn check_predecessors_ready(
        &self,
        workflow: &Workflow,
        node_id: &str,
        predecessors: &[String],
        completed_nodes: &HashSet<String>,
        executed_hil_handles: &HashMap<String, HashSet<String>>,
    ) -> Result<bool> {
        for pred_id in predecessors {
            // Check if predecessor is completed
            if !completed_nodes.contains(pred_id) {
                tracing::debug!("Node '{}' not ready - predecessor '{}' not completed", node_id, pred_id);
                return Ok(false);
            }

            // Check if predecessor is a HIL node
            if let Some(pred_node) = workflow.nodes.iter().find(|n| n.id == *pred_id) {
                if matches!(pred_node.node_type, NodeType::HumanInLoop { .. }) {
                    // HIL predecessor - check specific handle routing
                    if let Some(required_handle) = self.get_required_handle(workflow, pred_id, node_id)? {
                        // Check if this specific handle has been executed
                        let hil_handles = executed_hil_handles.get(pred_id).cloned().unwrap_or_default();
                        if !hil_handles.contains(&required_handle) {
                            tracing::debug!(
                                "Node '{}' not ready - HIL predecessor '{}' handle '{}' not executed (executed: {:?})",
                                node_id, pred_id, required_handle, hil_handles
                            );
                            return Ok(false);
                        }
                        tracing::debug!(
                            "Node '{}' HIL predecessor '{}' handle '{}' is executed",
                            node_id, pred_id, required_handle
                        );
                    }
                }
            }
        }

        tracing::debug!("Node '{}' all predecessors ready", node_id);
        Ok(true)
    }

    /// Get the required handle for the edge from HIL predecessor to target node
    fn get_required_handle(
        &self,
        workflow: &Workflow,
        hil_node_id: &str,
        target_node_id: &str,
    ) -> Result<Option<String>> {
        // Find the edge from HIL node to target node
        for edge in &workflow.edges {
            if edge.from_node_id == hil_node_id && edge.to_node_id == target_node_id {
                let handle = edge.source_handle_id.as_deref().unwrap_or("notification");
                // Map default handle to notification for HIL nodes
                let mapped_handle = match handle {
                    "default" => "notification",
                    other => other,
                };
                tracing::debug!("Found edge from HIL '{}' to '{}' with handle '{}'",
                    hil_node_id, target_node_id, mapped_handle);
                return Ok(Some(mapped_handle.to_string()));
            }
        }

        // No edge found - shouldn't happen if predecessor relationship exists
        tracing::warn!("No edge found from HIL node '{}' to '{}'", hil_node_id, target_node_id);
        Ok(None)
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
                                    "Condition edge: pred_id='{}', expected={}, actual={}, follow={}, condition_results={:?}",
                                    pred_id, expected_result, actual_result, actual_result == *expected_result, pred_output.condition_results
                                );

                                if actual_result == *expected_result {
                                    inputs.push(pred_output.clone());
                                }
                            } else {
                                // Unconditional edge (no condition_result or source_handle_id specific routing)
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
    ) -> Result<NodeOutput> {
        let merged_input = self.merge_inputs(inputs, node)?;
        self.node_executor.execute_node_with_output(node, merged_input, execution_id).await
    }

    /// Static version for use in spawned tasks
    async fn execute_single_node_static(
        node_executor: Arc<NodeExecutor>,
        node: &Node,
        inputs: Vec<WorkflowEvent>,
        execution_id: &str,
    ) -> Result<NodeOutput> {
        let merged_input = Self::merge_inputs_static(inputs, node)?;
        node_executor.execute_node_with_output(node, merged_input, execution_id).await
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
        hil_task: None,
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
            if completed_nodes.contains(&node.id) {
                // Check if this is a leaf node
                let has_successors = successors.contains_key(&node.id) && !successors[&node.id].is_empty();

                if !has_successors {
                    // Simple leaf node with no successors
                    leaf_nodes.push(node.id.clone());
                } else {
                    // Node has successors - check if all successor paths lead to incomplete/HIL nodes
                    let all_successors_blocked = successors[&node.id]
                        .iter()
                        .all(|(successor_id, _)| {
                            // Check if successor is not completed (meaning this path is blocked)
                            !completed_nodes.contains(successor_id) ||
                            // Or if successor is a HIL node (which may be blocking the path)
                            workflow.nodes.iter()
                                .find(|n| n.id == *successor_id)
                                .map(|n| matches!(n.node_type, NodeType::HumanInLoop { .. }))
                                .unwrap_or(false)
                        });

                    if all_successors_blocked {
                        // This node is effectively a leaf because all its paths are blocked
                        leaf_nodes.push(node.id.clone());
                    }
                }
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

    /// Check if the workflow is blocked for human input after HIL execution
    /// Returns true if:
    /// 1. HIL notification path has been executed
    /// 2. All remaining runnable nodes have been completed or scheduled
    /// 3. The only blocked nodes are HIL approved/denied paths waiting for human response
    async fn is_workflow_blocked_for_human_input(
        &self,
        workflow: &Workflow,
        completed_nodes: &HashSet<String>,
        executed_hil_handles: &HashMap<String, HashSet<String>>,
        pending_executions: &mut tokio::task::JoinSet<Result<(String, NodeOutput)>>,
    ) -> Result<bool> {
        // Wait for any notification path executions to complete first
        // This ensures we don't return blocked status while notification is still running
        while !pending_executions.is_empty() {
            if let Some(result) = pending_executions.join_next().await {
                match result {
                    Ok(Ok((node_id, output))) => {
                        tracing::debug!("Notification path node '{}' completed during blocking check", node_id);

                        match output {
                            NodeOutput::Continue(_) => {
                                // Notification node completed - this is expected
                            },
                            NodeOutput::Complete => {
                                tracing::debug!("Notification node '{}' marked workflow complete", node_id);
                            },
                            _ => {
                                tracing::warn!("Unexpected output type from notification node '{}': {:?}", node_id, output);
                            }
                        }
                    },
                    Ok(Err(e)) => {
                        tracing::error!("Notification path execution failed: {}", e);
                        return Err(e);
                    },
                    Err(join_error) => {
                        tracing::error!("Notification path task join failed: {}", join_error);
                        return Err(SwissPipeError::Generic(format!("Task join error: {join_error}")));
                    }
                }
            } else {
                break;
            }
        }

        // Now check if we have HIL nodes with only notification executed
        let mut has_blocking_hil = false;

        for node in &workflow.nodes {
            if let NodeType::HumanInLoop { .. } = node.node_type {
                if let Some(executed_handles) = executed_hil_handles.get(&node.id) {
                    // Check if notification was executed but approved/denied are still pending
                    if executed_handles.contains("notification") &&
                       !executed_handles.contains("approved") &&
                       !executed_handles.contains("denied") {
                        has_blocking_hil = true;
                        tracing::debug!(
                            "HIL node '{}' is blocking - notification executed, approved/denied pending",
                            node.name
                        );
                    }
                }
            }
        }

        // If we have blocking HIL nodes, check if any other nodes can still run
        if has_blocking_hil {
            // Count nodes that could potentially still execute (not completed, not HIL blocked)
            let mut runnable_nodes = 0;
            let predecessors = self.build_predecessor_map(workflow);

            for node in &workflow.nodes {
                if !completed_nodes.contains(&node.id) {
                    // Skip HIL nodes - they're handled separately
                    if matches!(node.node_type, NodeType::HumanInLoop { .. }) {
                        continue;
                    }

                    // Check if this node's predecessors are ready
                    let node_predecessors = predecessors.get(&node.id).cloned().unwrap_or_default();
                    if self.check_predecessors_ready(
                        workflow,
                        &node.id,
                        &node_predecessors,
                        completed_nodes,
                        executed_hil_handles,
                    )? {
                        runnable_nodes += 1;
                        tracing::debug!("Node '{}' is still runnable", node.name);
                    }
                }
            }

            if runnable_nodes == 0 {
                tracing::info!(
                    "Workflow is blocked for human input - {} HIL nodes pending response, no other runnable nodes",
                    workflow.nodes.iter()
                        .filter(|n| matches!(n.node_type, NodeType::HumanInLoop { .. }))
                        .count()
                );
                return Ok(true);
            } else {
                tracing::debug!(
                    "Workflow not blocked - {} nodes still runnable despite HIL blocking",
                    runnable_nodes
                );
            }
        }

        Ok(false)
    }
}