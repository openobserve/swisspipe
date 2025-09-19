use axum::{
    extract::{Request, State},
    http::{HeaderMap, Method, Uri},
    middleware::Next,
    response::Response,
};
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

/// Helper function to emit structured JSON logs with error handling
fn emit_admin_api_log(data: &serde_json::Value, level: tracing::Level) {
    match serde_json::to_string(data) {
        Ok(json_str) => {
            match level {
                tracing::Level::DEBUG => tracing::debug!(target: "admin_api", "{}", json_str),
                tracing::Level::INFO => tracing::info!(target: "admin_api", "{}", json_str),
                tracing::Level::WARN => tracing::warn!(target: "admin_api", "{}", json_str),
                _ => tracing::info!(target: "admin_api", "{}", json_str),
            }
        }
        Err(e) => {
            tracing::error!(target: "admin_api", "Failed to serialize log: {}", e);
        }
    }
}

/// Helper function to emit structured JSON audit logs with error handling
fn emit_admin_audit_log(data: &serde_json::Value, level: tracing::Level) {
    match serde_json::to_string(data) {
        Ok(json_str) => {
            match level {
                tracing::Level::DEBUG => tracing::debug!(target: "admin_api_audit", "{}", json_str),
                tracing::Level::INFO => tracing::info!(target: "admin_api_audit", "{}", json_str),
                tracing::Level::WARN => tracing::warn!(target: "admin_api_audit", "{}", json_str),
                _ => tracing::info!(target: "admin_api_audit", "{}", json_str),
            }
        }
        Err(e) => {
            tracing::error!(target: "admin_api_audit", "Failed to serialize audit log: {}", e);
        }
    }
}

/// Admin API logging middleware that only logs admin routes
pub async fn admin_api_logging_middleware(
    State(state): State<AppState>,
    request: Request,
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

    // Body extraction disabled to prevent request body consumption issue
    let request_body: Option<String> = None;

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

    emit_admin_api_log(&request_log, tracing::Level::DEBUG);

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

    emit_admin_api_log(&response_log, tracing::Level::DEBUG);

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

    let audit_level = if status.is_success() {
        tracing::Level::INFO
    } else {
        tracing::Level::WARN
    };
    emit_admin_audit_log(&audit_log, audit_level);

    response
}