use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use std::time::Duration;
use tokio::{sync::RwLock, task::JoinHandle, time::sleep};
use sea_orm::{DatabaseConnection, EntityTrait};

use crate::async_execution::{
    ExecutionService, HttpLoopScheduler, DelayScheduler, AsyncHilService,
    mpsc_job_distributor::{MpscJobDistributor, JobMessage},
};
use crate::workflow::{
    engine::WorkflowEngine,
    errors::{Result, SwissPipeError},
};
use crate::{log_workflow_error, log_workflow_warn};
// Removed unused ExecutionStatus import

/// MPSC-enabled Worker Pool that receives jobs via channels instead of database polling
/// This eliminates the "database is locked" race conditions by using single consumer pattern
#[derive(Clone)]
pub struct MpscWorkerPool {
    db: Arc<DatabaseConnection>,
    execution_service: Arc<ExecutionService>,
    workflow_engine: Arc<WorkflowEngine>,
    mpsc_distributor: Arc<MpscJobDistributor>,
    job_receiver: Arc<tokio::sync::Mutex<tokio::sync::mpsc::UnboundedReceiver<JobMessage>>>,
    config: MpscWorkerPoolConfig,
    workers: Arc<RwLock<Vec<MpscWorker>>>,
    is_running: Arc<AtomicBool>,
    processed_jobs: Arc<AtomicU64>,
    delay_scheduler: Arc<RwLock<Option<Arc<DelayScheduler>>>>,
    async_hil_service: Arc<AsyncHilService>,
}

/// Configuration for MPSC worker pool (separate from config::WorkerPoolConfig)
#[derive(Debug, Clone)]
pub struct MpscWorkerPoolConfig {
    pub worker_count: usize,
    pub job_claim_timeout_seconds: u64,
    pub job_claim_cleanup_interval_seconds: u64,
    pub mpsc_polling_interval_ms: u64,
}

impl Default for MpscWorkerPoolConfig {
    fn default() -> Self {
        Self {
            worker_count: 3,
            job_claim_timeout_seconds: 300, // 5 minutes
            job_claim_cleanup_interval_seconds: 60, // 1 minute
            mpsc_polling_interval_ms: 100, // 100ms for MPSC consumer
        }
    }
}


#[derive(Debug)]
pub struct MpscWorker {
    pub id: String,
    pub handle: Option<JoinHandle<()>>,
    pub status: WorkerStatus,
    pub current_job: Option<String>,
    pub processed_count: u64,
    pub last_activity: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WorkerStatus {
    Idle,
    Busy,
    Stopped,
}


impl MpscWorkerPool {
    /// Create new MPSC worker pool with job distribution via channels
    pub fn new(
        db: Arc<DatabaseConnection>,
        workflow_engine: Arc<WorkflowEngine>,
        config: Option<MpscWorkerPoolConfig>,
    ) -> Self {
        let execution_service = Arc::new(ExecutionService::new(db.clone()));
        let delay_scheduler = Arc::new(RwLock::new(None));

        // Create MPSC job distributor with unbounded channel capacity
        let (mpsc_distributor, job_receiver) = MpscJobDistributor::new(
            db.clone(),
            1000, // Channel capacity (not used with unbounded)
        );

        // Create AsyncHilService for HIL job processing
        let async_hil_service = Arc::new(AsyncHilService::new(
            db.clone(),
            workflow_engine.clone(),
            Arc::new(mpsc_distributor.clone()),
        ));

        Self {
            db,
            execution_service,
            workflow_engine,
            mpsc_distributor: Arc::new(mpsc_distributor),
            job_receiver: Arc::new(tokio::sync::Mutex::new(job_receiver)),
            config: config.unwrap_or_default(),
            workers: Arc::new(RwLock::new(Vec::new())),
            is_running: Arc::new(AtomicBool::new(false)),
            processed_jobs: Arc::new(AtomicU64::new(0)),
            delay_scheduler,
            async_hil_service,
        }
    }

    /// Create new MPSC worker pool with external distributor and job receiver
    /// This prevents resource leaks by reusing the same channel from main.rs
    pub fn with_distributor(
        db: Arc<DatabaseConnection>,
        workflow_engine: Arc<WorkflowEngine>,
        distributor: Arc<MpscJobDistributor>,
        job_receiver: tokio::sync::mpsc::UnboundedReceiver<JobMessage>,
        config: Option<MpscWorkerPoolConfig>,
    ) -> Self {
        let execution_service = Arc::new(ExecutionService::new(db.clone()));
        let delay_scheduler = Arc::new(RwLock::new(None));

        // Create AsyncHilService for HIL job processing
        let async_hil_service = Arc::new(AsyncHilService::new(
            db.clone(),
            workflow_engine.clone(),
            distributor.clone(),
        ));

        Self {
            db,
            execution_service,
            workflow_engine,
            mpsc_distributor: distributor,
            job_receiver: Arc::new(tokio::sync::Mutex::new(job_receiver)),
            config: config.unwrap_or_default(),
            workers: Arc::new(RwLock::new(Vec::new())),
            is_running: Arc::new(AtomicBool::new(false)),
            processed_jobs: Arc::new(AtomicU64::new(0)),
            delay_scheduler,
            async_hil_service,
        }
    }

    /// Get reference to the MPSC distributor for job queuing
    pub fn get_mpsc_distributor(&self) -> Arc<MpscJobDistributor> {
        self.mpsc_distributor.clone()
    }

    /// Set the HTTP loop scheduler for this worker pool
    pub async fn set_http_loop_scheduler(&self, scheduler: Arc<HttpLoopScheduler>) -> Result<()> {
        tracing::info!("Setting HTTP loop scheduler on MPSC worker pool");

        // Try to set the scheduler on the workflow engine (might already be set)
        match self.workflow_engine.set_http_loop_scheduler(scheduler.clone()) {
            Ok(()) => {
                tracing::info!("HTTP loop scheduler set on workflow engine");
            }
            Err(e) if e.to_string().contains("already initialized") => {
                tracing::info!("HTTP loop scheduler already set on workflow engine, skipping");
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    "Failed to set HTTP loop scheduler on workflow engine"
                );
                return Err(e);
            }
        }

        // Note: WorkflowExecutor was part of the legacy worker_pool system and has been removed

        Ok(())
    }

    /// Set the delay scheduler for this worker pool (compatible with WorkerPool interface)
    pub async fn set_delay_scheduler(&self, scheduler: Arc<DelayScheduler>) {
        tracing::info!("Setting delay scheduler on MPSC worker pool");

        // Store the scheduler in the delay_scheduler field
        let mut delay_scheduler_guard = self.delay_scheduler.write().await;
        *delay_scheduler_guard = Some(scheduler.clone());

        // Note: WorkflowExecutor gets the delay scheduler through its NodeExecutor
        // which was configured during construction, so no need to set it here

        tracing::info!("Delay scheduler set successfully on MPSC worker pool");
    }

    /// Start the MPSC worker pool with both consumer and workers
    pub async fn start(&self) -> Result<()> {
        tracing::debug!("MpscWorkerPool::start() called");

        if self.is_running.load(Ordering::SeqCst) {
            tracing::warn!("MPSC worker pool is already running");
            return Ok(());
        }

        tracing::info!("Starting MPSC worker pool...");
        self.is_running.store(true, Ordering::SeqCst);

        // First, recover any crashed jobs before starting
        tracing::info!("Running MPSC crash recovery...");
        self.recover_crashed_jobs().await?;
        tracing::info!("MPSC crash recovery completed");

        // Start the MPSC job consumer (single consumer that pulls from database)
        tracing::info!("Starting MPSC job consumer with {}ms polling interval", self.config.mpsc_polling_interval_ms);
        self.mpsc_distributor.start_consumer(self.config.mpsc_polling_interval_ms).await?;
        tracing::info!("MPSC job consumer started successfully");

        // Spawn worker tasks that process jobs from MPSC channels
        tracing::info!("Starting MPSC worker pool with {} workers", self.config.worker_count);
        let mut workers = self.workers.write().await;
        for i in 0..self.config.worker_count {
            let worker_id = format!("mpsc-worker-{i}");
            tracing::debug!("Spawning MPSC worker: {}", worker_id);
            let handle = self.spawn_mpsc_worker(worker_id.clone()).await;

            workers.push(MpscWorker {
                id: worker_id.clone(),
                handle: Some(handle),
                status: WorkerStatus::Idle,
                current_job: None,
                processed_count: 0,
                last_activity: chrono::Utc::now(),
            });
            tracing::debug!("MPSC worker {} spawned successfully", worker_id);
        }

        tracing::info!("MPSC worker pool started successfully with {} workers", workers.len());
        Ok(())
    }

    /// Spawn an MPSC worker that consumes jobs from channels instead of polling database
    async fn spawn_mpsc_worker(&self, worker_id: String) -> JoinHandle<()> {
        let pool = self.clone();
        let id = worker_id.clone();

        tokio::spawn(async move {
            tracing::info!("MPSC worker {} started", id);

            while pool.is_running.load(Ordering::SeqCst) {
                // Try to receive a job from MPSC channel (non-blocking)
                let job_message = {
                    let mut receiver_guard = pool.job_receiver.lock().await;

                    // Use try_recv to avoid blocking - workers should be responsive to shutdown
                    match receiver_guard.try_recv() {
                        Ok(job) => Some(job),
                        Err(tokio::sync::mpsc::error::TryRecvError::Empty) => None,
                        Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                            tracing::warn!("MPSC worker {} - channel disconnected, stopping", id);
                            break;
                        }
                    }
                };

                match job_message {
                    Some(job) => {
                        // Process the job received from MPSC channel
                        match pool.process_mpsc_job(&id, job).await {
                            Ok(()) => {
                                // Job processed successfully
                                pool.processed_jobs.fetch_add(1, Ordering::SeqCst);
                            }
                            Err(e) => {
                                tracing::error!(
                                    worker_id = %id,
                                    error = %e,
                                    "MPSC worker error processing job"
                                );
                            }
                        }
                    }
                    None => {
                        // No job available, wait briefly before checking again
                        sleep(Duration::from_millis(50)).await;
                    }
                }
            }

            tracing::info!("MPSC worker {} stopped", id);
        })
    }

    /// Process a job received from MPSC channel
    async fn process_mpsc_job(&self, worker_id: &str, job_message: JobMessage) -> Result<()> {
        tracing::info!("MPSC worker {} processing job {} for execution {}",
            worker_id, job_message.job_id, job_message.execution_id);

        // Update worker status
        self.update_worker_status(worker_id, WorkerStatus::Busy, Some(job_message.job_id.clone())).await;

        // Check if execution was cancelled before processing
        match self.execution_service.get_execution(&job_message.execution_id).await {
            Ok(Some(execution)) => {
                if execution.status == "cancelled" {
                    tracing::info!("Skipping MPSC job {} - execution {} was cancelled",
                                 job_message.job_id, job_message.execution_id);

                    // Mark job as failed with cancellation message
                    if let Err(e) = self.mpsc_distributor.fail_job(&job_message.job_id, "Execution was cancelled").await {
                        log_workflow_error!(
                            &job_message.execution_id,
                            &job_message.execution_id,
                            "Failed to mark cancelled MPSC job as failed",
                            e
                        );
                    }

                    // Update worker status back to idle
                    self.update_worker_status(worker_id, WorkerStatus::Idle, None).await;
                    return Ok(());
                }
            }
            Ok(None) => {
                log_workflow_warn!(
                    &job_message.execution_id,
                    &job_message.execution_id,
                    "Execution not found for MPSC job"
                );
            }
            Err(e) => {
                log_workflow_error!(
                    &job_message.execution_id,
                    &job_message.execution_id,
                    "Failed to check execution status for MPSC job",
                    e
                );
            }
        }

        // Process the job based on payload content (check for HIL resumption first)
        let result = if let Some(payload) = &job_message.payload {
            // Check if this is an HIL resumption job by examining payload
            if let Ok(payload_json) = serde_json::from_str::<serde_json::Value>(payload) {
                if let Some(job_type) = payload_json.get("type").and_then(|t| t.as_str()) {
                    if job_type == "hil_resumption" {
                        // Handle HIL resumption job
                        return match self.handle_hil_resumption_job(Some(payload)).await {
                            Ok(()) => {
                                // Mark job as completed and update worker status
                                if let Err(e) = self.mpsc_distributor.complete_job(&job_message.job_id).await {
                                    log_workflow_error!(
                                        &job_message.execution_id,
                                        &job_message.execution_id,
                                        "Failed to mark HIL resumption job as completed",
                                        e
                                    );
                                }
                                self.update_worker_status(worker_id, WorkerStatus::Idle, None).await;
                                Ok(())
                            }
                            Err(e) => {
                                // Mark job as failed and update worker status
                                if let Err(mark_err) = self.mpsc_distributor.fail_job(&job_message.job_id, &e.to_string()).await {
                                    log_workflow_error!(
                                        &job_message.execution_id,
                                        &job_message.execution_id,
                                        "Failed to mark HIL resumption job as failed",
                                        mark_err
                                    );
                                }
                                self.update_worker_status(worker_id, WorkerStatus::Idle, None).await;
                                Err(e)
                            }
                        };
                    }
                }
            }
            // Continue with other job type processing
            if let Ok(payload_json) = serde_json::from_str::<serde_json::Value>(payload) {
                if let Some(job_type) = payload_json.get("type").and_then(|t| t.as_str()) {
                    match job_type {
                        "hil_execution" => {
                            // Extract HIL configuration and event data
                            if let Some(hil_config) = payload_json.get("hil_config").and_then(|c| c.as_str()) {
                                // Create mock event for HIL processing - in real scenario this would come from job data
                                let mock_event = crate::workflow::models::WorkflowEvent {
                                    data: serde_json::json!({}),
                                    metadata: std::collections::HashMap::new(),
                                    headers: std::collections::HashMap::new(),
                                    condition_results: std::collections::HashMap::new(),
        hil_task: None,
        sources: Vec::new(),
                                };

                                // Process HIL job using AsyncHilService
                                self.async_hil_service.process_hil_job(&job_message.execution_id, hil_config, &mock_event).await
                            } else {
                                let err = crate::workflow::errors::SwissPipeError::Generic("HIL job missing configuration".to_string());
                                log_workflow_error!(
                                    &job_message.execution_id,
                                    &job_message.execution_id,
                                    "HIL job missing hil_config in payload",
                                    err
                                );
                                Err(crate::workflow::errors::SwissPipeError::Generic("HIL job missing configuration".to_string()))
                            }
                        },
                        "hil_notification" => {
                            // Handle notification execution (blue handle)
                            tracing::info!("Processing HIL notification job for execution: {}", job_message.execution_id);

                            // Extract notification node ID and event data from payload
                            if let (Some(node_id), Some(event_data)) = (
                                payload_json.get("node_id").and_then(|n| n.as_str()),
                                payload_json.get("event")
                            ) {
                                // Parse the event data
                                if let Ok(event) = serde_json::from_value::<crate::workflow::models::WorkflowEvent>(event_data.clone()) {
                                    // Execute the notification node (e.g., email node) with HIL task data
                                    tracing::info!("Executing HIL notification node: {} for execution: {}", node_id, job_message.execution_id);

                                    // Execute the notification node using the workflow engine
                                    self.execute_notification_node(&job_message.execution_id, node_id, event).await
                                } else {
                                    let err = crate::workflow::errors::SwissPipeError::Generic("Invalid HIL notification event data".to_string());
                                    log_workflow_error!(
                                        &job_message.execution_id,
                                        &job_message.execution_id,
                                        "Failed to parse HIL notification event data",
                                        err
                                    );
                                    Err(crate::workflow::errors::SwissPipeError::Generic("Invalid HIL notification event data".to_string()))
                                }
                            } else {
                                let err = crate::workflow::errors::SwissPipeError::Generic("HIL notification job missing required data".to_string());
                                log_workflow_error!(
                                    &job_message.execution_id,
                                    &job_message.execution_id,
                                    "HIL notification job missing node_id or event data",
                                    err
                                );
                                Err(crate::workflow::errors::SwissPipeError::Generic("HIL notification job missing required data".to_string()))
                            }
                        },
                        "node_execution" => {
                            // Handle individual node execution (from HIL resumption)
                            tracing::info!("Processing node execution job for execution: {}", job_message.execution_id);

                            // Extract node ID and event data from payload
                            if let (Some(node_id), Some(event_data)) = (
                                payload_json.get("node_id").and_then(|n| n.as_str()),
                                payload_json.get("event")
                            ) {
                                // Parse the event data
                                if let Ok(event) = serde_json::from_value::<crate::workflow::models::WorkflowEvent>(event_data.clone()) {
                                    // Execute the specific node with the provided event data
                                    tracing::info!("Executing target node: {} for execution: {}", node_id, job_message.execution_id);

                                    // Execute the node using the workflow engine's node executor
                                    self.execute_single_node(&job_message.execution_id, node_id, event).await
                                } else {
                                    let err = crate::workflow::errors::SwissPipeError::Generic("Invalid node execution event data".to_string());
                                    log_workflow_error!(
                                        &job_message.execution_id,
                                        &job_message.execution_id,
                                        "Failed to parse node execution event data",
                                        err
                                    );
                                    Err(crate::workflow::errors::SwissPipeError::Generic("Invalid node execution event data".to_string()))
                                }
                            } else {
                                let err = crate::workflow::errors::SwissPipeError::Generic("Node execution job missing required data".to_string());
                                log_workflow_error!(
                                    &job_message.execution_id,
                                    &job_message.execution_id,
                                    "Node execution job missing node_id or event data",
                                    err
                                );
                                Err(crate::workflow::errors::SwissPipeError::Generic("Node execution job missing required data".to_string()))
                            }
                        },
                        _ => {
                            // Handle regular workflow execution job
                            self.execute_regular_workflow(&job_message.execution_id).await
                        }
                    }
                } else {
                    // Handle regular workflow execution job (no type specified)
                    self.execute_regular_workflow(&job_message.execution_id).await
                }
            } else {
                // Handle regular workflow execution job (non-JSON payload)
                self.execute_regular_workflow(&job_message.execution_id).await
            }
        } else {
            // Handle regular workflow execution job (no payload)
            self.execute_regular_workflow(&job_message.execution_id).await
        };

        // Handle job completion or failure
        match result {
            Ok(()) => {
                // Mark job as completed in MPSC distributor
                if let Err(e) = self.mpsc_distributor.complete_job(&job_message.job_id).await {
                    log_workflow_error!(
                        &job_message.execution_id,
                        &job_message.execution_id,
                        "Failed to mark MPSC job as completed",
                        e
                    );
                }
                tracing::info!("MPSC job {} completed successfully", job_message.job_id);
            }
            Err(e) => {
                // Mark job as failed with retry logic
                let error_msg = format!("Workflow execution failed: {e}");
                match self.mpsc_distributor.fail_job(&job_message.job_id, &error_msg).await {
                    Ok(will_retry) => {
                        if will_retry {
                            tracing::info!("MPSC job {} failed, will retry: {}", job_message.job_id, error_msg);
                        } else {
                            let err = crate::workflow::errors::SwissPipeError::Generic(error_msg.clone());
                            log_workflow_error!(
                                &job_message.execution_id,
                                &job_message.execution_id,
                                "MPSC job failed permanently",
                                err
                            );
                        }
                    }
                    Err(fail_err) => {
                        log_workflow_error!(
                            &job_message.execution_id,
                            &job_message.execution_id,
                            "Failed to mark MPSC job as failed",
                            fail_err
                        );
                    }
                }
            }
        }

        // Update worker status back to idle
        self.update_worker_status(worker_id, WorkerStatus::Idle, None).await;
        Ok(())
    }

    /// Handle HIL resumption job (migrated from original worker pool)
    async fn handle_hil_resumption_job(&self, payload: Option<&str>) -> Result<()> {
        let payload = payload.ok_or_else(|| {
            SwissPipeError::Generic("HIL resumption job missing payload".to_string())
        })?;

        // Parse the job payload to extract the nested HIL resumption payload
        let job_payload: serde_json::Value = serde_json::from_str(payload)
            .map_err(|e| SwissPipeError::Generic(format!("Failed to parse HIL job payload: {e}")))?;

        let resumption_payload: crate::workflow::models::HilResumptionPayload =
            serde_json::from_value(job_payload.get("payload").unwrap_or(&serde_json::Value::Null).clone())
                .map_err(|e| SwissPipeError::Generic(format!("Failed to parse HIL resumption payload: {e}")))?;

        tracing::info!(
            "MPSC_AUDIT: HIL resumption processing - node_execution_id: {}, decision: {}, task_id: {}",
            resumption_payload.node_execution_id,
            resumption_payload.hil_response.decision,
            resumption_payload.hil_response.task_id
        );

        // Handle HIL resumption using the same logic as the original worker pool
        self.handle_hil_resumption_direct(resumption_payload).await
    }

    /// Handle HIL resumption directly (copied from original worker pool)
    async fn handle_hil_resumption_direct(&self, resumption_payload: crate::workflow::models::HilResumptionPayload) -> Result<()> {
        // Find the HIL task to get execution and workflow context
        let hil_service = crate::hil::service::HilService::new(self.db.clone());
        let hil_task = hil_service
            .get_hil_task_by_node_execution_id(&resumption_payload.node_execution_id)
            .await?
            .ok_or_else(|| SwissPipeError::Generic(
                format!("HIL task not found for node_execution_id: {}", resumption_payload.node_execution_id)
            ))?;

        // Get the execution context (for validation)
        let _execution = self.execution_service
            .get_execution(&hil_task.execution_id)
            .await?
            .ok_or_else(|| SwissPipeError::Generic(
                format!("Execution not found: {}", hil_task.execution_id)
            ))?;

        // Load the workflow to get node structure for routing
        let workflow = self.workflow_engine
            .load_workflow(&hil_task.workflow_id)
            .await?;

        // Validate the HIL node exists in the workflow
        let _hil_node = workflow.nodes.iter()
            .find(|node| node.id == hil_task.node_id)
            .ok_or_else(|| SwissPipeError::Generic(
                format!("HIL node {} not found in workflow {}", hil_task.node_id, hil_task.workflow_id)
            ))?;

        // Get target edges based on the decision
        let target_edges: Vec<_> = workflow.edges.iter()
            .filter(|edge| edge.from_node_id == hil_task.node_id)
            .collect();

        // Route based on HIL decision (approved/denied)
        let target_nodes: Vec<String> = if resumption_payload.resume_path == "approved" {
            target_edges.iter()
                .filter(|e| e.source_handle_id.as_deref() == Some("approved"))
                .map(|e| e.to_node_id.clone())
                .collect()
        } else if resumption_payload.resume_path == "denied" {
            target_edges.iter()
                .filter(|e| e.source_handle_id.as_deref() == Some("denied"))
                .map(|e| e.to_node_id.clone())
                .collect()
        } else {
            return Err(SwissPipeError::Generic(format!(
                "Invalid HIL decision: {}", resumption_payload.resume_path
            )));
        };

        if target_nodes.is_empty() {
            log_workflow_warn!(
                &hil_task.workflow_id,
                &hil_task.execution_id,
                &hil_task.node_id,
                "No target nodes found for HIL decision"
            );
            return Ok(());
        }

        // Get the original workflow execution data to preserve context
        let original_data = if let Some(original_event_data_str) = _execution.input_data.as_ref() {
            // Parse the JSON string stored in the database
            serde_json::from_str(original_event_data_str)
                .map_err(|e| {
                    tracing::warn!("Failed to parse original input_data as JSON: {} - data: {}", e, original_event_data_str);
                    e
                })
                .unwrap_or_else(|_| serde_json::json!({}))
        } else {
            serde_json::json!({})
        };

        // Extract the data field from the original event structure
        // The original input_data contains the full WorkflowEvent, we need just the data field
        let original_event_data = if let Some(data_field) = original_data.get("data") {
            data_field.clone()
        } else {
            // Fallback: if no "data" field, assume the entire object is the data
            original_data.clone()
        };

        // Use original user data as-is (hil_decision is metadata, not data)
        let merged_data = original_event_data;

        let mut event = crate::workflow::models::WorkflowEvent {
            data: merged_data.clone(),
            metadata: std::collections::HashMap::new(),
            headers: std::collections::HashMap::new(),
            condition_results: std::collections::HashMap::new(),
        hil_task: None,
        sources: Vec::new(),
        };

        // Add HIL decision to metadata
        event.metadata.insert("hil_decision".to_string(), resumption_payload.resume_path.clone());

        // Debug logging for HIL data structure
        tracing::debug!("HIL_DATA_DEBUG: Original data from HIL task: {}",
            serde_json::to_string_pretty(&original_data).unwrap_or_else(|_| "Failed to serialize".to_string())
        );
        tracing::debug!("HIL_DATA_DEBUG: Merged data for HIL resumption: {}",
            serde_json::to_string_pretty(&merged_data).unwrap_or_else(|_| "Failed to serialize".to_string())
        );
        tracing::debug!("HIL_DATA_DEBUG: Final event structure for HIL resumption: {}",
            serde_json::to_string_pretty(&event).unwrap_or_else(|_| "Failed to serialize".to_string())
        );
        event.metadata.insert("hil_task_id".to_string(), resumption_payload.hil_response.task_id.clone());
        event.metadata.insert("execution_id".to_string(), hil_task.execution_id.clone());
        event.metadata.insert("current_node_id".to_string(), hil_task.node_id.clone());

        // Create jobs for target nodes to continue workflow execution
        for target_node_id in target_nodes {
            tracing::info!("HIL resumption continuing execution to node: {}", target_node_id);

            // Create a job for the target node using the job queue
            let job_id = uuid::Uuid::new_v4().to_string();
            let now = chrono::Utc::now().timestamp_micros();

            // Create job payload for target node execution (JSON format)
            let job_payload = serde_json::json!({
                "type": "node_execution",
                "execution_id": hil_task.execution_id,
                "node_id": target_node_id,
                "event": event,
                "is_resumption": true,
                "parent_job_id": serde_json::Value::Null
            });

            let payload_json = serde_json::to_string(&job_payload)
                .map_err(|e| SwissPipeError::Generic(format!("Failed to serialize job payload: {e}")))?;

            let job = crate::database::job_queue::ActiveModel {
                id: sea_orm::Set(job_id.clone()),
                execution_id: sea_orm::Set(hil_task.execution_id.clone()),
                priority: sea_orm::Set(5), // Medium priority for HIL continuation
                scheduled_at: sea_orm::Set(now),
                claimed_at: sea_orm::Set(None),
                claimed_by: sea_orm::Set(None),
                max_retries: sea_orm::Set(3),
                retry_count: sea_orm::Set(0),
                status: sea_orm::Set(crate::database::job_queue::JobStatus::Pending.to_string()),
                error_message: sea_orm::Set(None),
                payload: sea_orm::Set(Some(payload_json)),
                created_at: sea_orm::Set(now),
                updated_at: sea_orm::Set(now),
            };

            use sea_orm::ActiveModelTrait;
            job.insert(self.db.as_ref()).await
                .map_err(|e| {
                    let err_msg = format!("Failed to create continuation job: {e}");
                    log_workflow_error!(
                        &hil_task.workflow_id,
                        &hil_task.execution_id,
                        &target_node_id,
                        "Failed to create HIL continuation job",
                        SwissPipeError::Generic(err_msg.clone())
                    );
                    SwissPipeError::Generic(err_msg)
                })?;

            tracing::info!("Created HIL continuation job {} for node {} in execution {}",
                          job_id, target_node_id, hil_task.execution_id);
        }

        tracing::info!("HIL resumption completed for task {}: {}",
                      resumption_payload.hil_response.task_id, resumption_payload.resume_path);
        Ok(())
    }

    /// Execute a notification node for HIL workflow (blue handle execution)
    async fn execute_notification_node(
        &self,
        execution_id: &str,
        node_id: &str,
        event: crate::workflow::models::WorkflowEvent,
    ) -> Result<()> {
        tracing::info!("Executing HIL notification node: {} for execution: {}", node_id, execution_id);

        // Fetch the workflow execution to get workflow_id
        if let Ok(Some(execution)) = crate::database::workflow_executions::Entity::find_by_id(execution_id)
            .one(&*self.db)
            .await
        {
            // Load the complete workflow using the workflow loader
            match self.workflow_engine.workflow_loader().load_workflow(&execution.workflow_id).await {
                Ok(workflow) => {
                    // Find the notification node
                    if let Some(notification_node) = workflow.nodes.iter().find(|n| n.id == node_id) {
                        // Execute the notification node using the node executor with step tracking
                        match self.workflow_engine.node_executor().execute_node_with_output(
                            notification_node,
                            event,
                            execution_id,
                        ).await {
                            Ok(_) => {
                                tracing::info!("Successfully executed HIL notification node: {} for execution: {}", node_id, execution_id);
                                Ok(())
                            }
                            Err(e) => {
                                log_workflow_error!(
                                    &execution.workflow_id,
                                    execution_id,
                                    node_id,
                                    "Failed to execute HIL notification node",
                                    e
                                );
                                Err(e)
                            }
                        }
                    } else {
                        let error_msg = format!("HIL notification node {node_id} not found in workflow");
                        let err = crate::workflow::errors::SwissPipeError::Generic(error_msg.clone());
                        log_workflow_error!(
                            &execution.workflow_id,
                            execution_id,
                            node_id,
                            &error_msg,
                            err
                        );
                        Err(crate::workflow::errors::SwissPipeError::Generic(error_msg))
                    }
                }
                Err(e) => {
                    log_workflow_error!(
                        &execution.workflow_id,
                        execution_id,
                        "Failed to load workflow for HIL notification",
                        e
                    );
                    Err(e)
                }
            }
        } else {
            let error_msg = format!("Execution not found: {execution_id}");
            tracing::error!(
                execution_id = execution_id,
                error = %error_msg,
                "Execution not found"
            );
            Err(crate::workflow::errors::SwissPipeError::Generic(error_msg))
        }
    }

    /// Execute a single node from HIL resumption
    async fn execute_single_node(
        &self,
        execution_id: &str,
        node_id: &str,
        event: crate::workflow::models::WorkflowEvent,
    ) -> Result<()> {
        tracing::info!("Executing single node: {} for execution: {}", node_id, execution_id);

        // Fetch the workflow execution to get workflow_id
        if let Ok(Some(execution)) = crate::database::workflow_executions::Entity::find_by_id(execution_id)
            .one(&*self.db)
            .await
        {
            // Load the complete workflow using the workflow loader
            match self.workflow_engine.workflow_loader().load_workflow(&execution.workflow_id).await {
                Ok(workflow) => {
                    // Find the target node
                    if let Some(target_node) = workflow.nodes.iter().find(|n| n.id == node_id) {
                        // Execute the target node using the node executor with step tracking
                        match self.workflow_engine.node_executor().execute_node_with_output(
                            target_node,
                            event,
                            execution_id,
                        ).await {
                            Ok(_node_output) => {
                                tracing::info!("Successfully executed target node: {} for execution: {}", node_id, execution_id);

                                // Find connected nodes and potentially schedule them for execution
                                let connected_edges: Vec<_> = workflow.edges.iter()
                                    .filter(|edge| edge.from_node_id == node_id)
                                    .collect();

                                if !connected_edges.is_empty() {
                                    tracing::info!("Node {} has {} connected edges, may need to continue execution",
                                                  node_id, connected_edges.len());
                                    // For now, just log this - implementing full DAG continuation would require more complex logic
                                    // The node_output contains the event data that would be passed to the next nodes
                                }

                                // Check if this was the last HIL continuation job and update workflow status if needed
                                self.check_and_complete_hil_workflow(execution_id).await?;

                                Ok(())
                            }
                            Err(e) => {
                                log_workflow_error!(
                                    &execution.workflow_id,
                                    execution_id,
                                    node_id,
                                    "Failed to execute target node",
                                    e
                                );
                                Err(e)
                            }
                        }
                    } else {
                        let error_msg = format!("Target node {node_id} not found in workflow");
                        let err = crate::workflow::errors::SwissPipeError::Generic(error_msg.clone());
                        log_workflow_error!(
                            &execution.workflow_id,
                            execution_id,
                            node_id,
                            &error_msg,
                            err
                        );
                        Err(crate::workflow::errors::SwissPipeError::Generic(error_msg))
                    }
                }
                Err(e) => {
                    log_workflow_error!(
                        &execution.workflow_id,
                        execution_id,
                        "Failed to load workflow for single node execution",
                        e
                    );
                    Err(e)
                }
            }
        } else {
            let error_msg = format!("Execution not found: {execution_id}");
            tracing::error!(
                execution_id = execution_id,
                error = %error_msg,
                "Execution not found"
            );
            Err(crate::workflow::errors::SwissPipeError::Generic(error_msg))
        }
    }

    /// Check if all HIL continuation jobs are complete and update workflow status
    async fn check_and_complete_hil_workflow(&self, execution_id: &str) -> Result<()> {
        use crate::database::{workflow_executions, job_queue};
        use sea_orm::{EntityTrait, Set, ActiveModelTrait, ColumnTrait, QueryFilter};

        tracing::debug!("Checking if HIL workflow {} is complete", execution_id);

        // Check if there are any pending HIL continuation jobs for this execution
        let pending_hil_jobs = job_queue::Entity::find()
            .filter(job_queue::Column::ExecutionId.eq(execution_id))
            .filter(job_queue::Column::Status.eq("pending"))
            .all(&*self.db)
            .await
            .map_err(|e| {
                tracing::error!(
                    execution_id = execution_id,
                    error = %e,
                    "Failed to check pending HIL jobs for execution"
                );
                crate::workflow::errors::SwissPipeError::Generic(format!("Database error: {e}"))
            })?;

        // If there are still pending HIL jobs, don't update the status yet
        if !pending_hil_jobs.is_empty() {
            tracing::debug!("HIL workflow {} still has {} pending jobs, not completing yet",
                          execution_id, pending_hil_jobs.len());
            return Ok(());
        }

        // Get the current execution to check if it's in pending_human_input status
        let execution = workflow_executions::Entity::find_by_id(execution_id)
            .one(&*self.db)
            .await
            .map_err(|e| {
                tracing::error!(
                    execution_id = execution_id,
                    error = %e,
                    "Failed to fetch execution for HIL completion check"
                );
                crate::workflow::errors::SwissPipeError::Generic(format!("Database error: {e}"))
            })?
            .ok_or_else(|| {
                tracing::error!(
                    execution_id = execution_id,
                    "Execution not found for HIL completion check"
                );
                crate::workflow::errors::SwissPipeError::Generic(format!("Execution not found: {execution_id}"))
            })?;

        // Only update if the execution is currently in pending_human_input status
        if execution.status != "pending_human_input" {
            tracing::debug!("HIL workflow {} status is {} (not pending_human_input), no status update needed",
                          execution_id, execution.status);
            return Ok(());
        }

        // All HIL continuation jobs are complete, update the execution status to completed
        let workflow_id = execution.workflow_id.clone();
        let mut execution_active: workflow_executions::ActiveModel = execution.into();
        execution_active.status = Set("completed".to_string());
        execution_active.completed_at = Set(Some(chrono::Utc::now().timestamp_micros()));
        execution_active.updated_at = Set(chrono::Utc::now().timestamp_micros());

        execution_active.update(&*self.db).await
            .map_err(|e| {
                let err_msg = format!("Database error: {e}");
                log_workflow_error!(
                    &workflow_id,
                    execution_id,
                    "Failed to update HIL workflow status to completed",
                    crate::workflow::errors::SwissPipeError::Generic(err_msg.clone())
                );
                crate::workflow::errors::SwissPipeError::Generic(err_msg)
            })?;

        tracing::info!("HIL workflow {} completed - updated status from pending_human_input to completed", execution_id);
        Ok(())
    }

    /// Execute a regular workflow (non-HIL) with proper status tracking
    async fn execute_regular_workflow(&self, execution_id: &str) -> Result<()> {
        use crate::database::workflow_executions::{self, ExecutionStatus};
        use sea_orm::{EntityTrait, Set, ActiveModelTrait};

        tracing::info!("Starting workflow execution for: {}", execution_id);

        // Get the execution record to find the workflow_id and input data
        let execution = workflow_executions::Entity::find_by_id(execution_id)
            .one(&*self.db)
            .await
            .map_err(|e| crate::workflow::errors::SwissPipeError::Generic(format!("Failed to fetch execution: {e}")))?
            .ok_or_else(|| crate::workflow::errors::SwissPipeError::Generic(format!("Execution not found: {execution_id}")))?;

        // Update execution status to running
        let mut execution_active: workflow_executions::ActiveModel = execution.clone().into();
        execution_active.status = Set(ExecutionStatus::Running.to_string());
        execution_active.started_at = Set(Some(chrono::Utc::now().timestamp_micros()));
        execution_active.updated_at = Set(chrono::Utc::now().timestamp_micros());
        execution_active.update(&*self.db).await
            .map_err(|e| crate::workflow::errors::SwissPipeError::Generic(format!("Failed to update execution status to running: {e}")))?;

        tracing::info!("Updated execution {} status to running", execution_id);

        // Workflow execution is tracked in workflow_executions table
        // Individual node executions are tracked as steps in NodeExecutor

        // Load the workflow
        let workflow = self.workflow_engine.workflow_loader().load_workflow(&execution.workflow_id).await
            .map_err(|e| {
                log_workflow_error!(
                    &execution.workflow_id,
                    execution_id,
                    "Failed to load workflow",
                    e
                );
                e
            })?;

        // Parse input data from execution record
        let event = if let Some(input_data_str) = &execution.input_data {
            match serde_json::from_str::<serde_json::Value>(input_data_str) {
                Ok(input_json) => {
                    // Check if this is the new execution data format (with data, headers, metadata)
                    // or the direct user payload format
                    if input_json.is_object() && input_json.get("data").is_some() && input_json.get("headers").is_some() {
                        // New format: extract data, headers, and metadata from the stored execution data
                        let data = input_json.get("data").unwrap_or(&serde_json::json!({})).clone();
                        let headers = input_json.get("headers")
                            .and_then(|h| h.as_object())
                            .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect())
                            .unwrap_or_default();
                        let metadata = input_json.get("metadata")
                            .and_then(|m| m.as_object())
                            .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect())
                            .unwrap_or_default();

                        crate::workflow::models::WorkflowEvent {
                            data,
                            headers,
                            metadata,
                            condition_results: std::collections::HashMap::new(),
        hil_task: None,
        sources: Vec::new(),
                        }
                    } else {
                        // Legacy format: input_json is the direct user payload, use it as data
                        tracing::debug!("Using legacy data format for execution {}: treating input_json as direct payload", execution_id);
                        crate::workflow::models::WorkflowEvent {
                            data: input_json,
                            headers: std::collections::HashMap::new(),
                            metadata: std::collections::HashMap::new(),
                            condition_results: std::collections::HashMap::new(),
        hil_task: None,
        sources: Vec::new(),
                        }
                    }
                }
                Err(e) => {
                    let err = crate::workflow::errors::SwissPipeError::Generic(format!("Invalid input data: {e}"));
                    log_workflow_error!(
                        &execution.workflow_id,
                        execution_id,
                        "Failed to parse input data for execution",
                        err
                    );
                    return Err(crate::workflow::errors::SwissPipeError::Generic(format!("Invalid input data: {e}")));
                }
            }
        } else {
            // No input data, create empty event
            crate::workflow::models::WorkflowEvent {
                data: serde_json::json!({}),
                headers: std::collections::HashMap::new(),
                metadata: std::collections::HashMap::new(),
                condition_results: std::collections::HashMap::new(),
        hil_task: None,
        sources: Vec::new(),
            }
        };

        // Execute the workflow using the workflow engine
        tracing::info!("Executing workflow {} for execution {}", workflow.name, execution_id);
        let result = self.workflow_engine.execute_workflow(&workflow, event, execution_id).await;

        // Update execution status based on result
        let mut final_execution: workflow_executions::ActiveModel = {
            // Re-fetch execution to get latest state
            let current_execution = workflow_executions::Entity::find_by_id(execution_id)
                .one(&*self.db)
                .await
                .map_err(|e| crate::workflow::errors::SwissPipeError::Generic(format!("Failed to fetch execution for final update: {e}")))?
                .ok_or_else(|| crate::workflow::errors::SwissPipeError::Generic(format!("Execution not found for final update: {execution_id}")))?;
            current_execution.into()
        };

        let now = chrono::Utc::now().timestamp_micros();
        final_execution.completed_at = Set(Some(now));
        final_execution.updated_at = Set(now);

        let execution_success = match &result {
            Ok(output_event) => {
                // Check if workflow is blocked for human input
                let is_hil_blocked = output_event.data
                    .get("status")
                    .and_then(|s| s.as_str())
                    .map(|s| s == "pending_human_input")
                    .unwrap_or(false);

                if is_hil_blocked {
                    final_execution.status = Set(ExecutionStatus::PendingHumanInput.to_string());
                    final_execution.completed_at = Set(None); // Don't mark as completed for HIL
                    tracing::info!("Workflow execution blocked for human input: {}", execution_id);
                } else {
                    final_execution.status = Set(ExecutionStatus::Completed.to_string());
                    tracing::info!("Workflow execution completed successfully for: {}", execution_id);
                }

                final_execution.output_data = Set(Some(serde_json::to_string(&output_event.data).unwrap_or_else(|_| "{}".to_string())));
                final_execution.error_message = Set(None);

                // Workflow completion is tracked here in workflow_executions table
                // Individual node completions are tracked as steps in NodeExecutor
                tracing::info!("Workflow execution completed successfully");

                true
            }
            Err(e) => {
                final_execution.status = Set(ExecutionStatus::Failed.to_string());
                final_execution.error_message = Set(Some(e.to_string()));

                // Workflow failure is tracked here in workflow_executions table
                // Individual node failures are tracked as steps in NodeExecutor
                log_workflow_error!(
                    &execution.workflow_id,
                    execution_id,
                    "Workflow execution failed",
                    e
                );

                false
            }
        };

        // Save final execution state
        final_execution.update(&*self.db).await
            .map_err(|e| crate::workflow::errors::SwissPipeError::Generic(format!("Failed to update final execution status: {e}")))?;

        if execution_success {
            tracing::info!("Successfully completed workflow execution for: {}", execution_id);
            Ok(())
        } else {
            Err(crate::workflow::errors::SwissPipeError::Generic("Workflow execution failed".to_string()))
        }
    }

    /// Update worker status (helper method)
    async fn update_worker_status(&self, worker_id: &str, status: WorkerStatus, current_job: Option<String>) {
        let mut workers = self.workers.write().await;
        if let Some(worker) = workers.iter_mut().find(|w| w.id == worker_id) {
            worker.status = status;
            worker.current_job = current_job;
            worker.last_activity = chrono::Utc::now();
            if worker.status == WorkerStatus::Idle {
                worker.processed_count += 1;
            }
        }
    }

    /// Recover crashed jobs (similar to original but adapted for MPSC)
    async fn recover_crashed_jobs(&self) -> Result<()> {
        // Recovery logic would be similar to original WorkerPool
        // For now, rely on MPSC distributor's job claim timeout handling
        tracing::info!("MPSC job recovery completed (handled by distributor timeout processing)");
        Ok(())
    }

    /// Stop the MPSC worker pool gracefully
    pub async fn stop(&self) -> Result<()> {
        tracing::info!("Stopping MPSC worker pool...");

        // Signal workers to stop
        self.is_running.store(false, Ordering::SeqCst);

        // Shutdown MPSC distributor
        self.mpsc_distributor.shutdown().await?;

        // Wait for workers to finish
        let mut workers = self.workers.write().await;
        for worker in workers.iter_mut() {
            if let Some(handle) = worker.handle.take() {
                match tokio::time::timeout(Duration::from_secs(30), handle).await {
                    Ok(Ok(())) => {
                        tracing::debug!("MPSC worker {} stopped gracefully", worker.id);
                    }
                    Ok(Err(e)) => {
                        tracing::error!(
                            worker_id = %worker.id,
                            error = %e,
                            "MPSC worker stopped with error"
                        );
                    }
                    Err(_) => {
                        tracing::warn!(
                            worker_id = %worker.id,
                            "MPSC worker shutdown timed out"
                        );
                    }
                }
                worker.status = WorkerStatus::Stopped;
            }
        }

        tracing::info!("MPSC worker pool stopped successfully");
        Ok(())
    }

    /// Shutdown the MPSC worker pool gracefully (alias for stop method)
    pub async fn shutdown(&self) -> Result<()> {
        self.stop().await
    }

    /// Cancel execution with delay cleanup (compatible with WorkerPool interface)
    pub async fn cancel_execution_with_delays(&self, execution_id: &str) -> Result<()> {
        tracing::info!("Starting comprehensive MPSC execution cancellation with delays for: {}", execution_id);

        // First, cancel via execution service (jobs, steps, execution status)
        self.execution_service.cancel_execution(execution_id).await?;

        // Then, cancel any scheduled delays
        let delay_scheduler = self.delay_scheduler.read().await;
        if let Some(scheduler) = delay_scheduler.as_ref() {
            match scheduler.cancel_delays_for_execution(execution_id).await {
                Ok(cancelled_count) => {
                    if cancelled_count > 0 {
                        tracing::info!("MPSC cancelled {} scheduled delays for execution {}", cancelled_count, execution_id);
                    }
                }
                Err(e) => {
                    tracing::error!(
                        execution_id = execution_id,
                        error = %e,
                        "Failed to cancel delays for MPSC execution"
                    );
                    // Don't fail the entire cancellation if delay cancellation fails
                }
            }
        } else {
            tracing::warn!(
                execution_id = execution_id,
                "DelayScheduler not available for cancelling delays in MPSC execution"
            );
        }

        tracing::info!("Completed comprehensive MPSC execution cancellation for: {}", execution_id);
        Ok(())
    }

    /// Get worker pool statistics
    pub async fn get_stats(&self) -> MpscWorkerPoolStats {
        let workers = self.workers.read().await;
        let mpsc_metrics = self.mpsc_distributor.get_metrics().await;

        MpscWorkerPoolStats {
            total_workers: workers.len(),
            idle_workers: workers.iter().filter(|w| w.status == WorkerStatus::Idle).count(),
            busy_workers: workers.iter().filter(|w| w.status == WorkerStatus::Busy).count(),
            total_processed_jobs: self.processed_jobs.load(Ordering::SeqCst),
            mpsc_jobs_distributed: mpsc_metrics.jobs_distributed,
            mpsc_jobs_failed: mpsc_metrics.jobs_failed,
            mpsc_channel_full_events: mpsc_metrics.channel_full_events,
            mpsc_active_jobs: mpsc_metrics.active_jobs_in_channel,
        }
    }
}

/// Statistics for MPSC worker pool monitoring
#[derive(Debug, serde::Serialize)]
pub struct MpscWorkerPoolStats {
    pub total_workers: usize,
    pub idle_workers: usize,
    pub busy_workers: usize,
    pub total_processed_jobs: u64,
    pub mpsc_jobs_distributed: u64,
    pub mpsc_jobs_failed: u64,
    pub mpsc_channel_full_events: u64,
    pub mpsc_active_jobs: u64,
}