use std::collections::{HashMap, HashSet};
use super::types::{NodeRequest, EdgeRequest, NodeOperations, EdgeOperations, NodeResponse, EdgeResponse, WorkflowResponse};
use crate::database::{nodes, edges, entities};
use crate::workflow::models::{NodeType, HttpMethod, RetryConfig, FailureAction};

/// Check if a node needs updating by comparing existing vs new data
pub fn node_needs_update(existing: &nodes::Model, new: &NodeRequest) -> bool {
    // Compare name
    if existing.name != new.name {
        tracing::debug!("Node needs update - name changed: node_id={}, old_name='{}', new_name='{}'", 
                       existing.id, existing.name, new.name);
        return true;
    }
    
    // Compare node type string representation
    let new_node_type_str = node_type_to_string(&new.node_type);
    
    if existing.node_type != new_node_type_str {
        tracing::debug!("Node needs update - type changed: node_id={}, old_type='{}', new_type='{}'", 
                       existing.id, existing.node_type, new_node_type_str);
        return true;
    }
    
    // Compare serialized config
    if let Ok(new_config) = serde_json::to_string(&new.node_type) {
        if existing.config != new_config {
            tracing::debug!("Node needs update - config changed: node_id={}, config_differs=true", existing.id);
            return true;
        }
    } else {
        tracing::warn!("Failed to serialize new node config for comparison: node_id={}", existing.id);
    }
    
    // Compare position
    let new_pos_x = new.position_x.unwrap_or(100.0);
    let new_pos_y = new.position_y.unwrap_or(100.0);
    
    if (existing.position_x - new_pos_x).abs() > f64::EPSILON 
        || (existing.position_y - new_pos_y).abs() > f64::EPSILON {
        tracing::debug!("Node needs update - position changed: node_id={}, old_pos=({}, {}), new_pos=({}, {})", 
                       existing.id, existing.position_x, existing.position_y, new_pos_x, new_pos_y);
        return true;
    }
    
    tracing::debug!("Node unchanged: node_id={}, name='{}'", existing.id, existing.name);
    false
}

/// Categorize node changes for differential updates
pub fn categorize_node_changes<'a>(
    existing_nodes: &'a [nodes::Model],
    new_nodes: &[NodeRequest],
    start_node_id: &str,
) -> NodeOperations<'a> {
    let existing_map: HashMap<String, &nodes::Model> = existing_nodes
        .iter()
        .map(|n| (n.id.clone(), n))
        .collect();
    
    let mut to_create = Vec::new();
    let mut to_update = Vec::new();
    let mut processed_ids = HashSet::new();
    
    // Process new nodes - determine if they should be created or updated
    for new_node in new_nodes {
        if let Some(node_id) = &new_node.id {
            if let Some(existing_node) = existing_map.get(node_id.as_str()) {
                // Node exists - check if it needs updating
                if node_needs_update(existing_node, new_node) {
                    to_update.push((node_id.clone(), (*new_node).clone(), *existing_node));
                }
                processed_ids.insert(node_id.clone());
            } else {
                // Node has ID but doesn't exist in database - create it
                to_create.push((*new_node).clone());
            }
        } else {
            // Node has no ID - create new one
            to_create.push((*new_node).clone());
        }
    }
    
    // Find nodes to delete (existing but not in new list, excluding start node)
    let to_delete: Vec<String> = existing_nodes
        .iter()
        .filter(|node| node.id != start_node_id && !processed_ids.contains(&node.id))
        .map(|node| node.id.clone())
        .collect();
    
    NodeOperations {
        to_create,
        to_update,
        to_delete,
    }
}

/// Categorize edge changes for differential updates
pub fn categorize_edge_changes(
    existing_edges: &[edges::Model],
    new_edges: &[EdgeRequest]
) -> EdgeOperations {
    let existing_set: HashSet<(String, String, Option<bool>, Option<String>)> = existing_edges
        .iter()
        .map(|e| (e.from_node_id.clone(), e.to_node_id.clone(), e.condition_result, e.source_handle_id.clone()))
        .collect();

    let new_set: HashSet<(String, String, Option<bool>, Option<String>)> = new_edges
        .iter()
        .map(|e| (e.from_node_id.clone(), e.to_node_id.clone(), e.condition_result, e.source_handle_id.clone()))
        .collect();

    // Find edges to create (in new but not in existing)
    let to_create: Vec<EdgeRequest> = new_edges
        .iter()
        .filter(|e| !existing_set.contains(&(e.from_node_id.clone(), e.to_node_id.clone(), e.condition_result, e.source_handle_id.clone())))
        .cloned()
        .collect();

    // Find edges to delete (in existing but not in new)
    let to_delete: Vec<String> = existing_edges
        .iter()
        .filter(|e| !new_set.contains(&(e.from_node_id.clone(), e.to_node_id.clone(), e.condition_result, e.source_handle_id.clone())))
        .map(|e| e.id.clone())
        .collect();

    EdgeOperations {
        to_create,
        to_delete,
    }
}

/// Convert NodeType enum to string representation
pub fn node_type_to_string(node_type: &NodeType) -> String {
    match node_type {
        NodeType::Trigger { .. } => "trigger".to_string(),
        NodeType::Condition { .. } => "condition".to_string(),
        NodeType::Transformer { .. } => "transformer".to_string(),
        NodeType::HttpRequest { .. } => "http_request".to_string(),
        NodeType::OpenObserve { .. } => "openobserve".to_string(),
        NodeType::Email { .. } => "email".to_string(),
        NodeType::Delay { .. } => "delay".to_string(),
        NodeType::Anthropic { .. } => "anthropic".to_string(),
        NodeType::HumanInLoop { .. } => "human_in_loop".to_string(),
    }
}

/// Convert database nodes to response format
pub fn nodes_to_response(nodes: Vec<nodes::Model>) -> Vec<NodeResponse> {
    nodes
        .into_iter()
        .map(|node| {
            let node_type: NodeType = serde_json::from_str(&node.config)
                .unwrap_or_else(|e| {
                    tracing::warn!("Failed to deserialize node config for node {}: {}", node.id, e);
                    NodeType::HttpRequest {
                        url: "".to_string(),
                        method: HttpMethod::Get,
                        timeout_seconds: 30,
                        failure_action: FailureAction::Stop,
                        retry_config: RetryConfig::default(),
                        headers: HashMap::new(),
                        loop_config: None,
                    }
                });
            NodeResponse {
                id: node.id,
                name: node.name,
                node_type,
                position_x: node.position_x,
                position_y: node.position_y,
            }
        })
        .collect()
}

/// Convert database edges to response format
pub fn edges_to_response(edges: Vec<edges::Model>) -> Vec<EdgeResponse> {
    edges
        .into_iter()
        .map(|edge| EdgeResponse {
            id: edge.id,
            from_node_id: edge.from_node_id,
            to_node_id: edge.to_node_id,
            condition_result: edge.condition_result,
            source_handle_id: edge.source_handle_id,
        })
        .collect()
}

/// Build complete workflow response from database entities
pub fn build_workflow_response(
    workflow: entities::Model,
    nodes: Vec<nodes::Model>,
    edges: Vec<edges::Model>,
    start_node_id: String,
) -> WorkflowResponse {
    let node_responses = nodes_to_response(nodes);
    let edge_responses = edges_to_response(edges);

    WorkflowResponse {
        id: workflow.id.clone(),
        name: workflow.name,
        description: workflow.description,
        start_node_id,
        endpoint_url: format!("/api/v1/{}/trigger", workflow.id),
        enabled: workflow.enabled,
        created_at: workflow.created_at,
        updated_at: workflow.updated_at,
        nodes: node_responses,
        edges: edge_responses,
    }
}