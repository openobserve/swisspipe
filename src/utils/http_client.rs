use crate::workflow::{
    errors::{AppError, SwissPipeError},
    models::{HttpMethod, RetryConfig, WorkflowEvent},
};
use reqwest::Client;
use std::time::Duration;


pub struct AppExecutor {
    client: Client,
}

impl Default for AppExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl AppExecutor {
    pub fn new() -> Self {
        // Create client with reasonable default timeout to prevent hanging
        let client = Client::builder()
            .timeout(Duration::from_secs(120)) // Default 120s timeout as fallback for external services
            .connect_timeout(Duration::from_secs(30)) // Connection timeout
            .build()
            .unwrap_or_else(|_| Client::new());
            
        Self {
            client,
        }
    }
    
    #[allow(clippy::too_many_arguments)]
    pub async fn execute_http_request(
        &self,
        url: &str,
        method: &HttpMethod,
        timeout_seconds: u64,
        retry_config: &RetryConfig,
        event: WorkflowEvent,
        node_headers: &std::collections::HashMap<String, String>,
    ) -> Result<WorkflowEvent, SwissPipeError> {
        tracing::info!("Starting HTTP request execution: url={}, method={:?}, timeout={}s, max_attempts={}", 
            url, method, timeout_seconds, retry_config.max_attempts);
        
        let mut attempts = 0;
        let mut delay = Duration::from_millis(retry_config.initial_delay_ms);
        
        loop {
            attempts += 1;
            tracing::info!("HTTP request execution attempt {} of {}", attempts, retry_config.max_attempts);
            
            let start_time = std::time::Instant::now();
            match self.execute_http_request_internal(url, method, timeout_seconds, &event, node_headers).await {
                Ok(response_event) => {
                    let elapsed = start_time.elapsed();
                    tracing::info!("HTTP request execution succeeded on attempt {} after {:?}", attempts, elapsed);
                    return Ok(response_event);
                },
                Err(e) if attempts >= retry_config.max_attempts => {
                    let elapsed = start_time.elapsed();
                    tracing::error!("HTTP request execution failed after {} attempts. Final attempt took {:?}. Error: {}", 
                        attempts, elapsed, e);
                    return Err(SwissPipeError::App(AppError::HttpRequestFailed {
                        attempts,
                        error: e.to_string(),
                    }));
                }
                Err(e) => {
                    let elapsed = start_time.elapsed();
                    tracing::warn!("HTTP request execution attempt {} failed after {:?}, retrying in {:?}. Error: {}", 
                        attempts, elapsed, delay, e);
                    
                    // Wait before retry
                    tokio::time::sleep(delay).await;
                    
                    // Exponential backoff
                    delay = Duration::from_millis(
                        ((delay.as_millis() as f64) * retry_config.backoff_multiplier) as u64
                    ).min(Duration::from_millis(retry_config.max_delay_ms));
                }
            }
        }
    }

    pub async fn execute_openobserve(
        &self,
        url: &str,
        authorization_header: &str,
        timeout_seconds: u64,
        retry_config: &RetryConfig,
        event: WorkflowEvent,
    ) -> Result<WorkflowEvent, SwissPipeError> {
        tracing::info!("Starting OpenObserve execution: url={}, timeout={}s, max_attempts={}", 
            url, timeout_seconds, retry_config.max_attempts);
        
        let mut attempts = 0;
        let mut delay = Duration::from_millis(retry_config.initial_delay_ms);
        
        loop {
            attempts += 1;
            tracing::info!("OpenObserve execution attempt {} of {}", attempts, retry_config.max_attempts);
            
            let start_time = std::time::Instant::now();
            match self.execute_openobserve_request(url, authorization_header, timeout_seconds, &event).await {
                Ok(response_event) => {
                    let elapsed = start_time.elapsed();
                    tracing::info!("OpenObserve execution succeeded on attempt {} after {:?}", attempts, elapsed);
                    return Ok(response_event);
                },
                Err(e) if attempts >= retry_config.max_attempts => {
                    let elapsed = start_time.elapsed();
                    tracing::error!("OpenObserve execution failed after {} attempts. Final attempt took {:?}. Error: {}", 
                        attempts, elapsed, e);
                    return Err(SwissPipeError::App(AppError::HttpRequestFailed {
                        attempts,
                        error: e.to_string(),
                    }));
                }
                Err(e) => {
                    let elapsed = start_time.elapsed();
                    tracing::warn!("OpenObserve execution attempt {} failed after {:?}, retrying in {:?}. Error: {}", 
                        attempts, elapsed, delay, e);
                    
                    // Wait before retry
                    tokio::time::sleep(delay).await;
                    
                    // Exponential backoff
                    delay = Duration::from_millis(
                        ((delay.as_millis() as f64) * retry_config.backoff_multiplier) as u64
                    ).min(Duration::from_millis(retry_config.max_delay_ms));
                }
            }
        }
    }
    
    async fn execute_http_request_internal(
        &self,
        url: &str,
        method: &HttpMethod,
        timeout_seconds: u64,
        event: &WorkflowEvent,
        node_headers: &std::collections::HashMap<String, String>,
    ) -> Result<WorkflowEvent, SwissPipeError> {
        let timeout = Duration::from_secs(timeout_seconds);
        tracing::info!("Executing HTTP request: url={}, timeout={:?}", url, timeout);
        
        {
                let mut request = match method {
                    HttpMethod::Post => self.client.post(url).json(&event.data),
                    HttpMethod::Put => self.client.put(url).json(&event.data),
                    HttpMethod::Delete => self.client.delete(url).json(&event.data),
                    HttpMethod::Patch => self.client.patch(url).json(&event.data),
                    HttpMethod::Get => {
                        // For GET, convert data to query parameters
                        let query_params = self.json_to_query_params(&event.data)?;
                        self.client.get(url).query(&query_params)
                    }
                };
                
                // Headers that should not be forwarded as they can cause issues
                let forbidden_headers = [
                    "host", "connection", "content-length", "transfer-encoding",
                    "accept-encoding", "expect", "upgrade", "proxy-authorization",
                    "te", "trailer"
                ];
                
                // Merge headers from both sources, with event headers taking precedence
                let mut combined_headers = node_headers.clone();
                for (key, value) in &event.headers {
                    let key_lower = key.to_lowercase();
                    // Filter out problematic headers
                    if !forbidden_headers.contains(&key_lower.as_str()) {
                        combined_headers.insert(key.clone(), value.clone());
                    } else {
                        tracing::debug!("Filtering out forbidden header: '{}': '{}'", key, value);
                    }
                }
                
                // Add all headers to the request, validating each one
                for (key, value) in &combined_headers {
                    let key_lower = key.to_lowercase();
                    // Double-check forbidden headers (in case they came from node_headers)
                    if forbidden_headers.contains(&key_lower.as_str()) {
                        tracing::debug!("Skipping forbidden header: '{}': '{}'", key, value);
                        continue;
                    }
                    
                    // Skip empty header values and invalid header names
                    if !value.is_empty() && !key.is_empty() {
                        match (reqwest::header::HeaderName::from_bytes(key.as_bytes()), reqwest::header::HeaderValue::from_str(value)) {
                            (Ok(name), Ok(val)) => {
                                request = request.header(name, val);
                            }
                            _ => {
                                tracing::warn!("Skipping invalid header: '{}': '{}'", key, value);
                            }
                        }
                    } else {
                        tracing::debug!("Skipping empty header key or value: '{}': '{}'", key, value);
                    }
                }
                
                tracing::info!("Sending HTTP request with {} headers: {:?}", combined_headers.len(), combined_headers);
                let request_start = std::time::Instant::now();
                
                let response = request
                    .timeout(timeout)
                    .send()
                    .await
                    .map_err(|e| {
                        let elapsed = request_start.elapsed();
                        tracing::error!("HTTP request failed after {:?}: {}", elapsed, e);
                        AppError::HttpRequestFailed { attempts: 1, error: e.to_string() }
                    })?;
                
                let request_elapsed = request_start.elapsed();
                tracing::info!("HTTP request completed in {:?}, status: {}", request_elapsed, response.status());
                
                if !response.status().is_success() {
                    return Err(SwissPipeError::App(AppError::InvalidStatus {
                        status: response.status().as_u16(),
                    }));
                }
                
                // Try to parse response as JSON, fallback to original event
                let response_data = response.json::<serde_json::Value>().await
                    .unwrap_or(event.data.clone());
                
                Ok(WorkflowEvent {
                    data: response_data,
                    metadata: event.metadata.clone(),
                    headers: event.headers.clone(),
                    condition_results: event.condition_results.clone(),
                })
            }
    }
    
    async fn execute_openobserve_request(
        &self,
        url: &str,
        authorization_header: &str,
        timeout_seconds: u64,
        event: &WorkflowEvent,
    ) -> Result<WorkflowEvent, SwissPipeError> {
        let timeout = Duration::from_secs(timeout_seconds);
        tracing::info!("Executing OpenObserve request: url={}, timeout={:?}", url, timeout);
        
        // OpenObserve expects JSON array format
        let payload = match &event.data {
            serde_json::Value::Array(arr) => arr.clone(),
            single_value => vec![single_value.clone()],
        };
        
        tracing::info!("Sending OpenObserve request to: {}", url);
        let request_start = std::time::Instant::now();
        
        let response = self.client
            .post(url)
            .header("Authorization", authorization_header)
            .header("Content-Type", "application/json")
            .json(&payload)
            .timeout(timeout)
            .send()
            .await
            .map_err(|e| {
                let elapsed = request_start.elapsed();
                tracing::error!("OpenObserve request failed after {:?}: {}", elapsed, e);
                AppError::HttpRequestFailed { attempts: 1, error: e.to_string() }
            })?;
        
        let request_elapsed = request_start.elapsed();
        tracing::info!("OpenObserve request completed in {:?}, status: {}", request_elapsed, response.status());
        
        match response.status().as_u16() {
            200..=299 => {
                // OpenObserve success - return original event for further processing
                Ok(event.clone())
            }
            401 => Err(SwissPipeError::App(AppError::AuthenticationFailed)),
            status => Err(SwissPipeError::App(AppError::InvalidStatus { status })),
        }
    }
    
    fn json_to_query_params(&self, value: &serde_json::Value) -> Result<Vec<(String, String)>, SwissPipeError> {
        let mut params = Vec::new();
        
        if let serde_json::Value::Object(map) = value {
            for (key, val) in map {
                let string_val = match val {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                params.push((key.clone(), string_val));
            }
        }
        
        Ok(params)
    }
}