mod api;
mod auth;
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
use tracing_subscriber;

use config::Config;
use database::establish_connection;
use workflow::engine::WorkflowEngine;
use async_execution::{WorkerPool, CleanupService};

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<sea_orm::DatabaseConnection>,
    pub engine: Arc<WorkflowEngine>,
    pub config: Arc<Config>,
    pub worker_pool: Arc<WorkerPool>,
    pub cleanup_service: Arc<CleanupService>,
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

    // Initialize and start cleanup service
    let cleanup_service = Arc::new(CleanupService::new(db.clone()));
    tracing::info!("Starting cleanup service...");
    let _cleanup_handle = cleanup_service.start().await;
    tracing::info!("Cleanup service started successfully");

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

    // Store port before moving config into Arc
    let port = config.port;
    
    // Create app state
    let state = AppState { 
        db, 
        engine, 
        config: Arc::new(config),
        worker_pool: worker_pool.clone(),
        cleanup_service: cleanup_service.clone(),
    };

    // Build application
    let app = api::create_router()
        .layer(middleware::from_fn_with_state(state.clone(), auth::auth_middleware))
        .layer(CorsLayer::permissive())
        .with_state(state);

    // Start server
    let listener = TcpListener::bind(&format!("0.0.0.0:{}", port)).await?;
    
    tracing::info!(
        "SwissPipe server listening on http://0.0.0.0:{}",
        port
    );

    println!("SwissPipe server listening on http://0.0.0.0:{}", port);

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

    // Shutdown cleanup service
    tracing::info!("Shutting down cleanup service...");
    cleanup_service.stop();

    // Shutdown worker pool
    tracing::info!("Shutting down worker pool...");
    worker_pool.shutdown().await?;
    
    tracing::info!("Application shutdown complete");
    Ok(())
}