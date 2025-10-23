pub mod ai;
pub mod executions;
pub mod health;
pub mod hil;
pub mod ingestion;
pub mod loops;
pub mod middleware;
pub mod schedules;
pub mod script;
pub mod segment;
pub mod settings;
pub mod static_files;
pub mod variables;
pub mod workflows;

use axum::Router;
use crate::{AppState, auth::handlers as auth_handlers};

pub fn create_router() -> Router<AppState> {
    Router::new()
        // Health check route (no auth required)
        .merge(health::routes())
        // API routes (higher priority)
        .nest("/api/v1", ingestion::routes())
        .nest("/api/v1/hil", hil::routes())
        .nest("/api", segment::create_segment_routes())
        // Admin routes with logging middleware
        .nest("/api/admin/v1/workflows", workflows::routes())
        .nest("/api/admin/v1/executions", executions::routes())
        .nest("/api/admin/v1/loops", loops::routes())
        .nest("/api/admin/v1/script", script::routes())
        .nest("/api/admin/v1/ai", ai::create_ai_routes())
        .nest("/api/admin/v1/settings", settings::routes())
        .nest("/api/admin/v1/hil", hil::routes())
        .nest("/api/admin/v1/variables", variables::routes())
        .nest("/api/admin/v1", schedules::routes())
        .nest("/auth", auth_handlers::routes())
        // Static file routes (lower priority, fallback for SPA)
        .merge(static_files::routes())
}