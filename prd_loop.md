# Native Loop Support for HTTP Nodes - Product Requirements Document

## Overview
Add native looping capabilities to HTTP nodes in SwissPipe workflows, enabling periodic HTTP requests with configurable intervals, retry logic, and termination conditions.

## Problem Statement
Currently, SwissPipe workflows lack native support for repeating HTTP operations, forcing users to:
- Implement complex JavaScript loops in condition nodes with manual `sleep()` calls
- Create multiple duplicate HTTP nodes for polling scenarios
- Handle timing and iteration logic manually in scripts
- Manage long-running polling operations without proper state management

**Real-world Use Cases:**
- **Customer onboarding**: Check data ingestion status every hour for 3 days
- **API polling**: Monitor external service status until completion
- **Health monitoring**: Periodic endpoint checks with failure detection
- **Data synchronization**: Retry failed API calls with backoff intervals

## Solution
Implement native loop support for HTTP nodes with declarative configuration, built-in scheduling, and proper state management.

## User Stories

### Primary User Stories
- As a workflow designer, I want to configure an HTTP node to poll an API every hour for 3 days without writing JavaScript loops
- As a developer, I want HTTP nodes to automatically retry failed requests with exponential backoff until success or max attempts
- As a system administrator, I want to monitor long-running HTTP polling operations and see their current iteration status
- As a workflow user, I want HTTP loops to terminate gracefully when success conditions are met

### Secondary User Stories
- As a workflow designer, I want to set dynamic loop conditions based on HTTP response content
- As a developer, I want HTTP loops to emit different outputs based on success/timeout/failure scenarios
- As a workflow user, I want HTTP loop progress to be visible in execution traces

## Functional Requirements

### Core Features

#### 1. **Loop Configuration**
```json
{
  "type": "http-request",
  "url": "https://api.example.com/status",
  "method": "GET",
  "loop_config": {
    "max_iterations": 72,
    "interval_seconds": 3600,
    "termination_conditions": [
      {
        "condition_type": "ResponseContent",
        "expression": "response.status === 'completed'",
        "action": "Success"
      },
      {
        "condition_type": "ResponseStatus",
        "expression": "status_code === 200",
        "action": "Success"
      },
      {
        "condition_type": "ConsecutiveFailures",
        "expression": "count >= 5",
        "action": "Failure"
      }
    ]
  }
}
```

#### 2. **Loop Execution Modes**
- **Fixed Interval**: Execute every N seconds/minutes/hours
- **Exponential Backoff**: Increase delay between iterations on failures
- **Custom Schedule**: Cron-like expressions for complex timing
- **Immediate Retry**: Continue immediately on specific response conditions

#### 3. **Termination Conditions**
- **Success Conditions**: Response content, HTTP status, custom JavaScript
- **Failure Conditions**: Max iterations, consecutive failures, timeout

#### 4. **State Management**
- **Persistent Loop State**: Current iteration, next execution time, failure count
- **Resumption Support**: Continue loops after application restart
- **Progress Tracking**: Real-time iteration status in workflow executions

### Advanced Features

#### 5. **Dynamic Configuration**
- **Context-Aware Looping**: Use workflow data to influence loop behavior
- **Conditional Loop Entry**: Start looping only when specific conditions are met

#### 6. **Output Differentiation**
```json
{
  "outputs": {
    "success": "Loop terminated due to success condition",
    "timeout": "Loop terminated due to max iterations",
    "failure": "Loop terminated due to failure condition"
  }
}
```

## Technical Requirements

### Integration with Existing HTTP Nodes

#### **Backward Compatibility Strategy**

HTTP loop functionality integrates seamlessly with existing HTTP nodes without breaking changes:

```rust
// Enhanced NodeType with optional loop support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeType {
    HttpRequest {
        url: String,
        method: HttpMethod,
        timeout_seconds: u64,
        failure_action: FailureAction,
        retry_config: RetryConfig,
        headers: HashMap<String, String>,
        loop_config: Option<LoopConfig>,  // ← NEW: Optional loop support
    },
    // ... other node types unchanged
}
```

**Compatibility Guarantee:**
- ✅ `loop_config: None` maintains exact existing behavior
- ✅ Same `WorkflowEvent` input/output format
- ✅ No changes to existing workflow execution flow
- ✅ Zero migration effort for existing workflows

#### **Enhanced HTTP Node Executor**

```rust
// src/workflow/engine/node_executor.rs
async fn execute_http_request_node(
    &self,
    config: &HttpRequestConfig<'_>,
    event: WorkflowEvent,
) -> Result<WorkflowEvent> {
    match &config.loop_config {
        // Standard HTTP request (existing behavior)
        None => {
            self.execute_single_http_request(config, event).await
        }

        // New loop HTTP request
        Some(loop_config) => {
            self.execute_http_loop(config, event, loop_config).await
        }
    }
}
```

#### **Loop Output Format**

Loop nodes produce `WorkflowEvent` output compatible with existing node expectations:

```rust
// Success case: HTTP response becomes event data
WorkflowEvent {
    data: successful_api_response,     // Last successful HTTP response JSON
    metadata: {
        "loop_completed": "true",
        "loop_iterations": "15",
        "loop_termination_reason": "Success",
        "loop_success_rate": "0.87",
        "last_http_status": "200",
        // ... original metadata preserved
    },
    headers: original_event.headers,   // Preserved
    condition_results: original_event.condition_results, // Preserved
}

// Timeout/Failure case: Original data preserved
WorkflowEvent {
    data: original_event.data,         // Original input data
    metadata: {
        "loop_completed": "true",
        "loop_iterations": "72",
        "loop_termination_reason": "MaxIterations",
        "loop_success_rate": "0.00",
        // ... original metadata preserved
    },
    // ... other fields preserved
}
```

#### **Next Node Integration Examples**

Downstream nodes can detect and process loop results using metadata:

```javascript
// Condition node: Route based on loop outcome
function condition(event) {
    const terminationReason = event.metadata.loop_termination_reason;

    if (terminationReason === "Success") {
        return true;  // → Send congrats email
    } else if (terminationReason === "MaxIterations") {
        return false; // → Send reminder email
    }

    // Standard HTTP node (no loop metadata)
    return event.data.status === "completed";
}

// Transformer node: Process loop results
function transformer(event) {
    // Handle both loop and non-loop HTTP responses
    const baseData = {
        ...event.data,
        processed_at: new Date().toISOString()
    };

    // Add loop summary if this came from a loop
    if (event.metadata.loop_completed === "true") {
        baseData.loop_summary = {
            iterations: parseInt(event.metadata.loop_iterations),
            success_rate: parseFloat(event.metadata.loop_success_rate),
            outcome: event.metadata.loop_termination_reason
        };
    }

    return baseData;
}
```

### Backend Implementation

#### 1. **Enhanced Node Type**
```rust
// src/workflow/models.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopConfig {
    pub max_iterations: Option<u32>,
    pub interval_seconds: u64,
    pub backoff_strategy: BackoffStrategy,
    pub termination_conditions: Vec<TerminationCondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackoffStrategy {
    Fixed(u64),                    // Fixed interval
    Exponential { base: u64, multiplier: f64, max: u64 },
    Custom(String),                // JavaScript expression
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminationCondition {
    pub condition_type: ConditionType,
    pub expression: String,        // JavaScript expression
    pub action: TerminationAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionType {
    ResponseContent,    // Check response body
    ResponseStatus,     // Check HTTP status
    ConsecutiveFailures, // Count failed attempts
    TotalTime,         // Total loop duration
    Custom,            // JavaScript condition
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TerminationAction {
    Success,   // Emit success signal and continue workflow
    Failure,   // Emit failure signal and continue workflow
    Stop,      // Stop workflow execution
}
```

#### 2. **Loop State Database Schema**
```sql
-- New table: http_loop_states
CREATE TABLE http_loop_states (
    id VARCHAR PRIMARY KEY,
    execution_id VARCHAR NOT NULL,
    node_id VARCHAR NOT NULL,
    current_iteration INTEGER NOT NULL DEFAULT 0,
    next_execution_at BIGINT, -- Unix epoch microseconds
    consecutive_failures INTEGER NOT NULL DEFAULT 0,
    loop_started_at BIGINT NOT NULL,
    last_response_status INTEGER,
    last_response_body TEXT,
    status VARCHAR NOT NULL, -- 'running', 'completed', 'failed'
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL
);
```

#### 3. **Loop Scheduler Service**
```rust
// src/async_execution/http_loop_scheduler.rs
pub struct HttpLoopScheduler {
    db: Arc<DatabaseConnection>,
    loop_tasks: Arc<RwLock<HashMap<String, JoinHandle<()>>>>,
}

impl HttpLoopScheduler {
    pub async fn schedule_http_loop(&self, config: HttpLoopConfig) -> Result<String>;
    pub async fn get_loop_status(&self, loop_id: &str) -> Result<LoopStatus>;
}
```

#### 4. **API Endpoints**
- **GET** `/api/v1/loops/{loop_id}/status` - Get loop status
- **GET** `/api/v1/executions/{execution_id}/loops` - List loops for execution

### Frontend Implementation

#### 1. **HTTP Node Configuration UI**
```vue
<!-- HttpRequestConfig.vue enhancement -->
<div v-if="config.loop_config?.enabled" class="mt-4 border-t pt-4">
  <h4 class="text-sm font-medium text-gray-300 mb-3">Loop Configuration</h4>

  <div class="grid grid-cols-2 gap-4">
    <div>
      <label class="block text-sm text-gray-300 mb-1">Max Iterations</label>
      <input v-model.number="config.loop_config.max_iterations"
             type="number"
             class="w-full bg-slate-700 border border-slate-600 text-gray-100 px-3 py-2 rounded-md" />
    </div>

    <div>
      <label class="block text-sm text-gray-300 mb-1">Interval (seconds)</label>
      <input v-model.number="config.loop_config.interval_seconds"
             type="number"
             class="w-full bg-slate-700 border border-slate-600 text-gray-100 px-3 py-2 rounded-md" />
    </div>
  </div>

  <div class="mt-4">
    <label class="block text-sm text-gray-300 mb-2">Success Condition (JavaScript)</label>
    <CodeEditor v-model="successCondition"
                language="javascript"
                :height="100"
                placeholder="return response.data.status === 'completed';" />
  </div>
</div>
```

#### 2. **Loop Status Visualization**
```vue
<!-- New component: HttpLoopStatus.vue -->
<template>
  <div class="bg-slate-800 rounded-lg p-4 border border-slate-700">
    <div class="flex items-center justify-between mb-3">
      <h3 class="text-sm font-medium text-gray-200">HTTP Loop Status</h3>
      <span :class="statusBadgeClass" class="px-2 py-1 rounded text-xs">
        {{ loop_status.status }}
      </span>
    </div>

    <div class="space-y-2 text-sm">
      <div class="flex justify-between">
        <span class="text-gray-400">Progress:</span>
        <span class="text-gray-200">
          {{ loop_status.current_iteration }} / {{ loop_status.max_iterations || '∞' }}
        </span>
      </div>

      <div class="flex justify-between">
        <span class="text-gray-400">Next Execution:</span>
        <span class="text-gray-200">{{ formatNextExecution(loop_status.next_execution_at) }}</span>
      </div>

      <div class="flex justify-between">
        <span class="text-gray-400">Consecutive Failures:</span>
        <span :class="loop_status.consecutive_failures > 0 ? 'text-red-400' : 'text-gray-200'">
          {{ loop_status.consecutive_failures }}
        </span>
      </div>
    </div>

  </div>
</template>
```

#### 3. **Workflow Canvas Enhancement**
- **Loop Indicator**: Visual indicator on HTTP nodes with active loops
- **Progress Ring**: Show completion percentage around node
- **Status Badges**: Running/Paused/Completed status display

## Non-Functional Requirements

### Performance
- **Minimal Resource Usage**: Efficient timer-based scheduling without active polling
- **Scalability**: Support unlimited concurrent HTTP loops per instance
- **Database Optimization**: Indexed queries for loop state retrieval
- **Memory Management**: Automatic cleanup of completed loop states

### Reliability
- **Fault Tolerance**: Resume loops after application restart
- **State Consistency**: ACID compliance for loop state updates
- **Error Recovery**: Graceful handling of network failures and timeouts
- **Resource Efficiency**: Each loop operates independently without resource contention

### Security
- **Rate Limiting**: Prevent abuse of loop functionality
- **Resource Quotas**: Limit loop iterations and duration per user
- **Audit Logging**: Track loop creation, modification, and termination
- **Permission Control**: Role-based access to loop management APIs

### Monitoring
- **Metrics Collection**: Loop performance and failure rate metrics
- **Health Checks**: Monitor loop scheduler service health
- **Alerting**: Notifications for stuck or failing loops
- **Resource Monitoring**: Track CPU/memory usage of loop operations

## Success Metrics
- **Adoption Rate**: 40% of HTTP nodes use loop functionality within 3 months
- **Performance**: Loop scheduling latency < 100ms, memory usage < 10MB per 100 loops
- **Reliability**: 99.9% loop execution success rate, < 1 second restart recovery time
- **User Experience**: 4.5+ rating for loop configuration usability

## Implementation Phases

### Phase 1: Core Loop Infrastructure (Weeks 1-2)
- Enhanced HTTP node data model with loop configuration
- Backward compatibility implementation for existing HTTP nodes
- Database schema for loop state management
- Basic loop scheduler with fixed intervals
- Loop state persistence and resumption

### Phase 2: Advanced Loop Features (Weeks 3-4)
- Termination conditions and dynamic configuration
- Exponential backoff and custom scheduling
- Loop status API endpoints
- Error handling and failure recovery

### Phase 3: Frontend Integration (Weeks 5-6)
- HTTP node configuration UI for loop settings (backward compatible)
- Loop status visualization components
- Enhanced workflow canvas with loop indicators
- Integration with existing HTTP node properties panel
- Real-time status updates

### Phase 4: Advanced UI & Monitoring (Weeks 7-8)
- Advanced termination condition builder
- Loop performance dashboards
- Integration testing and optimization

### Phase 5: Production Readiness (Weeks 9-10)
- Load testing and performance optimization
- Security auditing and rate limiting
- Documentation and user guides
- Consistency validation and integration testing
- Beta testing with real workflows

## Future Enhancements
- **Webhook Integration**: Trigger loops from external events
- **Adaptive Intervals**: Dynamic interval adjustment based on response patterns
- **Distributed Loops**: Execute loops across multiple SwissPipe instances
- **Loop Templates**: Predefined configurations for common use cases
- **Visual Loop Builder**: Drag-and-drop interface for complex loop logic
- **Integration Marketplace**: Pre-configured loops for popular APIs (Stripe, Salesforce, etc.)

## Risk Mitigation

### Technical Risks
- **Resource Exhaustion**: Implement strict limits and monitoring
- **State Corruption**: Use database transactions and validation
- **Timing Drift**: Implement precise scheduling with drift compensation
- **Loop Runaway**: Enforce maximum iteration limits and automatic termination

### Business Risks
- **Complexity Overload**: Progressive disclosure in UI, start with simple use cases
- **Performance Impact**: Thorough load testing and resource optimization
- **User Confusion**: Comprehensive documentation and examples
- **Breaking Changes**: Rigorous backward compatibility testing with existing workflows

## Consistency Standards

To ensure implementation consistency across all components, the following standards must be maintained:

### **Data Format Standards**

#### **Termination Conditions Schema**
```json
{
  "condition_type": "ResponseContent|ResponseStatus|ConsecutiveFailures|TotalTime|Custom",
  "expression": "JavaScript expression string",
  "action": "Success|Failure|Stop"
}
```

#### **Loop Configuration Schema**
```json
{
  "max_iterations": 72,              // Optional u32
  "interval_seconds": 3600,          // Required u64
  "backoff_strategy": {
    "Fixed": 3600                    // OR
    "Exponential": {"base": 30, "multiplier": 1.5, "max": 300}
  },
  "termination_conditions": [...]    // Array of TerminationCondition
}
```

#### **Metadata Key Naming**
All metadata keys use **snake_case**:
- `loop_completed`: "true|false"
- `loop_iterations`: "15"
- `loop_termination_reason`: "Success|MaxIterations|Failure"
- `loop_success_rate`: "0.87"
- `last_http_status`: "200"

#### **Status Value Standards**
- **Database/API**: lowercase (`"running"`, `"completed"`, `"failed"`)
- **Termination Reasons**: PascalCase (`"Success"`, `"MaxIterations"`, `"Failure"`)
- **UI Variables**: snake_case (`loop_status.status`, `loop_status.consecutive_failures`)

### **API Path Standards**
- Loop status: `GET /api/v1/loops/{loop_id}/status`
- Execution loops: `GET /api/v1/executions/{execution_id}/loops`

### **Loop Enabling Pattern**
- Loops enabled by presence of `loop_config` field
- No explicit `"enabled": true` field required
- `loop_config: Option<LoopConfig>` in Rust

### **Validation Checklist**
- [ ] All termination conditions use standard schema
- [ ] All metadata keys use snake_case naming
- [ ] All status values follow case standards
- [ ] All API endpoints use standard path structure
- [ ] All configuration examples are consistent
- [ ] All code examples use same JSON formatting

## Detailed Configuration Examples

### Example 1: Migration Path - Existing to Loop HTTP Node

**Before (Existing HTTP Node):**
```json
{
  "type": "http-request",
  "url": "https://api.example.com/customers/{{customer_id}}/status",
  "method": "GET",
  "timeout_seconds": 30,
  "failure_action": "Continue"
}
```

**After (Enhanced with Loop - Opt-in):**
```json
{
  "type": "http-request",
  "url": "https://api.example.com/customers/{{customer_id}}/status",
  "method": "GET",
  "timeout_seconds": 30,
  "failure_action": "Continue",
  "loop_config": {
    "max_iterations": 72,
    "interval_seconds": 3600,
    "termination_conditions": [
      {
        "condition_type": "ResponseContent",
        "expression": "response.data.has_ingested_data === true",
        "action": "Success"
      },
      {
        "condition_type": "ConsecutiveFailures",
        "expression": "count >= 3",
        "action": "Failure"
      }
    ]
  }
}
```

### Example 2: API Health Monitoring
```json
{
  "type": "http-request",
  "url": "https://api.service.com/health",
  "method": "GET",
  "loop_config": {
    "enabled": true,
    "interval_seconds": 30,
    "backoff_strategy": {
      "Exponential": {
        "base": 30,
        "multiplier": 1.5,
        "max": 300
      }
    },
    "termination_conditions": [
      {
        "condition_type": "ResponseStatus",
        "expression": "status_code === 200",
        "action": "Succeed"
      },
      {
        "condition_type": "TotalTime",
        "expression": "duration_seconds > 3600",
        "action": "Fail"
      }
    ]
  }
}
```

### Example 3: Data Synchronization with Retry
```json
{
  "type": "http-request",
  "url": "https://api.target.com/sync",
  "method": "POST",
  "loop_config": {
    "enabled": true,
    "max_iterations": 5,
    "backoff_strategy": {
      "Exponential": {
        "base": 1000,
        "multiplier": 2.0,
        "max": 30000
      }
    },
    "termination_conditions": [
      {
        "condition_type": "ResponseContent",
        "expression": "response.data.sync_status === 'completed'",
        "action": "Succeed"
      },
      {
        "condition_type": "ResponseStatus",
        "expression": "status_code >= 400 && status_code < 500",
        "action": "Fail"
      }
    ]
  }
}
```