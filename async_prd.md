# Asynchronous Workflow Execution - Product Requirements Document

## Overview

SwissPipe currently executes all workflow triggers synchronously, which can lead to timeout issues and poor user experience for long-running workflows. This PRD outlines the requirements for implementing asynchronous workflow execution with proper state management and crash recovery.

## Problem Statement

**Current Issues:**
- All workflow triggers execute synchronously, blocking HTTP responses
- Long-running workflows can cause client timeouts
- No recovery mechanism if system crashes during workflow execution
- Poor scalability for high-volume workflow processing

## Success Metrics

- Workflow trigger responses return within 100ms (HTTP 202 accepted)
- 100% workflow execution state persistence
- Zero workflow loss during system crashes
- Support for concurrent workflow execution

## Requirements

### 1. Asynchronous Trigger Response

**Requirement:** When a workflow is triggered, the system must immediately respond with HTTP 202 (Accepted) and execute the workflow asynchronously.

**Acceptance Criteria:**
- Trigger endpoint responds with HTTP 202 within 100ms
- Response includes a unique execution ID for tracking
- Workflow execution happens in background without blocking the response

### 2. Workflow Execution Tracking

**Requirement:** Maintain persistent state for all workflow executions to enable monitoring and recovery.

**Database Schema:**

**Note:** 
- All timestamps are stored as Unix epoch microseconds (INTEGER) for high precision timing, cross-platform consistency, and efficient operations
- Primary keys use UUIDv7 format for time-ordered, globally unique identifiers with natural sorting properties

```sql
CREATE TABLE workflow_executions (
    id TEXT PRIMARY KEY, -- UUIDv7
    workflow_id TEXT NOT NULL,
    status TEXT NOT NULL, -- 'pending', 'running', 'completed', 'failed', 'cancelled'
    current_node_name TEXT,
    input_data TEXT, -- JSON
    output_data TEXT, -- JSON
    error_message TEXT,
    started_at INTEGER, -- Unix epoch microseconds
    completed_at INTEGER, -- Unix epoch microseconds
    created_at INTEGER DEFAULT (unixepoch('subsec') * 1000000), -- Unix epoch microseconds
    updated_at INTEGER DEFAULT (unixepoch('subsec') * 1000000), -- Unix epoch microseconds
    FOREIGN KEY (workflow_id) REFERENCES workflows(id)
);

CREATE TABLE workflow_execution_steps (
    id TEXT PRIMARY KEY, -- UUIDv7
    execution_id TEXT NOT NULL,
    node_id TEXT NOT NULL,
    node_name TEXT NOT NULL,
    status TEXT NOT NULL, -- 'pending', 'running', 'completed', 'failed', 'skipped'
    input_data TEXT, -- JSON
    output_data TEXT, -- JSON
    error_message TEXT,
    started_at INTEGER, -- Unix epoch microseconds
    completed_at INTEGER, -- Unix epoch microseconds
    created_at INTEGER DEFAULT (unixepoch('subsec') * 1000000), -- Unix epoch microseconds
    FOREIGN KEY (execution_id) REFERENCES workflow_executions(id),
    FOREIGN KEY (node_id) REFERENCES nodes(id)
);

-- Performance indices for critical queries
CREATE INDEX idx_workflow_executions_status ON workflow_executions(status);
CREATE INDEX idx_workflow_executions_workflow_id ON workflow_executions(workflow_id);
CREATE INDEX idx_workflow_executions_created_at ON workflow_executions(created_at);
CREATE INDEX idx_execution_steps_execution_id ON workflow_execution_steps(execution_id);
CREATE INDEX idx_execution_steps_status ON workflow_execution_steps(status);
```

**Acceptance Criteria:**
- All workflow executions are persisted before starting
- Each node execution step is tracked individually
- State updates are atomic and consistent
- Historical execution data is maintained for auditing
- Database indices optimize critical queries for status filtering, workflow lookups, and execution step retrieval

### 3. Crash Recovery

**Requirement:** System must be able to resume workflow execution after crashes or restarts.

**Acceptance Criteria:**
- On startup, identify all executions with status 'running'
- Resume execution from the current node
- Handle partial node execution gracefully
- Maintain data consistency during recovery

### 4. Execution Status API

**Requirement:** Provide API endpoints to query workflow execution status and results.

**Endpoints:**
- `GET /api/v1/executions/{execution_id}` - Get execution details
- `GET /api/v1/executions/{execution_id}/status` - Get execution status
- `GET /api/v1/executions/{execution_id}/logs` - Get execution logs
- `GET /api/v1/workflows/{workflow_id}/executions` - List executions for workflow
- `DELETE /api/v1/executions/{execution_id}` - Cancel execution

**Acceptance Criteria:**
- Real-time status updates
- Detailed execution logs and step-by-step progress
- Ability to cancel running executions
- Pagination for execution lists

### 5. Worker Pool Architecture

**Requirement:** Implement a worker pool system that picks up and executes workflow jobs asynchronously.

**Architecture Components:**

**Job Queue:**
```sql
CREATE TABLE job_queue (
    id TEXT PRIMARY KEY, -- UUIDv7
    execution_id TEXT NOT NULL,
    priority INTEGER DEFAULT 0,
    scheduled_at INTEGER DEFAULT (unixepoch('subsec') * 1000000), -- Unix epoch microseconds
    claimed_at INTEGER, -- Unix epoch microseconds
    claimed_by TEXT, -- worker_id
    max_retries INTEGER DEFAULT 3,
    retry_count INTEGER DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'pending', -- 'pending', 'claimed', 'processing', 'completed', 'failed', 'dead_letter'
    error_message TEXT,
    created_at INTEGER DEFAULT (unixepoch('subsec') * 1000000), -- Unix epoch microseconds
    updated_at INTEGER DEFAULT (unixepoch('subsec') * 1000000), -- Unix epoch microseconds
    FOREIGN KEY (execution_id) REFERENCES workflow_executions(id)
);

-- Performance indices for job queue operations
CREATE INDEX idx_job_queue_status_priority ON job_queue(status, priority DESC, scheduled_at ASC);
CREATE INDEX idx_job_queue_claimed_by ON job_queue(claimed_by);
CREATE INDEX idx_job_queue_execution_id ON job_queue(execution_id);
CREATE INDEX idx_job_queue_claimed_at ON job_queue(claimed_at);
CREATE INDEX idx_job_queue_retry_count ON job_queue(retry_count, status);
```

**Worker Pool Components:**
- **Job Scheduler**: Creates jobs for new workflow executions
- **Worker Manager**: Manages worker lifecycle and health
- **Job Poller**: Continuously polls for available jobs
- **Execution Engine**: Executes workflow steps
- **Result Handler**: Updates execution status and handles job completion

**Acceptance Criteria:**
- Configurable worker pool size (1-50 workers)
- Job claiming mechanism with timeout to prevent deadlocks
- Worker health monitoring and automatic restart
- Job priority support for urgent workflows
- Graceful worker shutdown with job completion
- Dead letter queue for permanently failed jobs
- Worker load balancing and distribution

## Technical Implementation

### 1. Execution Engine Changes

**Current Flow:**
```
HTTP Request → Trigger → Execute Workflow → HTTP Response
```

**New Flow:**
```
HTTP Request → Create Execution Record → Create Job → HTTP 202 Response
                                           ↓
Worker Pool → Claim Job → Execute Workflow → Update Status → Complete Job
```

### 2. State Management

- Use SeaORM entities for execution tracking with UUIDv7 primary keys
- Implement state machine for execution status transitions
- Ensure atomic updates for state changes
- Add migration for new database tables
- Generate UUIDv7 IDs for natural time-ordering and improved database performance

### 3. Worker Pool Implementation

**Worker Architecture:**
```rust
struct WorkerPool {
    workers: Vec<Worker>,
    job_poller: JobPoller,
    manager: WorkerManager,
}

struct Worker {
    id: String,
    status: WorkerStatus, // Idle, Busy, Shutdown
    executor: WorkflowExecutor,
    current_job: Option<JobId>,
}
```

**Key Components:**
- **JobPoller**: Continuously polls database for pending jobs using optimistic locking
- **WorkerManager**: Spawns, monitors, and restarts workers as needed
- **JobClaimer**: Claims jobs atomically to prevent race conditions
- **WorkflowExecutor**: Executes individual workflow steps with state persistence
- **RetryManager**: Handles failed jobs with exponential backoff

**Job Processing Flow:**
1. Worker polls for available jobs with `FOR UPDATE SKIP LOCKED`
2. Atomically claim job by setting `claimed_by` and `claimed_at`
3. Execute workflow with periodic status updates
4. Handle success/failure and update job status
5. Release worker for next job

### 4. Recovery Mechanism

- On startup, scan for incomplete executions
- Implement resume logic for each node type
- Handle idempotent operations where possible
- Add configuration for recovery behavior

## API Changes

### Modified Trigger Endpoint

**Before:**
```http
POST /api/v1/{workflow_id}/trigger
Content-Type: application/json

{
    "data": { "key": "value" }
}

Response: 200 OK
{
    "result": { "processed": true }
}
```

**After:**
```http
POST /api/v1/{workflow_id}/trigger
Content-Type: application/json

{
    "data": { "key": "value" }
}

Response: 202 Accepted
{
    "execution_id": "01934b2a-7890-7abc-9def-123456789abc",
    "status": "pending", 
    "created_at": 1736939400000000
}
```

### New Status Endpoint

**Note:** All timestamps in API responses are Unix epoch microseconds for precision and consistency.

```http
GET /api/v1/executions/{execution_id}

Response: 200 OK
{
    "id": "01934b2a-7890-7abc-9def-123456789abc",
    "workflow_id": "wf-456",
    "status": "completed",
    "current_node_name": null,
    "started_at": 1736939401000000,
    "completed_at": 1736939415000000,
    "input_data": { "key": "value" },
    "output_data": { "processed": true },
    "steps": [
        {
            "node_name": "Start",
            "status": "completed",
            "completed_at": 1736939402000000
        },
        {
            "node_name": "ProcessNode",
            "status": "completed",
            "completed_at": 1736939415000000
        }
    ]
}
```

## Configuration

Add configuration options for async execution and worker pool:

```toml
[async_execution]
enabled = true
execution_history_retention_days = 30
recovery_on_startup = true

[worker_pool]
# Worker pool settings
worker_count = 5
min_workers = 1
max_workers = 20
worker_idle_timeout_seconds = 300

# Job processing settings
job_poll_interval_ms = 1000
job_claim_timeout_seconds = 300
job_execution_timeout_seconds = 3600
max_retry_attempts = 3
retry_backoff_multiplier = 2.0
retry_max_delay_seconds = 3600

# Job queue settings
job_batch_size = 10
priority_levels = 5
dead_letter_threshold = 10

# Health monitoring
worker_health_check_interval_seconds = 30
job_claim_cleanup_interval_seconds = 600
metrics_collection_enabled = true

[security]
# Configurable dangerous headers (environment variable: SP_DANGEROUS_HEADERS)
# Default: "authorization,cookie,x-forwarded-for,x-real-ip,x-forwarded-proto,host,origin,referer,x-csrf-token,x-api-key,x-auth-token,bearer,www-authenticate,proxy-authorization,proxy-authenticate"
# Set SP_DANGEROUS_HEADERS to override (comma-separated list)
# Example: SP_DANGEROUS_HEADERS="authorization,x-secret,x-private"
```

## Environment Variables

The following environment variables can be used to configure SwissPipe:

- **`SP_DANGEROUS_HEADERS`**: Comma-separated list of header names to strip from incoming requests for security
  - Default: `"authorization,cookie,x-forwarded-for,x-real-ip,x-forwarded-proto,host,origin,referer,x-csrf-token,x-api-key,x-auth-token,bearer,www-authenticate,proxy-authorization,proxy-authenticate"`
  - Example: `SP_DANGEROUS_HEADERS="authorization,x-secret,x-private"`
  - Set to empty string to disable header stripping: `SP_DANGEROUS_HEADERS=""`

## Migration Strategy

### Phase 1: Infrastructure Setup
1. Add database tables for execution tracking and job queue
2. Implement SeaORM entities for new tables
3. Create basic worker pool framework
4. Add status API endpoints

### Phase 2: Worker Pool Implementation
1. Implement job claiming mechanism with atomic operations
2. Create worker manager and job poller
3. Implement workflow execution engine for workers
4. Add worker health monitoring and restart logic

### Phase 3: Async Triggers & Job Processing
1. Modify trigger endpoints to return 202 and create jobs
2. Implement job retry mechanism with exponential backoff
3. Add execution state management and step tracking
4. Implement job priority and scheduling

### Phase 4: Recovery & Advanced Features
1. Implement crash recovery for claimed jobs
2. Add dead letter queue handling
3. Implement execution cancellation and cleanup
4. Add comprehensive logging and monitoring

### Phase 5: Optimization & Scaling
1. Performance tuning for high-volume job processing
2. Worker pool auto-scaling based on queue size
3. Advanced metrics and alerting integration
4. Load testing and capacity planning

## Risks & Mitigation

**Risk:** Data loss during crashes
**Mitigation:** Atomic database operations and proper transaction handling

**Risk:** Memory leaks from long-running executions
**Mitigation:** Resource monitoring and cleanup mechanisms

**Risk:** Worker pool resource exhaustion
**Mitigation:** Configurable worker limits, resource monitoring, and auto-scaling

**Risk:** Job queue bottleneck
**Mitigation:** Database indexing, job batching, and queue partitioning strategies

**Risk:** Dead letter queue growth
**Mitigation:** Automated cleanup, alerting on queue size, and manual intervention tools

**Risk:** Worker deadlocks or hanging jobs
**Mitigation:** Job claim timeouts, worker health checks, and automatic cleanup

**Risk:** Breaking changes to existing API consumers
**Mitigation:** Feature flag for async mode, backward compatibility support

## Success Criteria

- [ ] All workflow triggers return HTTP 202 within 100ms
- [ ] Worker pool processes jobs concurrently with configurable limits
- [ ] Zero workflow execution data loss during system crashes
- [ ] Successful recovery of interrupted workflows and claimed jobs on restart
- [ ] Complete execution audit trail available via API
- [ ] Job retry mechanism with exponential backoff working correctly
- [ ] Dead letter queue handling for permanently failed jobs
- [ ] Worker health monitoring and automatic restart functionality
- [ ] Job claiming mechanism prevents race conditions
- [ ] Configurable worker pool size and job processing settings
- [ ] Comprehensive monitoring and alerting for worker pool and job queue

## Dependencies

- SeaORM migration system (already implemented)
- Worker pool framework with job queue (to be implemented)
- UUIDv7 generation library (e.g., `uuid` crate with v7 feature)
- Atomic database operations for job claiming (SQLite/PostgreSQL features)
- Enhanced logging and monitoring system
- Configuration management system
- Tokio async runtime for worker management

## Timeline

**Estimated Duration:** 5-6 weeks

- Week 1: Database schema for job queue and execution tracking
- Week 2: Basic worker pool framework and job claiming mechanism
- Week 3: Workflow execution engine integration with workers
- Week 4: Job retry, recovery, and dead letter queue handling
- Week 5: Status APIs, monitoring, and comprehensive testing
- Week 6: Performance optimization, documentation, and deployment