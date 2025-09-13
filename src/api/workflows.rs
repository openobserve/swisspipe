use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    Router,
};
use sea_orm::{ActiveModelTrait, EntityTrait, Set, ColumnTrait, QueryFilter, TransactionTrait, PaginatorTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    database::{edges, entities, nodes, workflow_executions},
    workflow::{
        models::{Edge, Node, NodeType, HttpMethod, RetryConfig, FailureAction},
        validation::WorkflowValidator,
    },
    AppState,
};
use std::collections::{HashMap, HashSet};

#[derive(Deserialize)]
pub struct CreateWorkflowRequest {
    pub name: String,
    pub description: Option<String>,
    pub nodes: Vec<NodeRequest>,
    pub edges: Vec<EdgeRequest>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct NodeRequest {
    pub id: Option<String>, // For updates: existing node ID
    pub name: String,
    pub node_type: NodeType,
    pub position_x: Option<f64>,
    pub position_y: Option<f64>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct EdgeRequest {
    pub from_node_id: String, // Source node ID
    pub to_node_id: String,   // Target node ID
    pub condition_result: Option<bool>,
}

#[derive(Serialize)]
pub struct NodeResponse {
    pub id: String,
    pub name: String,
    pub node_type: NodeType,
    pub position_x: f64,
    pub position_y: f64,
}

#[derive(Serialize)]
pub struct EdgeResponse {
    pub id: String,
    pub from_node_id: String,   // Source node ID
    pub to_node_id: String,     // Target node ID
    pub condition_result: Option<bool>,
}

#[derive(Serialize)]
pub struct WorkflowResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub start_node_id: String,   // Starting node ID
    pub endpoint_url: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub nodes: Vec<NodeResponse>,
    pub edges: Vec<EdgeResponse>,
}

#[derive(Serialize)]
pub struct WorkflowListResponse {
    pub workflows: Vec<WorkflowResponse>,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub details: Option<String>,
}

pub fn routes() -> Router<AppState> {
    #[allow(unused_imports)]
    use axum::routing::{delete, get, post, put};
    
    Router::new()
        .route("/", get(list_workflows).post(create_workflow))
        .route("/:id", get(get_workflow).put(update_workflow).delete(delete_workflow))
}

pub async fn list_workflows(
    State(state): State<AppState>,
) -> std::result::Result<Json<WorkflowListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let workflows = entities::Entity::find()
        .all(&*state.db)
        .await
        .map_err(|e| {
            tracing::error!("Database error in list_workflows: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                error: "Failed to fetch workflows from database".to_string(),
                details: Some(format!("Database error: {e}")),
            }))
        })?;

    let workflow_responses: Vec<WorkflowResponse> = workflows
        .into_iter()
        .map(|w| WorkflowResponse {
            endpoint_url: format!("/api/v1/{}/trigger", w.id),
            id: w.id.clone(),
            name: w.name,
            description: w.description,
            start_node_id: w.start_node_id.unwrap_or_else(|| {
                tracing::warn!("Workflow {} has no start_node_id", w.id);
                "".to_string()
            }),
            created_at: w.created_at,
            updated_at: w.updated_at,
            nodes: vec![], // Not included in list view for performance
            edges: vec![], // Not included in list view for performance
        })
        .collect();

    Ok(Json(WorkflowListResponse {
        workflows: workflow_responses,
    }))
}

pub async fn create_workflow(
    State(state): State<AppState>,
    Json(request): Json<CreateWorkflowRequest>,
) -> std::result::Result<(StatusCode, Json<WorkflowResponse>), StatusCode> {
    let workflow_id = Uuid::new_v4().to_string();
    
    // Auto-create start node
    let start_node_id = Uuid::new_v4().to_string();
    let start_node = Node {
        id: start_node_id.clone(),
        workflow_id: workflow_id.clone(),
        name: "Start".to_string(),
        node_type: NodeType::Trigger {
            methods: vec![HttpMethod::Get, HttpMethod::Post, HttpMethod::Put]
        },
        input_merge_strategy: None,
    };

    // Convert request nodes to internal models
    let mut nodes: Vec<Node> = vec![start_node]; // Start with the auto-created start node
    nodes.extend(request.nodes.iter().map(|n| Node {
        id: n.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string()),
        workflow_id: workflow_id.clone(),
        name: n.name.clone(),
        node_type: n.node_type.clone(),
        input_merge_strategy: None,
    }));
    
    let edges: Vec<Edge> = request.edges.iter().map(|e| Edge {
        id: Uuid::new_v4().to_string(),
        workflow_id: workflow_id.clone(),
        from_node_id: e.from_node_id.clone(),
        to_node_id: e.to_node_id.clone(),
        condition_result: e.condition_result,
    }).collect();

    // Validate workflow structure
    if let Err(validation_error) = WorkflowValidator::validate_workflow(
        &request.name,
        &start_node_id,
        &nodes,
        &edges,
    ) {
        tracing::warn!(
            "Workflow creation validation failed: workflow_name='{}', nodes_count={}, edges_count={}, error='{}'", 
            request.name, nodes.len(), edges.len(), validation_error
        );
        return Err(StatusCode::BAD_REQUEST);
    }

    // Check for warnings and log them
    let warnings = WorkflowValidator::validate_condition_completeness(&nodes, &edges);
    for warning in warnings {
        tracing::warn!("Workflow creation warning: workflow_name='{}', warning='{}'", request.name, warning);
    }

    // Create workflow
    let workflow_model = entities::ActiveModel {
        id: Set(workflow_id.clone()),
        name: Set(request.name.clone()),
        description: Set(request.description.clone()),
        start_node_id: Set(Some(start_node_id.clone())),
        ..Default::default()
    };

    let workflow = workflow_model
        .insert(&*state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create workflow: workflow_id={}, name='{}', error={:?}", 
                           workflow_id, request.name, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Create all nodes (start node + user nodes)
    for node in &nodes {
        let node_config = serde_json::to_string(&node.node_type)
            .map_err(|e| {
                tracing::error!("Failed to serialize node config during creation: workflow_id={}, node_id={}, error={:?}", 
                               workflow_id, node.id, e);
                StatusCode::BAD_REQUEST
            })?;

        let (position_x, position_y) = if node.id == start_node_id {
            // Position start node at top-middle of canvas
            (400.0, 50.0)
        } else {
            // For user nodes, try to find position from request, otherwise default
            let user_node = request.nodes.iter().find(|n| 
                n.id.as_ref().unwrap_or(&String::new()) == &node.id || n.name == node.name
            );
            (
                user_node.and_then(|n| n.position_x).unwrap_or(100.0),
                user_node.and_then(|n| n.position_y).unwrap_or(100.0)
            )
        };

        let node_model = nodes::ActiveModel {
            id: Set(node.id.clone()),
            workflow_id: Set(workflow_id.clone()),
            name: Set(node.name.clone()),
            node_type: Set(match &node.node_type {
                NodeType::Trigger { .. } => "trigger".to_string(),
                NodeType::Condition { .. } => "condition".to_string(),
                NodeType::Transformer { .. } => "transformer".to_string(),
                NodeType::HttpRequest { .. } => "http_request".to_string(),
                NodeType::OpenObserve { .. } => "openobserve".to_string(),
                NodeType::Email { .. } => "email".to_string(),
                NodeType::Delay { .. } => "delay".to_string(),
            }),
            config: Set(node_config),
            position_x: Set(position_x),
            position_y: Set(position_y),
            ..Default::default()
        };

        node_model
            .insert(&*state.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to create node: workflow_id={}, node_id={}, node_name='{}', error={:?}", 
                               workflow_id, node.id, node.name, e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
    }

    // Create edges using provided node IDs
    for edge_req in request.edges {
        let edge_model = edges::ActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            workflow_id: Set(workflow_id.clone()),
            from_node_id: Set(edge_req.from_node_id.clone()),
            to_node_id: Set(edge_req.to_node_id.clone()),
            condition_result: Set(edge_req.condition_result),
            ..Default::default()
        };

        edge_model
            .insert(&*state.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to create edge: workflow_id={}, from_node={}, to_node={}, error={:?}", 
                               workflow_id, edge_req.from_node_id, edge_req.to_node_id, e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
    }

    // Fetch nodes
    let nodes = nodes::Entity::find()
        .filter(nodes::Column::WorkflowId.eq(&workflow_id))
        .all(&*state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch created nodes: workflow_id={}, error={:?}", workflow_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Fetch edges
    let edges = edges::Entity::find()
        .filter(edges::Column::WorkflowId.eq(&workflow_id))
        .all(&*state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch created edges: workflow_id={}, error={:?}", workflow_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Convert nodes to response format
    let node_responses: Vec<NodeResponse> = nodes
        .into_iter()
        .map(|node| {
            let node_type: NodeType = serde_json::from_str(&node.config)
                .unwrap_or(NodeType::HttpRequest {
                    url: "".to_string(),
                    method: HttpMethod::Get,
                    timeout_seconds: 30,
                    failure_action: FailureAction::Stop,
                    retry_config: RetryConfig::default(),
                    headers: std::collections::HashMap::new(),
                });
            NodeResponse {
                id: node.id,
                name: node.name,
                node_type,
                position_x: node.position_x,
                position_y: node.position_y,
            }
        })
        .collect();

    // Convert edges to response format
    let edge_responses: Vec<EdgeResponse> = edges
        .into_iter()
        .map(|edge| EdgeResponse {
            id: edge.id.clone(),
            from_node_id: edge.from_node_id.clone(),
            to_node_id: edge.to_node_id.clone(),
            condition_result: edge.condition_result,
        })
        .collect();

    let response = WorkflowResponse {
        id: workflow.id.clone(),
        name: workflow.name,
        description: workflow.description,
        start_node_id: start_node_id.clone(),
        endpoint_url: format!("/api/v1/{}/trigger", workflow.id),
        created_at: workflow.created_at,
        updated_at: workflow.updated_at,
        nodes: node_responses,
        edges: edge_responses,
    };

    // Cache the newly created workflow metadata for performance
    state.workflow_cache.put(workflow_id.clone(), start_node_id.clone()).await;

    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn get_workflow(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> std::result::Result<Json<WorkflowResponse>, StatusCode> {
    let workflow = entities::Entity::find_by_id(&id)
        .one(&*state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch workflow: workflow_id={}, error={:?}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or_else(|| {
            tracing::warn!("Workflow not found: workflow_id={}", id);
            StatusCode::NOT_FOUND
        })?;

    // Fetch nodes
    let nodes = nodes::Entity::find()
        .filter(nodes::Column::WorkflowId.eq(&id))
        .all(&*state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch workflow nodes: workflow_id={}, error={:?}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Fetch edges
    let edges = edges::Entity::find()
        .filter(edges::Column::WorkflowId.eq(&id))
        .all(&*state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch workflow edges: workflow_id={}, error={:?}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Convert nodes to response format
    let node_responses: Vec<NodeResponse> = nodes
        .into_iter()
        .map(|node| {
            let node_type: NodeType = serde_json::from_str(&node.config)
                .unwrap_or(NodeType::HttpRequest {
                    url: "".to_string(),
                    method: HttpMethod::Get,
                    timeout_seconds: 30,
                    failure_action: FailureAction::Stop,
                    retry_config: RetryConfig::default(),
                    headers: std::collections::HashMap::new(),
                });
            NodeResponse {
                id: node.id,
                name: node.name,
                node_type,
                position_x: node.position_x,
                position_y: node.position_y,
            }
        })
        .collect();

    // Convert edges to response format
    let edge_responses: Vec<EdgeResponse> = edges
        .into_iter()
        .map(|edge| EdgeResponse {
            id: edge.id.clone(),
            from_node_id: edge.from_node_id.clone(),
            to_node_id: edge.to_node_id.clone(),
            condition_result: edge.condition_result,
        })
        .collect();

    let response = WorkflowResponse {
        id: workflow.id.clone(),
        name: workflow.name,
        description: workflow.description,
        start_node_id: workflow.start_node_id.clone().ok_or_else(|| {
            tracing::error!("Workflow {} missing start_node_id in get_workflow", workflow.id);
            StatusCode::INTERNAL_SERVER_ERROR
        })?,
        endpoint_url: format!("/api/v1/{}/trigger", workflow.id),
        created_at: workflow.created_at,
        updated_at: workflow.updated_at,
        nodes: node_responses,
        edges: edge_responses,
    };

    // Update cache with current workflow metadata
    if let Some(start_node_id) = &workflow.start_node_id {
        state.workflow_cache.put(id.clone(), start_node_id.clone()).await;
    } else {
        tracing::warn!("Cannot cache workflow {} - missing start_node_id", id);
    }

    Ok(Json(response))
}

#[derive(Debug)]
struct NodeOperations<'a> {
    to_create: Vec<NodeRequest>,
    to_update: Vec<(String, NodeRequest, &'a nodes::Model)>, // (existing_id, updated_data, existing_node_ref)
    to_delete: Vec<String>,
}

#[derive(Debug)]
struct EdgeOperations {
    to_create: Vec<EdgeRequest>,
    to_delete: Vec<String>,
}

/// Categorize node changes for differential updates
fn categorize_node_changes<'a>(
    existing_nodes: &'a [nodes::Model],
    new_nodes: &[NodeRequest],
    start_node_id: &str,
) -> NodeOperations<'a> {
    let existing_map: HashMap<String, &nodes::Model> = existing_nodes
        .iter()
        .map(|n| (n.id.clone(), n))
        .collect();
    
    let mut to_create = Vec::new();
    let mut to_update = Vec::new();
    let mut processed_ids = HashSet::new();
    
    // Process new nodes - determine if they should be created or updated
    for new_node in new_nodes {
        if let Some(node_id) = &new_node.id {
            if let Some(existing_node) = existing_map.get(node_id.as_str()) {
                // Node exists - check if it needs updating
                if node_needs_update(existing_node, new_node) {
                    to_update.push((node_id.clone(), (*new_node).clone(), *existing_node));
                }
                processed_ids.insert(node_id.clone());
            } else {
                // Node has ID but doesn't exist in database - create it
                to_create.push((*new_node).clone());
            }
        } else {
            // Node has no ID - create new one
            to_create.push((*new_node).clone());
        }
    }
    
    // Find nodes to delete (existing but not in new list, excluding start node)
    let to_delete: Vec<String> = existing_nodes
        .iter()
        .filter(|node| node.id != start_node_id && !processed_ids.contains(&node.id))
        .map(|node| node.id.clone())
        .collect();
    
    NodeOperations {
        to_create,
        to_update,
        to_delete,
    }
}

/// Check if a node needs updating by comparing existing vs new data
fn node_needs_update(existing: &nodes::Model, new: &NodeRequest) -> bool {
    // Compare name
    if existing.name != new.name {
        tracing::debug!("Node needs update - name changed: node_id={}, old_name='{}', new_name='{}'", 
                       existing.id, existing.name, new.name);
        return true;
    }
    
    // Compare node type string representation
    let new_node_type_str = node_type_to_string(&new.node_type);
    
    if existing.node_type != new_node_type_str {
        tracing::debug!("Node needs update - type changed: node_id={}, old_type='{}', new_type='{}'", 
                       existing.id, existing.node_type, new_node_type_str);
        return true;
    }
    
    // Compare serialized config
    if let Ok(new_config) = serde_json::to_string(&new.node_type) {
        if existing.config != new_config {
            tracing::debug!("Node needs update - config changed: node_id={}, config_differs=true", existing.id);
            return true;
        }
    } else {
        tracing::warn!("Failed to serialize new node config for comparison: node_id={}", existing.id);
    }
    
    // Compare position
    let new_pos_x = new.position_x.unwrap_or(100.0);
    let new_pos_y = new.position_y.unwrap_or(100.0);
    
    if (existing.position_x - new_pos_x).abs() > f64::EPSILON 
        || (existing.position_y - new_pos_y).abs() > f64::EPSILON {
        tracing::debug!("Node needs update - position changed: node_id={}, old_pos=({}, {}), new_pos=({}, {})", 
                       existing.id, existing.position_x, existing.position_y, new_pos_x, new_pos_y);
        return true;
    }
    
    tracing::debug!("Node unchanged: node_id={}, name='{}'", existing.id, existing.name);
    false
}

/// Categorize edge changes for differential updates
fn categorize_edge_changes(
    existing_edges: &[edges::Model], 
    new_edges: &[EdgeRequest]
) -> EdgeOperations {
    let existing_set: HashSet<(String, String, Option<bool>)> = existing_edges
        .iter()
        .map(|e| (e.from_node_id.clone(), e.to_node_id.clone(), e.condition_result))
        .collect();
    
    let new_set: HashSet<(String, String, Option<bool>)> = new_edges
        .iter()
        .map(|e| (e.from_node_id.clone(), e.to_node_id.clone(), e.condition_result))
        .collect();
    
    // Find edges to create (in new but not in existing)
    let to_create: Vec<EdgeRequest> = new_edges
        .iter()
        .filter(|e| !existing_set.contains(&(e.from_node_id.clone(), e.to_node_id.clone(), e.condition_result)))
        .cloned()
        .collect();
    
    // Find edges to delete (in existing but not in new)
    let to_delete: Vec<String> = existing_edges
        .iter()
        .filter(|e| !new_set.contains(&(e.from_node_id.clone(), e.to_node_id.clone(), e.condition_result)))
        .map(|e| e.id.clone())
        .collect();
    
    EdgeOperations {
        to_create,
        to_delete,
    }
}

/// Validate that a string is a valid UUID
fn is_valid_uuid(id: &str) -> bool {
    Uuid::parse_str(id).is_ok()
}

/// Validate workflow update request
fn validate_workflow_update_request(
    request: &CreateWorkflowRequest, 
    start_node_id: &str,
    existing_nodes: &[nodes::Model]
) -> Result<(), String> {
    tracing::debug!(
        "Starting workflow update validation: workflow_name='{}', request_nodes={}, request_edges={}, existing_nodes={}", 
        request.name, request.nodes.len(), request.edges.len(), existing_nodes.len()
    );
    // Collect ALL valid node IDs (existing + new + updated)
    let mut all_valid_node_ids = HashSet::new();
    
    // Include ALL existing nodes (they will remain valid even if not in update)
    for existing_node in existing_nodes {
        all_valid_node_ids.insert(existing_node.id.clone());
    }
    
    // Track which nodes are being updated/created in this request
    let mut request_node_ids = HashSet::new();
    
    // Validate request nodes and collect their IDs
    for node in &request.nodes {
        // Validate node ID format if provided
        if let Some(node_id) = &node.id {
            if !is_valid_uuid(node_id) {
                return Err(format!("Invalid UUID format for node ID: {node_id}"));
            }
            all_valid_node_ids.insert(node_id.clone());
            request_node_ids.insert(node_id.clone());
        }
        
        // Validate node name is not empty
        if node.name.trim().is_empty() {
            return Err(format!("Node name cannot be empty (node_id: {:?})", node.id));
        }
        
        // Validate node name length (reasonable limit)
        if node.name.len() > 255 {
            return Err(format!("Node name too long: '{}' ({} characters, max 255, node_id: {:?})", 
                              node.name, node.name.len(), node.id));
        }
    }
    
    // Validate no cycles in the workflow
    if let Err(cycle_error) = detect_cycles(&request.edges) {
        return Err(format!("Workflow contains cycles: {cycle_error}"));
    }
    
    // Validate edges against the complete set of valid nodes
    for edge in &request.edges {
        // Validate edge node ID formats
        if !is_valid_uuid(&edge.from_node_id) {
            return Err(format!("Invalid UUID format for edge from_node_id: {}", edge.from_node_id));
        }
        if !is_valid_uuid(&edge.to_node_id) {
            return Err(format!("Invalid UUID format for edge to_node_id: {}", edge.to_node_id));
        }
        
        // Validate that referenced nodes exist (existing OR being created/updated)
        if !all_valid_node_ids.contains(&edge.from_node_id) {
            return Err(format!("Edge references non-existent from_node_id: {}", edge.from_node_id));
        }
        if !all_valid_node_ids.contains(&edge.to_node_id) {
            return Err(format!("Edge references non-existent to_node_id: {}", edge.to_node_id));
        }
        
        // Validate no self-loops
        if edge.from_node_id == edge.to_node_id {
            return Err(format!("Self-loop detected: node {} connects to itself", edge.from_node_id));
        }
        
        // Additional validation: Check for edges referencing nodes that will be deleted
        // A node will be deleted if it exists but is not in the request and is not the start node
        let from_will_be_deleted = existing_nodes.iter()
            .any(|n| n.id == edge.from_node_id && n.id != start_node_id && !request_node_ids.contains(&n.id));
        let to_will_be_deleted = existing_nodes.iter()
            .any(|n| n.id == edge.to_node_id && n.id != start_node_id && !request_node_ids.contains(&n.id));
            
        if from_will_be_deleted {
            return Err(format!(
                "Edge references from_node_id {} which will be deleted in this update", 
                edge.from_node_id
            ));
        }
        if to_will_be_deleted {
            return Err(format!(
                "Edge references to_node_id {} which will be deleted in this update", 
                edge.to_node_id
            ));
        }
    }
    
    // Validate workflow name
    if request.name.trim().is_empty() {
        return Err("Workflow name cannot be empty".to_string());
    }
    
    if request.name.len() > 255 {
        return Err(format!("Workflow name too long: '{}' ({} characters, max 255)", 
                          request.name, request.name.len()));
    }
    
    tracing::debug!(
        "Workflow update validation completed successfully: workflow_name='{}', total_valid_nodes={}, request_edges={}", 
        request.name, all_valid_node_ids.len(), request.edges.len()
    );
    
    Ok(())
}

/// Detect cycles in workflow using DFS
fn detect_cycles(edges: &[EdgeRequest]) -> Result<(), String> {
    // Build adjacency list
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();
    let mut all_nodes: HashSet<String> = HashSet::new();
    
    for edge in edges {
        graph.entry(edge.from_node_id.clone())
            .or_default()
            .push(edge.to_node_id.clone());
        all_nodes.insert(edge.from_node_id.clone());
        all_nodes.insert(edge.to_node_id.clone());
    }
    
    let mut visiting: HashSet<String> = HashSet::new();
    let mut visited: HashSet<String> = HashSet::new();
    
    fn dfs(
        node: &str,
        graph: &HashMap<String, Vec<String>>,
        visiting: &mut HashSet<String>,
        visited: &mut HashSet<String>,
    ) -> Result<(), String> {
        if visiting.contains(node) {
            return Err(format!("Cycle detected involving node: {node}"));
        }
        
        if visited.contains(node) {
            return Ok(());
        }
        
        visiting.insert(node.to_string());
        
        if let Some(neighbors) = graph.get(node) {
            for neighbor in neighbors {
                dfs(neighbor, graph, visiting, visited)?;
            }
        }
        
        visiting.remove(node);
        visited.insert(node.to_string());
        Ok(())
    }
    
    // Check each node for cycles
    for node in &all_nodes {
        if !visited.contains(node) {
            dfs(node, &graph, &mut visiting, &mut visited)?;
        }
    }
    
    Ok(())
}

/// Convert NodeType enum to string representation
fn node_type_to_string(node_type: &NodeType) -> String {
    match node_type {
        NodeType::Trigger { .. } => "trigger".to_string(),
        NodeType::Condition { .. } => "condition".to_string(),
        NodeType::Transformer { .. } => "transformer".to_string(),
        NodeType::HttpRequest { .. } => "http_request".to_string(),
        NodeType::OpenObserve { .. } => "openobserve".to_string(),
        NodeType::Email { .. } => "email".to_string(),
        NodeType::Delay { .. } => "delay".to_string(),
    }
}

/// Check if there are any active executions for the workflow
async fn has_active_executions(db: &sea_orm::DatabaseConnection, workflow_id: &str) -> Result<bool, sea_orm::DbErr> {
    let count = workflow_executions::Entity::find()
        .filter(workflow_executions::Column::WorkflowId.eq(workflow_id))
        .filter(workflow_executions::Column::Status.is_in(["running", "pending"]))
        .count(db)
        .await?;
    
    let has_active = count > 0;
    tracing::debug!(
        "Active executions check: workflow_id={}, active_count={}, has_active={}", 
        workflow_id, count, has_active
    );
    
    Ok(has_active)
}

pub async fn update_workflow(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(request): Json<CreateWorkflowRequest>,
) -> std::result::Result<Json<WorkflowResponse>, StatusCode> {
    let update_start = std::time::Instant::now();
    tracing::info!(
        "Workflow update initiated: workflow_id={}, nodes_count={}, edges_count={}",
        id, request.nodes.len(), request.edges.len()
    );
    
    let workflow = entities::Entity::find_by_id(&id)
        .one(&*state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch workflow for update: workflow_id={}, error={:?}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or_else(|| {
            tracing::warn!("Workflow not found for update: workflow_id={}", id);
            StatusCode::NOT_FOUND
        })?;
        
    tracing::info!(
        "Workflow update: found existing workflow '{}' (description: {:?})", 
        workflow.name, workflow.description
    );

    let existing_start_node_id = workflow.start_node_id.clone()
        .ok_or_else(|| {
            tracing::error!("Workflow {} has no start_node_id", id);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Get existing nodes for validation and processing
    tracing::debug!("Workflow update: fetching existing nodes for workflow_id={}", id);
    let existing_nodes = nodes::Entity::find()
        .filter(nodes::Column::WorkflowId.eq(&id))
        .all(&*state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch existing nodes for workflow update: workflow_id={}, error={:?}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Validate input request (now that we have existing nodes)
    tracing::info!("Workflow update: validating request for workflow_id={}", id);
    if let Err(validation_error) = validate_workflow_update_request(&request, &existing_start_node_id, &existing_nodes) {
        tracing::warn!(
            "Workflow update validation failed: workflow_id={}, nodes_count={}, edges_count={}, error='{}'", 
            id, request.nodes.len(), request.edges.len(), validation_error
        );
        return Err(StatusCode::BAD_REQUEST);
    }
    tracing::info!("Workflow update: validation passed for workflow_id={}", id);

    // Get existing start node to preserve it
    let existing_start_node = existing_nodes.iter()
        .find(|n| n.id == existing_start_node_id)
        .ok_or_else(|| {
            tracing::error!("Start node {} not found for workflow {}", existing_start_node_id, id);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Convert existing start node to internal model
    let start_node_config: NodeType = serde_json::from_str(&existing_start_node.config)
        .map_err(|e| {
            tracing::error!(
                "Failed to parse start node config: workflow_id={}, start_node_id={}, config='{}', error={:?}", 
                id, existing_start_node_id, existing_start_node.config, e
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    let start_node = Node {
        id: existing_start_node_id.clone(),
        workflow_id: id.clone(),
        name: existing_start_node.name.clone(),
        node_type: start_node_config,
        input_merge_strategy: None,
    };

    // Convert request nodes to internal models and add start node
    let mut nodes: Vec<Node> = vec![start_node]; // Start with the preserved start node
    nodes.extend(request.nodes.iter().map(|n| Node {
        id: n.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string()),
        workflow_id: id.clone(),
        name: n.name.clone(),
        node_type: n.node_type.clone(),
        input_merge_strategy: None,
    }));
    
    let edges: Vec<Edge> = request.edges.iter().map(|e| Edge {
        id: Uuid::new_v4().to_string(),
        workflow_id: id.clone(),
        from_node_id: e.from_node_id.clone(),
        to_node_id: e.to_node_id.clone(),
        condition_result: e.condition_result,
    }).collect();

    // Validate workflow structure
    if let Err(validation_error) = WorkflowValidator::validate_workflow(
        &request.name,
        &existing_start_node_id,
        &nodes,
        &edges,
    ) {
        tracing::warn!(
            "Workflow structure validation failed: workflow_id={}, workflow_name='{}', error='{}'", 
            id, request.name, validation_error
        );
        return Err(StatusCode::BAD_REQUEST);
    }

    // Check for warnings and log them
    let warnings = WorkflowValidator::validate_condition_completeness(&nodes, &edges);
    for warning in warnings {
        tracing::warn!("Workflow update warning: workflow_id={}, warning='{}'", id, warning);
    }

    let mut workflow: entities::ActiveModel = workflow.into();
    workflow.name = Set(request.name);
    workflow.description = Set(request.description);
    // Keep existing start_node_id - don't update it

    let workflow = workflow
        .update(&*state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update workflow metadata: workflow_id={}, error={:?}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Check for active executions before making destructive changes
    tracing::info!("Workflow update: checking for active executions for workflow_id={}", id);
    if has_active_executions(&state.db, &id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to check for active executions: workflow_id={}, error={:?}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
    {
        tracing::warn!(
            "Cannot update workflow - has active executions: workflow_id={}, workflow_name='{}'", 
            id, workflow.name
        );
        return Err(StatusCode::CONFLICT);
    }
    tracing::info!("Workflow update: no active executions found, proceeding with update for workflow_id={}", id);

    // Get existing edges for comparison (nodes already fetched above)
    tracing::debug!("Workflow update: fetching existing edges for workflow_id={}", id);
    let existing_edges = edges::Entity::find()
        .filter(edges::Column::WorkflowId.eq(&id))
        .all(&*state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch existing edges for workflow update: workflow_id={}, error={:?}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Categorize changes
    let node_ops = categorize_node_changes(&existing_nodes, &request.nodes, &existing_start_node_id);
    let edge_ops = categorize_edge_changes(&existing_edges, &request.edges);

    tracing::info!(
        "Workflow {} differential update: nodes(create={}, update={}, delete={}), edges(create={}, delete={})",
        id, node_ops.to_create.len(), node_ops.to_update.len(), node_ops.to_delete.len(),
        edge_ops.to_create.len(), edge_ops.to_delete.len()
    );

    // Start transaction for atomic updates
    tracing::info!("Workflow update: starting database transaction for workflow_id={}", id);
    let txn = state.db.begin().await.map_err(|e| {
        tracing::error!("Failed to start transaction: workflow_id={}, error={:?}", id, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // 1. Update existing nodes
    tracing::info!("Workflow update: updating {} existing nodes for workflow_id={}", node_ops.to_update.len(), id);
    for (node_id, node_data, existing_node) in &node_ops.to_update {
        tracing::debug!("Updating node: workflow_id={}, node_id={}, name='{}'", id, node_id, node_data.name);
        
        let node_config = serde_json::to_string(&node_data.node_type)
            .map_err(|e| {
                tracing::error!("Failed to serialize node config: workflow_id={}, node_id={}, error={:?}", id, node_id, e);
                StatusCode::BAD_REQUEST
            })?;

        let position_x = node_data.position_x.unwrap_or(100.0);
        let position_y = node_data.position_y.unwrap_or(100.0);

        let node_type_str = node_type_to_string(&node_data.node_type);

        // Update existing node using the reference we already have
        let mut node_model: nodes::ActiveModel = (*existing_node).clone().into();
        node_model.name = Set(node_data.name.clone());
        node_model.node_type = Set(node_type_str);
        node_model.config = Set(node_config);
        node_model.position_x = Set(position_x);
        node_model.position_y = Set(position_y);

        node_model.update(&txn).await.map_err(|e| {
            tracing::error!("Failed to update node: workflow_id={}, node_id={}, error={:?}", id, node_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        
        tracing::debug!("Successfully updated node: workflow_id={}, node_id={}", id, node_id);
    }

    // 2. Create new nodes
    tracing::info!("Workflow update: creating {} new nodes for workflow_id={}", node_ops.to_create.len(), id);
    for node_data in &node_ops.to_create {
        let node_id = node_data.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
        tracing::debug!("Creating node: workflow_id={}, node_id={}, name='{}'", id, node_id, node_data.name);
        
        let node_config = serde_json::to_string(&node_data.node_type)
            .map_err(|e| {
                tracing::error!("Failed to serialize node config for creation: workflow_id={}, node_id={}, error={:?}", id, node_id, e);
                StatusCode::BAD_REQUEST
            })?;

        let position_x = node_data.position_x.unwrap_or(100.0);
        let position_y = node_data.position_y.unwrap_or(100.0);

        let node_type_str = node_type_to_string(&node_data.node_type);

        let node_model = nodes::ActiveModel {
            id: Set(node_id.clone()),
            workflow_id: Set(id.clone()),
            name: Set(node_data.name.clone()),
            node_type: Set(node_type_str),
            config: Set(node_config),
            position_x: Set(position_x),
            position_y: Set(position_y),
            ..Default::default()
        };

        node_model.insert(&txn).await.map_err(|e| {
            tracing::error!("Failed to create node: workflow_id={}, node_id={}, error={:?}", id, node_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        
        tracing::debug!("Successfully created node: workflow_id={}, node_id={}", id, node_id);
    }

    // 3. Delete edges first (before nodes to avoid FK violations)
    if !edge_ops.to_delete.is_empty() {
        tracing::info!("Workflow update: deleting {} edges for workflow_id={}", edge_ops.to_delete.len(), id);
        tracing::debug!("Deleting edges: workflow_id={}, edge_ids={:?}", id, edge_ops.to_delete);
        
        edges::Entity::delete_many()
            .filter(edges::Column::Id.is_in(&edge_ops.to_delete))
            .exec(&txn)
            .await
            .map_err(|e| {
                tracing::error!("Failed to delete edges: workflow_id={}, error={:?}", id, e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
            
        tracing::info!("Workflow update: successfully deleted {} edges for workflow_id={}", edge_ops.to_delete.len(), id);
    }

    // 4. Create new edges 
    tracing::info!("Workflow update: creating {} new edges for workflow_id={}", edge_ops.to_create.len(), id);
    for edge_data in &edge_ops.to_create {
        let edge_id = Uuid::new_v4().to_string();
        tracing::debug!("Creating edge: workflow_id={}, edge_id={}, from_node={}, to_node={}", 
                       id, edge_id, edge_data.from_node_id, edge_data.to_node_id);
        
        let edge_model = edges::ActiveModel {
            id: Set(edge_id.clone()),
            workflow_id: Set(id.clone()),
            from_node_id: Set(edge_data.from_node_id.clone()),
            to_node_id: Set(edge_data.to_node_id.clone()),
            condition_result: Set(edge_data.condition_result),
            ..Default::default()
        };

        edge_model.insert(&txn).await.map_err(|e| {
            tracing::error!("Failed to create edge: workflow_id={}, edge_id={}, error={:?}", id, edge_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        
        tracing::debug!("Successfully created edge: workflow_id={}, edge_id={}", id, edge_id);
    }

    // 5. Delete unused nodes (after edges to avoid FK violations)
    if !node_ops.to_delete.is_empty() {
        tracing::info!("Workflow update: deleting {} unused nodes for workflow_id={}", node_ops.to_delete.len(), id);
        tracing::debug!("Deleting nodes: workflow_id={}, node_ids={:?}", id, node_ops.to_delete);
        
        nodes::Entity::delete_many()
            .filter(nodes::Column::Id.is_in(&node_ops.to_delete))
            .exec(&txn)
            .await
            .map_err(|e| {
                tracing::error!("Failed to delete nodes: workflow_id={}, error={:?}", id, e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
            
        tracing::info!("Workflow update: successfully deleted {} nodes for workflow_id={}", node_ops.to_delete.len(), id);
    }

    // Commit transaction
    tracing::info!("Workflow update: committing transaction for workflow_id={}", id);
    let commit_start = std::time::Instant::now();
    txn.commit().await.map_err(|e| {
        tracing::error!("Failed to commit transaction: workflow_id={}, error={:?}", id, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let commit_duration = commit_start.elapsed();
    tracing::info!("Workflow update: transaction committed successfully for workflow_id={}, duration={:?}", id, commit_duration);

    // Fetch nodes
    tracing::debug!("Workflow update: fetching updated nodes for response for workflow_id={}", id);
    let nodes = nodes::Entity::find()
        .filter(nodes::Column::WorkflowId.eq(&id))
        .all(&*state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch updated nodes: workflow_id={}, error={:?}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Fetch edges
    tracing::debug!("Workflow update: fetching updated edges for response for workflow_id={}", id);
    let edges = edges::Entity::find()
        .filter(edges::Column::WorkflowId.eq(&id))
        .all(&*state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch updated edges: workflow_id={}, error={:?}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Convert nodes to response format
    let node_responses: Vec<NodeResponse> = nodes
        .into_iter()
        .map(|node| {
            let node_type: NodeType = serde_json::from_str(&node.config)
                .unwrap_or(NodeType::HttpRequest {
                    url: "".to_string(),
                    method: HttpMethod::Get,
                    timeout_seconds: 30,
                    failure_action: FailureAction::Stop,
                    retry_config: RetryConfig::default(),
                    headers: std::collections::HashMap::new(),
                });
            NodeResponse {
                id: node.id,
                name: node.name,
                node_type,
                position_x: node.position_x,
                position_y: node.position_y,
            }
        })
        .collect();

    // Convert edges to response format
    let edge_responses: Vec<EdgeResponse> = edges
        .into_iter()
        .map(|edge| EdgeResponse {
            id: edge.id.clone(),
            from_node_id: edge.from_node_id.clone(),
            to_node_id: edge.to_node_id.clone(),
            condition_result: edge.condition_result,
        })
        .collect();

    let response = WorkflowResponse {
        id: workflow.id.clone(),
        name: workflow.name,
        description: workflow.description,
        start_node_id: existing_start_node_id.clone(),
        endpoint_url: format!("/api/v1/{}/trigger", workflow.id),
        created_at: workflow.created_at,
        updated_at: workflow.updated_at,
        nodes: node_responses,
        edges: edge_responses,
    };

    // Invalidate cache since workflow was updated, then cache new version
    tracing::debug!("Workflow update: invalidating cache for workflow_id={}", id);
    state.workflow_cache.invalidate(&id).await;
    state.workflow_cache.put(id.clone(), existing_start_node_id.clone()).await;

    let total_duration = update_start.elapsed();
    
    tracing::info!(
        "Workflow update completed successfully: workflow_id={}, total_nodes={}, total_edges={}, total_duration={:?}",
        id, response.nodes.len(), response.edges.len(), total_duration
    );

    Ok(Json(response))
}

pub async fn delete_workflow(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> std::result::Result<StatusCode, StatusCode> {
    tracing::info!("Workflow deletion initiated: workflow_id={}", id);
    
    let result = entities::Entity::delete_by_id(&id)
        .exec(&*state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete workflow: workflow_id={}, error={:?}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if result.rows_affected == 0 {
        tracing::warn!("Workflow not found for deletion: workflow_id={}", id);
        return Err(StatusCode::NOT_FOUND);
    }

    // Invalidate cache for deleted workflow
    tracing::debug!("Invalidating cache for deleted workflow: workflow_id={}", id);
    state.workflow_cache.invalidate(&id).await;

    tracing::info!("Workflow deleted successfully: workflow_id={}, rows_affected={}", id, result.rows_affected);
    Ok(StatusCode::NO_CONTENT)
}