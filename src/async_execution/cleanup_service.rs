use crate::database::{workflow_executions, workflow_execution_steps};
use crate::workflow::errors::Result;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, PaginatorTrait};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;

pub struct CleanupService {
    db: Arc<DatabaseConnection>,
    retention_hours: u64,
    cleanup_interval_minutes: u64,
    is_running: std::sync::atomic::AtomicBool,
}

impl CleanupService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        // Get retention period from environment variable, default to 1 hour
        let retention_hours = std::env::var("SP_EXECUTION_RETENTION_HOURS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);

        // Get cleanup interval from environment variable, default to 1 minute
        let cleanup_interval_minutes = std::env::var("SP_CLEANUP_INTERVAL_MINUTES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);

        tracing::info!(
            "CleanupService configured: retention_hours={}, cleanup_interval_minutes={}",
            retention_hours,
            cleanup_interval_minutes
        );

        Self {
            db,
            retention_hours,
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
        let cutoff_time = self.calculate_cutoff_time();
        
        tracing::debug!("Performing cleanup for records older than timestamp: {}", cutoff_time);

        // First delete execution steps (they have foreign key to executions)
        let deleted_steps = self.delete_old_execution_steps(cutoff_time).await?;

        // Then delete executions
        let deleted_executions = self.delete_old_executions(cutoff_time).await?;

        Ok((deleted_executions, deleted_steps))
    }

    /// Calculate the cutoff timestamp for cleanup (retention period ago)
    fn calculate_cutoff_time(&self) -> i64 {
        let retention_microseconds = self.retention_hours * 60 * 60 * 1_000_000; // Convert hours to microseconds
        let now = chrono::Utc::now().timestamp_micros();
        now - retention_microseconds as i64
    }

    /// Delete old execution steps
    async fn delete_old_execution_steps(&self, cutoff_time: i64) -> Result<u64> {
        use sea_orm::DeleteResult;

        let result: DeleteResult = workflow_execution_steps::Entity::delete_many()
            .filter(workflow_execution_steps::Column::CreatedAt.lt(cutoff_time))
            .exec(self.db.as_ref())
            .await?;

        Ok(result.rows_affected)
    }

    /// Delete old executions
    async fn delete_old_executions(&self, cutoff_time: i64) -> Result<u64> {
        use sea_orm::DeleteResult;

        let result: DeleteResult = workflow_executions::Entity::delete_many()
            .filter(workflow_executions::Column::CreatedAt.lt(cutoff_time))
            .exec(self.db.as_ref())
            .await?;

        Ok(result.rows_affected)
    }

    /// Get cleanup statistics (for monitoring endpoints)
    pub async fn get_cleanup_stats(&self) -> Result<CleanupStats> {
        let cutoff_time = self.calculate_cutoff_time();

        // Count records that would be cleaned up
        let old_executions = workflow_executions::Entity::find()
            .filter(workflow_executions::Column::CreatedAt.lt(cutoff_time))
            .count(self.db.as_ref())
            .await?;

        let old_steps = workflow_execution_steps::Entity::find()
            .filter(workflow_execution_steps::Column::CreatedAt.lt(cutoff_time))
            .count(self.db.as_ref())
            .await?;

        // Count total records
        let total_executions = workflow_executions::Entity::find()
            .count(self.db.as_ref())
            .await?;

        let total_steps = workflow_execution_steps::Entity::find()
            .count(self.db.as_ref())
            .await?;

        Ok(CleanupStats {
            retention_hours: self.retention_hours,
            cleanup_interval_minutes: self.cleanup_interval_minutes,
            is_running: self.is_running.load(std::sync::atomic::Ordering::SeqCst),
            cutoff_timestamp: cutoff_time,
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
            retention_hours: self.retention_hours,
            cleanup_interval_minutes: self.cleanup_interval_minutes,
            is_running: std::sync::atomic::AtomicBool::new(
                self.is_running.load(std::sync::atomic::Ordering::SeqCst)
            ),
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct CleanupStats {
    pub retention_hours: u64,
    pub cleanup_interval_minutes: u64,
    pub is_running: bool,
    pub cutoff_timestamp: i64,
    pub old_executions: u64,
    pub old_steps: u64,
    pub total_executions: u64,
    pub total_steps: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cutoff_time_calculation() {
        // Set a known retention period
        let db = Arc::new(sea_orm::DatabaseConnection::default()); // Mock for testing
        
        // Test with 2 hours retention
        std::env::set_var("SP_EXECUTION_RETENTION_HOURS", "2");
        let service = CleanupService::new(db);
        
        assert_eq!(service.retention_hours, 2);
        
        let cutoff = service.calculate_cutoff_time();
        let now = chrono::Utc::now().timestamp_micros();
        let expected_cutoff = now - (2 * 60 * 60 * 1_000_000);
        
        // Allow for small timing differences (within 1 second)
        assert!((cutoff - expected_cutoff).abs() < 1_000_000);
    }

    #[test]
    fn test_default_configuration() {
        // Clear environment variables
        std::env::remove_var("SP_EXECUTION_RETENTION_HOURS");
        std::env::remove_var("SP_CLEANUP_INTERVAL_MINUTES");
        
        let db = Arc::new(sea_orm::DatabaseConnection::default()); // Mock for testing
        let service = CleanupService::new(db);
        
        assert_eq!(service.retention_hours, 1);
        assert_eq!(service.cleanup_interval_minutes, 1);
    }
}