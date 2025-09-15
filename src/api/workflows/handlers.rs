use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use sea_orm::{ActiveModelTrait, EntityTrait, Set, ColumnTrait, QueryFilter};
use std::collections::HashMap;
use uuid::Uuid;

use crate::{
    database::{edges, entities, nodes},
    workflow::{
        models::{Edge, Node, NodeType, HttpMethod, RetryConfig, FailureAction},
        validation::WorkflowValidator,
    },
    AppState,
};

use super::{
    types::*,
};

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

    // Convert request nodes to internal models first
    let mut nodes: Vec<Node> = request.nodes.iter().map(|n| Node {
        id: n.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string()),
        workflow_id: workflow_id.clone(),
        name: n.name.clone(),
        node_type: n.node_type.clone(),
        input_merge_strategy: None,
    }).collect();

    // Determine start node ID and validate
    let start_node_id = if let Some(provided_start_id) = &request.start_node_id {
        // Frontend provided a start node ID - validate it exists in nodes list and is a trigger
        let start_node = nodes.iter().find(|n| &n.id == provided_start_id);
        match start_node {
            Some(node) => {
                if !matches!(node.node_type, NodeType::Trigger { .. }) {
                    tracing::warn!(
                        "Provided start_node_id '{}' is not a trigger node: workflow_name='{}'",
                        provided_start_id, request.name
                    );
                    return Err(StatusCode::BAD_REQUEST);
                }
                provided_start_id.clone()
            }
            None => {
                tracing::warn!(
                    "Provided start_node_id '{}' not found in nodes list: workflow_name='{}'",
                    provided_start_id, request.name
                );
                return Err(StatusCode::BAD_REQUEST);
            }
        }
    } else {
        // No start node provided - check if there's already a trigger node, otherwise auto-create
        if let Some(trigger_node) = nodes.iter().find(|n| matches!(n.node_type, NodeType::Trigger { .. })) {
            // Use existing trigger node as start
            trigger_node.id.clone()
        } else {
            // Auto-create start node for backward compatibility
            let auto_start_id = Uuid::new_v4().to_string();
            let start_node = Node {
                id: auto_start_id.clone(),
                workflow_id: workflow_id.clone(),
                name: "Start".to_string(),
                node_type: NodeType::Trigger {
                    methods: vec![HttpMethod::Get, HttpMethod::Post, HttpMethod::Put]
                },
                input_merge_strategy: None,
            };
            nodes.insert(0, start_node);
            auto_start_id
        }
    };

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

        let (position_x, position_y) = {
            // First try to find position from request nodes
            let user_node = request.nodes.iter().find(|n|
                n.id.as_ref().unwrap_or(&String::new()) == &node.id || n.name == node.name
            );

            if let Some(user_node) = user_node {
                // Use provided position or defaults
                (
                    user_node.position_x.unwrap_or(100.0),
                    user_node.position_y.unwrap_or(100.0)
                )
            } else if node.id == start_node_id && node.name == "Start" {
                // Auto-created start node gets special positioning
                (400.0, 50.0)
            } else {
                // Default positioning for any other nodes
                (100.0, 100.0)
            }
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
                NodeType::Anthropic { .. } => "anthropic".to_string(),
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
                    headers: HashMap::new(),
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
                    headers: HashMap::new(),
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

pub async fn update_workflow(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(request): Json<CreateWorkflowRequest>,
) -> Result<Json<WorkflowResponse>, StatusCode> {
    tracing::info!("Updating workflow: workflow_id={}", id);
    
    let service = super::service::UpdateWorkflowService::new(&state, id, request);
    let response = service.update_workflow().await?;
    
    Ok(Json(response))
}