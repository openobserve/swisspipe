use crate::{
    anthropic::{AnthropicService, AnthropicCallConfig},
    email::{service::EmailService, EmailConfig},
    utils::{http_client::AppExecutor, javascript::JavaScriptExecutor},
    workflow::{
        errors::{Result, SwissPipeError},
        models::{Node, NodeType, WorkflowEvent, FailureAction, RetryConfig},
    },
};
use std::sync::Arc;

pub struct NodeExecutor {
    js_executor: Arc<JavaScriptExecutor>,
    app_executor: Arc<AppExecutor>,
    email_service: Arc<EmailService>,
    anthropic_service: Arc<AnthropicService>,
}

impl NodeExecutor {
    pub fn new(
        js_executor: Arc<JavaScriptExecutor>,
        app_executor: Arc<AppExecutor>,
        email_service: Arc<EmailService>,
        anthropic_service: Arc<AnthropicService>,
    ) -> Self {
        Self {
            js_executor,
            app_executor,
            email_service,
            anthropic_service,
        }
    }

    /// Execute a single node based on its type
    pub async fn execute_node(
        &self,
        node: &Node,
        event: WorkflowEvent,
        execution_id: &str,
    ) -> Result<WorkflowEvent> {
        self.execute_node_by_type(
            &node.node_type,
            event,
            execution_id,
            &node.name,
            &node.workflow_id,
            &node.id,
        ).await
    }

    /// Execute node based on its specific type
    async fn execute_node_by_type(
        &self,
        node_type: &NodeType,
        event: WorkflowEvent,
        _execution_id: &str,
        node_name: &str,
        workflow_id: &str,
        node_id: &str,
    ) -> Result<WorkflowEvent> {
        match node_type {
            NodeType::Trigger { .. } => Ok(event),
            NodeType::Condition { script } => {
                self.execute_condition_node(script, event, node_name, node_id).await
            }
            NodeType::Transformer { script } => {
                self.execute_transformer_node(script, event, node_name).await
            }
            NodeType::HttpRequest { url, method, timeout_seconds, failure_action, retry_config, headers } => {
                let config = HttpRequestConfig {
                    url,
                    method,
                    timeout_seconds: *timeout_seconds,
                    failure_action,
                    retry_config,
                    headers,
                    node_name,
                };
                self.execute_http_request_node(&config, event).await
            }
            NodeType::OpenObserve { url, authorization_header, timeout_seconds, failure_action, retry_config } => {
                let config = OpenObserveConfig {
                    url,
                    authorization_header,
                    timeout_seconds: *timeout_seconds,
                    failure_action,
                    retry_config,
                    node_name,
                };
                self.execute_openobserve_node(&config, event).await
            }
            NodeType::Email { config } => {
                self.execute_email_node(config, event, workflow_id, node_id, node_name).await
            }
            NodeType::Delay { duration, unit } => {
                self.execute_delay_node(*duration, unit, node_name, event).await
            }
            NodeType::Anthropic { model, max_tokens, temperature, system_prompt, user_prompt, timeout_seconds, failure_action, retry_config } => {
                let config = AnthropicNodeConfig {
                    model,
                    max_tokens: *max_tokens,
                    temperature: *temperature,
                    system_prompt: system_prompt.as_deref(),
                    user_prompt,
                    timeout_seconds: *timeout_seconds,
                    failure_action,
                    retry_config,
                    node_name,
                };
                self.execute_anthropic_node(&config, event).await
            }
        }
    }

    /// Execute condition node
    async fn execute_condition_node(
        &self,
        script: &str,
        mut event: WorkflowEvent,
        node_name: &str,
        node_id: &str,
    ) -> Result<WorkflowEvent> {
        let condition_result = self.js_executor.execute_condition(script, &event).await?;
        tracing::info!("Condition node '{}' evaluated to: {}", node_name, condition_result);

        // Store condition result in event for edge routing using node ID as key
        event.condition_results.insert(node_id.to_string(), condition_result);
        Ok(event)
    }

    /// Execute transformer node
    async fn execute_transformer_node(
        &self,
        script: &str,
        event: WorkflowEvent,
        node_name: &str,
    ) -> Result<WorkflowEvent> {
        let mut transformed_event = self.js_executor.execute_transformer(script, event.clone()).await
            .map_err(SwissPipeError::JavaScript)?;

        // Preserve condition results from the original event
        transformed_event.condition_results = event.condition_results;

        tracing::debug!("Transformer node '{}' completed transformation", node_name);
        Ok(transformed_event)
    }

    /// Execute HTTP request node
    async fn execute_http_request_node(
        &self,
        config: &HttpRequestConfig<'_>,
        event: WorkflowEvent,
    ) -> Result<WorkflowEvent> {
        match config.failure_action {
            FailureAction::Retry => {
                self.app_executor.execute_http_request(
                    config.url,
                    config.method,
                    config.timeout_seconds,
                    config.retry_config,
                    event,
                    config.headers,
                ).await
            }
            FailureAction::Continue => {
                let single_attempt_config = RetryConfig {
                    max_attempts: 1,
                    ..config.retry_config.clone()
                };
                match self.app_executor.execute_http_request(
                    config.url,
                    config.method,
                    config.timeout_seconds,
                    &single_attempt_config,
                    event.clone(),
                    config.headers,
                ).await {
                    Ok(result) => Ok(result),
                    Err(e) => {
                        tracing::warn!("HTTP request node '{}' failed but continuing: {}", config.node_name, e);
                        Ok(event)
                    }
                }
            }
            FailureAction::Stop => {
                let single_attempt_config = RetryConfig {
                    max_attempts: 1,
                    ..config.retry_config.clone()
                };
                self.app_executor.execute_http_request(
                    config.url,
                    config.method,
                    config.timeout_seconds,
                    &single_attempt_config,
                    event,
                    config.headers,
                ).await
            }
        }
    }

    /// Execute OpenObserve node
    async fn execute_openobserve_node(
        &self,
        config: &OpenObserveConfig<'_>,
        event: WorkflowEvent,
    ) -> Result<WorkflowEvent> {
        match config.failure_action {
            FailureAction::Retry => {
                self.app_executor.execute_openobserve(
                    config.url,
                    config.authorization_header,
                    config.timeout_seconds,
                    config.retry_config,
                    event,
                ).await
            }
            FailureAction::Continue => {
                let single_attempt_config = RetryConfig {
                    max_attempts: 1,
                    ..config.retry_config.clone()
                };
                match self.app_executor.execute_openobserve(
                    config.url,
                    config.authorization_header,
                    config.timeout_seconds,
                    &single_attempt_config,
                    event.clone(),
                ).await {
                    Ok(result) => Ok(result),
                    Err(e) => {
                        tracing::warn!("OpenObserve node '{}' failed but continuing: {}", config.node_name, e);
                        Ok(event)
                    }
                }
            }
            FailureAction::Stop => {
                let single_attempt_config = RetryConfig {
                    max_attempts: 1,
                    ..config.retry_config.clone()
                };
                self.app_executor.execute_openobserve(
                    config.url,
                    config.authorization_header,
                    config.timeout_seconds,
                    &single_attempt_config,
                    event,
                ).await
            }
        }
    }

    /// Execute email node
    async fn execute_email_node(
        &self,
        config: &EmailConfig,
        event: WorkflowEvent,
        workflow_id: &str,
        node_id: &str,
        node_name: &str,
    ) -> Result<WorkflowEvent> {
        tracing::debug!("Executing email node '{}' with config: {:?}", node_name, config);
        match self.email_service.send_email(config, &event, workflow_id, node_id).await {
            Ok(result) => {
                tracing::info!("Email node '{}' executed successfully: {:?}", node_name, result);
                Ok(event)
            }
            Err(e) => {
                tracing::error!("Email node '{}' failed: {}", node_name, e);
                Err(SwissPipeError::Generic(format!("Email node failed: {e}")))
            }
        }
    }

    /// Execute delay node
    async fn execute_delay_node(
        &self,
        duration: u64,
        unit: &crate::workflow::models::DelayUnit,
        node_name: &str,
        event: WorkflowEvent,
    ) -> Result<WorkflowEvent> {
        use crate::workflow::models::DelayUnit;
        use tokio::time::{sleep, Duration};

        let delay_ms = match unit {
            DelayUnit::Seconds => duration.saturating_mul(1000),
            DelayUnit::Minutes => duration.saturating_mul(60_000),
            DelayUnit::Hours => duration.saturating_mul(3_600_000),
            DelayUnit::Days => duration.saturating_mul(86_400_000),
        };

        // Cap at 1 hour for safety in direct execution
        let capped_delay_ms = delay_ms.min(3_600_000);
        if delay_ms > capped_delay_ms {
            tracing::warn!(
                "Delay node '{}' requested {}ms but capped to {}ms (1 hour)",
                node_name, delay_ms, capped_delay_ms
            );
        }

        tracing::info!("Delay node '{}' sleeping for {}ms", node_name, capped_delay_ms);
        sleep(Duration::from_millis(capped_delay_ms)).await;
        tracing::debug!("Delay node '{}' completed", node_name);

        Ok(event)
    }

    /// Execute Anthropic node
    async fn execute_anthropic_node(
        &self,
        config: &AnthropicNodeConfig<'_>,
        event: WorkflowEvent,
    ) -> Result<WorkflowEvent> {
        let anthropic_config = AnthropicCallConfig {
            model: config.model,
            max_tokens: config.max_tokens,
            temperature: config.temperature,
            system_prompt: config.system_prompt,
            user_prompt: config.user_prompt,
            timeout_seconds: config.timeout_seconds,
            retry_config: config.retry_config,
        };

        match config.failure_action {
            FailureAction::Retry => {
                self.anthropic_service.call_anthropic(&anthropic_config, &event).await
            }
            FailureAction::Continue => {
                let single_attempt_config = AnthropicCallConfig {
                    retry_config: &RetryConfig {
                        max_attempts: 1,
                        ..config.retry_config.clone()
                    },
                    ..anthropic_config
                };
                match self.anthropic_service.call_anthropic(&single_attempt_config, &event).await {
                    Ok(result) => Ok(result),
                    Err(e) => {
                        tracing::warn!("Anthropic node '{}' failed but continuing: {}", config.node_name, e);
                        Ok(event)
                    }
                }
            }
            FailureAction::Stop => {
                let single_attempt_config = AnthropicCallConfig {
                    retry_config: &RetryConfig {
                        max_attempts: 1,
                        ..config.retry_config.clone()
                    },
                    ..anthropic_config
                };
                self.anthropic_service.call_anthropic(&single_attempt_config, &event).await
            }
        }
    }
}

// Configuration structs to group related parameters (following clippy recommendations)

struct HttpRequestConfig<'a> {
    url: &'a str,
    method: &'a crate::workflow::models::HttpMethod,
    timeout_seconds: u64,
    failure_action: &'a FailureAction,
    retry_config: &'a RetryConfig,
    headers: &'a std::collections::HashMap<String, String>,
    node_name: &'a str,
}

struct OpenObserveConfig<'a> {
    url: &'a str,
    authorization_header: &'a str,
    timeout_seconds: u64,
    failure_action: &'a FailureAction,
    retry_config: &'a RetryConfig,
    node_name: &'a str,
}

struct AnthropicNodeConfig<'a> {
    model: &'a str,
    max_tokens: u32,
    temperature: f64,
    system_prompt: Option<&'a str>,
    user_prompt: &'a str,
    timeout_seconds: u64,
    failure_action: &'a FailureAction,
    retry_config: &'a RetryConfig,
    node_name: &'a str,
}