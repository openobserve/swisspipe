use crate::{
    database::{edges, entities, nodes},
    utils::{http_client::AppExecutor, javascript::JavaScriptExecutor},
    workflow::{
        errors::{Result, SwissPipeError},
        models::{Edge, Node, NodeType, Workflow, WorkflowEvent, InputMergeStrategy},
        input_sync::InputSyncService,
    },
    email::service::EmailService,
};
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait};
use std::{collections::{HashMap, HashSet}, sync::Arc};
use tokio::task::JoinSet;
use uuid::Uuid;

struct ExecutionContext<'a> {
    predecessors: &'a HashMap<String, Vec<String>>,
    successors: &'a HashMap<String, Vec<(String, Option<bool>)>>,
    completed_nodes: &'a HashSet<String>,
    node_outputs: &'a HashMap<String, WorkflowEvent>,
    pending_executions: &'a mut JoinSet<Result<(String, WorkflowEvent)>>,
    execution_id: &'a str,
}

pub struct WorkflowEngine {
    db: Arc<DatabaseConnection>,
    pub js_executor: Arc<JavaScriptExecutor>,
    pub app_executor: Arc<AppExecutor>,
    pub email_service: Arc<EmailService>,
    pub input_sync_service: Arc<InputSyncService>,
}

impl WorkflowEngine {
    pub fn new(db: Arc<DatabaseConnection>) -> Result<Self> {
        let js_executor = Arc::new(JavaScriptExecutor::new()?);
        let app_executor = Arc::new(AppExecutor::new());
        let email_service = Arc::new(EmailService::new(db.clone())
            .map_err(|e| SwissPipeError::Generic(e.to_string()))?);
        let input_sync_service = Arc::new(InputSyncService::new(db.clone()));
        
        Ok(Self {
            db,
            js_executor,
            app_executor,
            email_service,
            input_sync_service,
        })
    }
    
    pub async fn load_workflow(&self, workflow_id: &str) -> Result<Workflow> {
        // Load workflow
        let workflow_model = entities::Entity::find_by_id(workflow_id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| SwissPipeError::WorkflowNotFound(workflow_id.to_string()))?;
        
        // Load nodes
        let node_models = nodes::Entity::find()
            .filter(nodes::Column::WorkflowId.eq(workflow_id))
            .all(self.db.as_ref())
            .await?;
        
        let mut nodes = Vec::new();
        for node_model in node_models {
            let node_type: NodeType = serde_json::from_str(&node_model.config)?;
            nodes.push(Node {
                id: node_model.id,
                workflow_id: node_model.workflow_id,
                name: node_model.name,
                node_type,
                input_merge_strategy: node_model.input_merge_strategy
                    .as_deref()
                    .and_then(|s| serde_json::from_str(s).ok()),
            });
        }
        
        // Load edges
        let edge_models = edges::Entity::find()
            .filter(edges::Column::WorkflowId.eq(workflow_id))
            .all(self.db.as_ref())
            .await?;
        
        let edges = edge_models
            .into_iter()
            .map(|edge_model| Edge {
                id: edge_model.id,
                workflow_id: edge_model.workflow_id,
                from_node_name: edge_model.from_node_name,
                to_node_name: edge_model.to_node_name,
                condition_result: edge_model.condition_result,
            })
            .collect();
        
        Ok(Workflow {
            id: workflow_model.id,
            name: workflow_model.name,
            description: workflow_model.description,
            start_node_name: workflow_model.start_node_name,
            nodes,
            edges,
        })
    }
    
    pub async fn get_workflow(&self, workflow_id: &str) -> Result<Option<Workflow>> {
        match self.load_workflow(workflow_id).await {
            Ok(workflow) => Ok(Some(workflow)),
            Err(SwissPipeError::WorkflowNotFound(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }
    
    pub async fn execute_workflow(&self, workflow: &Workflow, event: WorkflowEvent) -> Result<WorkflowEvent> {
        let execution_id = Uuid::new_v4().to_string();
        tracing::info!("Starting DAG execution for workflow '{}' with execution_id '{}'", workflow.name, execution_id);

        // Build graph structures for DAG traversal
        let node_map: HashMap<String, &Node> = workflow.nodes
            .iter()
            .map(|node| (node.name.clone(), node))
            .collect();

        let predecessors = self.build_predecessor_map(workflow);
        let successors = self.build_successor_map(workflow);
        
        // Initialize execution state
        let mut completed_nodes: HashSet<String> = HashSet::new();
        let mut node_outputs: HashMap<String, WorkflowEvent> = HashMap::new();
        let mut pending_executions: JoinSet<Result<(String, WorkflowEvent)>> = JoinSet::new();
        
        // Start with the trigger node
        let start_node = node_map.get(&workflow.start_node_name)
            .ok_or_else(|| SwissPipeError::NodeNotFound(workflow.start_node_name.clone()))?;
        
        // Execute trigger node first
        let trigger_result = self.execute_single_node(start_node, vec![event], &execution_id).await?;
        completed_nodes.insert(workflow.start_node_name.clone());
        node_outputs.insert(workflow.start_node_name.clone(), trigger_result.clone());
        
        tracing::info!("Completed trigger node '{}', looking for next nodes", workflow.start_node_name);

        // Schedule immediately ready nodes for concurrent execution
        let mut execution_context = ExecutionContext {
            predecessors: &predecessors,
            successors: &successors,
            completed_nodes: &completed_nodes,
            node_outputs: &node_outputs,
            pending_executions: &mut pending_executions,
            execution_id: &execution_id,
        };
        self.schedule_ready_nodes(workflow, &mut execution_context).await?;

        // Main execution loop for DAG traversal
        while !pending_executions.is_empty() {
            // Wait for any node to complete
            if let Some(result) = pending_executions.join_next().await {
                match result {
                    Ok(Ok((node_name, output))) => {
                        tracing::info!("Node '{}' completed successfully", node_name);
                        completed_nodes.insert(node_name.clone());
                        node_outputs.insert(node_name.clone(), output);

                        // Schedule any newly ready nodes
                        let mut execution_context = ExecutionContext {
                            predecessors: &predecessors,
                            successors: &successors,
                            completed_nodes: &completed_nodes,
                            node_outputs: &node_outputs,
                            pending_executions: &mut pending_executions,
                            execution_id: &execution_id,
                        };
                        self.schedule_ready_nodes(workflow, &mut execution_context).await?;
                    }
                    Ok(Err(e)) => {
                        // Node execution failed, cancel remaining tasks
                        pending_executions.abort_all();
                        return Err(e);
                    }
                    Err(join_error) => {
                        // Task join failed
                        pending_executions.abort_all();
                        return Err(SwissPipeError::Generic(format!("Task join error: {join_error}")));
                    }
                }
            }
        }

        tracing::info!("DAG execution completed successfully for execution_id '{}'", execution_id);

        // Return the final output (last completed node or aggregate)
        self.get_final_output(workflow, &completed_nodes, &node_outputs)
    }

    fn build_predecessor_map(&self, workflow: &Workflow) -> HashMap<String, Vec<String>> {
        let mut predecessors: HashMap<String, Vec<String>> = HashMap::new();
        
        for edge in &workflow.edges {
            predecessors
                .entry(edge.to_node_name.clone())
                .or_default()
                .push(edge.from_node_name.clone());
        }
        
        predecessors
    }

    fn build_successor_map(&self, workflow: &Workflow) -> HashMap<String, Vec<(String, Option<bool>)>> {
        let mut successors: HashMap<String, Vec<(String, Option<bool>)>> = HashMap::new();
        
        for edge in &workflow.edges {
            successors
                .entry(edge.from_node_name.clone())
                .or_default()
                .push((edge.to_node_name.clone(), edge.condition_result));
        }
        
        successors
    }

    async fn schedule_ready_nodes(
        &self,
        workflow: &Workflow,
        execution_context: &mut ExecutionContext<'_>,
    ) -> Result<()> {
        // Find all nodes that are ready to execute
        for node in &workflow.nodes {
            if execution_context.completed_nodes.contains(&node.name) {
                continue; // Already completed
            }
            
            // Check if all predecessors are completed
            let node_predecessors = execution_context.predecessors.get(&node.name).cloned().unwrap_or_default();
            let all_predecessors_ready = node_predecessors.iter().all(|pred| execution_context.completed_nodes.contains(pred));
            
            if all_predecessors_ready {
                tracing::info!("Node '{}' is ready for execution", node.name);
                
                // Collect inputs from predecessors
                let inputs = self.collect_node_inputs(
                    &node.name,
                    &node_predecessors,
                    execution_context.successors,
                    execution_context.node_outputs,
                    execution_context.completed_nodes,
                )?;
                
                // Clone necessary data for the async task
                let node_clone = node.clone();
                let execution_id = execution_context.execution_id.to_string();
                let engine_components = (
                    self.js_executor.clone(),
                    self.app_executor.clone(),
                    self.email_service.clone(),
                    self.input_sync_service.clone(),
                );
                
                // Spawn async execution
                execution_context.pending_executions.spawn(async move {
                    let result = Self::execute_node_with_components(
                        &node_clone,
                        inputs,
                        &execution_id,
                        engine_components,
                    ).await?;
                    Ok((node_clone.name, result))
                });
            }
        }
        
        Ok(())
    }

    fn collect_node_inputs(
        &self,
        node_name: &str,
        predecessors: &[String],
        successors: &HashMap<String, Vec<(String, Option<bool>)>>,
        node_outputs: &HashMap<String, WorkflowEvent>,
        _completed_nodes: &HashSet<String>,
    ) -> Result<Vec<WorkflowEvent>> {
        let mut inputs = Vec::new();
        
        for pred_name in predecessors {
            if let Some(pred_output) = node_outputs.get(pred_name) {
                // Check if this edge should be followed based on conditions
                if let Some(pred_successors) = successors.get(pred_name) {
                    for (succ_name, condition_result) in pred_successors {
                        if succ_name == node_name {
                            if let Some(expected_result) = condition_result {
                                // Check if condition matches
                                let actual_result = pred_output.condition_results
                                    .get(pred_name)
                                    .copied()
                                    .unwrap_or(false);
                                
                                if actual_result == *expected_result {
                                    inputs.push(pred_output.clone());
                                }
                            } else {
                                // Unconditional edge
                                inputs.push(pred_output.clone());
                            }
                        }
                    }
                }
            }
        }
        
        Ok(inputs)
    }

    async fn execute_single_node(
        &self,
        node: &Node,
        inputs: Vec<WorkflowEvent>,
        execution_id: &str,
    ) -> Result<WorkflowEvent> {
        let components = (
            self.js_executor.clone(),
            self.app_executor.clone(), 
            self.email_service.clone(),
            self.input_sync_service.clone(),
        );
        
        Self::execute_node_with_components(node, inputs, execution_id, components).await
    }

    async fn execute_node_with_components(
        node: &Node,
        inputs: Vec<WorkflowEvent>,
        _execution_id: &str,
        (js_executor, app_executor, email_service, _input_sync_service): (
            Arc<JavaScriptExecutor>,
            Arc<AppExecutor>,
            Arc<EmailService>,
            Arc<InputSyncService>,
        ),
    ) -> Result<WorkflowEvent> {
        tracing::info!("Executing node '{}' with {} inputs", node.name, inputs.len());

        // Merge inputs based on strategy
        let input_event = if inputs.len() <= 1 {
            inputs.into_iter().next().unwrap_or_else(|| WorkflowEvent {
                data: serde_json::Value::Object(serde_json::Map::new()),
                metadata: HashMap::new(),
                headers: HashMap::new(),
                condition_results: HashMap::new(),
            })
        } else {
            let merge_strategy = node.input_merge_strategy
                .as_ref()
                .unwrap_or(&InputMergeStrategy::WaitForAll);
            
            InputSyncService::merge_inputs(inputs, merge_strategy)?
        };

        // Execute the node based on its type
        Self::execute_node_by_type(node, input_event, js_executor, app_executor, email_service).await
    }

    async fn execute_node_by_type(
        node: &Node,
        mut event: WorkflowEvent,
        js_executor: Arc<JavaScriptExecutor>,
        app_executor: Arc<AppExecutor>,
        email_service: Arc<EmailService>,
    ) -> Result<WorkflowEvent> {
        match &node.node_type {
            NodeType::Trigger { .. } => Ok(event),
            NodeType::Condition { script } => {
                let condition_result = js_executor.execute_condition(script, &event).await?;
                tracing::info!("Condition node '{}' evaluated to: {}", node.name, condition_result);
                event.condition_results.insert(node.name.clone(), condition_result);
                Ok(event)
            }
            NodeType::Transformer { script } => {
                let mut transformed_event = js_executor.execute_transformer(script, event.clone()).await
                    .map_err(SwissPipeError::JavaScript)?;
                transformed_event.condition_results = event.condition_results;
                Ok(transformed_event)
            }
            NodeType::App { app_type, url, method, timeout_seconds, failure_action, retry_config, headers } => {
                match failure_action {
                    crate::workflow::models::FailureAction::Retry => {
                        app_executor
                            .execute_app(app_type, url, method, *timeout_seconds, retry_config, event, headers)
                            .await
                    },
                    crate::workflow::models::FailureAction::Continue => {
                        match app_executor
                            .execute_app(app_type, url, method, *timeout_seconds, &crate::workflow::models::RetryConfig { max_attempts: 1, ..retry_config.clone() }, event.clone(), headers)
                            .await 
                        {
                            Ok(result) => Ok(result),
                            Err(e) => {
                                tracing::warn!("App node '{}' failed but continuing: {}", node.name, e);
                                Ok(event)
                            }
                        }
                    },
                    crate::workflow::models::FailureAction::Stop => {
                        app_executor
                            .execute_app(app_type, url, method, *timeout_seconds, &crate::workflow::models::RetryConfig { max_attempts: 1, ..retry_config.clone() }, event, headers)
                            .await
                    }
                }
            }
            NodeType::Email { config } => {
                match email_service.send_email(config, &event, &node.workflow_id, &node.id).await {
                    Ok(result) => {
                        tracing::info!("Email node '{}' executed successfully: {:?}", node.name, result);
                        Ok(event)
                    }
                    Err(e) => {
                        tracing::error!("Email node '{}' failed: {}", node.name, e);
                        Err(SwissPipeError::Generic(format!("Email node failed: {e}")))
                    }
                }
            }
            NodeType::Delay { duration, unit } => {
                use crate::workflow::models::DelayUnit;
                use tokio::time::{sleep, Duration};
                
                let delay_ms = match unit {
                    DelayUnit::Seconds => duration * 1000,
                    DelayUnit::Minutes => duration * 60 * 1000,
                    DelayUnit::Hours => duration * 60 * 60 * 1000,
                    DelayUnit::Days => duration * 24 * 60 * 60 * 1000,
                };
                
                tracing::info!("Delay node '{}' waiting for {} {:?} ({} ms)", 
                    node.name, duration, unit, delay_ms);
                
                sleep(Duration::from_millis(delay_ms)).await;
                tracing::debug!("Delay node '{}' completed", node.name);
                
                Ok(event)
            }
        }
    }

    fn get_final_output(
        &self,
        workflow: &Workflow,
        completed_nodes: &HashSet<String>,
        node_outputs: &HashMap<String, WorkflowEvent>,
    ) -> Result<WorkflowEvent> {
        // Find leaf nodes (nodes with no successors)
        let successors = self.build_successor_map(workflow);
        let mut leaf_nodes = Vec::new();
        
        for node in &workflow.nodes {
            if completed_nodes.contains(&node.name)
                && (!successors.contains_key(&node.name) || successors[&node.name].is_empty()) {
                    leaf_nodes.push(node.name.clone());
                }
        }
        
        if leaf_nodes.len() == 1 {
            // Single leaf node - return its output
            Ok(node_outputs[&leaf_nodes[0]].clone())
        } else if leaf_nodes.len() > 1 {
            // Multiple leaf nodes - merge their outputs
            let leaf_outputs: Vec<WorkflowEvent> = leaf_nodes.iter()
                .filter_map(|name| node_outputs.get(name).cloned())
                .collect();
            
            InputSyncService::merge_inputs(leaf_outputs, &InputMergeStrategy::WaitForAll)
        } else {
            // No leaf nodes found - this shouldn't happen in a valid DAG
            Err(SwissPipeError::Generic("No leaf nodes found in completed workflow".to_string()))
        }
    }
    
    
}