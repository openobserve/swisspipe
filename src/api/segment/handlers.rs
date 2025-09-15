use axum::{
    extract::{State, Path},
    http::{HeaderMap, StatusCode, Extensions},
    response::Json,
};
use futures::future::join_all;
use serde::Serialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    api::ingestion::trigger_workflow_post,
    AppState,
};

use super::middleware::extract_write_key_from_request;

#[derive(Serialize)]
pub struct SegmentResponse {
    pub success: bool,
    #[serde(rename = "messageId", skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<SegmentErrorDetails>,
}

#[derive(Serialize)]
pub struct SegmentErrorDetails {
    pub error_type: String,
    pub workflow_id: Option<String>,
    pub event_count: Option<usize>,
    pub failed_events: Option<Vec<usize>>,
    pub validation_errors: Option<Vec<String>>,
}

/// Convert Segment.com request to SwissPipe format and trigger workflow
async fn handle_segment_request(
    state: State<AppState>,
    headers: HeaderMap,
    extensions: Extensions,
    body_value: Value,
    segment_type: &str,
) -> Result<Json<SegmentResponse>, StatusCode> {
    // Extract write key (workflow UUID) using middleware or fallback
    let workflow_id = match extract_write_key_from_request(&extensions, &body_value) {
        Ok(write_key) => write_key,
        Err(error) => {
            tracing::warn!("Segment API auth failed: {}", error);
            return Ok(Json(SegmentResponse {
                success: false,
                message_id: None,
                error: Some("Authentication failed".to_string()),
                details: Some(SegmentErrorDetails {
                    error_type: "auth_error".to_string(),
                    workflow_id: None,
                    event_count: None,
                    failed_events: None,
                    validation_errors: Some(vec![error.to_string()]),
                }),
            }));
        }
    };

    // Transform Segment request to SwissPipe format
    let mut swissipe_data = body_value.clone();

    // Add metadata about the segment event type
    if let Some(obj) = swissipe_data.as_object_mut() {
        obj.insert("_segment_type".to_string(), json!(segment_type));
        obj.insert("_segment_original".to_string(), body_value.clone());

        // Add messageId if not present
        if !obj.contains_key("messageId") {
            obj.insert("messageId".to_string(), json!(Uuid::new_v4().to_string()));
        }
    }

    // Call existing workflow trigger handler directly
    match trigger_workflow_post(
        state,
        Path(workflow_id.clone()),
        headers,
        Json(swissipe_data.clone()),
    ).await {
        Ok((status_code, _response)) => {
            if status_code == StatusCode::ACCEPTED || status_code == StatusCode::OK {
                let message_id = swissipe_data.get("messageId")
                    .and_then(|v| v.as_str())
                    .unwrap_or_else(|| "unknown")
                    .to_string();

                Ok(Json(SegmentResponse {
                    success: true,
                    message_id: Some(message_id),
                    error: None,
                    details: None,
                }))
            } else {
                Ok(Json(SegmentResponse {
                    success: false,
                    message_id: None,
                    error: Some("Workflow execution failed".to_string()),
                    details: Some(SegmentErrorDetails {
                        error_type: "execution_error".to_string(),
                        workflow_id: Some(workflow_id),
                        event_count: Some(1),
                        failed_events: Some(vec![0]),
                        validation_errors: None,
                    }),
                }))
            }
        }
        Err(status_code) => {
            let (error_message, error_type) = match status_code {
                StatusCode::NOT_FOUND => ("Workflow not found".to_string(), "workflow_not_found"),
                StatusCode::UNAUTHORIZED => ("Unauthorized".to_string(), "unauthorized"),
                StatusCode::BAD_REQUEST => ("Invalid request format".to_string(), "invalid_request"),
                StatusCode::INTERNAL_SERVER_ERROR => ("Internal server error".to_string(), "internal_error"),
                _ => ("Unknown error".to_string(), "unknown_error"),
            };

            Ok(Json(SegmentResponse {
                success: false,
                message_id: None,
                error: Some(error_message),
                details: Some(SegmentErrorDetails {
                    error_type: error_type.to_string(),
                    workflow_id: Some(workflow_id),
                    event_count: Some(1),
                    failed_events: Some(vec![0]),
                    validation_errors: None,
                }),
            }))
        }
    }
}

pub async fn segment_track(
    state: State<AppState>,
    headers: HeaderMap,
    extensions: Extensions,
    Json(data): Json<Value>,
) -> Result<Json<SegmentResponse>, StatusCode> {
    handle_segment_request(state, headers, extensions, data, "track").await
}

pub async fn segment_identify(
    state: State<AppState>,
    headers: HeaderMap,
    extensions: Extensions,
    Json(data): Json<Value>,
) -> Result<Json<SegmentResponse>, StatusCode> {
    handle_segment_request(state, headers, extensions, data, "identify").await
}

pub async fn segment_page(
    state: State<AppState>,
    headers: HeaderMap,
    extensions: Extensions,
    Json(data): Json<Value>,
) -> Result<Json<SegmentResponse>, StatusCode> {
    handle_segment_request(state, headers, extensions, data, "page").await
}

pub async fn segment_screen(
    state: State<AppState>,
    headers: HeaderMap,
    extensions: Extensions,
    Json(data): Json<Value>,
) -> Result<Json<SegmentResponse>, StatusCode> {
    handle_segment_request(state, headers, extensions, data, "screen").await
}

pub async fn segment_group(
    state: State<AppState>,
    headers: HeaderMap,
    extensions: Extensions,
    Json(data): Json<Value>,
) -> Result<Json<SegmentResponse>, StatusCode> {
    handle_segment_request(state, headers, extensions, data, "group").await
}

pub async fn segment_alias(
    state: State<AppState>,
    headers: HeaderMap,
    extensions: Extensions,
    Json(data): Json<Value>,
) -> Result<Json<SegmentResponse>, StatusCode> {
    handle_segment_request(state, headers, extensions, data, "alias").await
}

pub async fn segment_batch(
    state: State<AppState>,
    headers: HeaderMap,
    extensions: Extensions,
    Json(body_value): Json<Value>,
) -> Result<Json<SegmentResponse>, StatusCode> {
    // Extract write key using middleware
    let workflow_id = match extract_write_key_from_request(&extensions, &body_value) {
        Ok(write_key) => write_key,
        Err(error) => {
            tracing::warn!("Segment batch API auth failed: {}", error);
            return Ok(Json(SegmentResponse {
                success: false,
                message_id: None,
                error: Some("Authentication failed".to_string()),
                details: Some(SegmentErrorDetails {
                    error_type: "auth_error".to_string(),
                    workflow_id: None,
                    event_count: None,
                    failed_events: None,
                    validation_errors: Some(vec![error.to_string()]),
                }),
            }));
        }
    };

    // Process batch of events
    if let Some(batch_array) = body_value.get("batch").and_then(|b| b.as_array()) {
        let batch_message_id = Uuid::new_v4().to_string();
        let total_events = batch_array.len();

        tracing::info!("Processing batch of {} events for workflow {}", total_events, workflow_id);

        // Prepare all events with metadata concurrently
        let mut event_futures = Vec::new();

        for (index, event) in batch_array.iter().enumerate() {
            let mut event_data = event.clone();

            // Add batch metadata
            if let Some(obj) = event_data.as_object_mut() {
                let event_type = obj.get("type").and_then(|t| t.as_str()).unwrap_or("unknown");
                obj.insert("_segment_type".to_string(), json!(format!("batch_{}", event_type)));
                obj.insert("_segment_original".to_string(), event.clone());
                obj.insert("_batch_id".to_string(), json!(batch_message_id.clone()));
                obj.insert("_batch_index".to_string(), json!(index));

                if !obj.contains_key("messageId") {
                    obj.insert("messageId".to_string(), json!(Uuid::new_v4().to_string()));
                }
            }

            // Create future for concurrent execution
            let state_clone = state.clone();
            let workflow_id_clone = workflow_id.clone();
            let headers_clone = headers.clone();

            let future = async move {
                let result = trigger_workflow_post(
                    state_clone,
                    Path(workflow_id_clone),
                    headers_clone,
                    Json(event_data),
                ).await;

                (index, result)
            };

            event_futures.push(future);
        }

        // Execute all events concurrently
        let results = join_all(event_futures).await;

        // Analyze results
        let mut failed_events = Vec::new();
        let mut success_count = 0;

        for (index, result) in results {
            match result {
                Ok((status_code, _)) => {
                    if status_code == StatusCode::ACCEPTED || status_code == StatusCode::OK {
                        success_count += 1;
                    } else {
                        failed_events.push(index);
                        tracing::warn!("Batch event {} failed with status: {}", index, status_code);
                    }
                }
                Err(status_code) => {
                    failed_events.push(index);
                    tracing::error!("Batch event {} failed with error: {}", index, status_code);
                }
            }
        }

        let all_success = failed_events.is_empty();

        tracing::info!(
            "Batch processing completed: {}/{} events successful, workflow_id={}",
            success_count, total_events, workflow_id
        );

        Ok(Json(SegmentResponse {
            success: all_success,
            message_id: Some(batch_message_id),
            error: if all_success {
                None
            } else {
                Some(format!("{} out of {} events failed", failed_events.len(), total_events))
            },
            details: if all_success {
                None
            } else {
                Some(SegmentErrorDetails {
                    error_type: "batch_partial_failure".to_string(),
                    workflow_id: Some(workflow_id),
                    event_count: Some(total_events),
                    failed_events: Some(failed_events),
                    validation_errors: None,
                })
            },
        }))
    } else {
        Ok(Json(SegmentResponse {
            success: false,
            message_id: None,
            error: Some("Invalid batch format".to_string()),
            details: Some(SegmentErrorDetails {
                error_type: "invalid_batch_format".to_string(),
                workflow_id: Some(workflow_id),
                event_count: Some(0),
                failed_events: None,
                validation_errors: Some(vec!["Missing 'batch' array in request body".to_string()]),
            }),
        }))
    }
}

pub async fn segment_import(
    state: State<AppState>,
    headers: HeaderMap,
    extensions: Extensions,
    data: Json<Value>,
) -> Result<Json<SegmentResponse>, StatusCode> {
    // Import is essentially the same as batch for our purposes
    segment_batch(state, headers, extensions, data).await
}