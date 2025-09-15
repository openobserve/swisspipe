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

/// Extract write key from Segment.com request and validate it
pub async fn segment_auth_middleware(
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Get headers first
    let headers = request.headers().clone();

    // Try to extract from Authorization header first
    if let Some(auth_header) = headers.get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(bearer_token) = auth_str.strip_prefix("Bearer ") {
                // Bearer authentication - token is the workflow UUID directly
                if let Ok(_) = Uuid::parse_str(bearer_token) {
                    request.extensions_mut().insert(SegmentAuth {
                        workflow_id: bearer_token.to_string(),
                    });
                    return Ok(next.run(request).await);
                } else {
                    tracing::warn!("Invalid Bearer token format: {}", bearer_token);
                    return Err(StatusCode::UNAUTHORIZED);
                }
            }
        }
    }

    // If header auth failed, we'll need to check the body
    // For now, let the handlers deal with body-based auth
    // This middleware primarily handles header-based auth
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
    InvalidBase64 { auth_header: String },
    HeaderParseError { header_value: String },
}

impl std::fmt::Display for SegmentAuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SegmentAuthError::Missing => {
                write!(f, "No write key found in Authorization header or request body")
            }
            SegmentAuthError::InvalidFormat { source, write_key } => {
                write!(f, "Invalid write key format from {}: '{}' is not a valid UUID", source, write_key)
            }
            SegmentAuthError::InvalidBase64 { auth_header } => {
                write!(f, "Invalid Base64 encoding in Authorization header: '{}'", auth_header)
            }
            SegmentAuthError::HeaderParseError { header_value } => {
                write!(f, "Failed to parse Authorization header value: '{}'", header_value)
            }
        }
    }
}

impl std::error::Error for SegmentAuthError {}