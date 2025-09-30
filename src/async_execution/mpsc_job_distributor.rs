use std::sync::Arc;
use tokio::sync::mpsc;
use sea_orm::{DatabaseConnection, EntityTrait, ColumnTrait, QueryFilter, QueryOrder, QuerySelect, Set, ActiveModelTrait};
use chrono::Utc;

use crate::database::job_queue::{self, JobStatus};
use crate::workflow::errors::{Result, SwissPipeError};

// Global mutex to ensure single consumer pattern - only one job distributor can claim jobs at a time
static GLOBAL_JOB_CONSUMER_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Job message containing all necessary data for worker processing
#[derive(Debug, Clone)]
pub struct JobMessage {
    pub job_id: String,
    pub execution_id: String,
    pub priority: i32,
    pub payload: Option<String>,
    pub retry_count: i32,
    pub max_retries: i32,
}

/// MPSC Job Distributor - eliminates database race conditions by using single consumer pattern
///
/// Architecture:
/// - Multiple producers (ingestion endpoints, HIL responses) send jobs via channels
/// - Single consumer pulls jobs from database and distributes via channels
/// - Workers receive jobs from channels instead of competing for database claims
#[derive(Clone)]
pub struct MpscJobDistributor {
    db: Arc<DatabaseConnection>,
    job_sender: mpsc::UnboundedSender<JobMessage>,
    shutdown_sender: tokio::sync::broadcast::Sender<()>,
    metrics: Arc<tokio::sync::RwLock<MpscMetrics>>,
}

/// Metrics for monitoring MPSC job distribution performance
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct MpscMetrics {
    pub jobs_distributed: u64,
    pub jobs_failed: u64,
    pub channel_full_events: u64,
    pub database_polling_cycles: u64,
    pub active_jobs_in_channel: u64,
    pub last_distribution_timestamp: Option<i64>,
}

impl MpscJobDistributor {
    /// Create new MPSC job distributor with bounded channel capacity
    pub fn new(
        db: Arc<DatabaseConnection>,
        _channel_capacity: usize,
    ) -> (Self, mpsc::UnboundedReceiver<JobMessage>) {
        let (job_sender, job_receiver) = mpsc::unbounded_channel();
        let (shutdown_sender, _) = tokio::sync::broadcast::channel(1);

        let distributor = Self {
            db,
            job_sender,
            shutdown_sender,
            metrics: Arc::new(tokio::sync::RwLock::new(MpscMetrics::default())),
        };

        (distributor, job_receiver)
    }

    /// Start the single consumer that polls database and distributes jobs
    /// This eliminates the "thundering herd" problem by having only one process claim jobs
    pub async fn start_consumer(&self, polling_interval_ms: u64) -> Result<()> {
        let db = self.db.clone();
        let job_sender = self.job_sender.clone();
        let metrics = self.metrics.clone();
        let mut shutdown_receiver = self.shutdown_sender.subscribe();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(polling_interval_ms));

            tracing::debug!("MPSC_AUDIT: Job consumer started - polling_interval_ms: {}, channel_capacity: unbounded",
                          polling_interval_ms);

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Update polling cycle metrics
                        {
                            let mut m = metrics.write().await;
                            m.database_polling_cycles += 1;
                        }

                        // Claim and distribute jobs in single transaction to prevent race conditions
                        match Self::claim_and_distribute_jobs(&db, &job_sender, &metrics).await {
                            Ok(distributed_count) => {
                                if distributed_count > 0 {
                                    tracing::trace!("MPSC_AUDIT: Jobs distributed in cycle - count: {}", distributed_count);
                                }
                            }
                            Err(e) => {
                                tracing::error!(
                                    error = %e,
                                    "MPSC_AUDIT: Job distribution error"
                                );

                                // Update failure metrics
                                let mut m = metrics.write().await;
                                m.jobs_failed += 1;
                            }
                        }
                    }
                    _ = shutdown_receiver.recv() => {
                        tracing::info!("MPSC_AUDIT: Job consumer shutting down gracefully");
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Single atomic operation to claim jobs and distribute via channels
    /// Eliminates race conditions by having only one consumer access the job queue
    async fn claim_and_distribute_jobs(
        db: &DatabaseConnection,
        job_sender: &mpsc::UnboundedSender<JobMessage>,
        metrics: &Arc<tokio::sync::RwLock<MpscMetrics>>,
    ) -> Result<usize> {
        use sea_orm::TransactionTrait;

        // Acquire global lock to ensure single consumer pattern (with timeout to prevent deadlocks)
        let _lock = match tokio::time::timeout(
            tokio::time::Duration::from_secs(30), // 30 second timeout
            GLOBAL_JOB_CONSUMER_LOCK.lock()
        ).await {
            Ok(lock) => lock,
            Err(_) => {
                tracing::error!(
                    timeout_seconds = 30,
                    "MPSC_AUDIT: Global job consumer lock timeout - potential deadlock detected"
                );
                return Err(SwissPipeError::Generic(
                    "Job consumer lock timeout - potential deadlock detected".to_string()
                ));
            }
        };

        tracing::trace!("MPSC_AUDIT: Acquired global job consumer lock");

        // Find available jobs without holding a transaction
        let pending_jobs = job_queue::Entity::find()
            .filter(job_queue::Column::Status.eq(JobStatus::Pending.to_string()))
            .filter(job_queue::Column::ClaimedAt.is_null())
            .order_by_desc(job_queue::Column::Priority)
            .order_by_asc(job_queue::Column::ScheduledAt)
            .limit(10) // Reduced batch size for shorter transactions
            .all(db)
            .await
            .map_err(|e| SwissPipeError::Generic(format!("Failed to query pending jobs: {e}")))?;

        if pending_jobs.is_empty() {
            return Ok(0);
        }

        let consumer_id = format!("mpsc-consumer-{}", std::process::id());
        let mut distributed_count = 0;

        // Claim jobs individually with short transactions
        for job in pending_jobs {
            // Each job gets its own short transaction
            let txn = db.begin().await
                .map_err(|e| SwissPipeError::Generic(format!("Failed to begin individual job claim transaction: {e}")))?;

            // Double-check the job is still available (prevent race conditions)
            let current_job = job_queue::Entity::find_by_id(&job.id)
                .one(&txn)
                .await
                .map_err(|e| SwissPipeError::Generic(format!("Failed to verify job availability: {e}")))?;

            let current_job = match current_job {
                Some(job) if job.status == JobStatus::Pending.to_string() && job.claimed_at.is_none() => job,
                Some(_) => {
                    // Job already claimed or processed, skip it
                    if let Err(e) = txn.commit().await {
                        tracing::warn!(
                            job_id = %job.id,
                            error = %e,
                            "MPSC_AUDIT: Failed to commit transaction for already claimed job"
                        );
                    }
                    continue;
                }
                None => {
                    // Job no longer exists, skip it
                    if let Err(e) = txn.commit().await {
                        tracing::warn!(
                            job_id = %job.id,
                            error = %e,
                            "MPSC_AUDIT: Failed to commit transaction for non-existent job"
                        );
                    }
                    continue;
                }
            };

            // Update job status to claimed
            let claim_timestamp = Utc::now().timestamp_micros();
            let mut job_active: job_queue::ActiveModel = current_job.clone().into();
            job_active.status = Set(JobStatus::Claimed.to_string());
            job_active.claimed_at = Set(Some(claim_timestamp));
            job_active.claimed_by = Set(Some(consumer_id.clone()));
            job_active.updated_at = Set(claim_timestamp);

            match job_active.update(&txn).await {
                Ok(_) => {
                    // Commit immediately after claiming
                    if let Err(e) = txn.commit().await {
                        tracing::error!(
                            job_id = %current_job.id,
                            execution_id = %current_job.execution_id,
                            error = %e,
                            "MPSC_AUDIT: Failed to commit job claim"
                        );
                        continue;
                    }
                }
                Err(e) => {
                    tracing::debug!("MPSC_AUDIT: Failed to claim job {} (likely race condition): {}", current_job.id, e);
                    if let Err(rollback_err) = txn.rollback().await {
                        tracing::warn!(
                            job_id = %current_job.id,
                            execution_id = %current_job.execution_id,
                            error = %rollback_err,
                            "MPSC_AUDIT: Failed to rollback transaction for job"
                        );
                    }
                    continue;
                }
            }

            // Create job message for channel distribution (after transaction is committed)
            let job_message = JobMessage {
                job_id: current_job.id.clone(),
                execution_id: current_job.execution_id.clone(),
                priority: current_job.priority,
                payload: current_job.payload.clone(),
                retry_count: current_job.retry_count,
                max_retries: current_job.max_retries,
            };

            // Send to worker channel (unbounded, so this won't block)
            if let Err(e) = job_sender.send(job_message) {
                tracing::error!(
                    job_id = %current_job.id,
                    execution_id = %current_job.execution_id,
                    error = %e,
                    "MPSC_AUDIT: Failed to send job to channel"
                );

                // Increment failure metrics
                let mut m = metrics.write().await;
                m.jobs_failed += 1;

                continue;
            }

            distributed_count += 1;

            tracing::debug!("MPSC_AUDIT: Job claimed and distributed - job_id: {}, execution_id: {}, priority: {}",
                          current_job.id, current_job.execution_id, current_job.priority);
        }

        // Update success metrics
        if distributed_count > 0 {
            let mut m = metrics.write().await;
            m.jobs_distributed += distributed_count as u64;
            m.active_jobs_in_channel = m.active_jobs_in_channel.saturating_add(distributed_count as u64);
            m.last_distribution_timestamp = Some(Utc::now().timestamp_micros());

            tracing::info!("MPSC_AUDIT: Jobs individually claimed and distributed - count: {}, consumer_id: {}",
                          distributed_count, consumer_id);
        }

        Ok(distributed_count)
    }

    /// Queue a new job (called by job producers like ingestion endpoints, HIL responses)
    /// Jobs are inserted into database, then picked up by the single consumer
    pub async fn queue_job(
        &self,
        execution_id: String,
        priority: i32,
        payload: Option<String>,
        max_retries: i32,
    ) -> Result<String> {
        let job_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().timestamp_micros();

        let new_job = job_queue::ActiveModel {
            id: Set(job_id.clone()),
            execution_id: Set(execution_id.clone()),
            priority: Set(priority),
            scheduled_at: Set(now),
            claimed_at: Set(None),
            claimed_by: Set(None),
            max_retries: Set(max_retries),
            retry_count: Set(0),
            status: Set(JobStatus::Pending.to_string()),
            error_message: Set(None),
            payload: Set(payload),
            created_at: Set(now),
            updated_at: Set(now),
        };

        new_job.insert(self.db.as_ref()).await
            .map_err(|e| SwissPipeError::Generic(format!("Failed to queue job: {e}")))?;

        tracing::info!("MPSC_AUDIT: Job queued for distribution - job_id: {}, execution_id: {}, priority: {}",
                      job_id, execution_id, priority);

        Ok(job_id)
    }

    /// Mark job as completed (called by workers after successful processing)
    pub async fn complete_job(&self, job_id: &str) -> Result<()> {
        let job = job_queue::Entity::find_by_id(job_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| SwissPipeError::Generic(format!("Failed to find job {job_id}: {e}")))?
            .ok_or_else(|| SwissPipeError::Generic(format!("Job not found: {job_id}")))?;

        let mut job_active: job_queue::ActiveModel = job.into();
        job_active.status = Set(JobStatus::Completed.to_string());
        job_active.updated_at = Set(Utc::now().timestamp_micros());

        job_active.update(self.db.as_ref()).await
            .map_err(|e| SwissPipeError::Generic(format!("Failed to complete job {job_id}: {e}")))?;

        // Update metrics - reduce active jobs count
        {
            let mut m = self.metrics.write().await;
            m.active_jobs_in_channel = m.active_jobs_in_channel.saturating_sub(1);
        }

        tracing::debug!("MPSC_AUDIT: Job completed - job_id: {}", job_id);

        Ok(())
    }

    /// Mark job as failed with retry logic
    pub async fn fail_job(&self, job_id: &str, error_message: &str) -> Result<bool> {
        let job = job_queue::Entity::find_by_id(job_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| SwissPipeError::Generic(format!("Failed to find job {job_id}: {e}")))?
            .ok_or_else(|| SwissPipeError::Generic(format!("Job not found: {job_id}")))?;

        let mut job_active: job_queue::ActiveModel = job.clone().into();

        let will_retry = job.retry_count < job.max_retries;

        if will_retry {
            // Retry: reset to pending status and increment retry count
            job_active.status = Set(JobStatus::Pending.to_string());
            job_active.retry_count = Set(job.retry_count + 1);
            job_active.claimed_at = Set(None);
            job_active.claimed_by = Set(None);
            job_active.error_message = Set(Some(error_message.to_string()));
        } else {
            // Max retries reached: mark as failed permanently
            job_active.status = Set(JobStatus::Failed.to_string());
            job_active.error_message = Set(Some(error_message.to_string()));
        }

        job_active.updated_at = Set(Utc::now().timestamp_micros());

        job_active.update(self.db.as_ref()).await
            .map_err(|e| SwissPipeError::Generic(format!("Failed to update failed job {job_id}: {e}")))?;

        // Update metrics
        {
            let mut m = self.metrics.write().await;
            if !will_retry {
                m.jobs_failed += 1;
            }
            m.active_jobs_in_channel = m.active_jobs_in_channel.saturating_sub(1);
        }

        tracing::info!("MPSC_AUDIT: Job failed - job_id: {}, will_retry: {}, retry_count: {}/{}, error: {}",
                      job_id, will_retry, job.retry_count + 1, job.max_retries, error_message);

        Ok(will_retry)
    }

    /// Get current metrics for monitoring and observability
    pub async fn get_metrics(&self) -> MpscMetrics {
        self.metrics.read().await.clone()
    }

    /// Graceful shutdown of MPSC system
    pub async fn shutdown(&self) -> Result<()> {
        tracing::info!("MPSC_AUDIT: Initiating graceful shutdown");

        // Signal shutdown to consumer
        if let Err(e) = self.shutdown_sender.send(()) {
            tracing::warn!(
                error = %e,
                "MPSC_AUDIT: Failed to send shutdown signal"
            );
        }

        // Give consumer time to process remaining jobs
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        tracing::info!("MPSC_AUDIT: MPSC shutdown completed");
        Ok(())
    }
}

impl std::fmt::Debug for MpscJobDistributor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MpscJobDistributor")
            .field("db", &"DatabaseConnection")
            .field("job_sender", &"UnboundedSender")
            .field("shutdown_sender", &"BroadcastSender")
            .finish()
    }
}