#[allow(unused_imports)]
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use sea_orm::{ActiveModelTrait, EntityTrait, Set, ColumnTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    database::{edges, entities, nodes},
    workflow::{
        models::{Edge, Node, NodeType, HttpMethod, RetryConfig, FailureAction},
        validation::WorkflowValidator,
    },
    AppState,
};

#[derive(Deserialize)]
pub struct CreateWorkflowRequest {
    pub name: String,
    pub description: Option<String>,
    pub start_node_name: String, // Deprecated: use start_node_id
    pub start_node_id: Option<String>, // New: node ID to start from
    pub nodes: Vec<NodeRequest>,
    pub edges: Vec<EdgeRequest>,
}

#[derive(Deserialize)]
pub struct NodeRequest {
    pub id: Option<String>, // For updates: existing node ID
    pub name: String,
    pub node_type: NodeType,
    pub position_x: Option<f64>,
    pub position_y: Option<f64>,
}

#[derive(Deserialize)]
pub struct EdgeRequest {
    pub from_node_name: String, // Deprecated: use from_node_id
    pub to_node_name: String,   // Deprecated: use to_node_id
    pub from_node_id: Option<String>, // New: source node ID
    pub to_node_id: Option<String>,   // New: target node ID
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
    pub from_node_name: String, // Deprecated: use from_node_id
    pub to_node_name: String,   // Deprecated: use to_node_id  
    pub from_node_id: String,   // New: source node ID
    pub to_node_id: String,     // New: target node ID
    pub condition_result: Option<bool>,
}

#[derive(Serialize)]
pub struct WorkflowResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub start_node_name: String, // Deprecated: use start_node_id
    pub start_node_id: String,   // New: starting node ID
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

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_workflows).post(create_workflow))
        .route("/:id", get(get_workflow).put(update_workflow).delete(delete_workflow))
}

pub async fn list_workflows(
    State(state): State<AppState>,
) -> std::result::Result<Json<WorkflowListResponse>, StatusCode> {
    let workflows = entities::Entity::find()
        .all(&*state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let workflow_responses: Vec<WorkflowResponse> = workflows
        .into_iter()
        .map(|w| WorkflowResponse {
            endpoint_url: format!("/api/v1/{}/trigger", w.id),
            id: w.id,
            name: w.name,
            description: w.description,
            start_node_name: w.start_node_name.clone(),
            start_node_id: w.start_node_id.unwrap_or_else(|| w.start_node_name.clone()),
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

    // Convert request to internal models for validation
    let nodes: Vec<Node> = request.nodes.iter().map(|n| Node {
        id: workflow_id.clone(),
        workflow_id: workflow_id.clone(),
        name: n.name.clone(),
        node_type: n.node_type.clone(),
        input_merge_strategy: None,
    }).collect();
    
    let edges: Vec<Edge> = request.edges.iter().map(|e| Edge {
        id: workflow_id.clone(),
        workflow_id: workflow_id.clone(),
        from_node_name: e.from_node_name.clone(),
        to_node_name: e.to_node_name.clone(),
        from_node_id: e.from_node_id.clone(),
        to_node_id: e.to_node_id.clone(),
        condition_result: e.condition_result,
    }).collect();

    // Validate workflow structure
    if let Err(validation_error) = WorkflowValidator::validate_workflow(
        &request.name,
        &request.start_node_name,
        &nodes,
        &edges,
    ) {
        tracing::warn!("Workflow validation failed: {}", validation_error);
        return Err(StatusCode::BAD_REQUEST);
    }

    // Check for warnings and log them
    let warnings = WorkflowValidator::validate_condition_completeness(&nodes, &edges);
    for warning in warnings {
        tracing::warn!("Workflow warning: {}", warning);
    }

    // Create workflow
    let workflow_model = entities::ActiveModel {
        id: Set(workflow_id.clone()),
        name: Set(request.name.clone()),
        description: Set(request.description.clone()),
        start_node_name: Set(request.start_node_name.clone()),
        start_node_id: Set(request.start_node_id.clone()), // Will be updated later if needed
        ..Default::default()
    };

    let workflow = workflow_model
        .insert(&*state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Create nodes and track ID mappings
    let mut node_id_to_name_mapping = std::collections::HashMap::new();
    for node_req in request.nodes {
        let node_config = serde_json::to_string(&node_req.node_type)
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        
        let node_id = node_req.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
        node_id_to_name_mapping.insert(node_req.name.clone(), node_id.clone());

        let node_model = nodes::ActiveModel {
            id: Set(node_id),
            workflow_id: Set(workflow_id.clone()),
            name: Set(node_req.name.clone()),
            node_type: Set(match node_req.node_type {
                NodeType::Trigger { .. } => "trigger".to_string(),
                NodeType::Condition { .. } => "condition".to_string(),
                NodeType::Transformer { .. } => "transformer".to_string(),
                NodeType::Webhook { .. } => "webhook".to_string(),
                NodeType::OpenObserve { .. } => "openobserve".to_string(),
                NodeType::Email { .. } => "email".to_string(),
                NodeType::Delay { .. } => "delay".to_string(),
            }),
            config: Set(node_config),
            position_x: Set(node_req.position_x.unwrap_or(100.0)),
            position_y: Set(node_req.position_y.unwrap_or(100.0)),
            ..Default::default()
        };

        node_model
            .insert(&*state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    
    // Update workflow start_node_id if it wasn't provided but we can resolve it
    if request.start_node_id.is_none() {
        if let Some(start_node_id) = node_id_to_name_mapping.get(&request.start_node_name) {
            let mut workflow_update: entities::ActiveModel = workflow.clone().into();
            workflow_update.start_node_id = Set(Some(start_node_id.clone()));
            workflow_update.update(&*state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
    }

    // Create edges with node ID resolution
    for edge_req in request.edges {
        // Determine source and target node IDs
        let from_node_id = if let Some(id) = &edge_req.from_node_id {
            id.clone()
        } else if let Some(id) = node_id_to_name_mapping.get(&edge_req.from_node_name) {
            id.clone()
        } else {
            "unknown".to_string() // Will be handled by validation
        };
        
        let to_node_id = if let Some(id) = &edge_req.to_node_id {
            id.clone()
        } else if let Some(id) = node_id_to_name_mapping.get(&edge_req.to_node_name) {
            id.clone()
        } else {
            "unknown".to_string() // Will be handled by validation
        };
        
        let edge_model = edges::ActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            workflow_id: Set(workflow_id.clone()),
            from_node_name: Set(edge_req.from_node_name),
            to_node_name: Set(edge_req.to_node_name),
            from_node_id: Set(Some(from_node_id)),
            to_node_id: Set(Some(to_node_id)),
            condition_result: Set(edge_req.condition_result),
            ..Default::default()
        };

        edge_model
            .insert(&*state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    // Fetch nodes
    let nodes = nodes::Entity::find()
        .filter(nodes::Column::WorkflowId.eq(&workflow_id))
        .all(&*state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Fetch edges
    let edges = edges::Entity::find()
        .filter(edges::Column::WorkflowId.eq(&workflow_id))
        .all(&*state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Convert nodes to response format
    let node_responses: Vec<NodeResponse> = nodes
        .into_iter()
        .map(|node| {
            let node_type: NodeType = serde_json::from_str(&node.config)
                .unwrap_or(NodeType::Webhook {
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
            from_node_name: edge.from_node_name.clone(),
            to_node_name: edge.to_node_name.clone(),
            from_node_id: edge.from_node_id.clone().unwrap_or_else(|| edge.from_node_name.clone()),
            to_node_id: edge.to_node_id.clone().unwrap_or_else(|| edge.to_node_name.clone()),
            condition_result: edge.condition_result,
        })
        .collect();

    let response = WorkflowResponse {
        id: workflow.id.clone(),
        name: workflow.name,
        description: workflow.description,
        start_node_name: workflow.start_node_name.clone(),
        start_node_id: workflow.start_node_id.clone().unwrap_or_else(|| workflow.start_node_name.clone()),
        endpoint_url: format!("/api/v1/{}/trigger", workflow.id),
        created_at: workflow.created_at,
        updated_at: workflow.updated_at,
        nodes: node_responses,
        edges: edge_responses,
    };

    // Cache the newly created workflow metadata for performance
    state.workflow_cache.put(workflow_id.clone(), workflow.start_node_name).await;

    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn get_workflow(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> std::result::Result<Json<WorkflowResponse>, StatusCode> {
    let workflow = entities::Entity::find_by_id(&id)
        .one(&*state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Fetch nodes
    let nodes = nodes::Entity::find()
        .filter(nodes::Column::WorkflowId.eq(&id))
        .all(&*state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Fetch edges
    let edges = edges::Entity::find()
        .filter(edges::Column::WorkflowId.eq(&id))
        .all(&*state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Convert nodes to response format
    let node_responses: Vec<NodeResponse> = nodes
        .into_iter()
        .map(|node| {
            let node_type: NodeType = serde_json::from_str(&node.config)
                .unwrap_or(NodeType::Webhook {
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
            from_node_name: edge.from_node_name.clone(),
            to_node_name: edge.to_node_name.clone(),
            from_node_id: edge.from_node_id.clone().unwrap_or_else(|| edge.from_node_name.clone()),
            to_node_id: edge.to_node_id.clone().unwrap_or_else(|| edge.to_node_name.clone()),
            condition_result: edge.condition_result,
        })
        .collect();

    let response = WorkflowResponse {
        id: workflow.id.clone(),
        name: workflow.name,
        description: workflow.description,
        start_node_name: workflow.start_node_name.clone(),
        start_node_id: workflow.start_node_id.clone().unwrap_or_else(|| workflow.start_node_name.clone()),
        endpoint_url: format!("/api/v1/{}/trigger", workflow.id),
        created_at: workflow.created_at,
        updated_at: workflow.updated_at,
        nodes: node_responses,
        edges: edge_responses,
    };

    // Update cache with current workflow metadata
    state.workflow_cache.put(id.clone(), workflow.start_node_name).await;

    Ok(Json(response))
}

pub async fn update_workflow(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(request): Json<CreateWorkflowRequest>,
) -> std::result::Result<Json<WorkflowResponse>, StatusCode> {
    let workflow = entities::Entity::find_by_id(&id)
        .one(&*state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Convert request to internal models for validation
    let nodes: Vec<Node> = request.nodes.iter().map(|n| Node {
        id: id.clone(),
        workflow_id: id.clone(),
        name: n.name.clone(),
        node_type: n.node_type.clone(),
        input_merge_strategy: None,
    }).collect();
    
    let edges: Vec<Edge> = request.edges.iter().map(|e| Edge {
        id: id.clone(),
        workflow_id: id.clone(),
        from_node_name: e.from_node_name.clone(),
        to_node_name: e.to_node_name.clone(),
        from_node_id: e.from_node_id.clone(),
        to_node_id: e.to_node_id.clone(),
        condition_result: e.condition_result,
    }).collect();

    // Validate workflow structure
    if let Err(validation_error) = WorkflowValidator::validate_workflow(
        &request.name,
        &request.start_node_name,
        &nodes,
        &edges,
    ) {
        tracing::warn!("Workflow validation failed: {}", validation_error);
        return Err(StatusCode::BAD_REQUEST);
    }

    // Check for warnings and log them
    let warnings = WorkflowValidator::validate_condition_completeness(&nodes, &edges);
    for warning in warnings {
        tracing::warn!("Workflow warning: {}", warning);
    }

    let mut workflow: entities::ActiveModel = workflow.into();
    workflow.name = Set(request.name);
    workflow.description = Set(request.description);
    workflow.start_node_name = Set(request.start_node_name.clone());
    
    // Use start_node_id if provided, otherwise try to resolve from start_node_name later
    if let Some(start_node_id) = &request.start_node_id {
        workflow.start_node_id = Set(Some(start_node_id.clone()));
    }

    let workflow = workflow
        .update(&*state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get existing nodes for comparison
    let existing_nodes = nodes::Entity::find()
        .filter(nodes::Column::WorkflowId.eq(&id))
        .all(&*state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    

    // Simplified node processing: update if ID exists, create if no ID, then delete nodes not in request
    let mut node_id_to_name_mapping = std::collections::HashMap::new();
    let mut request_node_ids = std::collections::HashSet::new();
    
    for node_req in request.nodes {
        let node_config = serde_json::to_string(&node_req.node_type)
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        
        let node_type_str = match node_req.node_type {
            NodeType::Trigger { .. } => "trigger".to_string(),
            NodeType::Condition { .. } => "condition".to_string(),
            NodeType::Transformer { .. } => "transformer".to_string(),
            NodeType::Webhook { .. } => "webhook".to_string(),
            NodeType::OpenObserve { .. } => "openobserve".to_string(),
            NodeType::Email { .. } => "email".to_string(),
            NodeType::Delay { .. } => "delay".to_string(),
        };
        
        if let Some(node_id) = &node_req.id {
            // Node has ID -> update existing node
            request_node_ids.insert(node_id.clone());
            node_id_to_name_mapping.insert(node_req.name.clone(), node_id.clone());
            
            // Find and update the existing node
            if let Some(existing_node) = existing_nodes.iter().find(|n| &n.id == node_id) {
                let mut node_model: nodes::ActiveModel = existing_node.clone().into();
                node_model.name = Set(node_req.name.clone());
                node_model.node_type = Set(node_type_str);
                node_model.config = Set(node_config);
                node_model.position_x = Set(node_req.position_x.unwrap_or(100.0));
                node_model.position_y = Set(node_req.position_y.unwrap_or(100.0));
                
                node_model
                    .update(&*state.db)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            } else {
                // Node ID not found, treat as new node but use the provided ID
                node_id_to_name_mapping.insert(node_req.name.clone(), node_id.clone());
                
                let node_model = nodes::ActiveModel {
                    id: Set(node_id.clone()),
                    workflow_id: Set(id.clone()),
                    name: Set(node_req.name.clone()),
                    node_type: Set(node_type_str),
                    config: Set(node_config),
                    position_x: Set(node_req.position_x.unwrap_or(100.0)),
                    position_y: Set(node_req.position_y.unwrap_or(100.0)),
                    ..Default::default()
                };

                node_model
                    .insert(&*state.db)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            }
        } else {
            // No ID -> create new node
            let new_node_id = Uuid::new_v4().to_string();
            request_node_ids.insert(new_node_id.clone());
            node_id_to_name_mapping.insert(node_req.name.clone(), new_node_id.clone());
            
            let node_model = nodes::ActiveModel {
                id: Set(new_node_id),
                workflow_id: Set(id.clone()),
                name: Set(node_req.name.clone()),
                node_type: Set(node_type_str),
                config: Set(node_config),
                position_x: Set(node_req.position_x.unwrap_or(100.0)),
                position_y: Set(node_req.position_y.unwrap_or(100.0)),
                ..Default::default()
            };

            node_model
                .insert(&*state.db)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
    }

    // Delete nodes not in the request (set difference)
    for existing_node in &existing_nodes {
        if !request_node_ids.contains(&existing_node.id) {
            nodes::Entity::delete_by_id(&existing_node.id)
                .exec(&*state.db)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
    }
    
    // Update workflow start_node_id if it wasn't provided but we can resolve it
    if request.start_node_id.is_none() {
        if let Some(start_node_id) = node_id_to_name_mapping.get(&request.start_node_name) {
            let mut workflow_update: entities::ActiveModel = workflow.clone().into();
            workflow_update.start_node_id = Set(Some(start_node_id.clone()));
            workflow_update.update(&*state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
    }

    // Simplified edge processing: delete all existing edges, then create new ones from request
    // Delete all existing edges for this workflow
    edges::Entity::delete_many()
        .filter(edges::Column::WorkflowId.eq(&id))
        .exec(&*state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Create all edges from request (require node IDs)
    for edge_req in request.edges {
        // Resolve node IDs with proper error handling
        let from_node_id = edge_req.from_node_id
            .or_else(|| node_id_to_name_mapping.get(&edge_req.from_node_name).cloned())
            .ok_or(StatusCode::BAD_REQUEST)?;
        
        let to_node_id = edge_req.to_node_id
            .or_else(|| node_id_to_name_mapping.get(&edge_req.to_node_name).cloned())
            .ok_or(StatusCode::BAD_REQUEST)?;
        
        let edge_model = edges::ActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            workflow_id: Set(id.clone()),
            from_node_name: Set(edge_req.from_node_name),
            to_node_name: Set(edge_req.to_node_name),
            from_node_id: Set(Some(from_node_id)),
            to_node_id: Set(Some(to_node_id)),
            condition_result: Set(edge_req.condition_result),
            ..Default::default()
        };

        edge_model
            .insert(&*state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    // Fetch nodes
    let nodes = nodes::Entity::find()
        .filter(nodes::Column::WorkflowId.eq(&id))
        .all(&*state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Fetch edges
    let edges = edges::Entity::find()
        .filter(edges::Column::WorkflowId.eq(&id))
        .all(&*state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Convert nodes to response format
    let node_responses: Vec<NodeResponse> = nodes
        .into_iter()
        .map(|node| {
            let node_type: NodeType = serde_json::from_str(&node.config)
                .unwrap_or(NodeType::Webhook {
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
            from_node_name: edge.from_node_name.clone(),
            to_node_name: edge.to_node_name.clone(),
            from_node_id: edge.from_node_id.clone().unwrap_or_else(|| edge.from_node_name.clone()),
            to_node_id: edge.to_node_id.clone().unwrap_or_else(|| edge.to_node_name.clone()),
            condition_result: edge.condition_result,
        })
        .collect();

    let response = WorkflowResponse {
        id: workflow.id.clone(),
        name: workflow.name,
        description: workflow.description,
        start_node_name: workflow.start_node_name.clone(),
        start_node_id: workflow.start_node_id.clone().unwrap_or_else(|| workflow.start_node_name.clone()),
        endpoint_url: format!("/api/v1/{}/trigger", workflow.id),
        created_at: workflow.created_at,
        updated_at: workflow.updated_at,
        nodes: node_responses,
        edges: edge_responses,
    };

    // Invalidate cache since workflow was updated, then cache new version
    state.workflow_cache.invalidate(&id).await;
    state.workflow_cache.put(id.clone(), workflow.start_node_name).await;

    Ok(Json(response))
}

pub async fn delete_workflow(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> std::result::Result<StatusCode, StatusCode> {
    let result = entities::Entity::delete_by_id(&id)
        .exec(&*state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    // Invalidate cache for deleted workflow
    state.workflow_cache.invalidate(&id).await;

    Ok(StatusCode::NO_CONTENT)
}