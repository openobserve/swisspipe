use axum::{extract::State, http::StatusCode, response::Json};
use serde::{Deserialize, Serialize};
use crate::workflow::models::WorkflowEvent;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct GenerateCodeRequest {
    pub system_prompt: String,
    pub user_prompt: String,
    pub model: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct GenerateCodeResponse {
    pub response: String,
    pub usage: Option<serde_json::Value>,
}

pub async fn generate_code(
    State(state): State<AppState>,
    Json(request): Json<GenerateCodeRequest>,
) -> Result<Json<GenerateCodeResponse>, StatusCode> {
    // Create a dummy event for the Anthropic service
    let dummy_event = WorkflowEvent {
        data: serde_json::json!({}),
        metadata: std::collections::HashMap::new(),
        headers: std::collections::HashMap::new(),
        condition_results: std::collections::HashMap::new(),
    };

    // Default configuration for code generation
    let model = request.model.unwrap_or_else(|| "claude-3-5-sonnet-20241022".to_string());
    let max_tokens = request.max_tokens.unwrap_or(4000);
    let temperature = request.temperature.unwrap_or(0.1);

    // Use the retry config but with fewer attempts for interactive use
    let retry_config = crate::workflow::models::RetryConfig {
        max_attempts: 1,
        initial_delay_ms: 1000,
        max_delay_ms: 5000,
        backoff_multiplier: 2.0,
    };

    match state.engine.anthropic_service
        .call_anthropic(
            &model,
            max_tokens,
            temperature,
            Some(&request.system_prompt),
            &request.user_prompt,
            &dummy_event,
            120, // 120 second timeout for AI generation
            &retry_config,
        )
        .await
    {
        Ok(result) => {
            // Extract the response from the event data
            if let Some(anthropic_response) = result.data.get("anthropic_response") {
                let response = anthropic_response.as_str().unwrap_or("").to_string();
                let usage = result.data.get("usage").cloned();

                Ok(Json(GenerateCodeResponse {
                    response,
                    usage,
                }))
            } else {
                tracing::error!("No anthropic_response found in result data");
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
        Err(e) => {
            tracing::error!("AI code generation failed: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}