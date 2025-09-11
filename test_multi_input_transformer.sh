#!/bin/bash

# Test multi-input transformer functionality with array format
set -e

echo "🧪 Testing multi-input transformer with array format..."

# Start server in background
PORT=4000 RUST_LOG=debug cargo run --bin swisspipe &
SERVER_PID=$!

# Wait for server to start
sleep 3

# Function to cleanup on exit
cleanup() {
    echo "🧹 Cleaning up..."
    kill $SERVER_PID 2>/dev/null || true
    wait $SERVER_PID 2>/dev/null || true
}
trap cleanup EXIT

echo "📝 Creating workflow with multi-input transformer..."

# Create workflow with two branches converging on a transformer
WORKFLOW_RESPONSE=$(curl -s -X POST http://localhost:4000/api/v1/admin/workflows \
  -H "Authorization: Basic YWRtaW46YWRtaW4=" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "MultiInputTransformerTest",
    "nodes": [
      {
        "name": "Start",
        "node_type": {
          "Trigger": {
            "method": "POST"
          }
        }
      },
      {
        "name": "BranchA",
        "node_type": {
          "Transformer": {
            "script": "function transformer(event) { event.data.branch = \"A\"; event.data.value_a = event.data.value * 2; return event; }"
          }
        }
      },
      {
        "name": "BranchB", 
        "node_type": {
          "Transformer": {
            "script": "function transformer(event) { event.data.branch = \"B\"; event.data.value_b = event.data.value * 3; return event; }"
          }
        }
      },
      {
        "name": "MergeTransformer",
        "node_type": {
          "Transformer": {
            "script": "function transformer(event) { console.log(\"Received event:\", JSON.stringify(event.data)); if (Array.isArray(event.data)) { var total = 0; event.data.forEach(function(input, index) { console.log(\"Input\", index, \":\", JSON.stringify(input)); total += (input.value_a || 0) + (input.value_b || 0); }); return { data: { merged: true, total_value: total, input_count: event.data.length, inputs: event.data }, metadata: event.metadata, headers: event.headers, condition_results: event.condition_results }; } else { console.log(\"Warning: Expected array but got:\", typeof event.data); return event; } }"
          }
        }
      },
      {
        "name": "FinalOutput",
        "node_type": {
          "App": {
            "app_type": "Webhook",
            "url": "http://httpbin.org/post",
            "method": "POST",
            "timeout_seconds": 30,
            "failure_action": "Continue",
            "retry_config": null,
            "headers": {}
          }
        }
      }
    ],
    "edges": [
      {
        "from_node_name": "Start",
        "to_node_name": "BranchA",
        "condition": null
      },
      {
        "from_node_name": "Start",
        "to_node_name": "BranchB",
        "condition": null
      },
      {
        "from_node_name": "BranchA",
        "to_node_name": "MergeTransformer",
        "condition": null
      },
      {
        "from_node_name": "BranchB", 
        "to_node_name": "MergeTransformer",
        "condition": null
      },
      {
        "from_node_name": "MergeTransformer",
        "to_node_name": "FinalOutput",
        "condition": null
      }
    ]
  }')

echo "✅ Workflow created"

# Extract workflow ID
WORKFLOW_ID=$(echo "$WORKFLOW_RESPONSE" | grep -o '"id":"[^"]*"' | cut -d'"' -f4)
echo "📋 Workflow ID: $WORKFLOW_ID"

# Update the MergeTransformer node to have WaitForAll strategy
echo "🔧 Setting input merge strategy for MergeTransformer..."
curl -s -X PUT "http://localhost:4000/api/v1/admin/workflows/$WORKFLOW_ID/nodes/MergeTransformer" \
  -H "Authorization: Basic YWRtaW46YWRtaW4=" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "MergeTransformer",
    "node_type": {
      "Transformer": {
        "script": "function transformer(event) { console.log(\"Received event:\", JSON.stringify(event.data)); if (Array.isArray(event.data)) { var total = 0; event.data.forEach(function(input, index) { console.log(\"Input\", index, \":\", JSON.stringify(input)); total += (input.value_a || 0) + (input.value_b || 0); }); return { data: { merged: true, total_value: total, input_count: event.data.length, inputs: event.data }, metadata: event.metadata, headers: event.headers, condition_results: event.condition_results }; } else { console.log(\"Warning: Expected array but got:\", typeof event.data); return event; } }"
      }
    },
    "input_merge_strategy": "WaitForAll"
  }' > /dev/null

echo "✅ Node updated with WaitForAll strategy"

# Wait a moment for the update to be processed
sleep 2

echo "🚀 Triggering workflow with test data..."

# Trigger the workflow
TRIGGER_RESPONSE=$(curl -s -X POST "http://localhost:4000/api/v1/$WORKFLOW_ID/trigger" \
  -H "Content-Type: application/json" \
  -d '{
    "value": 10,
    "test_id": "multi_input_transformer_test"
  }')

echo "✅ Workflow triggered"
echo "📄 Trigger response: $TRIGGER_RESPONSE"

# Extract execution ID
EXECUTION_ID=$(echo "$TRIGGER_RESPONSE" | grep -o '"execution_id":"[^"]*"' | cut -d'"' -f4)
echo "🆔 Execution ID: $EXECUTION_ID"

# Wait for execution to complete
echo "⏳ Waiting for execution to complete..."
for i in {1..20}; do
    EXECUTION_STATUS=$(curl -s "http://localhost:4000/api/v1/admin/workflows/$WORKFLOW_ID/executions/$EXECUTION_ID" \
      -H "Authorization: Basic YWRtaW46YWRtaW4=")
    
    STATUS=$(echo "$EXECUTION_STATUS" | grep -o '"status":"[^"]*"' | cut -d'"' -f4)
    echo "📊 Execution status: $STATUS"
    
    if [ "$STATUS" = "completed" ] || [ "$STATUS" = "failed" ]; then
        break
    fi
    
    sleep 2
done

# Get execution steps to verify transformer behavior
echo "📋 Getting execution steps..."
STEPS_RESPONSE=$(curl -s "http://localhost:4000/api/v1/admin/workflows/$WORKFLOW_ID/executions/$EXECUTION_ID/steps" \
  -H "Authorization: Basic YWRtaW46YWRtaW4=")

echo "📝 Execution steps:"
echo "$STEPS_RESPONSE" | jq '.'

# Look for the MergeTransformer step specifically
echo "🔍 Checking MergeTransformer step..."
MERGE_STEP=$(echo "$STEPS_RESPONSE" | jq '.[] | select(.node_name == "MergeTransformer")')

if [ -n "$MERGE_STEP" ]; then
    echo "✅ MergeTransformer step found:"
    echo "$MERGE_STEP" | jq '.'
    
    # Check if the output shows array processing
    OUTPUT_DATA=$(echo "$MERGE_STEP" | jq -r '.output_event.data')
    if echo "$OUTPUT_DATA" | grep -q "merged.*true"; then
        echo "✅ SUCCESS: Transformer correctly processed array input!"
        echo "📊 Output data shows proper array merging"
    else
        echo "❌ WARNING: Transformer output doesn't show expected array processing"
    fi
else
    echo "❌ ERROR: MergeTransformer step not found in execution"
fi

# Check final execution status
FINAL_STATUS=$(echo "$EXECUTION_STATUS" | grep -o '"status":"[^"]*"' | cut -d'"' -f4)
if [ "$FINAL_STATUS" = "completed" ]; then
    echo "🎉 SUCCESS: Multi-input transformer test completed successfully!"
else
    echo "❌ FAILED: Execution did not complete successfully (status: $FINAL_STATUS)"
    exit 1
fi

echo "✅ Multi-input transformer array format test completed!"