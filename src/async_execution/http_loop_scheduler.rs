use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use chrono::Utc;
use sea_orm::{entity::Set, ActiveModelTrait, EntityTrait};
use serde_json;

use crate::database::http_loop_states;
use crate::database::http_loop_states::{LoopStatus, LoopTerminationReason, IterationResult};
use crate::workflow::errors::{SwissPipeError, Result};
use crate::workflow::models::{WorkflowEvent, LoopConfig, BackoffStrategy, TerminationCondition, TerminationAction};
use crate::utils::{http_client::AppExecutor, javascript::JavaScriptExecutor};
use crate::workflow::models::{HttpMethod, RetryConfig};
use sea_orm::DatabaseConnection;
use crate::{log_workflow_error, log_workflow_warn};

// Constants for configuration limits and defaults
#[allow(dead_code)]
const MAX_RESPONSE_TRUNCATE_LENGTH: usize = 1000;

pub struct HttpLoopScheduler {
    db: Arc<DatabaseConnection>,
    app_executor: Arc<AppExecutor>,
    js_executor: Arc<JavaScriptExecutor>,
    // Track running loop tasks
    loop_tasks: Arc<RwLock<HashMap<String, tokio::task::JoinHandle<()>>>>,
    config: crate::config::HttpLoopConfig,
}

#[derive(Debug, Clone)]
pub struct HttpLoopConfig {
    pub loop_id: String,
    pub execution_step_id: String,
    pub url: String,
    pub method: HttpMethod,
    pub timeout_seconds: u64,
    pub headers: HashMap<String, String>,
    pub loop_config: LoopConfig,
    pub initial_event: WorkflowEvent,
}


#[derive(Debug, Clone)]
pub struct HttpLoopResponse {
    pub workflow_event: WorkflowEvent,
    pub status_code: u16,
    pub success: bool,
    pub error_message: Option<String>,
}

impl HttpLoopScheduler {
    /// Parse execution_id from execution_step_id (format: "{execution_id}_{node_id}")
    fn parse_execution_id(execution_step_id: &str) -> &str {
        if let Some(underscore_pos) = execution_step_id.find('_') {
            &execution_step_id[..underscore_pos]
        } else {
            execution_step_id
        }
    }

    /// Parse node_id from execution_step_id (format: "{execution_id}_{node_id}")
    fn parse_node_id(execution_step_id: &str) -> &str {
        if let Some(underscore_pos) = execution_step_id.find('_') {
            &execution_step_id[underscore_pos + 1..]
        } else {
            "unknown"
        }
    }

    /// Get workflow_id from execution_step_id by querying the database
    async fn get_workflow_id_from_step(db: &DatabaseConnection, execution_step_id: &str) -> Option<String> {
        use crate::database::workflow_executions;
        use sea_orm::EntityTrait;

        // First get the execution step to find execution_id
        let execution_id = Self::parse_execution_id(execution_step_id);

        // Then get the workflow execution to find workflow_id
        match workflow_executions::Entity::find_by_id(execution_id)
            .one(db)
            .await
        {
            Ok(Some(execution)) => Some(execution.workflow_id),
            _ => None,
        }
    }

    pub async fn new(db: Arc<DatabaseConnection>, config: crate::config::HttpLoopConfig) -> Result<Self> {
        tracing::info!("Creating HttpLoopScheduler...");

        let js_executor = Arc::new(JavaScriptExecutor::new()?);

        let scheduler = Self {
            db,
            app_executor: Arc::new(AppExecutor::new()),
            js_executor,
            loop_tasks: Arc::new(RwLock::new(HashMap::new())),
            config,
        };

        tracing::info!("HttpLoopScheduler initialized with config: {:?}", scheduler.config);
        Ok(scheduler)
    }

    /// Start the background scheduler service that processes ready loops
    pub async fn start_scheduler_service(&self) -> Result<()> {
        tracing::info!("Starting HTTP loop scheduler service...");

        let db = Arc::clone(&self.db);
        let app_executor = Arc::clone(&self.app_executor);
        let js_executor = Arc::clone(&self.js_executor);
        let loop_tasks = Arc::clone(&self.loop_tasks);
        let scheduler_interval = self.config.scheduler_interval_seconds;
        let scheduler_config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(scheduler_interval));
            tracing::info!("HTTP loop scheduler service started (checking every {} seconds)", scheduler_interval);

            loop {
                interval.tick().await;

                if let Err(e) = Self::process_ready_loops(
                    db.clone(),
                    app_executor.clone(),
                    js_executor.clone(),
                    loop_tasks.clone(),
                    scheduler_config.clone(),
                ).await {
                    // This is a system-level error without specific workflow context
                    tracing::error!("Error processing ready loops: {}", e);
                }
            }
        });

        Ok(())
    }

    /// Find and process loops that are ready for execution
    async fn process_ready_loops(
        db: Arc<DatabaseConnection>,
        app_executor: Arc<AppExecutor>,
        js_executor: Arc<JavaScriptExecutor>,
        loop_tasks: Arc<RwLock<HashMap<String, tokio::task::JoinHandle<()>>>>,
        scheduler_config: crate::config::HttpLoopConfig,
    ) -> Result<()> {
        use sea_orm::{EntityTrait, QueryFilter, ColumnTrait};

        let now = chrono::Utc::now().timestamp_micros();

        // Find loops that are ready for execution
        let ready_loops = http_loop_states::Entity::find()
            .filter(http_loop_states::Column::Status.eq("running"))
            .filter(http_loop_states::Column::NextExecutionAt.lte(now))
            .filter(http_loop_states::Column::NextExecutionAt.is_not_null())
            .all(db.as_ref())
            .await
            .map_err(|e| SwissPipeError::Generic(format!("Failed to query ready loops: {e}")))?;

        if ready_loops.is_empty() {
            tracing::trace!("No loops ready for execution");
            return Ok(());
        }

        tracing::info!("Found {} loops ready for execution", ready_loops.len());

        for loop_state in ready_loops {
            let loop_id = loop_state.id.clone();

            // Create loop configuration from database state
            let loop_config = match Self::create_loop_config_from_state(&loop_state) {
                Ok(config) => config,
                Err(e) => {
                    // Get workflow context for logging
                    let execution_id = Self::parse_execution_id(&loop_state.execution_step_id);
                    let node_id = Self::parse_node_id(&loop_state.execution_step_id);
                    if let Some(workflow_id) = Self::get_workflow_id_from_step(db.as_ref(), &loop_state.execution_step_id).await {
                        log_workflow_error!(&workflow_id, execution_id, node_id,
                            format!("Failed to create loop config for {}", loop_id), e);
                    } else {
                        tracing::error!("Failed to create loop config for {}: {}", loop_id, e);
                    }
                    continue;
                }
            };

            // Atomically check-and-insert task to prevent race conditions
            {
                let mut tasks = loop_tasks.write().await;

                // Check if this loop already has a running task
                if let Some(existing_task) = tasks.get(&loop_id) {
                    if !existing_task.is_finished() {
                        tracing::debug!("Loop {} already has a running task, skipping", loop_id);
                        continue;
                    }
                    // Task is finished, just drop it to free resources
                    let _ = existing_task;
                    tracing::debug!("Cleaned up finished task for loop: {}", loop_id);
                }

                // Spawn new task for this loop
                let db_clone = db.clone();
                let app_executor_clone = app_executor.clone();
                let js_executor_clone = js_executor.clone();
                let loop_id_clone = loop_id.clone();
                let scheduler_config_clone = scheduler_config.clone();
                let execution_step_id_clone = loop_config.execution_step_id.clone();
                let task = tokio::spawn(async move {
                    if let Err(e) = Self::execute_loop_iteration(db_clone.clone(), app_executor_clone, js_executor_clone, loop_config, scheduler_config_clone).await {
                        // Get workflow context for logging
                        let execution_id = Self::parse_execution_id(&execution_step_id_clone);
                        let node_id = Self::parse_node_id(&execution_step_id_clone);
                        if let Some(workflow_id) = Self::get_workflow_id_from_step(db_clone.as_ref(), &execution_step_id_clone).await {
                            log_workflow_error!(&workflow_id, execution_id, node_id,
                                format!("HTTP loop iteration failed for {}", loop_id_clone), e);
                        } else {
                            tracing::error!("HTTP loop iteration failed for {}: {}", loop_id_clone, e);
                        }
                    }
                });

                // Store the new task handle
                tasks.insert(loop_id.clone(), task);
                tracing::debug!("Spawned execution task for loop: {}", loop_id);
            }
        }

        // Clean up finished tasks
        Self::cleanup_finished_tasks(loop_tasks).await;

        Ok(())
    }

    /// Create HttpLoopConfig from database state for resumption
    fn create_loop_config_from_state(loop_state: &http_loop_states::Model) -> Result<HttpLoopConfig> {
        // Deserialize stored configuration
        let headers: std::collections::HashMap<String, String> =
            serde_json::from_str(&loop_state.headers)
            .map_err(|e| SwissPipeError::Generic(format!("Failed to deserialize headers: {e}")))?;

        let loop_config: crate::workflow::models::LoopConfig =
            serde_json::from_str(&loop_state.loop_configuration)
            .map_err(|e| SwissPipeError::Generic(format!("Failed to deserialize loop config: {e}")))?;

        let initial_event: crate::workflow::models::WorkflowEvent =
            serde_json::from_str(&loop_state.initial_event)
            .map_err(|e| SwissPipeError::Generic(format!("Failed to deserialize initial event: {e}")))?;

        // Parse HTTP method from string
        let method = match loop_state.method.as_str() {
            "GET" => crate::workflow::models::HttpMethod::Get,
            "POST" => crate::workflow::models::HttpMethod::Post,
            "PUT" => crate::workflow::models::HttpMethod::Put,
            "DELETE" => crate::workflow::models::HttpMethod::Delete,
            "PATCH" => crate::workflow::models::HttpMethod::Patch,
            _ => {
                // This is a system-level validation warning, no workflow context needed
                tracing::warn!("Unknown HTTP method '{}', defaulting to GET", loop_state.method);
                crate::workflow::models::HttpMethod::Get
            }
        };

        Ok(HttpLoopConfig {
            loop_id: loop_state.id.clone(),
            execution_step_id: loop_state.execution_step_id.clone(),
            url: loop_state.url.clone(),
            method,
            timeout_seconds: loop_state.timeout_seconds as u64,
            headers,
            loop_config,
            initial_event,
        })
    }

    /// Clean up finished task handles to prevent memory leaks
    async fn cleanup_finished_tasks(loop_tasks: Arc<RwLock<HashMap<String, tokio::task::JoinHandle<()>>>>) {
        let mut tasks = loop_tasks.write().await;
        let mut finished_tasks = Vec::new();

        // Collect finished task handles and their IDs
        for (loop_id, task) in tasks.iter() {
            if task.is_finished() {
                finished_tasks.push(loop_id.clone());
            }
        }

        // Remove finished tasks and properly clean them up
        for loop_id in finished_tasks {
            if let Some(task) = tasks.remove(&loop_id) {
                // Task is already finished, no need to abort
                // Just remove it from tracking to free memory
                drop(task); // Explicitly drop the handle
                tracing::debug!("Cleaned up finished task for loop: {}", loop_id);
            }
        }
    }

    /// Resume interrupted HTTP loops on application startup
    pub async fn resume_interrupted_loops(&self) -> Result<usize> {
        tracing::info!("Resuming interrupted HTTP loops...");

        use sea_orm::{EntityTrait, QueryFilter, ColumnTrait};

        // Find all running loops (interrupted when application stopped)
        let interrupted_loops = http_loop_states::Entity::find()
            .filter(http_loop_states::Column::Status.eq(LoopStatus::Running.to_string()))
            .all(self.db.as_ref())
            .await
            .map_err(|e| SwissPipeError::Generic(format!("Failed to query interrupted loops: {e}")))?;

        if interrupted_loops.is_empty() {
            tracing::info!("No interrupted HTTP loops found");
            return Ok(0);
        }

        let count = interrupted_loops.len();
        tracing::info!("Found {} interrupted HTTP loops to resume", count);

        for loop_state in interrupted_loops {
            let loop_id = loop_state.id.clone();

            // Create minimal loop configuration for resumption
            let _loop_config = match Self::create_loop_config_from_state(&loop_state) {
                Ok(config) => config,
                Err(e) => {
                    // Get workflow context for logging
                    let execution_id = Self::parse_execution_id(&loop_state.execution_step_id);
                    let node_id = Self::parse_node_id(&loop_state.execution_step_id);
                    if let Some(workflow_id) = Self::get_workflow_id_from_step(self.db.as_ref(), &loop_state.execution_step_id).await {
                        log_workflow_error!(&workflow_id, execution_id, node_id,
                            format!("Failed to create config for loop {}, skipping", loop_id), e);
                    } else {
                        tracing::error!("Failed to create config for loop {}: {}, skipping", loop_id, e);
                    }
                    continue;
                }
            };

            // Set next execution time to now (resume immediately)
            if let Err(e) = self.update_next_execution_time(&loop_id, chrono::Utc::now()).await {
                // Get workflow context for logging
                let execution_id = Self::parse_execution_id(&loop_state.execution_step_id);
                let node_id = Self::parse_node_id(&loop_state.execution_step_id);
                if let Some(workflow_id) = Self::get_workflow_id_from_step(self.db.as_ref(), &loop_state.execution_step_id).await {
                    log_workflow_error!(&workflow_id, execution_id, node_id,
                        format!("Failed to update next execution time for loop {}", loop_id), e);
                } else {
                    tracing::error!("Failed to update next execution time for loop {}: {}", loop_id, e);
                }
                continue;
            }

            tracing::info!("Resumed HTTP loop: {}", loop_id);
        }

        tracing::info!("Successfully resumed {} HTTP loops", count);
        Ok(count)
    }

    /// Update the next execution time for a loop
    async fn update_next_execution_time(&self, loop_id: &str, next_time: chrono::DateTime<chrono::Utc>) -> Result<()> {
        use sea_orm::{EntityTrait, Set, ActiveModelTrait};

        let loop_state = http_loop_states::Entity::find_by_id(loop_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| SwissPipeError::Generic(format!("Failed to find loop {loop_id}: {e}")))?
            .ok_or_else(|| SwissPipeError::Generic(format!("Loop not found: {loop_id}")))?;

        let mut active_model: http_loop_states::ActiveModel = loop_state.into();
        active_model.next_execution_at = Set(Some(next_time.timestamp_micros()));

        active_model.update(self.db.as_ref()).await
            .map_err(|e| SwissPipeError::Generic(format!("Failed to update loop next execution time: {e}")))?;

        Ok(())
    }

    /// Schedule a new HTTP loop for execution
    pub async fn schedule_http_loop(&self, config: HttpLoopConfig) -> Result<String> {
        let loop_id = config.loop_id.clone();

        tracing::info!("Scheduling HTTP loop: {}", loop_id);
        tracing::debug!("Loop config details: max_iterations={:?}, interval_seconds={}, termination_condition_present={}",
            config.loop_config.max_iterations, config.loop_config.interval_seconds,
            config.loop_config.termination_condition.is_some());

        // Validate configuration before scheduling
        Self::validate_loop_config(&config)?;

        // Concurrent limit will be enforced atomically during insertion

        // Serialize configuration for persistence
        let headers_json = serde_json::to_string(&config.headers)
            .map_err(|e| SwissPipeError::Generic(format!("Failed to serialize headers: {e}")))?;

        let loop_config_json = serde_json::to_string(&config.loop_config)
            .map_err(|e| SwissPipeError::Generic(format!("Failed to serialize loop config: {e}")))?;

        let initial_event_json = serde_json::to_string(&config.initial_event)
            .map_err(|e| SwissPipeError::Generic(format!("Failed to serialize initial event: {e}")))?;

        // Create initial loop state in database with full configuration
        let loop_state = http_loop_states::ActiveModel {
            id: Set(loop_id.clone()),
            execution_step_id: Set(config.execution_step_id.clone()),
            current_iteration: Set(0),
            max_iterations: Set(config.loop_config.max_iterations.map(|i| i as i32)),
            next_execution_at: Set(Some(Utc::now().timestamp_micros())), // Start immediately
            consecutive_failures: Set(0),
            loop_started_at: Set(Utc::now().timestamp_micros()),
            last_response_status: Set(None),
            last_response_body: Set(None),
            iteration_history: Set("[]".to_string()),
            status: Set(LoopStatus::Running.to_string()),
            termination_reason: Set(None),
            // Store configuration for proper resumption
            url: Set(config.url.clone()),
            method: Set(config.method.to_string()),
            timeout_seconds: Set(config.timeout_seconds as i64),
            headers: Set(headers_json),
            loop_configuration: Set(loop_config_json),
            initial_event: Set(initial_event_json),
            ..Default::default()
        };

        // Insert loop state directly without limits
        self.insert_loop_state(loop_state).await?;

        // Atomically create and store task handle to prevent race conditions
        let _task = {
            let mut tasks = self.loop_tasks.write().await;

            // Check if task already exists (shouldn't happen, but safety check)
            if tasks.contains_key(&loop_id) {
                return Err(SwissPipeError::Generic(format!("Loop task already exists: {loop_id}")));
            }

            // Spawn the task and immediately store its handle
            let task = self.spawn_loop_task(config).await;
            tasks.insert(loop_id.clone(), task);

            // Return a clone of the handle for logging purposes
            tasks.get(&loop_id).unwrap().is_finished() // Just to verify it's there
        };

        tracing::info!("HTTP loop scheduled successfully: {}", loop_id);
        Ok(loop_id)
    }

    /// Get the status of a running loop
    pub async fn get_loop_status(&self, loop_id: &str) -> Result<http_loop_states::Model> {
        let loop_state = http_loop_states::Entity::find_by_id(loop_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| SwissPipeError::Generic(format!("Failed to query loop state: {e}")))?
            .ok_or_else(|| SwissPipeError::Generic(format!("Loop not found: {loop_id}")))?;

        Ok(loop_state)
    }

    /// Wait for HTTP loop to complete and return final result
    pub async fn wait_for_loop_completion(&self, loop_id: &str) -> Result<WorkflowEvent> {
        use tokio::time::{sleep, Duration};
        use std::str::FromStr;
        use crate::database::http_loop_states::LoopStatus;

        const POLL_INTERVAL_MS: u64 = 100; // Poll every 100ms
        const MAX_WAIT_SECONDS: u64 = 3600; // Maximum 1 hour wait
        let max_polls = (MAX_WAIT_SECONDS * 1000) / POLL_INTERVAL_MS;

        tracing::info!("Waiting for HTTP loop completion: {}", loop_id);

        // Log initial state immediately after scheduling
        if let Ok(initial_state) = self.get_loop_status(loop_id).await {
            tracing::debug!("Initial loop state: status={}, iteration={}, termination_reason={:?}",
                initial_state.status, initial_state.current_iteration, initial_state.termination_reason);
        }

        for poll_count in 0..max_polls {
            let loop_state = self.get_loop_status(loop_id).await?;
            let status = LoopStatus::from_str(&loop_state.status).unwrap_or(LoopStatus::Running);

            // Log at DEBUG level for regular polls, INFO only for significant events
            if poll_count == 0 || poll_count % 50 == 0 { // Log first poll and every 5 seconds
                tracing::info!("Loop {} poll #{}: status={:?}, iteration={}, termination_reason={:?}",
                    loop_id, poll_count, status, loop_state.current_iteration, loop_state.termination_reason);
            } else {
                tracing::debug!("Loop {} poll #{}: status={:?}, iteration={}, termination_reason={:?}",
                    loop_id, poll_count, status, loop_state.current_iteration, loop_state.termination_reason);
            }

            // Add detailed logging for immediate completion
            if poll_count == 0 && matches!(status, LoopStatus::Completed | LoopStatus::Failed | LoopStatus::Cancelled) {
                // This is a system-level race condition warning
                tracing::warn!("Loop {} completed immediately after scheduling - possible race condition! Status: {:?}, Iteration: {}, Reason: {:?}",
                    loop_id, status, loop_state.current_iteration, loop_state.termination_reason);
            }

            match status {
                LoopStatus::Completed => {
                    tracing::info!("HTTP loop completed successfully: {}", loop_id);

                    // Return the final output data from last response body if available
                    if let Some(response_body_json) = &loop_state.last_response_body {
                        let final_data: serde_json::Value = serde_json::from_str(response_body_json)
                            .map_err(|e| SwissPipeError::Generic(format!("Failed to parse final response body: {e}")))?;

                        return Ok(WorkflowEvent {
                            data: final_data,
                            metadata: std::collections::HashMap::new(),
                            headers: std::collections::HashMap::new(),
                            condition_results: std::collections::HashMap::new(),
        hil_task: None,
                        });
                    } else {
                        // Return success event without specific data
                        return Ok(WorkflowEvent {
                            data: serde_json::json!({"loop_completed": true, "status": "success"}),
                            metadata: std::collections::HashMap::new(),
                            headers: std::collections::HashMap::new(),
                            condition_results: std::collections::HashMap::new(),
        hil_task: None,
                        });
                    }
                }
                LoopStatus::Failed => {
                    // Get workflow context for logging
                    let execution_id = Self::parse_execution_id(&loop_state.execution_step_id);
                    let node_id = Self::parse_node_id(&loop_state.execution_step_id);
                    if let Some(workflow_id) = Self::get_workflow_id_from_step(self.db.as_ref(), &loop_state.execution_step_id).await {
                        log_workflow_warn!(&workflow_id, execution_id, node_id,
                            format!("HTTP loop failed: {}", loop_id));
                    } else {
                        tracing::warn!("HTTP loop failed: {}", loop_id);
                    }
                    return Err(SwissPipeError::Generic(format!("HTTP loop failed: {loop_id}")));
                }
                LoopStatus::Cancelled => {
                    // Get workflow context for logging
                    let execution_id = Self::parse_execution_id(&loop_state.execution_step_id);
                    let node_id = Self::parse_node_id(&loop_state.execution_step_id);
                    if let Some(workflow_id) = Self::get_workflow_id_from_step(self.db.as_ref(), &loop_state.execution_step_id).await {
                        log_workflow_warn!(&workflow_id, execution_id, node_id,
                            format!("HTTP loop was cancelled: {}", loop_id));
                    } else {
                        tracing::warn!("HTTP loop was cancelled: {}", loop_id);
                    }
                    return Err(SwissPipeError::Generic(format!("HTTP loop was cancelled: {loop_id}")));
                }
                LoopStatus::Running | LoopStatus::Paused => {
                    // Still running, continue polling
                    if poll_count % 50 == 0 && poll_count > 0 { // Log every 5 seconds
                        tracing::debug!("HTTP loop still running: {} ({}s elapsed)",
                            loop_id, (poll_count * POLL_INTERVAL_MS) / 1000);
                    }
                    sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;
                }
            }
        }

        Err(SwissPipeError::Generic(format!(
            "HTTP loop wait timeout after {MAX_WAIT_SECONDS} seconds: {loop_id}"
        )))
    }

    /// Pause a running loop
    pub async fn pause_loop(&self, loop_id: &str) -> Result<()> {
        tracing::info!("Pausing loop: {}", loop_id);

        Self::safe_update_loop_state(self.db.as_ref(), loop_id, |active_model| {
            let current_status_str = match &active_model.status {
                Set(status) => status.as_str(),
                _ => "running",
            };
            let current_status = current_status_str.parse::<LoopStatus>().unwrap_or(LoopStatus::Running);

            // Validate state transition
            Self::validate_state_transition(&current_status, &LoopStatus::Paused)?;

            active_model.status = Set(LoopStatus::Paused.to_string());
            active_model.next_execution_at = Set(None); // Clear next execution time

            Ok(())
        }).await?;

        tracing::info!("Successfully paused loop: {}", loop_id);
        Ok(())
    }

    /// Resume a paused loop
    pub async fn resume_loop(&self, loop_id: &str) -> Result<()> {
        tracing::info!("Resuming loop: {}", loop_id);

        Self::safe_update_loop_state(self.db.as_ref(), loop_id, |active_model| {
            let current_status_str = match &active_model.status {
                Set(status) => status.as_str(),
                _ => "failed",
            };
            let current_status = current_status_str.parse::<LoopStatus>().unwrap_or(LoopStatus::Failed);

            // Validate state transition
            Self::validate_state_transition(&current_status, &LoopStatus::Running)?;

            active_model.status = Set(LoopStatus::Running.to_string());

            // Schedule immediate execution
            let next_execution = chrono::Utc::now().timestamp_micros() + 1_000; // 1ms from now
            active_model.next_execution_at = Set(Some(next_execution));

            Ok(())
        }).await?;

        tracing::info!("Successfully resumed loop: {}", loop_id);
        Ok(())
    }

    /// Cancel a loop and its associated workflow execution
    pub async fn cancel_loop(&self, loop_id: &str) -> Result<()> {
        use sea_orm::{EntityTrait, Set, ActiveModelTrait, TransactionTrait};

        tracing::info!("Cancelling loop: {}", loop_id);

        let txn = self.db.begin().await
            .map_err(|e| SwissPipeError::Generic(format!("Failed to start transaction: {e}")))?;

        let loop_state = http_loop_states::Entity::find_by_id(loop_id)
            .one(&txn)
            .await
            .map_err(|e| SwissPipeError::Generic(format!("Failed to query loop state: {e}")))?
            .ok_or_else(|| SwissPipeError::Generic(format!("Loop not found: {loop_id}")))?;

        // Validate state transition to cancelled
        let current_status = loop_state.status.parse::<LoopStatus>()
            .map_err(|e| SwissPipeError::Generic(format!("Invalid current status: {e}")))?;
        Self::validate_state_transition(&current_status, &LoopStatus::Cancelled)?;

        // Update loop state to cancelled
        let mut loop_active_model: http_loop_states::ActiveModel = loop_state.clone().into();
        loop_active_model.status = Set(LoopStatus::Cancelled.to_string());
        loop_active_model.termination_reason = Set(Some(LoopTerminationReason::Stopped.to_string()));
        loop_active_model.next_execution_at = Set(None);

        loop_active_model.update(&txn).await
            .map_err(|e| SwissPipeError::Generic(format!("Failed to cancel loop: {e}")))?;

        // Cancel the associated workflow execution
        let execution_step_id = &loop_state.execution_step_id;
        let execution_id = if let Some(underscore_pos) = execution_step_id.find('_') {
            &execution_step_id[..underscore_pos]
        } else {
            execution_step_id
        };

        // Cancel workflow execution and step (using same logic as terminate_loop for stopped workflows)
        use crate::database::{workflow_executions, workflow_execution_steps, workflow_executions::ExecutionStatus, workflow_execution_steps::StepStatus};

        // Update the workflow execution to cancelled
        let execution = workflow_executions::Entity::find_by_id(execution_id)
            .one(&txn)
            .await?;

        if let Some(exec_model) = execution {
            let mut exec_active: workflow_executions::ActiveModel = exec_model.into();
            exec_active.status = Set(ExecutionStatus::Cancelled.to_string());
            exec_active.completed_at = Set(Some(chrono::Utc::now().timestamp_micros()));
            exec_active.update(&txn).await?;
            tracing::info!("Cancelled workflow execution {} due to loop cancellation", execution_id);
        }

        // Update the execution step to cancelled as well
        let execution_step = workflow_execution_steps::Entity::find_by_id(execution_step_id)
            .one(&txn)
            .await?;

        if let Some(step_model) = execution_step {
            let mut step_active: workflow_execution_steps::ActiveModel = step_model.into();
            step_active.status = Set(StepStatus::Cancelled.to_string());
            step_active.completed_at = Set(Some(chrono::Utc::now().timestamp_micros()));
            step_active.update(&txn).await?;
            tracing::debug!("Updated execution step {} to cancelled", execution_step_id);
        }

        txn.commit().await
            .map_err(|e| SwissPipeError::Generic(format!("Failed to commit cancellation: {e}")))?;

        tracing::info!("Successfully cancelled loop: {}", loop_id);
        Ok(())
    }

    /// Spawn an async task to execute the HTTP loop
    async fn spawn_loop_task(&self, config: HttpLoopConfig) -> tokio::task::JoinHandle<()> {
        let db = Arc::clone(&self.db);
        let app_executor = Arc::clone(&self.app_executor);
        let js_executor = Arc::clone(&self.js_executor);
        let scheduler_config = self.config.clone();
        let execution_step_id = config.execution_step_id.clone();

        tokio::spawn(async move {
            if let Err(e) = Self::execute_loop_iteration(db.clone(), app_executor, js_executor, config, scheduler_config).await {
                // Get workflow context for logging
                let execution_id = Self::parse_execution_id(&execution_step_id);
                let node_id = Self::parse_node_id(&execution_step_id);
                if let Some(workflow_id) = Self::get_workflow_id_from_step(db.as_ref(), &execution_step_id).await {
                    log_workflow_error!(&workflow_id, execution_id, node_id,
                        "HTTP loop iteration failed".to_string(), e);
                } else {
                    tracing::error!("HTTP loop iteration failed: {}", e);
                }
            }
        })
    }

    /// Execute a single iteration of an HTTP loop (called by background scheduler)
    async fn execute_loop_iteration(
        db: Arc<DatabaseConnection>,
        app_executor: Arc<AppExecutor>,
        js_executor: Arc<JavaScriptExecutor>,
        config: HttpLoopConfig,
        scheduler_config: crate::config::HttpLoopConfig,
    ) -> Result<()> {
        use tokio::time::{timeout, Duration};

        // Wrap the entire iteration in a timeout
        let timeout_duration = Duration::from_secs(scheduler_config.max_iteration_timeout_seconds);

        timeout(timeout_duration, Self::execute_loop_iteration_internal(
            db, app_executor, js_executor, config, scheduler_config
        ))
        .await
        .map_err(|_| SwissPipeError::Generic("HTTP loop iteration timed out".to_string()))?
    }

    /// Internal iteration execution without timeout wrapper
    async fn execute_loop_iteration_internal(
        db: Arc<DatabaseConnection>,
        app_executor: Arc<AppExecutor>,
        js_executor: Arc<JavaScriptExecutor>,
        config: HttpLoopConfig,
        scheduler_config: crate::config::HttpLoopConfig,
    ) -> Result<()> {
        let loop_id = &config.loop_id;

        tracing::debug!("Executing single HTTP loop iteration: {}", loop_id);

        // Pre-execution checks (read-only, no transaction needed)
        let loop_state = http_loop_states::Entity::find_by_id(loop_id)
            .one(db.as_ref())
            .await?
            .ok_or_else(|| SwissPipeError::Generic(format!("Loop state not found: {loop_id}")))?;

        // Check loop status - only execute if running
        match loop_state.status.as_str() {
            "running" => {
                // Continue with execution
            }
            status => {
                tracing::debug!("Loop {} is in {} state, skipping iteration", loop_id, status);
                return Ok(());
            }
        }

        // Note: Max iterations check moved to after HTTP request execution to preserve final response

            // Execute HTTP request
            let iteration_start = Utc::now();
            let iteration_num = loop_state.current_iteration + 1;

            tracing::debug!("Executing HTTP request for loop {} iteration {}", loop_id, iteration_num);

            let request_result = Self::execute_http_request(
                &app_executor,
                &config,
                &config.initial_event,
                &scheduler_config
            ).await;

            // Process the result with comprehensive response data
            let (success, response_status, response_body, error_msg, workflow_event) = match &request_result {
                Ok(http_response) => {
                    let body_snippet = serde_json::to_string(&http_response.workflow_event.data)
                        .unwrap_or_default();
                    let snippet = if body_snippet.len() > 1000 {
                        format!("{}...", &body_snippet[..1000])
                    } else {
                        body_snippet
                    };

                    tracing::info!(
                        "HTTP loop {} iteration {} completed: status={}, success={}",
                        loop_id, iteration_num, http_response.status_code, http_response.success
                    );

                    (
                        http_response.success,
                        Some(http_response.status_code as i32),
                        Some(snippet),
                        http_response.error_message.clone(),
                        Ok(http_response.workflow_event.clone())
                    )
                }
                Err(e) => {
                    let error_str = e.to_string();
                    // Get workflow context for logging
                    let execution_id = Self::parse_execution_id(&config.execution_step_id);
                    let node_id = Self::parse_node_id(&config.execution_step_id);
                    if let Some(workflow_id) = Self::get_workflow_id_from_step(db.as_ref(), &config.execution_step_id).await {
                        log_workflow_warn!(&workflow_id, execution_id, node_id,
                            format!("HTTP request execution failed for loop {} iteration {}: {}", loop_id, iteration_num, error_str));
                    } else {
                        tracing::warn!("HTTP request execution failed for loop {} iteration {}: {}", loop_id, iteration_num, error_str);
                    }
                    let error = SwissPipeError::Generic(format!("HTTP loop execution failed: {e}"));
                    (false, None, None, Some(error_str), Err(error))
                }
            };

            // Create iteration result
            let iteration_result = IterationResult {
                iteration: iteration_num as u32,
                timestamp: iteration_start.timestamp_micros(),
                http_status: response_status,
                success,
                response_snippet: response_body.clone(),
                error_message: error_msg.clone(),
            };

        // All database operations within a single transaction for atomicity
        use sea_orm::{TransactionTrait, ActiveModelTrait, EntityTrait, Set};

        let txn = db.begin().await?;

        // Load current loop state within the transaction
        let current_loop_state = http_loop_states::Entity::find_by_id(loop_id)
            .one(&txn)
            .await?
            .ok_or_else(|| SwissPipeError::Generic(format!("Loop state not found in transaction: {loop_id}")))?;

        // Update iteration result and loop state
        let mut updated_loop_state = current_loop_state.clone();
        updated_loop_state.add_iteration_result_with_limit(iteration_result, scheduler_config.max_history_entries)?;

        let mut active_model: http_loop_states::ActiveModel = updated_loop_state.into();
        let current_iteration = current_loop_state.current_iteration + 1;
        active_model.current_iteration = Set(current_iteration);
        active_model.last_response_status = Set(response_status);
        active_model.last_response_body = Set(response_body);

        // Handle consecutive failures within transaction
        let consecutive_failures = if success {
            active_model.consecutive_failures = Set(0); // Reset on success
            0
        } else {
            let new_failures = current_loop_state.consecutive_failures + 1;
            active_model.consecutive_failures = Set(new_failures);
            new_failures
        };

        // Update the loop state
        active_model.update(&txn).await?;

        // Check max iterations after HTTP request - if this iteration reached the limit, terminate with final response
        if let Some(max_iter) = loop_state.max_iterations {
            if current_iteration >= max_iter {
                let final_response_data = workflow_event.as_ref().ok().map(|e| e.data.clone());
                Self::terminate_loop_in_transaction(&txn, loop_id, LoopTerminationReason::MaxIterations, final_response_data).await?;
                txn.commit().await?;
                return Ok(());
            }
        }

        // Evaluate termination conditions (JavaScript evaluation doesn't need transaction)
        let should_stop = Self::should_terminate_on_stop(&js_executor, config.loop_config.termination_condition.as_ref(), &workflow_event, consecutive_failures, loop_state.loop_started_at, current_iteration).await?;
        let should_terminate_success = if success {
            Self::should_terminate_on_success(&js_executor, config.loop_config.termination_condition.as_ref(), &workflow_event, loop_state.loop_started_at, current_iteration).await?
        } else {
            false
        };
        let should_terminate_failure = if !success {
            Self::should_terminate_on_failure(&js_executor, config.loop_config.termination_condition.as_ref(), consecutive_failures, loop_state.loop_started_at, current_iteration).await?
        } else {
            false
        };

        // Handle termination within the same transaction
        // IMPORTANT: Use HTTP response data from current iteration, not original input data
        let final_response_data = workflow_event.as_ref().ok().map(|http_response_event| {
            // Use the HTTP response data from the current iteration
            http_response_event.data.clone()
        });

        if should_stop {
            Self::terminate_loop_in_transaction(&txn, loop_id, LoopTerminationReason::Stopped, final_response_data.clone()).await?;
            txn.commit().await?;
            return Ok(());
        } else if should_terminate_success {
            Self::terminate_loop_in_transaction(&txn, loop_id, LoopTerminationReason::Success, final_response_data.clone()).await?;
            txn.commit().await?;
            return Ok(());
        } else if should_terminate_failure {
            Self::terminate_loop_in_transaction(&txn, loop_id, LoopTerminationReason::Failure, final_response_data.clone()).await?;
            txn.commit().await?;
            return Ok(());
        }

        // Calculate next execution time and update within transaction
        let current_interval = config.loop_config.interval_seconds;
        let next_interval = Self::calculate_next_interval(&js_executor, &config.loop_config.backoff_strategy,
            current_interval, success).await?;

        let next_execution = Utc::now() + chrono::Duration::seconds(next_interval as i64);

        // Update next execution time within the same transaction
        let loop_for_next_update = http_loop_states::Entity::find_by_id(loop_id)
            .one(&txn)
            .await?
            .ok_or_else(|| SwissPipeError::Generic(format!("Loop state not found for next execution update: {loop_id}")))?;

        let mut next_exec_model: http_loop_states::ActiveModel = loop_for_next_update.into();
        next_exec_model.next_execution_at = Set(Some(next_execution.timestamp_micros()));
        next_exec_model.update(&txn).await?;

        // Commit all changes atomically
        txn.commit().await?;

        tracing::debug!("HTTP loop iteration completed: {}, next execution: {}", loop_id, next_execution);
        Ok(())
    }

    /// Execute a single HTTP request with comprehensive response capture
    async fn execute_http_request(
        app_executor: &AppExecutor,
        config: &HttpLoopConfig,
        event: &WorkflowEvent,
        scheduler_config: &crate::config::HttpLoopConfig,
    ) -> Result<HttpLoopResponse> {
        // For loops, we need to capture ALL HTTP responses, not just successful ones
        // We'll use the standard app_executor but handle the error cases specially
        let retry_config = RetryConfig {
            max_attempts: 1, // Single attempt for loops
            initial_delay_ms: 100,
            max_delay_ms: 1000,
            backoff_multiplier: 1.0,
        };

        match app_executor.execute_http_request(
            &config.url,
            &config.method,
            config.timeout_seconds,
            &retry_config,
            event.clone(),
            &config.headers,
        ).await {
            Ok(response_event) => {
                // Successful HTTP response (2xx status codes)
                let status_code = response_event.metadata
                    .get("http_status")
                    .and_then(|s| s.parse::<u16>().ok())
                    .unwrap_or(200);

                // Check response size limit
                let response_size = serde_json::to_string(&response_event.data)
                    .map(|s| s.len())
                    .unwrap_or(0);

                if response_size > scheduler_config.max_response_size_bytes {
                    return Err(SwissPipeError::Generic(format!(
                        "Response size {} bytes exceeds maximum allowed {} bytes",
                        response_size,
                        scheduler_config.max_response_size_bytes
                    )));
                }

                Ok(HttpLoopResponse {
                    workflow_event: response_event,
                    status_code,
                    success: true,
                    error_message: None,
                })
            }
            Err(SwissPipeError::App(crate::workflow::errors::AppError::InvalidStatus { status })) => {
                // HTTP response with non-2xx status code - we still want to capture this
                // This is an informational log at INFO level, no workflow context needed
                tracing::info!("HTTP loop captured non-success response: status {}", status);

                // We need to make another request to capture the response body for non-2xx responses
                // For now, we'll create a basic event with status information
                let mut response_event = event.clone();
                response_event.metadata.insert("http_status".to_string(), status.to_string());
                response_event.data = serde_json::json!({
                    "error": "HTTP response with non-success status",
                    "status_code": status
                });

                Ok(HttpLoopResponse {
                    workflow_event: response_event,
                    status_code: status,
                    success: Self::is_success_status(status),
                    error_message: Some(format!("HTTP {status} status")),
                })
            }
            Err(e) => {
                // Network error, timeout, or other failure
                // This is already logged in execute_loop_iteration_internal, no need to duplicate
                tracing::warn!("HTTP loop request failed: {}", e);

                let mut error_event = event.clone();
                error_event.data = serde_json::json!({
                    "error": e.to_string(),
                    "request_failed": true
                });

                Ok(HttpLoopResponse {
                    workflow_event: error_event,
                    status_code: 0, // Indicates network/connection failure
                    success: false,
                    error_message: Some(e.to_string()),
                })
            }
        }
    }

    /// Determine if HTTP status code represents success for loop purposes
    fn is_success_status(status_code: u16) -> bool {
        match status_code {
            200..=299 => true,  // 2xx Success
            _ => false,
        }
    }

    // Note: update_loop_iteration removed - now handled within transactions

    // Note: terminate_loop method removed - using terminate_loop_in_transaction for atomic operations

    /// Terminate a loop within an existing transaction (for atomic operations)
    async fn terminate_loop_in_transaction<C>(
        txn: &C,
        loop_id: &str,
        reason: LoopTerminationReason,
        successful_response: Option<serde_json::Value>,
    ) -> Result<()>
    where
        C: sea_orm::ConnectionTrait,
    {
        use sea_orm::{EntityTrait, Set, ActiveModelTrait};

        tracing::info!("Terminating loop {} within transaction with reason: {:?}", loop_id, reason);

        let loop_state = http_loop_states::Entity::find_by_id(loop_id)
            .one(txn)
            .await?
            .ok_or_else(|| SwissPipeError::Generic(format!("Loop state not found: {loop_id}")))?;

        // Update loop state to completed/failed based on termination reason
        let mut loop_active_model: http_loop_states::ActiveModel = loop_state.clone().into();
        loop_active_model.status = Set(match reason {
            LoopTerminationReason::Success => LoopStatus::Completed.to_string(),
            LoopTerminationReason::MaxIterations => LoopStatus::Completed.to_string(),
            LoopTerminationReason::Failure => LoopStatus::Failed.to_string(),
            LoopTerminationReason::Stopped => LoopStatus::Cancelled.to_string(),
        });
        loop_active_model.termination_reason = Set(Some(reason.to_string()));
        loop_active_model.next_execution_at = Set(None); // Clear scheduled execution

        // Store the final response data in the loop state before updating
        if let Some(response_data) = &successful_response {
            loop_active_model.last_response_body = Set(Some(response_data.to_string()));
        }

        loop_active_model.update(txn).await?;

        Ok(())
    }

    /// Create enhanced loop event context with comprehensive metadata
    fn create_loop_event_context(
        response: &Result<WorkflowEvent>,
        consecutive_failures: i32,
        loop_started_at: i64,
        current_iteration: i32,
    ) -> Result<WorkflowEvent> {
        let now = chrono::Utc::now().timestamp_micros();
        let elapsed_micros = now - loop_started_at;
        let elapsed_seconds = elapsed_micros / 1_000_000;

        match response {
            Ok(workflow_event) => {
                let mut enhanced_metadata = workflow_event.metadata.clone();

                // Add loop metadata (kept as strings for backward compatibility)
                enhanced_metadata.insert("loop_iteration".to_string(), current_iteration.to_string());
                enhanced_metadata.insert("consecutive_failures".to_string(), consecutive_failures.to_string());
                enhanced_metadata.insert("loop_started_at".to_string(), loop_started_at.to_string());
                enhanced_metadata.insert("current_timestamp".to_string(), now.to_string());
                enhanced_metadata.insert("elapsed_seconds".to_string(), elapsed_seconds.to_string());
                enhanced_metadata.insert("elapsed_micros".to_string(), elapsed_micros.to_string());

                // Ensure http_status is properly set and available
                if !enhanced_metadata.contains_key("http_status") {
                    enhanced_metadata.insert("http_status".to_string(), "200".to_string());
                }

                // Enhanced data structure with numeric metadata embedded for JavaScript access
                let enhanced_data = match &workflow_event.data {
                    serde_json::Value::Object(obj) => {
                        let mut new_obj = obj.clone();
                        // Add a metadata object with numeric types for easier JavaScript access
                        let mut js_metadata = serde_json::Map::new();
                        js_metadata.insert("loop_iteration".to_string(), serde_json::Value::Number(current_iteration.into()));
                        js_metadata.insert("consecutive_failures".to_string(), serde_json::Value::Number(consecutive_failures.into()));
                        js_metadata.insert("elapsed_seconds".to_string(), serde_json::Value::Number(elapsed_seconds.into()));

                        // Add http_status as number
                        let http_status = enhanced_metadata.get("http_status")
                            .and_then(|s| s.parse::<u16>().ok())
                            .unwrap_or(200);
                        js_metadata.insert("http_status".to_string(), serde_json::Value::Number(http_status.into()));

                        new_obj.insert("metadata".to_string(), serde_json::Value::Object(js_metadata));
                        serde_json::Value::Object(new_obj)
                    }
                    other => {
                        // For non-object responses, create wrapper with metadata
                        let mut wrapper = serde_json::Map::new();
                        wrapper.insert("response_data".to_string(), other.clone());

                        let mut js_metadata = serde_json::Map::new();
                        js_metadata.insert("loop_iteration".to_string(), serde_json::Value::Number(current_iteration.into()));
                        js_metadata.insert("consecutive_failures".to_string(), serde_json::Value::Number(consecutive_failures.into()));
                        js_metadata.insert("elapsed_seconds".to_string(), serde_json::Value::Number(elapsed_seconds.into()));

                        let http_status = enhanced_metadata.get("http_status")
                            .and_then(|s| s.parse::<u16>().ok())
                            .unwrap_or(200);
                        js_metadata.insert("http_status".to_string(), serde_json::Value::Number(http_status.into()));

                        wrapper.insert("metadata".to_string(), serde_json::Value::Object(js_metadata));
                        serde_json::Value::Object(wrapper)
                    }
                };

                Ok(WorkflowEvent {
                    data: enhanced_data,
                    metadata: enhanced_metadata,
                    headers: workflow_event.headers.clone(),
                    condition_results: workflow_event.condition_results.clone(),
        hil_task: None,
                })
            }
            Err(_) => {
                // For failed responses, create minimal event with loop metadata
                let mut metadata = HashMap::new();
                metadata.insert("loop_iteration".to_string(), current_iteration.to_string());
                metadata.insert("consecutive_failures".to_string(), consecutive_failures.to_string());
                metadata.insert("loop_started_at".to_string(), loop_started_at.to_string());
                metadata.insert("current_timestamp".to_string(), now.to_string());
                metadata.insert("elapsed_seconds".to_string(), elapsed_seconds.to_string());
                metadata.insert("elapsed_micros".to_string(), elapsed_micros.to_string());
                metadata.insert("http_status".to_string(), "0".to_string()); // 0 indicates failure

                // Create failure data with proper numeric types
                let mut js_metadata = serde_json::Map::new();
                js_metadata.insert("loop_iteration".to_string(), serde_json::Value::Number(current_iteration.into()));
                js_metadata.insert("consecutive_failures".to_string(), serde_json::Value::Number(consecutive_failures.into()));
                js_metadata.insert("elapsed_seconds".to_string(), serde_json::Value::Number(elapsed_seconds.into()));
                js_metadata.insert("http_status".to_string(), serde_json::Value::Number(0.into()));

                let mut failure_data = serde_json::Map::new();
                failure_data.insert("error".to_string(), serde_json::Value::String("HTTP request failed".to_string()));
                failure_data.insert("metadata".to_string(), serde_json::Value::Object(js_metadata));

                Ok(WorkflowEvent {
                    data: serde_json::Value::Object(failure_data),
                    metadata,
                    headers: HashMap::new(),
                    condition_results: HashMap::new(),
        hil_task: None,
                })
            }
        }
    }

    /// Unified termination condition evaluation using JavaScript functions
    async fn should_terminate(
        js_executor: &JavaScriptExecutor,
        termination_condition: Option<&TerminationCondition>,
        response: &Result<WorkflowEvent>,
        consecutive_failures: i32,
        loop_started_at: i64,
        current_iteration: i32,
    ) -> Result<bool> {
        if let Some(condition) = termination_condition {
            // Create enhanced event with loop metadata
            let enhanced_event = Self::create_loop_event_context(
                response, consecutive_failures, loop_started_at, current_iteration
            )?;

            // Use existing execute_condition method from JavaScriptExecutor
            match js_executor.execute_condition(&condition.script, &enhanced_event).await {
                Ok(true) => {
                    tracing::info!("Termination condition met for action {:?}: {}", condition.action, condition.script.chars().take(100).collect::<String>());
                    return Ok(true);
                }
                Ok(false) => {
                    tracing::debug!("Termination condition not met: {}", condition.script.chars().take(100).collect::<String>());
                    return Ok(false);
                }
                Err(e) => {
                    // This is a script evaluation error - system-level warning
                    tracing::warn!("Termination condition evaluation error: {} - Script: {}", e, condition.script.chars().take(100).collect::<String>());
                    return Ok(false); // Don't fail loop on condition errors
                }
            }
        }
        Ok(false)
    }

    /// Check termination condition for success cases
    async fn should_terminate_on_success(
        js_executor: &JavaScriptExecutor,
        termination_condition: Option<&TerminationCondition>,
        response: &Result<WorkflowEvent>,
        loop_started_at: i64,
        current_iteration: i32,
    ) -> Result<bool> {
        // Only check condition if it's configured for Success action
        if let Some(condition) = termination_condition {
            if condition.action == TerminationAction::Success {
                return Self::should_terminate(
                    js_executor,
                    Some(condition),
                    response,
                    0, // consecutive_failures not relevant for success
                    loop_started_at,
                    current_iteration,
                ).await;
            }
        }
        Ok(false)
    }

    /// Check termination condition for failure cases
    async fn should_terminate_on_failure(
        js_executor: &JavaScriptExecutor,
        termination_condition: Option<&TerminationCondition>,
        consecutive_failures: i32,
        loop_started_at: i64,
        current_iteration: i32,
    ) -> Result<bool> {
        // Only check condition if it's configured for Failure action
        if let Some(condition) = termination_condition {
            if condition.action == TerminationAction::Failure {
                // Create empty response for failure cases
                let failure_response: Result<WorkflowEvent> = Err(SwissPipeError::Generic("HTTP request failed".to_string()));

                return Self::should_terminate(
                    js_executor,
                    Some(condition),
                    &failure_response,
                    consecutive_failures,
                    loop_started_at,
                    current_iteration,
                ).await;
            }
        }
        Ok(false)
    }

    /// Check termination condition for stop cases (applies to both success and failure scenarios)
    async fn should_terminate_on_stop(
        js_executor: &JavaScriptExecutor,
        termination_condition: Option<&TerminationCondition>,
        response: &Result<WorkflowEvent>,
        consecutive_failures: i32,
        loop_started_at: i64,
        current_iteration: i32,
    ) -> Result<bool> {
        // Only check condition if it's configured for Stop action
        if let Some(condition) = termination_condition {
            if condition.action == TerminationAction::Stop {
                return Self::should_terminate(
                    js_executor,
                    Some(condition),
                    response,
                    consecutive_failures,
                    loop_started_at,
                    current_iteration,
                ).await;
            }
        }
        Ok(false)
    }

    // Note: update_next_execution_time_static removed - now handled within transactions

    /// Calculate next interval based on backoff strategy
    async fn calculate_next_interval(
        js_executor: &JavaScriptExecutor,
        strategy: &BackoffStrategy,
        current_interval: u64,
        success: bool,
    ) -> Result<u64> {
        match strategy {
            BackoffStrategy::Fixed(_) => Ok(current_interval), // Use interval_seconds instead of fixed value
            BackoffStrategy::Exponential { base, multiplier, max } => {
                if success {
                    Ok(*base) // Reset to base on success
                } else {
                    let next = (current_interval as f64 * multiplier) as u64;
                    Ok(next.min(*max))
                }
            }
            BackoffStrategy::Custom(expression) => {
                // Evaluate custom JavaScript expression for interval calculation
                let script = format!(
                    "(function() {{ const current = {current_interval}; const success = {success}; return Math.max(1, Math.floor({expression})); }})()"
                );

                match js_executor.execute_numeric(&script).await {
                    Ok(result) => {
                        let interval = (result as u64).clamp(1, 86400); // Cap between 1 second and 1 day
                        tracing::info!("Custom backoff strategy evaluated to: {} seconds (from expression: {})", interval, expression);
                        Ok(interval)
                    }
                    Err(e) => {
                        // This is a script execution error - system-level error without specific workflow context
                        tracing::error!("Custom backoff strategy execution failed: {}, using current interval: {}", e, current_interval);
                        Ok(current_interval)
                    }
                }
            }
        }
    }

    // Note: Old helper methods removed - database operations now use transactions for atomicity

    // === VALIDATION AND ERROR HANDLING ===

    /// Validate HTTP loop configuration before scheduling
    fn validate_loop_config(config: &HttpLoopConfig) -> Result<()> {
        // Validate URL format (trim whitespace first)
        let trimmed_url = config.url.trim();
        if trimmed_url.is_empty() {
            return Err(SwissPipeError::Generic("HTTP loop URL cannot be empty".to_string()));
        }

        // Basic URL format validation on trimmed URL
        if !trimmed_url.starts_with("http://") && !trimmed_url.starts_with("https://") {
            return Err(SwissPipeError::Generic(format!("Invalid URL format: {}", config.url)));
        }

        // Validate timeout
        if config.timeout_seconds == 0 || config.timeout_seconds > 3600 {
            return Err(SwissPipeError::Generic(format!(
                "Invalid timeout: {} (must be between 1 and 3600 seconds)",
                config.timeout_seconds
            )));
        }

        // Validate loop configuration
        if let Some(max_iterations) = config.loop_config.max_iterations {
            if max_iterations == 0 || max_iterations > 10000 {
                return Err(SwissPipeError::Generic(format!(
                    "Invalid max_iterations: {max_iterations} (must be between 1 and 10000)"
                )));
            }
        }

        // Validate interval settings
        if config.loop_config.interval_seconds == 0 || config.loop_config.interval_seconds > 86400 {
            return Err(SwissPipeError::Generic(format!(
                "Invalid interval: {} (must be between 1 and 86400 seconds)",
                config.loop_config.interval_seconds
            )));
        }

        // Validate backoff strategy
        match &config.loop_config.backoff_strategy {
            BackoffStrategy::Fixed(interval) => {
                if *interval == 0 || *interval > 86400 {
                    return Err(SwissPipeError::Generic(format!(
                        "Invalid fixed backoff interval: {interval} (must be between 1 and 86400 seconds)"
                    )));
                }
            }
            BackoffStrategy::Exponential { base, multiplier, max } => {
                if *base == 0 || *base > 3600 {
                    return Err(SwissPipeError::Generic(format!(
                        "Invalid exponential base: {base} (must be between 1 and 3600 seconds)"
                    )));
                }
                if *multiplier <= 1.0 || *multiplier > 10.0 {
                    return Err(SwissPipeError::Generic(format!(
                        "Invalid exponential multiplier: {multiplier} (must be between 1.0 and 10.0)"
                    )));
                }
                if *max == 0 || *max > 86400 {
                    return Err(SwissPipeError::Generic(format!(
                        "Invalid exponential max: {max} (must be between 1 and 86400 seconds)"
                    )));
                }
            }
            BackoffStrategy::Custom(expression) => {
                if expression.trim().is_empty() {
                    return Err(SwissPipeError::Generic("Custom backoff expression cannot be empty".to_string()));
                }
                // Basic JavaScript validation - check for dangerous patterns
                Self::validate_javascript_expression(expression, "backoff expression")?;
            }
        }

        // Validate termination condition
        if let Some(condition) = &config.loop_config.termination_condition {
            if condition.script.trim().is_empty() {
                return Err(SwissPipeError::Generic(
                    "Termination condition script cannot be empty".to_string()
                ));
            }
            Self::validate_javascript_expression(&condition.script, "termination condition")?;
        }

        Ok(())
    }

    /// Validate JavaScript expressions for security and basic syntax
    fn validate_javascript_expression(expression: &str, context: &str) -> Result<()> {
        let expression = expression.trim();

        if expression.is_empty() {
            return Err(SwissPipeError::Generic(format!("{context} expression cannot be empty")));
        }

        // Check for potentially dangerous patterns
        let dangerous_patterns = [
            "eval(",
            "Function(",
            "new Function(",
            "require(",
            "import ",
            "export ",
            "__proto__",
            ".constructor",
            "Array.constructor",
            "Object.constructor",
            "process.",
            "global.",
            "window.",
            "document.",
            "setTimeout(",
            "setInterval(",
            "XMLHttpRequest",
            "fetch(",
            "fs.",
            "child_process",
        ];

        for pattern in &dangerous_patterns {
            if expression.contains(pattern) {
                return Err(SwissPipeError::Generic(format!(
                    "Potentially dangerous pattern '{pattern}' found in {context} expression"
                )));
            }
        }

        // Basic bracket/parentheses balance check
        let mut parens = 0i32;
        let mut brackets = 0i32;
        let mut braces = 0i32;

        for ch in expression.chars() {
            match ch {
                '(' => parens += 1,
                ')' => parens -= 1,
                '[' => brackets += 1,
                ']' => brackets -= 1,
                '{' => braces += 1,
                '}' => braces -= 1,
                _ => {}
            }

            if parens < 0 || brackets < 0 || braces < 0 {
                return Err(SwissPipeError::Generic(format!(
                    "Unbalanced brackets/parentheses in {context} expression"
                )));
            }
        }

        if parens != 0 || brackets != 0 || braces != 0 {
            return Err(SwissPipeError::Generic(format!(
                "Unbalanced brackets/parentheses in {context} expression"
            )));
        }

        Ok(())
    }


    /// Insert loop state directly without limits
    /// Each loop operates independently with its own row
    async fn insert_loop_state(&self, loop_state: http_loop_states::ActiveModel) -> Result<()> {
        // Direct insert - each loop operates independently
        match loop_state.insert(self.db.as_ref()).await {
            Ok(_) => {
                tracing::debug!("Loop state inserted successfully");
                Ok(())
            }
            Err(e) => {
                // This is a database-level error - system-level without workflow context
                tracing::error!("Failed to insert loop state: {}", e);
                Err(SwissPipeError::Generic(format!("Failed to insert loop state: {e}")))
            }
        }
    }




    /// Validate state transition is allowed
    fn validate_state_transition(from: &LoopStatus, to: &LoopStatus) -> Result<()> {
        use LoopStatus::*;

        let valid_transitions = match from {
            Running => vec![Paused, Completed, Failed, Cancelled],
            Paused => vec![Running, Cancelled, Failed],
            Completed => vec![], // Final state
            Failed => vec![], // Final state
            Cancelled => vec![], // Final state
        };

        if valid_transitions.contains(to) {
            Ok(())
        } else {
            Err(SwissPipeError::Generic(format!(
                "Invalid state transition from {from:?} to {to:?}"
            )))
        }
    }

    /// Safely update loop state with transaction rollback on error
    async fn safe_update_loop_state<F, T>(
        db: &DatabaseConnection,
        loop_id: &str,
        operation: F,
    ) -> Result<T>
    where
        F: FnOnce(&mut http_loop_states::ActiveModel) -> Result<T> + Send,
    {
        use sea_orm::{TransactionTrait, ActiveModelTrait};

        let txn = db.begin().await?;

        let result = async {
            let loop_state = http_loop_states::Entity::find_by_id(loop_id)
                .one(&txn)
                .await?
                .ok_or_else(|| SwissPipeError::Generic(format!("Loop state not found: {loop_id}")))?;

            let mut active_model: http_loop_states::ActiveModel = loop_state.into();
            let result = operation(&mut active_model)?;

            active_model.update(&txn).await?;
            Ok(result)
        }.await;

        match result {
            Ok(value) => {
                txn.commit().await?;
                Ok(value)
            }
            Err(e) => {
                if let Err(rollback_err) = txn.rollback().await {
                    // This is a database transaction error - system-level
                    tracing::error!("Failed to rollback transaction for loop {}: {}", loop_id, rollback_err);
                }
                Err(e)
            }
        }
    }

    /// Clear all loop tasks from the scheduler state (useful for testing)
    /// This method cancels any running tasks and clears both the internal HashMap and database records
    pub async fn clear_all_loop_tasks(&self) -> Result<()> {
        tracing::info!("Clearing all loop tasks from scheduler state...");

        // Clear in-memory tasks first
        let mut tasks = self.loop_tasks.write().await;
        let task_count = tasks.len();

        // Abort all running tasks
        for (loop_id, task_handle) in tasks.drain() {
            if !task_handle.is_finished() {
                tracing::debug!("Aborting running loop task: {}", loop_id);
                task_handle.abort();
            }
        }

        drop(tasks); // Release the lock

        // Clear database records
        use sea_orm::{EntityTrait, DeleteResult};
        let delete_result: DeleteResult = http_loop_states::Entity::delete_many()
            .exec(self.db.as_ref())
            .await
            .map_err(SwissPipeError::Database)?;

        tracing::info!(
            "Cleared {} loop tasks from memory and {} records from database",
            task_count,
            delete_result.rows_affected
        );
        Ok(())
    }

    /// Get the count of currently tracked loop tasks (useful for testing/debugging)
    pub async fn get_active_task_count(&self) -> usize {
        let tasks = self.loop_tasks.read().await;
        tasks.len()
    }

}