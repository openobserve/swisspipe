use crate::workflow::{
    errors::{AppError, SwissPipeError},
    models::{AppType, HttpMethod, RetryConfig, WorkflowEvent},
};
use reqwest::Client;
use std::time::Duration;

pub struct AppExecutor {
    client: Client,
}

impl AppExecutor {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
    
    pub async fn execute_app(
        &self,
        app_type: &AppType,
        url: &str,
        method: &HttpMethod,
        timeout_seconds: u64,
        retry_config: &RetryConfig,
        event: WorkflowEvent,
        node_headers: &std::collections::HashMap<String, String>,
    ) -> Result<WorkflowEvent, SwissPipeError> {
        let mut attempts = 0;
        let mut delay = Duration::from_millis(retry_config.initial_delay_ms);
        
        loop {
            attempts += 1;
            
            match self.execute_app_request(app_type, url, method, timeout_seconds, &event, node_headers).await {
                Ok(response_event) => return Ok(response_event),
                Err(e) if attempts >= retry_config.max_attempts => {
                    return Err(SwissPipeError::App(AppError::HttpRequestFailed {
                        attempts,
                        error: e.to_string(),
                    }));
                }
                Err(_) => {
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
    
    async fn execute_app_request(
        &self,
        app_type: &AppType,
        url: &str,
        method: &HttpMethod,
        timeout_seconds: u64,
        event: &WorkflowEvent,
        node_headers: &std::collections::HashMap<String, String>,
    ) -> Result<WorkflowEvent, SwissPipeError> {
        let timeout = Duration::from_secs(timeout_seconds);
        
        match app_type {
            AppType::Webhook => {
                let mut request = match method {
                    HttpMethod::POST => self.client.post(url).json(&event.data),
                    HttpMethod::PUT => self.client.put(url).json(&event.data),
                    HttpMethod::GET => {
                        // For GET, convert data to query parameters
                        let query_params = self.json_to_query_params(&event.data)?;
                        self.client.get(url).query(&query_params)
                    }
                };
                
                // Add headers from node configuration (from frontend)
                for (key, value) in node_headers {
                    request = request.header(key, value);
                }
                
                // Add headers from workflow event (runtime headers)
                for (key, value) in &event.headers {
                    request = request.header(key, value);
                }
                
                let response = request
                    .timeout(timeout)
                    .send()
                    .await
                    .map_err(|e| AppError::HttpRequestFailed { attempts: 1, error: e.to_string() })?;
                
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
            
            AppType::OpenObserve { url: openobserve_url, authorization_header } => {
                // OpenObserve expects JSON array format
                let payload = match &event.data {
                    serde_json::Value::Array(arr) => arr.clone(),
                    single_value => vec![single_value.clone()],
                };
                
                let full_url = openobserve_url.clone();
                
                let response = self.client
                    .post(&full_url)
                    .header("Authorization", authorization_header)
                    .header("Content-Type", "application/json")
                    .json(&payload)
                    .timeout(timeout)
                    .send()
                    .await
                    .map_err(|e| AppError::HttpRequestFailed { attempts: 1, error: e.to_string() })?;
                
                match response.status().as_u16() {
                    200..=299 => {
                        // OpenObserve success - return original event for further processing
                        Ok(event.clone())
                    }
                    401 => Err(SwissPipeError::App(AppError::AuthenticationFailed)),
                    status => Err(SwissPipeError::App(AppError::InvalidStatus { status })),
                }
            }
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