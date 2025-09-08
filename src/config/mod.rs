use crate::workflow::errors::SwissPipeError;
use crate::async_execution::worker_pool::WorkerPoolConfig;
use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub username: String,
    pub password: String,
    pub database_url: String,
    pub port: u16,
    pub worker_pool: WorkerPoolConfig,
}

impl Config {
    pub fn from_env() -> Result<Self, SwissPipeError> {
        let username = env::var("SP_USERNAME")
            .unwrap_or_else(|_| "admin".to_string());
        let password = env::var("SP_PASSWORD")
            .unwrap_or_else(|_| "admin".to_string());
        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite:data/swisspipe.db?mode=rwc".to_string());
        let port = env::var("PORT")
            .unwrap_or_else(|_| "3700".to_string())
            .parse()
            .map_err(|_| SwissPipeError::Config("Invalid PORT value".to_string()))?;

        // Worker pool configuration
        let worker_count = env::var("WORKER_COUNT")
            .unwrap_or_else(|_| "5".to_string())
            .parse()
            .map_err(|_| SwissPipeError::Config("Invalid WORKER_COUNT value".to_string()))?;
            
        let job_poll_interval_ms = env::var("JOB_POLL_INTERVAL_MS")
            .unwrap_or_else(|_| "1000".to_string())
            .parse()
            .map_err(|_| SwissPipeError::Config("Invalid JOB_POLL_INTERVAL_MS value".to_string()))?;
            
        let job_claim_timeout_seconds = env::var("JOB_CLAIM_TIMEOUT_SECONDS")
            .unwrap_or_else(|_| "300".to_string())
            .parse()
            .map_err(|_| SwissPipeError::Config("Invalid JOB_CLAIM_TIMEOUT_SECONDS value".to_string()))?;
            
        let worker_health_check_interval_seconds = env::var("WORKER_HEALTH_CHECK_INTERVAL_SECONDS")
            .unwrap_or_else(|_| "30".to_string())
            .parse()
            .map_err(|_| SwissPipeError::Config("Invalid WORKER_HEALTH_CHECK_INTERVAL_SECONDS value".to_string()))?;
            
        let job_claim_cleanup_interval_seconds = env::var("JOB_CLAIM_CLEANUP_INTERVAL_SECONDS")
            .unwrap_or_else(|_| "600".to_string())
            .parse()
            .map_err(|_| SwissPipeError::Config("Invalid JOB_CLAIM_CLEANUP_INTERVAL_SECONDS value".to_string()))?;

        let worker_pool_config = WorkerPoolConfig {
            worker_count,
            job_poll_interval_ms,
            job_claim_timeout_seconds,
            worker_health_check_interval_seconds,
            job_claim_cleanup_interval_seconds,
        };

        // Ensure data directory exists
        if let Some(db_path_str) = database_url.strip_prefix("sqlite:") {
            if let Some(db_path) = db_path_str.split('?').next() {
                if let Some(parent) = std::path::Path::new(db_path).parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| SwissPipeError::Config(format!("Failed to create data directory: {e}")))?;
                }
            }
        }

        Ok(Config {
            username,
            password,
            database_url,
            port,
            worker_pool: worker_pool_config,
        })
    }
}