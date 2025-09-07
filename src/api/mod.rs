pub mod ingestion;
pub mod workflows;

use axum::Router;
use crate::AppState;

pub fn create_router() -> Router<AppState> {
    Router::new()
        .nest("/api/v1", ingestion::routes())
        .nest("/workflows", workflows::routes())
}