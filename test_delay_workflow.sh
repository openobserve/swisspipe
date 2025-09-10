#!/bin/bash

# Test script for delay node functionality

BASE_URL="http://localhost:3700"
WORKFLOW_NAME="delay-test-workflow"

echo "Testing Delay Node Functionality"
echo "================================"

# Create a workflow with delay node
echo "1. Creating workflow with delay node..."
WORKFLOW_RESPONSE=$(curl -s -X POST \
  "${BASE_URL}/api/v1/workflows" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "'${WORKFLOW_NAME}'",
    "description": "Test workflow with delay node"
  }')

WORKFLOW_ID=$(echo "$WORKFLOW_RESPONSE" | jq -r '.id')
echo "Created workflow: $WORKFLOW_ID"

# Add trigger node
echo "2. Adding trigger node..."
TRIGGER_RESPONSE=$(curl -s -X POST \
  "${BASE_URL}/api/v1/workflows/${WORKFLOW_ID}/nodes" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Start",
    "node_type": {
      "Trigger": {
        "methods": ["POST"]
      }
    }
  }')

TRIGGER_ID=$(echo "$TRIGGER_RESPONSE" | jq -r '.id')
echo "Created trigger node: $TRIGGER_ID"

# Add delay node
echo "3. Adding delay node..."
DELAY_RESPONSE=$(curl -s -X POST \
  "${BASE_URL}/api/v1/workflows/${WORKFLOW_ID}/nodes" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Wait5Seconds",
    "node_type": {
      "Delay": {
        "duration": 5,
        "unit": "Seconds"
      }
    }
  }')

DELAY_ID=$(echo "$DELAY_RESPONSE" | jq -r '.id')
echo "Created delay node: $DELAY_ID"

# Add transformer node to log completion
echo "4. Adding transformer node..."
TRANSFORMER_RESPONSE=$(curl -s -X POST \
  "${BASE_URL}/api/v1/workflows/${WORKFLOW_ID}/nodes" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "LogCompletion",
    "node_type": {
      "Transformer": {
        "script": "function transformer(event) { console.log(\"Delay completed at:\", new Date().toISOString()); event.data.delay_completed = true; return event; }"
      }
    }
  }')

TRANSFORMER_ID=$(echo "$TRANSFORMER_RESPONSE" | jq -r '.id')
echo "Created transformer node: $TRANSFORMER_ID"

# Create edges
echo "5. Creating edges..."
curl -s -X POST \
  "${BASE_URL}/api/v1/workflows/${WORKFLOW_ID}/edges" \
  -H "Content-Type: application/json" \
  -d '{
    "from_node_name": "Start",
    "to_node_name": "Wait5Seconds"
  }' > /dev/null

curl -s -X POST \
  "${BASE_URL}/api/v1/workflows/${WORKFLOW_ID}/edges" \
  -H "Content-Type: application/json" \
  -d '{
    "from_node_name": "Wait5Seconds",
    "to_node_name": "LogCompletion"
  }' > /dev/null

echo "Created edges between nodes"

# Execute workflow and measure time
echo "6. Executing workflow and measuring delay..."
START_TIME=$(date +%s)

EXECUTION_RESPONSE=$(curl -s -X POST \
  "${BASE_URL}/api/v1/workflows/${WORKFLOW_ID}/ep" \
  -H "Content-Type: application/json" \
  -d '{"test": "delay_functionality", "timestamp": "'$(date -Iseconds)'"}')

EXECUTION_ID=$(echo "$EXECUTION_RESPONSE" | jq -r '.execution_id')
echo "Started execution: $EXECUTION_ID"

# Wait for execution to complete and check timing
echo "7. Waiting for execution to complete..."
while true; do
  EXECUTION_STATUS=$(curl -s "${BASE_URL}/api/v1/executions/${EXECUTION_ID}" | jq -r '.status')
  
  if [ "$EXECUTION_STATUS" = "completed" ] || [ "$EXECUTION_STATUS" = "failed" ]; then
    END_TIME=$(date +%s)
    DURATION=$((END_TIME - START_TIME))
    echo "Execution completed in ${DURATION} seconds"
    echo "Status: $EXECUTION_STATUS"
    break
  fi
  
  echo "Execution status: $EXECUTION_STATUS"
  sleep 1
done

# Get execution details
echo "8. Getting execution details..."
curl -s "${BASE_URL}/api/v1/executions/${EXECUTION_ID}" | jq '.'

echo ""
echo "Test Summary:"
echo "============="
if [ $DURATION -ge 4 ] && [ $DURATION -le 7 ]; then
  echo "✅ PASS: Delay node worked correctly (${DURATION}s duration, expected ~5s)"
else
  echo "❌ FAIL: Delay duration unexpected (${DURATION}s, expected ~5s)"
fi

echo ""
echo "Delay Node Test Complete!"