pub mod handlers;

use axum::{routing::post, Router};
use crate::AppState;

pub fn create_ai_routes() -> Router<AppState> {
    Router::new()
        .route("/generate-code", post(handlers::generate_code))
        .route("/generate-workflow", post(handlers::generate_workflow))
}