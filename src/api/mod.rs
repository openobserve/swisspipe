pub mod ai;
pub mod executions;
pub mod ingestion;
pub mod script;
pub mod segment;
pub mod workflows;

use axum::Router;
use crate::AppState;

pub fn create_router() -> Router<AppState> {
    Router::new()
        .nest("/api/v1", ingestion::routes())
        .nest("/api/v1/ai", ai::create_ai_routes())
        .nest("/api", segment::create_segment_routes())
        .nest("/api/admin/v1/workflows", workflows::routes())
        .nest("/api/admin/v1/executions", executions::routes())
        .nest("/api/admin/v1/script", script::routes())
}