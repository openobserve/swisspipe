use crate::database::{
    workflow_executions::{self, ExecutionStatus},
    job_queue::{self, JobStatus},
    workflow_execution_steps::{self, StepStatus},
};
use crate::workflow::errors::Result;
use sea_orm::{
    ActiveModelTrait, DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, Set,
    TransactionTrait, QueryOrder,
};
use std::sync::Arc;
use uuid::Uuid;

/// Information about where to resume an execution
#[derive(Debug)]
struct ResumeInfo {
    resume_node: Option<String>,
    interrupted_step_id: Option<String>,
}

/// Simple workflow resumption service
/// Resumes interrupted executions on application startup
#[derive(Clone)]
pub struct ResumptionService {
    db: Arc<DatabaseConnection>,
}

impl ResumptionService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Resume interrupted workflows on startup
    /// This method should be called once during application startup
    pub async fn resume_interrupted_executions(&self) -> Result<usize> {
        tracing::info!("Starting workflow resumption process...");

        let txn = self.db.begin().await?;
        
        // Find all executions that were running when the application stopped
        let interrupted_executions = workflow_executions::Entity::find()
            .filter(
                workflow_executions::Column::Status
                    .is_in([ExecutionStatus::Running.to_string(), ExecutionStatus::Pending.to_string()])
            )
            .all(&txn)
            .await?;

        if interrupted_executions.is_empty() {
            tracing::info!("No interrupted executions found");
            txn.commit().await?;
            return Ok(0);
        }

        let count = interrupted_executions.len();
        tracing::info!("Found {} interrupted executions to resume", count);

        // Get max retries from environment
        let max_retries = std::env::var("SP_WORKFLOW_MAX_RETRIES")
            .ok()
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(0);

        let now = chrono::Utc::now().timestamp_micros();

        for execution in interrupted_executions {
            let execution_id = execution.id.clone();
            
            // Determine resumption point based on execution steps
            let resume_info = self.determine_resume_point(&execution_id, &txn).await?;
            
            // Update execution with resumption information
            let mut execution_model: workflow_executions::ActiveModel = execution.clone().into();
            execution_model.status = Set(ExecutionStatus::Pending.to_string());
            execution_model.current_node_name = Set(resume_info.resume_node.clone());
            execution_model.updated_at = Set(now);
            
            // Only reset started_at if we're starting from the beginning
            if resume_info.resume_node.is_none() {
                execution_model.started_at = Set(None);
            }
            
            execution_model.update(&txn).await?;
            
            match &resume_info.resume_node {
                Some(node_name) => {
                    tracing::info!("Execution {} will resume from step '{}'", execution_id, node_name);
                }
                None => {
                    tracing::debug!("Execution {} will restart from beginning", execution_id);
                }
            }
            
            // Handle interrupted step if there was one
            if let Some(interrupted_step_id) = &resume_info.interrupted_step_id {
                self.reset_interrupted_step(interrupted_step_id, &txn).await?;
            }

            // Check if there's already a pending job for this execution
            let existing_job = job_queue::Entity::find()
                .filter(job_queue::Column::ExecutionId.eq(&execution_id))
                .filter(job_queue::Column::Status.eq(JobStatus::Pending.to_string()))
                .one(&txn)
                .await?;

            if existing_job.is_some() {
                tracing::debug!("Job already exists for execution {}, skipping", execution_id);
                continue;
            }

            // Create a new job queue entry for the execution
            let job = job_queue::ActiveModel {
                id: Set(Uuid::now_v7().to_string()),
                execution_id: Set(execution_id.clone()),
                priority: Set(0), // Default priority for resumed executions
                scheduled_at: Set(now),
                claimed_at: Set(None),
                claimed_by: Set(None),
                max_retries: Set(max_retries),
                retry_count: Set(0), // Reset retry count for resumed execution
                status: Set(JobStatus::Pending.to_string()),
                error_message: Set(None),
                payload: Set(None), // Regular workflow resumption, no special payload
                created_at: Set(now),
                updated_at: Set(now),
            };

            job.insert(&txn).await?;
            tracing::debug!("Created job for resumed execution {}", execution_id);
        }

        txn.commit().await?;
        tracing::info!("Successfully resumed {} executions", count);
        Ok(count)
    }

    /// Clean up stale jobs (optional, for robustness)
    /// Removes jobs that are claimed but the worker is no longer running
    pub async fn cleanup_stale_jobs(&self, stale_timeout_minutes: i32) -> Result<usize> {
        let cutoff_time = chrono::Utc::now().timestamp_micros() - (stale_timeout_minutes as i64 * 60 * 1_000_000);
        
        // Find jobs that have been claimed for too long without completion
        let stale_jobs = job_queue::Entity::find()
            .filter(job_queue::Column::Status.is_in([JobStatus::Claimed.to_string(), JobStatus::Processing.to_string()]))
            .filter(job_queue::Column::ClaimedAt.lt(cutoff_time))
            .all(self.db.as_ref())
            .await?;

        if stale_jobs.is_empty() {
            return Ok(0);
        }

        let count = stale_jobs.len();
        tracing::warn!("Found {} stale jobs, resetting to pending", count);

        let txn = self.db.begin().await?;
        let now = chrono::Utc::now().timestamp_micros();

        for job in stale_jobs {
            let mut job_model: job_queue::ActiveModel = job.into();
            job_model.status = Set(JobStatus::Pending.to_string());
            job_model.claimed_at = Set(None);
            job_model.claimed_by = Set(None);
            job_model.updated_at = Set(now);
            job_model.update(&txn).await?;
        }

        txn.commit().await?;
        tracing::info!("Reset {} stale jobs to pending", count);
        Ok(count)
    }

    /// Determine the appropriate resumption point for an execution
    /// Based on the execution steps, find where the workflow should continue
    async fn determine_resume_point(
        &self,
        execution_id: &str,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<ResumeInfo> {
        // Get all execution steps for this execution, ordered by creation time
        let steps = workflow_execution_steps::Entity::find()
            .filter(workflow_execution_steps::Column::ExecutionId.eq(execution_id))
            .order_by_asc(workflow_execution_steps::Column::CreatedAt)
            .all(txn)
            .await?;

        if steps.is_empty() {
            // No steps exist - start from the beginning
            tracing::debug!("No execution steps found for {}, starting from beginning", execution_id);
            return Ok(ResumeInfo {
                resume_node: None,
                interrupted_step_id: None,
            });
        }

        // Find the first non-completed step or the last interrupted step
        for step in steps {
            match step.status.as_str() {
                "pending" => {
                    // Found a pending step - resume from here
                    tracing::debug!(
                        "Found pending step '{}' for execution {}, will resume here",
                        step.node_name, execution_id
                    );
                    return Ok(ResumeInfo {
                        resume_node: Some(step.node_name),
                        interrupted_step_id: None,
                    });
                }
                "running" => {
                    // Found an interrupted step - reset it and resume from here
                    tracing::debug!(
                        "Found interrupted running step '{}' for execution {}, will reset and resume",
                        step.node_name, execution_id
                    );
                    return Ok(ResumeInfo {
                        resume_node: Some(step.node_name),
                        interrupted_step_id: Some(step.id),
                    });
                }
                "completed" | "skipped" => {
                    // Step is done, continue checking
                    continue;
                }
                "failed" => {
                    // Step failed - execution should have been marked as failed
                    // But since we're here, let's restart from this step
                    tracing::warn!(
                        "Found failed step '{}' for execution {}, will retry from here",
                        step.node_name, execution_id
                    );
                    return Ok(ResumeInfo {
                        resume_node: Some(step.node_name),
                        interrupted_step_id: Some(step.id),
                    });
                }
                _ => continue,
            }
        }

        // All steps are completed - this shouldn't happen for interrupted executions
        // But let's be safe and restart from beginning
        tracing::warn!(
            "All steps completed for interrupted execution {}, restarting from beginning",
            execution_id
        );
        Ok(ResumeInfo {
            resume_node: None,
            interrupted_step_id: None,
        })
    }

    /// Reset an interrupted step back to pending state
    async fn reset_interrupted_step(
        &self,
        step_id: &str,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<()> {
        let step = workflow_execution_steps::Entity::find_by_id(step_id)
            .one(txn)
            .await?;

        if let Some(step_model) = step {
            let mut step_active: workflow_execution_steps::ActiveModel = step_model.into();
            step_active.status = Set(StepStatus::Pending.to_string());
            step_active.started_at = Set(None);
            step_active.completed_at = Set(None);
            step_active.error_message = Set(None);
            
            step_active.update(txn).await?;
            tracing::debug!("Reset interrupted step {} to pending", step_id);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_resumption_service_creation() {
        let db = Arc::new(sea_orm::DatabaseConnection::default());
        let service = ResumptionService::new(db.clone());
        
        // Service should be created successfully
        // Just verify the service holds the database connection
        assert!(Arc::ptr_eq(&service.db, &db));
    }
    
    // Note: Full integration tests would require a real database setup
    // These would be better placed in integration test files
}