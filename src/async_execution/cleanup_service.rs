use crate::database::{workflow_executions, workflow_execution_steps};
use crate::workflow::errors::Result;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, PaginatorTrait, DeleteResult};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;

pub struct CleanupService {
    db: Arc<DatabaseConnection>,
    retention_count: u64,
    cleanup_interval_minutes: u64,
    is_running: std::sync::atomic::AtomicBool,
}

impl CleanupService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        // Get retention count from environment variable, default to 100
        let retention_count = std::env::var("SP_EXECUTION_RETENTION_COUNT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(100);

        // Get cleanup interval from environment variable, default to 1 minute
        let cleanup_interval_minutes = std::env::var("SP_CLEANUP_INTERVAL_MINUTES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);

        tracing::info!(
            "CleanupService configured: retention_count={}, cleanup_interval_minutes={}",
            retention_count,
            cleanup_interval_minutes
        );

        Self {
            db,
            retention_count,
            cleanup_interval_minutes,
            is_running: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Start the background cleanup task
    pub async fn start(&self) -> JoinHandle<()> {
        use std::sync::atomic::Ordering;

        if self.is_running.swap(true, Ordering::SeqCst) {
            tracing::warn!("CleanupService is already running");
            return tokio::spawn(async {});
        }

        let service = self.clone();
        let handle = tokio::spawn(async move {
            service.run_cleanup_loop().await;
        });

        tracing::info!("CleanupService started successfully");
        handle
    }

    /// Stop the cleanup service
    pub fn stop(&self) {
        use std::sync::atomic::Ordering;
        self.is_running.store(false, Ordering::SeqCst);
        tracing::info!("CleanupService stop requested");
    }

    /// Main cleanup loop that runs continuously
    async fn run_cleanup_loop(&self) {
        use std::sync::atomic::Ordering;

        let interval_duration = Duration::from_secs(self.cleanup_interval_minutes * 60);
        let mut interval = tokio::time::interval(interval_duration);

        tracing::info!("Starting cleanup loop with interval: {:?}", interval_duration);

        while self.is_running.load(Ordering::SeqCst) {
            interval.tick().await;

            match self.perform_cleanup().await {
                Ok((deleted_executions, deleted_steps)) => {
                    if deleted_executions > 0 || deleted_steps > 0 {
                        tracing::info!(
                            "Cleanup completed: deleted {} executions, {} steps",
                            deleted_executions,
                            deleted_steps
                        );
                    } else {
                        tracing::debug!("Cleanup completed: no old records found");
                    }
                }
                Err(e) => {
                    tracing::error!("Cleanup failed: {}", e);
                    // Continue running even if cleanup fails
                }
            }
        }

        tracing::info!("CleanupService stopped");
    }

    /// Perform the actual cleanup of old records
    async fn perform_cleanup(&self) -> Result<(u64, u64)> {
        tracing::debug!("Performing cleanup to keep last {} executions per workflow", self.retention_count);

        // Get executions to delete (to count steps before deletion)
        let executions_to_delete = self.get_executions_to_delete().await?;
        
        if executions_to_delete.is_empty() {
            return Ok((0, 0));
        }

        // Count steps that will be cascade deleted (for logging purposes)
        let steps_count = workflow_execution_steps::Entity::find()
            .filter(workflow_execution_steps::Column::ExecutionId.is_in(executions_to_delete.clone()))
            .count(self.db.as_ref())
            .await?;

        // Delete executions - cascade delete will automatically remove related steps
        let result: DeleteResult = workflow_executions::Entity::delete_many()
            .filter(workflow_executions::Column::Id.is_in(executions_to_delete))
            .exec(self.db.as_ref())
            .await?;

        Ok((result.rows_affected, steps_count))
    }


    /// Get list of execution IDs that should be deleted (keeping last N per workflow)
    async fn get_executions_to_delete(&self) -> Result<Vec<String>> {
        use sea_orm::{Statement, ConnectionTrait, DatabaseBackend};
        
        // SQL to find executions to delete - keeps last N executions per workflow
        // We use a window function to rank executions by created_at within each workflow
        let sql = format!(
            r#"
            SELECT id FROM (
                SELECT id,
                       ROW_NUMBER() OVER (PARTITION BY workflow_id ORDER BY created_at DESC) as row_num
                FROM workflow_executions
            ) ranked
            WHERE row_num > {}
            "#,
            self.retention_count
        );
        
        let statement = Statement::from_string(DatabaseBackend::Sqlite, sql);
        let query_result = self.db.query_all(statement).await?;
        
        let mut executions_to_delete = Vec::new();
        for row in query_result {
            if let Ok(id) = row.try_get_by_index::<String>(0) {
                executions_to_delete.push(id);
            }
        }
        
        Ok(executions_to_delete)
    }

    /// Get cleanup statistics (for monitoring endpoints)
    pub async fn get_cleanup_stats(&self) -> Result<CleanupStats> {
        // Count executions that would be cleaned up (old executions beyond retention count)
        let executions_to_delete = self.get_executions_to_delete().await?;
        let old_executions = executions_to_delete.len() as u64;

        // Count steps that would be deleted (steps belonging to old executions)
        let old_steps = if executions_to_delete.is_empty() {
            0
        } else {
            workflow_execution_steps::Entity::find()
                .filter(workflow_execution_steps::Column::ExecutionId.is_in(executions_to_delete))
                .count(self.db.as_ref())
                .await?
        };

        // Count total records
        let total_executions = workflow_executions::Entity::find()
            .count(self.db.as_ref())
            .await?;

        let total_steps = workflow_execution_steps::Entity::find()
            .count(self.db.as_ref())
            .await?;

        Ok(CleanupStats {
            retention_count: self.retention_count,
            cleanup_interval_minutes: self.cleanup_interval_minutes,
            is_running: self.is_running.load(std::sync::atomic::Ordering::SeqCst),
            old_executions,
            old_steps,
            total_executions,
            total_steps,
        })
    }
}

impl Clone for CleanupService {
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
            retention_count: self.retention_count,
            cleanup_interval_minutes: self.cleanup_interval_minutes,
            is_running: std::sync::atomic::AtomicBool::new(
                self.is_running.load(std::sync::atomic::Ordering::SeqCst)
            ),
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct CleanupStats {
    pub retention_count: u64,
    pub cleanup_interval_minutes: u64,
    pub is_running: bool,
    pub old_executions: u64,
    pub old_steps: u64,
    pub total_executions: u64,
    pub total_steps: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retention_count_configuration() {
        // Clear environment variables first to ensure clean state
        std::env::remove_var("SP_EXECUTION_RETENTION_COUNT");
        std::env::remove_var("SP_CLEANUP_INTERVAL_MINUTES");
        
        let db = Arc::new(sea_orm::DatabaseConnection::default()); // Mock for testing
        
        // Test with 50 executions retention
        std::env::set_var("SP_EXECUTION_RETENTION_COUNT", "50");
        let service = CleanupService::new(db);
        
        assert_eq!(service.retention_count, 50);
        
        // Clean up after test
        std::env::remove_var("SP_EXECUTION_RETENTION_COUNT");
    }

    #[test]
    fn test_default_configuration() {
        // Clear environment variables
        std::env::remove_var("SP_EXECUTION_RETENTION_COUNT");
        std::env::remove_var("SP_CLEANUP_INTERVAL_MINUTES");
        
        let db = Arc::new(sea_orm::DatabaseConnection::default()); // Mock for testing
        let service = CleanupService::new(db);
        
        assert_eq!(service.retention_count, 100);
        assert_eq!(service.cleanup_interval_minutes, 1);
    }
}