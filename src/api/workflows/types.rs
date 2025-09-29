use serde::{Deserialize, Serialize};
use crate::workflow::models::NodeType;

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub details: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateWorkflowRequest {
    pub name: String,
    pub description: Option<String>,
    pub start_node_id: Option<String>, // Optional start node ID - if None, first trigger node is used
    pub nodes: Vec<NodeRequest>,
    pub edges: Vec<EdgeRequest>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct NodeRequest {
    pub id: Option<String>, // For updates: existing node ID
    pub name: String,
    pub node_type: NodeType,
    pub position_x: Option<f64>,
    pub position_y: Option<f64>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct EdgeRequest {
    pub from_node_id: String, // Source node ID
    pub to_node_id: String,   // Target node ID
    pub condition_result: Option<bool>,
    pub source_handle_id: Option<String>, // Added for 3-handle routing support
}

#[derive(Serialize)]
pub struct NodeResponse {
    pub id: String,
    pub name: String,
    pub node_type: NodeType,
    pub position_x: f64,
    pub position_y: f64,
}

#[derive(Serialize)]
pub struct EdgeResponse {
    pub id: String,
    pub from_node_id: String,   // Source node ID
    pub to_node_id: String,     // Target node ID
    pub condition_result: Option<bool>,
    pub source_handle_id: Option<String>, // Added for 3-handle routing support
}

#[derive(Serialize)]
pub struct WorkflowResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub start_node_id: String,   // Starting node ID
    pub endpoint_url: String,
    pub enabled: bool,
    pub created_at: i64, // Unix epoch microseconds
    pub updated_at: i64, // Unix epoch microseconds
    pub nodes: Vec<NodeResponse>,
    pub edges: Vec<EdgeResponse>,
}

#[derive(Serialize)]
pub struct WorkflowListResponse {
    pub workflows: Vec<WorkflowResponse>,
}


#[derive(Debug)]
pub struct NodeOperations<'a> {
    pub to_create: Vec<NodeRequest>,
    pub to_update: Vec<(String, NodeRequest, &'a crate::database::nodes::Model)>, // (existing_id, updated_data, existing_node_ref)
    pub to_delete: Vec<String>,
}

#[derive(Debug)]
pub struct EdgeOperations {
    pub to_create: Vec<EdgeRequest>,
    pub to_delete: Vec<String>,
}

/// Context information for workflow updates
#[derive(Debug)]
pub struct UpdateContext {
    pub workflow: crate::database::entities::Model,
    pub existing_nodes: Vec<crate::database::nodes::Model>,
    pub existing_edges: Vec<crate::database::edges::Model>,
    pub existing_start_node_id: String,
    pub internal_nodes: Vec<crate::workflow::models::Node>,
    pub internal_edges: Vec<crate::workflow::models::Edge>,
}

/// Planned operations for workflow update
#[derive(Debug)]
pub struct PlannedOperations<'a> {
    pub node_ops: NodeOperations<'a>,
    pub edge_ops: EdgeOperations,
    pub updated_workflow: crate::database::entities::Model,
}

/// Result of executing workflow update operations
#[derive(Debug)]
pub struct UpdateResult {
    pub workflow: crate::database::entities::Model,
    pub nodes: Vec<crate::database::nodes::Model>,
    pub edges: Vec<crate::database::edges::Model>,
    pub start_node_id: String,
    pub total_duration: std::time::Duration,
}