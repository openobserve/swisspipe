use crate::workflow::{
    errors::{Result, SwissPipeError},
    models::{Edge, Node, NodeType},
};
use std::collections::{HashMap, HashSet, VecDeque};

pub struct WorkflowValidator;

impl WorkflowValidator {
    /// Validate the entire workflow structure
    pub fn validate_workflow(
        _name: &str,
        start_node_id: &str,
        nodes: &[Node],
        edges: &[Edge],
    ) -> Result<()> {
        // 1. Check if start_node_id exists in nodes
        Self::validate_start_node_exists(start_node_id, nodes)?;
        
        // 2. Validate edge consistency (from/to nodes exist)
        Self::validate_edge_consistency(nodes, edges)?;
        
        // 3. Validate DAG structure (no cycles)
        Self::validate_no_cycles(nodes, edges)?;
        
        // 4. Validate connectivity (all nodes reachable from start)
        Self::validate_connectivity(start_node_id, nodes, edges)?;
        
        // 5. Validate conditional edges have condition nodes
        Self::validate_conditional_edges(nodes, edges)?;
        
        Ok(())
    }
    
    /// Validate that start_node_id exists in the workflow's nodes
    fn validate_start_node_exists(start_node_id: &str, nodes: &[Node]) -> Result<()> {
        let node_exists = nodes.iter().any(|node| node.id == start_node_id);
        
        if !node_exists {
            return Err(SwissPipeError::Config(format!(
                "Start node with ID '{start_node_id}' not found in workflow nodes"
            )));
        }
        
        Ok(())
    }
    
    /// Validate that all edge from_node_id and to_node_id exist in nodes
    fn validate_edge_consistency(nodes: &[Node], edges: &[Edge]) -> Result<()> {
        let node_ids: HashSet<String> = nodes.iter().map(|n| n.id.clone()).collect();

        // Create a mapping of node IDs to names for better error messages
        let node_names: HashMap<String, String> = nodes.iter()
            .map(|n| (n.id.clone(), n.name.clone()))
            .collect();

        // Helper function to get node display name
        let get_node_display = |node_id: &str| -> String {
            match node_names.get(node_id) {
                Some(name) => format!("'{name}' ({node_id})"),
                None => format!("node with ID '{node_id}'"),
            }
        };

        for edge in edges {
            if !node_ids.contains(&edge.from_node_id) {
                return Err(SwissPipeError::Config(format!(
                    "Edge references non-existent from_node: {}",
                    get_node_display(&edge.from_node_id)
                )));
            }

            if !node_ids.contains(&edge.to_node_id) {
                return Err(SwissPipeError::Config(format!(
                    "Edge references non-existent to_node: {}",
                    get_node_display(&edge.to_node_id)
                )));
            }
        }
        
        Ok(())
    }
    
    /// Validate that the workflow DAG has no cycles
    fn validate_no_cycles(nodes: &[Node], edges: &[Edge]) -> Result<()> {
        let node_ids: HashSet<String> = nodes.iter().map(|n| n.id.clone()).collect();
        
        // Build adjacency list
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        for node_id in &node_ids {
            graph.insert(node_id.clone(), Vec::new());
        }
        
        for edge in edges {
            graph.get_mut(&edge.from_node_id)
                .unwrap()
                .push(edge.to_node_id.clone());
        }
        
        // Use DFS to detect cycles
        let mut white_set = node_ids.clone();
        let mut gray_set = HashSet::new();
        let mut black_set = HashSet::new();
        
        for node_id in &node_ids {
            if white_set.contains(node_id)
                && Self::has_cycle_dfs(node_id, &graph, &mut white_set, &mut gray_set, &mut black_set) {
                    return Err(SwissPipeError::Config(
                        "Workflow contains cycles - DAG structure required".to_string()
                    ));
                }
        }
        
        Ok(())
    }
    
    /// DFS helper for cycle detection using three-color algorithm
    fn has_cycle_dfs(
        node: &str,
        graph: &HashMap<String, Vec<String>>,
        white_set: &mut HashSet<String>,
        gray_set: &mut HashSet<String>,
        black_set: &mut HashSet<String>,
    ) -> bool {
        // Move node from white to gray
        white_set.remove(node);
        gray_set.insert(node.to_string());
        
        // Visit all neighbors
        if let Some(neighbors) = graph.get(node) {
            for neighbor in neighbors {
                if black_set.contains(neighbor) {
                    continue; // Already processed
                }
                if gray_set.contains(neighbor) {
                    return true; // Back edge found - cycle detected
                }
                if Self::has_cycle_dfs(neighbor, graph, white_set, gray_set, black_set) {
                    return true;
                }
            }
        }
        
        // Move node from gray to black
        gray_set.remove(node);
        black_set.insert(node.to_string());
        
        false
    }
    
    /// Validate that all nodes are reachable from the start node
    fn validate_connectivity(start_node_id: &str, nodes: &[Node], edges: &[Edge]) -> Result<()> {
        let node_ids: HashSet<String> = nodes.iter().map(|n| n.id.clone()).collect();
        
        // Build adjacency list
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        for node_id in &node_ids {
            graph.insert(node_id.clone(), Vec::new());
        }
        
        for edge in edges {
            graph.get_mut(&edge.from_node_id)
                .unwrap()
                .push(edge.to_node_id.clone());
        }
        
        // BFS from start node to find all reachable nodes
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        
        queue.push_back(start_node_id.to_string());
        visited.insert(start_node_id.to_string());
        
        while let Some(current) = queue.pop_front() {
            if let Some(neighbors) = graph.get(&current) {
                for neighbor in neighbors {
                    if !visited.contains(neighbor) {
                        visited.insert(neighbor.clone());
                        queue.push_back(neighbor.clone());
                    }
                }
            }
        }
        
        // Check if all nodes are reachable
        for node_id in &node_ids {
            if !visited.contains(node_id) {
                // Get node name for better error message
                let node_display_name = nodes.iter()
                    .find(|n| &n.id == node_id)
                    .map(|n| n.name.as_str())
                    .unwrap_or("unknown");
                let start_node_display_name = nodes.iter()
                    .find(|n| n.id == start_node_id)
                    .map(|n| n.name.as_str())
                    .unwrap_or("unknown");
                return Err(SwissPipeError::Config(format!(
                    "Node '{node_display_name}' (id: {node_id}) is not reachable from start node '{start_node_display_name}' (id: {start_node_id})"
                )));
            }
        }
        
        Ok(())
    }
    
    /// Validate that conditional edges originate from condition nodes
    fn validate_conditional_edges(nodes: &[Node], edges: &[Edge]) -> Result<()> {
        let condition_node_ids: HashSet<String> = nodes
            .iter()
            .filter_map(|node| match &node.node_type {
                NodeType::Condition { .. } => Some(node.id.clone()),
                _ => None,
            })
            .collect();
        
        for edge in edges {
            if edge.condition_result.is_some()
                && !condition_node_ids.contains(&edge.from_node_id) {
                    // Get node name for better error message
                    let node_display_name = nodes.iter()
                        .find(|n| n.id == edge.from_node_id)
                        .map(|n| n.name.as_str())
                        .unwrap_or("unknown");
                    return Err(SwissPipeError::Config(format!(
                        "Conditional edge from node '{}' (id: {}) requires a Condition node",
                        node_display_name, edge.from_node_id
                    )));
                }
        }
        
        Ok(())
    }
    
    /// Validate that condition nodes have both true and false edges (warning only)
    pub fn validate_condition_completeness(nodes: &[Node], edges: &[Edge]) -> Vec<String> {
        let mut warnings = Vec::new();
        
        let condition_nodes: Vec<&Node> = nodes
            .iter()
            .filter(|node| matches!(&node.node_type, NodeType::Condition { .. }))
            .collect();
        
        for condition_node in &condition_nodes {
            let has_true_edge = edges.iter().any(|e| {
                e.from_node_id == condition_node.id && e.condition_result == Some(true)
            });
            let has_false_edge = edges.iter().any(|e| {
                e.from_node_id == condition_node.id && e.condition_result == Some(false)
            });
            
            if !has_true_edge {
                warnings.push(format!(
                    "Condition node '{}' (id: {}) has no true edge - some data may be dropped",
                    condition_node.name, condition_node.id
                ));
            }
            
            if !has_false_edge {
                warnings.push(format!(
                    "Condition node '{}' (id: {}) has no false edge - some data may be dropped",
                    condition_node.name, condition_node.id
                ));
            }
        }
        
        warnings
    }
}