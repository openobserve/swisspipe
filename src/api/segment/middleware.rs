use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use serde_json::Value;
use uuid::Uuid;

/// Extension to store extracted write key in request
#[derive(Clone, Debug)]
pub struct SegmentAuth {
    pub workflow_id: String,
}

/// Extract and validate Bearer token from Authorization header
pub async fn segment_auth_middleware(
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Get Authorization header
    let auth_header = request
        .headers()
        .get("authorization")
        .ok_or_else(|| {
            tracing::warn!("Missing Authorization header for Segment API");
            StatusCode::UNAUTHORIZED
        })?;

    // Parse Authorization header
    let auth_str = auth_header.to_str().map_err(|_| {
        tracing::warn!("Invalid Authorization header format");
        StatusCode::UNAUTHORIZED
    })?;

    // Extract Bearer token
    let bearer_token = auth_str.strip_prefix("Bearer ").ok_or_else(|| {
        tracing::warn!("Authorization header must use Bearer scheme");
        StatusCode::UNAUTHORIZED
    })?;

    // Validate UUID format and clone the token before using it
    let workflow_id = bearer_token.to_string();
    Uuid::parse_str(&workflow_id).map_err(|_| {
        tracing::warn!("Bearer token is not a valid UUID: {}", workflow_id);
        StatusCode::UNAUTHORIZED
    })?;

    // Store workflow ID in request extensions for handlers to use
    request.extensions_mut().insert(SegmentAuth {
        workflow_id: workflow_id.clone(),
    });

    tracing::debug!("Segment API Bearer token validated: {}", workflow_id);
    Ok(next.run(request).await)
}

/// Extract write key from Segment.com request (either from extensions or request body)
pub fn extract_write_key_from_request(
    request_extensions: &axum::http::Extensions,
    body_value: &Value,
) -> Result<String, SegmentAuthError> {
    // Try request extensions first (set by middleware)
    if let Some(auth) = request_extensions.get::<SegmentAuth>() {
        return Ok(auth.workflow_id.clone());
    }

    // Fall back to writeKey in request body
    if let Some(write_key) = body_value.get("writeKey") {
        if let Some(write_key_str) = write_key.as_str() {
            // Validate UUID format
            if Uuid::parse_str(write_key_str).is_ok() {
                return Ok(write_key_str.to_string());
            } else {
                return Err(SegmentAuthError::InvalidFormat {
                    source: "request_body".to_string(),
                    write_key: write_key_str.to_string(),
                });
            }
        }
    }

    Err(SegmentAuthError::Missing)
}

/// Detailed authentication errors for better debugging
#[derive(Debug)]
pub enum SegmentAuthError {
    Missing,
    InvalidFormat { source: String, write_key: String },
}

impl std::fmt::Display for SegmentAuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SegmentAuthError::Missing => {
                write!(f, "No write key found in Authorization header or request body")
            }
            SegmentAuthError::InvalidFormat { source, write_key } => {
                write!(f, "Invalid write key format from {source}: '{write_key}' is not a valid UUID")
            }
        }
    }
}

impl std::error::Error for SegmentAuthError {}