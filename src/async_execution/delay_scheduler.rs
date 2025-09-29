use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tokio::time::{sleep_until, Duration, Instant};
use chrono::{DateTime, Utc};
use sea_orm::{entity::Set, ActiveModelTrait, EntityTrait, QueryFilter, ColumnTrait, TransactionTrait};
use serde_json;
use uuid::Uuid;

use crate::database::{job_queue, scheduled_delays};
use crate::database::scheduled_delays::{DelayStatus};
use crate::database::job_queue::JobStatus;
use crate::workflow::errors::{SwissPipeError, Result};
use crate::workflow::models::{WorkflowEvent};
use sea_orm::DatabaseConnection;

pub struct DelayScheduler {
    db: Arc<DatabaseConnection>,
    // Track running delay tasks  
    delay_tasks: Arc<RwLock<HashMap<String, tokio::task::JoinHandle<()>>>>,
}

impl DelayScheduler {
    pub async fn new(
        db: Arc<DatabaseConnection>,
    ) -> Result<Self> {
        tracing::info!("Creating DelayScheduler with tokio timers...");
        
        let delay_scheduler = Self {
            db,
            delay_tasks: Arc::new(RwLock::new(HashMap::new())),
        };
        
        tracing::info!("DelayScheduler fully initialized");
        Ok(delay_scheduler)
    }

    /// Schedule a delay for workflow execution
    pub async fn schedule_delay(
        &self,
        execution_id: String,
        current_node_id: String,
        next_node_id: String,
        delay_duration: chrono::Duration,
        workflow_state: WorkflowEvent,
    ) -> Result<String> {
        let delay_id = Uuid::now_v7().to_string();
        let scheduled_at = Utc::now() + delay_duration;
        let scheduled_at_micros = scheduled_at.timestamp_micros();
        
        // Serialize workflow state
        let workflow_state_json = serde_json::to_string(&workflow_state)
            .map_err(|e| SwissPipeError::Generic(format!("Failed to serialize workflow state: {e}")))?;

        let txn = self.db.begin().await?;

        // 1. Persist delay in database
        let delay_model = scheduled_delays::ActiveModel {
            id: Set(delay_id.clone()),
            execution_id: Set(execution_id.clone()),
            current_node_id: Set(current_node_id),     // Node ID field
            next_node_id: Set(next_node_id.clone()),   // Node ID field
            scheduled_at: Set(scheduled_at_micros),
            status: Set(DelayStatus::Pending.to_string()),
            workflow_state: Set(workflow_state_json),
            scheduler_job_id: Set(None), // We don't use external scheduler anymore
            created_at: Set(Utc::now().timestamp_micros()),
        };

        let _delay_record = delay_model.insert(&txn).await
            .map_err(|e| {
                tracing::error!("Failed to insert delay record: {}", e);
                SwissPipeError::Generic(format!("Database insert failed: {e}"))
            })?;

        txn.commit().await?;

        // 2. Schedule tokio delay task
        let delay_id_clone = delay_id.clone();
        let db_clone = self.db.clone();
        
        // Calculate duration from now with strict overflow protection
        let now = Utc::now();
        let time_diff = scheduled_at - now;
        let duration_secs = time_diff.num_seconds()
            .clamp(1, 86400 * 365) as u64;  // Clamp between 1 second and 1 year
        
        let sleep_duration = Duration::from_secs(duration_secs.min(86400 * 30)); // Cap at 30 days for safety
        
        let wake_time = match Instant::now().checked_add(sleep_duration) {
            Some(time) => time,
            None => {
                return Err(SwissPipeError::Generic(format!(
                    "Duration too large for delay scheduling: {duration_secs} seconds"
                )));
            }
        };

        let delay_tasks_for_cleanup = self.delay_tasks.clone();
        let delay_id_for_cleanup = delay_id.clone();
        
        let delay_task = tokio::spawn(async move {
            tracing::debug!("Delay task scheduled to wake at: {:?}", wake_time);
            sleep_until(wake_time).await;
            
            tracing::info!("Delay task woken for delay_id: {}", delay_id_clone);
            if let Err(e) = Self::trigger_delay_direct(db_clone, delay_id_clone.clone()).await {
                tracing::error!("Failed to trigger delay {}: {}", delay_id_clone, e);
            }
            
            // Clean up completed task handle to prevent memory leak
            if let Some(_handle) = delay_tasks_for_cleanup.write().await.remove(&delay_id_for_cleanup) {
                tracing::debug!("Cleaned up completed delay task handle: {}", delay_id_for_cleanup);
            }
        });

        // Store the task handle for potential cancellation
        self.delay_tasks.write().await.insert(delay_id.clone(), delay_task);

        tracing::info!(
            "Scheduled delay '{}' for execution '{}' to trigger at {}",
            delay_id,
            execution_id,
            scheduled_at.format("%Y-%m-%d %H:%M:%S UTC")
        );

        Ok(delay_id)
    }

    /// Trigger a scheduled delay directly (for delay tasks)
    async fn trigger_delay_direct(
        db: Arc<DatabaseConnection>,
        delay_id: String,
    ) -> Result<()> {
        tracing::debug!("Directly triggering scheduled delay: {}", delay_id);

        let txn = db.begin().await?;

        // Get the delay record
        let delay_record = scheduled_delays::Entity::find_by_id(&delay_id)
            .one(&txn)
            .await?
            .ok_or_else(|| SwissPipeError::Generic(format!("Delay record not found: {delay_id}")))?;

        // Skip if already triggered/cancelled
        if delay_record.status != DelayStatus::Pending.to_string() {
            tracing::debug!("Delay {} already processed (status: {})", delay_id, delay_record.status);
            return Ok(()); // Already processed
        }

        // Deserialize workflow state
        let workflow_state: WorkflowEvent = serde_json::from_str(&delay_record.workflow_state)
            .map_err(|e| SwissPipeError::Generic(format!("Failed to deserialize workflow state: {e}")))?;

        // Mark delay as triggered
        let mut delay_update: scheduled_delays::ActiveModel = delay_record.clone().into();
        delay_update.status = Set(DelayStatus::Triggered.to_string());
        delay_update.update(&txn).await?;

        // Create immediate job to resume workflow execution
        let now = Utc::now().timestamp_micros();
        let job_payload = serde_json::json!({
            "type": "workflow_resume",
            "delay_id": delay_id,
            "execution_id": delay_record.execution_id.clone(),
            "current_node_id": delay_record.current_node_id.clone(),
            "next_node_id": delay_record.next_node_id.clone(),
            "workflow_state": workflow_state
        });

        let job = job_queue::ActiveModel {
            id: Set(Uuid::now_v7().to_string()),
            execution_id: Set(delay_record.execution_id.clone()),
            priority: Set(1),
            scheduled_at: Set(now), // Schedule immediately
            claimed_at: Set(None),
            claimed_by: Set(None),
            max_retries: Set(3),
            retry_count: Set(0),
            status: Set(JobStatus::Pending.to_string()),
            error_message: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            payload: Set(Some(serde_json::to_string(&job_payload)?)),
        };

        job.insert(&txn).await?;
        txn.commit().await?;

        tracing::info!(
            "Created job to resume workflow execution {} at node ID '{}' after scheduled delay",
            delay_record.execution_id,
            &delay_record.next_node_id
        );

        Ok(())
    }

    /// Cancel a scheduled delay
    pub async fn cancel_delay(&self, delay_id: &str) -> Result<()> {
        // CRITICAL: Remove task handle FIRST to prevent race condition
        // Task could complete between DB update and task removal
        let task_handle = self.delay_tasks.write().await.remove(delay_id);
        
        let txn = self.db.begin().await?;

        // Get the delay record
        let delay_record = scheduled_delays::Entity::find_by_id(delay_id)
            .one(&txn)
            .await?
            .ok_or_else(|| SwissPipeError::Generic(format!("Delay record not found: {delay_id}")))?;

        // Only cancel if still pending
        if delay_record.status != DelayStatus::Pending.to_string() {
            // If already processed, put task handle back if it exists
            if let Some(handle) = task_handle {
                self.delay_tasks.write().await.insert(delay_id.to_string(), handle);
            }
            return Ok(()); // Already triggered/cancelled
        }

        // Mark as cancelled
        let mut delay_update: scheduled_delays::ActiveModel = delay_record.into();
        delay_update.status = Set(DelayStatus::Cancelled.to_string());
        delay_update.update(&txn).await?;

        txn.commit().await?;

        // Now abort the task (handle was already removed above)
        if let Some(handle) = task_handle {
            handle.abort();
            tracing::info!("Cancelled delay task for delay_id: {}", delay_id);
        }

        tracing::info!("Cancelled scheduled delay: {}", delay_id);
        Ok(())
    }

    /// Cancel all scheduled delays for a specific execution
    pub async fn cancel_delays_for_execution(&self, execution_id: &str) -> Result<usize> {
        tracing::info!("Cancelling all scheduled delays for execution: {}", execution_id);
        
        // Find all pending delays for this execution
        let pending_delays = scheduled_delays::Entity::find()
            .filter(scheduled_delays::Column::ExecutionId.eq(execution_id))
            .filter(scheduled_delays::Column::Status.eq(DelayStatus::Pending.to_string()))
            .all(&*self.db)
            .await?;

        let mut cancelled_count = 0;
        for delay_record in pending_delays {
            match self.cancel_delay(&delay_record.id).await {
                Ok(_) => {
                    cancelled_count += 1;
                    tracing::debug!("Cancelled delay {} for execution {}", delay_record.id, execution_id);
                }
                Err(e) => {
                    tracing::error!("Failed to cancel delay {} for execution {}: {}", delay_record.id, execution_id, e);
                }
            }
        }
        
        if cancelled_count > 0 {
            tracing::info!("Cancelled {} scheduled delays for execution {}", cancelled_count, execution_id);
        }
        
        Ok(cancelled_count)
    }

    /// Restore scheduled delays from database on startup
    pub async fn restore_from_database(&self) -> Result<usize> {
        tracing::info!("Restoring scheduled delays from database...");

        let now = Utc::now().timestamp_micros();
        
        // Find all pending delays
        let pending_delays = scheduled_delays::Entity::find()
            .filter(scheduled_delays::Column::Status.eq(DelayStatus::Pending.to_string()))
            .all(&*self.db)
            .await?;

        let mut restored_count = 0;
        let mut triggered_count = 0;

        for delay_record in pending_delays {
            // Add safety margin - treat delays within 5 seconds as overdue to prevent timing races
            let safety_margin_micros = 5_000_000; // 5 seconds in microseconds
            let is_overdue = delay_record.scheduled_at <= (now + safety_margin_micros);
            
            if is_overdue {
                // Past due or very close - create immediate job
                tracing::info!("Creating immediate job for overdue delay: {} (scheduled: {}, now: {}, margin: {}s)", 
                    delay_record.id, 
                    delay_record.scheduled_at, 
                    now,
                    safety_margin_micros / 1_000_000
                );
                
                match self.create_immediate_trigger_job(&delay_record).await {
                    Ok(_) => {
                        triggered_count += 1;
                        tracing::debug!("Created immediate trigger job for overdue delay: {}", delay_record.id);
                    }
                    Err(e) => {
                        tracing::error!("Failed to create immediate trigger job for overdue delay {}: {}", delay_record.id, e);
                        // Continue processing other delays even if this one fails
                    }
                }
            } else {
                // Future delay with sufficient time remaining - reschedule safely
                let delay_id = delay_record.id.clone();
                let remaining_seconds = (delay_record.scheduled_at - now) / 1_000_000;
                tracing::info!("Restoring future delay: {} (remaining: {}s)", delay_id, remaining_seconds);
                
                match self.restore_single_delay(delay_record).await {
                    Ok(restored_delay_id) => {
                        restored_count += 1;
                        tracing::debug!("Restored scheduled delay: {}", restored_delay_id);
                    }
                    Err(e) => {
                        tracing::error!("Failed to restore delay {}: {}", delay_id, e);
                        // Continue processing other delays even if this one fails
                    }
                }
            }
        }

        tracing::info!(
            "Delay restoration complete: {} delays restored, {} overdue delays triggered immediately",
            restored_count,
            triggered_count
        );

        Ok(restored_count + triggered_count)
    }

    /// Create immediate trigger job for overdue delays (avoids recursion)
    async fn create_immediate_trigger_job(&self, delay_record: &crate::database::scheduled_delays::Model) -> Result<()> {
        tracing::debug!("Creating immediate trigger job for overdue delay: {}", delay_record.id);

        let txn = self.db.begin().await?;

        // Deserialize workflow state
        let workflow_state: WorkflowEvent = serde_json::from_str(&delay_record.workflow_state)
            .map_err(|e| SwissPipeError::Generic(format!("Failed to deserialize workflow state: {e}")))?;

        // Mark delay as triggered
        let mut delay_update: scheduled_delays::ActiveModel = delay_record.clone().into();
        delay_update.status = Set(DelayStatus::Triggered.to_string());
        delay_update.update(&txn).await?;

        // Create immediate job to resume workflow execution
        let now = Utc::now().timestamp_micros();
        let job_payload = serde_json::json!({
            "type": "workflow_resume",
            "delay_id": delay_record.id.clone(),
            "execution_id": delay_record.execution_id.clone(),
            "current_node_id": delay_record.current_node_id.clone(),
            "next_node_id": delay_record.next_node_id.clone(),
            "workflow_state": workflow_state
        });

        let job = job_queue::ActiveModel {
            id: Set(Uuid::now_v7().to_string()),
            execution_id: Set(delay_record.execution_id.clone()),
            priority: Set(1),
            scheduled_at: Set(now), // Schedule immediately
            claimed_at: Set(None),
            claimed_by: Set(None),
            max_retries: Set(3),
            retry_count: Set(0),
            status: Set(JobStatus::Pending.to_string()),
            error_message: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            payload: Set(Some(serde_json::to_string(&job_payload)?)),
        };

        job.insert(&txn).await?;
        txn.commit().await?;

        tracing::info!(
            "Created immediate job to resume workflow execution {} at node ID '{}' after overdue delay",
            delay_record.execution_id,
            &delay_record.next_node_id
        );

        Ok(())
    }

    /// Restore a single delay from database
    async fn restore_single_delay(&self, delay_record: crate::database::scheduled_delays::Model) -> Result<String> {
        let scheduled_at = DateTime::<Utc>::from_timestamp_micros(delay_record.scheduled_at)
            .ok_or_else(|| SwissPipeError::Generic("Invalid scheduled_at timestamp".to_string()))?;
        
        // Double-check this delay isn't overdue (safety check)
        let now = Utc::now();
        let time_diff = scheduled_at - now;
        if time_diff.num_seconds() <= 5 {  // If <= 5 seconds remaining, treat as overdue
            tracing::warn!("Delay {} is overdue or very close ({}s remaining), triggering immediately instead of restoring", 
                delay_record.id, time_diff.num_seconds());
            return self.create_immediate_trigger_job(&delay_record).await.map(|_| delay_record.id);
        }
        
        let delay_id_clone = delay_record.id.clone();
        let db_clone = self.db.clone();
        
        // Calculate duration from now with strict overflow protection
        let duration_secs = time_diff.num_seconds()
            .clamp(1, 86400 * 365) as u64;  // Clamp between 1 second and 1 year to prevent extreme durations
        
        // Additional safety: ensure duration won't overflow when added to Instant
        let sleep_duration = Duration::from_secs(duration_secs.min(86400 * 30)); // Cap at 30 days for safety
        
        let wake_time = match Instant::now().checked_add(sleep_duration) {
            Some(time) => time,
            None => {
                tracing::error!("Duration overflow when restoring delay {}, triggering immediately", delay_record.id);
                return self.create_immediate_trigger_job(&delay_record).await.map(|_| delay_record.id);
            }
        };

        let delay_tasks_for_cleanup = self.delay_tasks.clone();
        let delay_id_for_cleanup = delay_record.id.clone();
        
        let delay_task = tokio::spawn(async move {
            tracing::debug!("Restored delay task scheduled to wake at: {:?}", wake_time);
            sleep_until(wake_time).await;
            
            tracing::info!("Restored delay task woken for delay_id: {}", delay_id_clone);
            if let Err(e) = Self::trigger_delay_direct(db_clone, delay_id_clone.clone()).await {
                tracing::error!("Failed to trigger restored delay {}: {}", delay_id_clone, e);
            }
            
            // Clean up completed task handle to prevent memory leak
            if let Some(_handle) = delay_tasks_for_cleanup.write().await.remove(&delay_id_for_cleanup) {
                tracing::debug!("Cleaned up completed restored delay task handle: {}", delay_id_for_cleanup);
            }
        });

        // Store the task handle for potential cancellation
        let delay_record_id = delay_record.id.clone();
        self.delay_tasks.write().await.insert(delay_record_id.clone(), delay_task);

        Ok(delay_record_id)
    }

    /// Shutdown the scheduler
    pub async fn shutdown(&self) -> Result<()> {
        tracing::info!("DelayScheduler shutdown requested");
        
        // Cancel all running delay tasks
        let mut tasks = self.delay_tasks.write().await;
        for (delay_id, handle) in tasks.drain() {
            handle.abort();
            tracing::debug!("Cancelled delay task for delay_id: {}", delay_id);
        }
        
        tracing::info!("DelayScheduler shutdown complete");
        Ok(())
    }
}