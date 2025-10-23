pub mod handlers;

use axum::{
    routing::{post, put},
    Router,
};
use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        // Schedule CRUD for specific workflow trigger
        .route(
            "/workflows/:workflow_id/triggers/:node_id/schedule",
            put(handlers::upsert_schedule)
                .get(handlers::get_schedule)
                .patch(handlers::update_enabled)
                .delete(handlers::delete_schedule),
        )
        // Validation endpoint
        .route(
            "/schedules/validate",
            post(handlers::validate_cron),
        )
}
