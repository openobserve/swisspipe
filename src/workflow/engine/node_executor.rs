use crate::{
    anthropic::{AnthropicService, AnthropicCallConfig},
    async_execution::HttpLoopScheduler,
    email::{service::EmailService, EmailConfig},
    utils::{http_client::AppExecutor, javascript::JavaScriptExecutor},
    workflow::{
        errors::{Result, SwissPipeError},
        models::{Node, NodeType, WorkflowEvent, FailureAction, RetryConfig},
    },
};
use std::sync::{Arc, OnceLock};
use sea_orm::DatabaseConnection;

pub struct NodeExecutor {
    js_executor: Arc<JavaScriptExecutor>,
    app_executor: Arc<AppExecutor>,
    email_service: Arc<EmailService>,
    anthropic_service: Arc<AnthropicService>,
    #[allow(dead_code)] // May be used in future for direct database operations
    db: Arc<DatabaseConnection>,
    http_loop_scheduler: Arc<OnceLock<Arc<HttpLoopScheduler>>>,
}

impl NodeExecutor {
    pub fn new(
        js_executor: Arc<JavaScriptExecutor>,
        app_executor: Arc<AppExecutor>,
        email_service: Arc<EmailService>,
        anthropic_service: Arc<AnthropicService>,
        db: Arc<DatabaseConnection>,
    ) -> Self {
        Self {
            js_executor,
            app_executor,
            email_service,
            anthropic_service,
            db,
            http_loop_scheduler: Arc::new(OnceLock::new()),
        }
    }

    /// Set the HTTP loop scheduler (used for dependency injection after construction)
    pub fn set_http_loop_scheduler(&self, scheduler: Arc<HttpLoopScheduler>) -> Result<()> {
        self.http_loop_scheduler.set(scheduler)
            .map_err(|_| SwissPipeError::Generic("HTTP loop scheduler already initialized".to_string()))
    }

    /// Execute a single node based on its type
    pub async fn execute_node(
        &self,
        node: &Node,
        event: WorkflowEvent,
        execution_id: &str,
    ) -> Result<WorkflowEvent> {
        tracing::debug!("NodeExecutor::execute_node called for node '{}' (type: {:?})", node.name, std::mem::discriminant(&node.node_type));
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
        execution_id: &str,
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
            NodeType::HttpRequest { url, method, timeout_seconds, failure_action, retry_config, headers, loop_config } => {
                let config = HttpRequestConfig {
                    url,
                    method,
                    timeout_seconds: *timeout_seconds,
                    failure_action,
                    retry_config,
                    headers,
                    node_name,
                    loop_config,
                };
                self.execute_http_request_node(&config, event, execution_id, node_id).await
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

    /// Execute HTTP request node (supports both single requests and loops)
    async fn execute_http_request_node(
        &self,
        config: &HttpRequestConfig<'_>,
        event: WorkflowEvent,
        execution_id: &str,
        node_id: &str,
    ) -> Result<WorkflowEvent> {
        tracing::debug!("Executing HTTP request node: loop_config_present={}", config.loop_config.is_some());
        match config.loop_config {
            None => {
                tracing::debug!("Taking single HTTP request path (no loop)");
                // Standard HTTP request (existing behavior)
                self.execute_single_http_request(config, event).await
            }
            Some(loop_config) => {
                tracing::debug!("Taking HTTP loop path");
                // HTTP loop request (new functionality)
                self.execute_http_loop(config, event, loop_config, execution_id, node_id).await
            }
        }
    }

    /// Execute a single HTTP request (existing behavior)
    async fn execute_single_http_request(
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

    /// Execute an HTTP loop (new functionality)
    async fn execute_http_loop(
        &self,
        config: &HttpRequestConfig<'_>,
        event: WorkflowEvent,
        loop_config: &crate::workflow::models::LoopConfig,
        execution_id: &str,
        node_id: &str,
    ) -> Result<WorkflowEvent> {
        use crate::async_execution::http_loop_scheduler::{HttpLoopConfig};
        use uuid::Uuid;

        tracing::info!("Starting HTTP loop for node: {}", config.node_name);
        tracing::debug!("HTTP loop config: max_iterations={:?}, interval_seconds={}, termination_condition={:?}",
            loop_config.max_iterations, loop_config.interval_seconds,
            loop_config.termination_condition.as_ref().map(|t| &t.script));

        // Create HTTP loop configuration
        let loop_id = Uuid::new_v4().to_string();
        // Generate execution step ID using execution_id + node_id for consistency
        let execution_step_id = format!("{execution_id}_{node_id}");

        let http_loop_config = HttpLoopConfig {
            loop_id: loop_id.clone(),
            execution_step_id,
            url: config.url.to_string(),
            method: config.method.clone(),
            timeout_seconds: config.timeout_seconds,
            headers: config.headers.clone(),
            loop_config: loop_config.clone(),
            initial_event: event.clone(),
        };

        // Use the injected singleton HTTP loop scheduler
        let loop_scheduler = self.http_loop_scheduler.get()
            .ok_or_else(|| SwissPipeError::Generic("HTTP loop scheduler not initialized".to_string()))?;

        // Schedule the HTTP loop
        let scheduled_loop_id = loop_scheduler.schedule_http_loop(http_loop_config).await?;

        tracing::info!("HTTP loop scheduled with ID: {}, waiting for completion...", scheduled_loop_id);

        // Add debug logging before waiting
        tracing::debug!("About to call wait_for_loop_completion for loop: {}", scheduled_loop_id);

        // Wait for the loop to complete and return the final result
        let final_result = loop_scheduler.wait_for_loop_completion(&scheduled_loop_id).await?;

        tracing::info!("HTTP loop completed successfully: {}", scheduled_loop_id);
        tracing::debug!("Final result from HTTP loop: {:?}", final_result);
        Ok(final_result)
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
    loop_config: &'a Option<crate::workflow::models::LoopConfig>,
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