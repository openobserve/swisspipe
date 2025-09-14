use crate::async_execution::{ExecutionService, JobManager, DelayScheduler, input_coordination::InputCoordination};
use crate::anthropic::AnthropicCallConfig;
use crate::database::{
    workflow_executions::ExecutionStatus,
    workflow_execution_steps::StepStatus,
};
use crate::workflow::{
    engine::WorkflowEngine,
    errors::{Result, SwissPipeError},
    models::{Workflow, WorkflowEvent},
    input_sync::InputSyncService,
};
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, ActiveModelTrait, Set};
use serde_json::Value;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use std::time::Duration;
use tokio::{sync::RwLock, task::JoinHandle, time::sleep};

// Configuration constants
const DEFAULT_SHUTDOWN_TIMEOUT_SECS: u64 = 30;
const DEFAULT_MAX_RETRIES: i32 = 0;
const DELAY_TIME_MULTIPLIERS: DelayTimeMultipliers = DelayTimeMultipliers {
    seconds: 1000,
    minutes: 60_000,
    hours: 3_600_000,
    days: 86_400_000,
};

struct DelayTimeMultipliers {
    seconds: u64,
    minutes: u64,
    hours: u64,
    days: u64,
}

#[derive(Clone)]
pub struct WorkerPool {
    db: Arc<DatabaseConnection>,
    execution_service: Arc<ExecutionService>,
    job_manager: Arc<JobManager>,
    workflow_engine: Arc<WorkflowEngine>,
    input_sync_service: Arc<InputSyncService>,
    config: WorkerPoolConfig,
    workers: Arc<RwLock<Vec<Worker>>>,
    is_running: Arc<AtomicBool>,
    processed_jobs: Arc<AtomicU64>,
    delay_scheduler: Arc<RwLock<Option<Arc<DelayScheduler>>>>,
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

impl InputCoordination for WorkerPool {
    fn get_input_sync_service(&self) -> &Arc<InputSyncService> {
        &self.input_sync_service
    }
}

impl WorkerPool {
    pub fn new(
        db: Arc<DatabaseConnection>,
        workflow_engine: Arc<WorkflowEngine>,
        config: Option<WorkerPoolConfig>,
    ) -> Self {
        let execution_service = Arc::new(ExecutionService::new(db.clone()));
        let job_manager = Arc::new(JobManager::new(db.clone()));
        let input_sync_service = Arc::new(InputSyncService::new(db.clone()));

        Self {
            db,
            execution_service,
            job_manager,
            workflow_engine,
            input_sync_service,
            config: config.unwrap_or_default(),
            workers: Arc::new(RwLock::new(Vec::new())),
            is_running: Arc::new(AtomicBool::new(false)),
            processed_jobs: Arc::new(AtomicU64::new(0)),
            delay_scheduler: Arc::new(RwLock::new(None)),
        }
    }

    /// Execute workflow with step-by-step tracking
    /// Supports resumption from a specific node if execution.current_node_id is set
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

        let is_resuming = execution.current_node_id.is_some();
        let mut current_node_id = execution.current_node_id
            .unwrap_or_else(|| workflow.start_node_id.clone().unwrap_or_default());

        // If resuming from a specific node, log it
        if is_resuming {
            tracing::info!("Resuming execution {} from node '{}'", execution_id, current_node_id);
        } else {
            tracing::debug!("Starting execution {} from beginning at node '{}'", execution_id, current_node_id);
        }

        let mut visited = std::collections::HashSet::new();
        
        // Get existing completed steps to avoid re-executing them
        let steps = self.execution_service.get_execution_steps(execution_id).await?;
        let completed_steps: std::collections::HashMap<String, _> = steps
            .into_iter()
            .filter(|step| matches!(step.status.as_str(), "completed" | "skipped" | "cancelled"))
            .map(|step| (step.node_id.clone(), step))
            .collect();
        
        // Build node lookup for efficiency
        let node_map: std::collections::HashMap<String, &crate::workflow::models::Node> = workflow.nodes
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
                
                // Execute the node
                match self.execute_node_with_tracking(execution_id, workflow, node, current_event.clone()).await {
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
            
            // Get next nodes using the workflow engine's logic
            let next_nodes = self.get_next_nodes(workflow, &current_node_id, &current_event)?;
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
                        let workflow_engine = self.workflow_engine.clone();
                        let delay_scheduler = self.delay_scheduler.clone();
                        let input_sync_service = self.input_sync_service.clone();
                        
                        let handle = tokio::spawn(async move {
                            let worker_pool = WorkerPoolForBranch {
                                execution_service,
                                workflow_engine,
                                delay_scheduler,
                                input_sync_service,
                            };
                            
                            tracing::debug!("Starting parallel branch execution for node: {}", next_node_id);
                            let result = worker_pool.execute_branch_static(
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
    

    
    /// Execute a single node with the same logic as workflow engine
    async fn execute_node_with_tracking(
        &self,
        execution_id: &str,
        workflow: &Workflow,
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
            NodeType::HttpRequest { url, method, timeout_seconds, failure_action, retry_config, headers } => {
                match failure_action {
                    crate::workflow::models::FailureAction::Retry => {
                        // Use retry_config for retries on failure
                        self.workflow_engine.app_executor
                            .execute_http_request(url, method, *timeout_seconds, retry_config, event, headers)
                            .await
                    },
                    crate::workflow::models::FailureAction::Continue => {
                        // Try once, if it fails, continue with original event
                        match self.workflow_engine.app_executor
                            .execute_http_request(url, method, *timeout_seconds, &crate::workflow::models::RetryConfig { max_attempts: 1, ..retry_config.clone() }, event.clone(), headers)
                            .await 
                        {
                            Ok(result) => Ok(result),
                            Err(e) => {
                                tracing::warn!("HTTP request node '{}' failed but continuing: {}", node.name, e);
                                Ok(event) // Continue with original event
                            }
                        }
                    },
                    crate::workflow::models::FailureAction::Stop => {
                        // Try once, if it fails, stop the workflow
                        self.workflow_engine.app_executor
                            .execute_http_request(url, method, *timeout_seconds, &crate::workflow::models::RetryConfig { max_attempts: 1, ..retry_config.clone() }, event, headers)
                            .await
                    }
                }
            }
            NodeType::OpenObserve { url, authorization_header, timeout_seconds, failure_action, retry_config } => {
                match failure_action {
                    crate::workflow::models::FailureAction::Retry => {
                        self.workflow_engine.app_executor
                            .execute_openobserve(url, authorization_header, *timeout_seconds, retry_config, event)
                            .await
                    },
                    crate::workflow::models::FailureAction::Continue => {
                        match self.workflow_engine.app_executor
                            .execute_openobserve(url, authorization_header, *timeout_seconds, &crate::workflow::models::RetryConfig { max_attempts: 1, ..retry_config.clone() }, event.clone())
                            .await 
                        {
                            Ok(result) => Ok(result),
                            Err(e) => {
                                tracing::warn!("OpenObserve node '{}' failed but continuing: {}", node.name, e);
                                Ok(event)
                            }
                        }
                    },
                    crate::workflow::models::FailureAction::Stop => {
                        self.workflow_engine.app_executor
                            .execute_openobserve(url, authorization_header, *timeout_seconds, &crate::workflow::models::RetryConfig { max_attempts: 1, ..retry_config.clone() }, event)
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
            NodeType::Delay { duration, unit } => {
                use crate::workflow::models::DelayUnit;
                use chrono::Duration as ChronoDuration;
                
                // Convert delay duration to chrono Duration
                let delay_duration = match unit {
                    DelayUnit::Seconds => ChronoDuration::seconds(*duration as i64),
                    DelayUnit::Minutes => ChronoDuration::minutes(*duration as i64),
                    DelayUnit::Hours => ChronoDuration::hours(*duration as i64),
                    DelayUnit::Days => ChronoDuration::days(*duration as i64),
                };
                
                tracing::info!("Delay node '{}' scheduling delay for {} {:?}", 
                    node.name, duration, unit);
                
                // Get DelayScheduler from WorkerPool
                if let Some(delay_scheduler) = self.get_delay_scheduler().await {
                    // Find next node to continue execution
                    let next_nodes = self.get_next_nodes(workflow, &node.id, &event)?;
                    if let Some(next_node_id) = next_nodes.first() {
                        // Schedule the delay and pause execution
                        match delay_scheduler.schedule_delay(
                            execution_id.to_string(),
                            node.id.clone(),
                            next_node_id.clone(),
                            delay_duration,
                            event.clone(),
                        ).await {
                            Ok(delay_id) => {
                                tracing::info!(
                                    "Delay node '{}' scheduled with ID '{}' - execution will resume at '{}'",
                                    node.name, delay_id, next_node_id
                                );
                                
                                // Return a special signal to pause workflow execution here
                                // The workflow will be resumed by the scheduler
                                Err(SwissPipeError::DelayScheduled(delay_id))
                            }
                            Err(e) => {
                                tracing::error!("Failed to schedule delay for node '{}': {}", node.name, e);
                                Err(e)
                            }
                        }
                    } else {
                        tracing::warn!("Delay node '{}' has no next nodes - delay will be ignored", node.name);
                        Ok(event)
                    }
                } else {
                    tracing::error!("DelayScheduler not available - falling back to blocking delay");
                    // Fallback to old blocking behavior if scheduler is not available
                    use tokio::time::{sleep, Duration};
                    let delay_ms = match unit {
                        DelayUnit::Seconds => duration.saturating_mul(DELAY_TIME_MULTIPLIERS.seconds),
                        DelayUnit::Minutes => duration.saturating_mul(DELAY_TIME_MULTIPLIERS.minutes),
                        DelayUnit::Hours => duration.saturating_mul(DELAY_TIME_MULTIPLIERS.hours),
                        DelayUnit::Days => duration.saturating_mul(DELAY_TIME_MULTIPLIERS.days),
                    };
                    // No artificial delay limit since DelayScheduler supports unlimited duration
                    sleep(Duration::from_millis(delay_ms)).await;
                    tracing::debug!("Delay node '{}' completed (blocking fallback)", node.name);
                    Ok(event)
                }
            }
            NodeType::Anthropic { model, max_tokens, temperature, system_prompt, user_prompt, timeout_seconds, failure_action, retry_config } => {
                // For Anthropic nodes, we use the async version from the workflow engine
                match failure_action {
                    crate::workflow::models::FailureAction::Retry => {
                        self.workflow_engine.anthropic_service
                            .call_anthropic(&AnthropicCallConfig {
                                model,
                                max_tokens: *max_tokens,
                                temperature: *temperature,
                                system_prompt: system_prompt.as_deref(),
                                user_prompt,
                                timeout_seconds: *timeout_seconds,
                                retry_config,
                            }, &event)
                            .await
                    },
                    crate::workflow::models::FailureAction::Continue => {
                        match self.workflow_engine.anthropic_service
                            .call_anthropic(&AnthropicCallConfig {
                                model,
                                max_tokens: *max_tokens,
                                temperature: *temperature,
                                system_prompt: system_prompt.as_deref(),
                                user_prompt,
                                timeout_seconds: *timeout_seconds,
                                retry_config: &crate::workflow::models::RetryConfig { max_attempts: 1, ..retry_config.clone() },
                            }, &event)
                            .await
                        {
                            Ok(result) => Ok(result),
                            Err(e) => {
                                tracing::warn!("Anthropic node '{}' failed but continuing: {}", node.name, e);
                                Ok(event)
                            }
                        }
                    },
                    crate::workflow::models::FailureAction::Stop => {
                        self.workflow_engine.anthropic_service
                            .call_anthropic(&AnthropicCallConfig {
                                model,
                                max_tokens: *max_tokens,
                                temperature: *temperature,
                                system_prompt: system_prompt.as_deref(),
                                user_prompt,
                                timeout_seconds: *timeout_seconds,
                                retry_config: &crate::workflow::models::RetryConfig { max_attempts: 1, ..retry_config.clone() },
                            }, &event)
                            .await
                    }
                }
            }
        }
    }

    /// Get next nodes - replicating the workflow engine logic
    fn get_next_nodes(
        &self,
        workflow: &Workflow,
        current_node_id: &str,
        event: &WorkflowEvent,
    ) -> Result<Vec<String>> {
        let mut next_nodes = Vec::new();
        
        tracing::info!("Finding next nodes from node_id: {}", current_node_id);
        
        for edge in &workflow.edges {
            if edge.from_node_id == current_node_id {
                let to_node_id = &edge.to_node_id;

                match edge.condition_result {
                    None => {
                        // Unconditional edge
                        tracing::info!("Following unconditional edge to node_id: {}", to_node_id);
                        next_nodes.push(to_node_id.clone());
                    }
                    Some(expected_result) => {
                        // Conditional edge - we need to evaluate the condition
                        if self.should_follow_conditional_edge(workflow, current_node_id, expected_result, event)? {
                            tracing::info!("Following conditional edge to node_id: {}", to_node_id);
                            next_nodes.push(to_node_id.clone());
                        } else {
                            tracing::info!("Skipping conditional edge to node_id: {}", to_node_id);
                        }
                    }
                }
            }
        }
        
        tracing::info!("Next node IDs: {:?}", next_nodes);
        Ok(next_nodes)
    }
    
    /// Should follow conditional edge - replicating workflow engine logic
    fn should_follow_conditional_edge(
        &self,
        workflow: &Workflow,
        current_node_id: &str,
        expected_result: bool,
        event: &WorkflowEvent,
    ) -> Result<bool> {
        use crate::workflow::models::NodeType;
        
        // Find the current node to check if it's a condition node
        let node = workflow.nodes
            .iter()
            .find(|n| n.id == current_node_id)
            .ok_or_else(|| SwissPipeError::NodeNotFound(current_node_id.to_string()))?;
        
        match &node.node_type {
            NodeType::Condition { .. } => {
                // Get the actual condition result from the event - use node ID as key
                let actual_result = event.condition_results
                    .get(current_node_id)
                    .copied()
                    .unwrap_or(false); // Default to false if no result stored
                
                tracing::info!("Edge from '{}' (id: {}): expected={}, actual={}, follow={}", 
                    node.name, current_node_id, expected_result, actual_result, actual_result == expected_result);
                
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
        let shutdown_timeout = Duration::from_secs(DEFAULT_SHUTDOWN_TIMEOUT_SECS);
        
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

        // Check if execution was cancelled BEFORE marking job as processing
        match self.execution_service.get_execution(&job.execution_id).await {
            Ok(Some(execution)) => {
                if execution.status == "cancelled" {
                    tracing::info!("Skipping job {} - execution {} was cancelled", job.id, job.execution_id);
                    // Mark job as failed with cancellation message (job is still in claimed state)
                    if let Err(e) = self.job_manager.fail_job(&job.id, "Execution was cancelled".to_string()).await {
                        tracing::error!("Failed to mark cancelled job {} as failed: {}", job.id, e);
                    }
                    return Ok(true);
                }
            }
            Ok(None) => {
                tracing::warn!("Execution {} not found for job {}", job.execution_id, job.id);
                // Continue processing - might be a race condition
            }
            Err(e) => {
                tracing::error!("Failed to check execution status for job {}: {}", job.id, e);
                // Continue processing - don't fail job due to status check error
            }
        }

        // Mark job as processing (only if execution is not cancelled)
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
            Err(SwissPipeError::DelayScheduled(delay_id)) => {
                // DelayScheduled is a successful operation, not a failure
                if let Err(e) = self.job_manager.complete_job(&job.id).await {
                    tracing::error!("Failed to mark delayed job {} as completed: {}", job.id, e);
                }
                self.processed_jobs.fetch_add(1, Ordering::SeqCst);
                tracing::info!("Worker {} successfully scheduled delay {} for job {}", 
                    worker_id, delay_id, job.id);
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
            exec_model.current_node_id = Set(None);
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
                    .unwrap_or(DEFAULT_MAX_RETRIES);
                    
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
                    payload: Set(None), // Regular workflow execution, no special payload
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
        // Check for special job payloads (e.g., workflow_resume from DelayScheduler)
        if let Some(payload_json) = &job.payload {
            match serde_json::from_str::<serde_json::Value>(payload_json) {
                Ok(payload) => {
                    if let Some(job_type) = payload.get("type").and_then(|v| v.as_str()) {
                        if job_type == "workflow_resume" {
                            // Handle workflow resumption after delay
                            return self.handle_workflow_resume_job(payload).await;
                        }
                        // Unknown job type - log warning and proceed with regular execution
                        tracing::warn!("Unknown job type '{}' in payload for job {}", job_type, job.id);
                    }
                }
                Err(e) => {
                    // Invalid payload JSON - log error and proceed with regular execution
                    tracing::error!("Failed to parse job payload for job {}: {}", job.id, e);
                }
            }
        }

        // Regular workflow execution
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
            Err(SwissPipeError::DelayScheduled(delay_id)) => {
                // DelayScheduled - keep execution as running during delay period
                tracing::info!("Workflow execution {} paused for delay {}", job.execution_id, delay_id);
                Ok(()) // Return success since delay was scheduled successfully
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
    delay_scheduler: Arc<RwLock<Option<Arc<DelayScheduler>>>>,
    input_sync_service: Arc<InputSyncService>,
}

impl InputCoordination for WorkerPoolForBranch {
    fn get_input_sync_service(&self) -> &Arc<InputSyncService> {
        &self.input_sync_service
    }
}

impl WorkerPoolForBranch {
    async fn execute_branch_static(
        &self,
        execution_id: &str,
        workflow: &Workflow,
        start_node_id: String,
        mut event: WorkflowEvent,
    ) -> Result<()> {
        let mut current_node_id = start_node_id;
        let mut visited = std::collections::HashSet::new();
        
        // Build node lookup for efficiency
        let node_map: std::collections::HashMap<String, &crate::workflow::models::Node> = workflow.nodes
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
                .update_execution_step(&step_id, crate::database::workflow_execution_steps::StepStatus::Running, None, None)
                .await?;
            
            // Execute the node
            match self.execute_node_static(execution_id, workflow, node, event).await {
                Ok(result_event) => {
                    // Mark step as completed
                    let output_data = serde_json::to_value(&result_event).ok();
                    self.execution_service
                        .update_execution_step(&step_id, crate::database::workflow_execution_steps::StepStatus::Completed, output_data, None)
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
                        .update_execution_step(&step_id, crate::database::workflow_execution_steps::StepStatus::Failed, None, Some(e.to_string()))
                        .await?;
                    return Err(e);
                }
            }
            
            // Get next nodes - use a simplified approach for parallel branches
            let next_nodes = self.get_next_nodes_static(workflow, &current_node_id, &event)?;
            match next_nodes.len() {
                0 => break, // End of branch
                1 => current_node_id = next_nodes[0].clone(),
                _ => {
                    // For nested branches within parallel execution, execute sequentially for now
                    tracing::debug!("Nested branch node '{}' has {} outgoing paths, executing sequentially", current_node_id, next_nodes.len());
                    
                    for next_node_id in next_nodes {
                        tracing::debug!("Starting nested branch execution for node: {}", next_node_id);
                        match Box::pin(self.execute_branch_static(execution_id, workflow, next_node_id, event.clone())).await {
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
        workflow: &Workflow,
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
            NodeType::HttpRequest { url, method, timeout_seconds, failure_action, retry_config, headers } => {
                match failure_action {
                    crate::workflow::models::FailureAction::Retry => {
                        self.workflow_engine.app_executor
                            .execute_http_request(url, method, *timeout_seconds, retry_config, event, headers)
                            .await
                    },
                    crate::workflow::models::FailureAction::Continue => {
                        match self.workflow_engine.app_executor
                            .execute_http_request(url, method, *timeout_seconds, &crate::workflow::models::RetryConfig { max_attempts: 1, ..retry_config.clone() }, event.clone(), headers)
                            .await 
                        {
                            Ok(result) => Ok(result),
                            Err(e) => {
                                tracing::warn!("HTTP request node '{}' failed but continuing: {}", node.name, e);
                                Ok(event)
                            }
                        }
                    },
                    crate::workflow::models::FailureAction::Stop => {
                        self.workflow_engine.app_executor
                            .execute_http_request(url, method, *timeout_seconds, &crate::workflow::models::RetryConfig { max_attempts: 1, ..retry_config.clone() }, event, headers)
                            .await
                    }
                }
            }
            NodeType::OpenObserve { url, authorization_header, timeout_seconds, failure_action, retry_config } => {
                match failure_action {
                    crate::workflow::models::FailureAction::Retry => {
                        self.workflow_engine.app_executor
                            .execute_openobserve(url, authorization_header, *timeout_seconds, retry_config, event)
                            .await
                    },
                    crate::workflow::models::FailureAction::Continue => {
                        match self.workflow_engine.app_executor
                            .execute_openobserve(url, authorization_header, *timeout_seconds, &crate::workflow::models::RetryConfig { max_attempts: 1, ..retry_config.clone() }, event.clone())
                            .await 
                        {
                            Ok(result) => Ok(result),
                            Err(e) => {
                                tracing::warn!("OpenObserve node '{}' failed but continuing: {}", node.name, e);
                                Ok(event)
                            }
                        }
                    },
                    crate::workflow::models::FailureAction::Stop => {
                        self.workflow_engine.app_executor
                            .execute_openobserve(url, authorization_header, *timeout_seconds, &crate::workflow::models::RetryConfig { max_attempts: 1, ..retry_config.clone() }, event)
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
            NodeType::Delay { duration, unit } => {
                use crate::workflow::models::DelayUnit;
                use chrono::Duration as ChronoDuration;
                
                // Convert delay duration to chrono Duration
                let delay_duration = match unit {
                    DelayUnit::Seconds => ChronoDuration::seconds(*duration as i64),
                    DelayUnit::Minutes => ChronoDuration::minutes(*duration as i64),
                    DelayUnit::Hours => ChronoDuration::hours(*duration as i64),
                    DelayUnit::Days => ChronoDuration::days(*duration as i64),
                };
                
                tracing::info!("Delay node '{}' scheduling delay for {} {:?}", 
                    node.name, duration, unit);
                
                // Get DelayScheduler from WorkerPool
                if let Some(delay_scheduler) = self.get_delay_scheduler().await {
                    // Find next node to continue execution
                    let next_nodes = self.get_next_nodes_static(workflow, &node.name, &event)?;
                    if let Some(next_node_id) = next_nodes.first() {
                        // Schedule the delay and pause execution
                        match delay_scheduler.schedule_delay(
                            execution_id.to_string(),
                            node.name.clone(),
                            next_node_id.clone(),
                            delay_duration,
                            event.clone(),
                        ).await {
                            Ok(delay_id) => {
                                tracing::info!(
                                    "Delay node '{}' scheduled with ID '{}' - execution will resume at '{}'",
                                    node.name, delay_id, next_node_id
                                );
                                
                                // Return a special signal to pause workflow execution here
                                // The workflow will be resumed by the scheduler
                                Err(SwissPipeError::DelayScheduled(delay_id))
                            }
                            Err(e) => {
                                tracing::error!("Failed to schedule delay for node '{}': {}", node.name, e);
                                Err(e)
                            }
                        }
                    } else {
                        tracing::warn!("Delay node '{}' has no next nodes - delay will be ignored", node.name);
                        Ok(event)
                    }
                } else {
                    tracing::error!("DelayScheduler not available - falling back to blocking delay");
                    // Fallback to old blocking behavior if scheduler is not available
                    use tokio::time::{sleep, Duration};
                    let delay_ms = match unit {
                        DelayUnit::Seconds => duration.saturating_mul(DELAY_TIME_MULTIPLIERS.seconds),
                        DelayUnit::Minutes => duration.saturating_mul(DELAY_TIME_MULTIPLIERS.minutes),
                        DelayUnit::Hours => duration.saturating_mul(DELAY_TIME_MULTIPLIERS.hours),
                        DelayUnit::Days => duration.saturating_mul(DELAY_TIME_MULTIPLIERS.days),
                    };
                    // No artificial delay limit since DelayScheduler supports unlimited duration
                    sleep(Duration::from_millis(delay_ms)).await;
                    tracing::debug!("Delay node '{}' completed (blocking fallback)", node.name);
                    Ok(event)
                }
            }
            NodeType::Anthropic { model, max_tokens, temperature, system_prompt, user_prompt, timeout_seconds, failure_action, retry_config } => {
                // For Anthropic nodes, we use the async version from the workflow engine
                match failure_action {
                    crate::workflow::models::FailureAction::Retry => {
                        self.workflow_engine.anthropic_service
                            .call_anthropic(&AnthropicCallConfig {
                                model,
                                max_tokens: *max_tokens,
                                temperature: *temperature,
                                system_prompt: system_prompt.as_deref(),
                                user_prompt,
                                timeout_seconds: *timeout_seconds,
                                retry_config,
                            }, &event)
                            .await
                    },
                    crate::workflow::models::FailureAction::Continue => {
                        match self.workflow_engine.anthropic_service
                            .call_anthropic(&AnthropicCallConfig {
                                model,
                                max_tokens: *max_tokens,
                                temperature: *temperature,
                                system_prompt: system_prompt.as_deref(),
                                user_prompt,
                                timeout_seconds: *timeout_seconds,
                                retry_config: &crate::workflow::models::RetryConfig { max_attempts: 1, ..retry_config.clone() },
                            }, &event)
                            .await
                        {
                            Ok(result) => Ok(result),
                            Err(e) => {
                                tracing::warn!("Anthropic node '{}' failed but continuing: {}", node.name, e);
                                Ok(event)
                            }
                        }
                    },
                    crate::workflow::models::FailureAction::Stop => {
                        self.workflow_engine.anthropic_service
                            .call_anthropic(&AnthropicCallConfig {
                                model,
                                max_tokens: *max_tokens,
                                temperature: *temperature,
                                system_prompt: system_prompt.as_deref(),
                                user_prompt,
                                timeout_seconds: *timeout_seconds,
                                retry_config: &crate::workflow::models::RetryConfig { max_attempts: 1, ..retry_config.clone() },
                            }, &event)
                            .await
                    }
                }
            }
        }
    }

    fn get_next_nodes_static(
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
                    // Look up the stored condition result for the current node
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

    
    /// Get the delay scheduler  
    async fn get_delay_scheduler(&self) -> Option<Arc<DelayScheduler>> {
        let delay_scheduler = self.delay_scheduler.read().await;
        delay_scheduler.clone()
    }

}

impl WorkerPool {
    /// Set the delay scheduler (called after initialization)
    pub async fn set_delay_scheduler(&self, scheduler: Arc<DelayScheduler>) {
        let mut delay_scheduler = self.delay_scheduler.write().await;
        *delay_scheduler = Some(scheduler);
    }

    /// Get the delay scheduler
    async fn get_delay_scheduler(&self) -> Option<Arc<DelayScheduler>> {
        let delay_scheduler = self.delay_scheduler.read().await;
        delay_scheduler.clone()
    }

    /// Get the job manager
    pub fn get_job_manager(&self) -> Arc<JobManager> {
        self.job_manager.clone()
    }

    /// Cancel execution comprehensively including scheduled delays
    pub async fn cancel_execution_with_delays(&self, execution_id: &str) -> Result<()> {
        tracing::info!("Starting comprehensive execution cancellation with delays for: {}", execution_id);
        
        // First, cancel via execution service (jobs, steps, execution status)
        self.execution_service.cancel_execution(execution_id).await?;
        
        // Then, cancel any scheduled delays
        let delay_scheduler = self.delay_scheduler.read().await;
        if let Some(scheduler) = delay_scheduler.as_ref() {
            match scheduler.cancel_delays_for_execution(execution_id).await {
                Ok(cancelled_count) => {
                    if cancelled_count > 0 {
                        tracing::info!("Cancelled {} scheduled delays for execution {}", cancelled_count, execution_id);
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to cancel delays for execution {}: {}", execution_id, e);
                    // Don't fail the entire cancellation if delay cancellation fails
                }
            }
        } else {
            tracing::warn!("DelayScheduler not available for cancelling delays in execution {}", execution_id);
        }
        
        tracing::info!("Completed comprehensive execution cancellation for: {}", execution_id);
        Ok(())
    }

    /// Handle workflow resumption job from DelayScheduler
    async fn handle_workflow_resume_job(&self, payload: serde_json::Value) -> Result<()> {
        // Extract payload data
        let execution_id = payload.get("execution_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SwissPipeError::Generic("Missing execution_id in workflow_resume payload".to_string()))?;
        
        let current_node_id = payload.get("current_node_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SwissPipeError::Generic("Missing current_node_id in workflow_resume payload".to_string()))?;
        
        let next_node_id = payload.get("next_node_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SwissPipeError::Generic("Missing next_node_id in workflow_resume payload".to_string()))?;
        
        let delay_id = payload.get("delay_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SwissPipeError::Generic("Missing delay_id in workflow_resume payload".to_string()))?;
        
        let workflow_state: WorkflowEvent = serde_json::from_value(
            payload.get("workflow_state")
                .ok_or_else(|| SwissPipeError::Generic("Missing workflow_state in workflow_resume payload".to_string()))?
                .clone()
        ).map_err(|e| SwissPipeError::Generic(format!("Failed to deserialize workflow state from resume payload: {e}")))?;

        // Mark the delay step as completed now that the delay has finished
        let steps = self.execution_service.get_execution_steps(execution_id).await?;
        if let Some(delay_step) = steps.iter().find(|s| s.node_id == current_node_id && s.status == "running") {
            let output_data = serde_json::json!({
                "delay_completed": true,
                "delay_id": delay_id,
                "delayed_for_ms": "delay_completed"  // Could calculate actual delay time if needed
            });
            self.execution_service
                .update_execution_step(&delay_step.id, StepStatus::Completed, Some(serde_json::to_value(&output_data).unwrap()), None)
                .await?;
            tracing::info!("Marked delay step '{}' as completed after delay {}", current_node_id, delay_id);
        } else {
            tracing::warn!("Could not find running delay step '{}' for execution '{}'", current_node_id, execution_id);
        }

        tracing::info!(
            "Resuming workflow execution '{}' from node '{}' after delay {}",
            execution_id, next_node_id, delay_id
        );

        // Resume execution from the specified node (execute_workflow_from_node handles execution status updates)
        match self.execute_workflow_from_node(execution_id.to_string(), next_node_id.to_string(), workflow_state).await {
            Ok(()) => {
                // Update execution as completed
                self.execution_service
                    .update_execution_status(
                        execution_id,
                        ExecutionStatus::Completed,
                        None,
                        None,
                    )
                    .await?;
                tracing::info!("Successfully resumed and completed workflow execution {}", execution_id);
                Ok(())
            }
            Err(SwissPipeError::DelayScheduled(_delay_id)) => {
                // Another delay was encountered - this is normal
                tracing::info!("Workflow execution {} paused again for another delay", execution_id);
                Ok(())
            }
            Err(e) => {
                // Update execution as failed
                self.execution_service
                    .update_execution_status(
                        execution_id,
                        ExecutionStatus::Failed,
                        None,
                        Some(e.to_string()),
                    )
                    .await?;
                tracing::error!("Failed to resume workflow execution {}: {}", execution_id, e);
                Err(e)
            }
        }
    }

    /// Execute workflow from a specific node - used for resuming after delays
    pub async fn execute_workflow_from_node(
        &self,
        execution_id: String,
        start_node_id: String, 
        event: WorkflowEvent,
    ) -> Result<()> {
        // Get the workflow from the execution record
        let execution_record = self.execution_service
            .get_execution(&execution_id)
            .await?
            .ok_or_else(|| SwissPipeError::Generic(format!("Execution not found: {execution_id}")))?;

        let workflow = self.workflow_engine
            .get_workflow(&execution_record.workflow_id)
            .await?
            .ok_or_else(|| SwissPipeError::Generic(format!("Workflow not found: {}", execution_record.workflow_id)))?;

        // Create a worker pool instance for branch execution
        let worker_pool_for_branch = WorkerPoolForBranch {
            execution_service: Arc::clone(&self.execution_service),
            workflow_engine: Arc::clone(&self.workflow_engine),
            delay_scheduler: Arc::clone(&self.delay_scheduler),
            input_sync_service: Arc::clone(&self.input_sync_service),
        };

        // Execute from the specified node
        worker_pool_for_branch.execute_branch_static(
            &execution_id,
            &workflow,
            start_node_id,
            event,
        ).await?;

        Ok(())
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

