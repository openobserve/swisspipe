use crate::database::{workflow_executions, entities};
use crate::workflow::errors::Result;
use sea_orm::{
    DatabaseConnection, EntityTrait, QueryOrder, QuerySelect,
    ColumnTrait, QueryFilter, ConnectionTrait, Statement, PaginatorTrait, DbBackend
};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[derive(Clone, Debug)]
pub struct CleanupService {
    db: Arc<DatabaseConnection>,
    retention_count: u64,
    cleanup_interval: Duration,
}

impl CleanupService {
    pub fn new(db: Arc<DatabaseConnection>, retention_count: u64, cleanup_interval_minutes: u64) -> Result<Self> {
        // Validate retention_count
        if retention_count == 0 {
            return Err(crate::workflow::errors::SwissPipeError::Config(
                "retention_count must be greater than 0 (would delete all executions)".to_string()
            ));
        }
        if retention_count > 100_000 {
            return Err(crate::workflow::errors::SwissPipeError::Config(
                "retention_count too large (maximum 100,000)".to_string()
            ));
        }

        // Validate cleanup_interval_minutes
        if cleanup_interval_minutes == 0 {
            return Err(crate::workflow::errors::SwissPipeError::Config(
                "cleanup_interval_minutes must be greater than 0 (would cause infinite loop)".to_string()
            ));
        }
        if cleanup_interval_minutes > 1440 { // 24 hours
            return Err(crate::workflow::errors::SwissPipeError::Config(
                "cleanup_interval_minutes too large (maximum 1440 minutes / 24 hours)".to_string()
            ));
        }

        // Check for potential overflow in Duration conversion
        let cleanup_interval_seconds = cleanup_interval_minutes.checked_mul(60)
            .ok_or_else(|| crate::workflow::errors::SwissPipeError::Config(
                "cleanup_interval_minutes causes overflow in seconds conversion".to_string()
            ))?;

        Ok(Self {
            db,
            retention_count,
            cleanup_interval: Duration::from_secs(cleanup_interval_seconds),
        })
    }

    /// Start the cleanup service background task
    pub async fn start(&self) -> Result<()> {
        let service = self.clone();
        tokio::spawn(async move {
            service.run_cleanup_loop().await;
        });

        tracing::info!(
            "Cleanup service started: retention_count={}, interval={:?}",
            self.retention_count,
            self.cleanup_interval
        );

        Ok(())
    }

    /// Main cleanup loop that runs periodically
    async fn run_cleanup_loop(&self) {
        loop {
            if let Err(e) = self.cleanup_old_executions().await {
                tracing::error!("Error during cleanup: {}", e);
            }

            sleep(self.cleanup_interval).await;
        }
    }

    /// Clean up old workflow executions, keeping only the most recent ones per workflow
    pub async fn cleanup_old_executions(&self) -> Result<u64> {
        tracing::debug!("Starting workflow executions cleanup");

        // Get all unique workflow IDs
        let workflow_ids = self.get_workflow_ids().await?;
        let mut total_deleted = 0u64;

        for workflow_id in workflow_ids {
            match self.cleanup_executions_for_workflow(&workflow_id).await {
                Ok(deleted_count) => {
                    if deleted_count > 0 {
                        tracing::info!(
                            "Cleaned up {} old executions for workflow {}",
                            deleted_count,
                            workflow_id
                        );
                    }
                    total_deleted += deleted_count;
                }
                Err(e) => {
                    tracing::error!(
                        "Error cleaning up executions for workflow {}: {}",
                        workflow_id,
                        e
                    );
                }
            }
        }

        if total_deleted > 0 {
            tracing::info!("Cleanup completed: removed {} total executions", total_deleted);
        } else {
            tracing::debug!("Cleanup completed: no executions to remove");
        }

        Ok(total_deleted)
    }

    /// Get all workflow IDs - much more efficient than DISTINCT on executions table
    async fn get_workflow_ids(&self) -> Result<Vec<String>> {
        let workflow_ids = entities::Entity::find()
            .select_only()
            .column(entities::Column::Id)
            .into_tuple::<String>()
            .all(self.db.as_ref())
            .await?;

        Ok(workflow_ids)
    }

    /// Clean up executions for a specific workflow, keeping only the most recent ones
    async fn cleanup_executions_for_workflow(&self, workflow_id: &str) -> Result<u64> {
        // Count total executions for this workflow
        let total_count = workflow_executions::Entity::find()
            .filter(workflow_executions::Column::WorkflowId.eq(workflow_id))
            .count(self.db.as_ref())
            .await?;

        // If we have fewer executions than the retention limit, nothing to clean
        if total_count <= self.retention_count {
            return Ok(0);
        }

        let executions_to_delete = total_count - self.retention_count;

        // Get the execution IDs to delete (oldest ones)
        // We order by created_at ASC to get the oldest first, then limit to the number we want to delete
        let executions_to_delete_ids = workflow_executions::Entity::find()
            .filter(workflow_executions::Column::WorkflowId.eq(workflow_id))
            .order_by_asc(workflow_executions::Column::CreatedAt)
            .limit(executions_to_delete)
            .select_only()
            .column(workflow_executions::Column::Id)
            .into_tuple::<String>()
            .all(self.db.as_ref())
            .await?;

        if executions_to_delete_ids.is_empty() {
            return Ok(0);
        }

        // Delete the executions (execution steps will be automatically deleted via CASCADE)
        let executions_deleted = self.delete_executions(&executions_to_delete_ids).await?;

        tracing::debug!(
            "Workflow {}: deleted {} executions (steps deleted automatically via CASCADE)",
            workflow_id,
            executions_deleted
        );

        Ok(executions_deleted)
    }

    /// Delete workflow executions with the given execution IDs
    async fn delete_executions(&self, execution_ids: &[String]) -> Result<u64> {
        if execution_ids.is_empty() {
            return Ok(0);
        }

        // Use batching for large deletion operations to avoid database limits
        const MAX_DELETE_BATCH: usize = 500; // Safe limit for most databases

        if execution_ids.len() <= MAX_DELETE_BATCH {
            return self.delete_execution_batch(execution_ids).await;
        }

        // Process in batches for large deletions
        let mut total_deleted = 0u64;
        for chunk in execution_ids.chunks(MAX_DELETE_BATCH) {
            let deleted = self.delete_execution_batch(chunk).await?;
            total_deleted += deleted;

            tracing::debug!(
                "Deleted batch of {} executions ({}/{} total)",
                deleted,
                total_deleted,
                execution_ids.len()
            );
        }

        Ok(total_deleted)
    }

    /// Delete a batch of workflow executions (internal method)
    async fn delete_execution_batch(&self, execution_ids: &[String]) -> Result<u64> {
        if execution_ids.is_empty() {
            return Ok(0);
        }

        // Create database-appropriate placeholders for the IN clause
        let backend = self.db.get_database_backend();
        let placeholders = match backend {
            DbBackend::Postgres => {
                execution_ids.iter()
                    .enumerate()
                    .map(|(i, _)| format!("${}", i + 1))
                    .collect::<Vec<_>>()
                    .join(",")
            }
            DbBackend::Sqlite => {
                execution_ids.iter()
                    .map(|_| "?")
                    .collect::<Vec<_>>()
                    .join(",")
            }
            _ => {
                // Default to ? for other databases (MySQL, etc.)
                execution_ids.iter()
                    .map(|_| "?")
                    .collect::<Vec<_>>()
                    .join(",")
            }
        };

        let sql = format!(
            "DELETE FROM workflow_executions WHERE id IN ({placeholders})"
        );

        let values: Vec<sea_orm::Value> = execution_ids.iter()
            .map(|id| id.as_str().into()) // Use as_str() to avoid cloning
            .collect();

        let statement = Statement::from_sql_and_values(
            self.db.get_database_backend(),
            &sql,
            values,
        );

        let result = self.db.execute(statement).await?;
        Ok(result.rows_affected())
    }

    /// Get cleanup statistics
    pub async fn get_cleanup_stats(&self) -> Result<CleanupStats> {
        // Get total executions count
        let total_executions = workflow_executions::Entity::find()
            .count(self.db.as_ref())
            .await?;

        // Get per-workflow counts
        let statement = Statement::from_sql_and_values(
            self.db.get_database_backend(),
            "SELECT workflow_id, COUNT(*) as count FROM workflow_executions GROUP BY workflow_id",
            [],
        );

        let query_result = self.db.query_all(statement).await?;
        let mut workflow_counts = Vec::new();

        for row in query_result {
            let workflow_id: String = row.try_get("", "workflow_id")?;
            let count: i64 = row.try_get("", "count")?;
            workflow_counts.push(WorkflowExecutionCount {
                workflow_id,
                execution_count: count as u64,
                exceeds_retention: count as u64 > self.retention_count,
            });
        }

        Ok(CleanupStats {
            total_executions,
            retention_count: self.retention_count,
            workflow_counts,
        })
    }
}

#[derive(Debug, serde::Serialize)]
pub struct CleanupStats {
    pub total_executions: u64,
    pub retention_count: u64,
    pub workflow_counts: Vec<WorkflowExecutionCount>,
}

#[derive(Debug, serde::Serialize)]
pub struct WorkflowExecutionCount {
    pub workflow_id: String,
    pub execution_count: u64,
    pub exceeds_retention: bool,
}