use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde_json::Value;

use crate::{
    variables::{CreateVariableRequest, UpdateVariableRequest},
    AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(get_all_variables).post(create_variable))
        .route("/:id", get(get_variable).put(update_variable).delete(delete_variable))
        .route("/validate", post(validate_variable_name))
}

/// Get all variables
pub async fn get_all_variables(
    State(state): State<AppState>,
) -> std::result::Result<Json<Value>, StatusCode> {
    let variables = state.variable_service
        .get_all_variables()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get variables");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(serde_json::json!({
        "variables": variables
    })))
}

/// Get variable by ID
pub async fn get_variable(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> std::result::Result<Json<Value>, StatusCode> {
    let variable = state.variable_service
        .get_variable(&id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, id = %id, "Failed to get variable");
            match e {
                crate::workflow::errors::SwissPipeError::NotFound(_) => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

    Ok(Json(serde_json::to_value(variable).unwrap()))
}

/// Create a new variable
pub async fn create_variable(
    State(state): State<AppState>,
    Json(req): Json<CreateVariableRequest>,
) -> std::result::Result<Json<Value>, StatusCode> {
    let variable = state.variable_service
        .create_variable(req)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create variable");
            match e {
                crate::workflow::errors::SwissPipeError::ValidationError(_) => StatusCode::BAD_REQUEST,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

    Ok(Json(serde_json::to_value(variable).unwrap()))
}

/// Update a variable
pub async fn update_variable(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateVariableRequest>,
) -> std::result::Result<Json<Value>, StatusCode> {
    let variable = state.variable_service
        .update_variable(&id, req)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, id = %id, "Failed to update variable");
            match e {
                crate::workflow::errors::SwissPipeError::NotFound(_) => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

    Ok(Json(serde_json::to_value(variable).unwrap()))
}

/// Delete a variable
pub async fn delete_variable(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> std::result::Result<Json<Value>, StatusCode> {
    state.variable_service
        .delete_variable(&id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, id = %id, "Failed to delete variable");
            match e {
                crate::workflow::errors::SwissPipeError::NotFound(_) => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

    Ok(Json(serde_json::json!({
        "message": "Variable deleted successfully"
    })))
}

/// Validate variable name
pub async fn validate_variable_name(
    Json(body): Json<Value>,
) -> std::result::Result<Json<Value>, StatusCode> {
    let name = body.get("name")
        .and_then(|v| v.as_str())
        .ok_or(StatusCode::BAD_REQUEST)?;

    match crate::variables::VariableService::validate_name(name) {
        Ok(_) => Ok(Json(serde_json::json!({
            "valid": true
        }))),
        Err(e) => Ok(Json(serde_json::json!({
            "valid": false,
            "error": format!("{}", e)
        }))),
    }
}
