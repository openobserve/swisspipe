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
        start_node_name: &str,
        nodes: &[Node],
        edges: &[Edge],
    ) -> Result<()> {
        // 1. Check if start_node_name exists in nodes
        Self::validate_start_node_exists(start_node_name, nodes)?;
        
        // 2. Validate edge consistency (from/to nodes exist)
        Self::validate_edge_consistency(nodes, edges)?;
        
        // 3. Validate DAG structure (no cycles)
        Self::validate_no_cycles(nodes, edges)?;
        
        // 4. Validate connectivity (all nodes reachable from start)
        Self::validate_connectivity(start_node_name, nodes, edges)?;
        
        // 5. Validate conditional edges have condition nodes
        Self::validate_conditional_edges(nodes, edges)?;
        
        Ok(())
    }
    
    /// Validate that start_node_name exists in the workflow's nodes
    fn validate_start_node_exists(start_node_name: &str, nodes: &[Node]) -> Result<()> {
        let node_exists = nodes.iter().any(|node| node.name == start_node_name);
        
        if !node_exists {
            return Err(SwissPipeError::Config(format!(
                "Start node '{start_node_name}' not found in workflow nodes"
            )));
        }
        
        Ok(())
    }
    
    /// Validate that all edge from_node_name and to_node_name exist in nodes
    fn validate_edge_consistency(nodes: &[Node], edges: &[Edge]) -> Result<()> {
        let node_names: HashSet<String> = nodes.iter().map(|n| n.name.clone()).collect();
        
        for edge in edges {
            if !node_names.contains(&edge.from_node_name) {
                return Err(SwissPipeError::Config(format!(
                    "Edge references non-existent from_node: '{}'",
                    edge.from_node_name
                )));
            }
            
            if !node_names.contains(&edge.to_node_name) {
                return Err(SwissPipeError::Config(format!(
                    "Edge references non-existent to_node: '{}'",
                    edge.to_node_name
                )));
            }
        }
        
        Ok(())
    }
    
    /// Validate that the workflow DAG has no cycles
    fn validate_no_cycles(nodes: &[Node], edges: &[Edge]) -> Result<()> {
        let node_names: HashSet<String> = nodes.iter().map(|n| n.name.clone()).collect();
        
        // Build adjacency list
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        for node_name in &node_names {
            graph.insert(node_name.clone(), Vec::new());
        }
        
        for edge in edges {
            graph.get_mut(&edge.from_node_name)
                .unwrap()
                .push(edge.to_node_name.clone());
        }
        
        // Use DFS to detect cycles
        let mut white_set = node_names.clone();
        let mut gray_set = HashSet::new();
        let mut black_set = HashSet::new();
        
        for node in &node_names {
            if white_set.contains(node)
                && Self::has_cycle_dfs(node, &graph, &mut white_set, &mut gray_set, &mut black_set) {
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
    fn validate_connectivity(start_node_name: &str, nodes: &[Node], edges: &[Edge]) -> Result<()> {
        let node_names: HashSet<String> = nodes.iter().map(|n| n.name.clone()).collect();
        
        // Build adjacency list
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        for node_name in &node_names {
            graph.insert(node_name.clone(), Vec::new());
        }
        
        for edge in edges {
            graph.get_mut(&edge.from_node_name)
                .unwrap()
                .push(edge.to_node_name.clone());
        }
        
        // BFS from start node to find all reachable nodes
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        
        queue.push_back(start_node_name.to_string());
        visited.insert(start_node_name.to_string());
        
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
        for node_name in &node_names {
            if !visited.contains(node_name) {
                return Err(SwissPipeError::Config(format!(
                    "Node '{node_name}' is not reachable from start node '{start_node_name}'"
                )));
            }
        }
        
        Ok(())
    }
    
    /// Validate that conditional edges originate from condition nodes
    fn validate_conditional_edges(nodes: &[Node], edges: &[Edge]) -> Result<()> {
        let condition_nodes: HashSet<String> = nodes
            .iter()
            .filter_map(|node| match &node.node_type {
                NodeType::Condition { .. } => Some(node.name.clone()),
                _ => None,
            })
            .collect();
        
        for edge in edges {
            if edge.condition_result.is_some()
                && !condition_nodes.contains(&edge.from_node_name) {
                    return Err(SwissPipeError::Config(format!(
                        "Conditional edge from '{}' requires a Condition node",
                        edge.from_node_name
                    )));
                }
        }
        
        Ok(())
    }
    
    /// Validate that condition nodes have both true and false edges (warning only)
    pub fn validate_condition_completeness(nodes: &[Node], edges: &[Edge]) -> Vec<String> {
        let mut warnings = Vec::new();
        
        let condition_nodes: HashSet<String> = nodes
            .iter()
            .filter_map(|node| match &node.node_type {
                NodeType::Condition { .. } => Some(node.name.clone()),
                _ => None,
            })
            .collect();
        
        for condition_node in &condition_nodes {
            let has_true_edge = edges.iter().any(|e| {
                e.from_node_name == *condition_node && e.condition_result == Some(true)
            });
            let has_false_edge = edges.iter().any(|e| {
                e.from_node_name == *condition_node && e.condition_result == Some(false)
            });
            
            if !has_true_edge {
                warnings.push(format!(
                    "Condition node '{condition_node}' has no true edge - some data may be dropped"
                ));
            }
            
            if !has_false_edge {
                warnings.push(format!(
                    "Condition node '{condition_node}' has no false edge - some data may be dropped"
                ));
            }
        }
        
        warnings
    }
}