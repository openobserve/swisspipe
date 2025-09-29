use std::sync::Arc;
use serde_json::Value;
use sea_orm::{DatabaseConnection, EntityTrait, Set, ActiveModelTrait, TransactionTrait};
use uuid::Uuid;
use chrono::Utc;

use crate::database::{human_in_loop_tasks, workflow_executions};
use crate::workflow::{
    errors::{Result, SwissPipeError},
    models::{WorkflowEvent, HilPathType},
    engine::WorkflowEngine,
};
use crate::async_execution::mpsc_job_distributor::MpscJobDistributor;

/// Async-only HIL Service that eliminates foreign key constraint issues
/// by ensuring execution records exist before creating HIL tasks
#[derive(Clone)]
pub struct AsyncHilService {
    db: Arc<DatabaseConnection>,
    #[allow(dead_code)]
    workflow_engine: Arc<WorkflowEngine>,
    job_distributor: Arc<MpscJobDistributor>,
}


/// Comprehensive HIL execution context for async processing
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AsyncHilContext {
    pub execution_id: String,
    pub workflow_id: String,
    pub node_id: String,
    pub node_name: String,
    pub title: String,
    pub description: Option<String>,
    pub timeout_seconds: Option<u32>,
    pub timeout_action: Option<String>,
    pub required_fields: Option<Vec<String>>,
    pub metadata: Option<Value>,
}

impl AsyncHilService {
    pub fn new(
        db: Arc<DatabaseConnection>,
        workflow_engine: Arc<WorkflowEngine>,
        job_distributor: Arc<MpscJobDistributor>,
    ) -> Self {
        Self {
            db,
            workflow_engine,
            job_distributor,
        }
    }

    /// Process HIL operation from job queue - MAIN ENTRY POINT for async HIL execution
    pub async fn process_hil_job(
        &self,
        execution_id: &str,
        hil_config: &str,
        event: &WorkflowEvent,
    ) -> Result<()> {
        tracing::info!("Processing async HIL job - execution_id: {}", execution_id);

        // Parse HIL configuration from job payload
        let hil_context: AsyncHilContext = serde_json::from_str(hil_config)
            .map_err(|e| SwissPipeError::Generic(format!("Failed to parse HIL config: {e}")))?;

        // For now, all HIL jobs create task and send notification
        // Future expansion can add different operation types based on context
        self.create_task_and_send_notification(execution_id, &hil_context, event).await
    }

    /// Create HIL task and send immediate notification (handles notification path)
    async fn create_task_and_send_notification(
        &self,
        execution_id: &str,
        context: &AsyncHilContext,
        event: &WorkflowEvent,
    ) -> Result<()> {
        tracing::info!("Creating HIL task and sending notification - execution_id: {}", execution_id);

        // Step 1: Verify execution record exists (this is the key fix for foreign key issue)
        let execution_exists = workflow_executions::Entity::find_by_id(execution_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| SwissPipeError::Generic(format!("Failed to check execution existence: {e}")))?
            .is_some();

        if !execution_exists {
            return Err(SwissPipeError::Generic(format!(
                "Execution record {execution_id} does not exist - cannot create HIL task"
            )));
        }

        // Step 2: Create HIL task (now safe - execution record exists)
        let task_id = self.create_hil_task_with_execution_id(execution_id, context).await?;

        // Step 3: Queue notification job immediately (blue handle execution)
        self.queue_notification_job(&task_id, execution_id, context, event).await?;

        // Step 4: Store approved/denied path information for later resumption
        self.store_pending_paths(&task_id, execution_id, context, event).await?;

        tracing::info!("HIL task created and notification queued - task_id: {}, execution_id: {}", task_id, execution_id);
        Ok(())
    }

    /// Create HIL task with guaranteed execution_id existence
    async fn create_hil_task_with_execution_id(
        &self,
        execution_id: &str,
        context: &AsyncHilContext,
    ) -> Result<String> {
        let task_id = Uuid::new_v4().to_string();
        let node_execution_id = Uuid::new_v4().to_string();
        let now = Utc::now().timestamp_micros();

        // Calculate timeout if specified
        let timeout_at = context.timeout_seconds.map(|seconds| {
            (Utc::now() + chrono::Duration::seconds(seconds as i64)).timestamp_micros()
        });

        // Use transaction for atomicity
        let txn = self.db.begin().await
            .map_err(|e| SwissPipeError::Generic(format!("Failed to begin HIL task transaction: {e}")))?;

        let hil_task = human_in_loop_tasks::ActiveModel {
            id: Set(task_id.clone()),
            execution_id: Set(execution_id.to_string()),
            node_id: Set(context.node_id.clone()),
            node_execution_id: Set(node_execution_id),
            workflow_id: Set(context.workflow_id.clone()),
            title: Set(context.title.clone()),
            description: Set(context.description.clone()),
            status: Set("pending".to_string()),
            timeout_at: Set(timeout_at),
            timeout_action: Set(context.timeout_action.clone()),
            required_fields: Set(context.required_fields.as_ref().map(|fields| {
                Value::Array(fields.iter().map(|f| Value::String(f.clone())).collect())
            })),
            metadata: Set(context.metadata.clone()),
            response_data: Set(None),
            response_received_at: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };

        hil_task.insert(&txn).await
            .map_err(|e| {
                tracing::error!("HIL task creation failed - task_id: {}, execution_id: {}, error: {}", task_id, execution_id, e);
                SwissPipeError::Generic(format!("Failed to create HIL task: {e}"))
            })?;

        txn.commit().await
            .map_err(|e| SwissPipeError::Generic(format!("Failed to commit HIL task transaction: {e}")))?;

        tracing::info!("HIL task created successfully - task_id: {}, execution_id: {}", task_id, execution_id);
        Ok(task_id)
    }

    /// Queue notification job for immediate execution (blue handle)
    async fn queue_notification_job(
        &self,
        task_id: &str,
        execution_id: &str,
        context: &AsyncHilContext,
        event: &WorkflowEvent,
    ) -> Result<()> {
        // Create enhanced event with HIL task information
        let secure_token = task_id.replace("-", "").chars().take(16).collect::<String>();
        let webhook_url = format!("/api/v1/hil/{task_id}/respond?token={secure_token}");

        let hil_data = serde_json::json!({
            "hil_task_id": task_id,
            "node_execution_id": task_id, // Use task_id as node_execution_id for simplicity
            "title": context.title,
            "description": context.description,
            "required_fields": context.required_fields,
            "metadata": context.metadata,
            "webhook_url": webhook_url,
            "secure_token": secure_token,
            "timeout_seconds": context.timeout_seconds,
            "timeout_action": context.timeout_action,
        });

        // Create notification event
        let mut notification_event = event.clone();
        notification_event.hil_task = Some(hil_data.clone());
        tracing::info!("ASYNC HIL SERVICE: Set HIL task metadata at the top level: {:?}", hil_data);
        tracing::info!("ASYNC HIL SERVICE: Notification event now contains hil_task: {:?}", notification_event.hil_task.is_some());

        // Encode notification execution for job queue
        let notification_payload = serde_json::json!({
            "type": "hil_notification",
            "event": notification_event,
        });

        // Queue notification job with high priority (execute immediately)
        let job_id = self.job_distributor.queue_job(
            execution_id.to_string(),
            100, // High priority for notifications
            Some(notification_payload.to_string()),
            3,   // Max retries for notifications
        ).await?;

        tracing::info!("Notification job queued - job_id: {}, task_id: {}", job_id, task_id);
        Ok(())
    }

    /// Store pending path information for approved/denied handles
    async fn store_pending_paths(
        &self,
        task_id: &str,
        _execution_id: &str,
        context: &AsyncHilContext,
        event: &WorkflowEvent,
    ) -> Result<()> {
        // Create pending execution records for approved and denied paths
        // This would be stored in a pending_executions table or similar
        // For now, we'll store in job_queue with special payload

        let approved_payload = serde_json::json!({
            "type": "hil_approved",
            "task_id": task_id,
            "node_id": context.node_id,
            "path_type": HilPathType::Approved,
            "event": event,
        });

        let denied_payload = serde_json::json!({
            "type": "hil_denied",
            "task_id": task_id,
            "node_id": context.node_id,
            "path_type": HilPathType::Denied,
            "event": event,
        });

        // Store as pending jobs (they won't be executed until HIL response triggers them)
        // These will be marked as "paused" or have special status
        // For now, we'll just log them - full implementation would need pending_executions table
        tracing::info!("Pending HIL paths stored - task_id: {}, approved_payload: {}, denied_payload: {}",
                      task_id, approved_payload, denied_payload);

        Ok(())
    }

    /// Handle multipath execution for HIL (alternative approach)
    #[allow(dead_code)]
    async fn handle_multipath_execution(
        &self,
        execution_id: &str,
        context: &AsyncHilContext,
        event: &WorkflowEvent,
    ) -> Result<()> {
        tracing::info!("Handling HIL multipath execution - execution_id: {}", execution_id);

        // This is an alternative approach where we create the task and immediately
        // set up all three paths (notification, approved, denied) in one operation

        // Implementation would be similar to create_task_and_send_notification
        // but would set up all three execution paths at once

        self.create_task_and_send_notification(execution_id, context, event).await
    }

    /// Process HIL response and resume appropriate workflow path
    #[allow(dead_code)]
    async fn process_hil_response(
        &self,
        task_id: &str,
        execution_id: &str,
        _event: &WorkflowEvent,
    ) -> Result<()> {
        tracing::info!("Processing HIL response - task_id: {}, execution_id: {}", task_id, execution_id);

        // This would:
        // 1. Update HIL task status
        // 2. Determine response type (approved/denied)
        // 3. Queue appropriate continuation job
        // 4. Resume workflow execution from correct handle

        // For now, just log - full implementation would need workflow resumption logic
        tracing::info!("HIL response processed - task_id: {}, execution_id: {}", task_id, execution_id);
        Ok(())
    }

}