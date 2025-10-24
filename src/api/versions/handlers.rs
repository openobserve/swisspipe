use crate::versions::CreateVersionRequest;
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use base64::prelude::*;
use serde::{Deserialize, Serialize};
use sea_orm::{EntityTrait, ColumnTrait, QueryFilter};

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

/// Extract username from request headers
async fn extract_username(headers: &HeaderMap, db: &sea_orm::DatabaseConnection) -> String {
    // Check for OAuth session cookie
    if let Some(session_id) = extract_session_id_from_cookies(headers) {
        // Look up session in database to get real user info
        if let Ok(Some(session)) = crate::database::sessions::Entity::find()
            .filter(crate::database::sessions::Column::Id.eq(&session_id))
            .one(db)
            .await
        {
            if !session.is_expired() {
                return session.email;
            }
        }
    }

    // Check for Basic Auth
    if let Some(auth_header) = headers.get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(encoded) = auth_str.strip_prefix("Basic ") {
                if let Ok(decoded) = BASE64_STANDARD.decode(encoded) {
                    if let Ok(credentials) = String::from_utf8(decoded) {
                        if let Some((username, _)) = credentials.split_once(':') {
                            return username.to_string();
                        }
                    }
                }
            }
        }
    }

    // Unknown/unauthenticated user
    "anonymous".to_string()
}

/// Extract session ID from cookie header
fn extract_session_id_from_cookies(headers: &HeaderMap) -> Option<String> {
    headers
        .get("cookie")?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|cookie| {
            let trimmed = cookie.trim();
            trimmed.strip_prefix("session_id=")
                .map(|session_id| session_id.to_string())
        })
}

/// Create a new workflow version
/// POST /api/v1/workflows/:workflow_id/versions
pub async fn create_version(
    State(state): State<AppState>,
    Path(workflow_id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<CreateVersionRequest>,
) -> std::result::Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    // Get username from auth context
    let changed_by = extract_username(&headers, &state.db).await;

    let version = state
        .version_service
        .create_version(
            &workflow_id,
            &req.workflow_snapshot,
            &req.commit_message,
            req.commit_description.as_deref(),
            &changed_by,
        )
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;

    tracing::info!(
        "Created version {} for workflow {} by user {}",
        version.version_number,
        workflow_id,
        changed_by
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
