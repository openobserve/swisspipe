# HTTP Loop Functionality Guide

This guide provides comprehensive documentation on using the HTTP Loop functionality in SwissPipe, including UI components, configuration options, and practical examples.

## Table of Contents

1. [Overview](#overview)
2. [UI Components](#ui-components)
3. [Configuration Examples](#configuration-examples)
4. [Advanced Use Cases](#advanced-use-cases)
5. [Monitoring and Control](#monitoring-and-control)
6. [Best Practices](#best-practices)
7. [Troubleshooting](#troubleshooting)

## Overview

HTTP Loop functionality allows you to repeatedly execute HTTP requests with configurable intervals, termination conditions, and backoff strategies. This is ideal for:

- **Polling APIs** for status updates
- **Data synchronization** workflows
- **Health monitoring** systems
- **Customer onboarding** processes
- **Webhook retry** mechanisms

---

## UI Components

### 1. HttpLoopConfig Component

**Location**: Node Properties Panel → HTTP Request Configuration → Loop Configuration

**Purpose**: Configure loop behavior, backoff strategies, and termination conditions.

#### Features:
- **Enable/Disable Toggle**: Quick loop activation
- **Basic Settings**: Max iterations and interval configuration
- **Backoff Strategy**: Fixed or Exponential backoff options
- **Termination Conditions**: JavaScript-based exit conditions
- **Quick Setup Templates**: Pre-configured scenarios

#### Screenshot Reference:
```
┌─────────────────────────────────────────────┐
│ Loop Configuration              [Enabled] ✓ │
├─────────────────────────────────────────────┤
│ Max Iterations: [5    ] Interval: [60] sec │
│                                             │
│ Backoff Strategy: [Exponential      ▼]     │
│ ┌─────────────────────────────────────────┐ │
│ │ Base: [30] Multiplier: [1.5] Max: [300]│ │
│ └─────────────────────────────────────────┘ │
│                                             │
│ Termination Conditions           [+ Add]   │
│ ┌─────────────────────────────────────────┐ │
│ │ Response Content | Success | [Remove]   │ │
│ │ response.data.status === "completed"    │ │
│ └─────────────────────────────────────────┘ │
│                                             │
│ Templates: [Customer Onboarding] [Health]  │
└─────────────────────────────────────────────┘
```

### 2. HttpRequestNode Component

**Location**: Workflow Canvas

**Purpose**: Visual indicator of loop status directly on workflow nodes.

#### Features:
- **Loop Indicator**: Shows when loop is configured
- **Progress Ring**: Visual progress for finite loops
- **Status Badge**: Current loop status (Running, Completed, Failed)
- **Next Execution Timer**: Countdown to next iteration
- **Animation Effects**: Pulse for active loops

#### Visual States:
```
┌─────────────────────┐     ┌─────────────────────┐     ┌─────────────────────┐
│   HTTP Request      │     │   HTTP Request      │     │   HTTP Request      │
│                     │     │  ● Loop Configured  │     │  ⟲ Loop Active     │
│  Standard Node      │     │                     │     │   ┌─────────────┐   │
│                     │     │                     │     │   │ ████░░░ 73% │   │
│                     │     │                     │     │   └─────────────┘   │
└─────────────────────┘     └─────────────────────┘     └─────────────────────┘
   No Loop Config              Loop Configured           Loop Running (73%)
```

### 3. HttpLoopStatus Component

**Location**: Bottom-right corner (active loops panel)

**Purpose**: Detailed real-time monitoring and control of running loops.

#### Features:
- **Progress Visualization**: Progress bars and percentages
- **Status Information**: Detailed execution metrics
- **Control Buttons**: Pause, Stop, Retry operations
- **Iteration History**: Success/failure tracking
- **Success Rate**: Overall performance metrics

#### Layout:
```
┌──────────────────────── HTTP Loop Status ────────────────────────┐
│ Status: [Running] ●                                              │
├──────────────────────────────────────────────────────────────────┤
│ Progress: 3/5 iterations                                         │
│ ████████████████░░░░░░░░ 60%                                     │
│                                                                  │
│ Next Execution: in 2m    │ Failures: 0                          │
│ Started: Dec 15, 10:30   │ Last Status: 200                     │
│ Duration: 15m            │ Success Rate: 100%                   │
│                                                                  │
│ [Pause Loop] [Stop Loop]                                        │
│                                                                  │
│ Iteration History (5) ▼                                         │
│ ┌────────────────────────────────────────────────────────────┐  │
│ │ #3  200  10:45:30  Duration: 250ms                        │  │
│ │ #2  200  10:43:30  Duration: 180ms                        │  │
│ │ #1  200  10:41:30  Duration: 220ms                        │  │
│ └────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
```

---

## Configuration Examples

### Example 1: Basic Polling Loop

**Scenario**: Poll an API every 30 seconds for up to 10 times.

#### Configuration:
```javascript
// HTTP Request URL
https://api.example.com/status/job-123

// Loop Configuration
{
  "max_iterations": 10,
  "interval_seconds": 30,
  "backoff_strategy": {
    "Fixed": 30
  },
  "termination_conditions": [
    {
      "condition_type": "ResponseContent",
      "expression": "response.data.status === 'completed'",
      "action": "Success"
    }
  ]
}
```

#### UI Steps:
1. Add HTTP Request node to workflow
2. Set URL to `https://api.example.com/status/job-123`
3. Open Node Properties Panel
4. Toggle Loop Configuration to "Enabled"
5. Set Max Iterations: `10`
6. Set Interval: `30` seconds
7. Keep Backoff Strategy as "Fixed Interval"
8. Add termination condition:
   - Type: "Response Content"
   - Expression: `response.data.status === 'completed'`
   - Action: "Success"

#### Expected Behavior:
- Requests every 30 seconds
- Stops when API returns `status: "completed"`
- Maximum 10 attempts (5 minutes total)
- Fixed 30-second intervals

### Example 2: Customer Onboarding Monitor

**Scenario**: Monitor customer data ingestion with exponential backoff.

#### Using Quick Template:
1. Open Loop Configuration
2. Click "Customer Onboarding" template
3. Modify URL to your customer API endpoint

#### Generated Configuration:
```javascript
{
  "max_iterations": 72,           // 72 hours max
  "interval_seconds": 3600,       // 1 hour intervals
  "backoff_strategy": {
    "Fixed": 3600
  },
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
```

#### Real-world Usage:
```javascript
// API Endpoint
https://api.company.com/customers/12345/ingestion-status

// Expected Response
{
  "customer_id": "12345",
  "has_ingested_data": false,  // Changes to true when complete
  "data_sources": ["crm", "billing"],
  "ingestion_progress": 0.45
}
```

### Example 3: Health Monitoring with Exponential Backoff

**Scenario**: Monitor service health with intelligent retry intervals.

#### Configuration:
```javascript
{
  "interval_seconds": 30,         // Start with 30 seconds
  "backoff_strategy": {
    "Exponential": {
      "base": 30,                 // Base interval
      "multiplier": 1.5,          // Increase by 50% each time
      "max": 300                  // Cap at 5 minutes
    }
  },
  "termination_conditions": [
    {
      "condition_type": "ResponseStatus",
      "expression": "status_code === 200",
      "action": "Success"
    },
    {
      "condition_type": "TotalTime",
      "expression": "duration_seconds > 3600",  // Stop after 1 hour
      "action": "Failure"
    }
  ]
}
```

#### Interval Progression:
- Attempt 1: 30 seconds
- Attempt 2: 45 seconds (30 × 1.5)
- Attempt 3: 67 seconds (45 × 1.5)
- Attempt 4: 100 seconds (67 × 1.5)
- Attempt 5: 150 seconds (100 × 1.5)
- Attempt 6+: 300 seconds (capped at max)

### Example 4: Data Sync with Custom Conditions

**Scenario**: Sync data with complex termination logic.

#### Configuration:
```javascript
{
  "max_iterations": 5,
  "interval_seconds": 1,
  "backoff_strategy": {
    "Exponential": {
      "base": 1,
      "multiplier": 2.0,
      "max": 30
    }
  },
  "termination_conditions": [
    {
      "condition_type": "ResponseContent",
      "expression": "response.data.sync_status === 'completed'",
      "action": "Success"
    },
    {
      "condition_type": "ResponseStatus",
      "expression": "status_code >= 400 && status_code < 500",
      "action": "Failure"  // Stop on client errors
    },
    {
      "condition_type": "Custom",
      "expression": "response.data.error_count > 10",
      "action": "Failure"
    }
  ]
}
```

#### Custom Expression Examples:
```javascript
// Check nested object properties
response.data.workflow.stage === 'final'

// Array length conditions
response.data.items.length >= 100

// Time-based conditions
new Date(response.data.last_updated) > new Date(Date.now() - 300000)

// Complex boolean logic
response.data.status === 'ready' && response.data.validation.passed

// Mathematical conditions
response.data.progress >= 0.95 || response.data.force_complete
```

---

## Advanced Use Cases

### Webhook Retry Pattern

**Use Case**: Retry webhook deliveries with intelligent backoff.

```javascript
// Webhook URL (parameterized)
https://customer-webhooks.com/api/{{customer_id}}/events

// Advanced Configuration
{
  "max_iterations": 8,
  "interval_seconds": 5,
  "backoff_strategy": {
    "Exponential": {
      "base": 5,      // 5 second start
      "multiplier": 2.0,
      "max": 3600     // Max 1 hour between retries
    }
  },
  "termination_conditions": [
    {
      "condition_type": "ResponseStatus",
      "expression": "status_code >= 200 && status_code < 300",
      "action": "Success"
    },
    {
      "condition_type": "ResponseStatus",
      "expression": "status_code === 410", // Gone - don't retry
      "action": "Failure"
    },
    {
      "condition_type": "Custom",
      "expression": "status_code >= 500 && current_iteration >= 5",
      "action": "Failure"  // Give up after 5 server errors
    }
  ]
}
```

### API Rate Limit Handling

**Use Case**: Respect API rate limits with adaptive delays.

```javascript
{
  "max_iterations": 50,
  "interval_seconds": 1,
  "backoff_strategy": {
    "Exponential": {
      "base": 1,
      "multiplier": 1.2,  // Gentle increase
      "max": 60
    }
  },
  "termination_conditions": [
    {
      "condition_type": "ResponseStatus",
      "expression": "status_code === 200",
      "action": "Success"
    },
    {
      "condition_type": "Custom",
      "expression": "status_code === 429 && parseInt(response.headers['retry-after'] || '60') > 300",
      "action": "Failure"  // Stop if rate limit > 5 minutes
    }
  ]
}
```

---

## Monitoring and Control

### Real-time Monitoring

#### Active Loops Panel
The active loops panel appears automatically when loops are running:

**Location**: Bottom-right corner of workflow designer

**Information Displayed**:
- Loop ID and associated workflow
- Current iteration / max iterations
- Progress percentage
- Next execution countdown
- Success/failure counts
- Duration since start

#### Node Status Indicators

**Visual Cues on Workflow Canvas**:
- 🔵 Blue dot: Loop configured but not running
- 🟢 Green pulse: Loop running successfully
- 🔴 Red indicator: Loop failed
- ⏸️ Yellow indicator: Loop paused
- ✅ Green checkmark: Loop completed successfully

### Control Operations

#### Pause Loop
**When to Use**: Temporarily stop loop execution while maintaining state.

**UI Action**: Click "Pause Loop" button in HttpLoopStatus panel

**Effect**:
- Stops scheduling new iterations
- Preserves current progress
- Can be resumed later
- Shows "Paused" status badge

#### Stop Loop
**When to Use**: Permanently terminate loop execution.

**UI Action**: Click "Stop Loop" button in HttpLoopStatus panel

**Effect**:
- Immediately cancels loop
- Marks as "Stopped" status
- Cannot be resumed (must retry to restart)
- Cleans up background polling

#### Retry Loop
**When to Use**: Restart a failed or stopped loop.

**UI Action**: Click "Retry Loop" button (appears for failed loops)

**Effect**:
- Resets iteration counter
- Uses original configuration
- Starts fresh execution cycle
- Clears previous failure state

### Status Interpretation

#### Loop Status Values

| Status | Meaning | Visual Indicator | Available Actions |
|--------|---------|------------------|-------------------|
| `running` | Loop actively executing | Blue pulse | Pause, Stop |
| `completed` | Successfully finished | Green checkmark | None |
| `failed` | Terminated due to error | Red X | Retry |
| `paused` | Temporarily stopped | Yellow pause | Resume, Stop |
| `stopped` | Manually terminated | Gray stop | Retry |

#### Progress Indicators

**Finite Loops** (with max_iterations):
- Progress ring showing completion percentage
- "X/Y iterations" counter
- Estimated completion time

**Infinite Loops** (no max_iterations):
- Pulsing indicator
- Current iteration count only
- "∞ (infinite loop)" label

---

## Best Practices

### 1. Interval Selection

#### Short Intervals (1-10 seconds)
- **Use for**: Real-time status checks, fast APIs
- **Caution**: High server load, potential rate limiting
- **Example**: Payment processing status

#### Medium Intervals (30-300 seconds)
- **Use for**: Regular health checks, moderate polling
- **Sweet spot**: Balance between responsiveness and resource usage
- **Example**: File processing status

#### Long Intervals (5+ minutes)
- **Use for**: Background sync, low-priority monitoring
- **Benefit**: Minimal server impact
- **Example**: Daily report generation

### 2. Backoff Strategy Selection

#### Fixed Intervals
```javascript
"backoff_strategy": { "Fixed": 60 }
```
- **Use when**: Predictable API behavior, consistent response times
- **Pros**: Predictable timing, simple debugging
- **Cons**: Not adaptive to failures

#### Exponential Backoff
```javascript
"backoff_strategy": {
  "Exponential": {
    "base": 30,
    "multiplier": 1.5,
    "max": 300
  }
}
```
- **Use when**: APIs may be temporarily overloaded
- **Pros**: Reduces server pressure, handles failures gracefully
- **Cons**: Less predictable timing

### 3. Termination Conditions

#### Success Conditions
Always include at least one success condition:
```javascript
{
  "condition_type": "ResponseContent",
  "expression": "response.data.status === 'completed'",
  "action": "Success"
}
```

#### Failure Conditions
Prevent infinite loops with failure conditions:
```javascript
{
  "condition_type": "ConsecutiveFailures",
  "expression": "count >= 5",
  "action": "Failure"
}
```

#### Timeout Conditions
Set maximum execution time:
```javascript
{
  "condition_type": "TotalTime",
  "expression": "duration_seconds > 3600",  // 1 hour max
  "action": "Failure"
}
```

### 4. Expression Best Practices

#### Safe Property Access
```javascript
// Good - safe navigation
response.data?.status === 'completed'

// Good - with fallback
(response.data && response.data.status) === 'completed'

// Avoid - may throw errors
response.data.nested.deep.property === 'value'
```

#### Type Checking
```javascript
// Good - verify types
typeof response.data.count === 'number' && response.data.count > 100

// Good - array checks
Array.isArray(response.data.items) && response.data.items.length > 0
```

#### Error Handling
```javascript
// Good - handle missing data
response.data.error_code ? response.data.error_code !== 'RETRY' : true
```

---

## Troubleshooting

### Common Issues

#### 1. Loop Not Starting

**Symptoms**:
- Node shows loop configured but no activity
- No entries in active loops panel

**Solutions**:
- Verify HTTP request configuration is valid
- Check that loop is enabled (toggle shows "Enabled")
- Ensure workflow is saved and enabled
- Check browser console for JavaScript errors

#### 2. Loop Terminating Immediately

**Symptoms**:
- Loop starts but stops after first iteration
- Status shows "Completed" or "Failed" unexpectedly

**Solutions**:
- Review termination conditions for overly broad expressions
- Check API response structure matches expected format
- Verify condition syntax in browser DevTools
- Test expressions with sample data

**Debug Expression**:
```javascript
// Add this temporary condition to log responses
console.log('Response:', response) || false
```

#### 3. Exponential Backoff Not Working

**Symptoms**:
- Intervals remain constant despite exponential config
- No gradual increase in delays

**Solutions**:
- Verify backoff strategy is set to "Exponential"
- Check multiplier is > 1.0
- Ensure base value is reasonable
- Monitor actual intervals in loop status panel

#### 4. Memory/Performance Issues

**Symptoms**:
- Browser becomes slow with active loops
- High CPU usage
- Network congestion

**Solutions**:
- Reduce polling frequency (increase intervals)
- Monitor resource usage patterns
- Use exponential backoff to reduce server load
- Set reasonable max_iterations limits

### Debugging Techniques

#### 1. Expression Testing

Use browser DevTools to test expressions:
```javascript
// In browser console
const response = { data: { status: 'pending', progress: 0.75 } }
const result = response.data.status === 'completed' || response.data.progress >= 0.9
console.log('Condition result:', result)
```

#### 2. Network Monitoring

Monitor actual HTTP requests:
1. Open browser DevTools → Network tab
2. Start loop execution
3. Observe request timing and responses
4. Verify intervals match configuration

#### 3. Console Logging

Add temporary logging to expressions:
```javascript
// Log and continue
console.log('Status:', response.data.status) || response.data.status === 'completed'

// Log response structure
console.log('Full response:', JSON.stringify(response, null, 2)) || false
```

#### 4. Loop Status Analysis

Check iteration history for patterns:
- Response codes: Look for consistent failures
- Timing: Verify interval progression
- Success rate: Identify intermittent issues
- Duration: Spot performance degradation

### Error Messages

#### Common Error Messages and Solutions

| Error Message | Cause | Solution |
|---------------|-------|----------|
| "Failed to parse iteration history" | Corrupted loop data | Check browser console, retry loop |
| "Loop configuration invalid" | Missing required fields | Verify all required fields are set |
| "Expression evaluation failed" | Syntax error in condition | Review and fix JavaScript expression |
| "Maximum iterations exceeded" | Loop reached limit | Increase max_iterations or fix termination conditions |
| "Network timeout" | API not responding | Check API availability, increase timeout |

---

## Quick Reference

### Template Configurations

#### Customer Onboarding
- **Duration**: 72 hours max
- **Interval**: 1 hour
- **Use case**: Long-running data ingestion processes

#### Health Monitoring
- **Duration**: Until success or 1 hour timeout
- **Interval**: 30 seconds with exponential backoff
- **Use case**: Service availability checks

#### Data Sync
- **Duration**: 5 attempts max
- **Interval**: 1 second with exponential backoff
- **Use case**: Fast synchronization processes

### Keyboard Shortcuts

| Action | Shortcut |
|--------|----------|
| Open node properties | Double-click node |
| Close modal | Escape |
| Save configuration | Ctrl/Cmd + S |

### API Response Examples

#### Successful Response
```json
{
  "status": "completed",
  "data": {
    "job_id": "abc123",
    "progress": 1.0,
    "result": "success"
  }
}
```

#### Pending Response
```json
{
  "status": "processing",
  "data": {
    "job_id": "abc123",
    "progress": 0.45,
    "eta_seconds": 120
  }
}
```

#### Error Response
```json
{
  "status": "error",
  "error": {
    "code": "VALIDATION_FAILED",
    "message": "Invalid input data"
  }
}
```

---

*This documentation covers the complete HTTP Loop functionality. For additional support, refer to the main SwissPipe documentation or contact the development team.*