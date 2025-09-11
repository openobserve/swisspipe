#[allow(unused_imports)]
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::{get, post, put},
    Router,
};
use serde_json::Value;
use std::collections::HashMap;

use crate::{
    async_execution::ExecutionService,
    AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/:workflow_id/trigger", get(trigger_workflow_get).post(trigger_workflow_post).put(trigger_workflow_put))
        .route("/:workflow_id/json_array", post(trigger_workflow_array))
}

pub async fn trigger_workflow_get(
    State(state): State<AppState>,
    Path(workflow_id): Path<String>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> std::result::Result<(StatusCode, Json<Value>), StatusCode> {
    let data = params
        .into_iter()
        .map(|(k, v)| (k, Value::String(v)))
        .collect::<serde_json::Map<_, _>>();

    let event_headers = extract_headers(&headers);
    execute_workflow_async(&state, &workflow_id, Value::Object(data), event_headers).await
}

pub async fn trigger_workflow_post(
    State(state): State<AppState>,
    Path(workflow_id): Path<String>,
    headers: HeaderMap,
    Json(data): Json<Value>,
) -> std::result::Result<(StatusCode, Json<Value>), StatusCode> {
    let event_headers = extract_headers(&headers);
    execute_workflow_async(&state, &workflow_id, data, event_headers).await
}

pub async fn trigger_workflow_put(
    State(state): State<AppState>,
    Path(workflow_id): Path<String>,
    headers: HeaderMap,
    Json(data): Json<Value>,
) -> std::result::Result<(StatusCode, Json<Value>), StatusCode> {
    let event_headers = extract_headers(&headers);
    execute_workflow_async(&state, &workflow_id, data, event_headers).await
}

pub async fn trigger_workflow_array(
    State(state): State<AppState>,
    Path(workflow_id): Path<String>,
    headers: HeaderMap,
    Json(data): Json<Vec<Value>>,
) -> std::result::Result<(StatusCode, Json<Value>), StatusCode> {
    let event_headers = extract_headers(&headers);
    execute_workflow_async(&state, &workflow_id, Value::Array(data), event_headers).await
}

fn extract_headers(headers: &HeaderMap) -> HashMap<String, String> {
    let mut event_headers = HashMap::new();
    
    for (name, value) in headers.iter() {
        if let Ok(value_str) = value.to_str() {
            event_headers.insert(name.to_string(), value_str.to_string());
        }
    }
    
    event_headers
}

async fn execute_workflow_async(
    state: &AppState,
    workflow_id: &str,
    input_data: Value,
    headers: HashMap<String, String>,
) -> std::result::Result<(StatusCode, Json<Value>), StatusCode> {
    tracing::info!("Executing workflow: {}", workflow_id);
    
    // Try to get workflow from cache first
    let cached_workflow = state.workflow_cache.get(workflow_id).await;
    
    let start_node_name = if let Some(cached) = cached_workflow {
        // Use cached workflow metadata
        tracing::debug!("Using cached workflow metadata for {}", workflow_id);
        cached.start_node_name
    } else {
        // Cache miss - load from database and cache the result
        tracing::debug!("Cache miss - loading workflow {} from database", workflow_id);
        let workflow = state
            .engine
            .load_workflow(workflow_id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to load workflow {}: {}", workflow_id, e);
                match e {
                    crate::workflow::errors::SwissPipeError::WorkflowNotFound(_) => StatusCode::NOT_FOUND,
                    _ => StatusCode::INTERNAL_SERVER_ERROR,
                }
            })?;

        let start_node = workflow.start_node_name.clone();
        
        // Cache the workflow metadata for future requests
        state.workflow_cache.put(workflow_id.to_string(), start_node.clone()).await;
        tracing::debug!("Cached workflow metadata for {}", workflow_id);
        
        start_node
    };

    tracing::info!("Workflow {} validated successfully (start_node: {})", workflow_id, start_node_name);

    // Create execution service
    let execution_service = ExecutionService::new(state.db.clone());
    
    // Create execution and queue job
    let execution_id = execution_service
        .create_execution(
            workflow_id.to_string(),
            input_data,
            headers,
            None, // No priority specified
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to create execution: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Return HTTP 202 with execution details
    let response = serde_json::json!({
        "status": "accepted",
        "execution_id": execution_id,
        "message": "Workflow execution has been queued"
    });

    Ok((StatusCode::ACCEPTED, Json(response)))
}