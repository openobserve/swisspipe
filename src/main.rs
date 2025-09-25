mod anthropic;
mod api;
mod auth;
mod cache;
mod config;
mod database;
mod utils;
mod workflow;
mod async_execution;
mod email;

use axum::middleware;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

use cache::WorkflowCache;
use config::Config;
use database::establish_connection;
use workflow::engine::WorkflowEngine;
use async_execution::{WorkerPool, ResumptionService, DelayScheduler, CleanupService, HttpLoopScheduler};
use sea_orm::{EntityTrait, ColumnTrait, QueryFilter};

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<sea_orm::DatabaseConnection>,
    pub engine: Arc<WorkflowEngine>,
    pub config: Arc<Config>,
    pub worker_pool: Arc<WorkerPool>,
    pub workflow_cache: Arc<WorkflowCache>,
    pub delay_scheduler: Arc<DelayScheduler>,
    pub http_loop_scheduler: Arc<HttpLoopScheduler>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Load configuration
    let config = Config::from_env()?;
    
    tracing::info!("Starting SwissPipe on port {}", config.port);

    // Connect to database
    let db = establish_connection(&config.database_url).await?;
    let db = Arc::new(db);

    // Tables are created automatically via migrations in establish_connection()

    // Initialize workflow engine
    let engine = Arc::new(WorkflowEngine::new(db.clone())?);

    // Initialize workflow cache (5 minute default TTL)
    let workflow_cache = Arc::new(WorkflowCache::new(Some(300)));

    // Initialize worker pool
    let worker_pool = Arc::new(WorkerPool::new(
        db.clone(),
        engine.clone(),
        Some(config.worker_pool.clone()), // Use configuration
    ));

    // Start worker pool
    tracing::info!("Initializing worker pool...");
    match worker_pool.start().await {
        Ok(()) => {
            tracing::info!("Worker pool started successfully");
        }
        Err(e) => {
            tracing::error!("Failed to start worker pool: {}", e);
            return Err(e.into());
        }
    }


    // Initialize and start input sync timeout manager
    let mut input_sync_manager = crate::workflow::input_sync_manager::InputSyncManager::new(db.clone());
    tracing::info!("Starting input sync timeout manager...");
    match input_sync_manager.start(30).await { // Check for timeouts every 30 seconds
        Ok(()) => {
            tracing::info!("Input sync timeout manager started successfully");
        }
        Err(e) => {
            tracing::error!("Failed to start input sync timeout manager: {}", e);
            // Don't fail startup, just log the error
        }
    }

    // Resume interrupted workflows on startup
    let resumption_service = ResumptionService::new(db.clone());
    match resumption_service.resume_interrupted_executions().await {
        Ok(count) => {
            if count > 0 {
                tracing::info!("Resumed {} interrupted workflow executions", count);
            } else {
                tracing::info!("No interrupted executions to resume");
            }
        }
        Err(e) => {
            tracing::error!("Failed to resume interrupted executions: {}", e);
            // Don't fail startup, just log the error
        }
    }

    // Also clean up any stale jobs (jobs claimed but worker no longer running)
    match resumption_service.cleanup_stale_jobs(10).await { // 10 minutes timeout
        Ok(count) => {
            if count > 0 {
                tracing::info!("Reset {} stale jobs to pending", count);
            }
        }
        Err(e) => {
            tracing::error!("Failed to cleanup stale jobs: {}", e);
        }
    }

    // Initialize and start email queue processor
    let email_service = engine.email_service.clone();
    let _email_db = db.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
        tracing::info!("Email queue processor started");
        
        loop {
            interval.tick().await;
            
            match email_service.process_email_queue().await {
                Ok(processed) => {
                    if processed > 0 {
                        tracing::debug!("Processed {} emails from queue", processed);
                    }
                }
                Err(e) => {
                    tracing::error!("Error processing email queue: {}", e);
                }
            }
            
            // Cleanup expired emails
            match email_service.cleanup_expired_emails().await {
                Ok(cleaned) => {
                    if cleaned > 0 {
                        tracing::info!("Cleaned up {} expired emails from queue", cleaned);
                    }
                }
                Err(e) => {
                    tracing::error!("Error cleaning up expired emails: {}", e);
                }
            }
        }
    });

    // Initialize and start workflow execution cleanup service
    tracing::info!("Starting workflow execution cleanup service...");
    let cleanup_service = match CleanupService::new(
        db.clone(),
        config.execution_retention_count,
        config.cleanup_interval_minutes,
    ) {
        Ok(service) => service,
        Err(e) => {
            tracing::error!("Failed to initialize cleanup service: {}", e);
            return Err(e.into());
        }
    };

    match cleanup_service.start().await {
        Ok(()) => {
            tracing::info!(
                "Workflow execution cleanup service started: retention_count={}, interval={}min",
                config.execution_retention_count,
                config.cleanup_interval_minutes
            );
        }
        Err(e) => {
            tracing::error!("Failed to start cleanup service: {}", e);
            // Don't fail startup, just log the error
        }
    }

    // Initialize and start HTTP loop scheduler
    tracing::info!("Initializing HTTP loop scheduler...");
    let http_loop_scheduler = match HttpLoopScheduler::new(db.clone(), config.http_loop.clone()).await {
        Ok(scheduler) => {
            tracing::info!("HTTP loop scheduler initialized");
            Arc::new(scheduler)
        }
        Err(e) => {
            tracing::error!("Failed to initialize HTTP loop scheduler: {}", e);
            return Err(e.into());
        }
    };

    // Inject HTTP loop scheduler into workflow engine
    engine.set_http_loop_scheduler(http_loop_scheduler.clone())?;
    tracing::info!("HTTP loop scheduler injected into workflow engine");

    // Inject HTTP loop scheduler into worker pool
    worker_pool.set_http_loop_scheduler(http_loop_scheduler.clone()).await?;
    tracing::info!("HTTP loop scheduler injected into worker pool");

    // Resume interrupted HTTP loops
    match http_loop_scheduler.resume_interrupted_loops().await {
        Ok(count) => {
            if count > 0 {
                tracing::info!("Resumed {} interrupted HTTP loops", count);
            } else {
                tracing::info!("No interrupted HTTP loops to resume");
            }
        }
        Err(e) => {
            tracing::error!("Failed to resume interrupted HTTP loops: {}", e);
            // Don't fail startup, just log the error
        }
    }

    // Start HTTP loop scheduler service
    match http_loop_scheduler.start_scheduler_service().await {
        Ok(()) => {
            tracing::info!("HTTP loop scheduler service started");
        }
        Err(e) => {
            tracing::error!("Failed to start HTTP loop scheduler service: {}", e);
            return Err(e.into());
        }
    }

    // Start workflow cache cleanup task
    let cache_cleanup = workflow_cache.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300)); // 5 minutes
        tracing::info!("Workflow cache cleanup task started");
        
        loop {
            interval.tick().await;
            
            match cache_cleanup.cleanup_expired().await {
                0 => {}, // No entries cleaned, no need to log
                count => tracing::debug!("Cleaned up {} expired workflow cache entries", count),
            }
        }
    });

    // Start session and CSRF token cleanup task
    let session_cleanup_db = db.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600)); // 1 hour
        tracing::info!("Session cleanup task started");

        loop {
            interval.tick().await;

            let now = chrono::Utc::now().timestamp_micros();

            // Clean up expired sessions
            match database::sessions::Entity::delete_many()
                .filter(database::sessions::Column::ExpiresAt.lt(now))
                .exec(&*session_cleanup_db)
                .await
            {
                Ok(result) => {
                    if result.rows_affected > 0 {
                        tracing::info!("Cleaned up {} expired sessions", result.rows_affected);
                    }
                }
                Err(e) => {
                    tracing::error!("Error cleaning up expired sessions: {}", e);
                }
            }

            // Clean up expired CSRF tokens
            match database::csrf_tokens::Entity::delete_many()
                .filter(database::csrf_tokens::Column::ExpiresAt.lt(now))
                .exec(&*session_cleanup_db)
                .await
            {
                Ok(result) => {
                    if result.rows_affected > 0 {
                        tracing::info!("Cleaned up {} expired CSRF tokens", result.rows_affected);
                    }
                }
                Err(e) => {
                    tracing::error!("Error cleaning up expired CSRF tokens: {}", e);
                }
            }

            // Clean up used CSRF tokens older than 24 hours
            let twenty_four_hours_ago = now - (24 * 60 * 60 * 1_000_000); // 24 hours in microseconds
            match database::csrf_tokens::Entity::delete_many()
                .filter(database::csrf_tokens::Column::Used.eq(true))
                .filter(database::csrf_tokens::Column::CreatedAt.lt(twenty_four_hours_ago))
                .exec(&*session_cleanup_db)
                .await
            {
                Ok(result) => {
                    if result.rows_affected > 0 {
                        tracing::debug!("Cleaned up {} used CSRF tokens", result.rows_affected);
                    }
                }
                Err(e) => {
                    tracing::error!("Error cleaning up used CSRF tokens: {}", e);
                }
            }
        }
    });

    // Initialize DelayScheduler with tokio timer implementation
    tracing::info!("Initializing DelayScheduler...");
    let delay_scheduler = Arc::new(DelayScheduler::new(worker_pool.get_job_manager(), db.clone()).await
        .map_err(|e| {
            tracing::error!("Failed to initialize DelayScheduler: {}", e);
            e
        })?);
    
    // Restore scheduled delays from database
    match delay_scheduler.restore_from_database().await {
        Ok(count) => {
            if count > 0 {
                tracing::info!("Restored {} scheduled delays from database", count);
            } else {
                tracing::info!("No scheduled delays to restore");
            }
        }
        Err(e) => {
            tracing::error!("Failed to restore scheduled delays: {}", e);
            // Don't fail startup, just log the error
        }
    }
    
    // Link DelayScheduler with WorkerPool
    tracing::info!("Linking DelayScheduler with WorkerPool...");
    worker_pool.set_delay_scheduler(delay_scheduler.clone()).await;

    // Store port before moving config into Arc
    let port = config.port;
    
    // Create app state
    let state = AppState {
        db,
        engine,
        config: Arc::new(config),
        worker_pool: worker_pool.clone(),
        workflow_cache: workflow_cache.clone(),
        delay_scheduler: delay_scheduler.clone(),
        http_loop_scheduler: http_loop_scheduler.clone(),
    };

    // Build application
    let cors = CorsLayer::new()
        // Allow same-origin requests for embedded frontend, plus localhost for development
        .allow_origin([
            format!("http://localhost:{port}").parse().unwrap(),
            format!("http://127.0.0.1:{port}").parse().unwrap(),
            "http://localhost:5173".parse().unwrap(),  // Keep for development
            "http://localhost:5174".parse().unwrap(),  // Keep for development
            "http://127.0.0.1:5173".parse().unwrap(),  // Keep for development
            "http://127.0.0.1:5174".parse().unwrap(),  // Keep for development
        ])
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
            axum::http::header::ACCEPT,
            axum::http::header::COOKIE,
        ])
        .allow_credentials(true);

    let app = api::create_router()
        .layer(middleware::from_fn_with_state(state.clone(), auth::auth_middleware))
        .layer(middleware::from_fn_with_state(state.clone(), api::middleware::admin_api_logging_middleware))
        .layer(cors)
        .with_state(state);

    // Start server
    let listener = TcpListener::bind(&format!("0.0.0.0:{port}")).await?;

    println!("SwissPipe server listening on http://0.0.0.0:{port}");

    // Setup graceful shutdown
    let shutdown_signal = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
        tracing::info!("Received shutdown signal");
    };

    // Start server with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await?;


    // Shutdown worker pool
    tracing::info!("Shutting down worker pool...");
    worker_pool.shutdown().await?;
    
    tracing::info!("Application shutdown complete");
    Ok(())
}