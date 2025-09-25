// Workflow execution engine for WorkerPool
// Handles step-by-step workflow execution with tracking and resumption

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::async_execution::{ExecutionService, DelayScheduler, HttpLoopScheduler, input_coordination::InputCoordination};
use crate::database::workflow_execution_steps::StepStatus;
use crate::workflow::{
    engine::WorkflowEngine,
    errors::{Result, SwissPipeError},
    models::{Workflow, WorkflowEvent, Node},
    input_sync::InputSyncService,
};

use super::node_executor::NodeExecutor;

/// Handles workflow execution with step-by-step tracking and resumption
pub struct WorkflowExecutor {
    execution_service: Arc<ExecutionService>,
    workflow_engine: Arc<WorkflowEngine>,
    input_sync_service: Arc<InputSyncService>,
    node_executor: NodeExecutor,
    http_loop_scheduler: Arc<RwLock<Option<Arc<HttpLoopScheduler>>>>,
}

impl InputCoordination for WorkflowExecutor {
    fn get_input_sync_service(&self) -> &Arc<InputSyncService> {
        &self.input_sync_service
    }
}

impl WorkflowExecutor {
    pub fn new(
        execution_service: Arc<ExecutionService>,
        workflow_engine: Arc<WorkflowEngine>,
        input_sync_service: Arc<InputSyncService>,
        delay_scheduler: Arc<RwLock<Option<Arc<DelayScheduler>>>>,
    ) -> Self {
        // For now, create without HTTP loop scheduler - will be updated when available
        let node_executor = NodeExecutor::new(workflow_engine.clone(), delay_scheduler);
        let http_loop_scheduler = Arc::new(RwLock::new(None));

        Self {
            execution_service,
            workflow_engine,
            input_sync_service,
            node_executor,
            http_loop_scheduler,
        }
    }

    /// Set the HTTP loop scheduler for this workflow executor
    pub async fn set_http_loop_scheduler(&self, scheduler: Arc<HttpLoopScheduler>) {
        *self.http_loop_scheduler.write().await = Some(scheduler);
    }

    /// Create a NodeExecutor with the current HTTP loop scheduler
    async fn create_node_executor(&self) -> NodeExecutor {
        let http_loop_scheduler = self.http_loop_scheduler.read().await;
        match http_loop_scheduler.as_ref() {
            Some(scheduler) => {
                NodeExecutor::new_with_http_loop_scheduler(
                    self.workflow_engine.clone(),
                    self.node_executor.delay_scheduler.clone(),
                    scheduler.clone()
                )
            }
            None => {
                NodeExecutor::new(
                    self.workflow_engine.clone(),
                    self.node_executor.delay_scheduler.clone()
                )
            }
        }
    }

    /// Execute workflow with step-by-step tracking
    /// Supports resumption from a specific node if execution.current_node_id is set
    pub async fn execute_workflow_with_tracking(
        &self,
        execution_id: &str,
        workflow: &Workflow,
        event: WorkflowEvent,
    ) -> Result<WorkflowEvent> {
        let mut current_event = event;

        // Check if we're resuming from a specific node
        let execution = self.execution_service
            .get_execution(execution_id)
            .await?
            .ok_or_else(|| SwissPipeError::WorkflowNotFound(execution_id.to_string()))?;

        let is_resuming = execution.current_node_id.is_some();
        let mut current_node_id = execution.current_node_id
            .unwrap_or_else(|| workflow.start_node_id.clone().unwrap_or_default());

        // If resuming from a specific node, log it
        if is_resuming {
            tracing::info!("Resuming execution {} from node '{}'", execution_id, current_node_id);
        } else {
            tracing::debug!("Starting execution {} from beginning at node '{}'", execution_id, current_node_id);
        }

        let mut visited = HashSet::new();

        // Get existing completed steps to avoid re-executing them
        let steps = self.execution_service.get_execution_steps(execution_id).await?;
        let completed_steps: HashMap<String, _> = steps
            .into_iter()
            .filter(|step| matches!(step.status.as_str(), "completed" | "skipped" | "cancelled"))
            .map(|step| (step.node_id.clone(), step))
            .collect();

        // Build node lookup for efficiency
        let node_map: HashMap<String, &Node> = workflow.nodes
            .iter()
            .map(|node| (node.id.clone(), node))
            .collect();

        loop {
            // Prevent infinite loops
            if visited.contains(&current_node_id) {
                return Err(SwissPipeError::CycleDetected);
            }
            visited.insert(current_node_id.clone());

            let node = node_map
                .get(&current_node_id)
                .ok_or_else(|| SwissPipeError::NodeNotFound(current_node_id.clone()))?;

            // Check if this step was already completed (resumption case)
            if let Some(completed_step) = completed_steps.get(&current_node_id) {
                tracing::debug!("Skipping already completed step '{}' for execution {}", current_node_id, execution_id);

                // Use the output data from the completed step as the current event
                if let Some(output_data_str) = &completed_step.output_data {
                    if let Ok(output_value) = serde_json::from_str::<WorkflowEvent>(output_data_str) {
                        current_event = output_value;
                    } else {
                        tracing::warn!("Failed to parse output data for completed step '{}', using current event", current_node_id);
                    }
                }
            } else {
                // Check if this node requires input coordination
                let (ready_to_execute, coordinated_event) = self.coordinate_node_inputs(
                    workflow,
                    execution_id,
                    &current_node_id,
                    &current_event,
                    node.input_merge_strategy.as_ref(),
                ).await?;

                if !ready_to_execute {
                    break;
                }

                current_event = coordinated_event;

                // Create and execute the step as normal
                let input_data = serde_json::to_value(&current_event).ok();
                let step_id = self.execution_service
                    .create_execution_step(
                        execution_id.to_string(),
                        node.id.clone(),
                        node.name.clone(),
                        input_data,
                    )
                    .await?;

                // Mark step as running
                self.execution_service
                    .update_execution_step(&step_id, StepStatus::Running, None, None)
                    .await?;

                // Execute the node using updated node executor with HTTP loop scheduler
                let node_executor = self.create_node_executor().await;
                match node_executor.execute_node(execution_id, workflow, node, current_event.clone()).await {
                    Ok(result_event) => {
                        // Mark step as completed
                        let output_data = serde_json::to_value(&result_event).ok();
                        self.execution_service
                            .update_execution_step(&step_id, StepStatus::Completed, output_data, None)
                            .await?;

                        // Log the input → output transformation for debugging
                        tracing::debug!(
                            "Node '{}' transformed input → output: input_data_size={}, output_data_size={}",
                            current_node_id,
                            serde_json::to_string(&current_event).map(|s| s.len()).unwrap_or(0),
                            serde_json::to_string(&result_event).map(|s| s.len()).unwrap_or(0)
                        );

                        // Ensure the output carries forward all necessary context
                        current_event = result_event;
                    }
                    Err(SwissPipeError::DelayScheduled(delay_id)) => {
                        // DelayScheduled - keep step as running during delay period
                        // Step will be marked completed when delay finishes and workflow resumes
                        tracing::info!("Delay scheduled with ID: {} for step {}", delay_id, step_id);
                        return Err(SwissPipeError::DelayScheduled(delay_id));
                    }
                    Err(e) => {
                        // Mark step as failed
                        self.execution_service
                            .update_execution_step(&step_id, StepStatus::Failed, None, Some(e.to_string()))
                            .await?;
                        return Err(e);
                    }
                }
            }

            // Get next nodes using the workflow engine's logic (use existing node executor for this utility function)
            let next_nodes = self.node_executor.get_next_nodes(workflow, &current_node_id, &current_event)?;
            match next_nodes.len() {
                0 => break, // End of workflow
                1 => current_node_id = next_nodes[0].clone(),
                _ => {
                    // Handle multiple outgoing paths by executing them in parallel
                    tracing::debug!("Node '{}' (id: {}) has {} outgoing paths, executing in parallel", node.name, current_node_id, next_nodes.len());

                    let mut handles = Vec::new();
                    for next_node_id in next_nodes {
                        // Clone all necessary data for the spawned task
                        let execution_id_clone = execution_id.to_string();
                        let workflow_clone = workflow.clone();
                        let event_clone = current_event.clone();
                        let execution_service = self.execution_service.clone();
                        let _workflow_engine = self.workflow_engine.clone();
                        let input_sync_service = self.input_sync_service.clone();
                        // Create node executor with HTTP loop scheduler if available
                        let node_executor = self.create_node_executor().await;

                        let handle = tokio::spawn(async move {
                            let parallel_executor = ParallelBranchExecutor {
                                execution_service,
                                input_sync_service,
                                node_executor,
                            };

                            tracing::debug!("Starting parallel branch execution for node: {}", next_node_id);
                            let result = parallel_executor.execute_branch(
                                &execution_id_clone,
                                &workflow_clone,
                                next_node_id,
                                event_clone
                            ).await;

                            match &result {
                                Ok(_) => tracing::debug!("Parallel branch execution completed successfully"),
                                Err(e) => tracing::error!("Parallel branch execution failed: {}", e),
                            }

                            result
                        });

                        handles.push(handle);
                    }

                    // Wait for all branches to complete
                    let results = futures::future::try_join_all(handles).await
                        .map_err(|e| SwissPipeError::Generic(format!("Failed to join parallel execution: {e}")))?;

                    // Check if any branch failed
                    for result in results {
                        result?
                    }

                    tracing::debug!("All parallel branches completed successfully");
                    // All branches completed successfully - workflow is done
                    break;
                }
            }
        }

        Ok(current_event)
    }
}

/// Helper struct for parallel branch execution that owns its dependencies
pub struct ParallelBranchExecutor {
    execution_service: Arc<ExecutionService>,
    input_sync_service: Arc<InputSyncService>,
    node_executor: NodeExecutor,
}

impl InputCoordination for ParallelBranchExecutor {
    fn get_input_sync_service(&self) -> &Arc<InputSyncService> {
        &self.input_sync_service
    }
}

impl ParallelBranchExecutor {
    pub fn new(
        execution_service: Arc<ExecutionService>,
        workflow_engine: Arc<WorkflowEngine>,
        input_sync_service: Arc<InputSyncService>,
        delay_scheduler: Arc<RwLock<Option<Arc<DelayScheduler>>>>,
    ) -> Self {
        // For now, create without HTTP loop scheduler - will be updated when available
        let node_executor = NodeExecutor::new(workflow_engine, delay_scheduler);

        Self {
            execution_service,
            input_sync_service,
            node_executor,
        }
    }

    pub async fn execute_branch(
        &self,
        execution_id: &str,
        workflow: &Workflow,
        start_node_id: String,
        mut event: WorkflowEvent,
    ) -> Result<()> {
        let mut current_node_id = start_node_id;
        let mut visited = HashSet::new();

        // Build node lookup for efficiency
        let node_map: HashMap<String, &Node> = workflow.nodes
            .iter()
            .map(|node| (node.id.clone(), node))
            .collect();

        loop {
            // Prevent infinite loops
            if visited.contains(&current_node_id) {
                return Err(SwissPipeError::CycleDetected);
            }
            visited.insert(current_node_id.clone());

            let node = node_map
                .get(&current_node_id)
                .ok_or_else(|| SwissPipeError::NodeNotFound(current_node_id.clone()))?;

            // Check if this node requires input coordination
            let (ready_to_execute, coordinated_event) = self.coordinate_node_inputs(
                workflow,
                execution_id,
                &current_node_id,
                &event,
                node.input_merge_strategy.as_ref(),
            ).await?;

            if !ready_to_execute {
                break;
            }

            event = coordinated_event;

            // Create execution step
            let input_data = serde_json::to_value(&event).ok();
            let step_id = self.execution_service
                .create_execution_step(
                    execution_id.to_string(),
                    node.id.clone(),
                    node.name.clone(),
                    input_data,
                )
                .await?;

            self.execution_service
                .update_execution_step(&step_id, StepStatus::Running, None, None)
                .await?;

            // Execute the node using current node executor (parallel branches use their own executor)
            match self.node_executor.execute_node(execution_id, workflow, node, event).await {
                Ok(result_event) => {
                    // Mark step as completed
                    let output_data = serde_json::to_value(&result_event).ok();
                    self.execution_service
                        .update_execution_step(&step_id, StepStatus::Completed, output_data, None)
                        .await?;

                    event = result_event;
                }
                Err(SwissPipeError::DelayScheduled(delay_id)) => {
                    // DelayScheduled - keep step as running during delay period
                    // Step will be marked completed when delay finishes and workflow resumes
                    tracing::info!("Delay scheduled with ID: {} for step {}", delay_id, step_id);
                    return Err(SwissPipeError::DelayScheduled(delay_id));
                }
                Err(e) => {
                    // Mark step as failed
                    self.execution_service
                        .update_execution_step(&step_id, StepStatus::Failed, None, Some(e.to_string()))
                        .await?;
                    return Err(e);
                }
            }

            // Get next nodes - use a simplified approach for parallel branches
            let next_nodes = self.get_next_nodes_for_branch(workflow, &current_node_id, &event)?;
            match next_nodes.len() {
                0 => break, // End of branch
                1 => current_node_id = next_nodes[0].clone(),
                _ => {
                    // For nested branches within parallel execution, execute sequentially for now
                    tracing::debug!("Nested branch node '{}' has {} outgoing paths, executing sequentially", current_node_id, next_nodes.len());

                    for next_node_id in next_nodes {
                        tracing::debug!("Starting nested branch execution for node: {}", next_node_id);
                        match Box::pin(self.execute_branch(execution_id, workflow, next_node_id, event.clone())).await {
                            Ok(_) => {
                                tracing::debug!("Nested branch execution completed successfully");
                            }
                            Err(e) => {
                                tracing::error!("Nested branch execution failed: {}", e);
                                return Err(e);
                            }
                        }
                    }

                    // All nested branches completed successfully
                    break;
                }
            }
        }

        Ok(())
    }

    fn get_next_nodes_for_branch(
        &self,
        workflow: &Workflow,
        current_node_id: &str,
        event: &WorkflowEvent,
    ) -> Result<Vec<String>> {
        let mut next_nodes = Vec::new();

        for edge in &workflow.edges {
            if edge.from_node_id == current_node_id {
                // Check if this edge has a condition
                if let Some(condition_result) = edge.condition_result {
                    // Look up the stored condition result for the current node - use node ID as key
                    if let Some(&stored_result) = event.condition_results.get(current_node_id) {
                        if stored_result == condition_result {
                            next_nodes.push(edge.to_node_id.clone());
                        }
                    }
                } else {
                    // Unconditional edge
                    next_nodes.push(edge.to_node_id.clone());
                }
            }
        }

        Ok(next_nodes)
    }
}