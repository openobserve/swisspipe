use axum::{
    extract::{Request, State},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::Response,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};

use crate::AppState;

pub mod google;
pub mod handlers;

pub async fn auth_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = request.uri().path();
    
    // Skip auth for ingestion endpoints, auth endpoints, and static files
    if path.starts_with("/api/v1/") ||
       path.starts_with("/auth/") ||
       path.starts_with("/assets/") ||
       path.starts_with("/monacoeditorwork/") ||
       path == "/" ||
       path == "/index.html" ||
       path == "/favicon.ico" ||
       is_spa_route(path) {
        return Ok(next.run(request).await);
    }

    // Try session-based auth first (check for session cookie)
    if let Some(session_id) = extract_session_id(&request) {
        if handlers::is_session_valid(&session_id, &state.db).await {
            return Ok(next.run(request).await);
        }
    }

    // Fallback to Basic Auth for management endpoints
    if let Some(auth_header) = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Basic "))
    {
        if let Ok(decoded) = STANDARD.decode(auth_header) {
            if let Ok(credentials) = String::from_utf8(decoded) {
                if let Some((username, password)) = credentials.split_once(':') {
                    if username == state.config.username && password == state.config.password {
                        return Ok(next.run(request).await);
                    }
                }
            }
        }
    }

    Err(StatusCode::UNAUTHORIZED)
}

/// Extract session ID from cookie header
fn extract_session_id(request: &Request) -> Option<String> {
    request
        .headers()
        .get("cookie")
        .and_then(|cookie| cookie.to_str().ok())
        .and_then(|cookie_str| {
            cookie_str
                .split(';')
                .find_map(|part| {
                    let trimmed = part.trim();
                    trimmed.strip_prefix("session_id=").map(|session_part| session_part.to_string())
                })
        })
}

/// Check if the path is a SPA route that should serve the frontend
fn is_spa_route(path: &str) -> bool {
    // Allow common SPA routes that don't start with /api/
    !path.starts_with("/api/") &&
    // Exclude static file extensions
    !path.contains('.') ||
    // But include .html files for SPA routing
    path.ends_with(".html")
}