use std::collections::HashSet;
use uuid::Uuid;
use super::types::{CreateWorkflowRequest, EdgeRequest, NodeRequest};
use crate::database::nodes;
use crate::workflow::{
    models::{Edge, Node, NodeType, InputMergeStrategy, RetryConfig, FailureAction, HttpMethod},
    validation::WorkflowValidator,
    errors::SwissPipeError,
};

/// Validate that a string is a valid UUID
pub fn is_valid_uuid(id: &str) -> bool {
    Uuid::parse_str(id).is_ok()
}

/// Convert API NodeRequest to workflow Node model
fn convert_node_request_to_node(
    node_request: &NodeRequest,
    workflow_id: &str,
    node_id: &str,
) -> Node {
    Node {
        id: node_id.to_string(),
        workflow_id: workflow_id.to_string(),
        name: node_request.name.clone(),
        node_type: node_request.node_type.clone(),
        input_merge_strategy: None, // Default for API requests
    }
}

/// Convert existing database node to workflow Node model
fn convert_db_node_to_node(db_node: &nodes::Model) -> Result<Node, String> {
    tracing::debug!(
        "Converting database node to workflow node: id={}, name='{}', node_type_json='{}'",
        db_node.id, db_node.name, db_node.node_type
    );

    // Check for empty or invalid JSON before parsing
    if db_node.node_type.trim().is_empty() {
        return Err(format!(
            "Node {} ('{}') has empty node_type JSON - database corruption detected",
            db_node.id, db_node.name
        ));
    }

    // Parse the JSON stored node_type with fallback for legacy data
    let node_type: NodeType = match serde_json::from_str(&db_node.node_type) {
        Ok(node_type) => node_type,
        Err(parse_error) => {
            tracing::warn!(
                "Failed to parse node_type as JSON for node {} ('{}'): {} - attempting legacy format conversion",
                db_node.id, db_node.name, parse_error
            );

            // Attempt to handle legacy string format
            match convert_legacy_node_type(&db_node.node_type) {
                Ok(legacy_node_type) => {
                    tracing::info!(
                        "Successfully converted legacy node_type '{}' for node {} ('{}')",
                        db_node.node_type, db_node.id, db_node.name
                    );
                    legacy_node_type
                }
                Err(legacy_error) => {
                    tracing::error!(
                        "Both JSON parsing and legacy conversion failed for node {}: node_type='{}', json_error={}, legacy_error={}",
                        db_node.id, db_node.node_type, parse_error, legacy_error
                    );
                    return Err(format!(
                        "Failed to parse node_type for node {} ('{}'): {} - JSON content: '{}' - Legacy conversion also failed: {}",
                        db_node.id, db_node.name, parse_error, db_node.node_type, legacy_error
                    ));
                }
            }
        }
    };

    // Parse the JSON stored input_merge_strategy if present
    let input_merge_strategy: Option<InputMergeStrategy> = match &db_node.input_merge_strategy {
        Some(json_str) => Some(serde_json::from_str(json_str)
            .map_err(|e| format!("Failed to parse input_merge_strategy for node {}: {}", db_node.id, e))?),
        None => None,
    };

    Ok(Node {
        id: db_node.id.clone(),
        workflow_id: db_node.workflow_id.clone(),
        name: db_node.name.clone(),
        node_type,
        input_merge_strategy,
    })
}

/// Convert API EdgeRequest to workflow Edge model
fn convert_edge_request_to_edge(
    edge_request: &EdgeRequest,
    workflow_id: &str,
    edge_id: &str,
) -> Edge {
    Edge {
        id: edge_id.to_string(),
        workflow_id: workflow_id.to_string(),
        from_node_id: edge_request.from_node_id.clone(),
        to_node_id: edge_request.to_node_id.clone(),
        condition_result: edge_request.condition_result,
    }
}


/// Comprehensive workflow update validation using core WorkflowValidator
pub fn validate_workflow_update_request(
    request: &CreateWorkflowRequest,
    start_node_id: &str,
    existing_nodes: &[nodes::Model]
) -> Result<(), String> {
    tracing::debug!(
        "Starting comprehensive workflow update validation: workflow_name='{}', request_nodes={}, request_edges={}, existing_nodes={}",
        request.name, request.nodes.len(), request.edges.len(), existing_nodes.len()
    );

    // Step 1: Basic input validation
    validate_basic_inputs(request)?;

    // Step 2: Build complete node and edge sets for validation
    let (all_nodes, all_edges) = build_complete_workflow_model(
        request,
        existing_nodes,
        start_node_id
    )?;

    // Step 3: Use core WorkflowValidator for comprehensive validation
    match WorkflowValidator::validate_workflow(
        &request.name,
        start_node_id,
        &all_nodes,
        &all_edges,
    ) {
        Ok(()) => {
            tracing::debug!(
                "Comprehensive validation completed successfully: workflow_name='{}', total_nodes={}, total_edges={}",
                request.name, all_nodes.len(), all_edges.len()
            );
            Ok(())
        }
        Err(SwissPipeError::Config(msg)) => {
            tracing::warn!(
                "Workflow validation failed: workflow_name='{}', error='{}'",
                request.name, msg
            );
            Err(msg)
        }
        Err(e) => {
            tracing::error!(
                "Unexpected validation error: workflow_name='{}', error='{:?}'",
                request.name, e
            );
            Err(format!("Validation error: {e}"))
        }
    }
}

/// Validate basic input constraints
fn validate_basic_inputs(request: &CreateWorkflowRequest) -> Result<(), String> {
    // Validate workflow name
    if request.name.trim().is_empty() {
        return Err("Workflow name cannot be empty".to_string());
    }

    if request.name.len() > 255 {
        return Err(format!("Workflow name too long: '{}' ({} characters, max 255)",
                          request.name, request.name.len()));
    }

    // Validate request nodes
    for node in &request.nodes {
        // Validate node ID format if provided
        if let Some(node_id) = &node.id {
            if !is_valid_uuid(node_id) {
                return Err(format!("Invalid UUID format for node ID: {node_id}"));
            }
        }

        // Validate node name is not empty
        if node.name.trim().is_empty() {
            return Err(format!("Node name cannot be empty (node_id: {:?})", node.id));
        }

        // Validate node name length (reasonable limit)
        if node.name.len() > 255 {
            return Err(format!("Node name too long: '{}' ({} characters, max 255, node_id: {:?})",
                              node.name, node.name.len(), node.id));
        }
    }

    // Validate edge node ID formats
    for edge in &request.edges {
        if !is_valid_uuid(&edge.from_node_id) {
            return Err(format!("Invalid UUID format for edge from_node_id: {}", edge.from_node_id));
        }
        if !is_valid_uuid(&edge.to_node_id) {
            return Err(format!("Invalid UUID format for edge to_node_id: {}", edge.to_node_id));
        }
    }

    Ok(())
}

/// Build complete workflow model for validation (existing + request nodes/edges)
fn build_complete_workflow_model(
    request: &CreateWorkflowRequest,
    existing_nodes: &[nodes::Model],
    start_node_id: &str,
) -> Result<(Vec<Node>, Vec<Edge>), String> {
    let workflow_id = "temp_validation_id"; // Temporary ID for validation

    // Build complete node set
    let mut all_nodes = Vec::new();
    let mut processed_node_ids = HashSet::new();

    // Add/update nodes from request
    for node_request in &request.nodes {
        if let Some(node_id) = &node_request.id {
            let workflow_node = convert_node_request_to_node(
                node_request,
                workflow_id,
                node_id
            );
            all_nodes.push(workflow_node);
            processed_node_ids.insert(node_id.clone());
        } else {
            // Generate temp ID for validation of nodes without IDs
            let temp_id = Uuid::new_v4().to_string();
            let workflow_node = convert_node_request_to_node(
                node_request,
                workflow_id,
                &temp_id
            );
            all_nodes.push(workflow_node);
            processed_node_ids.insert(temp_id);
        }
    }

    // Add remaining existing nodes (not being updated)
    tracing::debug!(
        "Processing existing nodes: total_existing={}, nodes_in_request={}",
        existing_nodes.len(), processed_node_ids.len()
    );

    for existing_node in existing_nodes {
        if !processed_node_ids.contains(&existing_node.id) {
            tracing::debug!(
                "Adding existing node to validation: id={}, name='{}'",
                existing_node.id, existing_node.name
            );
            match convert_db_node_to_node(existing_node) {
                Ok(workflow_node) => {
                    all_nodes.push(workflow_node);
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to convert existing node {}: {}",
                        existing_node.id, e
                    );
                    return Err(format!(
                        "Database integrity issue with existing node {} ('{}'): {}",
                        existing_node.id, existing_node.name, e
                    ));
                }
            }
        } else {
            tracing::debug!(
                "Skipping existing node {} - being updated in request",
                existing_node.id
            );
        }
    }

    // Build complete edge set
    let mut all_edges = Vec::new();
    for (i, edge_request) in request.edges.iter().enumerate() {
        let edge_id = format!("temp_edge_{i}"); // Temporary ID for validation
        let workflow_edge = convert_edge_request_to_edge(
            edge_request,
            workflow_id,
            &edge_id
        );
        all_edges.push(workflow_edge);
    }

    // Validate that start_node_id exists in the final node set
    let node_exists = all_nodes.iter().any(|n| n.id == start_node_id);
    if !node_exists {
        return Err(format!(
            "Start node with ID '{start_node_id}' not found in workflow nodes"
        ));
    }

    tracing::debug!(
        "Built complete workflow model: total_nodes={}, total_edges={}, start_node_id={}",
        all_nodes.len(), all_edges.len(), start_node_id
    );

    Ok((all_nodes, all_edges))
}

/// Convert legacy string node_type values to proper NodeType structures
/// This handles database corruption where simple strings were stored instead of JSON
fn convert_legacy_node_type(legacy_value: &str) -> Result<NodeType, String> {
    let trimmed = legacy_value.trim().to_lowercase();

    match trimmed.as_str() {
        "trigger" => Ok(NodeType::Trigger {
            methods: vec![HttpMethod::Post], // Default to POST for legacy triggers
        }),
        "condition" => Ok(NodeType::Condition {
            script: "function condition(event) { return true; }".to_string(), // Default script
        }),
        "transformer" => Ok(NodeType::Transformer {
            script: "function transformer(event) { return event; }".to_string(), // Default script
        }),
        "httprequest" | "http_request" | "http-request" => Ok(NodeType::HttpRequest {
            url: "https://httpbin.org/post".to_string(), // Default URL
            method: HttpMethod::Post,
            timeout_seconds: 30,
            failure_action: FailureAction::Stop,
            retry_config: RetryConfig {
                max_attempts: 3,
                initial_delay_ms: 100,
                max_delay_ms: 5000,
                backoff_multiplier: 2.0,
            },
            headers: std::collections::HashMap::new(),
        }),
        "openobserve" => Ok(NodeType::OpenObserve {
            url: "".to_string(), // Will need to be configured
            authorization_header: "".to_string(),
            timeout_seconds: 30,
            failure_action: FailureAction::Stop,
            retry_config: RetryConfig {
                max_attempts: 3,
                initial_delay_ms: 100,
                max_delay_ms: 5000,
                backoff_multiplier: 2.0,
            },
        }),
        _ => Err(format!("Unrecognized legacy node_type: '{}'", legacy_value)),
    }
}