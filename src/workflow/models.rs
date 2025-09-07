use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEvent {
    pub data: serde_json::Value,
    pub metadata: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub condition_results: HashMap<String, bool>, // Store condition results by node name
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 100,
            max_delay_ms: 5000,
            backoff_multiplier: 2.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppType {
    Webhook,
    OpenObserve {
        username: String,
        password: String,
        stream_name: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeType {
    Trigger { 
        methods: Vec<HttpMethod> 
    },
    Condition { 
        script: String 
    },
    Transformer { 
        script: String 
    },
    App {
        app_type: AppType,
        url: String,
        method: HttpMethod,
        timeout_seconds: u64,
        retry_config: RetryConfig,
    },
}

#[derive(Debug, Clone)]
pub struct Node {
    pub id: String,
    pub workflow_id: String,
    pub name: String,
    pub node_type: NodeType,
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub id: String,
    pub workflow_id: String,
    pub from_node_name: String,
    pub to_node_name: String,
    pub condition_result: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub start_node_name: String,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}