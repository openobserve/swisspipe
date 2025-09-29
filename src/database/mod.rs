pub mod entities;
pub mod nodes;
pub mod edges;
pub mod migrator;
pub mod workflow_executions;
pub mod workflow_execution_steps;
pub mod job_queue;
pub mod email_queue;
pub mod email_audit_log;
pub mod scheduled_delays;
pub mod node_input_sync;
pub mod sessions;
pub mod csrf_tokens;
pub mod settings;
pub mod http_loop_states;
pub mod human_in_loop_tasks;

use sea_orm::{Database, DatabaseConnection, DbErr, ConnectionTrait, DatabaseBackend};
use sea_orm_migration::MigratorTrait;
use migrator::Migrator;

pub async fn establish_connection(database_url: &str) -> Result<DatabaseConnection, DbErr> {
    tracing::info!("Connecting to database: {}", redact_database_url(database_url));

    // Configure database connection with appropriate settings
    let mut connect_options = sea_orm::ConnectOptions::new(database_url);

    // Set connection pool settings
    connect_options
        .max_connections(100)  // Maximum connections in pool
        .min_connections(5)    // Minimum connections to maintain
        .connect_timeout(std::time::Duration::from_secs(30))
        .acquire_timeout(std::time::Duration::from_secs(30))
        .idle_timeout(std::time::Duration::from_secs(600))  // 10 minutes
        .max_lifetime(std::time::Duration::from_secs(3600)) // 1 hour
        .sqlx_logging(false);   // Disable SQL query logging to avoid log spam

    let db = Database::connect(connect_options).await?;

    // Log database backend type
    let db_backend = db.get_database_backend();
    tracing::info!("Connected to database backend: {:?}", db_backend);

    // Apply database-specific optimizations
    match db_backend {
        DatabaseBackend::Postgres => {
            tracing::debug!("Applying PostgreSQL optimizations");
            // PostgreSQL-specific settings are handled via connection URL parameters
            // Example: postgresql://user:pass@host/db?sslmode=disable&application_name=swisspipe
        }
        DatabaseBackend::Sqlite => {
            tracing::debug!("Applying SQLite optimizations");
            // SQLite optimizations via PRAGMA statements for high concurrency
            let _ = db.execute_unprepared("PRAGMA journal_mode = WAL").await;
            let _ = db.execute_unprepared("PRAGMA synchronous = NORMAL").await;
            let _ = db.execute_unprepared("PRAGMA cache_size = 1000000").await;
            let _ = db.execute_unprepared("PRAGMA temp_store = MEMORY").await;
            let _ = db.execute_unprepared("PRAGMA mmap_size = 268435456").await; // 256MB

            // Additional concurrency optimizations
            let _ = db.execute_unprepared("PRAGMA busy_timeout = 30000").await; // 30 seconds
            let _ = db.execute_unprepared("PRAGMA wal_autocheckpoint = 1000").await;
            let _ = db.execute_unprepared("PRAGMA optimize").await;

            tracing::info!("SQLite WAL mode enabled with concurrency optimizations");
        }
        _ => {
            tracing::debug!("No specific optimizations for this database backend");
        }
    }

    // Run pending migrations
    tracing::info!("Running database migrations...");
    Migrator::up(&db, None).await?;
    tracing::info!("Database migrations completed");

    Ok(db)
}

/// Redact sensitive information from database URLs for logging
fn redact_database_url(url: &str) -> String {
    if let Ok(parsed) = url::Url::parse(url) {
        let mut redacted = parsed.clone();
        if redacted.password().is_some() {
            let _ = redacted.set_password(Some("***"));
        }
        redacted.to_string()
    } else {
        // For non-URL formats like "sqlite:path", just return as-is since no password
        url.to_string()
    }
}

