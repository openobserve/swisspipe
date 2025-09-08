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
        models::{Edge, Node, NodeType, AppType, HttpMethod, RetryConfig, FailureAction},
        validation::WorkflowValidator,
    },
    AppState,
};

#[derive(Deserialize)]
pub struct CreateWorkflowRequest {
    pub name: String,
    pub description: Option<String>,
    pub start_node_name: String,
    pub nodes: Vec<NodeRequest>,
    pub edges: Vec<EdgeRequest>,
}

#[derive(Deserialize)]
pub struct NodeRequest {
    pub name: String,
    pub node_type: NodeType,
    pub position_x: Option<f64>,
    pub position_y: Option<f64>,
}

#[derive(Deserialize)]
pub struct EdgeRequest {
    pub from_node_name: String,
    pub to_node_name: String,
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
    pub from_node_name: String,
    pub to_node_name: String,
    pub condition_result: Option<bool>,
}

#[derive(Serialize)]
pub struct WorkflowResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub start_node_name: String,
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
            endpoint_url: format!("/api/v1/{}/ep", w.id),
            id: w.id,
            name: w.name,
            description: w.description,
            start_node_name: w.start_node_name,
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
    }).collect();
    
    let edges: Vec<Edge> = request.edges.iter().map(|e| Edge {
        id: workflow_id.clone(),
        workflow_id: workflow_id.clone(),
        from_node_name: e.from_node_name.clone(),
        to_node_name: e.to_node_name.clone(),
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
        ..Default::default()
    };

    let workflow = workflow_model
        .insert(&*state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Create nodes
    for node_req in request.nodes {
        let node_config = serde_json::to_string(&node_req.node_type)
            .map_err(|_| StatusCode::BAD_REQUEST)?;

        let node_model = nodes::ActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            workflow_id: Set(workflow_id.clone()),
            name: Set(node_req.name),
            node_type: Set(match node_req.node_type {
                NodeType::Trigger { .. } => "trigger".to_string(),
                NodeType::Condition { .. } => "condition".to_string(),
                NodeType::Transformer { .. } => "transformer".to_string(),
                NodeType::App { .. } => "app".to_string(),
                NodeType::Email { .. } => "email".to_string(),
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

    // Create edges
    for edge_req in request.edges {
        let edge_model = edges::ActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            workflow_id: Set(workflow_id.clone()),
            from_node_name: Set(edge_req.from_node_name),
            to_node_name: Set(edge_req.to_node_name),
            condition_result: Set(edge_req.condition_result),
            ..Default::default()
        };

        edge_model
            .insert(&*state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    let response = WorkflowResponse {
        id: workflow.id.clone(),
        name: workflow.name,
        description: workflow.description,
        start_node_name: workflow.start_node_name,
        endpoint_url: format!("/api/v1/{}/ep", workflow.id),
        created_at: workflow.created_at,
        updated_at: workflow.updated_at,
        nodes: vec![], // Will be populated by subsequent GET request
        edges: vec![], // Will be populated by subsequent GET request
    };

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
                .unwrap_or(NodeType::App {
                    app_type: AppType::Webhook,
                    url: "".to_string(),
                    method: HttpMethod::GET,
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
            id: edge.id,
            from_node_name: edge.from_node_name,
            to_node_name: edge.to_node_name,
            condition_result: edge.condition_result,
        })
        .collect();

    let response = WorkflowResponse {
        id: workflow.id.clone(),
        name: workflow.name,
        description: workflow.description,
        start_node_name: workflow.start_node_name,
        endpoint_url: format!("/api/v1/{}/ep", workflow.id),
        created_at: workflow.created_at,
        updated_at: workflow.updated_at,
        nodes: node_responses,
        edges: edge_responses,
    };

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
    }).collect();
    
    let edges: Vec<Edge> = request.edges.iter().map(|e| Edge {
        id: id.clone(),
        workflow_id: id.clone(),
        from_node_name: e.from_node_name.clone(),
        to_node_name: e.to_node_name.clone(),
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
    workflow.start_node_name = Set(request.start_node_name);

    let workflow = workflow
        .update(&*state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Delete existing nodes and edges
    nodes::Entity::delete_many()
        .filter(nodes::Column::WorkflowId.eq(&id))
        .exec(&*state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    edges::Entity::delete_many()
        .filter(edges::Column::WorkflowId.eq(&id))
        .exec(&*state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Create new nodes
    for node_req in request.nodes {
        let node_config = serde_json::to_string(&node_req.node_type)
            .map_err(|_| StatusCode::BAD_REQUEST)?;

        let node_model = nodes::ActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            workflow_id: Set(id.clone()),
            name: Set(node_req.name),
            node_type: Set(match node_req.node_type {
                NodeType::Trigger { .. } => "trigger".to_string(),
                NodeType::Condition { .. } => "condition".to_string(),
                NodeType::Transformer { .. } => "transformer".to_string(),
                NodeType::App { .. } => "app".to_string(),
                NodeType::Email { .. } => "email".to_string(),
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

    // Create new edges
    for edge_req in request.edges {
        let edge_model = edges::ActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            workflow_id: Set(id.clone()),
            from_node_name: Set(edge_req.from_node_name),
            to_node_name: Set(edge_req.to_node_name),
            condition_result: Set(edge_req.condition_result),
            ..Default::default()
        };

        edge_model
            .insert(&*state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    let response = WorkflowResponse {
        id: workflow.id.clone(),
        name: workflow.name,
        description: workflow.description,
        start_node_name: workflow.start_node_name,
        endpoint_url: format!("/api/v1/{}/ep", workflow.id),
        created_at: workflow.created_at,
        updated_at: workflow.updated_at,
        nodes: vec![], // Will be populated by subsequent GET request
        edges: vec![], // Will be populated by subsequent GET request
    };

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

    Ok(StatusCode::NO_CONTENT)
}