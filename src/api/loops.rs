use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};

use crate::{
    database::http_loop_states,
    AppState,
};

#[derive(Serialize, Deserialize)]
pub struct LoopStatusResponse {
    pub loop_id: String,
    pub execution_step_id: String,
    pub current_iteration: i32,
    pub max_iterations: Option<i32>,
    pub next_execution_at: Option<i64>,
    pub consecutive_failures: i32,
    pub loop_started_at: i64,
    pub last_response_status: Option<i32>,
    pub last_response_body: Option<String>,
    pub status: String,
    pub termination_reason: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub success_rate: Option<f64>,
}

#[derive(Serialize, Deserialize)]
pub struct ExecutionLoopsResponse {
    pub execution_id: String,
    pub loops: Vec<LoopStatusResponse>,
    pub total_loops: usize,
}

#[derive(Deserialize)]
pub struct ActiveLoopsQuery {
    pub execution_id: Option<String>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/active", get(get_active_loops))
        .route("/:loop_id/status", get(get_loop_status))
        .route("/:loop_id/pause", post(pause_loop))
        .route("/:loop_id/resume", post(resume_loop))
        .route("/:loop_id/cancel", post(cancel_loop))
        .route("/execution/:execution_id/loops", get(get_execution_loops))
}

/// Get all active loops, optionally filtered by execution_id
pub async fn get_active_loops(
    State(state): State<AppState>,
    Query(query): Query<ActiveLoopsQuery>,
) -> std::result::Result<Json<Vec<LoopStatusResponse>>, StatusCode> {
    use sea_orm::{EntityTrait, ColumnTrait, QueryFilter};

    // Build base query for active loop states (running, paused)
    let mut query_builder = http_loop_states::Entity::find()
        .filter(http_loop_states::Column::Status.is_in(["running", "paused"]));

    // If execution_id is provided, filter by it
    if let Some(execution_id) = query.execution_id {
        query_builder = query_builder
            .filter(http_loop_states::Column::ExecutionStepId.starts_with(&execution_id));
    }

    let loop_states = query_builder
        .all(state.db.as_ref())
        .await
        .map_err(|e| {
            tracing::error!("Failed to query active loop states: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let loops: Vec<LoopStatusResponse> = loop_states
        .into_iter()
        .map(|loop_state| {
            let success_rate = loop_state.calculate_success_rate();
            let success_rate = if success_rate > 0.0 { Some(success_rate) } else { None };

            LoopStatusResponse {
                loop_id: loop_state.id,
                execution_step_id: loop_state.execution_step_id,
                current_iteration: loop_state.current_iteration,
                max_iterations: loop_state.max_iterations,
                next_execution_at: loop_state.next_execution_at,
                consecutive_failures: loop_state.consecutive_failures,
                loop_started_at: loop_state.loop_started_at,
                last_response_status: loop_state.last_response_status,
                last_response_body: loop_state.last_response_body,
                status: loop_state.status,
                termination_reason: loop_state.termination_reason,
                created_at: loop_state.created_at,
                updated_at: loop_state.updated_at,
                success_rate,
            }
        })
        .collect();

    Ok(Json(loops))
}

/// Get loop status by loop ID
pub async fn get_loop_status(
    State(state): State<AppState>,
    Path(loop_id): Path<String>,
) -> std::result::Result<Json<LoopStatusResponse>, StatusCode> {
    let loop_state = state.http_loop_scheduler
        .get_loop_status(&loop_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get loop status: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Calculate success rate if we have iteration history
    let success_rate = loop_state.calculate_success_rate();
    let success_rate = if success_rate > 0.0 { Some(success_rate) } else { None };

    let response = LoopStatusResponse {
        loop_id: loop_state.id,
        execution_step_id: loop_state.execution_step_id,
        current_iteration: loop_state.current_iteration,
        max_iterations: loop_state.max_iterations,
        next_execution_at: loop_state.next_execution_at,
        consecutive_failures: loop_state.consecutive_failures,
        loop_started_at: loop_state.loop_started_at,
        last_response_status: loop_state.last_response_status,
        last_response_body: loop_state.last_response_body,
        status: loop_state.status,
        termination_reason: loop_state.termination_reason,
        created_at: loop_state.created_at,
        updated_at: loop_state.updated_at,
        success_rate,
    };

    Ok(Json(response))
}

/// Get all loops for a specific execution
pub async fn get_execution_loops(
    State(state): State<AppState>,
    Path(execution_id): Path<String>,
) -> std::result::Result<Json<ExecutionLoopsResponse>, StatusCode> {
    use sea_orm::{EntityTrait, ColumnTrait, QueryFilter};

    // Find all loop states for the given execution
    let loop_states = http_loop_states::Entity::find()
        .filter(http_loop_states::Column::ExecutionStepId.starts_with(&execution_id))
        .all(state.db.as_ref())
        .await
        .map_err(|e| {
            tracing::error!("Failed to query loop states: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let loops: Vec<LoopStatusResponse> = loop_states
        .into_iter()
        .map(|loop_state| {
            let success_rate = loop_state.calculate_success_rate();
            let success_rate = if success_rate > 0.0 { Some(success_rate) } else { None };

            LoopStatusResponse {
                loop_id: loop_state.id,
                execution_step_id: loop_state.execution_step_id,
                current_iteration: loop_state.current_iteration,
                max_iterations: loop_state.max_iterations,
                next_execution_at: loop_state.next_execution_at,
                consecutive_failures: loop_state.consecutive_failures,
                loop_started_at: loop_state.loop_started_at,
                last_response_status: loop_state.last_response_status,
                last_response_body: loop_state.last_response_body,
                status: loop_state.status,
                termination_reason: loop_state.termination_reason,
                created_at: loop_state.created_at,
                updated_at: loop_state.updated_at,
                success_rate,
            }
        })
        .collect();

    let total_loops = loops.len();

    let response = ExecutionLoopsResponse {
        execution_id,
        loops,
        total_loops,
    };

    Ok(Json(response))
}

/// Pause a running loop
pub async fn pause_loop(
    State(state): State<AppState>,
    Path(loop_id): Path<String>,
) -> std::result::Result<StatusCode, StatusCode> {
    state.http_loop_scheduler
        .pause_loop(&loop_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to pause loop {}: {}", loop_id, e);
            match e.to_string().contains("Cannot pause loop") {
                true => StatusCode::BAD_REQUEST,
                false => StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

    tracing::info!("Loop {} paused successfully", loop_id);
    Ok(StatusCode::OK)
}

/// Resume a paused loop
pub async fn resume_loop(
    State(state): State<AppState>,
    Path(loop_id): Path<String>,
) -> std::result::Result<StatusCode, StatusCode> {
    state.http_loop_scheduler
        .resume_loop(&loop_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to resume loop {}: {}", loop_id, e);
            match e.to_string().contains("Cannot resume loop") {
                true => StatusCode::BAD_REQUEST,
                false => StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

    tracing::info!("Loop {} resumed successfully", loop_id);
    Ok(StatusCode::OK)
}

/// Cancel a loop and its workflow execution
pub async fn cancel_loop(
    State(state): State<AppState>,
    Path(loop_id): Path<String>,
) -> std::result::Result<StatusCode, StatusCode> {
    state.http_loop_scheduler
        .cancel_loop(&loop_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to cancel loop {}: {}", loop_id, e);
            match e.to_string().contains("Cannot cancel loop") {
                true => StatusCode::BAD_REQUEST,
                false => StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

    tracing::info!("Loop {} cancelled successfully", loop_id);
    Ok(StatusCode::OK)
}