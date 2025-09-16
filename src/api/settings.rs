use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, put},
    Router,
};
use sea_orm::{ActiveModelTrait, EntityTrait, Set, ColumnTrait, QueryFilter};
use serde::{Deserialize, Serialize};

use crate::{
    database::settings,
    AppState,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct SettingResponse {
    pub key: String,
    pub value: String,
    pub description: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SettingsListResponse {
    pub settings: Vec<SettingResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateSettingRequest {
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub details: Option<String>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_settings))
        .route("/:key", get(get_setting))
        .route("/:key", put(update_setting))
}

pub async fn list_settings(
    State(state): State<AppState>,
) -> std::result::Result<Json<SettingsListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let settings_models = settings::Entity::find()
        .all(&*state.db)
        .await
        .map_err(|e| {
            tracing::error!("Database error in list_settings: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                error: "Failed to fetch settings from database".to_string(),
                details: Some(format!("Database error: {e}")),
            }))
        })?;

    let settings: Vec<SettingResponse> = settings_models
        .into_iter()
        .map(|s| SettingResponse {
            key: s.key,
            value: s.value,
            description: s.description,
            created_at: s.created_at,
            updated_at: s.updated_at,
        })
        .collect();

    Ok(Json(SettingsListResponse { settings }))
}

pub async fn get_setting(
    Path(key): Path<String>,
    State(state): State<AppState>,
) -> std::result::Result<Json<SettingResponse>, (StatusCode, Json<ErrorResponse>)> {
    let setting = settings::Entity::find()
        .filter(settings::Column::Key.eq(&key))
        .one(&*state.db)
        .await
        .map_err(|e| {
            tracing::error!("Database error in get_setting: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                error: "Failed to fetch setting from database".to_string(),
                details: Some(format!("Database error: {e}")),
            }))
        })?
        .ok_or_else(|| {
            (StatusCode::NOT_FOUND, Json(ErrorResponse {
                error: format!("Setting with key '{key}' not found"),
                details: None,
            }))
        })?;

    Ok(Json(SettingResponse {
        key: setting.key,
        value: setting.value,
        description: setting.description,
        created_at: setting.created_at,
        updated_at: setting.updated_at,
    }))
}

pub async fn update_setting(
    Path(key): Path<String>,
    State(state): State<AppState>,
    Json(request): Json<UpdateSettingRequest>,
) -> std::result::Result<Json<SettingResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Check if setting exists
    let existing_setting = settings::Entity::find()
        .filter(settings::Column::Key.eq(&key))
        .one(&*state.db)
        .await
        .map_err(|e| {
            tracing::error!("Database error in update_setting: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                error: "Failed to fetch setting from database".to_string(),
                details: Some(format!("Database error: {e}")),
            }))
        })?
        .ok_or_else(|| {
            (StatusCode::NOT_FOUND, Json(ErrorResponse {
                error: format!("Setting with key '{key}' not found"),
                details: None,
            }))
        })?;

    // Update the setting
    let now = chrono::Utc::now().timestamp_micros();
    let mut active_model: settings::ActiveModel = existing_setting.into();
    active_model.value = Set(request.value);
    active_model.updated_at = Set(now);

    let updated_setting = active_model.update(&*state.db)
        .await
        .map_err(|e| {
            tracing::error!("Database error updating setting: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                error: "Failed to update setting".to_string(),
                details: Some(format!("Database error: {e}")),
            }))
        })?;

    tracing::info!("Setting '{}' updated successfully", key);

    Ok(Json(SettingResponse {
        key: updated_setting.key,
        value: updated_setting.value,
        description: updated_setting.description,
        created_at: updated_setting.created_at,
        updated_at: updated_setting.updated_at,
    }))
}