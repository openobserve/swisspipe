use std::collections::{HashMap, HashSet};
use uuid::Uuid;
use super::types::{CreateWorkflowRequest, EdgeRequest};
use crate::database::nodes;

/// Validate that a string is a valid UUID
pub fn is_valid_uuid(id: &str) -> bool {
    Uuid::parse_str(id).is_ok()
}

/// Detect cycles in workflow using DFS
pub fn detect_cycles(edges: &[EdgeRequest]) -> Result<(), String> {
    // Build adjacency list
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();
    let mut all_nodes: HashSet<String> = HashSet::new();
    
    for edge in edges {
        graph.entry(edge.from_node_id.clone())
            .or_default()
            .push(edge.to_node_id.clone());
        all_nodes.insert(edge.from_node_id.clone());
        all_nodes.insert(edge.to_node_id.clone());
    }
    
    let mut visiting: HashSet<String> = HashSet::new();
    let mut visited: HashSet<String> = HashSet::new();
    
    fn dfs(
        node: &str,
        graph: &HashMap<String, Vec<String>>,
        visiting: &mut HashSet<String>,
        visited: &mut HashSet<String>,
    ) -> Result<(), String> {
        if visiting.contains(node) {
            return Err(format!("Cycle detected involving node: {node}"));
        }
        
        if visited.contains(node) {
            return Ok(());
        }
        
        visiting.insert(node.to_string());
        
        if let Some(neighbors) = graph.get(node) {
            for neighbor in neighbors {
                dfs(neighbor, graph, visiting, visited)?;
            }
        }
        
        visiting.remove(node);
        visited.insert(node.to_string());
        Ok(())
    }
    
    // Check each node for cycles
    for node in &all_nodes {
        if !visited.contains(node) {
            dfs(node, &graph, &mut visiting, &mut visited)?;
        }
    }
    
    Ok(())
}

/// Validate workflow update request
pub fn validate_workflow_update_request(
    request: &CreateWorkflowRequest, 
    start_node_id: &str,
    existing_nodes: &[nodes::Model]
) -> Result<(), String> {
    tracing::debug!(
        "Starting workflow update validation: workflow_name='{}', request_nodes={}, request_edges={}, existing_nodes={}", 
        request.name, request.nodes.len(), request.edges.len(), existing_nodes.len()
    );
    // Collect ALL valid node IDs (existing + new + updated)
    let mut all_valid_node_ids = HashSet::new();
    
    // Include ALL existing nodes (they will remain valid even if not in update)
    for existing_node in existing_nodes {
        all_valid_node_ids.insert(existing_node.id.clone());
    }
    
    // Track which nodes are being updated/created in this request
    let mut request_node_ids = HashSet::new();
    
    // Validate request nodes and collect their IDs
    for node in &request.nodes {
        // Validate node ID format if provided
        if let Some(node_id) = &node.id {
            if !is_valid_uuid(node_id) {
                return Err(format!("Invalid UUID format for node ID: {node_id}"));
            }
            all_valid_node_ids.insert(node_id.clone());
            request_node_ids.insert(node_id.clone());
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
    
    // Validate no cycles in the workflow
    if let Err(cycle_error) = detect_cycles(&request.edges) {
        return Err(format!("Workflow contains cycles: {cycle_error}"));
    }
    
    // Validate edges against the complete set of valid nodes
    for edge in &request.edges {
        // Validate edge node ID formats
        if !is_valid_uuid(&edge.from_node_id) {
            return Err(format!("Invalid UUID format for edge from_node_id: {}", edge.from_node_id));
        }
        if !is_valid_uuid(&edge.to_node_id) {
            return Err(format!("Invalid UUID format for edge to_node_id: {}", edge.to_node_id));
        }
        
        // Validate that referenced nodes exist (existing OR being created/updated)
        if !all_valid_node_ids.contains(&edge.from_node_id) {
            return Err(format!("Edge references non-existent from_node_id: {}", edge.from_node_id));
        }
        if !all_valid_node_ids.contains(&edge.to_node_id) {
            return Err(format!("Edge references non-existent to_node_id: {}", edge.to_node_id));
        }
        
        // Validate no self-loops
        if edge.from_node_id == edge.to_node_id {
            return Err(format!("Self-loop detected: node {} connects to itself", edge.from_node_id));
        }
        
        // Additional validation: Check for edges referencing nodes that will be deleted
        // A node will be deleted if it exists but is not in the request and is not the start node
        let from_will_be_deleted = existing_nodes.iter()
            .any(|n| n.id == edge.from_node_id && n.id != start_node_id && !request_node_ids.contains(&n.id));
        let to_will_be_deleted = existing_nodes.iter()
            .any(|n| n.id == edge.to_node_id && n.id != start_node_id && !request_node_ids.contains(&n.id));
            
        if from_will_be_deleted {
            return Err(format!(
                "Edge references from_node_id {} which will be deleted in this update", 
                edge.from_node_id
            ));
        }
        if to_will_be_deleted {
            return Err(format!(
                "Edge references to_node_id {} which will be deleted in this update", 
                edge.to_node_id
            ));
        }
    }
    
    // Validate workflow name
    if request.name.trim().is_empty() {
        return Err("Workflow name cannot be empty".to_string());
    }
    
    if request.name.len() > 255 {
        return Err(format!("Workflow name too long: '{}' ({} characters, max 255)", 
                          request.name, request.name.len()));
    }
    
    tracing::debug!(
        "Workflow update validation completed successfully: workflow_name='{}', total_valid_nodes={}, request_edges={}", 
        request.name, all_valid_node_ids.len(), request.edges.len()
    );
    
    Ok(())
}