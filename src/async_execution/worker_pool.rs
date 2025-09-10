use crate::async_execution::{ExecutionService, JobManager};
use crate::database::{
    workflow_executions::ExecutionStatus,
    workflow_execution_steps::StepStatus,
};
use crate::workflow::{
    engine::WorkflowEngine,
    errors::{Result, SwissPipeError},
    models::{Workflow, WorkflowEvent},
};
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, ActiveModelTrait, Set};
use serde_json::Value;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use std::time::Duration;
use tokio::{sync::RwLock, task::JoinHandle, time::sleep};

#[derive(Clone)]
pub struct WorkerPool {
    db: Arc<DatabaseConnection>,
    execution_service: Arc<ExecutionService>,
    job_manager: Arc<JobManager>,
    workflow_engine: Arc<WorkflowEngine>,
    config: WorkerPoolConfig,
    workers: Arc<RwLock<Vec<Worker>>>,
    is_running: Arc<AtomicBool>,
    processed_jobs: Arc<AtomicU64>,
}

#[derive(Clone, Debug)]
pub struct WorkerPoolConfig {
    pub worker_count: usize,
    pub job_poll_interval_ms: u64,
    pub job_claim_timeout_seconds: i64,
    pub worker_health_check_interval_seconds: u64,
    pub job_claim_cleanup_interval_seconds: u64,
}

impl Default for WorkerPoolConfig {
    fn default() -> Self {
        Self {
            worker_count: 5,
            job_poll_interval_ms: 1000,
            job_claim_timeout_seconds: 300,
            worker_health_check_interval_seconds: 30,
            job_claim_cleanup_interval_seconds: 600,
        }
    }
}

#[derive(Debug)]
struct Worker {
    id: String,
    handle: Option<JoinHandle<()>>,
    status: WorkerStatus,
    current_job: Option<String>,
    processed_count: u64,
    last_activity: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, PartialEq)]
enum WorkerStatus {
    Idle,
    Busy,
    Shutdown,
}

impl WorkerPool {
    pub fn new(
        db: Arc<DatabaseConnection>,
        workflow_engine: Arc<WorkflowEngine>,
        config: Option<WorkerPoolConfig>,
    ) -> Self {
        let execution_service = Arc::new(ExecutionService::new(db.clone()));
        let job_manager = Arc::new(JobManager::new(db.clone()));

        Self {
            db,
            execution_service,
            job_manager,
            workflow_engine,
            config: config.unwrap_or_default(),
            workers: Arc::new(RwLock::new(Vec::new())),
            is_running: Arc::new(AtomicBool::new(false)),
            processed_jobs: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Execute workflow with step-by-step tracking
    /// Supports resumption from a specific node if execution.current_node_name is set
    async fn execute_workflow_with_tracking(
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

        let is_resuming = execution.current_node_name.is_some();
        let mut current_node_name = execution.current_node_name
            .unwrap_or_else(|| workflow.start_node_name.clone());

        // If resuming from a specific node, log it
        if is_resuming {
            tracing::info!("Resuming execution {} from node '{}'", execution_id, current_node_name);
        } else {
            tracing::debug!("Starting execution {} from beginning at node '{}'", execution_id, current_node_name);
        }

        let mut visited = std::collections::HashSet::new();
        
        // Get existing completed steps to avoid re-executing them
        let steps = self.execution_service.get_execution_steps(execution_id).await?;
        let completed_steps: std::collections::HashMap<String, _> = steps
            .into_iter()
            .filter(|step| matches!(step.status.as_str(), "completed" | "skipped"))
            .map(|step| (step.node_name.clone(), step))
            .collect();
        
        // Build node lookup for efficiency
        let node_map: std::collections::HashMap<String, &crate::workflow::models::Node> = workflow.nodes
            .iter()
            .map(|node| (node.name.clone(), node))
            .collect();
        
        loop {
            // Prevent infinite loops
            if visited.contains(&current_node_name) {
                return Err(SwissPipeError::CycleDetected);
            }
            visited.insert(current_node_name.clone());
            
            let node = node_map
                .get(&current_node_name)
                .ok_or_else(|| SwissPipeError::NodeNotFound(current_node_name.clone()))?;
            
            // Check if this step was already completed (resumption case)
            if let Some(completed_step) = completed_steps.get(&current_node_name) {
                tracing::debug!("Skipping already completed step '{}' for execution {}", current_node_name, execution_id);
                
                // Use the output data from the completed step as the current event
                if let Some(output_data_str) = &completed_step.output_data {
                    if let Ok(output_value) = serde_json::from_str::<WorkflowEvent>(output_data_str) {
                        current_event = output_value;
                    } else {
                        tracing::warn!("Failed to parse output data for completed step '{}', using current event", current_node_name);
                    }
                }
            } else {
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
                
                // Execute the node
                match self.execute_node_with_tracking(execution_id, node, current_event).await {
                    Ok(result_event) => {
                        // Mark step as completed
                        let output_data = serde_json::to_value(&result_event).ok();
                        self.execution_service
                            .update_execution_step(&step_id, StepStatus::Completed, output_data, None)
                            .await?;
                        current_event = result_event;
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
            
            // Get next nodes using the workflow engine's logic
            let next_nodes = self.get_next_nodes(workflow, &current_node_name, &current_event)?;
            match next_nodes.len() {
                0 => break, // End of workflow
                1 => current_node_name = next_nodes[0].clone(),
                _ => {
                    // Handle multiple outgoing paths by executing them in parallel
                    tracing::debug!("Node '{}' has {} outgoing paths, executing in parallel", current_node_name, next_nodes.len());
                    
                    let mut handles = Vec::new();
                    for next_node_name in next_nodes {
                        // Clone all necessary data for the spawned task
                        let execution_id_clone = execution_id.to_string();
                        let workflow_clone = workflow.clone();
                        let event_clone = current_event.clone();
                        let execution_service = self.execution_service.clone();
                        let workflow_engine = self.workflow_engine.clone();
                        
                        let handle = tokio::spawn(async move {
                            let worker_pool = WorkerPoolForBranch {
                                execution_service,
                                workflow_engine,
                            };
                            
                            tracing::debug!("Starting parallel branch execution for node: {}", next_node_name);
                            let result = worker_pool.execute_branch_static(
                                &execution_id_clone,
                                &workflow_clone,
                                next_node_name,
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
    

    
    /// Execute a single node with the same logic as workflow engine
    async fn execute_node_with_tracking(
        &self,
        execution_id: &str,
        node: &crate::workflow::models::Node,
        mut event: WorkflowEvent,
    ) -> Result<WorkflowEvent> {
        use crate::workflow::models::NodeType;
        
        match &node.node_type {
            NodeType::Trigger { .. } => Ok(event),
            NodeType::Condition { script } => {
                // Execute the condition and store the result
                let condition_result = self.workflow_engine.js_executor.execute_condition(script, &event).await?;
                
                tracing::info!("Condition node '{}' evaluated to: {}", node.name, condition_result);
                
                // Store the condition result in the event for edge routing
                event.condition_results.insert(node.name.clone(), condition_result);
                
                // Condition nodes pass through event with stored condition result
                Ok(event)
            }
            NodeType::Transformer { script } => {
                // For transformers, preserve condition_results from input event
                let mut transformed_event = self.workflow_engine.js_executor.execute_transformer(script, event.clone()).await
                    .map_err(SwissPipeError::JavaScript)?;
                
                // Preserve condition results from the original event
                transformed_event.condition_results = event.condition_results;
                
                Ok(transformed_event)
            }
            NodeType::App { app_type, url, method, timeout_seconds, failure_action, retry_config, headers } => {
                match failure_action {
                    crate::workflow::models::FailureAction::Retry => {
                        // Use retry_config for retries on failure
                        self.workflow_engine.app_executor
                            .execute_app(app_type, url, method, *timeout_seconds, retry_config, event, headers)
                            .await
                    },
                    crate::workflow::models::FailureAction::Continue => {
                        // Try once, if it fails, continue with original event
                        match self.workflow_engine.app_executor
                            .execute_app(app_type, url, method, *timeout_seconds, &crate::workflow::models::RetryConfig { max_attempts: 1, ..retry_config.clone() }, event.clone(), headers)
                            .await 
                        {
                            Ok(result) => Ok(result),
                            Err(e) => {
                                tracing::warn!("App node '{}' failed but continuing: {}", node.name, e);
                                Ok(event) // Continue with original event
                            }
                        }
                    },
                    crate::workflow::models::FailureAction::Stop => {
                        // Try once, if it fails, stop the workflow
                        self.workflow_engine.app_executor
                            .execute_app(app_type, url, method, *timeout_seconds, &crate::workflow::models::RetryConfig { max_attempts: 1, ..retry_config.clone() }, event, headers)
                            .await
                    }
                }
            }
            NodeType::Email { config } => {
                // Execute email node
                tracing::debug!("Executing email node '{}' with config: {:?}", node.name, config);
                match self.workflow_engine.email_service.send_email(config, &event, execution_id, &node.id).await {
                    Ok(result) => {
                        tracing::info!("Email node '{}' executed successfully: {:?}", node.name, result);
                        // Email nodes pass through the original event
                        Ok(event)
                    }
                    Err(e) => {
                        tracing::error!("Email node '{}' failed: {}", node.name, e);
                        Err(SwissPipeError::Generic(format!("Email node failed: {e}")))
                    }
                }
            }
        }
    }
    
    /// Get next nodes - replicating the workflow engine logic
    fn get_next_nodes(
        &self,
        workflow: &Workflow,
        current_node_name: &str,
        event: &WorkflowEvent,
    ) -> Result<Vec<String>> {
        let mut next_nodes = Vec::new();
        
        tracing::info!("Finding next nodes from '{}'", current_node_name);
        
        for edge in &workflow.edges {
            if edge.from_node_name == current_node_name {
                tracing::info!("Processing edge: {} -> {} (condition_result: {:?})", 
                    edge.from_node_name, edge.to_node_name, edge.condition_result);
                
                match edge.condition_result {
                    None => {
                        // Unconditional edge
                        tracing::info!("Following unconditional edge to '{}'", edge.to_node_name);
                        next_nodes.push(edge.to_node_name.clone());
                    }
                    Some(expected_result) => {
                        // Conditional edge - we need to evaluate the condition
                        if self.should_follow_conditional_edge(workflow, current_node_name, expected_result, event)? {
                            tracing::info!("Following conditional edge to '{}'", edge.to_node_name);
                            next_nodes.push(edge.to_node_name.clone());
                        } else {
                            tracing::info!("Skipping conditional edge to '{}'", edge.to_node_name);
                        }
                    }
                }
            }
        }
        
        tracing::info!("Next nodes: {:?}", next_nodes);
        Ok(next_nodes)
    }
    
    /// Should follow conditional edge - replicating workflow engine logic
    fn should_follow_conditional_edge(
        &self,
        workflow: &Workflow,
        current_node_name: &str,
        expected_result: bool,
        event: &WorkflowEvent,
    ) -> Result<bool> {
        use crate::workflow::models::NodeType;
        
        // Find the current node to check if it's a condition node
        let node = workflow.nodes
            .iter()
            .find(|n| n.name == current_node_name)
            .ok_or_else(|| SwissPipeError::NodeNotFound(current_node_name.to_string()))?;
        
        match &node.node_type {
            NodeType::Condition { .. } => {
                // Get the actual condition result from the event
                let actual_result = event.condition_results
                    .get(current_node_name)
                    .copied()
                    .unwrap_or(false); // Default to false if no result stored
                
                tracing::info!("Edge from '{}': expected={}, actual={}, follow={}", 
                    current_node_name, expected_result, actual_result, actual_result == expected_result);
                
                // Only follow the edge if the actual result matches the expected result
                Ok(actual_result == expected_result)
            }
            _ => {
                // Non-condition nodes should only have unconditional edges
                Ok(true)
            }
        }
    }

    pub async fn start(&self) -> Result<()> {
        tracing::debug!("WorkerPool::start() called");
        
        if self.is_running.load(Ordering::SeqCst) {
            tracing::warn!("Worker pool is already running");
            return Ok(());
        }

        tracing::info!("Starting worker pool...");
        self.is_running.store(true, Ordering::SeqCst);
        
        // First, recover any crashed jobs before starting workers
        tracing::info!("Running crash recovery...");
        self.recover_crashed_jobs().await?;
        tracing::info!("Crash recovery completed");
        
        tracing::info!("Starting worker pool with {} workers", self.config.worker_count);

        // Spawn workers
        tracing::info!("Spawning {} workers...", self.config.worker_count);
        let mut workers = self.workers.write().await;
        for i in 0..self.config.worker_count {
            let worker_id = format!("worker-{i}");
            tracing::debug!("Spawning worker: {}", worker_id);
            let handle = self.spawn_worker(worker_id.clone()).await;
            
            workers.push(Worker {
                id: worker_id.clone(),
                handle: Some(handle),
                status: WorkerStatus::Idle,
                current_job: None,
                processed_count: 0,
                last_activity: chrono::Utc::now(),
            });
            tracing::debug!("Worker {} spawned successfully", worker_id);
        }

        // Start cleanup task
        tracing::info!("Starting cleanup task...");
        self.spawn_cleanup_task().await;
        tracing::info!("Cleanup task started");

        tracing::info!("Worker pool started successfully with {} workers", workers.len());
        Ok(())
    }

    pub async fn shutdown(&self) -> Result<()> {
        if !self.is_running.load(Ordering::SeqCst) {
            return Ok(());
        }

        tracing::info!("Shutting down worker pool gracefully...");
        self.is_running.store(false, Ordering::SeqCst);

        // Wait for workers to finish with proper timeout and error handling
        let mut workers = self.workers.write().await;
        let shutdown_timeout = Duration::from_secs(30); // 30 second timeout
        
        for worker in workers.iter_mut() {
            worker.status = WorkerStatus::Shutdown;
            if let Some(handle) = worker.handle.take() {
                tracing::info!("Waiting for worker {} to shutdown...", worker.id);
                
                // Wait for worker with timeout
                match tokio::time::timeout(shutdown_timeout, handle).await {
                    Ok(Ok(())) => {
                        tracing::info!("Worker {} shut down cleanly", worker.id);
                    }
                    Ok(Err(e)) => {
                        tracing::error!("Worker {} panicked during shutdown: {}", worker.id, e);
                        // Continue with other workers even if one panicked
                    }
                    Err(_) => {
                        tracing::error!("Worker {} shutdown timed out after {:?}, forcing termination", 
                            worker.id, shutdown_timeout);
                        // Worker is now considered dead, continue with cleanup
                    }
                }
            }
        }

        // Clear all workers after shutdown attempts
        let worker_count = workers.len();
        workers.clear();
        
        tracing::info!("Worker pool shutdown completed: {} workers processed", worker_count);
        Ok(())
    }

    async fn spawn_worker(&self, worker_id: String) -> JoinHandle<()> {
        let pool = self.clone();
        let id = worker_id.clone();

        tokio::spawn(async move {
            tracing::info!("Worker {} started", id);

            while pool.is_running.load(Ordering::SeqCst) {
                match pool.process_next_job(&id).await {
                    Ok(processed) => {
                        if !processed {
                            // No job available, wait before polling again
                            sleep(Duration::from_millis(pool.config.job_poll_interval_ms)).await;
                        }
                    }
                    Err(e) => {
                        tracing::error!("Worker {} error: {}", id, e);
                        sleep(Duration::from_millis(pool.config.job_poll_interval_ms)).await;
                    }
                }
            }

            tracing::info!("Worker {} stopped", id);
        })
    }

    async fn spawn_cleanup_task(&self) -> JoinHandle<()> {
        let pool = self.clone();

        tokio::spawn(async move {
            while pool.is_running.load(Ordering::SeqCst) {
                // Clean up stale jobs
                if let Err(e) = pool.job_manager.cleanup_stale_jobs(pool.config.job_claim_timeout_seconds).await {
                    tracing::error!("Failed to cleanup stale jobs: {}", e);
                }

                sleep(Duration::from_secs(pool.config.job_claim_cleanup_interval_seconds)).await;
            }
        })
    }

    async fn process_next_job(&self, worker_id: &str) -> Result<bool> {
        // Claim a job
        let job = match self.job_manager.claim_job(worker_id).await? {
            Some(job) => job,
            None => return Ok(false), // No job available
        };

        tracing::info!("Worker {} processing job {} for execution {}", 
            worker_id, job.id, job.execution_id);

        // Update worker status
        self.update_worker_status(worker_id, WorkerStatus::Busy, Some(job.id.clone())).await;

        // Mark job as processing
        if let Err(e) = self.job_manager.start_job_processing(&job.id).await {
            tracing::error!("Failed to mark job {} as processing: {}", job.id, e);
            return Ok(true);
        }

        // Execute the workflow
        let result = self.execute_workflow_job(&job).await;

        // Handle job completion
        match result {
            Ok(()) => {
                if let Err(e) = self.job_manager.complete_job(&job.id).await {
                    tracing::error!("Failed to mark job {} as completed: {}", job.id, e);
                }
                self.processed_jobs.fetch_add(1, Ordering::SeqCst);
                tracing::info!("Worker {} completed job {}", worker_id, job.id);
            }
            Err(e) => {
                tracing::error!("Worker {} failed job {}: {}", worker_id, job.id, e);
                if let Err(fail_error) = self.job_manager.fail_job(&job.id, e.to_string()).await {
                    tracing::error!("Failed to mark job {} as failed: {}", job.id, fail_error);
                }
            }
        }

        // Reset worker status
        self.update_worker_status(worker_id, WorkerStatus::Idle, None).await;

        Ok(true)
    }

    /// Recover crashed jobs on startup
    async fn recover_crashed_jobs(&self) -> Result<()> {
        tracing::info!("Starting crash recovery process...");
        
        // 1. Reset all claimed jobs that were never processed (older than claim timeout)
        let recovered_claimed = self.job_manager
            .cleanup_stale_jobs(self.config.job_claim_timeout_seconds)
            .await?;
        
        if recovered_claimed > 0 {
            tracing::warn!("Recovered {} stale claimed jobs", recovered_claimed);
        }
        
        // 2. Reset all executions stuck in 'running' state back to 'pending'
        let recovered_executions = self.recover_running_executions().await?;
        
        if recovered_executions > 0 {
            tracing::warn!("Recovered {} running executions", recovered_executions);
        }
        
        tracing::info!("Crash recovery completed: {} claimed jobs, {} running executions", 
            recovered_claimed, recovered_executions);
        
        Ok(())
    }
    
    /// Reset running executions back to pending and create new jobs for them
    async fn recover_running_executions(&self) -> Result<u64> {
        // Find all executions in 'running' state
        let running_executions = crate::database::workflow_executions::Entity::find()
            .filter(crate::database::workflow_executions::Column::Status.eq("running"))
            .all(self.db.as_ref())
            .await?;
            
        let mut recovered_count = 0;
        
        for execution in running_executions {
            tracing::info!("Recovering running execution: {}", execution.id);
            
            // Reset execution status to pending
            let mut exec_model: crate::database::workflow_executions::ActiveModel = execution.clone().into();
            exec_model.status = Set("pending".to_string());
            exec_model.current_node_name = Set(None);
            exec_model.updated_at = Set(chrono::Utc::now().timestamp_micros());
            exec_model.update(self.db.as_ref()).await?;
            
            // Create a new job for this execution (if one doesn't already exist)
            let existing_job = crate::database::job_queue::Entity::find()
                .filter(crate::database::job_queue::Column::ExecutionId.eq(&execution.id))
                .filter(crate::database::job_queue::Column::Status.is_in([
                    "pending", "claimed", "processing"
                ]))
                .one(self.db.as_ref())
                .await?;
                
            if existing_job.is_none() {
                // Create new job for recovered execution
                let max_retries = std::env::var("SP_WORKFLOW_MAX_RETRIES")
                    .ok()
                    .and_then(|v| v.parse::<i32>().ok())
                    .unwrap_or(0);
                    
                let job = crate::database::job_queue::ActiveModel {
                    id: Set(uuid::Uuid::now_v7().to_string()),
                    execution_id: Set(execution.id.clone()),
                    priority: Set(0),
                    scheduled_at: Set(chrono::Utc::now().timestamp_micros()),
                    claimed_at: Set(None),
                    claimed_by: Set(None),
                    max_retries: Set(max_retries),
                    retry_count: Set(0),
                    status: Set("pending".to_string()),
                    error_message: Set(Some("Recovered from crash".to_string())),
                    created_at: Set(chrono::Utc::now().timestamp_micros()),
                    updated_at: Set(chrono::Utc::now().timestamp_micros()),
                };
                
                job.insert(self.db.as_ref()).await?;
                tracing::info!("Created recovery job for execution: {}", execution.id);
            }
            
            recovered_count += 1;
        }
        
        Ok(recovered_count)
    }

    async fn execute_workflow_job(&self, job: &crate::database::job_queue::Model) -> Result<()> {
        // Get the execution details
        let execution = self.execution_service
            .get_execution(&job.execution_id)
            .await?
            .ok_or_else(|| SwissPipeError::WorkflowNotFound(job.execution_id.clone()))?;

        // Update execution status to running
        self.execution_service
            .update_execution_status(&job.execution_id, ExecutionStatus::Running, None, None)
            .await?;

        // Load the workflow
        let workflow = self.workflow_engine.load_workflow(&execution.workflow_id).await?;

        // Parse input data and extract headers
        let execution_data: Value = execution.input_data
            .as_ref()
            .map(|data| serde_json::from_str(data))
            .transpose()?
            .unwrap_or(Value::Null);

        // Extract data, headers, and metadata from stored execution data
        let (input_data, headers, metadata) = if execution_data.is_object() {
            let data = execution_data.get("data").unwrap_or(&Value::Null).clone();
            
            // Extract headers with proper lifetime handling
            let default_headers = serde_json::json!({});
            let headers_value = execution_data.get("headers").unwrap_or(&default_headers);
            let headers: std::collections::HashMap<String, String> = serde_json::from_value(headers_value.clone())
                .unwrap_or_default();
            
            // Extract metadata with proper lifetime handling  
            let default_metadata = serde_json::json!({});
            let metadata_value = execution_data.get("metadata").unwrap_or(&default_metadata);
            let metadata: std::collections::HashMap<String, String> = serde_json::from_value(metadata_value.clone())
                .unwrap_or_default();
            
            (data, headers, metadata)
        } else {
            // Legacy format - treat entire execution_data as input data
            (execution_data, std::collections::HashMap::new(), std::collections::HashMap::new())
        };

        // Create workflow event with restored headers
        let event = WorkflowEvent {
            data: input_data,
            metadata,
            headers,
            condition_results: std::collections::HashMap::new(),
        };

        // Execute the workflow with step tracking
        match self.execute_workflow_with_tracking(&job.execution_id, &workflow, event).await {
            Ok(_result_event) => {
                // Update execution as completed
                self.execution_service
                    .update_execution_status(
                        &job.execution_id,
                        ExecutionStatus::Completed,
                        None,
                        None,
                    )
                    .await?;

                tracing::info!("Successfully executed workflow for execution {}", job.execution_id);
                Ok(())
            }
            Err(e) => {
                // Update execution as failed
                self.execution_service
                    .update_execution_status(
                        &job.execution_id,
                        ExecutionStatus::Failed,
                        None,
                        Some(e.to_string()),
                    )
                    .await?;

                Err(e)
            }
        }
    }

    async fn update_worker_status(&self, worker_id: &str, status: WorkerStatus, current_job: Option<String>) {
        let mut workers = self.workers.write().await;
        if let Some(worker) = workers.iter_mut().find(|w| w.id == worker_id) {
            worker.status = status;
            worker.current_job = current_job;
            worker.last_activity = chrono::Utc::now();
        }
    }

    pub async fn get_stats(&self) -> WorkerPoolStats {
        let workers = self.workers.read().await;
        let queue_stats = self.job_manager.get_queue_stats().await.unwrap_or_default();

        let idle_workers = workers.iter().filter(|w| w.status == WorkerStatus::Idle).count();
        let busy_workers = workers.iter().filter(|w| w.status == WorkerStatus::Busy).count();
        let _total_processed: u64 = workers.iter().map(|w| w.processed_count).sum();

        WorkerPoolStats {
            total_workers: workers.len(),
            idle_workers,
            busy_workers,
            total_processed_jobs: self.processed_jobs.load(Ordering::SeqCst),
            queue_pending: queue_stats.pending,
            queue_processing: queue_stats.processing,
            queue_failed: queue_stats.failed,
            queue_dead_letter: queue_stats.dead_letter,
        }
    }
}

/// Helper struct for parallel branch execution that owns its dependencies
struct WorkerPoolForBranch {
    execution_service: Arc<ExecutionService>,
    workflow_engine: Arc<crate::workflow::engine::WorkflowEngine>,
}

impl WorkerPoolForBranch {
    async fn execute_branch_static(
        &self,
        execution_id: &str,
        workflow: &Workflow,
        start_node_name: String,
        mut event: WorkflowEvent,
    ) -> Result<()> {
        let mut current_node_name = start_node_name;
        let mut visited = std::collections::HashSet::new();
        
        // Build node lookup for efficiency
        let node_map: std::collections::HashMap<String, &crate::workflow::models::Node> = workflow.nodes
            .iter()
            .map(|node| (node.name.clone(), node))
            .collect();
        
        loop {
            // Prevent infinite loops
            if visited.contains(&current_node_name) {
                return Err(SwissPipeError::CycleDetected);
            }
            visited.insert(current_node_name.clone());
            
            let node = node_map
                .get(&current_node_name)
                .ok_or_else(|| SwissPipeError::NodeNotFound(current_node_name.clone()))?;
            
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
                .update_execution_step(&step_id, crate::database::workflow_execution_steps::StepStatus::Running, None, None)
                .await?;
            
            // Execute the node
            match self.execute_node_static(execution_id, node, event).await {
                Ok(result_event) => {
                    // Mark step as completed
                    let output_data = serde_json::to_value(&result_event).ok();
                    self.execution_service
                        .update_execution_step(&step_id, crate::database::workflow_execution_steps::StepStatus::Completed, output_data, None)
                        .await?;
                    
                    event = result_event;
                }
                Err(e) => {
                    // Mark step as failed
                    self.execution_service
                        .update_execution_step(&step_id, crate::database::workflow_execution_steps::StepStatus::Failed, None, Some(e.to_string()))
                        .await?;
                    return Err(e);
                }
            }
            
            // Get next nodes - use a simplified approach for parallel branches
            let next_nodes = self.get_next_nodes_static(workflow, &current_node_name, &event)?;
            match next_nodes.len() {
                0 => break, // End of branch
                1 => current_node_name = next_nodes[0].clone(),
                _ => {
                    // For nested branches within parallel execution, execute sequentially for now
                    tracing::debug!("Nested branch node '{}' has {} outgoing paths, executing sequentially", current_node_name, next_nodes.len());
                    
                    for next_node_name in next_nodes {
                        tracing::debug!("Starting nested branch execution for node: {}", next_node_name);
                        match Box::pin(self.execute_branch_static(execution_id, workflow, next_node_name, event.clone())).await {
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
    
    async fn execute_node_static(
        &self,
        execution_id: &str,
        node: &crate::workflow::models::Node,
        mut event: WorkflowEvent,
    ) -> Result<WorkflowEvent> {
        use crate::workflow::models::NodeType;
        
        match &node.node_type {
            NodeType::Trigger { .. } => Ok(event),
            NodeType::Condition { script } => {
                let condition_result = self.workflow_engine.js_executor.execute_condition(script, &event).await?;
                event.condition_results.insert(node.name.clone(), condition_result);
                Ok(event)
            }
            NodeType::Transformer { script } => {
                let mut transformed_event = self.workflow_engine.js_executor.execute_transformer(script, event.clone()).await
                    .map_err(SwissPipeError::JavaScript)?;
                transformed_event.condition_results = event.condition_results;
                Ok(transformed_event)
            }
            NodeType::App { app_type, url, method, timeout_seconds, failure_action, retry_config, headers } => {
                match failure_action {
                    crate::workflow::models::FailureAction::Retry => {
                        self.workflow_engine.app_executor
                            .execute_app(app_type, url, method, *timeout_seconds, retry_config, event, headers)
                            .await
                    },
                    crate::workflow::models::FailureAction::Continue => {
                        match self.workflow_engine.app_executor
                            .execute_app(app_type, url, method, *timeout_seconds, &crate::workflow::models::RetryConfig { max_attempts: 1, ..retry_config.clone() }, event.clone(), headers)
                            .await 
                        {
                            Ok(result) => Ok(result),
                            Err(e) => {
                                tracing::warn!("App node '{}' failed but continuing: {}", node.name, e);
                                Ok(event)
                            }
                        }
                    },
                    crate::workflow::models::FailureAction::Stop => {
                        self.workflow_engine.app_executor
                            .execute_app(app_type, url, method, *timeout_seconds, &crate::workflow::models::RetryConfig { max_attempts: 1, ..retry_config.clone() }, event, headers)
                            .await
                    }
                }
            }
            NodeType::Email { config } => {
                match self.workflow_engine.email_service.send_email(config, &event, execution_id, &node.id).await {
                    Ok(result) => {
                        tracing::info!("Email node '{}' executed successfully: {:?}", node.name, result);
                        Ok(event)
                    }
                    Err(e) => {
                        tracing::error!("Email node '{}' failed: {}", node.name, e);
                        Err(SwissPipeError::Generic(format!("Email node failed: {e}")))
                    }
                }
            }
        }
    }
    
    fn get_next_nodes_static(
        &self,
        workflow: &Workflow,
        current_node_name: &str,
        event: &WorkflowEvent,
    ) -> Result<Vec<String>> {
        let mut next_nodes = Vec::new();
        
        for edge in &workflow.edges {
            if edge.from_node_name == current_node_name {
                // Check if this edge has a condition
                if let Some(condition_result) = edge.condition_result {
                    // Look up the stored condition result for the current node
                    if let Some(&stored_result) = event.condition_results.get(current_node_name) {
                        if stored_result == condition_result {
                            next_nodes.push(edge.to_node_name.clone());
                        }
                    }
                } else {
                    // Unconditional edge
                    next_nodes.push(edge.to_node_name.clone());
                }
            }
        }
        
        Ok(next_nodes)
    }

}

#[derive(Debug, serde::Serialize)]
pub struct WorkerPoolStats {
    pub total_workers: usize,
    pub idle_workers: usize,
    pub busy_workers: usize,
    pub total_processed_jobs: u64,
    pub queue_pending: u64,
    pub queue_processing: u64,
    pub queue_failed: u64,
    pub queue_dead_letter: u64,
}

