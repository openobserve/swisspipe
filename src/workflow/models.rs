use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEvent {
    pub data: serde_json::Value,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub condition_results: HashMap<String, bool>, // Store condition results by node ID
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HttpMethod {
    #[serde(alias = "GET")]
    Get,
    #[serde(alias = "POST")]
    Post,
    #[serde(alias = "PUT")]
    Put,
    #[serde(alias = "DELETE")]
    Delete,
    #[serde(alias = "PATCH")]
    Patch,
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpMethod::Get => write!(f, "GET"),
            HttpMethod::Post => write!(f, "POST"),
            HttpMethod::Put => write!(f, "PUT"),
            HttpMethod::Delete => write!(f, "DELETE"),
            HttpMethod::Patch => write!(f, "PATCH"),
        }
    }
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


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FailureAction {
    Continue,    // Continue to next node
    Stop,        // Stop workflow execution
    Retry,       // Retry the current node
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DelayUnit {
    Seconds,
    Minutes,
    Hours,
    Days,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopConfig {
    pub max_iterations: Option<u32>,
    pub interval_seconds: u64,
    pub backoff_strategy: BackoffStrategy,
    pub termination_condition: Option<TerminationCondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackoffStrategy {
    Fixed(u64),                    // Fixed interval
    Exponential { base: u64, multiplier: f64, max: u64 },
    Custom(String),                // JavaScript expression
}

impl Default for BackoffStrategy {
    fn default() -> Self {
        BackoffStrategy::Fixed(3600) // Default 1 hour
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminationCondition {
    pub script: String,           // JavaScript function: function condition(event) { return boolean; }
    pub action: TerminationAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TerminationAction {
    Success,   // Emit success signal and continue workflow
    Failure,   // Emit failure signal and continue workflow
    Stop,      // Stop workflow execution
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputMergeStrategy {
    /// Wait for all expected inputs before executing (default for multiple inputs)
    WaitForAll,
    /// Execute on first input, ignore others (default for single inputs)
    FirstWins,
    /// Wait up to N seconds for inputs, then execute with whatever was received
    TimeoutBased(u64),
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
    HttpRequest {
        url: String,
        method: HttpMethod,
        timeout_seconds: u64,
        failure_action: FailureAction,
        retry_config: RetryConfig,
        headers: HashMap<String, String>,
        loop_config: Option<LoopConfig>,
    },
    OpenObserve {
        url: String,
        authorization_header: String,
        timeout_seconds: u64,
        failure_action: FailureAction,
        retry_config: RetryConfig,
    },
    Email {
        config: crate::email::EmailConfig,
    },
    Delay {
        duration: u64,
        unit: DelayUnit,
    },
    Anthropic {
        model: String,
        max_tokens: u32,
        temperature: f64,
        system_prompt: Option<String>,
        user_prompt: String,
        timeout_seconds: u64,
        failure_action: FailureAction,
        retry_config: RetryConfig,
    },
}

#[derive(Debug, Clone)]
pub struct Node {
    pub id: String,
    pub workflow_id: String,
    pub name: String,
    pub node_type: NodeType,
    pub input_merge_strategy: Option<InputMergeStrategy>,
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub id: String,
    pub workflow_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub condition_result: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub start_node_id: Option<String>, // New: node ID reference
    pub enabled: bool,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}