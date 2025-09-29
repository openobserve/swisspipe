use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEvent {
    pub data: serde_json::Value,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub condition_results: HashMap<String, bool>, // Store condition results by node ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hil_task: Option<serde_json::Value>, // HIL task metadata
}

impl Default for WorkflowEvent {
    fn default() -> Self {
        Self {
            data: serde_json::Value::Object(serde_json::Map::new()),
            metadata: HashMap::new(),
            headers: HashMap::new(),
            condition_results: HashMap::new(),
            hil_task: None,
        }
    }
}

/// HIL-specific execution path types for the 3-handle architecture
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HilPathType {
    #[serde(rename = "notification")]
    Notification,  // Blue handle - immediate execution
    #[serde(rename = "approved")]
    Approved,      // Green handle - after human approval
    #[serde(rename = "denied")]
    Denied,        // Red handle - after human denial
}

/// Execution path for HIL multi-path routing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPath {
    pub path_type: HilPathType,
    pub target_node_ids: Vec<String>, // Connected nodes for this path
    pub event: WorkflowEvent,
    pub executed_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Pending execution context for blocked HIL paths
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingExecution {
    pub execution_id: Uuid,
    pub node_id: String,
    pub path_type: HilPathType,
    pub event: WorkflowEvent,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Multi-path execution result for HIL nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HilMultiPathResult {
    pub notification_path: ExecutionPath,        // Immediate execution
    pub approved_pending: PendingExecution,      // Blocked until approval
    pub denied_pending: PendingExecution,        // Blocked until denial
    pub hil_task_id: String,
    pub node_execution_id: String,
}

/// HIL Resumption Payload for database job queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HilResumptionPayload {
    pub node_execution_id: String,
    pub hil_response: crate::hil::HilResponse,
    pub resume_path: String, // "approved" or "denied"
}

/// Workflow resumption state for HIL job queue storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowResumptionState {
    pub execution_id: String,
    pub workflow_id: String,
    pub current_node_id: String, // HIL node ID
    pub event_data: WorkflowEvent,
    pub hil_task_id: String,
}

/// Node execution output types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeOutput {
    /// Standard single-path continuation
    Continue(WorkflowEvent),
    /// HIL multi-path execution with immediate and blocked paths
    MultiPath(Box<HilMultiPathResult>),
    /// Node execution completed with no continuation
    Complete,
    /// Async execution pending - triggers job queue processing
    AsyncPending(WorkflowEvent),
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
    HumanInLoop {
        title: String,
        description: Option<String>,
        timeout_seconds: Option<u64>,
        timeout_action: Option<String>, // "approved" or "denied"
        required_fields: Option<Vec<String>>,
        metadata: Option<serde_json::Value>,
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
    pub source_handle_id: Option<String>, // Added for 3-handle routing support
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

impl Workflow {
    /// Check if workflow contains Human-in-Loop nodes that require synchronous execution
    pub fn contains_hil_nodes(&self) -> bool {
        self.nodes.iter().any(|node| {
            matches!(node.node_type, NodeType::HumanInLoop { .. })
        })
    }

    /// Check if workflow contains nodes that require specialized scheduling and blocking behavior
    /// This includes: HIL nodes, HTTP loops, and Delay nodes
    pub fn requires_sync_execution(&self) -> bool {
        self.nodes.iter().any(|node| {
            match &node.node_type {
                // HIL nodes require sync execution for multi-path handling
                NodeType::HumanInLoop { .. } => true,
                // HTTP nodes with loop configs require specialized scheduler
                NodeType::HttpRequest { loop_config, .. } => loop_config.is_some(),
                // Delay nodes require specialized scheduler for timing
                NodeType::Delay { .. } => true,
                _ => false,
            }
        })
    }
}