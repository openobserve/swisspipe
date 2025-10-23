use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::schedule::ScheduleConfig;
use crate::AppState;

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateScheduleRequest {
    pub schedule_name: Option<String>,
    pub cron_expression: String,
    pub timezone: String,
    pub test_payload: serde_json::Value,
    pub enabled: bool,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct ScheduleResponse {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub trigger_node_id: String,
    pub schedule_name: Option<String>,
    pub cron_expression: String,
    pub timezone: String,
    pub test_payload: serde_json::Value,
    pub enabled: bool,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub last_execution_time: Option<DateTime<Utc>>,
    pub next_execution_time: Option<DateTime<Utc>>,
    pub execution_count: i64,
    pub failure_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEnabledRequest {
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct ValidateCronRequest {
    pub cron_expression: String,
    pub timezone: String,
}

#[derive(Debug, Serialize)]
pub struct ValidateCronResponse {
    pub valid: bool,
    pub next_executions: Vec<String>,
}

/// Create or update schedule for a trigger
pub async fn upsert_schedule(
    State(state): State<AppState>,
    Path((workflow_id, node_id)): Path<(Uuid, String)>,
    Json(request): Json<CreateScheduleRequest>,
) -> std::result::Result<Json<ScheduleResponse>, (StatusCode, Json<ErrorResponse>)> {
    let schedule_service = &state.schedule_service;
    let config = ScheduleConfig {
        schedule_name: request.schedule_name,
        cron_expression: request.cron_expression,
        timezone: request.timezone,
        test_payload: request.test_payload,
        enabled: request.enabled,
        start_date: request.start_date,
        end_date: request.end_date,
    };

    let schedule = schedule_service.upsert_schedule(workflow_id, node_id, config).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;

    let response = ScheduleResponse {
        id: schedule.id,
        workflow_id: schedule.workflow_id,
        trigger_node_id: schedule.trigger_node_id,
        schedule_name: schedule.schedule_name,
        cron_expression: schedule.cron_expression,
        timezone: schedule.timezone,
        test_payload: schedule.test_payload,
        enabled: schedule.enabled,
        start_date: schedule.start_date,
        end_date: schedule.end_date,
        last_execution_time: schedule.last_execution_time,
        next_execution_time: schedule.next_execution_time,
        execution_count: schedule.execution_count,
        failure_count: schedule.failure_count,
        created_at: schedule.created_at,
        updated_at: schedule.updated_at,
    };

    Ok(Json(response))
}

/// Get schedule for a trigger
pub async fn get_schedule(
    State(state): State<AppState>,
    Path((workflow_id, node_id)): Path<(Uuid, String)>,
) -> std::result::Result<Json<Option<ScheduleResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let schedule_service = &state.schedule_service;
    let schedule = schedule_service.get_schedule(workflow_id, &node_id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;

    match schedule {
        Some(schedule) => {
            let response = ScheduleResponse {
                id: schedule.id,
                workflow_id: schedule.workflow_id,
                trigger_node_id: schedule.trigger_node_id,
                schedule_name: schedule.schedule_name,
                cron_expression: schedule.cron_expression,
                timezone: schedule.timezone,
                test_payload: schedule.test_payload,
                enabled: schedule.enabled,
                start_date: schedule.start_date,
                end_date: schedule.end_date,
                last_execution_time: schedule.last_execution_time,
                next_execution_time: schedule.next_execution_time,
                execution_count: schedule.execution_count,
                failure_count: schedule.failure_count,
                created_at: schedule.created_at,
                updated_at: schedule.updated_at,
            };
            Ok(Json(Some(response)))
        }
        None => Ok(Json(None)),
    }
}

/// Enable/disable schedule
pub async fn update_enabled(
    State(state): State<AppState>,
    Path((workflow_id, node_id)): Path<(Uuid, String)>,
    Json(request): Json<UpdateEnabledRequest>,
) -> std::result::Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let schedule_service = &state.schedule_service;
    schedule_service.set_enabled(workflow_id, &node_id, request.enabled).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Schedule {} successfully", if request.enabled { "enabled" } else { "disabled" })
    })))
}

/// Delete schedule
pub async fn delete_schedule(
    State(state): State<AppState>,
    Path((workflow_id, node_id)): Path<(Uuid, String)>,
) -> std::result::Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let schedule_service = &state.schedule_service;
    schedule_service.delete_schedule(workflow_id, &node_id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Schedule deleted successfully"
    })))
}

/// Validate cron expression and preview executions
pub async fn validate_cron(
    State(state): State<AppState>,
    Json(request): Json<ValidateCronRequest>,
) -> std::result::Result<Json<ValidateCronResponse>, (StatusCode, Json<ErrorResponse>)> {
    let schedule_service = &state.schedule_service;
    // Validate cron expression
    match schedule_service.validate_cron(&request.cron_expression) {
        Ok(_) => {
            // Get preview of next 5 executions
            let executions = schedule_service.preview_executions(
                &request.cron_expression,
                &request.timezone,
                5,
            ).map_err(|e| (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: e.to_string() })))?;

            let next_executions: Vec<String> = executions
                .iter()
                .map(|dt| dt.to_rfc3339())
                .collect();

            Ok(Json(ValidateCronResponse {
                valid: true,
                next_executions,
            }))
        }
        Err(e) => {
            Ok(Json(ValidateCronResponse {
                valid: false,
                next_executions: vec![format!("Error: {}", e)],
            }))
        }
    }
}
