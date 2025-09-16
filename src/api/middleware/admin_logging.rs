use axum::{
    extract::{Request, State},
    http::{HeaderMap, Method, Uri},
    middleware::Next,
    response::Response,
    body::Body,
};
use axum::body::to_bytes;
use base64::prelude::*;
use serde_json::{Value, json};
use std::{time::Instant, sync::Arc};
use chrono;
use sea_orm::{EntityTrait, ColumnTrait, QueryFilter};
use crate::{AppState, database};
use tracing;

/// User information for logging purposes
#[derive(Debug, Clone)]
pub struct LoggingUser {
    pub id: String,
    pub identifier: String, // username, email, or user_id
    pub auth_type: String,  // "basic_auth" or "oauth"
    pub name: Option<String>, // Full name for OAuth users
    pub email: Option<String>, // Email for OAuth users
}

/// Extract user information from request headers and database lookup
async fn extract_user_info(headers: &HeaderMap, db: &Arc<sea_orm::DatabaseConnection>) -> LoggingUser {
    // Check for OAuth session cookie
    if let Some(session_id) = extract_session_id_from_cookies(headers) {
        // Look up session in database to get real user info
        if let Ok(Some(session)) = database::sessions::Entity::find()
            .filter(database::sessions::Column::Id.eq(&session_id))
            .one(db.as_ref())
            .await
        {
            if !session.is_expired() {
                return LoggingUser {
                    id: session.user_id.clone(),
                    identifier: session.email.clone(),
                    auth_type: "oauth".to_string(),
                    name: Some(session.name.clone()),
                    email: Some(session.email.clone()),
                };
            }
        }

        // Fallback if session lookup fails or is expired
        // Use a safe truncation that ensures at least 8 characters or the full session ID
        let safe_id = if session_id.len() >= 8 {
            session_id.chars().take(8).collect()
        } else {
            format!("short_{session_id}") // Prefix to avoid collisions with short IDs
        };

        return LoggingUser {
            id: safe_id,
            identifier: "oauth_user_expired".to_string(),
            auth_type: "oauth".to_string(),
            name: None,
            email: None,
        };
    }

    // Check for Basic Auth
    if let Some(auth_header) = headers.get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(encoded) = auth_str.strip_prefix("Basic ") {
                if let Ok(decoded) = BASE64_STANDARD.decode(encoded) {
                    if let Ok(credentials) = String::from_utf8(decoded) {
                        if let Some((username, _)) = credentials.split_once(':') {
                            return LoggingUser {
                                id: format!("basic_{username}"),
                                identifier: username.to_string(),
                                auth_type: "basic_auth".to_string(),
                                name: None, // Basic auth doesn't have name info
                                email: None,
                            };
                        }
                    }
                }
            }
        }
    }

    // Unknown/unauthenticated user
    LoggingUser {
        id: "unknown".to_string(),
        identifier: "anonymous".to_string(),
        auth_type: "none".to_string(),
        name: None,
        email: None,
    }
}


/// Extract session ID from cookie header
fn extract_session_id_from_cookies(headers: &HeaderMap) -> Option<String> {
    headers
        .get("cookie")?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|cookie| {
            let trimmed = cookie.trim();
            trimmed.strip_prefix("session_id=")
                .map(|session_id| session_id.to_string())
        })
}

/// Get basic request information for logging
fn get_request_info(method: &Method, uri: &Uri, headers: &HeaderMap) -> Value {
    let mut info = serde_json::Map::new();
    info.insert("method".to_string(), Value::String(method.to_string()));
    info.insert("path".to_string(), Value::String(uri.path().to_string()));
    info.insert("query".to_string(), Value::String(uri.query().unwrap_or("").to_string()));

    // Add relevant headers (excluding sensitive ones)
    let mut header_map = serde_json::Map::new();
    for (name, value) in headers {
        let name_str = name.as_str().to_lowercase();
        // Skip sensitive headers - expanded list for better security
        let sensitive_headers = [
            "authorization", "cookie", "x-api-key", "x-auth-token",
            "x-forwarded-for", "x-real-ip", "x-original-forwarded-for",
            "set-cookie", "www-authenticate", "proxy-authorization",
            "x-csrf-token", "x-xsrf-token"
        ];

        if !sensitive_headers.contains(&name_str.as_str()) {
            if let Ok(value_str) = value.to_str() {
                // Limit header value size to prevent log bloat
                let truncated_value = if value_str.len() > 200 {
                    format!("{}... [truncated]", &value_str[..200])
                } else {
                    value_str.to_string()
                };
                header_map.insert(name.as_str().to_string(), Value::String(truncated_value));
            }
        }
    }
    info.insert("headers".to_string(), Value::Object(header_map));

    Value::Object(info)
}

/// Extract request body safely with size limits
async fn extract_request_body(request: &mut Request) -> Option<String> {
    const MAX_BODY_SIZE: usize = 4096; // 4KB limit for logging

    // Only capture body for methods that typically have bodies
    if !matches!(request.method(), &Method::POST | &Method::PUT | &Method::PATCH) {
        return None;
    }

    // Check content-type - only log JSON, form data, etc.
    let content_type = request.headers()
        .get("content-type")
        .and_then(|ct| ct.to_str().ok())
        .unwrap_or("");

    if !content_type.starts_with("application/json")
        && !content_type.starts_with("application/x-www-form-urlencoded")
        && !content_type.starts_with("multipart/form-data") {
        return None;
    }

    // Extract body
    let body = std::mem::replace(request.body_mut(), Body::empty());

    match to_bytes(body, MAX_BODY_SIZE).await {
        Ok(bytes) => {
            // Put the body back for the actual request
            *request.body_mut() = Body::from(bytes.clone());

            // Convert to string if it's valid UTF-8
            match String::from_utf8(bytes.to_vec()) {
                Ok(body_str) => {
                    if body_str.len() > MAX_BODY_SIZE {
                        Some(format!("{}... [truncated at {} bytes]", &body_str[..MAX_BODY_SIZE-50], MAX_BODY_SIZE))
                    } else {
                        Some(body_str)
                    }
                }
                Err(_) => Some(format!("<binary data {} bytes>", bytes.len()))
            }
        }
        Err(_) => {
            // If we fail to read the body, put an empty body back
            *request.body_mut() = Body::empty();
            Some("<failed to read body>".to_string())
        }
    }
}

/// Admin API logging middleware that only logs admin routes
pub async fn admin_api_logging_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    // Only log admin API routes
    if !request.uri().path().starts_with("/api/admin/") {
        return next.run(request).await;
    }
    let start_time = Instant::now();
    let method = request.method().clone();
    let uri = request.uri().clone();
    let headers = request.headers().clone();

    // Single timestamp for all logs in this request
    let timestamp = chrono::Utc::now().to_rfc3339();

    // Extract request body for logging (with size limits)
    let request_body = extract_request_body(&mut request).await;

    // Extract user information with database lookup
    let user = extract_user_info(&headers, &state.db).await;

    // Get request information for logging (only for detailed audit log)
    let mut request_info = get_request_info(&method, &uri, &headers);

    // Add body to request_info if available
    if let Some(body) = &request_body {
        if let Value::Object(ref mut info) = request_info {
            info.insert("body".to_string(), Value::String(body.clone()));
        }
    }

    // Create structured JSON log for request (lightweight)
    let request_log = json!({
        "event": "admin_api_request_started",
        "method": method.to_string(),
        "uri": uri.to_string(),
        "user_id": user.id,
        "user_identifier": user.identifier,
        "user_name": user.name,
        "user_email": user.email,
        "auth_type": user.auth_type,
        "timestamp": timestamp
    });

    match serde_json::to_string(&request_log) {
        Ok(json_str) => {
            tracing::debug!(target: "admin_api", "{}", json_str);
        }
        Err(e) => {
            tracing::error!(target: "admin_api", "Failed to serialize request log: {}", e);
        }
    }

    // Process the request
    let response = next.run(request).await;

    let duration = start_time.elapsed();
    let status = response.status();

    // Create structured JSON log for response (lightweight)
    let response_log = json!({
        "event": "admin_api_request_completed",
        "method": method.to_string(),
        "uri": uri.to_string(),
        "status": status.as_u16(),
        "duration_ms": duration.as_millis(),
        "user_id": user.id,
        "user_identifier": user.identifier,
        "user_name": user.name,
        "user_email": user.email,
        "auth_type": user.auth_type,
        "timestamp": timestamp
    });

    match serde_json::to_string(&response_log) {
        Ok(json_str) => {
            tracing::debug!(target: "admin_api", "{}", json_str);
        }
        Err(e) => {
            tracing::error!(target: "admin_api", "Failed to serialize response log: {}", e);
        }
    }

    // Create structured JSON audit log (detailed with request_info)
    let audit_log = json!({
        "event": if status.is_success() { "admin_api_operation_success" } else { "admin_api_operation_failed" },
        "method": method.to_string(),
        "uri": uri.to_string(),
        "status": status.as_u16(),
        "duration_ms": duration.as_millis(),
        "user_id": user.id,
        "user_identifier": user.identifier,
        "user_name": user.name,
        "user_email": user.email,
        "auth_type": user.auth_type,
        "request_info": request_info,
        "timestamp": timestamp
    });

    match serde_json::to_string(&audit_log) {
        Ok(json_str) => {
            if status.is_success() {
                tracing::info!(target: "admin_api_audit", "{}", json_str);
            } else {
                tracing::warn!(target: "admin_api_audit", "{}", json_str);
            }
        }
        Err(e) => {
            tracing::error!(target: "admin_api_audit", "Failed to serialize audit log: {}", e);
        }
    }

    response
}