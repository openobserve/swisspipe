# SwissPipe Workflow Engine - Technical Design Document

## System Architecture

### High-Level Architecture
```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   HTTP Client   │───▶│   Axum Server    │───▶│   Workflow      │
│                 │    │                  │    │   Engine        │
└─────────────────┘    └──────────────────┘    └─────────────────┘
                                │                        │
                                ▼                        ▼
                       ┌──────────────────┐    ┌─────────────────┐
                       │   Auth Layer     │    │   QuickJS       │
                       │                  │    │   Runtime       │
                       └──────────────────┘    └─────────────────┘
                                │                        │
                                ▼                        ▼
                       ┌──────────────────┐    ┌─────────────────┐
                       │   SQLite DB      │    │   HTTP Client   │
                       │   (SeaORM)       │    │   (Webhook)     │
                       └──────────────────┘    └─────────────────┘
```

## Module Structure

### Core Modules

```rust
src/
├── main.rs                 // Application entry point
├── config.rs              // Environment configuration
├── auth/
│   └── mod.rs             // Basic auth middleware
├── api/
│   ├── mod.rs             // API router setup
│   ├── workflows.rs       // Workflow CRUD endpoints  
│   └── ingestion.rs       // Data ingestion endpoints
├── workflow/
│   ├── mod.rs             // Workflow execution engine
│   ├── engine.rs          // DAG execution logic
│   ├── nodes.rs           // Node implementations
│   └── executor.rs        // JavaScript runtime integration
├── database/
│   ├── mod.rs             // Database connection and models
│   ├── entities.rs        // SeaORM entity definitions
│   └── migrations.rs      // Database schema migrations
└── utils/
    ├── mod.rs
    ├── javascript.rs      // QuickJS wrapper
    └── http_client.rs     // Webhook HTTP client
```

## Dependencies

### Cargo.toml
```toml
[dependencies]
axum = "0.7"
sea-orm = { version = "0.12", features = ["sqlx-sqlite", "runtime-tokio-rustls"] }
rquickjs = { version = "0.4", features = ["full"] }
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
reqwest = { version = "0.11", features = ["json"] }
uuid = { version = "1.0", features = ["v4"] }
thiserror = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
base64 = "0.21"
```

## Database Schema

### Tables

#### workflows
```sql
CREATE TABLE workflows (
    id TEXT PRIMARY KEY,           -- UUID
    name TEXT NOT NULL,
    description TEXT,
    start_node_name TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (id, start_node_name) REFERENCES nodes(workflow_id, name)
);
```

#### nodes
```sql
CREATE TABLE nodes (
    id TEXT PRIMARY KEY,           -- UUID
    workflow_id TEXT NOT NULL,
    name TEXT NOT NULL,            -- Human readable name
    node_type TEXT NOT NULL,       -- 'trigger', 'condition', 'transformer', 'app'
    config TEXT NOT NULL,          -- JSON configuration
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (workflow_id) REFERENCES workflows(id) ON DELETE CASCADE,
    UNIQUE(workflow_id, name)      -- Unique names within workflow
);
```

#### edges
```sql
CREATE TABLE edges (
    id TEXT PRIMARY KEY,           -- UUID
    workflow_id TEXT NOT NULL,
    from_node_name TEXT NOT NULL,
    to_node_name TEXT NOT NULL,
    condition_result BOOLEAN,      -- NULL for unconditional, true/false for conditional
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (workflow_id) REFERENCES workflows(id) ON DELETE CASCADE
);
```

### Indexes
```sql
CREATE INDEX idx_nodes_workflow_id ON nodes(workflow_id);
CREATE INDEX idx_edges_workflow_id ON edges(workflow_id);
CREATE INDEX idx_edges_from_node ON edges(workflow_id, from_node_name);
```

## Data Models

### Core Structs

```rust
#[derive(Serialize, Deserialize, Clone)]
pub struct WorkflowEvent {
    pub data: serde_json::Value,
    pub metadata: HashMap<String, String>,
    pub headers: HashMap<String, String>,
}

#[derive(Clone)]
pub enum NodeType {
    Trigger { methods: Vec<HttpMethod> },
    Condition { script: String },
    Transformer { script: String },
    App { 
        app_type: AppType,
        url: String, 
        method: HttpMethod,
        timeout_seconds: u64,
        retry_config: RetryConfig,
    },
}

#[derive(Clone)]
pub enum AppType {
    Webhook,
    OpenObserve {
        username: String,
        password: String,
        stream_name: String,
    },
}

#[derive(Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
}

#[derive(Clone)]
pub struct Node {
    pub id: String,
    pub workflow_id: String,
    pub name: String,
    pub node_type: NodeType,
}

#[derive(Clone)]
pub struct Edge {
    pub id: String,
    pub workflow_id: String,
    pub from_node_name: String,
    pub to_node_name: String,
    pub condition_result: Option<bool>,
}

#[derive(Clone)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub start_node_name: String,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}
```

## API Implementation

### Routing Structure

```rust
// Main router setup
pub fn create_router() -> Router {
    Router::new()
        .nest("/api/v1", ingestion_routes())
        .nest("/workflows", management_routes())
        .layer(middleware::from_fn(auth_middleware))
}

// Management API (requires auth)
fn management_routes() -> Router {
    Router::new()
        .route("/", get(list_workflows).post(create_workflow))
        .route("/:id", get(get_workflow).put(update_workflow).delete(delete_workflow))
}

// Data ingestion API (UUID-based auth)
fn ingestion_routes() -> Router {
    Router::new()
        .route("/:workflow_id/trigger", get(trigger_workflow).post(trigger_workflow).put(trigger_workflow))
        .route("/:workflow_id/json_array", post(trigger_workflow_array))
}
```

### Request/Response Models

```rust
#[derive(Deserialize)]
pub struct CreateWorkflowRequest {
    pub name: String,
    pub description: Option<String>,
    pub nodes: Vec<NodeRequest>,
    pub edges: Vec<EdgeRequest>,
}

#[derive(Serialize)]
pub struct WorkflowResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub endpoint_url: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct NodeRequest {
    pub name: String,
    pub node_type: String,
    pub config: serde_json::Value,
}
```

## Workflow Execution Engine

### Execution Flow

1. **Request Validation**: Validate workflow UUID and extract data
2. **DAG Resolution**: Load workflow definition and build execution graph
3. **Event Creation**: Create WorkflowEvent from HTTP request
4. **Node Execution**: Execute nodes following DAG edges
5. **Response Generation**: Return final result or error

### DAG Execution Algorithm

```rust
pub struct WorkflowExecutor {
    workflow: Workflow,
    js_runtime: Arc<Mutex<QuickJsRuntime>>,
    http_client: reqwest::Client,
}

impl WorkflowExecutor {
    pub async fn execute(&self, event: WorkflowEvent) -> Result<WorkflowEvent> {
        let mut current_event = event;
        let mut current_node_name = &self.workflow.start_node_name;
        let mut visited = HashSet::new();
        
        loop {
            // Prevent infinite loops
            if visited.contains(current_node_name) {
                return Err(ExecutionError::CycleDetected);
            }
            visited.insert(current_node_name.clone());
            
            let node = self.find_node(current_node_name)?;
            current_event = self.execute_node(node, current_event).await?;
            
            let next_nodes = self.get_next_nodes(current_node_name, &current_event)?;
            match next_nodes.len() {
                0 => break, // End of workflow
                1 => current_node_name = &next_nodes[0],
                _ => return Err(ExecutionError::MultiplePathsNotSupported),
            }
        }
        
        Ok(current_event)
    }
    
    async fn execute_node(&self, node: &Node, event: WorkflowEvent) -> Result<WorkflowEvent> {
        match &node.node_type {
            NodeType::Trigger { .. } => Ok(event),
            NodeType::Condition { script } => {
                let result = self.execute_js_condition(script, &event).await?;
                // Condition nodes pass through event unchanged
                Ok(event)
            },
            NodeType::Transformer { script } => {
                self.execute_js_transformer(script, event).await
            },
            NodeType::App { app_type, url, method, timeout_seconds, retry_config } => {
                self.execute_app(app_type, url, method, *timeout_seconds, retry_config, event).await
            },
        }
    }
}
```

## JavaScript Runtime Integration

### Function Signatures

Users must provide complete JavaScript function implementations with these exact signatures:

#### Transformer Functions
```javascript
function transformer(event) {
   // Do some processing with the event
   
   return event;  // Must return the modified event. return null to drop the event.
}
```

#### Condition Functions  
```javascript
function condition(event) {
   // Do some processing with the event
   
   return true; // Must return true or false
}
```

### QuickJS Wrapper

```rust
use rquickjs::{Context, Runtime};
use std::time::Duration;

pub struct JavaScriptExecutor {
    runtime: Arc<Mutex<Runtime>>,
}

impl JavaScriptExecutor {
    pub fn new() -> Result<Self, JavaScriptError> {
        let runtime = Runtime::new()
            .map_err(|e| JavaScriptError::RuntimeError(e.to_string()))?;
            
        Ok(JavaScriptExecutor {
            runtime: Arc::new(Mutex::new(runtime)),
        })
    }
    
    pub async fn execute_condition(&self, script: &str, event: &WorkflowEvent) -> Result<bool> {
        let runtime = self.runtime.lock().await;
        let event_json = serde_json::to_string(event)
            .map_err(|e| JavaScriptError::SerializationError(e.to_string()))?;
        
        let context = Context::full(&runtime)
            .map_err(|e| JavaScriptError::RuntimeError(e.to_string()))?;
            
        let result = context.with(|ctx| {
            // User provides the complete function implementation
            let full_script = format!(
                r#"
                {}
                condition({});
                "#,
                script, event_json
            );
            
            let result: rquickjs::Result<bool> = ctx.eval(&full_script);
            result.map_err(|e| JavaScriptError::ExecutionError(e.to_string()))
        })?;
        
        Ok(result)
    }
    
    pub async fn execute_transformer(&self, script: &str, event: WorkflowEvent) -> Result<WorkflowEvent> {
        let runtime = self.runtime.lock().await;
        let event_json = serde_json::to_string(&event)
            .map_err(|e| JavaScriptError::SerializationError(e.to_string()))?;
        
        let context = Context::full(&runtime)
            .map_err(|e| JavaScriptError::RuntimeError(e.to_string()))?;
            
        let result = context.with(|ctx| {
            // User provides the complete function implementation
            let full_script = format!(
                r#"
                {}
                JSON.stringify(transformer({}));
                "#,
                script, event_json
            );
            
            let result: rquickjs::Result<String> = ctx.eval(&full_script);
            result.map_err(|e| JavaScriptError::ExecutionError(e.to_string()))
        })?;
        
        // Handle null return (drop event case)
        if result == "null" {
            return Err(JavaScriptError::EventDropped);
        }
        
        let transformed_event: WorkflowEvent = serde_json::from_str(&result)
            .map_err(|e| JavaScriptError::SerializationError(e.to_string()))?;
            
        Ok(transformed_event)
    }
    
    // Pseudocode for app execution with retry logic
    async fn execute_app(...) -> Result<WorkflowEvent> {
        for attempt in 1..=max_attempts {
            match execute_single_request() {
                Ok(result) => return Ok(result),
                Err(e) if attempt == max_attempts => return Err(e),
                Err(_) => {
                    wait_with_exponential_backoff(attempt);
                    continue;
                }
            }
        }
    }
}
```

## App Types and Failure Handling

### Supported App Types

#### Webhook Apps
- Send HTTP requests (GET, POST, PUT) to arbitrary endpoints  
- Support query parameters for GET requests
- Parse JSON responses and pass data to next workflow nodes
- Standard HTTP app integration

#### OpenObserve Apps
- Specialized integration for OpenObserve log ingestion platform
- Automatically converts single events to JSON array format
- Uses HTTP Basic Authentication
- Endpoint format: `{base_url}/api/{stream_name}/ingest`
- Returns original event for further workflow processing

### Retry and Timeout Configuration

  1. 🔄 Retry (failure_action: "Retry")
    - Uses the full retry_config settings (max_attempts, backoff, etc.)
    - Keeps retrying according to user-defined retry configuration
    - Only fails if all retry attempts are exhausted
  2. ➡️ Continue (failure_action: "Continue")
    - Tries only once (max_attempts: 1)
    - If successful: Passes the app response to next nodes
    - If fails: Logs a warning and continues workflow with original event data
    - Never stops the workflow due to app failures
  3. ⏹️ Stop (failure_action: "Stop")
    - Tries only once (max_attempts: 1)
    - If successful: Passes the app response to next nodes
    - If fails: Stops the entire workflow execution

#### RetryConfig Structure
```
max_attempts: u32        // Maximum retry attempts (default: 3)
initial_delay_ms: u64    // Initial delay before first retry (default: 100ms)
max_delay_ms: u64        // Maximum delay between retries (default: 5000ms)
backoff_multiplier: f64  // Exponential backoff multiplier (default: 2.0)
```

#### Timeout Configuration
- timeout_seconds: Per-request timeout in seconds (default: 30s)
- Applied to each individual request attempt

### Failure Handling Strategy

#### Retry Algorithm (Pseudocode)
```
for attempt = 1 to max_attempts:
    try:
        response = make_http_request(with timeout)
        if response.status in [200-299]:
            return success(response)
        else:
            throw http_error(response.status)
    catch error:
        if attempt == max_attempts:
            return failure(error)
        else:
            delay = min(initial_delay * (backoff_multiplier ^ (attempt-1)), max_delay)
            wait(delay)
            continue
```

#### Error Categories
- **Retryable**: Network errors, 5xx status codes, timeouts
- **Non-Retryable**: 4xx status codes (except for specific cases), authentication failures
- **Special Cases**: OpenObserve 401 errors are not retried

## Configuration Management

### Environment Configuration

```rust
#[derive(Clone)]
pub struct Config {
    pub username: String,
    pub password: String,
    pub database_url: String,
    pub port: u16,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Config {
            username: env::var("SP_USERNAME").unwrap_or_else(|_| "admin".to_string()),
            password: env::var("SP_PASSWORD").unwrap_or_else(|_| "admin".to_string()),
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:data/swisspipe.db?mode=rwc".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "3700".to_string())
                .parse()
                .map_err(|_| ConfigError::InvalidPort)?,
        })
    }
}
```

## Error Handling

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
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
}

#[derive(Debug, thiserror::Error)]
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

#[derive(Debug, thiserror::Error)]
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
```

## Security Considerations

### Authentication Flow

1. **Management API**: HTTP Basic Auth using environment variables
2. **Data Ingestion**: UUID-based authentication (workflow UUID in URL)
3. **JavaScript Sandboxing**: QuickJS runtime isolation
4. **Input Validation**: Strict JSON schema validation

### Security Measures

```rust
// Basic auth middleware
pub async fn auth_middleware(
    req: Request<Body>,
    next: Next<Body>,
) -> Result<Response, StatusCode> {
    if req.uri().path().starts_with("/api/v1/") {
        // Skip auth for ingestion endpoints
        return Ok(next.run(req).await);
    }
    
    let auth_header = req.headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Basic "))
        .ok_or(StatusCode::UNAUTHORIZED)?;
        
    let decoded = base64::decode(auth_header)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
        
    let credentials = String::from_utf8(decoded)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
        
    let (username, password) = credentials
        .split_once(':')
        .ok_or(StatusCode::UNAUTHORIZED)?;
        
    if username == CONFIG.username && password == CONFIG.password {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}
```

## Performance Optimizations

### Database Optimizations
- Connection pooling via SeaORM
- Prepared statements for common queries
- Indexes on frequently queried columns
- SQLite WAL mode for better concurrency

### Runtime Optimizations
- QuickJS runtime pooling for JavaScript execution
- HTTP client connection reuse for webhooks
- Async/await throughout the application
- Minimal memory allocations in hot paths

### Monitoring and Observability
- Structured logging with tracing
- Request timing metrics
- Error rate tracking
- Memory usage monitoring

## Deployment

### Single Binary Deployment
- Static linking of all dependencies
- Embedded SQLite database
- Self-contained executable
- No external runtime requirements

### Startup Sequence
1. Load environment configuration
2. Initialize database connection and run migrations
3. Initialize JavaScript runtime pool
4. Start HTTP server on configured port
5. Begin accepting requests

## Testing Strategy

### Unit Tests
- JavaScript execution engine
- Workflow DAG validation
- Node execution logic
- HTTP client integration

### Integration Tests
- End-to-end workflow execution
- Database operations
- API endpoint testing
- Authentication flow validation

### Performance Tests
- Concurrent workflow execution
- Memory usage under load
- JavaScript execution performance
- Database query optimization

