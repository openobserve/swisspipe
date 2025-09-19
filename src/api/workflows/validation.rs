use uuid::Uuid;
use super::types::CreateWorkflowRequest;
use crate::database::nodes;
use thiserror::Error;

/// Custom validation errors with structured information
#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("Invalid UUID format for {field}: {value}")]
    InvalidUuid { field: String, value: String },

    #[error("Field '{field}' cannot be empty")]
    EmptyField { field: String },

    #[error("Field '{field}' exceeds maximum length of {max_len}: {actual_len} characters")]
    FieldTooLong { field: String, max_len: usize, actual_len: usize },

    #[error("Start node '{node_id}' not found in workflow nodes")]
    StartNodeNotFound { node_id: String },

    #[error("Database integrity issue with node {node_id} ('{node_name}'): {details}")]
    DatabaseIntegrity { node_id: String, node_name: String, details: String },

    #[error("Workflow validation failed: {message}")]
    WorkflowValidation { message: String },
}

impl From<ValidationError> for String {
    fn from(err: ValidationError) -> Self {
        err.to_string()
    }
}

/// Validate that a string is a valid UUID
pub fn is_valid_uuid(id: &str) -> bool {
    Uuid::parse_str(id).is_ok()
}

/// Basic workflow update request validation
///
/// NOTE: This only validates basic input structure and format.
/// Comprehensive workflow structure validation (reachability, cycles, etc.)
/// is performed separately using the correct final node set.
pub fn validate_workflow_update_request(
    request: &CreateWorkflowRequest,
    _start_node_id: &str,
    _existing_nodes: &[nodes::Model]
) -> Result<(), ValidationError> {
    tracing::debug!(
        "Starting basic workflow update validation: workflow_name='{}', request_nodes={}, request_edges={}",
        request.name, request.nodes.len(), request.edges.len()
    );

    // Step 1: Basic input validation
    validate_basic_inputs(request)?;

    // Skip comprehensive workflow structure validation here
    // This is handled separately with the correct final node set
    // (not including deleted nodes like build_complete_workflow_model does)

    tracing::debug!(
        "Basic validation completed successfully: workflow_name='{}'",
        request.name
    );

    Ok(())
}

/// Validate basic input constraints with structured errors and batch validation
fn validate_basic_inputs(request: &CreateWorkflowRequest) -> Result<(), ValidationError> {
    // Validate workflow name
    let name = request.name.trim();
    if name.is_empty() {
        return Err(ValidationError::EmptyField { field: "workflow name".to_string() });
    }
    if name.len() > 255 {
        return Err(ValidationError::FieldTooLong {
            field: "workflow name".to_string(),
            max_len: 255,
            actual_len: name.len()
        });
    }

    // Batch validate nodes for better performance
    for (index, node) in request.nodes.iter().enumerate() {
        // Validate node ID format if provided
        if let Some(node_id) = &node.id {
            if !is_valid_uuid(node_id) {
                return Err(ValidationError::InvalidUuid {
                    field: format!("node[{index}].id"),
                    value: node_id.clone(),
                });
            }
        }

        // Validate node name
        let node_name = node.name.trim();
        if node_name.is_empty() {
            return Err(ValidationError::EmptyField {
                field: format!("node[{index}].name")
            });
        }
        if node_name.len() > 255 {
            return Err(ValidationError::FieldTooLong {
                field: format!("node[{index}].name"),
                max_len: 255,
                actual_len: node_name.len(),
            });
        }
    }

    // Batch validate edges
    for (index, edge) in request.edges.iter().enumerate() {
        if !is_valid_uuid(&edge.from_node_id) {
            return Err(ValidationError::InvalidUuid {
                field: format!("edge[{index}].from_node_id"),
                value: edge.from_node_id.clone(),
            });
        }
        if !is_valid_uuid(&edge.to_node_id) {
            return Err(ValidationError::InvalidUuid {
                field: format!("edge[{index}].to_node_id"),
                value: edge.to_node_id.clone(),
            });
        }
    }

    Ok(())
}