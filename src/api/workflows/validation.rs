use std::collections::{HashMap, HashSet};
use uuid::Uuid;
use super::types::{CreateWorkflowRequest, EdgeRequest};
use crate::database::nodes;

/// Validate that a string is a valid UUID
pub fn is_valid_uuid(id: &str) -> bool {
    Uuid::parse_str(id).is_ok()
}

/// Detect cycles in workflow using DFS with enhanced error reporting
pub fn detect_cycles_with_node_info(edges: &[EdgeRequest], nodes: &[super::types::NodeRequest]) -> Result<(), String> {
    // Build node name mapping for better error messages
    let node_name_map: HashMap<String, String> = nodes.iter()
        .filter_map(|n| n.id.as_ref().map(|id| (id.clone(), n.name.clone())))
        .collect();

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
    let mut path: Vec<String> = Vec::new();

    fn dfs(
        node: &str,
        graph: &HashMap<String, Vec<String>>,
        visiting: &mut HashSet<String>,
        visited: &mut HashSet<String>,
        path: &mut Vec<String>,
        node_name_map: &HashMap<String, String>,
    ) -> Result<(), String> {
        if visiting.contains(node) {
            // Find the cycle start in the path
            let cycle_start = path.iter().position(|n| n == node).unwrap_or(0);
            let cycle_nodes: Vec<String> = path[cycle_start..]
                .iter()
                .chain(std::iter::once(&node.to_string()))
                .map(|id| {
                    let name = node_name_map.get(id).cloned().unwrap_or_else(|| "unknown".to_string());
                    format!("'{}' ({})", name, id)
                })
                .collect();

            return Err(format!(
                "Cycle detected in workflow. Cycle path: {}",
                cycle_nodes.join(" → ")
            ));
        }

        if visited.contains(node) {
            return Ok(());
        }

        visiting.insert(node.to_string());
        path.push(node.to_string());

        if let Some(neighbors) = graph.get(node) {
            for neighbor in neighbors {
                dfs(neighbor, graph, visiting, visited, path, node_name_map)?;
            }
        }

        path.pop();
        visiting.remove(node);
        visited.insert(node.to_string());
        Ok(())
    }

    // Check each node for cycles
    for node in &all_nodes {
        if !visited.contains(node) {
            dfs(node, &graph, &mut visiting, &mut visited, &mut path, &node_name_map)?;
        }
    }

    Ok(())
}

/// Legacy cycle detection function for backward compatibility
pub fn detect_cycles(edges: &[EdgeRequest]) -> Result<(), String> {
    // Use enhanced version with empty node info
    detect_cycles_with_node_info(edges, &[])
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
    
    // Validate no cycles in the workflow (using enhanced version with node names)
    if let Err(cycle_error) = detect_cycles_with_node_info(&request.edges, &request.nodes) {
        return Err(cycle_error);
    }

    // Build a comprehensive node lookup map for better error messages
    let mut node_lookup = HashMap::new();

    // Add existing nodes to lookup
    for existing_node in existing_nodes {
        node_lookup.insert(existing_node.id.clone(), existing_node.name.clone());
    }

    // Add/update with request nodes
    for node in &request.nodes {
        if let Some(node_id) = &node.id {
            node_lookup.insert(node_id.clone(), node.name.clone());
        }
    }

    // Helper function to get node display name
    let get_node_display = |node_id: &str| -> String {
        match node_lookup.get(node_id) {
            Some(name) => format!("'{}' ({})", name, node_id),
            None => node_id.to_string(),
        }
    };

    // Validate edges against the complete set of valid nodes
    for edge in &request.edges {
        // Validate edge node ID formats
        if !is_valid_uuid(&edge.from_node_id) {
            return Err(format!(
                "Invalid UUID format for edge from_node_id: {}",
                get_node_display(&edge.from_node_id)
            ));
        }
        if !is_valid_uuid(&edge.to_node_id) {
            return Err(format!(
                "Invalid UUID format for edge to_node_id: {}",
                get_node_display(&edge.to_node_id)
            ));
        }

        // Validate that referenced nodes exist (existing OR being created/updated)
        if !all_valid_node_ids.contains(&edge.from_node_id) {
            return Err(format!(
                "Edge references non-existent from_node: {}",
                get_node_display(&edge.from_node_id)
            ));
        }
        if !all_valid_node_ids.contains(&edge.to_node_id) {
            return Err(format!(
                "Edge references non-existent to_node: {}",
                get_node_display(&edge.to_node_id)
            ));
        }

        // Validate no self-loops
        if edge.from_node_id == edge.to_node_id {
            return Err(format!(
                "Self-loop detected: node {} connects to itself",
                get_node_display(&edge.from_node_id)
            ));
        }
        
        // Additional validation: Check for edges referencing nodes that will be deleted
        // A node will be deleted if it exists but is not in the request and is not the start node
        let from_will_be_deleted = existing_nodes.iter()
            .any(|n| n.id == edge.from_node_id && n.id != start_node_id && !request_node_ids.contains(&n.id));
        let to_will_be_deleted = existing_nodes.iter()
            .any(|n| n.id == edge.to_node_id && n.id != start_node_id && !request_node_ids.contains(&n.id));
            
        if from_will_be_deleted {
            return Err(format!(
                "Edge references from_node {} which will be deleted in this update",
                get_node_display(&edge.from_node_id)
            ));
        }
        if to_will_be_deleted {
            return Err(format!(
                "Edge references to_node {} which will be deleted in this update",
                get_node_display(&edge.to_node_id)
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