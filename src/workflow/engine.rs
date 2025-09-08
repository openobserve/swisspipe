use crate::{
    database::{edges, entities, nodes},
    utils::{http_client::AppExecutor, javascript::JavaScriptExecutor},
    workflow::{
        errors::{Result, SwissPipeError},
        models::{Edge, Node, NodeType, Workflow, WorkflowEvent},
    },
    email::service::EmailService,
};
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait};
use std::{collections::{HashMap, HashSet}, sync::Arc};

pub struct WorkflowEngine {
    db: Arc<DatabaseConnection>,
    pub js_executor: Arc<JavaScriptExecutor>,
    pub app_executor: Arc<AppExecutor>,
    pub email_service: Arc<EmailService>,
}

impl WorkflowEngine {
    pub fn new(db: Arc<DatabaseConnection>) -> Result<Self> {
        let js_executor = Arc::new(JavaScriptExecutor::new()?);
        let app_executor = Arc::new(AppExecutor::new());
        let email_service = Arc::new(EmailService::new(db.clone())
            .map_err(|e| SwissPipeError::Generic(e.to_string()))?);
        
        Ok(Self {
            db,
            js_executor,
            app_executor,
            email_service,
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
    
    pub async fn execute_workflow(&self, workflow: &Workflow, event: WorkflowEvent) -> Result<WorkflowEvent> {
        let mut current_event = event;
        let mut current_node_name = workflow.start_node_name.clone();
        let mut visited = HashSet::new();
        
        // Build node lookup for efficiency
        let node_map: HashMap<String, &Node> = workflow.nodes
            .iter()
            .map(|node| (node.name.clone(), node))
            .collect();
        
        loop {
            // Prevent infinite loops
            if visited.contains(&current_node_name) {
                return Err(SwissPipeError::CycleDetected);
            }
            visited.insert(current_node_name.clone());
            
            let node = node_map
                .get(&current_node_name)
                .ok_or_else(|| SwissPipeError::NodeNotFound(current_node_name.clone()))?;
            
            current_event = self.execute_node(node, current_event).await?;
            
            let next_nodes = self.get_next_nodes(workflow, &current_node_name, &current_event)?;
            match next_nodes.len() {
                0 => break, // End of workflow
                1 => current_node_name = next_nodes[0].clone(),
                _ => return Err(SwissPipeError::Config("Multiple paths not supported".to_string())),
            }
        }
        
        Ok(current_event)
    }
    
    async fn execute_node(&self, node: &Node, mut event: WorkflowEvent) -> Result<WorkflowEvent> {
        match &node.node_type {
            NodeType::Trigger { .. } => Ok(event),
            NodeType::Condition { script } => {
                // Execute the condition and store the result
                let condition_result = self.js_executor.execute_condition(script, &event).await?;
                
                tracing::info!("Condition node '{}' evaluated to: {}", node.name, condition_result);
                
                // Store the condition result in the event for edge routing
                event.condition_results.insert(node.name.clone(), condition_result);
                
                // Condition nodes pass through event with stored condition result
                Ok(event)
            }
            NodeType::Transformer { script } => {
                // For transformers, preserve condition_results from input event
                let mut transformed_event = self.js_executor.execute_transformer(script, event.clone()).await
                    .map_err(SwissPipeError::JavaScript)?;
                
                // Preserve condition results from the original event
                transformed_event.condition_results = event.condition_results;
                
                Ok(transformed_event)
            }
            NodeType::App { app_type, url, method, timeout_seconds, failure_action, retry_config, headers } => {
                match failure_action {
                    crate::workflow::models::FailureAction::Retry => {
                        // Use retry_config for retries on failure
                        self.app_executor
                            .execute_app(app_type, url, method, *timeout_seconds, retry_config, event, headers)
                            .await
                    },
                    crate::workflow::models::FailureAction::Continue => {
                        // Try once, if it fails, continue with original event
                        match self.app_executor
                            .execute_app(app_type, url, method, *timeout_seconds, &crate::workflow::models::RetryConfig { max_attempts: 1, ..retry_config.clone() }, event.clone(), headers)
                            .await 
                        {
                            Ok(result) => Ok(result),
                            Err(e) => {
                                tracing::warn!("App node '{}' failed but continuing: {}", node.name, e);
                                Ok(event) // Continue with original event
                            }
                        }
                    },
                    crate::workflow::models::FailureAction::Stop => {
                        // Try once, if it fails, stop the workflow
                        self.app_executor
                            .execute_app(app_type, url, method, *timeout_seconds, &crate::workflow::models::RetryConfig { max_attempts: 1, ..retry_config.clone() }, event, headers)
                            .await
                    }
                }
            }
            NodeType::Email { config } => {
                // Execute email node
                match self.email_service.send_email(config, &event, &node.workflow_id, &node.id).await {
                    Ok(result) => {
                        tracing::info!("Email node '{}' executed successfully: {:?}", node.name, result);
                        // Email nodes pass through the original event
                        Ok(event)
                    }
                    Err(e) => {
                        tracing::error!("Email node '{}' failed: {}", node.name, e);
                        Err(SwissPipeError::Generic(format!("Email node failed: {e}")))
                    }
                }
            }
        }
    }
    
    fn get_next_nodes(&self, workflow: &Workflow, current_node_name: &str, event: &WorkflowEvent) -> Result<Vec<String>> {
        let mut next_nodes = Vec::new();
        
        tracing::info!("Finding next nodes from '{}'", current_node_name);
        
        for edge in &workflow.edges {
            if edge.from_node_name == current_node_name {
                tracing::info!("Processing edge: {} -> {} (condition_result: {:?})", 
                    edge.from_node_name, edge.to_node_name, edge.condition_result);
                
                match edge.condition_result {
                    None => {
                        // Unconditional edge
                        tracing::info!("Following unconditional edge to '{}'", edge.to_node_name);
                        next_nodes.push(edge.to_node_name.clone());
                    }
                    Some(expected_result) => {
                        // Conditional edge - we need to evaluate the condition
                        // This is a simplified approach - in practice, you'd need to store
                        // the condition result from the previous node execution
                        if self.should_follow_conditional_edge(workflow, current_node_name, expected_result, event)? {
                            tracing::info!("Following conditional edge to '{}'", edge.to_node_name);
                            next_nodes.push(edge.to_node_name.clone());
                        } else {
                            tracing::info!("Skipping conditional edge to '{}'", edge.to_node_name);
                        }
                    }
                }
            }
        }
        
        tracing::info!("Next nodes: {:?}", next_nodes);
        Ok(next_nodes)
    }
    
    fn should_follow_conditional_edge(
        &self,
        workflow: &Workflow,
        current_node_name: &str,
        expected_result: bool,
        event: &WorkflowEvent,
    ) -> Result<bool> {
        // Find the current node to check if it's a condition node
        let node = workflow.nodes
            .iter()
            .find(|n| n.name == current_node_name)
            .ok_or_else(|| SwissPipeError::NodeNotFound(current_node_name.to_string()))?;
        
        match &node.node_type {
            NodeType::Condition { .. } => {
                // Get the actual condition result from the event
                let actual_result = event.condition_results
                    .get(current_node_name)
                    .copied()
                    .unwrap_or(false); // Default to false if no result stored
                
                tracing::info!("Edge from '{}': expected={}, actual={}, follow={}", 
                    current_node_name, expected_result, actual_result, actual_result == expected_result);
                
                // Only follow the edge if the actual result matches the expected result
                Ok(actual_result == expected_result)
            }
            _ => {
                // Non-condition nodes should only have unconditional edges
                Ok(true)
            }
        }
    }
}