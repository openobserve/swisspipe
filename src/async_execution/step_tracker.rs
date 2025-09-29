use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait, Set, ActiveModelTrait, QueryFilter, ColumnTrait};
use serde_json::Value;
use uuid::Uuid;

use crate::database::workflow_execution_steps::{self, StepStatus};
use crate::workflow::errors::{Result, SwissPipeError};

/// Service for tracking workflow execution steps with granular node-level visibility
#[derive(Clone)]
pub struct StepTracker {
    db: Arc<DatabaseConnection>,
}

impl StepTracker {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Create a new execution step when node starts
    pub async fn create_step(
        &self,
        execution_id: &str,
        node_id: &str,
        node_name: &str,
        input_data: Option<&Value>,
    ) -> Result<String> {
        let step_id = Uuid::now_v7().to_string();
        let now = chrono::Utc::now().timestamp_micros();

        let new_step = workflow_execution_steps::ActiveModel {
            id: Set(step_id.clone()),
            execution_id: Set(execution_id.to_string()),
            node_id: Set(node_id.to_string()),
            node_name: Set(node_name.to_string()),
            status: Set(StepStatus::Pending.to_string()),
            input_data: Set(input_data.map(|d| serde_json::to_string(d).unwrap_or_else(|_| "{}".to_string()))),
            output_data: Set(None),
            error_message: Set(None),
            started_at: Set(None),
            completed_at: Set(None),
            created_at: Set(now),
        };

        new_step.insert(&*self.db).await
            .map_err(|e| SwissPipeError::Generic(format!("Failed to create execution step: {e}")))?;

        tracing::debug!("Created execution step {} for node '{}' in execution {}",
                       step_id, node_name, execution_id);

        Ok(step_id)
    }

    /// Mark step as running when node execution starts
    pub async fn mark_step_running(&self, step_id: &str) -> Result<()> {
        let step = workflow_execution_steps::Entity::find_by_id(step_id)
            .one(&*self.db)
            .await
            .map_err(|e| SwissPipeError::Generic(format!("Failed to fetch step for running update: {e}")))?
            .ok_or_else(|| SwissPipeError::Generic(format!("Execution step not found: {step_id}")))?;

        let mut active_step: workflow_execution_steps::ActiveModel = step.into();
        active_step.status = Set(StepStatus::Running.to_string());
        active_step.started_at = Set(Some(chrono::Utc::now().timestamp_micros()));

        active_step.update(&*self.db).await
            .map_err(|e| SwissPipeError::Generic(format!("Failed to mark step as running: {e}")))?;

        tracing::debug!("Marked execution step {} as running", step_id);
        Ok(())
    }

    /// Mark step as completed with output data
    pub async fn complete_step(
        &self,
        step_id: &str,
        output_data: Option<&Value>,
    ) -> Result<()> {
        let step = workflow_execution_steps::Entity::find_by_id(step_id)
            .one(&*self.db)
            .await
            .map_err(|e| SwissPipeError::Generic(format!("Failed to fetch step for completion: {e}")))?
            .ok_or_else(|| SwissPipeError::Generic(format!("Execution step not found: {step_id}")))?;

        let mut active_step: workflow_execution_steps::ActiveModel = step.into();
        active_step.status = Set(StepStatus::Completed.to_string());
        active_step.completed_at = Set(Some(chrono::Utc::now().timestamp_micros()));
        active_step.output_data = Set(output_data.map(|d| serde_json::to_string(d).unwrap_or_else(|_| "{}".to_string())));
        active_step.error_message = Set(None);

        active_step.update(&*self.db).await
            .map_err(|e| SwissPipeError::Generic(format!("Failed to complete step: {e}")))?;

        tracing::debug!("Completed execution step {}", step_id);
        Ok(())
    }

    /// Mark step as failed with error message
    pub async fn fail_step(
        &self,
        step_id: &str,
        error_message: &str,
        output_data: Option<&Value>,
    ) -> Result<()> {
        let step = workflow_execution_steps::Entity::find_by_id(step_id)
            .one(&*self.db)
            .await
            .map_err(|e| SwissPipeError::Generic(format!("Failed to fetch step for failure: {e}")))?
            .ok_or_else(|| SwissPipeError::Generic(format!("Execution step not found: {step_id}")))?;

        let mut active_step: workflow_execution_steps::ActiveModel = step.into();
        active_step.status = Set(StepStatus::Failed.to_string());
        active_step.completed_at = Set(Some(chrono::Utc::now().timestamp_micros()));
        active_step.error_message = Set(Some(error_message.to_string()));
        active_step.output_data = Set(output_data.map(|d| serde_json::to_string(d).unwrap_or_else(|_| "{}".to_string())));

        active_step.update(&*self.db).await
            .map_err(|e| SwissPipeError::Generic(format!("Failed to fail step: {e}")))?;

        tracing::debug!("Failed execution step {} with error: {}", step_id, error_message);
        Ok(())
    }

    /// Mark step as skipped (for condition nodes with false results)
    pub async fn skip_step(&self, step_id: &str, reason: &str) -> Result<()> {
        let step = workflow_execution_steps::Entity::find_by_id(step_id)
            .one(&*self.db)
            .await
            .map_err(|e| SwissPipeError::Generic(format!("Failed to fetch step for skip: {e}")))?
            .ok_or_else(|| SwissPipeError::Generic(format!("Execution step not found: {step_id}")))?;

        let mut active_step: workflow_execution_steps::ActiveModel = step.into();
        active_step.status = Set(StepStatus::Skipped.to_string());
        active_step.completed_at = Set(Some(chrono::Utc::now().timestamp_micros()));
        active_step.error_message = Set(Some(format!("Skipped: {reason}")));

        active_step.update(&*self.db).await
            .map_err(|e| SwissPipeError::Generic(format!("Failed to skip step: {e}")))?;

        tracing::debug!("Skipped execution step {} with reason: {}", step_id, reason);
        Ok(())
    }

    /// Get all steps for an execution (for debugging/monitoring)
    pub async fn get_execution_steps(&self, execution_id: &str) -> Result<Vec<workflow_execution_steps::Model>> {
        let steps = workflow_execution_steps::Entity::find()
            .filter(workflow_execution_steps::Column::ExecutionId.eq(execution_id))
            .all(&*self.db)
            .await
            .map_err(|e| SwissPipeError::Generic(format!("Failed to get execution steps: {e}")))?;

        Ok(steps)
    }

    /// Create a composite step for workflow-level operations (like DAG execution start/end)
    pub async fn create_workflow_step(
        &self,
        execution_id: &str,
        step_name: &str,
        status: StepStatus,
    ) -> Result<String> {
        let step_id = Uuid::now_v7().to_string();
        let now = chrono::Utc::now().timestamp_micros();

        let new_step = workflow_execution_steps::ActiveModel {
            id: Set(step_id.clone()),
            execution_id: Set(execution_id.to_string()),
            node_id: Set("workflow".to_string()), // Special node_id for workflow-level steps
            node_name: Set(step_name.to_string()),
            status: Set(status.to_string()),
            input_data: Set(None),
            output_data: Set(None),
            error_message: Set(None),
            started_at: Set(Some(now)),
            completed_at: Set(if status == StepStatus::Completed { Some(now) } else { None }),
            created_at: Set(now),
        };

        new_step.insert(&*self.db).await
            .map_err(|e| SwissPipeError::Generic(format!("Failed to create workflow step: {e}")))?;

        tracing::debug!("Created workflow step '{}' for execution {}", step_name, execution_id);
        Ok(step_id)
    }
}