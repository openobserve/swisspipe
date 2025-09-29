use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait, ColumnTrait, QueryFilter};
use tokio::sync::Mutex;

use crate::database::human_in_loop_tasks;
use crate::workflow::errors::{Result, SwissPipeError};
use crate::workflow::models::{WorkflowEvent, WorkflowResumptionState};

/// HIL Service manages workflow resumption through database job queue (no in-memory channels)
#[derive(Clone)]
pub struct HilService {
    db: Arc<DatabaseConnection>,
    /// Mutex to prevent concurrent timeout processing operations
    timeout_processing_lock: Arc<Mutex<()>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HilResponse {
    pub decision: String,
    pub response_data: Option<serde_json::Value>,
    pub task_id: String,
}

pub struct HilTaskParams<'a> {
    pub execution_id: &'a str,
    pub workflow_id: &'a str,
    pub node_id: &'a str,
    pub node_execution_id: &'a str,
    pub config: &'a crate::workflow::models::NodeType,
    pub event: &'a WorkflowEvent,
}

impl HilService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self {
            db,
            timeout_processing_lock: Arc::new(Mutex::new(())),
        }
    }

    /// Create HIL task and prepare resumption state for database job queue
    pub async fn create_hil_task_and_prepare_resumption(
        &self,
        params: HilTaskParams<'_>,
    ) -> Result<(String, WorkflowResumptionState)> {
        // Create HIL task in database with transaction isolation (already implemented)
        let hil_task_id = self.create_hil_task_transactional(
            params.execution_id, params.workflow_id, params.node_id, params.node_execution_id, params.config
        ).await?;

        // Prepare resumption state for job queue storage
        let resumption_state = WorkflowResumptionState {
            execution_id: params.execution_id.to_string(),
            workflow_id: params.workflow_id.to_string(),
            current_node_id: params.node_id.to_string(),
            event_data: params.event.clone(),
            hil_task_id: hil_task_id.clone(),
        };

        Ok((hil_task_id, resumption_state))
    }

    /// Create HIL task with proper transaction isolation
    async fn create_hil_task_transactional(
        &self,
        execution_id: &str,
        workflow_id: &str,
        node_id: &str,
        node_execution_id: &str,
        config: &crate::workflow::models::NodeType,
    ) -> Result<String> {
        use sea_orm::{TransactionTrait, Set};
        use chrono::Utc;

        // Extract HIL-specific configuration
        let (title, description, timeout_seconds, timeout_action, required_fields, metadata) = match config {
            crate::workflow::models::NodeType::HumanInLoop {
                title,
                description,
                timeout_seconds,
                timeout_action,
                required_fields,
                metadata,
                ..
            } => (
                title.clone(),
                description.clone(),
                *timeout_seconds,
                timeout_action.clone(),
                required_fields.clone(),
                metadata.clone(),
            ),
            _ => return Err(crate::workflow::errors::SwissPipeError::InvalidInput(
                "Expected HumanInLoop node configuration".to_string()
            )),
        };

        let task_id = uuid::Uuid::new_v4().to_string();
        let now_microseconds = Utc::now().timestamp_micros();

        // Calculate timeout in Unix epoch microseconds if specified
        let timeout_at = timeout_seconds.map(|seconds| {
            (Utc::now() + chrono::Duration::seconds(seconds as i64)).timestamp_micros()
        });

        // Store values for logging before they're moved
        let title_for_log = title.clone();
        let timeout_action_for_log = timeout_action.clone();

        // Use metadata as-is without token enhancement
        let enhanced_metadata = metadata;

        // Create transaction for atomic HIL task creation
        let txn = self.db.begin().await
            .map_err(|e| crate::workflow::errors::SwissPipeError::Generic(
                format!("Failed to begin transaction: {e}")
            ))?;

        // Create HIL task record with all required fields (cloned for race condition check)
        let _hil_task = human_in_loop_tasks::ActiveModel {
            id: Set(task_id.clone()),
            execution_id: Set(execution_id.to_string()),
            node_id: Set(node_id.to_string()),
            node_execution_id: Set(node_execution_id.to_string()),
            workflow_id: Set(workflow_id.to_string()),
            title: Set(title.clone()),
            description: Set(description.clone()),
            status: Set("pending".to_string()),
            timeout_at: Set(timeout_at),
            timeout_action: Set(timeout_action.clone()),
            required_fields: Set(required_fields.clone().map(|fields| {
                serde_json::Value::Array(
                    fields.into_iter().map(serde_json::Value::String).collect()
                )
            })),
            metadata: Set(enhanced_metadata.clone()),
            response_data: Set(None),
            response_received_at: Set(None),
            created_at: Set(now_microseconds),
            updated_at: Set(now_microseconds),
        };

        // Check if task already exists (race condition protection)
        if let Some(existing_task) = human_in_loop_tasks::Entity::find()
            .filter(human_in_loop_tasks::Column::NodeExecutionId.eq(node_execution_id))
            .one(&txn)
            .await
            .map_err(|e| crate::workflow::errors::SwissPipeError::Generic(
                format!("Failed to check existing HIL task: {e}")
            ))?
        {
            // Task already exists - return existing task ID to avoid race condition
            tracing::info!(
                "HIL task already exists for node_execution_id: {} - returning existing task_id: {}",
                node_execution_id, existing_task.id
            );

            // Commit transaction and return existing task ID
            txn.commit().await
                .map_err(|e| crate::workflow::errors::SwissPipeError::Generic(
                    format!("Failed to commit HIL task check transaction: {e}")
                ))?;

            return Ok(existing_task.id);
        }

        // Insert HIL task using raw SQL to bypass SeaORM validation issues
        use sea_orm::{Statement, DbBackend, ConnectionTrait};

        let insert_sql = r#"
            INSERT INTO human_in_loop_tasks (
                id, execution_id, node_id, node_execution_id, workflow_id,
                title, description, status, timeout_at, timeout_action,
                required_fields, metadata, response_data, response_received_at,
                created_at, updated_at
            ) VALUES (
                ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?
            )
        "#;

        let timeout_at_value = timeout_at.map(|t| sea_orm::Value::BigInt(Some(t)));
        let required_fields_str = required_fields.map(|fields| serde_json::to_string(&fields).unwrap_or_default());
        let metadata_str = enhanced_metadata.as_ref().map(|m| serde_json::to_string(m).unwrap_or_default());

        let stmt = Statement::from_sql_and_values(
            DbBackend::Sqlite,
            insert_sql,
            [
                task_id.clone().into(),
                execution_id.to_string().into(),
                node_id.to_string().into(),
                node_execution_id.to_string().into(),
                workflow_id.to_string().into(),
                title.into(),
                description.into(),
                "pending".into(),
                timeout_at_value.unwrap_or(sea_orm::Value::BigInt(None)),
                timeout_action.into(),
                required_fields_str.into(),
                metadata_str.into(),
                sea_orm::Value::Json(None),
                sea_orm::Value::Json(None),
                now_microseconds.into(),
                now_microseconds.into(),
            ]
        );

        txn.execute(stmt).await
            .map_err(|e| {
                tracing::error!("HIL task insertion failed - task_id: {}, node_execution_id: {}, error: {}",
                               task_id, node_execution_id, e);
                crate::workflow::errors::SwissPipeError::Generic(
                    format!("Failed to create HIL task: {e}")
                )
            })?;

        // Task insertion succeeded - log success
        tracing::debug!("HIL task inserted successfully - task_id: {}", task_id);

        // Commit transaction
        txn.commit().await
            .map_err(|e| crate::workflow::errors::SwissPipeError::Generic(
                format!("Failed to commit HIL task transaction: {e}")
            ))?;

        // Audit logging for HIL task creation
        tracing::info!(
            "HIL_AUDIT: Task created - task_id: {}, node_execution_id: {}, execution_id: {}, \
            workflow_id: {}, node_id: {}, title: '{}', timeout_seconds: {:?}, timeout_action: '{}'",
            task_id,
            node_execution_id,
            execution_id,
            workflow_id,
            node_id,
            title_for_log,
            timeout_seconds,
            timeout_action_for_log.as_deref().unwrap_or("denied")
        );

        tracing::info!("Created HIL task {} for execution {} in workflow {} node {}",
                       task_id, execution_id, workflow_id, node_id);

        Ok(task_id)
    }

    /// Check HIL task status in database (replaces in-memory channel check)
    pub async fn get_hil_task_by_node_execution_id(&self, node_execution_id: &str) -> Result<Option<human_in_loop_tasks::Model>> {
        human_in_loop_tasks::Entity::find()
            .filter(human_in_loop_tasks::Column::NodeExecutionId.eq(node_execution_id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| SwissPipeError::Generic(format!("Failed to query HIL task: {e}")))
    }

    /// Get all pending HIL tasks from database
    pub async fn get_pending_tasks(&self) -> Result<Vec<human_in_loop_tasks::Model>> {
        human_in_loop_tasks::Entity::find()
            .filter(human_in_loop_tasks::Column::Status.eq("pending"))
            .all(self.db.as_ref())
            .await
            .map_err(|e| SwissPipeError::Generic(format!("Failed to query pending HIL tasks: {e}")))
    }

    /// Clean up expired HIL tasks (database-only cleanup, no in-memory channels)
    pub async fn cleanup_expired_blocks(&self) -> Result<()> {
        // In the new database job queue model, cleanup is handled by:
        // 1. HIL timeout processor (already implemented below)
        // 2. Job queue cleanup service
        // No in-memory channels to clean up
        Ok(())
    }

    /// Process expired HIL tasks (timeout handling) with concurrency control
    pub async fn process_expired_tasks(&self) -> Result<usize> {
        use sea_orm::{QueryFilter, Set, ActiveModelTrait, TransactionTrait};
        use chrono::Utc;

        // Acquire concurrency control lock to prevent overlapping timeout processing
        let _lock = match self.timeout_processing_lock.try_lock() {
            Ok(lock) => {
                tracing::debug!("Acquired timeout processing lock - proceeding with timeout processing");
                lock
            }
            Err(_) => {
                // Another timeout processing operation is already running
                tracing::info!("Timeout processing already in progress - skipping this cycle to prevent concurrency issues");
                return Ok(0);
            }
        };

        // Start transaction for atomic timeout processing
        let txn = self.db.begin().await
            .map_err(|e| SwissPipeError::Generic(
                format!("Failed to begin timeout processing transaction: {e}")
            ))?;

        // Find tasks that have expired (use transaction for consistency)
        let expired_tasks = human_in_loop_tasks::Entity::find()
            .filter(human_in_loop_tasks::Column::Status.eq("pending"))
            .filter(human_in_loop_tasks::Column::TimeoutAt.is_not_null())
            .filter(human_in_loop_tasks::Column::TimeoutAt.lt(Utc::now().timestamp_micros()))
            .all(&txn)
            .await
            .map_err(|e| SwissPipeError::Generic(format!("Failed to query expired HIL tasks: {e}")))?;

        if expired_tasks.is_empty() {
            // No expired tasks - commit empty transaction and return early
            txn.commit().await
                .map_err(|e| SwissPipeError::Generic(
                    format!("Failed to commit empty timeout processing transaction: {e}")
                ))?;
            return Ok(0);
        }

        let mut processed_count = 0;
        let mut failed_updates = Vec::new();

        for task in expired_tasks {
            let timeout_action = task.timeout_action
                .as_deref()
                .unwrap_or("denied"); // Default to denied if no timeout action

            // Audit logging for timeout processing
            tracing::info!(
                "HIL_AUDIT: Timeout processed - task_id: {}, node_execution_id: {}, timeout_action: '{}', \
                task_title: '{}', timeout_time: {}, task_age_seconds: {}",
                task.id,
                task.node_execution_id,
                timeout_action,
                task.title,
                task.timeout_at.map(|micros| {
                    chrono::DateTime::from_timestamp_micros(micros).unwrap_or_default().to_rfc3339()
                }).unwrap_or("unknown".to_string()),
                (chrono::Utc::now().timestamp_micros() - task.created_at) / 1_000_000
            );

            tracing::info!("Processing expired HIL task {}: applying timeout action '{}'", task.id, timeout_action);

            // Update task status to timeout action
            let mut task_active: human_in_loop_tasks::ActiveModel = task.clone().into();
            task_active.status = Set(timeout_action.to_string());
            let now_micros = chrono::Utc::now().timestamp_micros();
            task_active.response_received_at = Set(Some(now_micros));
            task_active.updated_at = Set(now_micros);

            match task_active.update(&txn).await {
                Ok(_) => {
                    // Resume workflow with timeout response
                    let _timeout_response = HilResponse {
                        decision: timeout_action.to_string(),
                        response_data: Some(serde_json::json!({
                            "timeout": true,
                            "reason": "HIL task expired"
                        })),
                        task_id: task.id.clone(),
                    };

                    let _node_execution_id = task.node_execution_id.clone();
                    // In new database job queue model, timeout would create resumption job
                    // For now, just log the timeout - job creation would be handled elsewhere
                    tracing::info!("HIL task {} timed out with action '{}' - job queue resumption would be triggered here",
                                   task.id, timeout_action);
                    processed_count += 1;
                }
                Err(e) => {
                    tracing::error!("Failed to update expired HIL task {} within transaction: {}", task.id, e);
                    failed_updates.push((task.id.clone(), e.to_string()));
                }
            }
        }

        // Handle transaction commit/rollback based on results
        if !failed_updates.is_empty() {
            // Some updates failed - rollback transaction to maintain consistency
            tracing::error!("Rolling back timeout processing transaction due to {} failed updates: {:?}",
                           failed_updates.len(), failed_updates);

            if let Err(rollback_err) = txn.rollback().await {
                tracing::error!("Failed to rollback timeout processing transaction: {}", rollback_err);
                return Err(SwissPipeError::Generic(format!(
                    "Failed to rollback timeout processing after errors: {rollback_err}. Original errors: {failed_updates:?}"
                )));
            }

            return Err(SwissPipeError::Generic(format!(
                "Timeout processing failed for {} tasks: {:?}",
                failed_updates.len(), failed_updates
            )));
        }

        // All updates succeeded - commit transaction
        txn.commit().await
            .map_err(|e| SwissPipeError::Generic(
                format!("Failed to commit timeout processing transaction: {e}")
            ))?;

        if processed_count > 0 {
            tracing::info!("Successfully processed {} expired HIL tasks in transaction", processed_count);
        }

        Ok(processed_count)
    }

    /// Start background timeout processing (should be called once at startup)
    pub async fn start_timeout_processor(&self, interval_seconds: u64) -> Result<()> {
        let service = Arc::new(self.clone());

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(interval_seconds));

            loop {
                interval.tick().await;

                match service.process_expired_tasks().await {
                    Ok(count) => {
                        if count > 0 {
                            tracing::info!("HIL timeout processor handled {} expired tasks", count);
                        }
                    }
                    Err(e) => {
                        tracing::error!("HIL timeout processor error: {}", e);
                    }
                }

                // Also cleanup any orphaned workflow blocks
                if let Err(e) = service.cleanup_expired_blocks().await {
                    tracing::error!("HIL cleanup error: {}", e);
                }
            }
        });

        tracing::info!("HIL timeout processor started (checking every {} seconds)", interval_seconds);
        Ok(())
    }
}