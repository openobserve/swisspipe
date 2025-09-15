pub mod handlers;
pub mod middleware;

use axum::{routing::post, Router, middleware as axum_middleware};
use crate::AppState;

pub fn create_segment_routes() -> Router<AppState> {
    Router::new()
        .route("/v1/track", post(handlers::segment_track))
        .route("/v1/identify", post(handlers::segment_identify))
        .route("/v1/page", post(handlers::segment_page))
        .route("/v1/screen", post(handlers::segment_screen))
        .route("/v1/group", post(handlers::segment_group))
        .route("/v1/alias", post(handlers::segment_alias))
        .route("/v1/batch", post(handlers::segment_batch))
        .route("/v1/import", post(handlers::segment_import))
        .layer(axum_middleware::from_fn(middleware::segment_auth_middleware))
}