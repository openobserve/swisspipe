use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use sea_orm::{EntityTrait, ColumnTrait, QueryFilter, Set, ActiveModelTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{Utc, DateTime};

use crate::{
    AppState,
    database::human_in_loop_tasks,
};

#[derive(Debug, Deserialize)]
pub struct HilResponseQuery {
    pub decision: String,
    pub data: Option<String>,
    pub comments: Option<String>,
}

// Input validation constants
const MAX_DATA_LENGTH: usize = 10_000; // 10KB limit for data field
const MAX_COMMENTS_LENGTH: usize = 5_000; // 5KB limit for comments
const ALLOWED_DECISIONS: &[&str] = &["approved", "denied"]; // Whitelist of valid decisions

// Validation helper functions
fn validate_decision(decision: &str) -> Result<(), String> {
    if !ALLOWED_DECISIONS.contains(&decision) {
        return Err(format!("Invalid decision '{}'. Must be one of: {}",
                          decision, ALLOWED_DECISIONS.join(", ")));
    }
    Ok(())
}

fn validate_data_field(data: &str) -> Result<(), String> {
    if data.len() > MAX_DATA_LENGTH {
        return Err(format!("Data field too long ({} bytes). Maximum allowed: {} bytes",
                          data.len(), MAX_DATA_LENGTH));
    }

    // Check for potential malicious patterns
    let suspicious_patterns = &["<script", "javascript:", "data:", "vbscript:", "onload=", "onerror="];
    for pattern in suspicious_patterns {
        if data.to_lowercase().contains(pattern) {
            return Err("Data field contains potentially malicious content".to_string());
        }
    }

    Ok(())
}

fn validate_comments_field(comments: &str) -> Result<(), String> {
    if comments.len() > MAX_COMMENTS_LENGTH {
        return Err(format!("Comments field too long ({} bytes). Maximum allowed: {} bytes",
                          comments.len(), MAX_COMMENTS_LENGTH));
    }

    // Basic sanitization - no HTML/script tags allowed in comments
    if comments.contains('<') || comments.contains('>') {
        return Err("Comments field cannot contain HTML tags".to_string());
    }

    Ok(())
}


#[derive(Debug, Deserialize)]
pub struct HilTasksQuery {
    pub status: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HilResponseSuccess {
    pub status: String,
    pub message: String,
    pub task: HilTaskSummary,
}

#[derive(Debug, Serialize)]
pub struct HilTaskSummary {
    pub id: String,
    pub title: String,
    pub status: String,
    pub response_received_at: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct HilErrorResponse {
    pub status: String,
    pub error: String,
}

/// Handle human response to HIL task
pub async fn respond_to_hil_task(
    State(state): State<AppState>,
    Path(node_execution_id): Path<Uuid>,
    Query(params): Query<HilResponseQuery>,
) -> Result<Json<HilResponseSuccess>, (StatusCode, Json<HilErrorResponse>)> {
    tracing::info!("Received HIL response for node execution: {}", node_execution_id);
    tracing::debug!("HIL response params: {:?}", params);

    // Comprehensive input validation
    if let Err(err) = validate_decision(&params.decision) {
        tracing::warn!("Invalid decision parameter from {}: {}", node_execution_id, err);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(HilErrorResponse {
                status: "error".to_string(),
                error: err,
            }),
        ));
    }

    // Validate data field if provided
    if let Some(data) = &params.data {
        if let Err(err) = validate_data_field(data) {
            tracing::warn!("Invalid data field from {}: {}", node_execution_id, err);
            return Err((
                StatusCode::BAD_REQUEST,
                Json(HilErrorResponse {
                    status: "error".to_string(),
                    error: format!("Data validation failed: {err}"),
                }),
            ));
        }
    }

    // Validate comments field if provided
    if let Some(comments) = &params.comments {
        if let Err(err) = validate_comments_field(comments) {
            tracing::warn!("Invalid comments field from {}: {}", node_execution_id, err);
            return Err((
                StatusCode::BAD_REQUEST,
                Json(HilErrorResponse {
                    status: "error".to_string(),
                    error: format!("Comments validation failed: {err}"),
                }),
            ));
        }
    }


    // Find the HIL task by node_execution_id (single query for both validation and processing)
    tracing::debug!("Searching for HIL task with node_execution_id: {} and status: pending", node_execution_id);

    let task = human_in_loop_tasks::Entity::find()
        .filter(human_in_loop_tasks::Column::NodeExecutionId.eq(node_execution_id.to_string()))
        .filter(human_in_loop_tasks::Column::Status.eq("pending"))
        .one(state.db.as_ref())
        .await
        .map_err(|e| {
            tracing::error!("Database error finding HIL task: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(HilErrorResponse {
                    status: "error".to_string(),
                    error: "Database error".to_string(),
                }),
            )
        })?;

    tracing::debug!("HIL task query result: {:?}", task.is_some());

    let task = task.ok_or_else(|| {
        tracing::warn!("HIL task not found or already completed: {}", node_execution_id);
        (
            StatusCode::NOT_FOUND,
            Json(HilErrorResponse {
                status: "error".to_string(),
                error: "HIL task not found or already completed".to_string(),
            }),
        )
    })?;


    // Prepare response data
    let mut response_data = serde_json::Map::new();
    response_data.insert("decision".to_string(), serde_json::Value::String(params.decision.clone()));

    if let Some(comments) = &params.comments {
        response_data.insert("comments".to_string(), serde_json::Value::String(comments.clone()));
    }

    if let Some(data) = &params.data {
        // Secure JSON parsing with proper error handling
        let data_value = match serde_json::from_str(data) {
            Ok(parsed_json) => {
                // Successfully parsed as JSON - validate it's not malicious
                match &parsed_json {
                    serde_json::Value::String(s) => {
                        // If it's a JSON string, validate the content
                        if let Err(err) = validate_data_field(s) {
                            tracing::warn!("Malicious content in JSON string from {}: {}", node_execution_id, err);
                            return Err((
                                StatusCode::BAD_REQUEST,
                                Json(HilErrorResponse {
                                    status: "error".to_string(),
                                    error: format!("Invalid JSON string content: {err}"),
                                }),
                            ));
                        }
                        parsed_json
                    },
                    serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
                        // Complex JSON structures - ensure they don't exceed size limits
                        let serialized_size = serde_json::to_string(&parsed_json)
                            .map(|s| s.len())
                            .unwrap_or(data.len());

                        if serialized_size > MAX_DATA_LENGTH {
                            tracing::warn!("JSON data too large from {}: {} bytes", node_execution_id, serialized_size);
                            return Err((
                                StatusCode::BAD_REQUEST,
                                Json(HilErrorResponse {
                                    status: "error".to_string(),
                                    error: format!("JSON data too large ({serialized_size} bytes). Maximum: {MAX_DATA_LENGTH} bytes"),
                                }),
                            ));
                        }
                        parsed_json
                    },
                    _ => parsed_json, // Numbers, booleans, null are safe
                }
            },
            Err(parse_error) => {
                // JSON parsing failed - treat as plain string (already validated above)
                tracing::debug!("Data field is not valid JSON from {}, treating as string: {}",
                              node_execution_id, parse_error);
                serde_json::Value::String(data.clone())
            }
        };
        response_data.insert("data".to_string(), data_value);
    }

    // Update the task with response
    let mut task_active: human_in_loop_tasks::ActiveModel = task.clone().into();
    task_active.status = Set(params.decision.clone());
    task_active.response_data = Set(Some(serde_json::Value::Object(response_data.clone())));
    let now_microseconds = Utc::now().timestamp_micros();
    task_active.response_received_at = Set(Some(now_microseconds));
    task_active.updated_at = Set(now_microseconds);

    let updated_task = task_active.update(state.db.as_ref()).await
        .map_err(|e| {
            tracing::error!("Database error updating HIL task: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(HilErrorResponse {
                    status: "error".to_string(),
                    error: "Failed to update task".to_string(),
                }),
            )
        })?;

    // Audit logging of human response
    tracing::info!(
        "HIL_AUDIT: Human response recorded - task_id: {}, node_execution_id: {}, decision: {}, \
        response_time: {}, task_title: '{}', has_comments: {}, has_data: {}",
        task.id,
        node_execution_id,
        params.decision,
        updated_task.response_received_at.map(|micros| {
            DateTime::from_timestamp_micros(micros).unwrap_or_default().to_rfc3339()
        }).unwrap_or("unknown".to_string()),
        task.title,
        params.comments.is_some(),
        params.data.is_some()
    );

    tracing::info!("HIL task {} updated with decision: {}", task.id, params.decision);

    // Create HIL resumption job in database job queue for background worker processing
    let hil_response = crate::hil::HilResponse {
        decision: params.decision.clone(),
        response_data: Some(serde_json::Value::Object(response_data)),
        task_id: task.id.clone(),
    };

    // Create HIL resumption payload for job queue
    let resumption_payload = crate::workflow::models::HilResumptionPayload {
        node_execution_id: node_execution_id.to_string(),
        hil_response,
        resume_path: params.decision.clone(),
    };

    // Create job payload with type identifier for job routing
    let job_payload = serde_json::json!({
        "type": "hil_resumption",
        "payload": resumption_payload
    });

    // Serialize payload for job queue storage
    let payload_json = serde_json::to_string(&job_payload)
        .map_err(|e| {
            tracing::error!("Failed to serialize HIL resumption job payload: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(HilErrorResponse {
                    status: "error".to_string(),
                    error: "Failed to create resumption job".to_string(),
                }),
            )
        })?;

    // Create HIL resumption job in job queue table
    let job_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp_micros();

    let hil_job = crate::database::job_queue::ActiveModel {
        id: Set(job_id.clone()),
        execution_id: Set(task.execution_id.clone()), // Use actual workflow execution ID from HIL task
        priority: Set(10), // High priority for HIL resumption
        scheduled_at: Set(now),
        claimed_at: Set(None),
        claimed_by: Set(None),
        max_retries: Set(3),
        retry_count: Set(0),
        status: Set(crate::database::job_queue::JobStatus::Pending.to_string()),
        error_message: Set(None),
        payload: Set(Some(payload_json)), // HIL resumption payload
        created_at: Set(now),
        updated_at: Set(now),
    };

    hil_job.insert(state.db.as_ref()).await
        .map_err(|e| {
            tracing::error!("Failed to create HIL resumption job: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(HilErrorResponse {
                    status: "error".to_string(),
                    error: "Failed to create resumption job".to_string(),
                }),
            )
        })?;

    tracing::info!("HIL resumption job {} created for task {}: {} - background workers will process resumption",
                   job_id, task.id, params.decision);

    Ok(Json(HilResponseSuccess {
        status: "success".to_string(),
        message: "Response recorded successfully".to_string(),
        task: HilTaskSummary {
            id: updated_task.id,
            title: updated_task.title,
            status: updated_task.status,
            response_received_at: updated_task.response_received_at,
        },
    }))
}

/// List HIL tasks (admin endpoint) with optional status filtering
pub async fn list_hil_tasks(
    State(state): State<AppState>,
    Query(query): Query<HilTasksQuery>,
) -> Result<Json<Vec<human_in_loop_tasks::Model>>, (StatusCode, Json<HilErrorResponse>)> {
    let mut finder = human_in_loop_tasks::Entity::find();

    // Apply status filter if provided, default to pending
    let status = query.status.as_deref().unwrap_or("pending");
    finder = finder.filter(human_in_loop_tasks::Column::Status.eq(status));

    let tasks = finder
        .all(state.db.as_ref())
        .await
        .map_err(|e| {
            tracing::error!("Database error listing HIL tasks: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(HilErrorResponse {
                    status: "error".to_string(),
                    error: "Database error".to_string(),
                }),
            )
        })?;

    Ok(Json(tasks))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/:node_execution_id/respond", get(respond_to_hil_task))
        .route("/tasks", get(list_hil_tasks))
}