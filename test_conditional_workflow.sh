#!/bin/bash

# Test Conditional Workflow Logic

BASE_URL="http://localhost:3700"
AUTH="admin:admin"

echo "🧪 Testing Conditional Workflow Logic"
echo "====================================="

# Create a conditional workflow with true/false paths
echo -e "\n📝 Creating conditional workflow..."

CONDITIONAL_WORKFLOW='{
  "name": "Conditional Test Workflow",
  "description": "Tests condition-based routing",
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
      "name": "check_value",
      "node_type": {
        "Condition": {
          "script": "function condition(event) { return event.data.value > 50; }"
        }
      }
    },
    {
      "name": "high_value_handler",
      "node_type": {
        "Transformer": {
          "script": "function transformer(event) { event.data.category = \"high\"; event.data.processed_by = \"high_value_handler\"; return event; }"
        }
      }
    },
    {
      "name": "low_value_handler", 
      "node_type": {
        "Transformer": {
          "script": "function transformer(event) { event.data.category = \"low\"; event.data.processed_by = \"low_value_handler\"; return event; }"
        }
      }
    },
    {
      "name": "final_webhook",
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
      "to_node_name": "check_value",
      "condition_result": null
    },
    {
      "from_node_name": "check_value",
      "to_node_name": "high_value_handler",
      "condition_result": true
    },
    {
      "from_node_name": "check_value",
      "to_node_name": "low_value_handler", 
      "condition_result": false
    },
    {
      "from_node_name": "high_value_handler",
      "to_node_name": "final_webhook",
      "condition_result": null
    },
    {
      "from_node_name": "low_value_handler",
      "to_node_name": "final_webhook",
      "condition_result": null
    }
  ]
}'

WORKFLOW_RESPONSE=$(curl -s -X POST "$BASE_URL/workflows" \
  -H "Content-Type: application/json" \
  -H "Authorization: Basic $(echo -n $AUTH | base64)" \
  -d "$CONDITIONAL_WORKFLOW")

if [ $? -eq 0 ]; then
  echo "✅ Conditional workflow created successfully!"
  WORKFLOW_ID=$(echo "$WORKFLOW_RESPONSE" | grep -o '"id":"[^"]*' | cut -d'"' -f4)
  echo "   Workflow ID: $WORKFLOW_ID"
else
  echo "❌ Failed to create conditional workflow"
  exit 1
fi

# Test Case 1: High value (condition should be true)
echo -e "\n🔥 Test Case 1: High value (value=75, should go to high_value_handler)"

HIGH_VALUE_DATA='{
  "value": 75,
  "description": "This should trigger the high value path"
}'

HIGH_VALUE_RESULT=$(curl -s -X POST "$BASE_URL/api/v1/$WORKFLOW_ID/trigger" \
  -H "Content-Type: application/json" \
  -d "$HIGH_VALUE_DATA")

echo "Result:"
echo "$HIGH_VALUE_RESULT" | jq -r '.category // "ERROR: No category set"'
echo "$HIGH_VALUE_RESULT" | jq -r '.processed_by // "ERROR: No processed_by set"'

# Test Case 2: Low value (condition should be false)
echo -e "\n🔥 Test Case 2: Low value (value=25, should go to low_value_handler)"

LOW_VALUE_DATA='{
  "value": 25,
  "description": "This should trigger the low value path"
}'

LOW_VALUE_RESULT=$(curl -s -X POST "$BASE_URL/api/v1/$WORKFLOW_ID/trigger" \
  -H "Content-Type: application/json" \
  -d "$LOW_VALUE_DATA")

echo "Result:"
echo "$LOW_VALUE_RESULT" | jq -r '.category // "ERROR: No category set"'
echo "$LOW_VALUE_RESULT" | jq -r '.processed_by // "ERROR: No processed_by set"'

# Test Case 3: Edge case (value=50, should be false)
echo -e "\n🔥 Test Case 3: Edge case (value=50, should go to low_value_handler)"

EDGE_VALUE_DATA='{
  "value": 50,
  "description": "This should trigger the low value path (not greater than 50)"
}'

EDGE_VALUE_RESULT=$(curl -s -X POST "$BASE_URL/api/v1/$WORKFLOW_ID/trigger" \
  -H "Content-Type: application/json" \
  -d "$EDGE_VALUE_DATA")

echo "Result:"
echo "$EDGE_VALUE_RESULT" | jq -r '.category // "ERROR: No category set"'
echo "$EDGE_VALUE_RESULT" | jq -r '.processed_by // "ERROR: No processed_by set"'

echo -e "\n🎉 Conditional workflow testing completed!"

echo -e "\n📊 Expected Results:"
echo "   Case 1 (value=75): category=high, processed_by=high_value_handler"
echo "   Case 2 (value=25): category=low, processed_by=low_value_handler" 
echo "   Case 3 (value=50): category=low, processed_by=low_value_handler"