use crate::database::{
    workflow_executions::{self, ExecutionStatus},
    workflow_execution_steps::{self, StepStatus},
    job_queue::{self, JobStatus},
    entities,
};
use crate::utils::validation;
use crate::workflow::{
    errors::{Result, SwissPipeError},
};
use sea_orm::{
    ActiveModelTrait, DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, Set,
    QuerySelect, QueryOrder, TransactionTrait,
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
        // Validate basic inputs first
        validation::validate_workflow_id(&workflow_id)?;
        validation::validate_priority(priority)?;
        
        // Sanitize headers by removing dangerous ones instead of rejecting the request
        let sanitized_headers = validation::validate_and_sanitize_headers(&headers)?;
        
        let execution_id = Uuid::now_v7().to_string();
        let now = chrono::Utc::now().timestamp_micros();

        // Create complete execution data structure
        let execution_data = serde_json::json!({
            "data": input_data,
            "headers": sanitized_headers,
            "metadata": {}
        });
        
        // Validate and serialize execution data once (eliminates duplicate serialization)
        let serialized_execution_data = validation::validate_and_serialize_execution_data(&execution_data)?;

        // Create execution and job records in a single transaction
        let max_retries = std::env::var("SP_WORKFLOW_MAX_RETRIES")
            .ok()
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(0);

        let txn = self.db.begin().await?;

        // Create workflow execution record
        let execution = workflow_executions::ActiveModel {
            id: Set(execution_id.clone()),
            workflow_id: Set(workflow_id),
            status: Set(ExecutionStatus::Pending.to_string()),
            current_node_id: Set(None),
            input_data: Set(Some(serialized_execution_data)),
            output_data: Set(None),
            error_message: Set(None),
            started_at: Set(None),
            completed_at: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };

        execution.insert(&txn).await?;

        // Create job queue entry
        let job = job_queue::ActiveModel {
            id: Set(Uuid::now_v7().to_string()),
            execution_id: Set(execution_id.clone()),
            priority: Set(priority.unwrap_or(0)),
            scheduled_at: Set(now),
            claimed_at: Set(None),
            claimed_by: Set(None),
            max_retries: Set(max_retries),
            retry_count: Set(0),
            status: Set(JobStatus::Pending.to_string()),
            error_message: Set(None),
            payload: Set(None), // Regular workflow execution, no special payload
            created_at: Set(now),
            updated_at: Set(now),
        };

        job.insert(&txn).await?;

        // Commit the transaction - both records are created atomically
        txn.commit().await?;

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




    /// Cancel execution comprehensively - cancel jobs, steps (delays handled at WorkerPool level)
    pub async fn cancel_execution(&self, execution_id: &str) -> Result<()> {
        validation::validate_execution_id(execution_id)?;
        
        tracing::info!("Starting comprehensive cancellation for execution {}", execution_id);
        
        // Use transaction for atomicity
        let txn = self.db.begin().await?;
        
        // Update execution status first (using transaction)
        let current_execution = workflow_executions::Entity::find_by_id(execution_id)
            .one(&txn)
            .await?
            .ok_or_else(|| SwissPipeError::WorkflowNotFound(execution_id.to_string()))?;

        let mut execution: workflow_executions::ActiveModel = current_execution.clone().into();
        execution.status = Set(ExecutionStatus::Cancelled.to_string());
        execution.current_node_id = Set(None);
        execution.error_message = Set(Some("Execution cancelled by user".to_string()));

        let now = chrono::Utc::now().timestamp_micros();
        execution.completed_at = Set(Some(now));
        execution.update(&txn).await?;

        // Cancel ALL jobs for this execution (pending, claimed, processing)
        let jobs = job_queue::Entity::find()
            .filter(job_queue::Column::ExecutionId.eq(execution_id))
            .filter(job_queue::Column::Status.ne(JobStatus::Completed.to_string()))
            .filter(job_queue::Column::Status.ne(JobStatus::Failed.to_string()))
            .filter(job_queue::Column::Status.ne(JobStatus::DeadLetter.to_string()))
            .all(&txn)
            .await?;

        let mut cancelled_jobs = 0;
        for job_model in jobs {
            let mut job: job_queue::ActiveModel = job_model.into();
            job.status = Set(JobStatus::Failed.to_string());
            job.error_message = Set(Some("Execution cancelled by user".to_string()));
            job.update(&txn).await?;
            cancelled_jobs += 1;
        }

        // Mark all active steps (running or pending) as cancelled
        let active_steps = workflow_execution_steps::Entity::find()
            .filter(workflow_execution_steps::Column::ExecutionId.eq(execution_id))
            .filter(workflow_execution_steps::Column::Status.is_in(["running", "pending"]))
            .all(&txn)
            .await?;
        
        let mut cancelled_steps = 0;
        for step_model in active_steps {
            let mut step: workflow_execution_steps::ActiveModel = step_model.into();
            step.status = Set(StepStatus::Cancelled.to_string());
            step.error_message = Set(Some("Step cancelled by user".to_string()));
            step.completed_at = Set(Some(chrono::Utc::now().timestamp_micros()));
            step.update(&txn).await?;
            cancelled_steps += 1;
        }
        
        txn.commit().await?;
        
        if cancelled_jobs > 0 {
            tracing::info!("Cancelled {} jobs for execution {}", cancelled_jobs, execution_id);
        }
        if cancelled_steps > 0 {
            tracing::info!("Marked {} running steps as cancelled for execution {}", cancelled_steps, execution_id);
        }

        tracing::info!("Completed comprehensive cancellation for execution {} (jobs: {}, steps: {})", 
            execution_id, cancelled_jobs, cancelled_steps);
        Ok(())
    }


    /// Get recent executions with workflow names using join
    pub async fn get_recent_executions_with_workflow_names_filtered(
        &self,
        status: Option<&str>,
        workflow_name: Option<&str>,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<(workflow_executions::Model, Option<entities::Model>)>> {
        let mut query = workflow_executions::Entity::find()
            .find_also_related(entities::Entity)
            .order_by_desc(workflow_executions::Column::CreatedAt);

        // Add status filter if provided
        if let Some(status_filter) = status {
            query = query.filter(workflow_executions::Column::Status.eq(status_filter));
        }

        // Add workflow name filter if provided
        if let Some(name_filter) = workflow_name {
            query = query.filter(entities::Column::Name.contains(name_filter));
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

    /// Get executions by workflow with workflow names using join
    pub async fn get_executions_by_workflow_with_names_filtered(
        &self,
        workflow_id: &str,
        status: Option<&str>,
        workflow_name: Option<&str>,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<(workflow_executions::Model, Option<entities::Model>)>> {
        let mut query = workflow_executions::Entity::find()
            .find_also_related(entities::Entity)
            .filter(workflow_executions::Column::WorkflowId.eq(workflow_id))
            .order_by_desc(workflow_executions::Column::CreatedAt);

        // Add status filter if provided
        if let Some(status_filter) = status {
            query = query.filter(workflow_executions::Column::Status.eq(status_filter));
        }

        // Add workflow name filter if provided
        if let Some(name_filter) = workflow_name {
            query = query.filter(entities::Column::Name.contains(name_filter));
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