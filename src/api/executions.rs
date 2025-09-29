use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde::Deserialize;
use serde_json::Value;

use crate::{
    async_execution::{ExecutionService, CleanupService},
    AppState,
};

#[derive(Deserialize)]
pub struct GetExecutionsQuery {
    pub limit: Option<u64>,
    pub offset: Option<u64>,
    pub workflow_id: Option<String>,
    pub status: Option<String>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(get_all_executions))
        .route("/:execution_id", get(get_execution))
        .route("/:execution_id/status", get(get_execution_status))
        .route("/:execution_id/steps", get(get_execution_steps))
        .route("/:execution_id/cancel", axum::routing::post(cancel_execution))
        .route("/stats", get(get_worker_pool_stats))
        .route("/cleanup/stats", get(get_cleanup_stats))
}

/// Get execution details by ID
pub async fn get_execution(
    State(state): State<AppState>,
    Path(execution_id): Path<String>,
) -> std::result::Result<Json<Value>, StatusCode> {
    let execution_service = ExecutionService::new(state.db.clone());
    
    let execution = execution_service
        .get_execution(&execution_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get execution: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    match execution {
        Some(exec) => {
            let response = serde_json::json!({
                "id": exec.id,
                "workflow_id": exec.workflow_id,
                "status": exec.status,
                "current_node_id": exec.current_node_id,
                "input_data": exec.input_data.and_then(|d| serde_json::from_str::<Value>(&d).ok()),
                "output_data": exec.output_data.and_then(|d| serde_json::from_str::<Value>(&d).ok()),
                "error_message": exec.error_message,
                "started_at": exec.started_at,
                "completed_at": exec.completed_at,
                "created_at": exec.created_at,
                "updated_at": exec.updated_at
            });
            Ok(Json(response))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Get execution steps by execution ID
pub async fn get_execution_steps(
    State(state): State<AppState>,
    Path(execution_id): Path<String>,
) -> std::result::Result<Json<Value>, StatusCode> {
    let execution_service = ExecutionService::new(state.db.clone());
    
    let steps = execution_service
        .get_execution_steps(&execution_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get execution steps: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let steps_json: Vec<Value> = steps
        .into_iter()
        .map(|step| {
            serde_json::json!({
                "id": step.id,
                "execution_id": step.execution_id,
                "node_id": step.node_id,
                "node_name": step.node_name,
                "status": step.status,
                "input_data": step.input_data.and_then(|d| serde_json::from_str::<Value>(&d).ok()),
                "output_data": step.output_data.and_then(|d| serde_json::from_str::<Value>(&d).ok()),
                "error_message": step.error_message,
                "started_at": step.started_at,
                "completed_at": step.completed_at,
                "created_at": step.created_at
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "execution_id": execution_id,
        "steps": steps_json
    })))
}


/// Cancel an execution comprehensively including scheduled delays
pub async fn cancel_execution(
    State(state): State<AppState>,
    Path(execution_id): Path<String>,
) -> std::result::Result<Json<Value>, StatusCode> {
    // Use comprehensive cancellation through WorkerPool to include delay cancellation
    state.worker_pool
        .cancel_execution_with_delays(&execution_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to cancel execution: {}", e);
            match e {
                crate::workflow::errors::SwissPipeError::WorkflowNotFound(_) => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

    Ok(Json(serde_json::json!({
        "status": "cancelled",
        "execution_id": execution_id,
        "message": "Execution cancelled successfully"
    })))
}

/// Get worker pool statistics and monitoring information
pub async fn get_worker_pool_stats(
    State(state): State<AppState>,
) -> std::result::Result<Json<Value>, StatusCode> {
    // Get worker pool statistics
    let worker_stats = state.worker_pool.get_stats().await;

    // Get MPSC distributor metrics
    let mpsc_metrics = state.mpsc_distributor.get_metrics().await;

    // Get additional metrics
    let system_info = serde_json::json!({
        "timestamp": chrono::Utc::now().timestamp_micros(),
        "version": env!("CARGO_PKG_VERSION"),
        "build_profile": if cfg!(debug_assertions) { "debug" } else { "release" }
    });

    let response = serde_json::json!({
        "worker_pool": worker_stats,
        "mpsc_distributor": mpsc_metrics,
        "system": system_info,
        "health": "healthy"
    });

    Ok(Json(response))
}

/// Get execution status (lightweight version)
pub async fn get_execution_status(
    State(state): State<AppState>,
    Path(execution_id): Path<String>,
) -> std::result::Result<Json<Value>, StatusCode> {
    let execution_service = ExecutionService::new(state.db.clone());
    
    let execution = execution_service
        .get_execution(&execution_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get execution: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    match execution {
        Some(exec) => {
            let response = serde_json::json!({
                "id": exec.id,
                "workflow_id": exec.workflow_id,
                "status": exec.status,
                "current_node_id": exec.current_node_id,
                "error_message": exec.error_message,
                "started_at": exec.started_at,
                "completed_at": exec.completed_at,
                "created_at": exec.created_at,
                "updated_at": exec.updated_at
            });
            Ok(Json(response))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}


/// Get all recent executions across workflows
pub async fn get_all_executions(
    State(state): State<AppState>,
    Query(params): Query<GetExecutionsQuery>,
) -> std::result::Result<Json<Value>, StatusCode> {
    let execution_service = ExecutionService::new(state.db.clone());
    
    let workflow_id_clone = params.workflow_id.clone();
    let status_clone = params.status.clone();
    let executions = if let Some(ref workflow_id) = params.workflow_id {
        // Filter by specific workflow ID
        execution_service
            .get_executions_by_workflow_filtered(
                workflow_id, 
                params.status.as_deref(), 
                params.limit, 
                params.offset
            )
            .await
            .map_err(|e| {
                tracing::error!("Failed to get executions by workflow: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
    } else {
        // Get all recent executions
        execution_service
            .get_recent_executions_filtered(
                params.status.as_deref(), 
                params.limit, 
                params.offset
            )
            .await
            .map_err(|e| {
                tracing::error!("Failed to get all executions: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
    };

    let executions_json: Vec<Value> = executions
        .into_iter()
        .map(|exec| {
            let duration_ms = if let (Some(started), Some(completed)) = (exec.started_at, exec.completed_at) {
                Some((completed - started) / 1000) // Convert microseconds to milliseconds
            } else {
                None
            };

            serde_json::json!({
                "id": exec.id,
                "workflow_id": exec.workflow_id,
                "status": exec.status,
                "current_node_id": exec.current_node_id,
                "input_data": exec.input_data.and_then(|d| serde_json::from_str::<Value>(&d).ok()),
                "output_data": exec.output_data.and_then(|d| serde_json::from_str::<Value>(&d).ok()),
                "error_message": exec.error_message,
                "started_at": exec.started_at,
                "completed_at": exec.completed_at,
                "duration_ms": duration_ms,
                "created_at": exec.created_at,
                "updated_at": exec.updated_at
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "executions": executions_json,
        "count": executions_json.len(),
        "workflow_id": workflow_id_clone,
        "status": status_clone
    })))
}

/// Get cleanup service statistics
pub async fn get_cleanup_stats(
    State(state): State<AppState>,
) -> std::result::Result<Json<Value>, StatusCode> {
    let cleanup_service = match CleanupService::new(
        state.db.clone(),
        state.config.execution_retention_count,
        state.config.cleanup_interval_minutes,
    ) {
        Ok(service) => service,
        Err(e) => {
            tracing::error!("Failed to initialize cleanup service: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let stats = cleanup_service
        .get_cleanup_stats()
        .await
        .map_err(|e| {
            tracing::error!("Failed to get cleanup stats: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let response = serde_json::json!({
        "cleanup_stats": stats,
        "config": {
            "retention_count": state.config.execution_retention_count,
            "cleanup_interval_minutes": state.config.cleanup_interval_minutes
        },
        "timestamp": chrono::Utc::now().timestamp_micros()
    });

    Ok(Json(response))
}