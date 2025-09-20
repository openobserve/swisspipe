# Enhanced Input Data for Newly Added Nodes

## Problem Statement

When transformer or condition nodes are added to a workflow after previous executions have run, selecting past executions shows unhelpful informational messages instead of realistic test data. Currently, users see:

```json
{
  "info": "This transformer node 'node-xyz' was not present during this execution. No execution data is available for testing.",
  "suggestion": "Please run a new workflow execution to see input data for this transformer node."
}
```

## Proposed Solution

Fetch data from the output of the previous node(s) in the workflow graph to provide realistic test data for newly added nodes, even when they weren't present during the selected execution.

## Technical Investigation Results

### ✅ Feasibility Assessment: **FULLY FEASIBLE**

The current SwissPipe architecture provides all necessary components:

#### Available Data Structures
- **Workflow Graph**: `workflowStore.currentWorkflow` contains complete nodes + edges structure
- **Edge Relationships**: Each edge has `from_node_id` and `to_node_id` showing exact node connections
- **Execution Steps**: All steps contain both `input_data` and `output_data`
- **Current Context**: Both config components already have access to `nodeId` and `workflowStore`

#### Required APIs (Already Available)
- `apiClient.getExecutionSteps(executionId)` - Gets all execution steps
- `workflowStore.currentWorkflow` - Contains workflow structure with nodes/edges
- Both components already call these APIs

## Implementation Strategy

### Simplified "One Node Back" Approach

The implementation focuses on checking only **immediate predecessors** (one level back) for realistic test data. No complex graph traversal or deep ancestor searching.

## Implementation Approach

**Single Phase Implementation**:
- Check immediate predecessors only (one level back)
- Handle both single and multiple predecessor scenarios
- Use clear informational messages when data unavailable
- Implement in both TransformerConfig and ConditionConfig simultaneously

## Edge Cases to Handle

### Real Edge Cases for DAG-Based Workflows

Since SwissPipe uses DAGs (Directed Acyclic Graphs), **circular references are impossible** and don't need to be handled. The real edge cases are:

### 1. Multiple Predecessors (Branch Convergence)

**Most Common**: New node added after condition branches merge
```
Trigger → Condition → [True Branch] → [NEW NODE]
                  ↘ [False Branch] ↗
```

**Challenge**: Which predecessor's output should we use?

**Solution Strategy**:
- **Single Predecessor**: Use output directly
- **Condition Branches**: Show available path data with context about branch uncertainty
- **Parallel Processing**: Create merged array structure mimicking SwissPipe's input coordination
- **Default**: Use first available output with clear labeling

### 2. Condition Branch Selection

**Problem**: New node after condition split - we can't know which branch was taken without execution data.

**Solution**:
- Check edge `condition_result` metadata (true/false/null)
- Prefer available data from any branch
- Add contextual message: "Data from condition branch (actual path may vary)"

### 3. Parallel Processing Convergence

**Scenario**: Node added after multiple parallel paths merge
```
Trigger → [Transform A] → [NEW NODE]
        → [Transform B] ↗
        → [Transform C] ↗
```

**Solution**: Create merged input structure:
```javascript
// Mirror SwissPipe's input coordination behavior - simple array format
mergedData = [transformA_output, transformB_output, transformC_output]
```

### 4. Missing Predecessor Data

**Scenarios**:
- Predecessor node failed (no `output_data`)
- Predecessor node was skipped due to conditions
- Predecessor node didn't complete

**Solution**: Show informational message requesting new execution

### 5. Simple Edge Cases
- **Unreachable Nodes**: Handle disconnected or isolated nodes
- **No Output Data**: Immediate predecessors exist but have no output data

### Simplified Fallback Strategy (One Node Back Only)

```javascript
async function getInputDataForNewNode(nodeId, executionSteps) {
  const currentWorkflow = workflowStore.currentWorkflow
  // 1. TRY: Direct execution step (shouldn't exist for new nodes)
  const directStep = executionSteps.find(step => step.node_id === nodeId)
  if (directStep?.input_data) {
    return { data: directStep.input_data, source: 'direct' }
  }

  // 2. TRY: Immediate predecessor outputs ONLY
  const predecessorOutput = await findImmediatePredecessorOutput(nodeId, executionSteps)
  if (predecessorOutput) {
    return { data: predecessorOutput, source: 'predecessor' }
  }

  // 3. FINAL: Informational message - NO fallback to trigger data
  const incomingEdges = currentWorkflow?.edges?.filter(e => e.to_node_id === nodeId) || []

  return {
    data: {
      info: `This node '${nodeId}' was not present during this execution.`,
      suggestion: 'Please run a new workflow execution to see realistic input data for testing.',
      reason: incomingEdges.length === 0
        ? 'No immediate predecessor nodes found.'
        : 'Immediate predecessor nodes have no output data.'
    },
    source: 'none'
  }
}

async function findImmediatePredecessorOutput(nodeId, executionSteps) {
  const currentWorkflow = workflowStore.currentWorkflow
  if (!currentWorkflow) return null

  // Find all edges leading TO this node
  const incomingEdges = currentWorkflow.edges.filter(e => e.to_node_id === nodeId)

  if (incomingEdges.length === 0) {
    return null // No predecessors
  }

  // Get output data from immediate predecessors only
  const predecessorOutputs = []
  for (const edge of incomingEdges) {
    const predStep = executionSteps.find(step => step.node_id === edge.from_node_id)
    if (predStep?.output_data) {
      predecessorOutputs.push({
        nodeId: edge.from_node_id,
        conditionResult: edge.condition_result,
        data: predStep.output_data
      })
    }
  }

  if (predecessorOutputs.length === 0) {
    return null // Predecessors exist but no output data
  }

  // Handle multiple predecessors
  return selectBestImmediatePredecessor(predecessorOutputs)
}

function selectBestImmediatePredecessor(predecessorOutputs) {
  if (predecessorOutputs.length === 1) {
    // Single predecessor: use directly
    return predecessorOutputs[0].data
  }

  // Multiple predecessors: create merged structure for parallel processing
  // or select first available for condition branches
  if (predecessorOutputs.some(p => p.conditionResult !== null)) {
    // This is after condition branches - use any available path
    return predecessorOutputs[0].data
  } else {
    // This is after parallel processing - create merged array
    return predecessorOutputs.map(output => output.data)
  }
}
```

## Benefits

### User Experience
- **✅ Realistic Test Data**: When available, users get actual data that would flow to their node
- **✅ Better Development**: Can test logic with meaningful data when predecessors have output
- **✅ Clear Guidance**: Users know exactly when they need to run new executions for test data
- **✅ Predictable Behavior**: Consistent logic for when data is/isn't available

### Technical Benefits
- **✅ No API Changes**: Uses existing data structures and endpoints
- **✅ Backward Compatible**: Maintains existing behavior for edge cases
- **✅ Simple Logic**: Straightforward implementation with predictable behavior
- **✅ Consistent UX**: Same enhancement for both transformer and condition nodes

## Files to Modify

1. **TransformerConfig.vue**: Update fallback logic in `onExecutionSelect` function
2. **ConditionConfig.vue**: Update fallback logic in `onExecutionSelect` function
3. **Optional**: Create utility functions for immediate predecessor detection

## Testing Strategy

### Test Scenarios (Simplified)
1. **Single Predecessor**: New node after single transformer
2. **Condition Branch Convergence**: New node after true/false paths merge
3. **Parallel Processing Merge**: New node after multiple parallel paths converge
4. **Missing Predecessor Data**: Predecessor exists but failed/has no output
5. **Unreachable Nodes**: New node disconnected from main workflow path

### Validation Points
- **Immediate Predecessor Identification**: Correct identification of direct predecessors
- **Condition Branch Handling**: Proper selection when multiple immediate predecessors available
- **Parallel Input Merging**: Correct array creation for multiple immediate inputs
- **Data Source Labeling**: Clear indication of where test data originated (predecessor vs none)
- **User Messaging**: Clear explanations when data is/isn't available

### Simplified Edge Case Testing Matrix
| Scenario | Expected Behavior | Data Source | User Message |
|----------|-------------------|-------------|--------------|
| Single predecessor with data | Use predecessor output | `predecessor` | "Test data from immediate predecessor node" |
| Condition branch (true/false) | Use available branch data | `predecessor` | "Data from condition branch (path may vary in actual execution)" |
| Parallel merge (3 inputs) | Create merged array | `predecessor` | "Array of data from 3 immediate predecessor nodes" |
| Predecessor exists, no output | Show informational message | `none` | "Predecessor nodes have no output data - run new execution" |
| No predecessors (isolated) | Show informational message | `none` | "No predecessor nodes found - run new execution" |
| Node was present in execution | Use actual step data | `direct` | "Actual execution data from this node" |

### Key Simplifications
- **No deep traversal**: Only check immediate predecessors (one level back)
- **No trigger fallback**: Don't show misleading workflow trigger data
- **Clear user guidance**: Always explain why data is/isn't available
- **Predictable behavior**: Users know exactly what data source they're seeing

## Priority: High

This enhancement significantly improves the developer experience by providing realistic test data for newly added nodes, reducing the friction in workflow development and testing.