use crate::database::job_queue::{self, JobStatus};
use crate::workflow::errors::{Result, SwissPipeError};
use sea_orm::{
    ActiveModelTrait, DatabaseConnection, EntityTrait, QueryFilter,
    ColumnTrait, Set,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct JobManager {
    db: Arc<DatabaseConnection>,
}

impl JobManager {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Claim the next available job for a worker
    pub async fn claim_job(&self, worker_id: &str) -> Result<Option<job_queue::Model>> {
        use sea_orm::{ConnectionTrait, Statement};
        
        // Use atomic UPDATE with WHERE condition to prevent race conditions
        // This ensures only one worker can claim a job by updating only if status is still 'pending'
        let now = chrono::Utc::now().timestamp_micros();
        
        // Atomic claim using raw SQL for database-level atomicity
        let backend = self.db.get_database_backend();
        let (sql, values) = match backend {
            sea_orm::DbBackend::Postgres => {
                (r#"
                UPDATE job_queue
                SET
                    status = $1,
                    claimed_at = $2,
                    claimed_by = $3,
                    updated_at = $4
                WHERE id = (
                    SELECT id FROM job_queue
                    WHERE status = 'pending'
                      AND scheduled_at <= $5
                    ORDER BY priority DESC, scheduled_at ASC
                    LIMIT 1
                )
                RETURNING *
                "#,
                vec![
                    JobStatus::Claimed.to_string().into(),
                    now.into(),
                    worker_id.to_string().into(),
                    now.into(),
                    now.into(),
                ])
            }
            _ => {
                // SQLite and other databases don't support RETURNING in UPDATE
                (r#"
                UPDATE job_queue
                SET
                    status = ?,
                    claimed_at = ?,
                    claimed_by = ?,
                    updated_at = ?
                WHERE id = (
                    SELECT id FROM job_queue
                    WHERE status = 'pending'
                      AND scheduled_at <= ?
                    ORDER BY priority DESC, scheduled_at ASC
                    LIMIT 1
                )
                "#,
                vec![
                    JobStatus::Claimed.to_string().into(),
                    now.into(),
                    worker_id.to_string().into(),
                    now.into(),
                    now.into(),
                ])
            }
        };

        let statement = Statement::from_sql_and_values(backend, sql, values);

        // Execute the atomic update
        let query_result = self.db.as_ref().execute(statement).await?;
        
        if query_result.rows_affected() > 0 {
            // If we successfully claimed a job, get its details
            // Since we used RETURNING *, we could parse the result directly,
            // but for simplicity, let's query it back (the job is now locked by worker_id)
            let claimed_job = job_queue::Entity::find()
                .filter(job_queue::Column::ClaimedBy.eq(worker_id))
                .filter(job_queue::Column::ClaimedAt.eq(now))
                .one(self.db.as_ref())
                .await?;

            if let Some(job) = claimed_job {
                tracing::info!("Worker {} atomically claimed job {} for execution {}", 
                    worker_id, job.id, job.execution_id);
                Ok(Some(job))
            } else {
                tracing::error!("Job claim succeeded but couldn't retrieve job for worker {}", worker_id);
                Ok(None)
            }
        } else {
            // No job was available to claim
            Ok(None)
        }
    }

    /// Update job status
    pub async fn update_job_status(
        &self,
        job_id: &str,
        status: JobStatus,
        error_message: Option<String>,
    ) -> Result<()> {
        let mut job: job_queue::ActiveModel = job_queue::Entity::find_by_id(job_id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| SwissPipeError::WorkflowNotFound(job_id.to_string()))?
            .into();

        job.status = Set(status.to_string());
        job.error_message = Set(error_message);

        job.update(self.db.as_ref()).await?;
        tracing::debug!("Updated job {} status to {}", job_id, status);

        Ok(())
    }

    /// Mark job as processing
    pub async fn start_job_processing(&self, job_id: &str) -> Result<()> {
        self.update_job_status(job_id, JobStatus::Processing, None).await
    }

    /// Complete job successfully
    pub async fn complete_job(&self, job_id: &str) -> Result<()> {
        self.update_job_status(job_id, JobStatus::Completed, None).await
    }

    /// Fail job and handle retry logic
    pub async fn fail_job(&self, job_id: &str, error_message: String) -> Result<()> {
        let job = job_queue::Entity::find_by_id(job_id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| SwissPipeError::WorkflowNotFound(job_id.to_string()))?;

        let mut job_active: job_queue::ActiveModel = job.clone().into();

        if job.retry_count < job.max_retries {
            // Retry the job
            job_active.retry_count = Set(job.retry_count + 1);
            job_active.status = Set(JobStatus::Pending.to_string());
            job_active.claimed_at = Set(None);
            job_active.claimed_by = Set(None);
            job_active.error_message = Set(Some(error_message.clone()));
            
            // Calculate exponential backoff delay
            let backoff_delay_ms = 1000 * 2_i64.pow(job.retry_count as u32);
            let scheduled_at = chrono::Utc::now().timestamp_micros() + (backoff_delay_ms * 1000);
            job_active.scheduled_at = Set(scheduled_at);

            job_active.update(self.db.as_ref()).await?;
            tracing::warn!("Job {} failed, retrying (attempt {}/{}): {}", 
                job_id, job.retry_count + 1, job.max_retries, error_message);
        } else {
            // Move to dead letter queue
            job_active.status = Set(JobStatus::DeadLetter.to_string());
            job_active.error_message = Set(Some(error_message.clone()));

            job_active.update(self.db.as_ref()).await?;
            tracing::error!("Job {} moved to dead letter queue after {} attempts: {}", 
                job_id, job.max_retries, error_message);
        }

        Ok(())
    }

    /// Get jobs claimed by a specific worker
    #[allow(dead_code)]
    pub async fn get_worker_jobs(&self, worker_id: &str) -> Result<Vec<job_queue::Model>> {
        let jobs = job_queue::Entity::find()
            .filter(job_queue::Column::ClaimedBy.eq(worker_id))
            .filter(
                job_queue::Column::Status
                    .is_in([JobStatus::Claimed.to_string(), JobStatus::Processing.to_string()])
            )
            .all(self.db.as_ref())
            .await?;

        Ok(jobs)
    }

    /// Clean up stale claimed jobs (jobs claimed but not processed)
    pub async fn cleanup_stale_jobs(&self, timeout_seconds: i64) -> Result<u64> {
        let cutoff_time = chrono::Utc::now().timestamp_micros() - (timeout_seconds * 1_000_000);
        
        let stale_jobs = job_queue::Entity::find()
            .filter(job_queue::Column::Status.eq(JobStatus::Claimed.to_string()))
            .filter(job_queue::Column::ClaimedAt.lt(cutoff_time))
            .all(self.db.as_ref())
            .await?;

        let mut cleaned_count = 0;
        for job in stale_jobs {
            let mut job_active: job_queue::ActiveModel = job.into();
            job_active.status = Set(JobStatus::Pending.to_string());
            job_active.claimed_at = Set(None);
            job_active.claimed_by = Set(None);
            job_active.scheduled_at = Set(chrono::Utc::now().timestamp_micros());

            job_active.update(self.db.as_ref()).await?;
            cleaned_count += 1;
        }

        if cleaned_count > 0 {
            tracing::warn!("Cleaned up {} stale jobs", cleaned_count);
        }

        Ok(cleaned_count)
    }

    /// Get queue statistics using efficient GROUP BY query
    pub async fn get_queue_stats(&self) -> Result<QueueStats> {
        use sea_orm::{ConnectionTrait, Statement};

        // Use a single GROUP BY query to get all statistics at once
        let statement = Statement::from_sql_and_values(
            self.db.get_database_backend(),
            r#"
            SELECT status, COUNT(*) as count 
            FROM job_queue 
            GROUP BY status
            "#,
            [],
        );

        // Execute the query and process results
        let query_result = self.db.as_ref().query_all(statement).await?;
        
        let mut stats = QueueStats::default();
        
        for row in query_result {
            let status: String = row.try_get("", "status")?;
            let count: i64 = row.try_get("", "count")?;
            let count_u64 = count as u64;
            
            match status.as_str() {
                "pending" => stats.pending = count_u64,
                "claimed" => stats.claimed = count_u64,
                "processing" => stats.processing = count_u64,
                "failed" => stats.failed = count_u64,
                "dead_letter" => stats.dead_letter = count_u64,
                _ => {
                    tracing::warn!("Unknown job status in statistics: {}", status);
                }
            }
        }

        Ok(stats)
    }
}

#[derive(Debug, Default, serde::Serialize)]
pub struct QueueStats {
    pub pending: u64,
    pub claimed: u64,
    pub processing: u64,
    pub failed: u64,
    pub dead_letter: u64,
}