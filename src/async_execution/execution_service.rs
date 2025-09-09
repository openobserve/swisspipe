use crate::database::{
    workflow_executions::{self, ExecutionStatus},
    workflow_execution_steps::{self, StepStatus},
    job_queue::{self, JobStatus},
};
use crate::utils::validation;
use crate::workflow::{
    errors::{Result, SwissPipeError},
};
use sea_orm::{
    ActiveModelTrait, DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, Set,
    QuerySelect, QueryOrder,
};
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct ExecutionService {
    db: Arc<DatabaseConnection>,
}

impl ExecutionService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Create a new workflow execution record and job queue entry
    pub async fn create_execution(
        &self,
        workflow_id: String,
        input_data: Value,
        headers: std::collections::HashMap<String, String>,
        priority: Option<i32>,
    ) -> Result<String> {
        // Validate all inputs for security and constraints
        validation::validate_workflow_id(&workflow_id)?;
        validation::validate_input_data(&input_data)?;
        
        // Sanitize headers by removing dangerous ones instead of rejecting the request
        let sanitized_headers = validation::validate_and_sanitize_headers(&headers)?;
        
        validation::validate_priority(priority)?;
        
        let execution_id = Uuid::now_v7().to_string();
        let now = chrono::Utc::now().timestamp_micros();

        // Wrap input data and sanitized headers together to preserve headers
        let execution_data = serde_json::json!({
            "data": input_data,
            "headers": sanitized_headers,
            "metadata": {}
        });

        // Create workflow execution record
        let execution = workflow_executions::ActiveModel {
            id: Set(execution_id.clone()),
            workflow_id: Set(workflow_id),
            status: Set(ExecutionStatus::Pending.to_string()),
            current_node_name: Set(None),
            input_data: Set(Some(serde_json::to_string(&execution_data)?)),
            output_data: Set(None),
            error_message: Set(None),
            started_at: Set(None),
            completed_at: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };

        execution.insert(self.db.as_ref()).await?;

        // Create job queue entry
        let job = job_queue::ActiveModel {
            id: Set(Uuid::now_v7().to_string()),
            execution_id: Set(execution_id.clone()),
            priority: Set(priority.unwrap_or(0)),
            scheduled_at: Set(now),
            claimed_at: Set(None),
            claimed_by: Set(None),
            max_retries: Set(3),
            retry_count: Set(0),
            status: Set(JobStatus::Pending.to_string()),
            error_message: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };

        job.insert(self.db.as_ref()).await?;

        tracing::info!("Created execution {} with job queued", execution_id);
        Ok(execution_id)
    }

    /// Get execution details by ID
    pub async fn get_execution(&self, execution_id: &str) -> Result<Option<workflow_executions::Model>> {
        validation::validate_execution_id(execution_id)?;
        let execution = workflow_executions::Entity::find_by_id(execution_id)
            .one(self.db.as_ref())
            .await?;

        Ok(execution)
    }

    /// Get execution steps by execution ID
    pub async fn get_execution_steps(&self, execution_id: &str) -> Result<Vec<workflow_execution_steps::Model>> {
        validation::validate_execution_id(execution_id)?;
        let steps = workflow_execution_steps::Entity::find()
            .filter(workflow_execution_steps::Column::ExecutionId.eq(execution_id))
            .all(self.db.as_ref())
            .await?;

        Ok(steps)
    }

    /// Update execution status
    pub async fn update_execution_status(
        &self,
        execution_id: &str,
        status: ExecutionStatus,
        current_node_name: Option<String>,
        error_message: Option<String>,
    ) -> Result<()> {
        let mut execution: workflow_executions::ActiveModel = workflow_executions::Entity::find_by_id(execution_id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| SwissPipeError::WorkflowNotFound(execution_id.to_string()))?
            .into();

        execution.status = Set(status.to_string());
        execution.current_node_name = Set(current_node_name);
        execution.error_message = Set(error_message);

        let now = chrono::Utc::now().timestamp_micros();
        match status {
            ExecutionStatus::Running => {
                // Always set started_at when changing to Running status if it's currently None
                // We need to check the actual database value, not the ActiveModel state
                let current_execution = workflow_executions::Entity::find_by_id(execution_id)
                    .one(self.db.as_ref())
                    .await?
                    .ok_or_else(|| SwissPipeError::WorkflowNotFound(execution_id.to_string()))?;
                
                if current_execution.started_at.is_none() {
                    execution.started_at = Set(Some(now));
                    tracing::info!("Setting started_at for execution {} at {}", execution_id, now);
                }
            }
            ExecutionStatus::Completed | ExecutionStatus::Failed | ExecutionStatus::Cancelled => {
                execution.completed_at = Set(Some(now));
            }
            _ => {}
        }

        execution.update(self.db.as_ref()).await?;
        tracing::info!("Updated execution {} status to {}", execution_id, status);

        Ok(())
    }

    /// Create an execution step record
    pub async fn create_execution_step(
        &self,
        execution_id: String,
        node_id: String,
        node_name: String,
        input_data: Option<Value>,
    ) -> Result<String> {
        let step_id = Uuid::now_v7().to_string();
        let now = chrono::Utc::now().timestamp_micros();

        let step = workflow_execution_steps::ActiveModel {
            id: Set(step_id.clone()),
            execution_id: Set(execution_id),
            node_id: Set(node_id),
            node_name: Set(node_name),
            status: Set(StepStatus::Pending.to_string()),
            input_data: Set(input_data.map(|v| serde_json::to_string(&v).unwrap_or_default())),
            output_data: Set(None),
            error_message: Set(None),
            started_at: Set(None),
            completed_at: Set(None),
            created_at: Set(now),
        };

        step.insert(self.db.as_ref()).await?;
        tracing::debug!("Created execution step {}", step_id);

        Ok(step_id)
    }

    /// Update execution step
    pub async fn update_execution_step(
        &self,
        step_id: &str,
        status: StepStatus,
        output_data: Option<Value>,
        error_message: Option<String>,
    ) -> Result<()> {
        let mut step: workflow_execution_steps::ActiveModel = workflow_execution_steps::Entity::find_by_id(step_id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| SwissPipeError::WorkflowNotFound(step_id.to_string()))?
            .into();

        step.status = Set(status.to_string());
        step.output_data = Set(output_data.map(|v| serde_json::to_string(&v).unwrap_or_default()));
        step.error_message = Set(error_message);

        let now = chrono::Utc::now().timestamp_micros();
        match status {
            StepStatus::Running => {
                // Always set started_at when changing to Running status if it's currently None
                let current_step = workflow_execution_steps::Entity::find_by_id(step_id)
                    .one(self.db.as_ref())
                    .await?
                    .ok_or_else(|| SwissPipeError::WorkflowNotFound(step_id.to_string()))?;
                
                if current_step.started_at.is_none() {
                    step.started_at = Set(Some(now));
                    tracing::debug!("Setting started_at for step {} at {}", step_id, now);
                }
            }
            StepStatus::Completed | StepStatus::Failed | StepStatus::Skipped => {
                step.completed_at = Set(Some(now));
            }
            _ => {}
        }

        step.update(self.db.as_ref()).await?;
        tracing::debug!("Updated execution step {} status to {}", step_id, status);

        Ok(())
    }

    /// Cancel execution
    pub async fn cancel_execution(&self, execution_id: &str) -> Result<()> {
        validation::validate_execution_id(execution_id)?;
        
        // Update execution status
        self.update_execution_status(
            execution_id,
            ExecutionStatus::Cancelled,
            None,
            Some("Execution cancelled by user".to_string()),
        ).await?;

        // Update any pending job to cancelled status
        let job = job_queue::Entity::find()
            .filter(job_queue::Column::ExecutionId.eq(execution_id))
            .filter(job_queue::Column::Status.eq(JobStatus::Pending.to_string()))
            .one(self.db.as_ref())
            .await?;

        if let Some(job_model) = job {
            let mut job: job_queue::ActiveModel = job_model.into();
            job.status = Set(JobStatus::Failed.to_string());
            job.error_message = Set(Some("Execution cancelled".to_string()));
            job.update(self.db.as_ref()).await?;
        }

        tracing::info!("Cancelled execution {}", execution_id);
        Ok(())
    }

    /// Get executions by workflow with optional status filter
    pub async fn get_executions_by_workflow_filtered(
        &self,
        workflow_id: &str,
        status: Option<&str>,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<workflow_executions::Model>> {
        let mut query = workflow_executions::Entity::find()
            .filter(workflow_executions::Column::WorkflowId.eq(workflow_id));

        // Add status filter if provided
        if let Some(status_filter) = status {
            query = query.filter(workflow_executions::Column::Status.eq(status_filter));
        }

        // Add ordering
        query = query.order_by_desc(workflow_executions::Column::CreatedAt);

        if let Some(limit) = limit {
            query = query.limit(limit);
        }

        if let Some(offset) = offset {
            query = query.offset(offset);
        }

        let executions = query.all(self.db.as_ref()).await?;
        Ok(executions)
    }

    /// Get recent executions with optional status filter
    pub async fn get_recent_executions_filtered(
        &self,
        status: Option<&str>,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<workflow_executions::Model>> {
        let mut query = workflow_executions::Entity::find()
            .order_by_desc(workflow_executions::Column::CreatedAt);

        // Add status filter if provided
        if let Some(status_filter) = status {
            query = query.filter(workflow_executions::Column::Status.eq(status_filter));
        }

        if let Some(limit) = limit {
            query = query.limit(limit);
        } else {
            // Default to 50 if no limit specified
            query = query.limit(50);
        }

        if let Some(offset) = offset {
            query = query.offset(offset);
        }

        let executions = query.all(self.db.as_ref()).await?;
        Ok(executions)
    }
}