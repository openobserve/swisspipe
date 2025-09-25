// Worker pool for SwissPipe async execution - refactored for modularity
// This module has been broken down into smaller, focused modules for better maintainability

// Import and re-export the modules
mod config;
mod node_executor;
mod workflow_executor;

// Re-export all public types for backward compatibility
pub use config::*;
pub use workflow_executor::{WorkflowExecutor, ParallelBranchExecutor};

use crate::async_execution::{ExecutionService, JobManager, DelayScheduler, HttpLoopScheduler, input_coordination::InputCoordination};
use crate::database::{
    workflow_executions::ExecutionStatus,
    workflow_execution_steps::StepStatus,
};
use crate::workflow::{
    engine::WorkflowEngine,
    errors::{Result, SwissPipeError},
    models::{WorkflowEvent},
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

/// Main WorkerPool struct managing workflow execution
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
    workflow_executor: Arc<WorkflowExecutor>,
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
        let delay_scheduler = Arc::new(RwLock::new(None));

        let workflow_executor = Arc::new(WorkflowExecutor::new(
            execution_service.clone(),
            workflow_engine.clone(),
            input_sync_service.clone(),
            delay_scheduler.clone(),
        ));

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
            delay_scheduler,
            workflow_executor,
        }
    }

    /// Set the HTTP loop scheduler for this worker pool
    pub async fn set_http_loop_scheduler(&self, scheduler: Arc<HttpLoopScheduler>) -> Result<()> {
        tracing::info!("Setting HTTP loop scheduler on worker pool");

        // Try to set the scheduler on the workflow engine (might already be set)
        match self.workflow_engine.set_http_loop_scheduler(scheduler.clone()) {
            Ok(()) => {
                tracing::info!("HTTP loop scheduler set on workflow engine");
            }
            Err(e) if e.to_string().contains("already initialized") => {
                tracing::info!("HTTP loop scheduler already set on workflow engine, skipping");
            }
            Err(e) => {
                tracing::error!("Failed to set HTTP loop scheduler on workflow engine: {}", e);
                return Err(e);
            }
        }

        // Set the scheduler on the workflow executor as well
        self.workflow_executor.set_http_loop_scheduler(scheduler).await;

        Ok(())
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
            match serde_json::from_str::<Value>(payload_json) {
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

        // Execute the workflow with step tracking using the workflow executor
        match self.workflow_executor.execute_workflow_with_tracking(&job.execution_id, &workflow, event).await {
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

    /// Set the delay scheduler (called after initialization)
    pub async fn set_delay_scheduler(&self, scheduler: Arc<DelayScheduler>) {
        let mut delay_scheduler = self.delay_scheduler.write().await;
        *delay_scheduler = Some(scheduler);
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
    async fn handle_workflow_resume_job(&self, payload: Value) -> Result<()> {
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

        // Use the parallel branch executor to execute from the specified node
        let parallel_executor = ParallelBranchExecutor::new(
            Arc::clone(&self.execution_service),
            Arc::clone(&self.workflow_engine),
            Arc::clone(&self.input_sync_service),
            Arc::clone(&self.delay_scheduler),
        );

        // Execute from the specified node
        parallel_executor.execute_branch(
            &execution_id,
            &workflow,
            start_node_id,
            event,
        ).await?;

        Ok(())
    }
}