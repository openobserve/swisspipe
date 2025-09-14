// Configuration types for WorkerPool
// Defines worker pool configuration, worker status, and related utilities

use serde::Serialize;

// Configuration constants
pub const DEFAULT_SHUTDOWN_TIMEOUT_SECS: u64 = 30;
pub const DEFAULT_MAX_RETRIES: i32 = 0;

#[derive(Clone)]
pub struct DelayTimeMultipliers {
    pub seconds: u64,
    pub minutes: u64,
    pub hours: u64,
    pub days: u64,
}

pub const DELAY_TIME_MULTIPLIERS: DelayTimeMultipliers = DelayTimeMultipliers {
    seconds: 1000,
    minutes: 60_000,
    hours: 3_600_000,
    days: 86_400_000,
};

/// Configuration for the WorkerPool behavior
#[derive(Clone, Debug)]
pub struct WorkerPoolConfig {
    pub worker_count: usize,
    pub job_poll_interval_ms: u64,
    pub job_claim_timeout_seconds: i64,
    pub worker_health_check_interval_seconds: u64,
    pub job_claim_cleanup_interval_seconds: u64,
}

impl Default for WorkerPoolConfig {
    fn default() -> Self {
        Self {
            worker_count: 5,
            job_poll_interval_ms: 1000,
            job_claim_timeout_seconds: 300,
            worker_health_check_interval_seconds: 30,
            job_claim_cleanup_interval_seconds: 600,
        }
    }
}

/// Represents a worker in the worker pool
#[derive(Debug)]
pub struct Worker {
    pub id: String,
    pub handle: Option<tokio::task::JoinHandle<()>>,
    pub status: WorkerStatus,
    pub current_job: Option<String>,
    pub processed_count: u64,
    pub last_activity: chrono::DateTime<chrono::Utc>,
}

/// Status of a worker in the pool
#[derive(Debug, Clone, PartialEq)]
pub enum WorkerStatus {
    Idle,
    Busy,
    Shutdown,
}

/// Statistics about the worker pool performance
#[derive(Debug, Serialize)]
pub struct WorkerPoolStats {
    pub total_workers: usize,
    pub idle_workers: usize,
    pub busy_workers: usize,
    pub total_processed_jobs: u64,
    pub queue_pending: u64,
    pub queue_processing: u64,
    pub queue_failed: u64,
    pub queue_dead_letter: u64,
}