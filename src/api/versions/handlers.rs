use crate::versions::CreateVersionRequest;
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct PaginationQuery {
    #[serde(default = "default_limit")]
    pub limit: u64,
    #[serde(default)]
    pub offset: u64,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

fn default_limit() -> u64 {
    50
}

/// Create a new workflow version
/// POST /api/v1/workflows/:workflow_id/versions
pub async fn create_version(
    State(state): State<AppState>,
    Path(workflow_id): Path<String>,
    Json(req): Json<CreateVersionRequest>,
) -> std::result::Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    // Get username from auth context (for now use "admin")
    // TODO: Extract from authentication context when available
    let changed_by = "admin";

    let version = state
        .version_service
        .create_version(
            &workflow_id,
            &req.workflow_snapshot,
            &req.commit_message,
            req.commit_description.as_deref(),
            changed_by,
        )
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;

    tracing::info!(
        "Created version {} for workflow {}",
        version.version_number,
        workflow_id
    );

    let response = serde_json::to_value(version)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;

    Ok((StatusCode::CREATED, Json(response)))
}

/// Get version history for a workflow with pagination
/// GET /api/v1/workflows/:workflow_id/versions
pub async fn get_versions(
    State(state): State<AppState>,
    Path(workflow_id): Path<String>,
    Query(params): Query<PaginationQuery>,
) -> std::result::Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // Enforce maximum limit
    let limit = params.limit.min(100);

    let versions = state
        .version_service
        .get_versions(&workflow_id, limit, params.offset)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;

    let response = serde_json::to_value(versions)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;

    Ok(Json(response))
}

/// Get specific version details with full workflow snapshot
/// GET /api/v1/workflows/:workflow_id/versions/:version_id
pub async fn get_version_detail(
    State(state): State<AppState>,
    Path((workflow_id, version_id)): Path<(String, String)>,
) -> std::result::Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let version = state
        .version_service
        .get_version_detail(&workflow_id, &version_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;

    let response = serde_json::to_value(version)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;

    Ok(Json(response))
}
