use thiserror::Error;

#[derive(Debug, Error)]
pub enum SwissPipeError {
    #[error("Database error: {0}")]
    Database(#[from] sea_orm::DbErr),
    
    #[error("JavaScript error: {0}")]
    JavaScript(#[from] JavaScriptError),
    
    #[error("App execution error: {0}")]
    App(#[from] AppError),
    
    #[error("Workflow not found: {0}")]
    WorkflowNotFound(String),
    
    #[error("Node not found: {0}")]
    NodeNotFound(String),
    
    #[error("Cycle detected in workflow")]
    CycleDetected,
    
    #[error("HTTP request failed: {0}")]
    HttpRequest(#[from] reqwest::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

#[derive(Debug, Error)]
pub enum JavaScriptError {
    #[error("JavaScript runtime error: {0}")]
    RuntimeError(String),
    
    #[error("JavaScript execution error: {0}")]
    ExecutionError(String),
    
    #[error("Event serialization error: {0}")]
    SerializationError(String),
    
    #[error("Event was dropped by transformer (returned null)")]
    EventDropped,
    
    #[error("JavaScript execution timeout")]
    Timeout,
    
    #[error("JavaScript memory limit exceeded")]
    MemoryLimitExceeded,
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("HTTP request failed after {attempts} attempts: {error}")]
    HttpRequestFailed { attempts: u32, error: String },
    
    #[error("Request timeout after {seconds}s")]
    Timeout { seconds: u64 },
    
    #[error("Invalid response status: {status}")]
    InvalidStatus { status: u16 },
    
    #[error("Authentication failed for OpenObserve")]
    AuthenticationFailed,
    
    #[error("Unsupported app type for operation")]
    UnsupportedOperation,
}

pub type Result<T> = std::result::Result<T, SwissPipeError>;