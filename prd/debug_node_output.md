# Node Output Propagation Analysis

## Current Flow Analysis

Based on my code review, here's what's happening:

### Single Node Chain (A → B)
1. Node A executes with `input_event`
2. Node A produces `output_event` 
3. Node A's `output_event` becomes Node B's `input_event` ✅

### Multiple Input Node (A → C, B → C)
1. Node A executes with `input_event_a` → produces `output_event_a`
2. Node B executes with `input_event_b` → produces `output_event_b`
3. Both branches reach Node C simultaneously
4. Input coordination triggers:
   - Branch A calls `coordinate_node_inputs(workflow, execution_id, "C", &output_event_a)`
   - Branch B calls `coordinate_node_inputs(workflow, execution_id, "C", &output_event_b)`
5. Input sync service receives both events and merges them
6. Node C executes with the merged event ✅

## The Real Issue

Looking at the code more carefully, I think the issue might not be with the basic output passing, but with **how the outputs are passed when using the async worker pool vs the sync workflow engine**.

Let me check if there's a discrepancy between:
1. `WorkflowEngine.execute_workflow()` (sync)
2. `WorkerPool.execute_workflow_from_node()` (async)

## Hypothesis

The issue might be that in complex workflows with multiple branches and input coordination, **the event that reaches the input coordination might not contain the full output from the previous node's execution**.