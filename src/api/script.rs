use axum::{extract::State, http::StatusCode, Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::utils::javascript::JavaScriptExecutor;
use crate::workflow::{models::WorkflowEvent, errors::JavaScriptError};
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct ScriptExecuteRequest {
    pub script: String,
    pub input: Value,
    pub script_type: Option<String>, // "transformer" or "condition"
}


#[derive(Debug, Serialize)]
pub struct ScriptExecuteError {
    pub error: String,
    pub details: Option<String>,
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/execute", axum::routing::post(execute_script))
}

/// Execute a script (transformer or condition) with input data for testing purposes.
///
/// This endpoint accepts either:
/// - Raw input data (will be wrapped in a WorkflowEvent)
/// - Complete WorkflowEvent structure (will be used directly)
///
/// The script_type parameter determines how the script is executed:
/// - "transformer": Executes as transformer script, returns WorkflowEvent
/// - "condition": Executes as condition script, returns boolean result
/// - If not specified, defaults to "transformer" for backward compatibility
///
/// Returns the script result or error details.
pub async fn execute_script(
    State(_state): State<AppState>,
    Json(request): Json<ScriptExecuteRequest>,
) -> Result<Json<Value>, (StatusCode, Json<ScriptExecuteError>)> {
    // Validate script is not empty
    if request.script.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ScriptExecuteError {
                error: "Script cannot be empty".to_string(),
                details: None,
            }),
        ));
    }

    // Try to deserialize input as WorkflowEvent, otherwise wrap it
    let workflow_event = match serde_json::from_value::<WorkflowEvent>(request.input.clone()) {
        Ok(event) => {
            tracing::info!("Input parsed as WorkflowEvent directly");
            event
        },
        Err(e) => {
            tracing::info!("Input not a WorkflowEvent, wrapping: {}", e);
            create_workflow_event(request.input)
        },
    };

    // Create JavaScript executor
    let js_executor = match JavaScriptExecutor::new() {
        Ok(executor) => executor,
        Err(error) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ScriptExecuteError {
                    error: "Failed to create JavaScript executor".to_string(),
                    details: Some(error.to_string()),
                }),
            ));
        }
    };

    // Determine script type - default to "transformer" for backward compatibility
    let script_type = request.script_type.as_deref().unwrap_or("transformer");

    match script_type {
        "condition" => {
            // Execute as condition script
            match js_executor.execute_condition(&request.script, &workflow_event).await {
                Ok(condition_result) => {
                    // Return boolean result
                    Ok(Json(serde_json::json!(condition_result)))
                },
                Err(error) => {
                    let error_message = error.to_string();
                    Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ScriptExecuteError {
                            error: "Condition script execution failed".to_string(),
                            details: Some(error_message),
                        }),
                    ))
                }
            }
        },
        "transformer" => {
            // Execute as transformer script
            match js_executor.execute_transformer(&request.script, workflow_event).await {
                Ok(result_event) => {
                    // Convert the result event back to JSON
                    let result_json = serde_json::to_value(result_event)
                        .map_err(|e| {
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(ScriptExecuteError {
                                    error: "Failed to serialize result".to_string(),
                                    details: Some(e.to_string()),
                                }),
                            )
                        })?;

                    Ok(Json(result_json))
                },
                Err(error) => {
                    let error_message = error.to_string();

                    // Check if it's an event dropped error (transformer returned null)
                    if matches!(error, JavaScriptError::EventDropped) {
                        Ok(Json(serde_json::json!({ "dropped": true, "message": "Event was dropped (transformer returned null)" })))
                    } else {
                        Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ScriptExecuteError {
                                error: "Transformer script execution failed".to_string(),
                                details: Some(error_message),
                            }),
                        ))
                    }
                }
            }
        },
        _ => {
            // Invalid script type
            Err((
                StatusCode::BAD_REQUEST,
                Json(ScriptExecuteError {
                    error: "Invalid script_type".to_string(),
                    details: Some("script_type must be either 'transformer' or 'condition'".to_string()),
                }),
            ))
        }
    }
}

fn create_workflow_event(input_data: Value) -> WorkflowEvent {
    // Create a workflow event structure similar to what's used in actual workflows
    WorkflowEvent {
        data: input_data,
        metadata: HashMap::new(),
        headers: HashMap::new(),
        condition_results: HashMap::new(),
    }
}