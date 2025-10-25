use crate::{
    anthropic::{AnthropicService, AnthropicCallConfig},
    async_execution::{HttpLoopScheduler, StepTracker},
    database::human_in_loop_tasks,
    email::{service::EmailService, EmailConfig},
    hil::{HilService, service::HilTaskParams},
    utils::{http_client::AppExecutor, javascript::JavaScriptExecutor},
    variables::{VariableService, TemplateEngine},
    workflow::{
        errors::{Result, SwissPipeError},
        models::{Node, NodeType, WorkflowEvent, FailureAction, RetryConfig, NodeOutput},
    },
    log_workflow_error, log_workflow_warn,
};

/// Configuration for Human in Loop node execution
struct HilNodeConfig<'a> {
    title: &'a str,
    description: Option<&'a str>,
    timeout_seconds: Option<u64>,
    timeout_action: Option<&'a str>,
    required_fields: Option<&'a Vec<String>>,
    metadata: Option<&'a serde_json::Value>,
}

/// Execution context for Human in Loop node to reduce function parameter count
struct HilExecutionContext<'a> {
    workflow_id: &'a str,
    node_id: &'a str,
    node_name: &'a str,
}

/// Parameters for execute_node_by_type function to reduce clippy warnings
struct ExecuteNodeParams<'a> {
    node_type: &'a NodeType,
    execution_id: &'a str,
    node_name: &'a str,
    workflow_id: &'a str,
    node_id: &'a str,
}
use std::sync::{Arc, OnceLock};
use sea_orm::{DatabaseConnection, ActiveModelTrait, Set};
use uuid::Uuid;

pub struct NodeExecutor {
    js_executor: Arc<JavaScriptExecutor>,
    app_executor: Arc<AppExecutor>,
    email_service: Option<Arc<EmailService>>,
    anthropic_service: Arc<AnthropicService>,
    #[allow(dead_code)] // May be used in future for direct database operations
    db: Arc<DatabaseConnection>,
    http_loop_scheduler: Arc<OnceLock<Arc<HttpLoopScheduler>>>,
    hil_service: Arc<OnceLock<Arc<HilService>>>,
    step_tracker: Arc<StepTracker>,
    variable_service: Arc<OnceLock<Arc<VariableService>>>,
    template_engine: Arc<OnceLock<Arc<TemplateEngine>>>,
}

impl NodeExecutor {
    pub fn new(
        js_executor: Arc<JavaScriptExecutor>,
        app_executor: Arc<AppExecutor>,
        email_service: Option<Arc<EmailService>>,
        anthropic_service: Arc<AnthropicService>,
        db: Arc<DatabaseConnection>,
        step_tracker: Arc<StepTracker>,
    ) -> Self {
        Self {
            js_executor,
            app_executor,
            email_service,
            anthropic_service,
            db,
            http_loop_scheduler: Arc::new(OnceLock::new()),
            hil_service: Arc::new(OnceLock::new()),
            step_tracker,
            variable_service: Arc::new(OnceLock::new()),
            template_engine: Arc::new(OnceLock::new()),
        }
    }

    /// Helper method to append current node's input data as a source to the event
    /// This tracks what data each node received, allowing downstream nodes to access historical inputs
    /// Takes ownership of the event to avoid unnecessary cloning
    fn append_source(
        &self,
        mut event: WorkflowEvent,
        node_id: &str,
        node_name: &str,
        node_type: &str,
    ) -> WorkflowEvent {
        use crate::workflow::models::NodeSource;

        // Calculate the next sequence number
        let sequence = event.sources.iter()
            .map(|s| s.sequence)
            .max()
            .map(|max| max + 1)
            .unwrap_or(0);

        // Create a new source entry with the current node's INPUT data
        let source = NodeSource {
            node_id: node_id.to_string(),
            node_name: node_name.to_string(),
            node_type: node_type.to_string(),
            data: event.data.clone(), // Clone data once for the source
            sequence,
            timestamp: chrono::Utc::now(),
            metadata: None,
        };

        // Add the source to the event's history
        event.sources.push(source);

        event
    }

    /// Set the HTTP loop scheduler (used for dependency injection after construction)
    pub fn set_http_loop_scheduler(&self, scheduler: Arc<HttpLoopScheduler>) -> Result<()> {
        self.http_loop_scheduler.set(scheduler)
            .map_err(|_| SwissPipeError::Generic("HTTP loop scheduler already initialized".to_string()))
    }

    /// Set the HIL service (used for dependency injection after construction)
    pub fn set_hil_service(&self, service: Arc<HilService>) -> Result<()> {
        self.hil_service.set(service)
            .map_err(|_| SwissPipeError::Generic("HIL service already initialized".to_string()))
    }

    /// Set the variable service (used for dependency injection after construction)
    pub fn set_variable_service(&self, service: Arc<VariableService>) -> Result<()> {
        self.variable_service.set(service)
            .map_err(|_| SwissPipeError::Generic("Variable service already initialized".to_string()))
    }

    /// Set the template engine (used for dependency injection after construction)
    pub fn set_template_engine(&self, engine: Arc<TemplateEngine>) -> Result<()> {
        self.template_engine.set(engine)
            .map_err(|_| SwissPipeError::Generic("Template engine already initialized".to_string()))
    }

    /// Execute a single node based on its type
    pub async fn execute_node(
        &self,
        node: &Node,
        event: WorkflowEvent,
        execution_id: &str,
    ) -> Result<WorkflowEvent> {
        tracing::debug!("NodeExecutor::execute_node called for node '{}' (type: {:?})", node.name, std::mem::discriminant(&node.node_type));
        let params = ExecuteNodeParams {
            node_type: &node.node_type,
            execution_id,
            node_name: &node.name,
            workflow_id: &node.workflow_id,
            node_id: &node.id,
        };
        self.execute_node_by_type(params, event).await
    }

    /// Execute a single node and return NodeOutput (for 3-handle HIL support)
    pub async fn execute_node_with_output(
        &self,
        node: &Node,
        event: WorkflowEvent,
        execution_id: &str,
    ) -> Result<NodeOutput> {
        // Create execution step for tracking - store complete event with data, metadata, headers, etc.
        let event_json = serde_json::to_value(&event).map_err(|e| {
            log_workflow_warn!(&node.workflow_id, execution_id, &node.id,
                format!("Failed to serialize event for step tracking: {}", e));
            e
        }).unwrap_or(serde_json::json!({}));
        let input_data = Some(&event_json);
        let step_id = self.step_tracker.create_step(
            execution_id,
            &node.id,
            &node.name,
            input_data,
        ).await.map_err(|e| {
            log_workflow_warn!(&node.workflow_id, execution_id, &node.id,
                format!("Failed to create execution step: {}", e));
            e
        }).unwrap_or_else(|_| "unknown_step".to_string());

        // Mark step as running
        if let Err(e) = self.step_tracker.mark_step_running(&step_id).await {
            log_workflow_warn!(&node.workflow_id, execution_id, &node.id,
                format!("Failed to mark step as running: {}", e));
        }

        // Execute the node
        let result = if let NodeType::HumanInLoop { .. } = node.node_type {
            // For HIL nodes, call the special HIL handler that returns NodeOutput::MultiPath
            let params = ExecuteNodeParams {
                node_type: &node.node_type,
                execution_id,
                node_name: &node.name,
                workflow_id: &node.workflow_id,
                node_id: &node.id,
            };
            self.execute_hil_node_with_output(params, event).await
        } else {
            // For all other node types, execute normally and wrap in Continue
            match self.execute_node(node, event, execution_id).await {
                Ok(result_event) => Ok(NodeOutput::Continue(result_event)),
                Err(e) => Err(e),
            }
        };

        // Update step based on result
        match &result {
            Ok(node_output) => {
                let output_data = match node_output {
                    NodeOutput::Continue(event) => Some(&event.data),
                    NodeOutput::Complete => None,
                    NodeOutput::MultiPath(_) => None,
                    NodeOutput::AsyncPending(event) => Some(&event.data),
                };

                if let Err(e) = self.step_tracker.complete_step(&step_id, output_data).await {
                    log_workflow_warn!(&node.workflow_id, execution_id, &node.id,
                        format!("Failed to complete execution step: {}", e));
                }
            }
            Err(error) => {
                let error_message = error.to_string();
                if let Err(e) = self.step_tracker.fail_step(&step_id, &error_message, None).await {
                    log_workflow_warn!(&node.workflow_id, execution_id, &node.id,
                        format!("Failed to mark step as failed: {}", e));
                }
            }
        }

        result
    }

    /// Execute a single node with workflow context (needed for HIL delegation)
    pub async fn execute_node_with_workflow(
        &self,
        node: &Node,
        event: WorkflowEvent,
        execution_id: &str,
        _workflow: Option<&crate::workflow::models::Workflow>,
    ) -> Result<WorkflowEvent> {
        tracing::debug!("NodeExecutor::execute_node_with_workflow called for node '{}' (type: {:?})", node.name, std::mem::discriminant(&node.node_type));
        let params = ExecuteNodeParams {
            node_type: &node.node_type,
            execution_id,
            node_name: &node.name,
            workflow_id: &node.workflow_id,
            node_id: &node.id,
        };
        self.execute_node_by_type(params, event).await
    }

    /// Execute node based on its specific type
    async fn execute_node_by_type(
        &self,
        params: ExecuteNodeParams<'_>,
        event: WorkflowEvent,
    ) -> Result<WorkflowEvent> {
        match params.node_type {
            NodeType::Trigger { .. } => Ok(event),
            NodeType::Condition { script } => {
                self.execute_condition_node(script, event, params.node_name, params.node_id).await
            }
            NodeType::Transformer { script } => {
                self.execute_transformer_node(script, event, params.node_name, params.node_id).await
            }
            NodeType::HttpRequest { url, method, timeout_seconds, failure_action, retry_config, headers, loop_config } => {
                let config = HttpRequestConfig {
                    url,
                    method,
                    timeout_seconds: *timeout_seconds,
                    failure_action,
                    retry_config,
                    headers,
                    node_name: params.node_name,
                    loop_config,
                    workflow_id: params.workflow_id,
                    node_id: params.node_id,
                };
                self.execute_http_request_node(&config, event, params.execution_id, params.node_id).await
            }
            NodeType::OpenObserve { url, authorization_header, timeout_seconds, failure_action, retry_config } => {
                let config = OpenObserveConfig {
                    url,
                    authorization_header,
                    timeout_seconds: *timeout_seconds,
                    failure_action,
                    retry_config,
                    node_name: params.node_name,
                    workflow_id: params.workflow_id,
                    node_id: params.node_id,
                };
                self.execute_openobserve_node(&config, event, params.execution_id).await
            }
            NodeType::Email { config } => {
                self.execute_email_node(config, event, params.execution_id, params.node_id, params.node_name, params.workflow_id).await
            }
            NodeType::Delay { duration, unit } => {
                self.execute_delay_node(*duration, unit, event, params).await
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
                    node_name: params.node_name,
                    workflow_id: params.workflow_id,
                    node_id: params.node_id,
                };
                self.execute_anthropic_node(&config, event, params.execution_id).await
            }
            NodeType::HumanInLoop { title, description, timeout_seconds, timeout_action, required_fields, metadata } => {
                let config = HilNodeConfig {
                    title,
                    description: description.as_deref(),
                    timeout_seconds: *timeout_seconds,
                    timeout_action: timeout_action.as_deref(),
                    required_fields: required_fields.as_ref(),
                    metadata: metadata.as_ref(),
                };
                let context = HilExecutionContext {
                    workflow_id: params.workflow_id,
                    node_id: params.node_id,
                    node_name: params.node_name,
                };
                self.execute_human_in_loop_node(&config, event, &context).await
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
        node_id: &str,
    ) -> Result<WorkflowEvent> {
        use crate::workflow::models::node_type_names;

        // Add source BEFORE transformation (track what the transformer received as input)
        // Takes ownership to avoid cloning
        let event_with_source = self.append_source(event, node_id, node_name, node_type_names::TRANSFORMER);

        let mut transformed_event = self.js_executor.execute_transformer(script, event_with_source.clone()).await
            .map_err(SwissPipeError::JavaScript)?;

        // Preserve condition results and sources from the event with source
        transformed_event.condition_results = event_with_source.condition_results;
        transformed_event.sources = event_with_source.sources;

        tracing::debug!("Transformer node '{}' completed transformation", node_name);
        Ok(transformed_event)
    }

    /// Resolve environment variable and event data templates in a string
    async fn resolve_template(&self, template: &str, event: Option<&WorkflowEvent>) -> Result<String> {
        // Check if template engine and variable service are available
        let Some(template_engine) = self.template_engine.get() else {
            // If not configured, return original string
            return Ok(template.to_string());
        };
        let Some(variable_service) = self.variable_service.get() else {
            // If not configured, return original string
            return Ok(template.to_string());
        };

        // Load all variables as a HashMap
        let variables = variable_service.load_variables_map().await
            .map_err(|e| SwissPipeError::Generic(format!("Failed to load variables: {e}")))?;

        // Create event data context if event is provided
        let event_data = event.map(|e| {
            serde_json::json!({
                "data": e.data,
                "metadata": e.metadata,
                "headers": e.headers,
                "condition_results": e.condition_results,
            })
        });

        // Resolve the template with both environment variables and event data
        template_engine.resolve_with_event(template, &variables, event_data.as_ref())
            .map_err(|e| SwissPipeError::Generic(format!("Template resolution failed: {e}")))
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

        // Resolve templates in URL and trim whitespace
        let resolved_url = self.resolve_template(config.url, Some(&event)).await?.trim().to_string();

        // Resolve templates in headers
        let mut resolved_headers = std::collections::HashMap::new();
        for (key, value) in config.headers {
            let resolved_value = self.resolve_template(value, Some(&event)).await?;
            resolved_headers.insert(key.clone(), resolved_value);
        }

        // Create resolved config
        let resolved_config = HttpRequestConfig {
            url: &resolved_url,
            method: config.method,
            timeout_seconds: config.timeout_seconds,
            failure_action: config.failure_action,
            retry_config: config.retry_config,
            headers: &resolved_headers,
            node_name: config.node_name,
            loop_config: config.loop_config,
            workflow_id: config.workflow_id,
            node_id: config.node_id,
        };

        match resolved_config.loop_config {
            None => {
                tracing::debug!("Taking single HTTP request path (no loop)");
                // Standard HTTP request (existing behavior)
                self.execute_single_http_request(&resolved_config, event, execution_id).await
            }
            Some(loop_config) => {
                tracing::debug!("Taking HTTP loop path");
                // HTTP loop request (new functionality)
                self.execute_http_loop(&resolved_config, event, loop_config, execution_id, node_id).await
            }
        }
    }

    /// Execute a single HTTP request (existing behavior)
    async fn execute_single_http_request(
        &self,
        config: &HttpRequestConfig<'_>,
        event: WorkflowEvent,
        execution_id: &str,
    ) -> Result<WorkflowEvent> {
        use crate::workflow::models::node_type_names;

        // Add source BEFORE making HTTP request (track what the node received as input)
        // Takes ownership to avoid cloning
        let event_with_source = self.append_source(event, config.node_id, config.node_name, node_type_names::HTTP_REQUEST);

        match config.failure_action {
            FailureAction::Retry => {
                let mut result = self.app_executor.execute_http_request(
                    config.url,
                    config.method,
                    config.timeout_seconds,
                    config.retry_config,
                    event_with_source.clone(),
                    config.headers,
                ).await?;
                // Preserve sources in the result
                result.sources = event_with_source.sources;
                Ok(result)
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
                    event_with_source.clone(),
                    config.headers,
                ).await {
                    Ok(mut result) => {
                        result.sources = event_with_source.sources;
                        Ok(result)
                    },
                    Err(e) => {
                        log_workflow_warn!(config.workflow_id, execution_id, config.node_id,
                            format!("HTTP request node '{}' failed but continuing: {}", config.node_name, e));
                        Ok(event_with_source)
                    }
                }
            }
            FailureAction::Stop => {
                let single_attempt_config = RetryConfig {
                    max_attempts: 1,
                    ..config.retry_config.clone()
                };
                let mut result = self.app_executor.execute_http_request(
                    config.url,
                    config.method,
                    config.timeout_seconds,
                    &single_attempt_config,
                    event_with_source.clone(),
                    config.headers,
                ).await?;
                result.sources = event_with_source.sources;
                Ok(result)
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
            url: config.url.to_string(), // Already trimmed after template resolution
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
        execution_id: &str,
    ) -> Result<WorkflowEvent> {
        use crate::workflow::models::node_type_names;

        // Add source BEFORE making OpenObserve request (track what the node received as input)
        // Takes ownership to avoid cloning
        let event_with_source = self.append_source(event, config.node_id, config.node_name, node_type_names::OPEN_OBSERVE);

        // Resolve templates in URL and authorization header
        let resolved_url = self.resolve_template(config.url, Some(&event_with_source)).await?;
        let resolved_auth_header = self.resolve_template(config.authorization_header, Some(&event_with_source)).await?;

        match config.failure_action {
            FailureAction::Retry => {
                let mut result = self.app_executor.execute_openobserve(
                    &resolved_url,
                    &resolved_auth_header,
                    config.timeout_seconds,
                    config.retry_config,
                    event_with_source.clone(),
                ).await?;
                result.sources = event_with_source.sources;
                Ok(result)
            }
            FailureAction::Continue => {
                let single_attempt_config = RetryConfig {
                    max_attempts: 1,
                    ..config.retry_config.clone()
                };
                match self.app_executor.execute_openobserve(
                    &resolved_url,
                    &resolved_auth_header,
                    config.timeout_seconds,
                    &single_attempt_config,
                    event_with_source.clone(),
                ).await {
                    Ok(mut result) => {
                        result.sources = event_with_source.sources;
                        Ok(result)
                    },
                    Err(e) => {
                        log_workflow_warn!(config.workflow_id, execution_id, config.node_id,
                            format!("OpenObserve node '{}' failed but continuing: {}", config.node_name, e));
                        Ok(event_with_source)
                    }
                }
            }
            FailureAction::Stop => {
                let single_attempt_config = RetryConfig {
                    max_attempts: 1,
                    ..config.retry_config.clone()
                };
                let mut result = self.app_executor.execute_openobserve(
                    &resolved_url,
                    &resolved_auth_header,
                    config.timeout_seconds,
                    &single_attempt_config,
                    event_with_source.clone(),
                ).await?;
                result.sources = event_with_source.sources;
                Ok(result)
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
        workflow_id: &str,
    ) -> Result<WorkflowEvent> {
        use crate::workflow::models::node_type_names;

        // Add source BEFORE sending email (track what the node received as input)
        // Takes ownership to avoid cloning
        let event_with_source = self.append_source(event, node_id, node_name, node_type_names::EMAIL);

        tracing::debug!("Executing email node '{}' with config: {:?}", node_name, config);
        tracing::info!("EMAIL NODE: Event received - hil_task present: {:?}", event_with_source.hil_task.is_some());
        if let Some(ref hil_task) = event_with_source.hil_task {
            tracing::info!("EMAIL NODE: HIL task data: {:?}", hil_task);
        }

        // Check if email service is available
        let email_service = self.email_service.as_ref()
            .ok_or_else(|| {
                log_workflow_error!(workflow_id, execution_id, node_id,
                    format!("Email node '{}' cannot execute", node_name),
                    "SMTP not configured - email service unavailable");
                SwissPipeError::Generic("Email service not configured. Set SMTP_HOST and SMTP_FROM_EMAIL environment variables.".to_string())
            })?;

        // Resolve templates in email configuration
        // Note: Only resolve subject for environment variables.
        // Body templates are handled by the email service's template engine,
        // which supports email-specific helpers like {{json event.data}}
        let resolved_subject = self.resolve_template(&config.subject, Some(&event_with_source)).await?;

        // Create resolved email config
        let resolved_config = EmailConfig {
            to: config.to.clone(),
            cc: config.cc.clone(),
            bcc: config.bcc.clone(),
            reply_to: config.reply_to.clone(),
            subject: resolved_subject,
            template_type: config.template_type.clone(),
            body_template: config.body_template.clone(),  // Pass as-is to email service
            text_body_template: config.text_body_template.clone(),  // Pass as-is to email service
            attachments: config.attachments.clone(),
        };

        match email_service.send_email(&resolved_config, &event_with_source, execution_id, node_id).await {
            Ok(result) => {
                tracing::info!("Email node '{}' executed successfully: {:?}", node_name, result);
                Ok(event_with_source)
            }
            Err(e) => {
                log_workflow_error!(workflow_id, execution_id, node_id,
                    format!("Email node '{}' failed", node_name), e);
                Err(SwissPipeError::Generic(format!("Email node failed: {e}")))
            }
        }
    }

    /// Execute delay node
    async fn execute_delay_node(
        &self,
        duration: u64,
        unit: &crate::workflow::models::DelayUnit,
        event: WorkflowEvent,
        params: ExecuteNodeParams<'_>,
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
            log_workflow_warn!(params.workflow_id, params.execution_id, params.node_id,
                format!("Delay node '{}' requested {}ms but capped to {}ms (1 hour)",
                    params.node_name, delay_ms, capped_delay_ms));
        }

        tracing::info!("Delay node '{}' sleeping for {}ms", params.node_name, capped_delay_ms);
        sleep(Duration::from_millis(capped_delay_ms)).await;
        tracing::debug!("Delay node '{}' completed", params.node_name);

        Ok(event)
    }

    /// Execute Anthropic node
    async fn execute_anthropic_node(
        &self,
        config: &AnthropicNodeConfig<'_>,
        event: WorkflowEvent,
        execution_id: &str,
    ) -> Result<WorkflowEvent> {
        use crate::workflow::models::node_type_names;

        // Add source BEFORE calling Anthropic (track what the node received as input)
        // Takes ownership to avoid cloning
        let event_with_source = self.append_source(event, config.node_id, config.node_name, node_type_names::ANTHROPIC);

        // Resolve templates in prompts
        let resolved_system_prompt = match config.system_prompt {
            Some(prompt) => Some(self.resolve_template(prompt, Some(&event_with_source)).await?),
            None => None,
        };
        let resolved_user_prompt = self.resolve_template(config.user_prompt, Some(&event_with_source)).await?;

        let anthropic_config = AnthropicCallConfig {
            model: config.model,
            max_tokens: config.max_tokens,
            temperature: config.temperature,
            system_prompt: resolved_system_prompt.as_deref(),
            user_prompt: &resolved_user_prompt,
            timeout_seconds: config.timeout_seconds,
            retry_config: config.retry_config,
        };

        match config.failure_action {
            FailureAction::Retry => {
                let mut result = self.anthropic_service.call_anthropic(&anthropic_config, &event_with_source).await?;
                result.sources = event_with_source.sources;
                Ok(result)
            }
            FailureAction::Continue => {
                let single_attempt_config = AnthropicCallConfig {
                    retry_config: &RetryConfig {
                        max_attempts: 1,
                        ..config.retry_config.clone()
                    },
                    ..anthropic_config
                };
                match self.anthropic_service.call_anthropic(&single_attempt_config, &event_with_source).await {
                    Ok(mut result) => {
                        result.sources = event_with_source.sources;
                        Ok(result)
                    },
                    Err(e) => {
                        log_workflow_warn!(config.workflow_id, execution_id, config.node_id,
                            format!("Anthropic node '{}' failed but continuing: {}", config.node_name, e));
                        Ok(event_with_source)
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
                let mut result = self.anthropic_service.call_anthropic(&single_attempt_config, &event_with_source).await?;
                result.sources = event_with_source.sources;
                Ok(result)
            }
        }
    }

    /// Execute human in loop node
    async fn execute_human_in_loop_node(
        &self,
        config: &HilNodeConfig<'_>,
        event: WorkflowEvent,
        context: &HilExecutionContext<'_>,
    ) -> Result<WorkflowEvent> {
        tracing::info!("Starting Human in Loop node '{}' execution", context.node_name);

        // Generate unique node execution ID for this HIL task
        let node_execution_id = Uuid::new_v4();
        let task_id = Uuid::new_v4();

        // Calculate timeout timestamp in microseconds if specified
        let timeout_at = config.timeout_seconds.map(|seconds| {
            (chrono::Utc::now() + chrono::Duration::seconds(seconds as i64)).timestamp_micros()
        });

        // Get execution_id from event metadata (set by synchronous execution)
        let execution_id = event.metadata.get("execution_id")
            .ok_or_else(|| SwissPipeError::Generic("execution_id not found in event metadata".to_string()))?;

        tracing::debug!("HIL task creation: execution_id='{}', workflow_id='{}'", execution_id, context.workflow_id);

        // Create HIL task record
        let hil_task = human_in_loop_tasks::ActiveModel {
            id: Set(task_id.to_string()),
            execution_id: Set(execution_id.to_string()),
            node_id: Set(context.node_id.to_string()),
            node_execution_id: Set(node_execution_id.to_string()),
            workflow_id: Set(context.workflow_id.to_string()),
            title: Set(config.title.to_string()),
            description: Set(config.description.map(|d| d.to_string())),
            status: Set("pending".to_string()),
            timeout_at: Set(timeout_at),
            timeout_action: Set(config.timeout_action.map(|a| a.to_string())),
            required_fields: Set(config.required_fields.map(|f| serde_json::to_value(f).unwrap_or(serde_json::Value::Null))),
            metadata: Set(config.metadata.cloned()),
            response_data: Set(None),
            response_received_at: Set(None),
            created_at: Set(chrono::Utc::now().timestamp_micros()),
            updated_at: Set(chrono::Utc::now().timestamp_micros()),
        };

        // Insert HIL task into database
        tracing::debug!("Attempting to insert HIL task with UUID: {} (execution_id='{}', workflow_id='{}')", task_id, execution_id, context.workflow_id);
        let _hil_task = hil_task.insert(self.db.as_ref()).await
            .map_err(|e| {
                log_workflow_error!(context.workflow_id, execution_id.as_str(), context.node_id,
                    format!("HIL task insertion failed - task_id: {}", task_id), e);
                SwissPipeError::Generic(format!("Failed to create HIL task: {e}"))
            })?;

        tracing::info!("Created HIL task with ID: {} for node execution: {}", task_id, node_execution_id);

        // Create enhanced event with HIL task information for notification node
        let mut enhanced_event = event.clone();

        // Add HIL task details to the event data
        // Generate secure token for webhook URL (using first 16 chars of task_id for simplicity)
        let secure_token = task_id.to_string().replace("-", "").chars().take(16).collect::<String>();
        let webhook_url = format!("/api/v1/hil/{node_execution_id}/respond?token={secure_token}");
        let hil_data = serde_json::json!({
            "hil_task_id": task_id,
            "node_execution_id": node_execution_id,
            "title": config.title,
            "description": config.description,
            "required_fields": config.required_fields,
            "metadata": config.metadata,
            "webhook_url": webhook_url,
            "secure_token": secure_token,
            "timeout_seconds": config.timeout_seconds,
            "timeout_action": config.timeout_action,
        });

        // Set HIL task metadata at the top level
        enhanced_event.hil_task = Some(hil_data.clone());
        tracing::info!("SYNC HIL: Set HIL task metadata at the top level: {:?}", hil_data);
        tracing::info!("SYNC HIL: Enhanced event now contains hil_task: {:?}", enhanced_event.hil_task.is_some());

        tracing::debug!("Enhanced event data with HIL information for notification node");

        // HIL task created successfully - return event with HIL metadata
        tracing::info!("HIL task {} created successfully - using dedicated notification system", task_id);

        if let Some(_hil_service) = self.hil_service.get() {
            // Store resumption state for database job queue
            tracing::info!("Database job queue resumption state would be stored for HIL task: {}", node_execution_id);
        } else {
            log_workflow_warn!(context.workflow_id, execution_id.as_str(), context.node_id,
                "HIL service not available - continuing anyway");
        }

        // Return simple event with HIL metadata (dedicated notification node will handle notifications)
        let mut result_event = event.clone();
        result_event.metadata.insert("hil_task_id".to_string(), task_id.to_string());
        result_event.metadata.insert("node_execution_id".to_string(), node_execution_id.to_string());
        result_event.metadata.insert("hil_status".to_string(), "task_created".to_string());

        Ok(result_event)
    }

    /// Execute HIL node with multipath execution support
    async fn execute_hil_node_with_output(
        &self,
        params: ExecuteNodeParams<'_>,
        event: WorkflowEvent,
    ) -> Result<NodeOutput> {
        // Extract HIL configuration
        if let NodeType::HumanInLoop {
            title,
            description,
            timeout_seconds,
            timeout_action,
            required_fields,
            metadata
        } = params.node_type {
            tracing::info!("Starting HIL execution for node '{}'", params.node_name);

            // Queue HIL job for async processing (task creation, notification sending)
            let hil_execution_data = serde_json::json!({
                "node_id": params.node_id,
                "node_name": params.node_name,
                "workflow_id": params.workflow_id,
                "title": title,
                "description": description,
                "timeout_seconds": timeout_seconds,
                "timeout_action": timeout_action,
                "required_fields": required_fields,
                "metadata": metadata,
                "hil_operation": "create_task_and_handle_multipath"
            });

            // Queue the HIL job for async processing
            let _job_payload = serde_json::json!({
                "type": "hil_execution",
                "hil_config": hil_execution_data.to_string(),
            });

            // Log the HIL job queuing (actual queuing will happen via MPSC job distributor in worker pool)
            tracing::info!(
                "HIL node '{}' queued for async execution - workflow_id: {}, title: '{}'",
                params.node_name, params.workflow_id, title
            );

            // Create enhanced event with HIL task data for notification path
            let mut enhanced_event = event.clone();

            // Generate secure tokens and separate webhook URLs for HIL response
            let task_id = uuid::Uuid::new_v4().to_string();
            let node_execution_id = uuid::Uuid::new_v4().to_string();

            // Generate separate URLs for approve and deny actions (using node_execution_id as unique identifier)
            let approve_url = format!("/api/v1/hil/{node_execution_id}/respond?decision=approved");
            let deny_url = format!("/api/v1/hil/{node_execution_id}/respond?decision=denied");

            // Create actual HIL task in database using HIL service
            if let Some(hil_service) = self.hil_service.get() {
                let hil_params = HilTaskParams {
                    execution_id: params.execution_id,
                    workflow_id: params.workflow_id,
                    node_id: params.node_id,
                    node_execution_id: &node_execution_id,
                    config: params.node_type,
                    event: &event,
                };
                let (_actual_task_id, _resumption_state) = hil_service
                    .create_hil_task_and_prepare_resumption(hil_params)
                    .await
                    .map_err(|e| SwissPipeError::Generic(format!("Failed to create HIL task: {e}")))?;

                tracing::info!("Created HIL task in database with node_execution_id: {}", node_execution_id);
            } else {
                log_workflow_warn!(params.workflow_id, params.execution_id, params.node_id,
                    "HIL service not available - HIL task creation skipped");
            }

            // Add HIL task details to the event data
            let hil_data = serde_json::json!({
                "hil_task_id": task_id,
                "node_execution_id": node_execution_id,
                "title": title,
                "description": description,
                "required_fields": required_fields,
                "metadata": metadata,
                "approve_url": approve_url,
                "deny_url": deny_url,
                "timeout_seconds": timeout_seconds,
                "timeout_action": timeout_action,
            });

            // Set HIL task metadata at the top level
            enhanced_event.hil_task = Some(hil_data.clone());
            tracing::info!("ASYNC HIL: Set HIL task metadata at the top level: {:?}", hil_data);
            tracing::info!("ASYNC HIL: Enhanced event now contains hil_task: {:?}", enhanced_event.hil_task.is_some());

            // Return MultiPath to trigger handle_multipath_execution in DAG executor
            // This ensures the notification handle is marked as executed immediately
            let notification_path = crate::workflow::models::ExecutionPath {
                path_type: crate::workflow::models::HilPathType::Notification,
                target_node_ids: vec![], // No notification node - dedicated notification system handles this
                event: enhanced_event, // Use enhanced event with HIL data
                executed_at: Some(chrono::Utc::now()),
            };

            let approved_pending = crate::workflow::models::PendingExecution {
                execution_id: uuid::Uuid::parse_str(params.execution_id)
                    .map_err(|_| SwissPipeError::Generic("Invalid execution ID format".to_string()))?,
                node_id: params.node_id.to_string(),
                path_type: crate::workflow::models::HilPathType::Approved,
                event: event.clone(),
                created_at: chrono::Utc::now(),
            };

            let denied_pending = crate::workflow::models::PendingExecution {
                execution_id: uuid::Uuid::parse_str(params.execution_id)
                    .map_err(|_| SwissPipeError::Generic("Invalid execution ID format".to_string()))?,
                node_id: params.node_id.to_string(),
                path_type: crate::workflow::models::HilPathType::Denied,
                event: event.clone(),
                created_at: chrono::Utc::now(),
            };

            let hil_result = crate::workflow::models::HilMultiPathResult {
                notification_path,
                approved_pending,
                denied_pending,
                hil_task_id: uuid::Uuid::new_v4().to_string(), // Temporary, will be replaced by async job
                node_execution_id: uuid::Uuid::new_v4().to_string(), // Temporary
            };

            tracing::info!(
                "HIL node '{}' returning MultiPath result - workflow_id: {}, title: '{}'",
                params.node_name, params.workflow_id, title
            );

            Ok(NodeOutput::MultiPath(Box::new(hil_result)))
        } else {
            Err(SwissPipeError::Generic("Invalid node type for HIL execution".to_string()))
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
    workflow_id: &'a str,
    node_id: &'a str,
}

struct OpenObserveConfig<'a> {
    url: &'a str,
    authorization_header: &'a str,
    timeout_seconds: u64,
    failure_action: &'a FailureAction,
    retry_config: &'a RetryConfig,
    node_name: &'a str,
    workflow_id: &'a str,
    node_id: &'a str,
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
    workflow_id: &'a str,
    node_id: &'a str,
}