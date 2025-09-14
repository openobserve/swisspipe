use serde::{Deserialize, Serialize};
use reqwest::Client;
use crate::workflow::models::{WorkflowEvent, RetryConfig};
use crate::workflow::errors::{Result, SwissPipeError};
use std::time::Duration;

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    temperature: f64,
    system: Option<String>,
    messages: Vec<AnthropicMessage>,
}

#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
    usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
struct AnthropicContent {
    text: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

pub struct AnthropicService {
    client: Client,
}

impl Default for AnthropicService {
    fn default() -> Self {
        Self::new()
    }
}

impl AnthropicService {
    pub fn new() -> Self {
        let client = Client::new();
        Self { client }
    }

    pub async fn call_anthropic(
        &self,
        model: &str,
        max_tokens: u32,
        temperature: f64,
        system_prompt: Option<&str>,
        user_prompt: &str,
        event: &WorkflowEvent,
        timeout_seconds: u64,
        retry_config: &RetryConfig,
    ) -> Result<WorkflowEvent> {
        // Get API key from environment
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| SwissPipeError::Generic(
                "ANTHROPIC_API_KEY environment variable not set".to_string()
            ))?;
        // Replace template variables in prompts
        let rendered_user_prompt = self.render_template(user_prompt, event)?;
        let rendered_system_prompt = system_prompt
            .map(|prompt| self.render_template(prompt, event))
            .transpose()?;

        let request = AnthropicRequest {
            model: model.to_string(),
            max_tokens,
            temperature,
            system: rendered_system_prompt,
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: rendered_user_prompt,
            }],
        };

        let mut attempts = 0;
        let mut delay = Duration::from_millis(retry_config.initial_delay_ms);

        loop {
            attempts += 1;

            let response = self
                .client
                .post("https://api.anthropic.com/v1/messages")
                .header("x-api-key", &api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .timeout(Duration::from_secs(timeout_seconds))
                .json(&request)
                .send()
                .await;

            match response {
                Ok(resp) if resp.status().is_success() => {
                    match resp.json::<AnthropicResponse>().await {
                        Ok(anthropic_response) => {
                            tracing::info!("Anthropic API call successful. Input tokens: {}, Output tokens: {}",
                                anthropic_response.usage.input_tokens, anthropic_response.usage.output_tokens);

                            let assistant_response = anthropic_response.content
                                .first()
                                .map(|c| c.text.as_str())
                                .unwrap_or("")
                                .to_string();

                            let mut result_event = event.clone();
                            result_event.data = serde_json::json!({
                                "original_data": event.data,
                                "anthropic_response": assistant_response,
                                "usage": {
                                    "input_tokens": anthropic_response.usage.input_tokens,
                                    "output_tokens": anthropic_response.usage.output_tokens
                                }
                            });

                            return Ok(result_event);
                        }
                        Err(e) => {
                            tracing::error!("Failed to parse Anthropic response: {}", e);
                            if attempts >= retry_config.max_attempts {
                                return Err(SwissPipeError::Generic(
                                    format!("Anthropic API parse error after {attempts} attempts: {e}")
                                ));
                            }
                        }
                    }
                }
                Ok(resp) => {
                    let status = resp.status();
                    let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                    tracing::error!("Anthropic API error {}: {}", status, error_text);

                    if attempts >= retry_config.max_attempts {
                        return Err(SwissPipeError::Generic(
                            format!("Anthropic API error {status} after {attempts} attempts: {error_text}")
                        ));
                    }
                }
                Err(e) => {
                    tracing::error!("Anthropic API request failed: {}", e);
                    if attempts >= retry_config.max_attempts {
                        return Err(SwissPipeError::Generic(
                            format!("Anthropic API request failed after {attempts} attempts: {e}")
                        ));
                    }
                }
            }

            if attempts < retry_config.max_attempts {
                tracing::warn!("Anthropic API call failed, retrying in {:?} (attempt {} of {})",
                    delay, attempts, retry_config.max_attempts);
                tokio::time::sleep(delay).await;

                let new_delay_ms = (delay.as_millis() as f64 * retry_config.backoff_multiplier) as u64;
                delay = Duration::from_millis(new_delay_ms.min(retry_config.max_delay_ms));
            }
        }
    }

    fn render_template(&self, template: &str, event: &WorkflowEvent) -> Result<String> {
        // Simple template rendering - replace {{key}} with values from event data
        let mut result = template.to_string();

        // Replace event.data variables
        if let Some(obj) = event.data.as_object() {
            for (key, value) in obj {
                let placeholder = format!("{{{{ event.data.{key} }}}}");
                let replacement = match value {
                    serde_json::Value::String(s) => s.clone(),
                    _ => value.to_string(),
                };
                result = result.replace(&placeholder, &replacement);
            }
        }

        // Replace full event.data with JSON
        let data_json = serde_json::to_string(&event.data)
            .map_err(|e| SwissPipeError::Generic(format!("Failed to serialize event data: {e}")))?;
        result = result.replace("{{ event.data }}", &data_json);

        Ok(result)
    }
}