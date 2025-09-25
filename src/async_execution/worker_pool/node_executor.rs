// Node execution logic for different node types
// Handles execution of individual workflow nodes with proper error handling

use std::collections::HashMap;

use crate::anthropic::AnthropicCallConfig;
use crate::email::EmailConfig;
use crate::workflow::{
    engine::WorkflowEngine,
    errors::{Result, SwissPipeError},
    models::{NodeType, WorkflowEvent, Node, Workflow, FailureAction, DelayUnit, HttpMethod},
};
use crate::async_execution::{DelayScheduler, HttpLoopScheduler};

use super::config::DELAY_TIME_MULTIPLIERS;

use std::sync::Arc;
use tokio::sync::RwLock;

/// Configuration for HTTP request node execution
struct HttpRequestConfig<'a> {
    url: &'a str,
    method: &'a HttpMethod,
    timeout_seconds: u32,
    failure_action: &'a FailureAction,
    retry_config: &'a crate::workflow::models::RetryConfig,
    headers: &'a HashMap<String, String>,
    node_name: &'a str,
    loop_config: &'a Option<crate::workflow::models::LoopConfig>,
}

/// Configuration for OpenObserve node execution
struct OpenObserveConfig<'a> {
    url: &'a str,
    authorization_header: &'a str,
    timeout_seconds: u32,
    failure_action: &'a FailureAction,
    retry_config: &'a crate::workflow::models::RetryConfig,
    node_name: &'a str,
}

/// Configuration for Anthropic node execution
struct AnthropicConfig<'a> {
    model: &'a str,
    max_tokens: u32,
    temperature: f64,
    system_prompt: &'a Option<String>,
    user_prompt: &'a str,
    timeout_seconds: u64,
    failure_action: &'a FailureAction,
    retry_config: &'a crate::workflow::models::RetryConfig,
    node_name: &'a str,
}

/// Handles execution of individual nodes in workflows
pub struct NodeExecutor {
    pub workflow_engine: Arc<WorkflowEngine>,
    pub delay_scheduler: Arc<RwLock<Option<Arc<DelayScheduler>>>>,
    pub http_loop_scheduler: Option<Arc<HttpLoopScheduler>>,
}

impl NodeExecutor {
    pub fn new(workflow_engine: Arc<WorkflowEngine>, delay_scheduler: Arc<RwLock<Option<Arc<DelayScheduler>>>>) -> Self {
        Self {
            workflow_engine,
            delay_scheduler,
            http_loop_scheduler: None,
        }
    }

    pub fn new_with_http_loop_scheduler(
        workflow_engine: Arc<WorkflowEngine>,
        delay_scheduler: Arc<RwLock<Option<Arc<DelayScheduler>>>>,
        http_loop_scheduler: Arc<HttpLoopScheduler>
    ) -> Self {
        Self {
            workflow_engine,
            delay_scheduler,
            http_loop_scheduler: Some(http_loop_scheduler),
        }
    }

    /// Execute a single node with the same logic as workflow engine
    pub async fn execute_node(
        &self,
        execution_id: &str,
        workflow: &Workflow,
        node: &Node,
        mut event: WorkflowEvent,
    ) -> Result<WorkflowEvent> {
        match &node.node_type {
            NodeType::Trigger { .. } => Ok(event),
            NodeType::Condition { script } => {
                // Execute the condition and store the result
                let condition_result = self.workflow_engine.js_executor.execute_condition(script, &event).await?;

                tracing::info!("Condition node '{}' evaluated to: {}", node.name, condition_result);

                // Store the condition result in the event for edge routing
                event.condition_results.insert(node.id.clone(), condition_result);

                // Condition nodes pass through event with stored condition result
                Ok(event)
            }
            NodeType::Transformer { script } => {
                // For transformers, preserve condition_results from input event
                let mut transformed_event = self.workflow_engine.js_executor.execute_transformer(script, event.clone()).await
                    .map_err(SwissPipeError::JavaScript)?;

                // Preserve condition results from the original event
                transformed_event.condition_results = event.condition_results;

                Ok(transformed_event)
            }
            NodeType::HttpRequest { url, method, timeout_seconds, failure_action, retry_config, headers, loop_config } => {
                let config = HttpRequestConfig {
                    url,
                    method,
                    timeout_seconds: (*timeout_seconds).try_into().unwrap(),
                    failure_action,
                    retry_config,
                    headers,
                    node_name: &node.name,
                    loop_config,
                };
                self.execute_http_request_node(config, event, execution_id, &node.id).await
            }
            NodeType::OpenObserve { url, authorization_header, timeout_seconds, failure_action, retry_config } => {
                let config = OpenObserveConfig {
                    url,
                    authorization_header,
                    timeout_seconds: (*timeout_seconds).try_into().unwrap(),
                    failure_action,
                    retry_config,
                    node_name: &node.name,
                };
                self.execute_openobserve_node(config, event).await
            }
            NodeType::Email { config } => {
                self.execute_email_node(config, event, execution_id, &node.id, &node.name).await
            }
            NodeType::Delay { duration, unit } => {
                self.execute_delay_node(execution_id, workflow, node, event, *duration, unit).await
            }
            NodeType::Anthropic { model, max_tokens, temperature, system_prompt, user_prompt, timeout_seconds, failure_action, retry_config } => {
                let config = AnthropicConfig {
                    model,
                    max_tokens: *max_tokens,
                    temperature: *temperature,
                    system_prompt,
                    user_prompt,
                    timeout_seconds: *timeout_seconds,
                    failure_action,
                    retry_config,
                    node_name: &node.name,
                };
                self.execute_anthropic_node(config, event).await
            }
        }
    }

    /// Execute HTTP request node with failure action handling (supports both single requests and loops)
    async fn execute_http_request_node(
        &self,
        config: HttpRequestConfig<'_>,
        event: WorkflowEvent,
        execution_id: &str,
        node_id: &str,
    ) -> Result<WorkflowEvent> {
        match config.loop_config {
            None => {
                // Standard HTTP request (existing behavior)
                self.execute_single_http_request(&config, event).await
            }
            Some(loop_config) => {
                // HTTP loop request (new functionality)
                self.execute_http_loop(&config, event, loop_config, execution_id, node_id).await
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
                // Use retry_config for retries on failure
                self.workflow_engine.app_executor
                    .execute_http_request(config.url, config.method, config.timeout_seconds.into(), config.retry_config, event, config.headers)
                    .await
            },
            FailureAction::Continue => {
                // Try once, if it fails, continue with original event
                match self.workflow_engine.app_executor
                    .execute_http_request(config.url, config.method, config.timeout_seconds.into(), &crate::workflow::models::RetryConfig { max_attempts: 1, ..config.retry_config.clone() }, event.clone(), config.headers)
                    .await
                {
                    Ok(result) => Ok(result),
                    Err(e) => {
                        tracing::warn!("HTTP request node '{}' failed but continuing: {}", config.node_name, e);
                        Ok(event) // Continue with original event
                    }
                }
            },
            FailureAction::Stop => {
                // Try once, if it fails, stop the workflow
                self.workflow_engine.app_executor
                    .execute_http_request(config.url, config.method, config.timeout_seconds.into(), &crate::workflow::models::RetryConfig { max_attempts: 1, ..config.retry_config.clone() }, event, config.headers)
                    .await
            }
        }
    }

    /// Execute HTTP request with loop functionality
    async fn execute_http_loop(
        &self,
        config: &HttpRequestConfig<'_>,
        event: WorkflowEvent,
        loop_config: &crate::workflow::models::LoopConfig,
        execution_id: &str,
        node_id: &str,
    ) -> Result<WorkflowEvent> {
        use crate::async_execution::http_loop_scheduler::HttpLoopConfig;
        use uuid::Uuid;

        tracing::info!("Starting HTTP loop for node: {}", config.node_name);

        // Create HTTP loop configuration
        let loop_id = Uuid::new_v4().to_string();
        // Generate execution step ID using execution_id + node_id for consistency
        let execution_step_id = format!("{execution_id}_{node_id}");

        let http_loop_config = HttpLoopConfig {
            loop_id: loop_id.clone(),
            execution_step_id,
            url: config.url.to_string(),
            method: config.method.clone(),
            timeout_seconds: config.timeout_seconds as u64,
            headers: config.headers.clone(),
            loop_config: loop_config.clone(),
            initial_event: event.clone(),
        };

        // Check if HTTP loop scheduler is available
        let http_loop_scheduler = match &self.http_loop_scheduler {
            Some(scheduler) => scheduler,
            None => {
                tracing::error!("HTTP loop functionality is not available - scheduler not initialized");
                return Err(SwissPipeError::Generic(
                    "HTTP loop functionality is not available in worker pool execution. \
                     The HTTP loop scheduler is not initialized.".to_string()
                ));
            }
        };

        // Schedule the HTTP loop
        let scheduled_loop_id = http_loop_scheduler.schedule_http_loop(http_loop_config).await?;

        tracing::info!("HTTP loop scheduled with ID: {}, waiting for completion...", scheduled_loop_id);

        // Add debug logging before waiting
        tracing::debug!("About to call wait_for_loop_completion for loop: {}", scheduled_loop_id);

        // Wait for the loop to complete and return the final result (BLOCKING)
        let final_result = http_loop_scheduler.wait_for_loop_completion(&scheduled_loop_id).await?;

        tracing::info!("HTTP loop completed successfully: {}", scheduled_loop_id);
        Ok(final_result)
    }

    /// Execute OpenObserve node with failure action handling
    async fn execute_openobserve_node(
        &self,
        config: OpenObserveConfig<'_>,
        event: WorkflowEvent,
    ) -> Result<WorkflowEvent> {
        match config.failure_action {
            FailureAction::Retry => {
                self.workflow_engine.app_executor
                    .execute_openobserve(config.url, config.authorization_header, config.timeout_seconds.into(), config.retry_config, event)
                    .await
            },
            FailureAction::Continue => {
                match self.workflow_engine.app_executor
                    .execute_openobserve(config.url, config.authorization_header, config.timeout_seconds.into(), &crate::workflow::models::RetryConfig { max_attempts: 1, ..config.retry_config.clone() }, event.clone())
                    .await
                {
                    Ok(result) => Ok(result),
                    Err(e) => {
                        tracing::warn!("OpenObserve node '{}' failed but continuing: {}", config.node_name, e);
                        Ok(event)
                    }
                }
            },
            FailureAction::Stop => {
                self.workflow_engine.app_executor
                    .execute_openobserve(config.url, config.authorization_header, config.timeout_seconds.into(), &crate::workflow::models::RetryConfig { max_attempts: 1, ..config.retry_config.clone() }, event)
                    .await
            }
        }
    }

    /// Execute email node
    async fn execute_email_node(
        &self,
        config: &EmailConfig,
        event: WorkflowEvent,
        execution_id: &str,
        node_id: &str,
        node_name: &str,
    ) -> Result<WorkflowEvent> {
        // Execute email node
        tracing::debug!("Executing email node '{}' with config: {:?}", node_name, config);
        match self.workflow_engine.email_service.send_email(config, &event, execution_id, node_id).await {
            Ok(result) => {
                tracing::info!("Email node '{}' executed successfully: {:?}", node_name, result);
                // Email nodes pass through the original event
                Ok(event)
            }
            Err(e) => {
                tracing::error!("Email node '{}' failed: {}", node_name, e);
                Err(SwissPipeError::Generic(format!("Email node failed: {e}")))
            }
        }
    }

    /// Execute delay node with scheduler integration
    async fn execute_delay_node(
        &self,
        execution_id: &str,
        workflow: &Workflow,
        node: &Node,
        event: WorkflowEvent,
        duration: u64,
        unit: &DelayUnit,
    ) -> Result<WorkflowEvent> {
        use chrono::Duration as ChronoDuration;

        // Convert delay duration to chrono Duration
        let delay_duration = match unit {
            DelayUnit::Seconds => ChronoDuration::seconds(duration as i64),
            DelayUnit::Minutes => ChronoDuration::minutes(duration as i64),
            DelayUnit::Hours => ChronoDuration::hours(duration as i64),
            DelayUnit::Days => ChronoDuration::days(duration as i64),
        };

        tracing::info!("Delay node '{}' scheduling delay for {} {:?}",
            node.name, duration, unit);

        // Get DelayScheduler from WorkerPool
        if let Some(delay_scheduler) = self.get_delay_scheduler().await {
            // Find next node to continue execution
            let next_nodes = self.get_next_nodes(workflow, &node.id, &event)?;
            if let Some(next_node_id) = next_nodes.first() {
                // Schedule the delay and pause execution
                match delay_scheduler.schedule_delay(
                    execution_id.to_string(),
                    node.id.clone(),
                    next_node_id.clone(),
                    delay_duration,
                    event.clone(),
                ).await {
                    Ok(delay_id) => {
                        tracing::info!(
                            "Delay node '{}' scheduled with ID '{}' - execution will resume at '{}'",
                            node.name, delay_id, next_node_id
                        );

                        // Return a special signal to pause workflow execution here
                        // The workflow will be resumed by the scheduler
                        Err(SwissPipeError::DelayScheduled(delay_id))
                    }
                    Err(e) => {
                        tracing::error!("Failed to schedule delay for node '{}': {}", node.name, e);
                        Err(e)
                    }
                }
            } else {
                tracing::warn!("Delay node '{}' has no next nodes - delay will be ignored", node.name);
                Ok(event)
            }
        } else {
            tracing::error!("DelayScheduler not available - falling back to blocking delay");
            // Fallback to old blocking behavior if scheduler is not available
            use tokio::time::{sleep, Duration};
            let delay_ms = match unit {
                DelayUnit::Seconds => duration.saturating_mul(DELAY_TIME_MULTIPLIERS.seconds),
                DelayUnit::Minutes => duration.saturating_mul(DELAY_TIME_MULTIPLIERS.minutes),
                DelayUnit::Hours => duration.saturating_mul(DELAY_TIME_MULTIPLIERS.hours),
                DelayUnit::Days => duration.saturating_mul(DELAY_TIME_MULTIPLIERS.days),
            };
            // No artificial delay limit since DelayScheduler supports unlimited duration
            sleep(Duration::from_millis(delay_ms)).await;
            tracing::debug!("Delay node '{}' completed (blocking fallback)", node.name);
            Ok(event)
        }
    }

    /// Execute Anthropic node with failure action handling
    async fn execute_anthropic_node(
        &self,
        config: AnthropicConfig<'_>,
        event: WorkflowEvent,
    ) -> Result<WorkflowEvent> {
        // For Anthropic nodes, we use the async version from the workflow engine
        match config.failure_action {
            FailureAction::Retry => {
                self.workflow_engine.anthropic_service
                    .call_anthropic(&AnthropicCallConfig {
                        model: config.model,
                        max_tokens: config.max_tokens,
                        temperature: config.temperature,
                        system_prompt: config.system_prompt.as_deref(),
                        user_prompt: config.user_prompt,
                        timeout_seconds: config.timeout_seconds,
                        retry_config: config.retry_config,
                    }, &event)
                    .await
            },
            FailureAction::Continue => {
                match self.workflow_engine.anthropic_service
                    .call_anthropic(&AnthropicCallConfig {
                        model: config.model,
                        max_tokens: config.max_tokens,
                        temperature: config.temperature,
                        system_prompt: config.system_prompt.as_deref(),
                        user_prompt: config.user_prompt,
                        timeout_seconds: config.timeout_seconds,
                        retry_config: &crate::workflow::models::RetryConfig { max_attempts: 1, ..config.retry_config.clone() },
                    }, &event)
                    .await
                {
                    Ok(result) => Ok(result),
                    Err(e) => {
                        tracing::warn!("Anthropic node '{}' failed but continuing: {}", config.node_name, e);
                        Ok(event)
                    }
                }
            },
            FailureAction::Stop => {
                self.workflow_engine.anthropic_service
                    .call_anthropic(&AnthropicCallConfig {
                        model: config.model,
                        max_tokens: config.max_tokens,
                        temperature: config.temperature,
                        system_prompt: config.system_prompt.as_deref(),
                        user_prompt: config.user_prompt,
                        timeout_seconds: config.timeout_seconds,
                        retry_config: &crate::workflow::models::RetryConfig { max_attempts: 1, ..config.retry_config.clone() },
                    }, &event)
                    .await
            }
        }
    }

    /// Get next nodes - replicating the workflow engine logic
    pub fn get_next_nodes(
        &self,
        workflow: &Workflow,
        current_node_id: &str,
        event: &WorkflowEvent,
    ) -> Result<Vec<String>> {
        let mut next_nodes = Vec::new();

        tracing::info!("Finding next nodes from node_id: {}", current_node_id);

        for edge in &workflow.edges {
            if edge.from_node_id == current_node_id {
                let to_node_id = &edge.to_node_id;

                match edge.condition_result {
                    None => {
                        // Unconditional edge
                        tracing::info!("Following unconditional edge to node_id: {}", to_node_id);
                        next_nodes.push(to_node_id.clone());
                    }
                    Some(expected_result) => {
                        // Conditional edge - we need to evaluate the condition
                        if self.should_follow_conditional_edge(workflow, current_node_id, expected_result, event)? {
                            tracing::info!("Following conditional edge to node_id: {}", to_node_id);
                            next_nodes.push(to_node_id.clone());
                        } else {
                            tracing::info!("Skipping conditional edge to node_id: {}", to_node_id);
                        }
                    }
                }
            }
        }

        tracing::info!("Next node IDs: {:?}", next_nodes);
        Ok(next_nodes)
    }

    /// Should follow conditional edge - replicating workflow engine logic
    fn should_follow_conditional_edge(
        &self,
        workflow: &Workflow,
        current_node_id: &str,
        expected_result: bool,
        event: &WorkflowEvent,
    ) -> Result<bool> {
        // Find the current node to check if it's a condition node
        let node = workflow.nodes
            .iter()
            .find(|n| n.id == current_node_id)
            .ok_or_else(|| SwissPipeError::NodeNotFound(current_node_id.to_string()))?;

        match &node.node_type {
            NodeType::Condition { .. } => {
                // Get the actual condition result from the event - use node ID as key
                let actual_result = event.condition_results
                    .get(current_node_id)
                    .copied()
                    .unwrap_or(false); // Default to false if no result stored

                tracing::info!("Edge from '{}' (id: {}): expected={}, actual={}, follow={}",
                    node.name, current_node_id, expected_result, actual_result, actual_result == expected_result);

                // Only follow the edge if the actual result matches the expected result
                Ok(actual_result == expected_result)
            }
            _ => {
                // Non-condition nodes should only have unconditional edges
                Ok(true)
            }
        }
    }

    /// Get the delay scheduler
    async fn get_delay_scheduler(&self) -> Option<Arc<DelayScheduler>> {
        let delay_scheduler = self.delay_scheduler.read().await;
        delay_scheduler.clone()
    }
}