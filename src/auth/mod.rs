use axum::{
    extract::{Request, State},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::Response,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};

use crate::AppState;

pub async fn auth_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = request.uri().path();
    
    // Skip auth for ingestion endpoints (they use UUID-based auth)
    if path.starts_with("/api/v1/") {
        return Ok(next.run(request).await);
    }
    
    // Require auth for management endpoints
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Basic "))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    let decoded = STANDARD
        .decode(auth_header)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    
    let credentials = String::from_utf8(decoded)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    
    let (username, password) = credentials
        .split_once(':')
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    // Use cached config from app state
    if username == state.config.username && password == state.config.password {
        Ok(next.run(request).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}