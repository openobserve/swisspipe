use axum::{
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde_json::{json, Value};

use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health_check))
}

pub async fn health_check() -> Result<Json<Value>, StatusCode> {
    let response = json!({
        "status": "healthy",
        "service": "swisspipe",
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    Ok(Json(response))
}