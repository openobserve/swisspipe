#!/bin/bash

# SwissPipe API Test Script

BASE_URL="http://localhost:3700"
AUTH="admin:admin"

echo "🚀 SwissPipe Workflow Engine Test"
echo "=================================="

# Test 1: Create a simple workflow
echo -e "\n📝 Creating a simple workflow..."

WORKFLOW_JSON='{
  "name": "Simple Test Workflow",
  "description": "A test workflow with transformer and webhook",
  "start_node_name": "trigger",
  "nodes": [
    {
      "name": "trigger",
      "node_type": {
        "Trigger": {
          "methods": ["POST"]
        }
      }
    },
    {
      "name": "transform",
      "node_type": {
        "Transformer": {
          "script": "function transformer(event) { event.data.processed = true; event.data.timestamp = Date.now(); return event; }"
        }
      }
    },
    {
      "name": "webhook",
      "node_type": {
        "App": {
          "app_type": "Webhook",
          "url": "https://httpbin.org/post",
          "method": "POST",
          "timeout_seconds": 30,
          "retry_config": {
            "max_attempts": 3,
            "initial_delay_ms": 100,
            "max_delay_ms": 5000,
            "backoff_multiplier": 2.0
          }
        }
      }
    }
  ],
  "edges": [
    {
      "from_node_name": "trigger",
      "to_node_name": "transform",
      "condition_result": null
    },
    {
      "from_node_name": "transform",
      "to_node_name": "webhook",
      "condition_result": null
    }
  ]
}'

WORKFLOW_RESPONSE=$(curl -s -X POST "$BASE_URL/workflows" \
  -H "Content-Type: application/json" \
  -H "Authorization: Basic $(echo -n $AUTH | base64)" \
  -d "$WORKFLOW_JSON")

if [ $? -eq 0 ]; then
  echo "✅ Workflow created successfully!"
  WORKFLOW_ID=$(echo "$WORKFLOW_RESPONSE" | grep -o '"id":"[^"]*' | cut -d'"' -f4)
  ENDPOINT_URL=$(echo "$WORKFLOW_RESPONSE" | grep -o '"endpoint_url":"[^"]*' | cut -d'"' -f4)
  echo "   Workflow ID: $WORKFLOW_ID"
  echo "   Endpoint: $ENDPOINT_URL"
else
  echo "❌ Failed to create workflow"
  exit 1
fi

# Test 2: List workflows
echo -e "\n📋 Listing workflows..."
curl -s -X GET "$BASE_URL/workflows" \
  -H "Authorization: Basic $(echo -n $AUTH | base64)" | jq '.' 2>/dev/null || echo "JSON parsing failed, raw response above"

# Test 3: Trigger the workflow
if [ ! -z "$WORKFLOW_ID" ]; then
  echo -e "\n🔥 Triggering workflow execution..."
  
  TEST_DATA='{
    "message": "Hello from SwissPipe!",
    "user_id": 123,
    "action": "test_workflow"
  }'
  
  EXECUTION_RESULT=$(curl -s -X POST "$BASE_URL/api/v1/$WORKFLOW_ID/ep" \
    -H "Content-Type: application/json" \
    -d "$TEST_DATA")
  
  if [ $? -eq 0 ]; then
    echo "✅ Workflow executed successfully!"
    echo "   Result:"
    echo "$EXECUTION_RESULT" | jq '.' 2>/dev/null || echo "$EXECUTION_RESULT"
  else
    echo "❌ Failed to execute workflow"
  fi
fi

echo -e "\n🎉 Test completed!"
echo "You can now interact with SwissPipe at $BASE_URL"