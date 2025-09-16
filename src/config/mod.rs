use crate::workflow::errors::SwissPipeError;
use crate::async_execution::worker_pool::WorkerPoolConfig;
use std::env;
use url::Url;

#[derive(Clone, Debug)]
pub struct Config {
    pub username: String,
    pub password: String,
    pub database_url: String,
    pub port: u16,
    pub worker_pool: WorkerPoolConfig,
    pub google_oauth: Option<GoogleOAuthConfig>,
    pub execution_retention_count: u64,
    pub cleanup_interval_minutes: u64,
}

#[derive(Clone, Debug)]
pub struct GoogleOAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub allowed_domains: Vec<String>,
    pub redirect_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self, SwissPipeError> {
        let username = env::var("SP_USERNAME")
            .map_err(|_| SwissPipeError::Config("SP_USERNAME environment variable is required".to_string()))?;
        let password = env::var("SP_PASSWORD")
            .map_err(|_| SwissPipeError::Config("SP_PASSWORD environment variable is required".to_string()))?;
        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite:data/swisspipe.db?mode=rwc".to_string());

        // Validate database URL format
        Self::validate_database_url(&database_url)?;
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

        // Execution cleanup configuration
        let execution_retention_count = env::var("SP_EXECUTION_RETENTION_COUNT")
            .unwrap_or_else(|_| "1000".to_string())
            .parse()
            .map_err(|_| SwissPipeError::Config("Invalid SP_EXECUTION_RETENTION_COUNT value".to_string()))?;

        let cleanup_interval_minutes = env::var("SP_CLEANUP_INTERVAL_MINUTES")
            .unwrap_or_else(|_| "1".to_string())
            .parse()
            .map_err(|_| SwissPipeError::Config("Invalid SP_CLEANUP_INTERVAL_MINUTES value".to_string()))?;

        let worker_pool_config = WorkerPoolConfig {
            worker_count,
            job_poll_interval_ms,
            job_claim_timeout_seconds,
            worker_health_check_interval_seconds,
            job_claim_cleanup_interval_seconds,
        };

        // Google OAuth configuration (optional)
        let google_oauth = if let (Ok(client_id), Ok(client_secret)) = (
            env::var("GOOGLE_OAUTH_CLIENT_ID"),
            env::var("GOOGLE_OAUTH_CLIENT_SECRET")
        ) {
            let allowed_domains = env::var("GOOGLE_OAUTH_ALLOWED_DOMAINS")
                .unwrap_or_default()
                .split(',')
                .filter_map(|domain| {
                    let trimmed = domain.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                })
                .collect();

            let redirect_url = env::var("GOOGLE_OAUTH_REDIRECT_URL")
                .unwrap_or_else(|_| format!("http://localhost:{port}/auth/google/callback"));

            // Validate the redirect URL format
            Url::parse(&redirect_url)
                .map_err(|e| SwissPipeError::Config(format!("Invalid GOOGLE_OAUTH_REDIRECT_URL '{redirect_url}': {e}")))?;

            // Validate that the redirect URL ends with the expected path
            if !redirect_url.ends_with("/auth/google/callback") {
                return Err(SwissPipeError::Config(
                    format!("GOOGLE_OAUTH_REDIRECT_URL must end with '/auth/google/callback', got: {redirect_url}")
                ));
            }

            Some(GoogleOAuthConfig {
                client_id,
                client_secret,
                allowed_domains,
                redirect_url,
            })
        } else {
            tracing::info!("Google OAuth not configured - using basic auth only");
            None
        };

        Ok(Config {
            username,
            password,
            database_url,
            port,
            worker_pool: worker_pool_config,
            google_oauth,
            execution_retention_count,
            cleanup_interval_minutes,
        })
    }

    /// Validate database URL format and create directories for SQLite if needed
    fn validate_database_url(database_url: &str) -> Result<(), SwissPipeError> {
        if database_url.starts_with("sqlite:") {
            // Handle SQLite database - ensure data directory exists
            if let Some(db_path_str) = database_url.strip_prefix("sqlite:") {
                if let Some(db_path) = db_path_str.split('?').next() {
                    if let Some(parent) = std::path::Path::new(db_path).parent() {
                        std::fs::create_dir_all(parent)
                            .map_err(|e| SwissPipeError::Config(format!("Failed to create SQLite data directory: {e}")))?;
                    }
                }
            }
            tracing::info!("Using SQLite database");
        } else if database_url.starts_with("postgres://") || database_url.starts_with("postgresql://") {
            // Handle PostgreSQL database - validate URL format
            Url::parse(database_url)
                .map_err(|e| SwissPipeError::Config(format!("Invalid PostgreSQL DATABASE_URL format: {e}")))?;
            tracing::info!("Using PostgreSQL database");
        } else {
            return Err(SwissPipeError::Config(
                format!("Unsupported database URL format: '{database_url}'. Supported formats: 'sqlite:path/to/db.db' or 'postgresql://user:pass@host:port/database'")
            ));
        }

        Ok(())
    }
}