use serde::{Deserialize, Serialize};
use reqwest::Client;
use crate::workflow::models::{WorkflowEvent, RetryConfig};
use crate::workflow::errors::{Result, SwissPipeError};
use std::time::Duration;
use handlebars::{Handlebars, Helper, HelperResult, Output, RenderContext, RenderError};

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

#[derive(Debug)]
pub struct AnthropicCallConfig<'a> {
    pub model: &'a str,
    pub max_tokens: u32,
    pub temperature: f64,
    pub system_prompt: Option<&'a str>,
    pub user_prompt: &'a str,
    pub timeout_seconds: u64,
    pub retry_config: &'a RetryConfig,
}

pub struct AnthropicService {
    client: Client,
    handlebars: Handlebars<'static>,
}

impl Default for AnthropicService {
    fn default() -> Self {
        Self::new()
    }
}

impl AnthropicService {
    pub fn new() -> Self {
        let client = Client::new();
        let mut handlebars = Handlebars::new();

        // Register the json helper
        handlebars.register_helper("json", Box::new(json_helper));
        handlebars.set_strict_mode(true);

        Self { client, handlebars }
    }

    pub async fn call_anthropic(
        &self,
        config: &AnthropicCallConfig<'_>,
        event: &WorkflowEvent,
    ) -> Result<WorkflowEvent> {
        // Get API key from environment
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| SwissPipeError::Generic(
                "ANTHROPIC_API_KEY environment variable not set".to_string()
            ))?;
        // Replace template variables in prompts
        let rendered_user_prompt = self.render_template(config.user_prompt, event)?;
        let rendered_system_prompt = config.system_prompt
            .map(|prompt| self.render_template(prompt, event))
            .transpose()?;

        let request = AnthropicRequest {
            model: config.model.to_string(),
            max_tokens: config.max_tokens,
            temperature: config.temperature,
            system: rendered_system_prompt,
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: rendered_user_prompt,
            }],
        };

        let mut attempts = 0;
        let mut delay = Duration::from_millis(config.retry_config.initial_delay_ms);

        loop {
            attempts += 1;

            let response = self
                .client
                .post("https://api.anthropic.com/v1/messages")
                .header("x-api-key", &api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .timeout(Duration::from_secs(config.timeout_seconds))
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
                            if attempts >= config.retry_config.max_attempts {
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

                    if attempts >= config.retry_config.max_attempts {
                        return Err(SwissPipeError::Generic(
                            format!("Anthropic API error {status} after {attempts} attempts: {error_text}")
                        ));
                    }
                }
                Err(e) => {
                    tracing::error!("Anthropic API request failed: {}", e);
                    if attempts >= config.retry_config.max_attempts {
                        return Err(SwissPipeError::Generic(
                            format!("Anthropic API request failed after {attempts} attempts: {e}")
                        ));
                    }
                }
            }

            if attempts < config.retry_config.max_attempts {
                tracing::warn!("Anthropic API call failed, retrying in {:?} (attempt {} of {})",
                    delay, attempts, config.retry_config.max_attempts);
                tokio::time::sleep(delay).await;

                let new_delay_ms = (delay.as_millis() as f64 * config.retry_config.backoff_multiplier) as u64;
                delay = Duration::from_millis(new_delay_ms.min(config.retry_config.max_delay_ms));
            }
        }
    }

    fn render_template(&self, template: &str, event: &WorkflowEvent) -> Result<String> {
        // Create template context with event data
        let mut context = serde_json::Map::new();

        // Add event data (consistent with email templates and other nodes)
        context.insert("event".to_string(), serde_json::json!({
            "data": event.data,
            "metadata": event.metadata,
            "headers": event.headers,
            "condition_results": event.condition_results,
            "hil_task": event.hil_task,
        }));

        // Flatten data properties to root level for easier access
        // This allows using {{name}} instead of {{event.data.name}}
        if let serde_json::Value::Object(ref data_obj) = event.data {
            for (key, value) in data_obj {
                // Only add if it doesn't conflict with existing root-level keys
                if !context.contains_key(key) {
                    context.insert(key.clone(), value.clone());
                }
            }
        }

        let context_value = serde_json::Value::Object(context);

        // Render template using handlebars
        self.handlebars
            .render_template(template, &context_value)
            .map_err(|e| SwissPipeError::Generic(format!("Template resolution failed: {e}")))
    }
}

// Handlebars helper function for JSON serialization
fn json_helper(
    h: &Helper,
    _: &Handlebars,
    _: &handlebars::Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let value = h.param(0)
        .ok_or_else(|| RenderError::new("json helper requires a parameter"))?;

    let json_str = serde_json::to_string_pretty(value.value())
        .map_err(|e| RenderError::new(format!("Failed to serialize to JSON: {e}")))?;

    out.write(&json_str)?;
    Ok(())
}