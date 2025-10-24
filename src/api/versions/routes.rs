use super::handlers;
use crate::AppState;
use axum::{routing::get, Router};

/// Create routes for version history endpoints
pub fn create_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/workflows/:workflow_id/versions",
            get(handlers::get_versions).post(handlers::create_version),
        )
        .route(
            "/workflows/:workflow_id/versions/:version_id",
            get(handlers::get_version_detail),
        )
}
