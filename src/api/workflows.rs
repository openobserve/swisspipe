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
        start_node_id: Set(Some(start_node_id.clone())),
        ..Default::default()
    };

    let workflow = workflow_model
        .insert(&*state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Create all nodes (start node + user nodes)
    for node in &nodes {
        let node_config = serde_json::to_string(&node.node_type)
            .map_err(|_| StatusCode::BAD_REQUEST)?;

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
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
        start_node_id: workflow.start_node_id.clone().unwrap_or_default(),
        endpoint_url: format!("/api/v1/{}/trigger", workflow.id),
        created_at: workflow.created_at,
        updated_at: workflow.updated_at,
        nodes: node_responses,
        edges: edge_responses,
    };

    // Update cache with current workflow metadata
    state.workflow_cache.put(id.clone(), workflow.start_node_id.clone().unwrap_or_default()).await;

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

    let existing_start_node_id = workflow.start_node_id.clone()
        .ok_or_else(|| {
            tracing::error!("Workflow {} has no start_node_id", id);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Get existing start node to preserve it
    let existing_start_node = nodes::Entity::find_by_id(&existing_start_node_id)
        .one(&*state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or_else(|| {
            tracing::error!("Start node {} not found for workflow {}", existing_start_node_id, id);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Convert existing start node to internal model
    let start_node_config: NodeType = serde_json::from_str(&existing_start_node.config)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
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
    // Keep existing start_node_id - don't update it

    let workflow = workflow
        .update(&*state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Delete user nodes and edges (preserve start node)
    nodes::Entity::delete_many()
        .filter(nodes::Column::WorkflowId.eq(&id))
        .filter(nodes::Column::Id.ne(&existing_start_node_id))
        .exec(&*state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    edges::Entity::delete_many()
        .filter(edges::Column::WorkflowId.eq(&id))
        .exec(&*state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Create all nodes (start node is preserved, so we only create user nodes)
    for node in &nodes {
        if node.id == existing_start_node_id {
            // Skip the start node - it's already preserved
            continue;
        }

        let node_config = serde_json::to_string(&node.node_type)
            .map_err(|_| StatusCode::BAD_REQUEST)?;

        // Find position from request
        let user_node = request.nodes.iter().find(|n| 
            n.id.as_ref().unwrap_or(&String::new()) == &node.id || n.name == node.name
        );
        let (position_x, position_y) = (
            user_node.and_then(|n| n.position_x).unwrap_or(100.0),
            user_node.and_then(|n| n.position_y).unwrap_or(100.0)
        );

        let node_model = nodes::ActiveModel {
            id: Set(node.id.clone()),
            workflow_id: Set(id.clone()),
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
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    // Create edges
    for edge_req in request.edges {
        let edge_model = edges::ActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            workflow_id: Set(id.clone()),
            from_node_id: Set(edge_req.from_node_id.clone()),
            to_node_id: Set(edge_req.to_node_id.clone()),
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
    state.workflow_cache.invalidate(&id).await;
    state.workflow_cache.put(id.clone(), existing_start_node_id.clone()).await;

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