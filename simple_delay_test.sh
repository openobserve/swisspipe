#!/bin/bash

# Simple delay node test

PORT=3730
BASE_URL="http://localhost:${PORT}"

echo "Testing Delay Node on port ${PORT}"
echo "=================================="

# Create workflow with delay
echo "1. Creating workflow..."
WORKFLOW_RESPONSE=$(curl -s -X POST \
  "${BASE_URL}/api/v1/workflows" \
  -H "Content-Type: application/json" \
  -d '{"name": "delay-test", "description": "Test delay node"}')

echo "Workflow response: $WORKFLOW_RESPONSE"
WORKFLOW_ID=$(echo "$WORKFLOW_RESPONSE" | jq -r '.id')

if [ "$WORKFLOW_ID" = "null" ] || [ -z "$WORKFLOW_ID" ]; then
  echo "Failed to create workflow"
  exit 1
fi

echo "Created workflow: $WORKFLOW_ID"

# Add trigger node
echo "2. Adding trigger node..."
TRIGGER_RESPONSE=$(curl -s -X POST \
  "${BASE_URL}/api/v1/workflows/${WORKFLOW_ID}/nodes" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Start",
    "node_type": {
      "Trigger": {"methods": ["POST"]}
    }
  }')

echo "Trigger response: $TRIGGER_RESPONSE"

# Add delay node (3 second delay for quick testing)
echo "3. Adding delay node..."
DELAY_RESPONSE=$(curl -s -X POST \
  "${BASE_URL}/api/v1/workflows/${WORKFLOW_ID}/nodes" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Wait3Seconds", 
    "node_type": {
      "Delay": {"duration": 3, "unit": "Seconds"}
    }
  }')

echo "Delay response: $DELAY_RESPONSE"

# Create edge
echo "4. Creating edge..."
EDGE_RESPONSE=$(curl -s -X POST \
  "${BASE_URL}/api/v1/workflows/${WORKFLOW_ID}/edges" \
  -H "Content-Type: application/json" \
  -d '{
    "from_node_name": "Start",
    "to_node_name": "Wait3Seconds"
  }')

echo "Edge response: $EDGE_RESPONSE"

# Execute and time
echo "5. Executing workflow..."
START_TIME=$(date +%s)

EXEC_RESPONSE=$(curl -s -X POST \
  "${BASE_URL}/api/v1/workflows/${WORKFLOW_ID}/trigger" \
  -H "Content-Type: application/json" \
  -d '{"test": "delay"}')

echo "Execution response: $EXEC_RESPONSE"
EXEC_ID=$(echo "$EXEC_RESPONSE" | jq -r '.execution_id')

if [ "$EXEC_ID" = "null" ] || [ -z "$EXEC_ID" ]; then
  echo "Failed to start execution"
  exit 1
fi

echo "Started execution: $EXEC_ID"

# Wait for completion
echo "6. Waiting for completion..."
for i in {1..15}; do
  STATUS_RESPONSE=$(curl -s "${BASE_URL}/api/v1/executions/${EXEC_ID}")
  STATUS=$(echo "$STATUS_RESPONSE" | jq -r '.status')
  
  echo "Check $i: Status = $STATUS"
  
  if [ "$STATUS" = "completed" ] || [ "$STATUS" = "failed" ]; then
    END_TIME=$(date +%s)
    DURATION=$((END_TIME - START_TIME))
    echo "Execution finished in ${DURATION} seconds with status: $STATUS"
    
    if [ $DURATION -ge 2 ] && [ $DURATION -le 5 ] && [ "$STATUS" = "completed" ]; then
      echo "✅ SUCCESS: Delay node worked correctly!"
    else
      echo "❌ ISSUE: Expected 3s delay and completed status"
    fi
    
    echo ""
    echo "Final execution details:"
    echo "$STATUS_RESPONSE" | jq '.'
    exit 0
  fi
  
  sleep 1
done

echo "❌ TIMEOUT: Execution did not complete within 15 seconds"