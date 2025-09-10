use crate::workflow::errors::{Result, SwissPipeError};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;
use uuid::Uuid;

/// Maximum size for input data (1MB)
const MAX_INPUT_DATA_SIZE: usize = 1_048_576;

/// Maximum size for individual header values (4KB) 
const MAX_HEADER_VALUE_SIZE: usize = 4_096;

/// Maximum number of headers
const MAX_HEADER_COUNT: usize = 100;

/// Default dangerous headers (used if SP_DANGEROUS_HEADERS is not set)
const DEFAULT_DANGEROUS_HEADERS: &[&str] = &[
    "authorization", "cookie", "x-forwarded-for", "x-real-ip",
    "x-forwarded-proto", "host", "origin", "referer",
    "x-csrf-token", "x-api-key", "x-auth-token", "bearer",
    "www-authenticate", "proxy-authorization", "proxy-authenticate"
];

/// Static cache for dangerous headers from environment variable
static DANGEROUS_HEADERS: OnceLock<HashSet<String>> = OnceLock::new();

/// Get dangerous headers from environment variable or use defaults
fn get_dangerous_headers() -> &'static HashSet<String> {
    DANGEROUS_HEADERS.get_or_init(|| {
        match std::env::var("SP_DANGEROUS_HEADERS") {
            Ok(env_headers) if !env_headers.trim().is_empty() => {
                let headers: HashSet<String> = env_headers
                    .split(',')
                    .map(|h| h.trim().to_lowercase())
                    .filter(|h| !h.is_empty())
                    .collect();
                
                tracing::info!(
                    "Using dangerous headers from SP_DANGEROUS_HEADERS: {:?}", 
                    headers.iter().collect::<Vec<_>>()
                );
                
                headers
            }
            _ => {
                let default_headers: HashSet<String> = DEFAULT_DANGEROUS_HEADERS
                    .iter()
                    .map(|&s| s.to_string())
                    .collect();
                
                tracing::info!(
                    "Using default dangerous headers: {:?}", 
                    default_headers.iter().collect::<Vec<_>>()
                );
                
                default_headers
            }
        }
    })
}

/// Validate workflow ID format (must be a valid UUID)
pub fn validate_workflow_id(workflow_id: &str) -> Result<()> {
    Uuid::parse_str(workflow_id)
        .map_err(|_| SwissPipeError::InvalidInput("Invalid workflow ID format - must be a valid UUID".to_string()))?;
    Ok(())
}

/// Validate execution data (complete structure) and return serialized string for reuse
/// This validates the final execution data structure to catch oversized payloads
pub fn validate_and_serialize_execution_data(execution_data: &Value) -> Result<String> {
    // Serialize the complete execution data once
    let serialized = serde_json::to_string(execution_data)
        .map_err(|e| SwissPipeError::InvalidInput(format!("Invalid execution data: {e}")))?;
    
    // Check size of complete execution data (input + headers + metadata)
    if serialized.len() > MAX_INPUT_DATA_SIZE * 2 { // Allow more space for headers/metadata
        return Err(SwissPipeError::InvalidInput(
            format!("Execution data too large: {} bytes (max: {} bytes)", 
                serialized.len(), MAX_INPUT_DATA_SIZE * 2)
        ));
    }

    // Validate depth of the complete structure
    validate_json_depth(execution_data, 0, 10)?;
    
    Ok(serialized)
}

/// Validate and sanitize headers for security and size limits
/// Returns cleaned headers with dangerous headers removed
/// Optimized to avoid memory allocation when no filtering is needed
pub fn validate_and_sanitize_headers(headers: &HashMap<String, String>) -> Result<HashMap<String, String>> {
    if headers.len() > MAX_HEADER_COUNT {
        return Err(SwissPipeError::InvalidInput(
            format!("Too many headers: {} (max: {})", headers.len(), MAX_HEADER_COUNT)
        ));
    }

    // Fast path: pre-scan to check if any headers need to be filtered
    // This avoids memory allocation in the common case where all headers are valid
    let mut needs_filtering = false;
    let mut stripped_headers = Vec::new();

    for (key, value) in headers {
        // Check for conditions that would cause filtering
        if key.is_empty() 
            || key.len() > 256
            || is_dangerous_header(&key.to_lowercase())
            || value.len() > MAX_HEADER_VALUE_SIZE
            || value.chars().any(|c| c.is_control() && c != '\t')
        {
            needs_filtering = true;
            // If it's a dangerous header, collect it for logging
            if !key.is_empty() && key.len() <= 256 && is_dangerous_header(&key.to_lowercase()) {
                stripped_headers.push(key.clone());
            }
        }
    }

    // Fast path: if no filtering needed, return a clone of the original map
    if !needs_filtering {
        return Ok(headers.clone());
    }

    // Slow path: filtering is needed, create new map
    let mut sanitized_headers = HashMap::new();
    stripped_headers.clear(); // Clear and rebuild for accurate logging

    for (key, value) in headers {
        // Validate header key
        if key.is_empty() {
            continue; // Skip empty keys
        }

        if key.len() > 256 {
            tracing::warn!("Header key too long, skipping: '{}' ({} chars)", key, key.len());
            continue; // Skip overly long keys
        }

        // Check for dangerous header keys and skip them
        let key_lower = key.to_lowercase();
        if is_dangerous_header(&key_lower) {
            stripped_headers.push(key.clone());
            continue; // Skip dangerous headers
        }

        // Validate header value
        if value.len() > MAX_HEADER_VALUE_SIZE {
            tracing::warn!("Header value too long, skipping: '{}' ({} chars)", key, value.len());
            continue; // Skip overly long values
        }

        // Check for control characters in header value
        if value.chars().any(|c| c.is_control() && c != '\t') {
            tracing::warn!("Header value contains invalid control characters, skipping: '{}'", key);
            continue; // Skip headers with control characters
        }

        // Header is safe, add it to sanitized headers
        sanitized_headers.insert(key.clone(), value.clone());
    }

    // Log stripped dangerous headers for monitoring
    if !stripped_headers.is_empty() {
        tracing::info!("Stripped dangerous headers from request: {:?}", stripped_headers);
    }

    Ok(sanitized_headers)
}


/// Validate priority range
pub fn validate_priority(priority: Option<i32>) -> Result<()> {
    if let Some(p) = priority {
        if !(0..=10).contains(&p) {
            return Err(SwissPipeError::InvalidInput(
                format!("Priority must be between 0 and 10, got: {p}")
            ));
        }
    }
    Ok(())
}

/// Check if header key is dangerous using configurable list
fn is_dangerous_header(key: &str) -> bool {
    get_dangerous_headers().contains(&key.to_lowercase())
}

/// Recursively validate JSON depth to prevent stack overflow
fn validate_json_depth(value: &Value, current_depth: usize, max_depth: usize) -> Result<()> {
    if current_depth > max_depth {
        return Err(SwissPipeError::InvalidInput(
            format!("JSON nesting too deep: {current_depth} levels (max: {max_depth})")
        ));
    }

    match value {
        Value::Object(map) => {
            for v in map.values() {
                validate_json_depth(v, current_depth + 1, max_depth)?;
            }
        }
        Value::Array(arr) => {
            for v in arr {
                validate_json_depth(v, current_depth + 1, max_depth)?;
            }
        }
        _ => {} // Primitives are fine
    }

    Ok(())
}

/// Validate execution ID format
pub fn validate_execution_id(execution_id: &str) -> Result<()> {
    if execution_id.is_empty() {
        return Err(SwissPipeError::InvalidInput("Execution ID cannot be empty".to_string()));
    }

    // Should be a valid UUID
    Uuid::parse_str(execution_id)
        .map_err(|_| SwissPipeError::InvalidInput("Invalid execution ID format - must be a valid UUID".to_string()))?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_workflow_id() {
        // Valid UUID
        assert!(validate_workflow_id("550e8400-e29b-41d4-a716-446655440000").is_ok());
        
        // Invalid UUID
        assert!(validate_workflow_id("invalid-uuid").is_err());
        assert!(validate_workflow_id("").is_err());
    }

    #[test]
    fn test_sanitize_headers() {
        // Mix of valid and dangerous headers
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());
        headers.insert("user-agent".to_string(), "test-client".to_string());
        headers.insert("authorization".to_string(), "Bearer token".to_string()); // Should be stripped
        headers.insert("host".to_string(), "example.com".to_string()); // Should be stripped
        
        let sanitized = validate_and_sanitize_headers(&headers).unwrap();
        
        // Valid headers should remain
        assert!(sanitized.contains_key("content-type"));
        assert!(sanitized.contains_key("user-agent"));
        
        // Dangerous headers should be removed
        assert!(!sanitized.contains_key("authorization"));
        assert!(!sanitized.contains_key("host"));
        
        assert_eq!(sanitized.len(), 2);
    }

    #[test]
    fn test_dangerous_headers_from_env() {
        // Test that we can configure dangerous headers via environment
        // Note: This test may be affected by other tests or the actual environment
        // In a real test, you'd want to use a test-specific approach
        
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());
        headers.insert("authorization".to_string(), "Bearer token".to_string());
        
        let sanitized = validate_and_sanitize_headers(&headers).unwrap();
        
        // Authorization should be stripped by default
        assert!(sanitized.contains_key("content-type"));
        assert!(!sanitized.contains_key("authorization"));
    }

    #[test]
    fn test_validate_priority() {
        assert!(validate_priority(Some(5)).is_ok());
        assert!(validate_priority(Some(0)).is_ok());
        assert!(validate_priority(Some(10)).is_ok());
        assert!(validate_priority(None).is_ok());
        
        assert!(validate_priority(Some(-1)).is_err());
        assert!(validate_priority(Some(11)).is_err());
    }

    #[test]
    fn test_header_processing_optimization() {
        // Test fast path: headers with no issues should be returned as-is (clone)
        let mut clean_headers = HashMap::new();
        clean_headers.insert("content-type".to_string(), "application/json".to_string());
        clean_headers.insert("user-agent".to_string(), "test-client".to_string());
        clean_headers.insert("x-custom-header".to_string(), "valid-value".to_string());
        
        let result = validate_and_sanitize_headers(&clean_headers).unwrap();
        
        // All headers should be preserved
        assert_eq!(result.len(), 3);
        assert!(result.contains_key("content-type"));
        assert!(result.contains_key("user-agent"));
        assert!(result.contains_key("x-custom-header"));
        
        // Test slow path: headers with dangerous content should be filtered
        let mut mixed_headers = HashMap::new();
        mixed_headers.insert("content-type".to_string(), "application/json".to_string());
        mixed_headers.insert("authorization".to_string(), "Bearer token".to_string()); // Dangerous
        mixed_headers.insert("user-agent".to_string(), "test-client".to_string());
        
        let filtered_result = validate_and_sanitize_headers(&mixed_headers).unwrap();
        
        // Only safe headers should remain
        assert_eq!(filtered_result.len(), 2);
        assert!(filtered_result.contains_key("content-type"));
        assert!(filtered_result.contains_key("user-agent"));
        assert!(!filtered_result.contains_key("authorization"));
        
        // Test with empty/invalid headers
        let mut invalid_headers = HashMap::new();
        invalid_headers.insert("".to_string(), "empty-key".to_string()); // Empty key
        invalid_headers.insert("valid-key".to_string(), "valid-value".to_string());
        invalid_headers.insert("long-key".repeat(100), "value".to_string()); // Too long key
        
        let clean_result = validate_and_sanitize_headers(&invalid_headers).unwrap();
        
        // Only valid header should remain
        assert_eq!(clean_result.len(), 1);
        assert!(clean_result.contains_key("valid-key"));
    }
}