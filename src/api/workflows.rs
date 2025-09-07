#[allow(unused_imports)]
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    database::{edges, entities, nodes},
    workflow::{
        models::{Edge, Node, NodeType},
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
}

#[derive(Deserialize)]
pub struct EdgeRequest {
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
            }),
            config: Set(node_config),
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

    let response = WorkflowResponse {
        id: workflow.id.clone(),
        name: workflow.name,
        description: workflow.description,
        start_node_name: workflow.start_node_name,
        endpoint_url: format!("/api/v1/{}/ep", workflow.id),
        created_at: workflow.created_at,
        updated_at: workflow.updated_at,
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

    let response = WorkflowResponse {
        id: workflow.id.clone(),
        name: workflow.name,
        description: workflow.description,
        start_node_name: workflow.start_node_name,
        endpoint_url: format!("/api/v1/{}/ep", workflow.id),
        created_at: workflow.created_at,
        updated_at: workflow.updated_at,
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