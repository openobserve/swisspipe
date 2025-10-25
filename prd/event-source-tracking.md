# Product Requirements Document: Event Source Tracking in Workflows

## 1. Overview

### 1.1 Problem Statement
Currently in SwissPipe workflows, each node only receives data from its immediate predecessor. For example, in a workflow chain `Node 1 → Node 2 → Node 3`:
- Node 2 receives output from Node 1
- Node 3 receives output from Node 2 only (Node 1's data is not accessible)

This limitation prevents downstream nodes from accessing data from non-adjacent upstream nodes, which is often needed for complex workflows where later nodes need to reference or combine data from multiple points in the execution chain.

### 1.2 Proposed Solution
Add a `sources` field to the `WorkflowEvent` structure that maintains a complete history of all upstream node inputs. Each source entry will include:
- Node ID and name for identification
- Complete data input that the node received
- Execution sequence number
- Timestamp when the node started execution

This enables any node in the workflow to access the input data of any previous node in the execution path, allowing visibility into data transformations and the ability to reference original/intermediate states.

### 1.3 Success Criteria
- All downstream nodes can access data from any upstream node in their execution path
- Source tracking has minimal performance impact:
  - < 10% overhead for workflows with < 20 nodes
  - < 20% overhead for workflows with 20-50 nodes
  - P99 latency increase < 100ms per workflow execution
- Source history is properly maintained through all node types (Trigger, Transformer, Condition, App nodes)
- Source data is available in JavaScript execution contexts (Transformer and Condition nodes)
- Source tracking works correctly with complex workflow patterns (branches, loops, HIL nodes)

---

## 2. Functional Requirements

### 2.1 Data Structure

#### 2.1.1 NodeSource Structure
Add a new structure to track individual node outputs:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSource {
    /// Unique identifier of the source node
    pub node_id: String,

    /// Human-readable name of the source node
    pub node_name: String,

    /// Type of node (for context)
    pub node_type: String,

    /// Complete data input that this node received
    pub data: serde_json::Value,

    /// Execution sequence number (0-indexed from first non-trigger node)
    pub sequence: u32,

    /// ISO 8601 timestamp when this node started execution
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// Optional metadata about the node execution
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}
```

#### 2.1.2 WorkflowEvent Enhancement
Modify the existing `WorkflowEvent` structure:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEvent {
    pub data: serde_json::Value,

    #[serde(default)]
    pub metadata: HashMap<String, String>,

    #[serde(default)]
    pub headers: HashMap<String, String>,

    #[serde(default)]
    pub condition_results: HashMap<String, bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hil_task: Option<serde_json::Value>,

    /// NEW: Complete history of all upstream node inputs
    #[serde(default)]
    pub sources: Vec<NodeSource>,
}
```

### 2.2 Source Tracking Behavior

#### 2.2.1 Initial Event Creation
- Trigger nodes create the initial event with an empty `sources` array
- The trigger node's output is NOT added to sources (it's already in `data`)

#### 2.2.2 Source Propagation
When a node executes and produces output:
1. Clone all sources from the input event
2. Add the current node's **input data** as a new `NodeSource` entry:
   - `node_id`: Current node's ID
   - `node_name`: Current node's name
   - `node_type`: Type of node (e.g., "Transformer", "HttpRequest")
   - `data`: Current node's **input data** (the `event.data` that the node received)
   - `sequence`: Previous max sequence + 1 (or 0 if sources is empty)
   - `timestamp`: Current UTC timestamp when node execution started
   - `metadata`: Optional execution metadata
3. Pass the augmented sources array to downstream nodes with the new output data

**Important**: Sources store what each node **received** as input, not what it produced as output. This allows downstream nodes to access the complete input history of the workflow execution.

#### 2.2.3 Source Tracking by Node Type

**Trigger Nodes:**
- Initialize event with empty `sources` array
- Do not add themselves to sources (trigger output is the initial `event.data`)

**Transformer Nodes:**
- Before executing transformation script, add current `event.data` (input) to sources
- Execute transformation script to produce new output
- Pass transformed output as new `event.data` to downstream nodes
- Downstream nodes can access the pre-transformation data via sources

**Condition Nodes:**
- Do not modify sources (they route data but don't transform it)
- Pass through sources unchanged
- Condition results stored in `event.condition_results`, not in sources

**HTTP/OpenObserve/Email/Anthropic Nodes:**
- Before making HTTP call/API request, add current `event.data` (input) to sources
- Execute HTTP call/API request to get response
- Response data becomes new `event.data`
- Downstream nodes can access the pre-request data via sources

**Delay Nodes:**
- Preserve sources unchanged during delay period
- Do not add to sources (they only delay, don't transform)
- Sources are serialized with delay job and restored on resumption

**Human-in-Loop Nodes:**
- Before creating HIL task, add current `event.data` (input) to sources
- All execution paths (notification, approved, denied) receive the same sources
- User responses/modifications don't alter source history
- Downstream nodes can access the pre-HIL data via sources

#### 2.2.4 Source Tracking in Branching and Merge Scenarios

When workflows have branching paths that converge at a merge node:

```
       A
      / \
     B   C
      \ /
       D (merge)
```

**Behavior at Merge Node D:**
- Receives multiple input events (one from B, one from C)
- Sources are **merged (union)** from all input events
- If same `node_id` appears in multiple inputs, keep most recent by timestamp
- Sequence numbers are preserved from original execution order
- Node D can access data from both branch paths via sources

**Example:**
```javascript
// At node D after receiving inputs from B and C
event.sources = [
    { node_id: "A", sequence: 0, data: {...} },  // Common ancestor
    { node_id: "B", sequence: 1, data: {...} },  // Path 1
    { node_id: "C", sequence: 2, data: {...} },  // Path 2
]
```

**Note**: Input merge strategy (WaitForAll, FirstWins, TimeoutBased) determines when node D executes, but sources always represent the union of all received paths once execution begins.

### 2.3 JavaScript Context Access

#### 2.3.1 Transformer Scripts
Transformer functions receive an `event` object with sources:

```javascript
function transformer(event) {
    // Access current data
    const currentData = event.data;

    // Access all upstream sources
    const sources = event.sources; // Array of NodeSource objects

    // Example: Find data from a specific node
    const node1Data = sources.find(s => s.node_name === "HTTP Fetch");

    // Example: Get the first node's data
    const firstNodeData = sources.find(s => s.sequence === 0);

    // Example: Combine data from multiple sources
    const combined = {
        current: currentData,
        httpResponse: sources.find(s => s.node_type === "HttpRequest")?.data,
        transformedValue: sources.find(s => s.node_id === "transform-1")?.data
    };

    return combined;
}
```

#### 2.3.2 Condition Scripts
Condition functions can access sources for decision-making:

```javascript
function condition(event) {
    // Check if a value exists in any upstream node
    const hasEmail = event.sources.some(s =>
        s.data && s.data.email
    );

    // Compare current data with historical data
    const originalValue = event.sources[0]?.data?.value;
    const currentValue = event.data.value;

    return currentValue > originalValue * 2;
}
```

### 2.4 API Response Enhancement

#### 2.4.1 Execution Step Details
When querying workflow execution details, include source information:

```json
{
    "execution_id": "exec-123",
    "node_id": "node-3",
    "output": {
        "data": { "result": "final" },
        "sources": [
            {
                "node_id": "node-1",
                "node_name": "HTTP Fetch",
                "node_type": "HttpRequest",
                "data": { "users": [...] },
                "sequence": 0,
                "timestamp": "2025-10-24T10:30:00Z"
            },
            {
                "node_id": "node-2",
                "node_name": "Transform Users",
                "node_type": "Transformer",
                "data": { "processed": [...] },
                "sequence": 1,
                "timestamp": "2025-10-24T10:30:01Z"
            }
        ]
    }
}
```

---

## 3. Non-Functional Requirements

### 3.1 Performance
- Source tracking should add < 10% overhead for typical workflows (< 20 nodes)
- Target < 20% overhead for complex workflows (20-50 nodes)
- P99 latency increase should be < 100ms per workflow execution
- Use shallow cloning where possible to minimize memory usage
- Implement maximum source history size (configurable, default: 50 nodes)
- Use lazy serialization (only serialize sources when storing to database)

### 3.2 Memory Management
- Implement optional source data compression for large payloads:
  - **Algorithm**: Use `flate2` crate with gzip compression (widely supported, good compression ratio)
  - **Threshold**: Compress sources when serialized JSON > 1MB (configurable via `SP_SOURCE_COMPRESSION_THRESHOLD`)
  - **Format**: Store as compressed bytes with metadata flag indicating compression
  - **Decompression**: Automatic on read from database or job queue
- Add configuration option to disable source tracking globally or per-workflow
- Add option to exclude specific nodes from source tracking (e.g., large data nodes)

### 3.3 Backward Compatibility
- Existing workflows without `sources` field continue to work
- Default value is empty array `[]`
- Frontend components gracefully handle missing sources
- API responses include sources only when present

### 3.4 Security
- Source data respects existing security headers exclusion (SP_DANGEROUS_HEADERS)
- Sensitive data in sources follows same sanitization rules as event data
- Source history in audit logs respects data retention policies

---

## 4. Technical Implementation

### 4.1 Core Components to Modify

#### 4.1.1 Data Models (`src/workflow/models.rs`)
- Add `NodeSource` struct
- Update `WorkflowEvent` with `sources` field
- Update `Default` implementation for `WorkflowEvent`

#### 4.1.2 Node Executor (`src/workflow/engine/node_executor.rs`)
- Add helper method `append_source()` to add current node to sources
- Modify each node type execution to call `append_source()`
- Update node execution logic to preserve sources

#### 4.1.3 DAG Executor (`src/workflow/engine/dag_executor.rs`)
- Ensure sources are properly propagated during DAG traversal
- Handle source tracking in parallel execution paths
- Manage sources in HIL multi-path execution

#### 4.1.4 JavaScript Runtime (`src/workflow/javascript.rs`)
- Expose `sources` array in JavaScript context
- Ensure proper serialization/deserialization of sources

#### 4.1.5 Database Layer
Update `workflow_execution_steps` table to store sources JSON:

```sql
-- Migration: Add sources column to workflow_execution_steps
ALTER TABLE workflow_execution_steps
ADD COLUMN sources JSONB DEFAULT '[]'::jsonb;

-- Optional: Add GIN index for source queries (Phase 2)
-- CREATE INDEX idx_execution_steps_sources
-- ON workflow_execution_steps USING GIN (sources);

-- For SQLite (if using SQLite instead of PostgreSQL)
-- ALTER TABLE workflow_execution_steps
-- ADD COLUMN sources TEXT DEFAULT '[]';
```

**Storage Considerations:**
- Sources stored as JSONB (PostgreSQL) or TEXT (SQLite)
- Compressed automatically by database if > compression threshold
- Subject to same retention policies as execution steps
- Consider partitioning for high-volume deployments

### 4.2 Configuration Options

Add environment variables:
```bash
# Enable/disable source tracking globally
SP_SOURCE_TRACKING_ENABLED=true

# Maximum number of sources to maintain (0 = unlimited)
SP_MAX_SOURCE_HISTORY=50

# Exclude specific node types from source tracking
SP_SOURCE_TRACKING_EXCLUDE="HttpRequest,OpenObserve"

# Enable source data compression for payloads > N bytes (default: 1MB)
SP_SOURCE_COMPRESSION_THRESHOLD=1048576

# Maximum loop iterations to track in sources (default: 10, 0 = track all)
SP_MAX_LOOP_ITERATIONS_TO_TRACK=10
```

Add per-workflow configuration:
```json
{
    "workflow_id": "wf-123",
    "source_tracking": {
        "enabled": true,
        "max_history": 20,
        "exclude_nodes": ["large-data-fetch"]
    }
}
```

---

## 5. Testing Requirements

### 5.1 Unit Tests
- Test source propagation through each node type
- Test source ordering and sequence numbers
- Test source limit enforcement
- Test source data sanitization

### 5.2 Integration Tests
- Test sources in linear workflows (A → B → C)
- Test sources in branching workflows (A → B → [C, D] → E)
  - Verify source union at merge point E
  - Test deduplication of common ancestor sources
  - Test with different input merge strategies
- Test sources with loop configurations
  - Test iteration metadata storage
  - Test max_iterations_to_track enforcement
- Test sources with HIL multi-path execution
  - Verify sources preserved across all paths
- Test sources with async execution (Delay, job queue resumption)

### 5.3 Performance Tests
- Benchmark workflow execution with/without source tracking
- Test memory usage with large source histories
- Test execution time with source limit vs unlimited

### 5.4 JavaScript Tests
- Test source access in transformer scripts
- Test source access in condition scripts
- Test source manipulation and edge cases

---

## 6. Migration Strategy

### 6.1 Phase 1: Core Implementation (Week 1-2)
1. Update data models and database schema
2. Implement source tracking in node executor
3. Add configuration options
4. Write unit tests

### 6.2 Phase 2: JavaScript Integration (Week 3)
1. Expose sources in JavaScript runtime
2. Update documentation and examples
3. Add integration tests

### 6.3 Phase 3: Optimization (Week 4)
1. Implement compression
2. Add caching strategies
3. Performance tuning

---

## 7. Documentation Requirements

### 7.1 User Documentation
- Guide: "Accessing Historical Data with Sources"
- Tutorial: "Building Multi-Stage Data Pipelines"
- Code examples for common patterns
- API reference updates

### 7.2 Developer Documentation
- Architecture decision record (ADR) for source tracking
- Implementation guide for node developers
- Performance considerations guide
- Migration guide for existing workflows

---

## 8. Open Questions

### 8.1 Technical
1. **Should sources include condition node results (since they're already in `condition_results`)?**
   - Recommendation: No. Condition nodes don't transform data, they only route. Keep condition results in `event.condition_results` as they serve a different purpose (routing decisions vs data lineage).

2. **How should sources behave in loop scenarios (append or overwrite)?**
   - **Option A**: Each iteration adds a new source entry
     - Pros: Complete history
     - Cons: Array bloat (100 iterations = 100 entries), exceeds default max_history
   - **Option B**: Single source entry with iteration metadata
     - Pros: Bounded size, still accessible
     - Cons: Doesn't preserve all iteration inputs
   - **Option C**: Only track first and final iteration
     - Pros: Minimal overhead
     - Cons: Loses intermediate iteration data
   - **Recommendation**: Option B - Store as `data: { iterations: [{input: ..., iteration: 1}, ...] }` with configurable `max_iterations_to_track`

3. **Should there be a way to "reset" sources at certain nodes?**
   - Recommendation: Add optional `reset_sources: true` flag in node configuration. Use case: Long workflows where early data is no longer relevant. Implementation: Clear sources array but preserve current node's input.

4. **Should sources be included in async job queue payloads?**
   - **Recommendation**: Yes. Sources must be serialized in job queue payloads for Delay, HTTP Loop, and HIL nodes. Otherwise, source history is lost on workflow resumption. Add compression if payload > 1MB.

### 8.2 Product
1. **Should source tracking be opt-in or opt-out by default?**
   - **Recommendation**: Opt-out (enabled by default). Rationale: The performance overhead is minimal (< 10% for typical workflows), and the feature provides significant value. Users who need to disable it for performance/storage reasons can set `SP_SOURCE_TRACKING_ENABLED=false`.

2. **What is the right default for max source history?**
   - **Recommendation**: 50 nodes. Rationale: Covers 99% of workflows while preventing unbounded growth. Configurable via `SP_MAX_SOURCE_HISTORY`. For very long workflows, users can increase the limit or use `reset_sources` flag at strategic points.

3. **Should sources be searchable in workflow execution history?**
   - **Recommendation**: Phase 2 feature. Initial implementation focuses on API access. Later, add database indexing and search capabilities for debugging/auditing purposes.

---

## 9. Success Metrics

### 9.1 Adoption Metrics
- % of workflows using source tracking within 3 months
- Number of transformer/condition scripts accessing sources

### 9.2 Performance Metrics
- Average execution time increase with source tracking
- Memory usage increase with source tracking
- 99th percentile latency impact

### 9.3 Quality Metrics
- Number of bugs related to source tracking
- Customer support tickets about source data access
- Documentation clarity ratings

### 9.4 Operational Metrics
- Average source array size per workflow execution
- % of executions hitting max source history limit
- Source data compression ratio (when compression enabled)
- Source-related error rate
- Database storage growth from sources (MB/day)
- Average source serialization/deserialization time
- % of workflows with source tracking disabled

---

## 10. Risks and Mitigations

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| Performance degradation | High | Medium | Implement configurable limits, compression, opt-out |
| Memory exhaustion with large sources | High | Low | Enforce max history, add monitoring/alerts |
| Breaking changes for existing workflows | Medium | Low | Full backward compatibility, default empty array |
| Complex debugging with large source histories | Medium | Medium | Add API query filters for source data |
| Database storage costs | Medium | Low | Compression, retention policies, archival |

---

## 11. Future Enhancements

### 11.1 Phase 2 Features
- **Source reset capability**: Add `reset_sources: true` flag to node configuration
  - Clears all historical sources at that node
  - Useful for long workflows where early data is no longer relevant
  - Implementation: Clear sources array but preserve current node's input
- Source data diffing (show changes between nodes)
- Source data rollback (revert to previous node's data)
- Conditional source tracking (only track specific fields)
- Source data projection (reduce payload size)

### 11.2 Advanced Features
- Source-based workflow branching (route based on historical data)
- Source data aggregation functions in JavaScript
- Cross-workflow source tracking (for workflow chains)
- Source data export for analytics

---

## 12. Appendix

### 12.1 Example Workflow Scenarios

#### Scenario 1: API Enrichment Pipeline
```
Trigger → Fetch User (HTTP) → Enrich Profile (HTTP) → Transform → Send Email

At "Send Email" node:
- event.data: Email template data (output of Transform node)
- event.sources[0]: Input to "Fetch User" (trigger data with user ID)
- event.sources[1]: Input to "Enrich Profile" (raw user data from API)
- event.sources[2]: Input to "Transform" (enriched profile data)
- event.sources[3]: Input to "Send Email" (pre-email template data)

This allows the email node to access the original trigger data (user ID),
the raw user data before enrichment, and the enriched data before transformation.
```

#### Scenario 2: Conditional Processing with Branching
```
Trigger → Parse Input → [Path A: Process Fast | Path B: Process Slow] → Merge → Output

At "Merge" node:
- event.data: Result from whichever path executed (Fast or Slow)
- event.sources[0]: Input to "Parse Input" (raw trigger data)
- event.sources[1]: Input to "Process Fast" OR "Process Slow" (parsed data)
- event.sources contains union of both branch paths if both executed

The merge node can:
- Access original unparsed input: sources.find(s => s.node_name === "Parse Input")
- Determine which path was taken: sources.find(s => s.node_name.includes("Fast"))
- Compare processing results from different paths
```

#### Scenario 3: Loop with History
```
Trigger → Initialize → HTTP Loop (5 iterations) → Aggregate Results

At "Aggregate Results" node:
- event.data: Final iteration's HTTP response
- event.sources[0]: Input to "Initialize" (trigger data)
- event.sources[1]: Input to first "HTTP Loop" iteration
- event.sources[2-5]: Inputs to subsequent iterations (if tracked separately)

Note: Loop behavior depends on implementation decision in Open Question 8.1.2:
- Option A: Each iteration adds a new source entry (5 entries for 5 iterations)
- Option B: Loop node updates single source entry with iteration metadata
- Option C: Only first and last iteration are tracked as sources

Recommendation: Option B (single entry with iteration metadata) to prevent
source array bloat in high-iteration loops.
```

### 12.2 Code Examples

#### Example 1: Accessing Specific Source by Name
```javascript
function transformer(event) {
    const apiResponse = event.sources.find(
        s => s.node_name === "External API Call"
    );

    if (!apiResponse) {
        throw new Error("External API data not found in sources");
    }

    return {
        current: event.data,
        original_api_data: apiResponse.data
    };
}
```

#### Example 2: Comparing First and Last
```javascript
function condition(event) {
    if (event.sources.length === 0) return false;

    const firstSource = event.sources[0];
    const lastSource = event.sources[event.sources.length - 1];

    return firstSource.data.status !== lastSource.data.status;
}
```

#### Example 3: Accessing Sources by Type
```javascript
function transformer(event) {
    const httpSources = event.sources.filter(
        s => s.node_type === "HttpRequest"
    );

    const transformerSources = event.sources.filter(
        s => s.node_type === "Transformer"
    );

    return {
        all_http_responses: httpSources.map(s => s.data),
        all_transformations: transformerSources.map(s => s.data),
        current: event.data
    };
}
```

---

## Approval and Sign-off

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Product Owner | | | |
| Engineering Lead | | | |
| Tech Lead | | | |
| QA Lead | | | |
