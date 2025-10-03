#!/bin/bash

# Test script for event data templating in HTTP URLs
set -e

BASE_URL="http://localhost:3700"
ADMIN_USER="admin"
ADMIN_PASS="admin"

echo "=========================================="
echo "Testing Event Data Templating in HTTP URLs"
echo "=========================================="

# Step 1: Create a workflow with HTTP node that uses event data in URL
echo -e "\n1. Creating workflow with event data templating in HTTP URL..."
WORKFLOW_RESPONSE=$(curl -s -X POST \
  "$BASE_URL/api/v1/workflows" \
  -H "Content-Type: application/json" \
  -u "$ADMIN_USER:$ADMIN_PASS" \
  -d '{
    "name": "Event Data Template Test",
    "description": "Test workflow for event data templating in URLs",
    "nodes": [
      {
        "id": "trigger-1",
        "name": "HTTP Trigger",
        "type": "Trigger",
        "position": {"x": 100, "y": 100},
        "config": {
          "Trigger": {
            "path": "trigger",
            "method": "Post"
          }
        }
      },
      {
        "id": "http-1",
        "name": "HTTP Request with Event Data",
        "type": "HttpRequest",
        "position": {"x": 300, "y": 100},
        "config": {
          "HttpRequest": {
            "url": "https://jsonplaceholder.typicode.com/users/{{ event.data.user_id }}",
            "method": "Get",
            "timeout_seconds": 30,
            "failure_action": "Retry",
            "retry_config": {
              "max_attempts": 3,
              "initial_interval_ms": 1000,
              "max_interval_ms": 10000,
              "multiplier": 2.0
            },
            "headers": {}
          }
        }
      }
    ],
    "edges": [
      {
        "id": "edge-1",
        "source": "trigger-1",
        "target": "http-1",
        "sourceHandle": "success",
        "targetHandle": "input"
      }
    ]
  }')

WORKFLOW_ID=$(echo "$WORKFLOW_RESPONSE" | grep -o '"id":"[^"]*"' | head -1 | cut -d'"' -f4)
echo "Workflow created with ID: $WORKFLOW_ID"

# Step 2: Trigger the workflow with event data
echo -e "\n2. Triggering workflow with user_id in event data..."
TRIGGER_RESPONSE=$(curl -s -X POST \
  "$BASE_URL/api/v1/$WORKFLOW_ID/trigger" \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "3",
    "test": "event_templating"
  }')

echo "Trigger response:"
echo "$TRIGGER_RESPONSE" | python3 -m json.tool 2>/dev/null || echo "$TRIGGER_RESPONSE"

# Extract execution ID
EXECUTION_ID=$(echo "$TRIGGER_RESPONSE" | grep -o '"execution_id":"[^"]*"' | cut -d'"' -f4)
echo -e "\nExecution ID: $EXECUTION_ID"

# Step 3: Wait a moment for execution to complete
echo -e "\n3. Waiting for execution to complete..."
sleep 3

# Step 4: Get execution details
echo -e "\n4. Checking execution details..."
EXECUTION_DETAILS=$(curl -s -X GET \
  "$BASE_URL/api/v1/executions/$EXECUTION_ID" \
  -u "$ADMIN_USER:$ADMIN_PASS")

echo "Execution details:"
echo "$EXECUTION_DETAILS" | python3 -m json.tool 2>/dev/null || echo "$EXECUTION_DETAILS"

# Step 5: Check if the HTTP request was made to the correct URL with templated user_id
echo -e "\n5. Verifying templating worked..."
if echo "$EXECUTION_DETAILS" | grep -q "users/3"; then
    echo "✓ SUCCESS: Event data templating worked! URL was resolved to include user_id=3"
else
    echo "✗ FAILED: Could not verify event data templating"
fi

echo -e "\n=========================================="
echo "Test completed!"
echo "=========================================="
