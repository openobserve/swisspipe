use axum::{
    extract::{Path, State, Request, FromRequest},
    http::StatusCode,
    response::{Json, IntoResponse},
};
use sea_orm::{ActiveModelTrait, EntityTrait, Set, ColumnTrait, QueryFilter};
use std::collections::HashMap;
use uuid::Uuid;
use tracing::{info, warn, error, debug};

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

// Custom JSON extractor for better error handling
pub struct JsonWithBetterErrors<T>(pub T);

#[axum::async_trait]
impl<T, S> FromRequest<S> for JsonWithBetterErrors<T>
where
    T: serde::de::DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<ErrorResponse>);

    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        // Extract and log request body details for debugging
        let content_length = req.headers().get("content-length")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse::<usize>().ok());

        let content_type = req.headers().get("content-type")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("unknown");

        tracing::debug!("Processing JSON request: content-length={:?}, content-type={}",
                       content_length, content_type);

        // For debugging empty body issues, let's read the body first and then reconstruct the request
        let body_bytes = match axum::body::to_bytes(req.into_body(), usize::MAX).await {
            Ok(bytes) => bytes,
            Err(err) => {
                tracing::error!("Failed to read request body bytes: {}", err);
                return Err((StatusCode::BAD_REQUEST, Json(ErrorResponse {
                    error: "BODY_READ_ERROR".to_string(),
                    message: "Failed to read request body".to_string(),
                    details: Some(format!("Body read error: {err}")),
                })));
            }
        };

        tracing::debug!("Request body size: {} bytes", body_bytes.len());

        if body_bytes.is_empty() {
            tracing::error!("Request body is completely empty - this explains the EOF error");
            return Err((StatusCode::BAD_REQUEST, Json(ErrorResponse {
                error: "EMPTY_BODY".to_string(),
                message: "Request body is empty".to_string(),
                details: Some("The request body contains no data. This may indicate a frontend serialization issue or network problem.".to_string()),
            })));
        }

        if body_bytes.len() < 100 {
            // Log small bodies completely for debugging
            let body_str = String::from_utf8_lossy(&body_bytes);
            tracing::debug!("Small request body content: '{}'", body_str);
        } else {
            // Log first and last 50 chars for large bodies
            let body_str = String::from_utf8_lossy(&body_bytes);
            let start = body_str.chars().take(50).collect::<String>();
            let end = body_str.chars().rev().take(50).collect::<String>().chars().rev().collect::<String>();
            tracing::debug!("Large request body preview: start='{}' ... end='{}'", start, end);
        }

        // Now try to deserialize the body
        match serde_json::from_slice::<T>(&body_bytes) {
            Ok(value) => Ok(JsonWithBetterErrors(value)),
            Err(err) => {
                tracing::error!("JSON deserialization failed: {}", err);

                Err((StatusCode::BAD_REQUEST, Json(ErrorResponse {
                    error: "INVALID_JSON".to_string(),
                    message: "Request body contains invalid JSON".to_string(),
                    details: Some(format!("Serde JSON error: {err}")),
                })))
            }
        }
    }
}

/// Maps HTTP status codes to structured error responses for workflow operations
fn map_status_to_error_response(status: StatusCode) -> ErrorResponse {
    match status {
        StatusCode::NOT_FOUND => ErrorResponse {
            error: "NOT_FOUND".to_string(),
            message: "Workflow not found".to_string(),
            details: None,
        },
        StatusCode::BAD_REQUEST => ErrorResponse {
            error: "BAD_REQUEST".to_string(),
            message: "Invalid workflow request".to_string(),
            details: None,
        },
        StatusCode::CONFLICT => ErrorResponse {
            error: "CONFLICT".to_string(),
            message: "Cannot update workflow while executions are running".to_string(),
            details: Some("The workflow has active executions. Please wait for them to complete or cancel them before updating the workflow.".to_string()),
        },
        _ => ErrorResponse {
            error: "INTERNAL_SERVER_ERROR".to_string(),
            message: "Internal server error occurred during workflow update".to_string(),
            details: None,
        },
    }
}


pub async fn list_workflows(
    State(state): State<AppState>,
) -> std::result::Result<Json<WorkflowListResponse>, (StatusCode, Json<ErrorResponse>)> {
    debug!("Listing all workflows");

    let workflows = entities::Entity::find()
        .all(&*state.db)
        .await
        .map_err(|e| {
            error!("Database error in list_workflows: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                error: "DATABASE_ERROR".to_string(),
                message: "Failed to fetch workflows from database".to_string(),
                details: Some(format!("Database error: {e}")),
            }))
        })?;

    let workflow_count = workflows.len();
    let workflow_responses: Vec<WorkflowResponse> = workflows
        .into_iter()
        .map(|w| {
            let workflow_id = w.id.clone();
            WorkflowResponse {
                endpoint_url: format!("/api/v1/{workflow_id}/trigger"),
                id: workflow_id.clone(),
                name: w.name,
                description: w.description,
                start_node_id: w.start_node_id.unwrap_or_else(|| {
                    warn!("Workflow {} has no start_node_id", workflow_id);
                    String::new() // More efficient than "".to_string()
                }),
                enabled: w.enabled,
                created_at: w.created_at,
                updated_at: w.updated_at,
                nodes: Vec::new(), // Not included in list view for performance
                edges: Vec::new(), // Not included in list view for performance
            }
        })
        .collect();

    info!("Successfully listed {} workflows", workflow_count);
    Ok(Json(WorkflowListResponse {
        workflows: workflow_responses,
    }))
}

pub async fn create_workflow(
    State(state): State<AppState>,
    JsonWithBetterErrors(request): JsonWithBetterErrors<CreateWorkflowRequest>,
) -> std::result::Result<(StatusCode, Json<WorkflowResponse>), (StatusCode, Json<ErrorResponse>)> {
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
                    return Err((StatusCode::BAD_REQUEST, Json(ErrorResponse {
                        error: "INVALID_START_NODE".to_string(),
                        message: format!("Provided start node '{provided_start_id}' is not a trigger node"),
                        details: Some("Start node must be a trigger node that can accept HTTP requests".to_string()),
                    })));
                }
                provided_start_id.clone()
            }
            None => {
                tracing::warn!(
                    "Provided start_node_id '{}' not found in nodes list: workflow_name='{}'",
                    provided_start_id, request.name
                );
                return Err((StatusCode::BAD_REQUEST, Json(ErrorResponse {
                    error: "START_NODE_NOT_FOUND".to_string(),
                    message: format!("Provided start_node_id '{provided_start_id}' not found in nodes list"),
                    details: Some("Start node ID must reference an existing node in the workflow".to_string()),
                })));
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
        return Err((StatusCode::BAD_REQUEST, Json(ErrorResponse {
            error: "WORKFLOW_VALIDATION_FAILED".to_string(),
            message: "Workflow validation failed".to_string(),
            details: Some(validation_error.to_string()),
        })));
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
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                error: "DATABASE_ERROR".to_string(),
                message: "Failed to create workflow".to_string(),
                details: Some(format!("Database error: {e}")),
            }))
        })?;

    // Create all nodes (start node + user nodes)
    for node in &nodes {
        let node_config = serde_json::to_string(&node.node_type)
            .map_err(|e| {
                tracing::error!("Failed to serialize node config during creation: workflow_id={}, node_id={}, error={:?}",
                               workflow_id, node.id, e);
                (StatusCode::BAD_REQUEST, Json(ErrorResponse {
                    error: "NODE_CONFIG_ERROR".to_string(),
                    message: format!("Failed to serialize configuration for node '{}'", node.name),
                    details: Some(format!("Node '{}' ({}): {}", node.name, node.id, e)),
                }))
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
                (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                    error: "DATABASE_ERROR".to_string(),
                    message: format!("Failed to create node '{}'", node.name),
                    details: Some(format!("Node '{}' ({}): {}", node.name, node.id, e)),
                }))
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
                (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                    error: "DATABASE_ERROR".to_string(),
                    message: "Failed to create workflow edge".to_string(),
                    details: Some(format!("Edge from {} to {}: {}", edge_req.from_node_id, edge_req.to_node_id, e)),
                }))
            })?;
    }

    // Fetch nodes
    let nodes = nodes::Entity::find()
        .filter(nodes::Column::WorkflowId.eq(&workflow_id))
        .all(&*state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch created nodes: workflow_id={}, error={:?}", workflow_id, e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                error: "DATABASE_ERROR".to_string(),
                message: "Failed to fetch created workflow nodes".to_string(),
                details: Some(format!("Database error: {e}")),
            }))
        })?;

    // Fetch edges
    let edges = edges::Entity::find()
        .filter(edges::Column::WorkflowId.eq(&workflow_id))
        .all(&*state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch created edges: workflow_id={}, error={:?}", workflow_id, e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                error: "DATABASE_ERROR".to_string(),
                message: "Failed to fetch created workflow edges".to_string(),
                details: Some(format!("Database error: {e}")),
            }))
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
        enabled: workflow.enabled,
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
        enabled: workflow.enabled,
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
    tracing::info!("Workflow disable initiated: workflow_id={}", id);

    // Find the workflow first
    let workflow = entities::Entity::find_by_id(&id)
        .one(&*state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to find workflow for disable: workflow_id={}, error={:?}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let workflow = match workflow {
        Some(w) => w,
        None => {
            tracing::warn!("Workflow not found for disable: workflow_id={}", id);
            return Err(StatusCode::NOT_FOUND);
        }
    };

    // Update workflow to disabled
    let mut workflow: entities::ActiveModel = workflow.into();
    workflow.enabled = Set(false);
    workflow.updated_at = Set(chrono::Utc::now().timestamp_micros());

    workflow.update(&*state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to disable workflow: workflow_id={}, error={:?}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Invalidate cache for disabled workflow
    tracing::debug!("Invalidating cache for disabled workflow: workflow_id={}", id);
    state.workflow_cache.invalidate(&id).await;

    tracing::info!("Workflow disabled successfully: workflow_id={}", id);
    Ok(StatusCode::NO_CONTENT)
}

pub async fn enable_workflow(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> std::result::Result<StatusCode, StatusCode> {
    tracing::info!("Workflow enable initiated: workflow_id={}", id);

    // Find the workflow first
    let workflow = entities::Entity::find_by_id(&id)
        .one(&*state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to find workflow for enable: workflow_id={}, error={:?}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let workflow = match workflow {
        Some(w) => w,
        None => {
            tracing::warn!("Workflow not found for enable: workflow_id={}", id);
            return Err(StatusCode::NOT_FOUND);
        }
    };

    // Update workflow to enabled
    let mut workflow: entities::ActiveModel = workflow.into();
    workflow.enabled = Set(true);
    workflow.updated_at = Set(chrono::Utc::now().timestamp_micros());

    workflow.update(&*state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to enable workflow: workflow_id={}, error={:?}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Invalidate cache for enabled workflow
    tracing::debug!("Invalidating cache for enabled workflow: workflow_id={}", id);
    state.workflow_cache.invalidate(&id).await;

    tracing::info!("Workflow enabled successfully: workflow_id={}", id);
    Ok(StatusCode::NO_CONTENT)
}

pub async fn update_workflow(
    State(state): State<AppState>,
    Path(id): Path<String>,
    JsonWithBetterErrors(request): JsonWithBetterErrors<CreateWorkflowRequest>,
) -> Result<Json<WorkflowResponse>, impl IntoResponse> {
    tracing::info!("Updating workflow: workflow_id={}", id);

    // Pre-validate request to provide detailed error messages
    // Fetch existing nodes for validation context
    let existing_nodes = nodes::Entity::find()
        .filter(nodes::Column::WorkflowId.eq(&id))
        .all(&*state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch existing nodes for validation: workflow_id={}, error={:?}", id, e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                error: "DATABASE_ERROR".to_string(),
                message: "Failed to fetch workflow data for validation".to_string(),
                details: Some(format!("Database error: {e}")),
            }))
        })?;

    // Get the existing workflow to find start node ID
    let workflow = entities::Entity::find_by_id(&id)
        .one(&*state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch workflow for validation: workflow_id={}, error={:?}", id, e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                error: "DATABASE_ERROR".to_string(),
                message: "Failed to fetch workflow data".to_string(),
                details: Some(format!("Database error: {e}")),
            }))
        })?
        .ok_or_else(|| {
            tracing::warn!("Workflow not found for validation: workflow_id={}", id);
            (StatusCode::NOT_FOUND, Json(ErrorResponse {
                error: "WORKFLOW_NOT_FOUND".to_string(),
                message: format!("Workflow '{id}' not found"),
                details: None,
            }))
        })?;

    let start_node_id = workflow.start_node_id.clone().ok_or_else(|| {
        tracing::error!("Workflow {} has no start_node_id", id);
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: "INVALID_WORKFLOW_STATE".to_string(),
            message: "Workflow is in invalid state - missing start node".to_string(),
            details: None,
        }))
    })?;

    // Validate the update request with detailed error reporting
    use super::validation::validate_workflow_update_request;
    if let Err(validation_error) = validate_workflow_update_request(&request, &start_node_id, &existing_nodes) {
        tracing::warn!(
            "Workflow update validation failed: workflow_id={}, error='{}'",
            id, validation_error
        );
        return Err((StatusCode::BAD_REQUEST, Json(ErrorResponse {
            error: "VALIDATION_FAILED".to_string(),
            message: "Workflow validation failed".to_string(),
            details: Some(validation_error.to_string()),
        })));
    }

    let workflow_id = id.clone();
    let service = super::service::UpdateWorkflowService::new(&state, id, request);

    match service.update_workflow().await {
        Ok(response) => {
            // Invalidate cache after successful workflow update
            // This ensures new executions get the updated workflow while ongoing ones continue with cached version
            tracing::debug!("Workflow update successful: invalidating cache for workflow_id={}", workflow_id);
            state.workflow_cache.invalidate(&workflow_id).await;

            // Cache the new workflow metadata for future requests
            state.workflow_cache.put(workflow_id, response.start_node_id.clone()).await;

            Ok(Json(response))
        }
        Err(status) => {
            let error_response = map_status_to_error_response(status);
            Err((status, Json(error_response)))
        }
    }
}