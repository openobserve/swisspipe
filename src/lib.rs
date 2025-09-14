use std::sync::Arc;

pub mod anthropic;
pub mod api;
pub mod async_execution;
pub mod auth;
pub mod cache;
pub mod config;
pub mod database;
pub mod email;
pub mod utils;
pub mod workflow;

pub use database::establish_connection;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<sea_orm::DatabaseConnection>,
    pub engine: Arc<workflow::engine::WorkflowEngine>,
    pub config: Arc<config::Config>,
    pub worker_pool: Arc<async_execution::worker_pool::WorkerPool>,
    pub workflow_cache: Arc<cache::WorkflowCache>,
    pub delay_scheduler: Arc<async_execution::DelayScheduler>,
}