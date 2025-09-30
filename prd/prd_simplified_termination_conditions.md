# PRD: Implement Single JavaScript Function Pattern for HTTP Loop Termination Conditions

## Overview

This PRD outlines the implementation of HTTP loop termination conditions using a unified JavaScript function pattern, consistent with condition nodes in the workflow engine. Since the current multiple expression approach has not been released to production, we can implement the optimal solution from the start.

## Current State Analysis

### Current Implementation (Pre-Production)
The HTTP loop termination conditions are currently implemented with 5 different condition types that have not been released to production:

1. **ResponseContent**: Evaluates response body with expressions like `response.data.status === "completed"`
2. **ResponseStatus**: Evaluates HTTP status codes with expressions like `status_code === 200`
3. **ConsecutiveFailures**: Evaluates failure counts with expressions like `count >= 3`
4. **TotalTime**: Evaluates loop duration with expressions like `duration_seconds > 3600`
5. **Custom**: Full JavaScript expressions with event context

### Current Backend Processing
Each condition type requires different evaluation logic in `src/async_execution/http_loop_scheduler.rs`:

- **ResponseContent**: Creates context with `response.data` and `event` variables
- **ResponseStatus**: Creates context with `status_code` variable
- **ConsecutiveFailures**: Creates context with `count` variable
- **TotalTime**: Creates context with `duration_seconds` and `elapsed_seconds` variables
- **Custom**: Creates context with `event` and `response` variables

All expressions are wrapped in IIFEs: `(function() { const variable = value; return (expression); })()`

### Current Frontend Implementation
- Located in `frontend/src/components/app-configs/HttpLoopConfig.vue`
- Complex UI with condition type dropdowns
- Different placeholder text for each condition type
- Multiple expression patterns that vary by type

## Problem Statement

**Issues with Current Approach:**
1. **Complexity**: 5 different condition types create unnecessary cognitive overhead
2. **Inconsistency**: Different from condition nodes which use unified `function condition(event)` pattern
3. **Maintenance Burden**: Complex backend evaluation logic with different context creation for each type
4. **User Experience**: Multiple dropdowns and varying expression patterns create confusion
5. **Suboptimal Architecture**: Not leveraging the existing optimized QuickJS `execute_condition` method

**Opportunity**: Since this hasn't been released to production, we can implement the optimal solution from the start without migration concerns.

## Proposed Solution

### Target State: Single JavaScript Function Pattern

Replace all termination condition types with a single pattern matching condition nodes:

```javascript
function condition(event) {
  // Access all loop context through event object
  // Return true to terminate loop, false to continue
  return boolean;
}
```

### Enhanced Event Object Structure

The `event` parameter will contain comprehensive loop context:

```javascript
{
  "data": { /* response data */ },
  "metadata": {
    "http_status": 200,
    "loop_iteration": 5,
    "consecutive_failures": 2,
    "loop_started_at": 1642694400000000,
    "current_timestamp": 1642694700000000,
    "elapsed_seconds": 300,
    "elapsed_micros": 300000000
  },
  "headers": { /* response headers */ },
  "condition_results": { /* previous conditions */ }
}
```

### Pattern Examples

Converting from current multiple patterns to unified pattern:

```javascript
// Current ResponseContent: response.data.status === "completed"
function condition(event) {
  return event.data.status === "completed";
}

// Current ResponseStatus: status_code === 200
function condition(event) {
  return event.metadata.http_status === 200;
}

// Current ConsecutiveFailures: count >= 3
function condition(event) {
  return event.metadata.consecutive_failures >= 3;
}

// Current TotalTime: duration_seconds > 3600
function condition(event) {
  return event.metadata.elapsed_seconds > 3600;
}

// Current Custom: complex expressions
function condition(event) {
  return event.data.has_ingested_data === true &&
         event.metadata.elapsed_seconds < 7200;
}
```

## Technical Implementation Plan

### Backend Changes

#### 1. Modify Data Models
**File**: `src/workflow/models.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminationCondition {
    pub script: String,           // JavaScript function
    pub action: TerminationAction, // Success, Failure, Stop
}

// Note: Loop configuration will use Option<TerminationCondition> instead of Vec<TerminationCondition>
// to enforce single condition per loop
```

#### 2. Update HTTP Loop Scheduler
**File**: `src/async_execution/http_loop_scheduler.rs`

Replace complex condition evaluation methods with unified single condition approach:

```rust
async fn should_terminate(
    js_executor: &JavaScriptExecutor,
    termination_condition: Option<&TerminationCondition>,
    response: &Result<WorkflowEvent>,
    consecutive_failures: i32,
    loop_started_at: i64,
    current_iteration: i32,
) -> Result<bool> {
    if let Some(condition) = termination_condition {
        // Create enhanced event with loop metadata
        let enhanced_event = create_loop_event_context(
            response, consecutive_failures, loop_started_at, current_iteration
        )?;

        // Use existing execute_condition method from JavaScriptExecutor
        match js_executor.execute_condition(&condition.script, &enhanced_event).await {
            Ok(true) => {
                tracing::info!("Termination condition met for action {:?}: {}", condition.action, condition.script.chars().take(100).collect::<String>());
                return Ok(true);
            }
            Ok(false) => {
                tracing::debug!("Termination condition not met: {}", condition.script.chars().take(100).collect::<String>());
                return Ok(false);
            }
            Err(e) => {
                tracing::warn!("Termination condition evaluation error: {} - Script: {}", e, condition.script.chars().take(100).collect::<String>());
                return Ok(false); // Don't fail loop on condition errors
            }
        }
    }
    Ok(false)
}

fn create_loop_event_context(
    response: &Result<WorkflowEvent>,
    consecutive_failures: i32,
    loop_started_at: i64,
    current_iteration: i32,
) -> Result<WorkflowEvent> {
    let now = chrono::Utc::now().timestamp_micros();
    let elapsed_micros = now - loop_started_at;

    let mut metadata = serde_json::Map::new();
    metadata.insert("loop_iteration".to_string(), current_iteration.into());
    metadata.insert("consecutive_failures".to_string(), consecutive_failures.into());
    metadata.insert("loop_started_at".to_string(), loop_started_at.into());
    metadata.insert("current_timestamp".to_string(), now.into());
    metadata.insert("elapsed_seconds".to_string(), (elapsed_micros / 1_000_000).into());
    metadata.insert("elapsed_micros".to_string(), elapsed_micros.into());

    match response {
        Ok(workflow_event) => {
            let mut enhanced_metadata = workflow_event.metadata.clone();
            enhanced_metadata.extend(metadata);

            Ok(WorkflowEvent {
                data: workflow_event.data.clone(),
                metadata: enhanced_metadata,
                headers: workflow_event.headers.clone(),
                condition_results: workflow_event.condition_results.clone(),
            })
        }
        Err(_) => {
            // For failed responses, create minimal event with loop metadata
            Ok(WorkflowEvent {
                data: serde_json::Value::Null,
                metadata,
                headers: HashMap::new(),
                condition_results: HashMap::new(),
            })
        }
    }
}
```


### Frontend Changes

#### 1. Implement Simplified HttpLoopConfig Component
**File**: `frontend/src/components/app-configs/HttpLoopConfig.vue`

Replace complex condition type selection with single termination condition editor:

```vue
<template>
  <!-- Single termination condition section -->
  <div>
    <label class="block text-sm font-medium text-gray-300 mb-3">Termination Condition</label>

    <div class="bg-slate-800 rounded-md p-3 border border-slate-700">
      <!-- JavaScript function editor and action selector side by side -->
      <div class="grid grid-cols-3 gap-4">
        <!-- Code editor (left side, takes 2/3 width) -->
        <div class="col-span-2">
          <label class="block text-xs text-gray-400 mb-1">Termination Function</label>
          <div class="h-64">
            <CodeEditor
              :modelValue="loopConfig.termination_condition.script"
              @update:modelValue="updateTerminationCondition('script', $event)"
              language="javascript"
            />
          </div>
          <p class="text-xs text-gray-500 mt-1">
            Function receives event object with data, metadata (loop_iteration, consecutive_failures, elapsed_seconds), and headers
          </p>
        </div>

        <!-- Action selector (right side, takes 1/3 width) -->
        <div>
          <label class="block text-xs text-gray-400 mb-1">Action</label>
          <select
            :value="loopConfig.termination_condition.action"
            @change="updateTerminationCondition('action', $event.target.value)"
            class="w-full bg-slate-700 border border-slate-600 text-gray-100 px-2 py-1 rounded text-sm"
          >
            <option value="Success">Success</option>
            <option value="Failure">Failure</option>
            <option value="Stop">Stop</option>
          </select>
        </div>
      </div>
    </div>
  </div>
</template>
```

#### 2. Update Type Definitions
**File**: `frontend/src/types/nodes.ts`

```typescript
export interface TerminationCondition {
  script: string            // JavaScript function
  action: TerminationAction // Success | Failure | Stop
}

// Note: LoopConfig will use termination_condition?: TerminationCondition (optional single condition)
// instead of termination_conditions: TerminationCondition[] (array)
```

#### 3. Implement Template Examples
**File**: `frontend/src/components/app-configs/HttpLoopConfig.vue`

```javascript
function applyTemplate(template: string) {
  let config: LoopConfig

  switch (template) {
    case 'customer-onboarding':
      config = {
        max_iterations: 72,
        interval_seconds: 3600,
        backoff_strategy: { Fixed: 3600 },
        termination_condition: {
          script: `function condition(event) {
  // Terminate successfully if data is ingested, fail after 3 consecutive failures
  if (event.data.has_ingested_data === true) return true;
  if (event.metadata.consecutive_failures >= 3) return true;
  return false;
}`,
          action: 'Success'
        }
      }
      break

    case 'health-monitoring':
      config = {
        interval_seconds: 30,
        backoff_strategy: { Exponential: { base: 30, multiplier: 1.5, max: 300 } },
        termination_condition: {
          script: `function condition(event) {
  // Succeed on HTTP 200, fail if running too long
  if (event.metadata.http_status === 200) return true;
  if (event.metadata.elapsed_seconds > 3600) return true;
  return false;
}`,
          action: 'Success'
        }
      }
      break
  }

  emit('update:modelValue', config)
}
```

## Implementation Strategy

### Phase 1: Backend Implementation
1. Update data models to use single JavaScript function pattern
2. Implement unified evaluation logic using existing `execute_condition`
3. Add comprehensive loop metadata to event context
4. Unit test the new evaluation logic
5. Integration test with workflow engine

### Phase 2: Frontend Implementation
1. Build simplified HttpLoopConfig component with CodeEditor
2. Update type definitions for new pattern
3. Implement template examples
4. Component testing and UI validation
5. Integration test with backend

### Phase 3: Testing and Documentation
1. End-to-end workflow testing
2. Update API documentation
3. Create user guide with examples
4. Performance validation
5. Final testing and deployment preparation

## Benefits

### Technical Benefits
1. **Clean Architecture**: Implement optimal solution from the start without legacy complexity
2. **Code Simplification**: Single evaluation path instead of 5 different condition handlers
3. **Consistency**: Align with existing condition node patterns from day one
4. **Maintainability**: Unified codebase with single evaluation method
5. **Performance**: Leverage optimized `execute_condition` method with no compatibility overhead
6. **Flexibility**: JavaScript functions provide unlimited expressiveness

### User Experience Benefits
1. **Single Condition Simplicity**: No more managing multiple conditions - one intelligent function handles all scenarios
2. **Familiar Pattern**: Consistent with condition nodes users already know
3. **Simplified Interface**: Single code editor instead of complex dropdowns and "Add/Remove" buttons
4. **Better Developer Experience**: CodeEditor with syntax highlighting and validation
5. **More Powerful**: Access to comprehensive event context in one place
6. **No Learning Curve**: Users learn one pattern that works everywhere

### Security Benefits
1. **Consistent Sandboxing**: Uses same proven QuickJS security as condition nodes
2. **Validated Execution**: Leverages existing script validation and timeout mechanisms
3. **No New Attack Vectors**: Reuses secure, battle-tested JavaScript execution infrastructure

### Implementation Benefits
1. **No Technical Debt**: Clean implementation without legacy code to maintain
2. **Faster Development**: No need to support multiple evaluation paths
3. **Simpler Testing**: Only one pattern to test and validate
4. **Optimal Performance**: Built for unified pattern from the start

## Risks and Mitigation

### Risk: JavaScript Complexity for Users
**Mitigation**:
- Provide comprehensive template library
- Include detailed documentation with examples
- Offer syntax validation in the UI
- Create migration guide from current patterns

### Risk: Performance Impact
**Mitigation**:
- Leverage proven `execute_condition` method
- Implement proper QuickJS context management
- Include performance benchmarking in testing

### Risk: Development Timeline
**Mitigation**:
- Reuse existing JavaScript execution infrastructure
- Clear implementation phases with defined deliverables
- Focus on single pattern reduces scope complexity

## Success Metrics

1. **Code Quality**: Clean, maintainable implementation with 50%+ reduction in condition evaluation logic
2. **User Experience**: Positive feedback on unified interface design
3. **Performance**: Optimal loop execution performance from day one
4. **Consistency**: 100% alignment with condition node patterns
5. **Development Velocity**: Faster feature delivery due to simplified architecture
6. **Template Adoption**: High usage of provided template examples

## Timeline

- **Week 1**: Backend implementation and unit testing
- **Week 2**: Frontend component development and testing
- **Week 3**: Integration testing and documentation
- **Week 4**: Final testing and deployment preparation

## Conclusion

This implementation establishes HTTP loop termination conditions using a **single JavaScript function pattern** that aligns with the proven condition node approach. By implementing this simplified approach pre-production, we deliver a clean, maintainable solution that eliminates the complexity of managing multiple conditions while providing users with maximum flexibility through intelligent JavaScript functions.

**Key Achievements:**
- **Single Condition Management**: Users write one intelligent function instead of managing multiple separate conditions
- **Unified Experience**: Same pattern as condition nodes throughout the application
- **Enhanced Capability**: Single functions can handle complex logic that previously required multiple conditions
- **Clean Architecture**: No arrays, no condition management UI complexity, just one powerful editor

The clean-slate implementation approach eliminates technical debt and delivers the optimal user experience without compromise.