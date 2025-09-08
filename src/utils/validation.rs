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

/// Validate input data size and structure
pub fn validate_input_data(input_data: &Value) -> Result<()> {
    // Check serialized size
    let serialized = serde_json::to_string(input_data)
        .map_err(|e| SwissPipeError::InvalidInput(format!("Invalid JSON input data: {e}")))?;
    
    if serialized.len() > MAX_INPUT_DATA_SIZE {
        return Err(SwissPipeError::InvalidInput(
            format!("Input data too large: {} bytes (max: {} bytes)", 
                serialized.len(), MAX_INPUT_DATA_SIZE)
        ));
    }

    // Validate depth to prevent deeply nested objects that could cause stack overflow
    validate_json_depth(input_data, 0, 10)?;
    
    Ok(())
}

/// Validate and sanitize headers for security and size limits
/// Returns cleaned headers with dangerous headers removed
pub fn validate_and_sanitize_headers(headers: &HashMap<String, String>) -> Result<HashMap<String, String>> {
    if headers.len() > MAX_HEADER_COUNT {
        return Err(SwissPipeError::InvalidInput(
            format!("Too many headers: {} (max: {})", headers.len(), MAX_HEADER_COUNT)
        ));
    }

    let mut sanitized_headers = HashMap::new();
    let mut stripped_headers = Vec::new();

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
    use serde_json::json;

    #[test]
    fn test_validate_workflow_id() {
        // Valid UUID
        assert!(validate_workflow_id("550e8400-e29b-41d4-a716-446655440000").is_ok());
        
        // Invalid UUID
        assert!(validate_workflow_id("invalid-uuid").is_err());
        assert!(validate_workflow_id("").is_err());
    }

    #[test]
    fn test_validate_input_data_size() {
        // Valid small data
        let small_data = json!({"key": "value"});
        assert!(validate_input_data(&small_data).is_ok());

        // Large data (would need actual large data to test size limit)
        // This is just a conceptual test
        let valid_data = json!({"data": "valid"});
        assert!(validate_input_data(&valid_data).is_ok());
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
}