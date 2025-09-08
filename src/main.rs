mod api;
mod auth;
mod config;
mod database;
mod utils;
mod workflow;

use axum::middleware;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tracing_subscriber;

use config::Config;
use database::establish_connection;
use workflow::engine::WorkflowEngine;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<sea_orm::DatabaseConnection>,
    pub engine: Arc<WorkflowEngine>,
    pub config: Arc<Config>,
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

    // Store port before moving config into Arc
    let port = config.port;
    
    // Create app state
    let state = AppState { db, engine, config: Arc::new(config) };

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

    axum::serve(listener, app).await?;

    Ok(())
}