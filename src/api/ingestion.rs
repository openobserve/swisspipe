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
    workflow::models::WorkflowEvent,
    AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/:workflow_id/ep", get(trigger_workflow_get).post(trigger_workflow_post).put(trigger_workflow_put))
        .route("/:workflow_id/json_array", post(trigger_workflow_array))
}

pub async fn trigger_workflow_get(
    State(state): State<AppState>,
    Path(workflow_id): Path<String>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> std::result::Result<Json<Value>, StatusCode> {
    let data = params
        .into_iter()
        .map(|(k, v)| (k, Value::String(v)))
        .collect::<serde_json::Map<_, _>>();

    let event = create_workflow_event(Value::Object(data), headers);
    execute_workflow(&state, &workflow_id, event).await
}

pub async fn trigger_workflow_post(
    State(state): State<AppState>,
    Path(workflow_id): Path<String>,
    headers: HeaderMap,
    Json(data): Json<Value>,
) -> std::result::Result<Json<Value>, StatusCode> {
    let event = create_workflow_event(data, headers);
    execute_workflow(&state, &workflow_id, event).await
}

pub async fn trigger_workflow_put(
    State(state): State<AppState>,
    Path(workflow_id): Path<String>,
    headers: HeaderMap,
    Json(data): Json<Value>,
) -> std::result::Result<Json<Value>, StatusCode> {
    let event = create_workflow_event(data, headers);
    execute_workflow(&state, &workflow_id, event).await
}

pub async fn trigger_workflow_array(
    State(state): State<AppState>,
    Path(workflow_id): Path<String>,
    headers: HeaderMap,
    Json(data): Json<Vec<Value>>,
) -> std::result::Result<Json<Value>, StatusCode> {
    let event = create_workflow_event(Value::Array(data), headers);
    execute_workflow(&state, &workflow_id, event).await
}

fn create_workflow_event(data: Value, headers: HeaderMap) -> WorkflowEvent {
    let mut event_headers = HashMap::new();
    let mut metadata = HashMap::new();

    for (name, value) in headers.iter() {
        if let Ok(value_str) = value.to_str() {
            event_headers.insert(name.to_string(), value_str.to_string());
        }
    }

    // Add some basic metadata
    metadata.insert("timestamp".to_string(), chrono::Utc::now().to_rfc3339());
    metadata.insert("event_id".to_string(), uuid::Uuid::new_v4().to_string());

    WorkflowEvent {
        data,
        metadata,
        headers: event_headers,
        condition_results: HashMap::new(),
    }
}

async fn execute_workflow(
    state: &AppState,
    workflow_id: &str,
    event: WorkflowEvent,
) -> std::result::Result<Json<Value>, StatusCode> {
    // Load workflow
    let workflow = state
        .engine
        .load_workflow(workflow_id)
        .await
        .map_err(|e| match e {
            crate::workflow::errors::SwissPipeError::WorkflowNotFound(_) => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })?;

    // Execute workflow
    let result = state
        .engine
        .execute_workflow(&workflow, event)
        .await
        .map_err(|e| {
            tracing::error!("Workflow execution failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(result.data))
}