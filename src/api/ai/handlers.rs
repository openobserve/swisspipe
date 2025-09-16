use axum::{extract::State, http::StatusCode, response::Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use crate::workflow::models::{WorkflowEvent, NodeType};
use crate::anthropic::AnthropicCallConfig;
use crate::api::workflows::types::{CreateWorkflowRequest, NodeRequest, EdgeRequest};
use crate::AppState;

// Input validation constants
const MAX_PROMPT_LENGTH: usize = 2000;
const MIN_PROMPT_LENGTH: usize = 10;

#[derive(Debug, Deserialize)]
pub struct GenerateCodeRequest {
    pub system_prompt: String,
    pub user_prompt: String,
    pub model: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct GenerateCodeResponse {
    pub response: String,
    pub usage: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct GenerateWorkflowRequest {
    pub prompt: String,
}

#[derive(Serialize, Debug)]
pub struct GenerateWorkflowResponse {
    pub success: bool,
    pub workflow_id: Option<String>,
    pub workflow_name: Option<String>,
    pub error: Option<String>,
}

pub async fn generate_code(
    State(state): State<AppState>,
    Json(request): Json<GenerateCodeRequest>,
) -> Result<Json<GenerateCodeResponse>, StatusCode> {
    // Create a dummy event for the Anthropic service
    let dummy_event = WorkflowEvent {
        data: serde_json::json!({}),
        metadata: std::collections::HashMap::new(),
        headers: std::collections::HashMap::new(),
        condition_results: std::collections::HashMap::new(),
    };

    // Default configuration for code generation
    let model = request.model.unwrap_or_else(|| "claude-3-5-sonnet-20241022".to_string());
    let max_tokens = request.max_tokens.unwrap_or(4000);
    let temperature = request.temperature.unwrap_or(0.1);

    // Use the retry config but with fewer attempts for interactive use
    let retry_config = crate::workflow::models::RetryConfig {
        max_attempts: 1,
        initial_delay_ms: 1000,
        max_delay_ms: 5000,
        backoff_multiplier: 2.0,
    };

    match state.engine.anthropic_service
        .call_anthropic(&AnthropicCallConfig {
            model: &model,
            max_tokens,
            temperature,
            system_prompt: Some(&request.system_prompt),
            user_prompt: &request.user_prompt,
            timeout_seconds: 120, // 120 second timeout for AI generation
            retry_config: &retry_config,
        }, &dummy_event)
        .await
    {
        Ok(result) => {
            // Extract the response from the event data
            if let Some(anthropic_response) = result.data.get("anthropic_response") {
                let response = anthropic_response.as_str().unwrap_or("").to_string();
                let usage = result.data.get("usage").cloned();

                Ok(Json(GenerateCodeResponse {
                    response,
                    usage,
                }))
            } else {
                tracing::error!("No anthropic_response found in result data");
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
        Err(e) => {
            tracing::error!("AI code generation failed: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn generate_workflow(
    State(state): State<AppState>,
    Json(request): Json<GenerateWorkflowRequest>,
) -> Result<Json<GenerateWorkflowResponse>, StatusCode> {
    tracing::info!("AI workflow generation request: {}", request.prompt);

    // Input validation
    if let Err(error) = validate_workflow_prompt(&request.prompt) {
        tracing::warn!("Invalid prompt for AI workflow generation: {}", error);
        return Ok(Json(GenerateWorkflowResponse {
            success: false,
            workflow_id: None,
            workflow_name: None,
            error: Some(error),
        }));
    }

    // Generate workflow using AI with comprehensive error handling
    match generate_workflow_with_ai(&state, &request.prompt).await {
        Ok(workflow_result) => {
            tracing::info!("Successfully created workflow: {}", workflow_result.workflow_id);
            Ok(Json(GenerateWorkflowResponse {
                success: true,
                workflow_id: Some(workflow_result.workflow_id),
                workflow_name: Some(workflow_result.workflow_name),
                error: None,
            }))
        }
        Err(error) => {
            tracing::error!("Failed to generate workflow: {}", error);
            Ok(Json(GenerateWorkflowResponse {
                success: false,
                workflow_id: None,
                workflow_name: None,
                error: Some(error.to_string()),
            }))
        }
    }
}

/// Validate user prompt for workflow generation
fn validate_workflow_prompt(prompt: &str) -> Result<(), String> {
    let trimmed = prompt.trim();

    if trimmed.is_empty() {
        return Err("Prompt cannot be empty".to_string());
    }

    if trimmed.len() < MIN_PROMPT_LENGTH {
        return Err(format!("Prompt must be at least {MIN_PROMPT_LENGTH} characters"));
    }

    if trimmed.len() > MAX_PROMPT_LENGTH {
        return Err(format!("Prompt must be less than {MAX_PROMPT_LENGTH} characters"));
    }

    // Check for potentially harmful content
    let dangerous_keywords = ["<script", "javascript:", "eval(", "document.", "window.", "exec("];
    for keyword in dangerous_keywords {
        if trimmed.to_lowercase().contains(keyword) {
            return Err("Prompt contains potentially unsafe content".to_string());
        }
    }

    Ok(())
}

#[derive(Debug)]
struct WorkflowGenerationResult {
    workflow_id: String,
    workflow_name: String,
}

/// Main AI workflow generation function with comprehensive error handling
async fn generate_workflow_with_ai(
    state: &AppState,
    prompt: &str,
) -> Result<WorkflowGenerationResult, Box<dyn std::error::Error + Send + Sync>> {
    tracing::debug!("Starting AI workflow generation for prompt: {}", prompt);

    // Create system prompt
    let system_prompt = create_workflow_generation_system_prompt();

    // Create event for Anthropic service
    let dummy_event = WorkflowEvent {
        data: serde_json::json!({}),
        metadata: std::collections::HashMap::new(),
        headers: std::collections::HashMap::new(),
        condition_results: std::collections::HashMap::new(),
    };

    // Configure retry for AI calls
    let retry_config = crate::workflow::models::RetryConfig {
        max_attempts: 2,
        initial_delay_ms: 1000,
        max_delay_ms: 5000,
        backoff_multiplier: 2.0,
    };

    // Call Anthropic AI service
    let ai_result = state.engine.anthropic_service
        .call_anthropic(&AnthropicCallConfig {
            model: "claude-3-5-sonnet-20241022",
            max_tokens: 4000,
            temperature: 0.2,
            system_prompt: Some(&system_prompt),
            user_prompt: prompt,
            timeout_seconds: 120,
            retry_config: &retry_config,
        }, &dummy_event)
        .await
        .map_err(|e| format!("AI service error: {e}"))?;

    // Extract AI response
    let ai_response_text = ai_result.data.get("anthropic_response")
        .and_then(|v| v.as_str())
        .ok_or("No AI response received")?;

    tracing::debug!("AI response received, length: {}", ai_response_text.len());

    // Parse AI response with fallback strategies
    let workflow_spec = parse_ai_workflow_response(ai_response_text)
        .map_err(|e| format!("Failed to parse AI response: {e}"))?;

    // Validate workflow complexity
    validate_workflow_complexity(&workflow_spec.nodes)
        .map_err(|e| format!("Workflow validation failed: {e}"))?;

    // Create workflow in database
    let workflow_result = create_workflow_from_ai_spec(workflow_spec, state).await
        .map_err(|e| format!("Failed to create workflow: {e}"))?;

    tracing::info!("AI workflow generation completed successfully");
    Ok(workflow_result)
}

/// Validate workflow complexity to prevent resource exhaustion
fn validate_workflow_complexity(nodes: &[NodeRequest]) -> Result<(), String> {
    if nodes.is_empty() {
        return Err("Workflow must contain at least one node".to_string());
    }

    if nodes.len() > 20 {
        return Err("Workflow is too complex (maximum 20 nodes allowed)".to_string());
    }

    let trigger_count = nodes.iter().filter(|n| matches!(n.node_type, NodeType::Trigger { .. })).count();
    if trigger_count != 1 {
        return Err("Workflow must have exactly one trigger node".to_string());
    }

    Ok(())
}

fn create_workflow_generation_system_prompt() -> String {
    r#"You are an expert SwissPipe workflow designer. SwissPipe is a workflow automation platform that processes data through DAG-based node execution.

Your task is to generate a complete workflow specification based on user requirements. You must respond ONLY with valid JSON in this exact format:

{
  "name": "Workflow Name",
  "description": "Brief description",
  "start_node_id": "uuid-of-trigger-node",
  "nodes": [
    {
      "id": "unique-uuid",
      "name": "Node Display Name",
      "node_type": { /* node type specification */ },
      "position_x": 100.0,
      "position_y": 50.0
    }
  ],
  "edges": [
    {
      "from_node_id": "source-uuid",
      "to_node_id": "target-uuid",
      "condition_result": null // or true/false for conditional edges
    }
  ]
}

## Node Types Available:

### 1. Trigger (Entry Point)
```json
{
  "Trigger": {
    "methods": ["Get", "Post", "Put"] // HTTP methods to accept
  }
}
```

### 2. Transformer (Data Processing)
```json
{
  "Transformer": {
    "script": "function transformer(event) { return {...event, data: {...event.data, processed: true}}; }"
  }
}
```

### 3. Condition (Flow Control)
```json
{
  "Condition": {
    "script": "function condition(event) { return event.data.status === 'active'; }"
  }
}
```

### 4. HTTP Request (External API Calls)
```json
{
  "HttpRequest": {
    "url": "https://api.example.com/endpoint",
    "method": "Post", // "Get", "Post", "Put"
    "timeout_seconds": 30,
    "failure_action": "Stop", // "Stop", "Continue", "Retry"
    "headers": {},
    "retry_config": {
      "max_attempts": 3,
      "initial_delay_ms": 100,
      "max_delay_ms": 5000,
      "backoff_multiplier": 2
    }
  }
}
```

### 5. Email (Notifications)
```json
{
  "Email": {
    "config": {
      "to": ["user@example.com"],
      "cc": [],
      "bcc": [],
      "subject": "Alert: {{data.title}}",
      "body": "Message: {{data.message}}"
    }
  }
}
```

### 6. Delay (Scheduling)
```json
{
  "Delay": {
    "duration": 30,
    "unit": "Seconds" // "Seconds", "Minutes", "Hours", "Days"
  }
}
```

### 7. Anthropic AI (AI Processing)
```json
{
  "Anthropic": {
    "model": "claude-3-5-sonnet-20241022",
    "max_tokens": 1000,
    "temperature": 0.7,
    "system_prompt": "You are a data analyst",
    "user_prompt": "Analyze this data: {{data}}",
    "timeout_seconds": 120,
    "failure_action": "Stop",
    "retry_config": {
      "max_attempts": 3,
      "initial_delay_ms": 1000,
      "max_delay_ms": 10000,
      "backoff_multiplier": 2
    }
  }
}
```

### 8. OpenObserve (Log Ingestion)
```json
{
  "OpenObserve": {
    "url": "https://observe.example.com/api/logs",
    "authorization_header": "Bearer token",
    "timeout_seconds": 30,
    "failure_action": "Continue",
    "retry_config": {
      "max_attempts": 2,
      "initial_delay_ms": 500,
      "max_delay_ms": 2000,
      "backoff_multiplier": 1.5
    }
  }
}
```

## Design Guidelines:
1. Always start with a Trigger node as the entry point
2. Use meaningful node names and workflow descriptions
3. Position nodes logically (x: 100-800, y: 50-400)
4. Space nodes ~170px apart horizontally, ~150px vertically
5. Use Transformer nodes to modify data structure
6. Use Condition nodes for branching logic with conditional edges
7. Include error handling via failure_action settings
8. Use template variables like {{data.field}} in prompts and messages
9. Generate realistic UUIDs for all IDs
10. Keep workflows focused and logical
11. Position nodes vertically (Not horizaontally) to represent data flow direction

Examples of good workflows:
- API data → Transform → Send email notification
- Webhook trigger → Condition check → Branch to different actions
- Data ingestion → AI processing → Log results
- Form submission → Validation → Multiple notification channels

IMPORTANT: Respond ONLY with the JSON workflow specification. No explanations or markdown formatting."#.to_string()
}

/// Parse AI workflow response with comprehensive fallback strategies
fn parse_ai_workflow_response(response: &str) -> Result<CreateWorkflowRequest, Box<dyn std::error::Error + Send + Sync>> {
    tracing::debug!("Parsing AI response, length: {}", response.len());

    // Stage 1: Clean response with multiple strategies
    let cleaned = clean_ai_response(response);
    tracing::debug!("Cleaned response length: {}", cleaned.len());

    // Stage 2: Parse JSON with fallback strategies
    let parsed = parse_json_with_fallbacks(&cleaned)
        .map_err(|e| format!("JSON parsing failed: {e}"))?;

    // Stage 3: Extract and validate workflow components
    let workflow_spec = extract_workflow_components(parsed)
        .map_err(|e| format!("Component extraction failed: {e}"))?;

    // Stage 4: Replace AI-generated UUIDs with unique ones
    let workflow_spec = replace_ai_uuids_with_unique(workflow_spec);

    // Stage 5: Final validation
    validate_workflow_structure(&workflow_spec)
        .map_err(|e| format!("Workflow validation failed: {e}"))?;

    tracing::info!("Successfully parsed AI workflow: '{}' with {} nodes",
        workflow_spec.name, workflow_spec.nodes.len());

    Ok(workflow_spec)
}

/// Clean AI response with multiple strategies
fn clean_ai_response(response: &str) -> String {
    let mut cleaned = response.trim().to_string();

    // Remove code block markers
    if cleaned.starts_with("```json") {
        cleaned = cleaned.strip_prefix("```json").unwrap().to_string();
    } else if cleaned.starts_with("```") {
        cleaned = cleaned.strip_prefix("```").unwrap().to_string();
    }

    if cleaned.ends_with("```") {
        cleaned = cleaned.strip_suffix("```").unwrap().to_string();
    }

    // Remove common AI response artifacts
    cleaned = cleaned
        .replace("// HTTP methods to accept", "")
        .replace("/* node type specification */", "")
        .replace("// Example comment", "");

    // Remove comment lines
    let lines: Vec<&str> = cleaned
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect();

    lines.join("\n").trim().to_string()
}

/// Parse JSON with multiple fallback strategies
fn parse_json_with_fallbacks(text: &str) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    // Strategy 1: Direct parsing
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(text) {
        return Ok(value);
    }

    // Strategy 2: Extract JSON from mixed content
    if let Ok(value) = extract_json_from_mixed_response(text) {
        return Ok(value);
    }

    // Strategy 3: Try fixing common JSON issues
    let fixed_text = fix_common_json_issues(text);
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&fixed_text) {
        return Ok(value);
    }

    // Strategy 4: Last resort - create minimal valid structure
    tracing::warn!("All JSON parsing strategies failed, creating fallback structure");
    Ok(create_fallback_workflow_json(text))
}

/// Fix common JSON formatting issues
fn fix_common_json_issues(text: &str) -> String {
    text
        .replace(",\n}", "\n}")  // Remove trailing commas
        .replace(",\n]", "\n]")   // Remove trailing commas in arrays
        .replace("'", "\"")       // Replace single quotes with double quotes
        .replace("True", "true")  // Fix Python-style booleans
        .replace("False", "false")
        .replace("None", "null")  // Fix Python-style null
}

/// Create fallback workflow structure when parsing fails
fn create_fallback_workflow_json(prompt_context: &str) -> serde_json::Value {
    use uuid::Uuid;

    let trigger_id = Uuid::new_v4().to_string();
    let webhook_id = Uuid::new_v4().to_string();

    serde_json::json!({
        "name": format!("AI Generated Workflow from: {}", prompt_context.chars().take(50).collect::<String>()),
        "description": "Fallback workflow created when AI response parsing failed",
        "start_node_id": trigger_id,
        "nodes": [
            {
                "id": trigger_id,
                "name": "Trigger",
                "node_type": {
                    "Trigger": {
                        "methods": ["Post"]
                    }
                },
                "position_x": 100.0,
                "position_y": 50.0
            },
            {
                "id": webhook_id,
                "name": "Webhook Response",
                "node_type": {
                    "HttpRequest": {
                        "url": "https://httpbin.org/post",
                        "method": "Post",
                        "timeout_seconds": 30,
                        "failure_action": "Continue",
                        "headers": {},
                        "retry_config": {
                            "max_attempts": 3,
                            "initial_delay_ms": 1000,
                            "max_delay_ms": 5000,
                            "backoff_multiplier": 2
                        }
                    }
                },
                "position_x": 300.0,
                "position_y": 50.0
            }
        ],
        "edges": [
            {
                "from_node_id": trigger_id,
                "to_node_id": webhook_id,
                "condition_result": null
            }
        ]
    })
}

/// Extract workflow components from parsed JSON
fn extract_workflow_components(parsed: serde_json::Value) -> Result<CreateWorkflowRequest, Box<dyn std::error::Error + Send + Sync>> {
    let name = parsed.get("name")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'name' field")?
        .to_string();

    let description = parsed.get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let start_node_id = parsed.get("start_node_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Parse nodes
    let nodes_json = parsed.get("nodes")
        .and_then(|v| v.as_array())
        .ok_or("Missing or invalid 'nodes' field")?;

    let mut nodes = Vec::new();
    for (index, node_json) in nodes_json.iter().enumerate() {
        let node_id = node_json.get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let node_name = node_json.get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(&format!("Node {}", index + 1))
            .to_string();

        let node_type: NodeType = serde_json::from_value(
            node_json.get("node_type")
                .ok_or(format!("Node '{node_name}' missing 'node_type' field"))?
                .clone()
        ).map_err(|e| format!("Failed to parse node_type for '{node_name}': {e}"))?;

        let position_x = node_json.get("position_x")
            .and_then(|v| v.as_f64());

        let position_y = node_json.get("position_y")
            .and_then(|v| v.as_f64());

        nodes.push(NodeRequest {
            id: node_id,
            name: node_name,
            node_type,
            position_x,
            position_y,
        });
    }

    // Parse edges
    let empty_vec = vec![];
    let edges_json = parsed.get("edges")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty_vec);

    let mut edges = Vec::new();
    for edge_json in edges_json {
        let from_node_id = edge_json.get("from_node_id")
            .and_then(|v| v.as_str())
            .ok_or("Edge missing 'from_node_id' field")?
            .to_string();

        let to_node_id = edge_json.get("to_node_id")
            .and_then(|v| v.as_str())
            .ok_or("Edge missing 'to_node_id' field")?
            .to_string();

        let condition_result = edge_json.get("condition_result")
            .and_then(|v| v.as_bool());

        edges.push(EdgeRequest {
            from_node_id,
            to_node_id,
            condition_result,
        });
    }

    Ok(CreateWorkflowRequest {
        name,
        description,
        start_node_id,
        nodes,
        edges,
    })
}

/// Validate workflow structure after parsing
fn validate_workflow_structure(workflow: &CreateWorkflowRequest) -> Result<(), String> {
    if workflow.name.is_empty() {
        return Err("Workflow name cannot be empty".to_string());
    }

    if workflow.nodes.is_empty() {
        return Err("Workflow must contain at least one node".to_string());
    }

    // Ensure all edge references point to valid nodes
    let node_ids: std::collections::HashSet<String> = workflow.nodes
        .iter()
        .filter_map(|n| n.id.as_ref())
        .cloned()
        .collect();

    for edge in &workflow.edges {
        if !node_ids.contains(&edge.from_node_id) {
            return Err(format!("Edge references non-existent from_node_id: {}", edge.from_node_id));
        }
        if !node_ids.contains(&edge.to_node_id) {
            return Err(format!("Edge references non-existent to_node_id: {}", edge.to_node_id));
        }
    }

    Ok(())
}

/// Extract JSON from mixed text response (fallback parsing strategy)
fn extract_json_from_mixed_response(text: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    // Look for JSON object pattern
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            let json_candidate = &text[start..=end];
            return serde_json::from_str(json_candidate).map_err(|e| e.into());
        }
    }

    // If no complete JSON found, return error
    Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "No valid JSON found in response").into())
}

/// Replace AI-generated UUIDs with programmatically generated unique ones
fn replace_ai_uuids_with_unique(mut workflow_spec: CreateWorkflowRequest) -> CreateWorkflowRequest {
    tracing::debug!("Replacing AI-generated UUIDs with unique ones");

    // Create mapping from old UUIDs to new unique UUIDs
    let mut uuid_mapping: HashMap<String, String> = HashMap::new();

    // First pass: generate unique UUIDs for all nodes
    for node in &mut workflow_spec.nodes {
        if let Some(old_id) = node.id.clone() {
            let new_id = Uuid::new_v4().to_string();
            uuid_mapping.insert(old_id.clone(), new_id.clone());
            node.id = Some(new_id.clone());
            tracing::debug!("Mapped node UUID: {} -> {}", old_id, new_id);
        } else {
            // Generate UUID for nodes without IDs
            let new_id = Uuid::new_v4().to_string();
            node.id = Some(new_id.clone());
            tracing::debug!("Generated UUID for node without ID: {}", new_id);
        }
    }

    // Update start_node_id if it exists in our mapping
    if let Some(old_start_id) = workflow_spec.start_node_id.clone() {
        if let Some(new_start_id) = uuid_mapping.get(&old_start_id) {
            workflow_spec.start_node_id = Some(new_start_id.clone());
            tracing::debug!("Updated start_node_id: {} -> {}", old_start_id, new_start_id);
        }
    }

    // Second pass: update edge references to use new UUIDs
    for edge in &mut workflow_spec.edges {
        if let Some(new_from_id) = uuid_mapping.get(&edge.from_node_id) {
            let old_from = edge.from_node_id.clone();
            edge.from_node_id = new_from_id.clone();
            tracing::debug!("Updated edge from_node_id: {} -> {}", old_from, new_from_id);
        }

        if let Some(new_to_id) = uuid_mapping.get(&edge.to_node_id) {
            let old_to = edge.to_node_id.clone();
            edge.to_node_id = new_to_id.clone();
            tracing::debug!("Updated edge to_node_id: {} -> {}", old_to, new_to_id);
        }
    }

    tracing::info!("Successfully replaced {} AI-generated UUIDs with unique ones", uuid_mapping.len());
    workflow_spec
}


/// Create workflow from AI-generated specification with full database integration
async fn create_workflow_from_ai_spec(
    spec: CreateWorkflowRequest,
    state: &AppState,
) -> Result<WorkflowGenerationResult, Box<dyn std::error::Error + Send + Sync>> {
    use uuid::Uuid;
    use sea_orm::{ActiveModelTrait, Set};
    use crate::database::{entities, nodes, edges};

    tracing::debug!("Creating workflow in database: {}", spec.name);

    let workflow_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp_micros();

    // Create workflow entity
    let workflow = entities::ActiveModel {
        id: Set(workflow_id.clone()),
        name: Set(spec.name.clone()),
        description: Set(spec.description.clone()),
        start_node_id: Set(spec.start_node_id.clone()),
        created_at: Set(now),
        updated_at: Set(now),
    };

    // Insert workflow
    workflow.insert(&*state.db)
        .await
        .map_err(|e| format!("Database error creating workflow: {e}"))?;

    // Insert nodes
    for node in &spec.nodes {
        let node_id = node.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());

        // Convert NodeType to string for node_type field
        let node_type_str = match &node.node_type {
            NodeType::Trigger { .. } => "trigger".to_string(),
            NodeType::Transformer { .. } => "transformer".to_string(),
            NodeType::Condition { .. } => "condition".to_string(),
            NodeType::HttpRequest { .. } => "http_request".to_string(),
            NodeType::Email { .. } => "email".to_string(),
            NodeType::Delay { .. } => "delay".to_string(),
            NodeType::Anthropic { .. } => "anthropic".to_string(),
            NodeType::OpenObserve { .. } => "openobserve".to_string(),
        };

        // Serialize full config to JSON
        let config_json = serde_json::to_string(&node.node_type)
            .map_err(|e| format!("Node config serialization error: {e}"))?;

        let node_entity = nodes::ActiveModel {
            id: Set(node_id),
            workflow_id: Set(workflow_id.clone()),
            name: Set(node.name.clone()),
            node_type: Set(node_type_str),
            config: Set(config_json),
            position_x: Set(node.position_x.unwrap_or(0.0)),
            position_y: Set(node.position_y.unwrap_or(0.0)),
            created_at: Set(now),
            input_merge_strategy: Set(None),
        };

        node_entity.insert(&*state.db)
            .await
            .map_err(|e| format!("Database error creating node '{}': {}", node.name, e))?;
    }

    // Insert edges
    for edge in &spec.edges {
        let edge_entity = edges::ActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            workflow_id: Set(workflow_id.clone()),
            from_node_id: Set(edge.from_node_id.clone()),
            to_node_id: Set(edge.to_node_id.clone()),
            condition_result: Set(edge.condition_result),
            created_at: Set(now),
        };

        edge_entity.insert(&*state.db)
            .await
            .map_err(|e| format!("Database error creating edge: {e}"))?;
    }

    tracing::info!("Successfully created workflow '{}' with {} nodes and {} edges",
        spec.name, spec.nodes.len(), spec.edges.len());

    Ok(WorkflowGenerationResult {
        workflow_id,
        workflow_name: spec.name,
    })
}

#[derive(Debug, Deserialize)]
pub struct UpdateWorkflowRequest {
    pub workflow_id: String,
    pub prompt: String,
}

#[derive(Debug, Serialize)]
pub struct UpdateWorkflowResponse {
    pub success: bool,
    pub message: String,
    pub workflow_name: Option<String>,
    pub changes_made: Vec<String>,
    pub error: Option<String>,
}

pub async fn update_workflow(
    _state: State<AppState>,
    Json(request): Json<UpdateWorkflowRequest>,
) -> Result<Json<UpdateWorkflowResponse>, StatusCode> {
    tracing::info!("AI workflow update request for workflow {}: {}", request.workflow_id, request.prompt);

    // Return a temporary response until we fully implement this
    Ok(Json(UpdateWorkflowResponse {
        success: true,
        message: "AI workflow updates are not yet implemented. This is a placeholder response.".to_string(),
        workflow_name: Some("Current Workflow".to_string()),
        changes_made: vec![
            "Feature under development".to_string(),
            "Please check back later".to_string(),
        ],
        error: None,
    }))
}

// TODO: Implement full workflow update functionality
// For now, these functions are commented out to avoid compilation issues

// fn create_update_workflow_system_prompt(
//     workflow: &entities::Model,
//     nodes: &[nodes::Model],
//     edges: &[edges::Model],
// ) -> String { ... }

// fn contains_malicious_content(input: &str) -> bool { ... }