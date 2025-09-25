#!/bin/bash

set -e

echo "🧪 Minimal HTTP Loop Test"
echo "========================="

SERVER_PORT=3750
BASE_URL="http://localhost:$SERVER_PORT"

# Generate unique node IDs
TRIGGER_ID=$(uuidgen | tr '[:upper:]' '[:lower:]')
LOOP_ID=$(uuidgen | tr '[:upper:]' '[:lower:]')

echo "📋 Creating minimal workflow with just HTTP loop..."

# Create workflow with only trigger and HTTP loop
WORKFLOW_RESPONSE=$(curl -s -w "\nHTTP_STATUS_CODE:%{http_code}" -X POST "$BASE_URL/api/admin/v1/workflows" \
  -H "Content-Type: application/json" \
  -u "admin:admin" \
  -d @- << EOF
{
  "name": "Minimal HTTP Loop Test",
  "description": "Minimal test with only HTTP loop",
  "start_node_id": "$TRIGGER_ID",
  "nodes": [
    {
      "id": "$TRIGGER_ID",
      "name": "Start",
      "node_type": {
        "Trigger": {
          "methods": ["POST"]
        }
      }
    },
    {
      "id": "$LOOP_ID",
      "name": "HTTP Loop Node",
      "node_type": {
        "HttpRequest": {
          "url": "https://httpbin.org/delay/1",
          "method": "GET",
          "timeout_seconds": 10,
          "failure_action": "Continue",
          "retry_config": {
            "max_attempts": 1,
            "initial_delay_ms": 100,
            "max_delay_ms": 1000,
            "backoff_multiplier": 2.0
          },
          "headers": {},
          "loop_config": {
            "max_iterations": 2,
            "interval_seconds": 2,
            "backoff_strategy": {
              "Fixed": 2
            }
          }
        }
      }
    }
  ],
  "edges": [
    {
      "from_node_id": "$TRIGGER_ID",
      "to_node_id": "$LOOP_ID"
    }
  ]
}
EOF
)

# Extract status code
HTTP_STATUS=$(echo "$WORKFLOW_RESPONSE" | grep "HTTP_STATUS_CODE:" | cut -d: -f2)
WORKFLOW_BODY=$(echo "$WORKFLOW_RESPONSE" | grep -v "HTTP_STATUS_CODE:")

echo "Workflow creation status: $HTTP_STATUS"

if [ "$HTTP_STATUS" != "200" ] && [ "$HTTP_STATUS" != "201" ]; then
  echo "❌ Failed to create workflow (status: $HTTP_STATUS)"
  echo "Response: $WORKFLOW_BODY"
  exit 1
fi

# Extract the actual workflow ID from the response
WORKFLOW_ID=$(echo "$WORKFLOW_BODY" | python3 -c "import sys, json; print(json.load(sys.stdin)['id'])")

echo "✅ Workflow created with ID: $WORKFLOW_ID"
echo ""

# Execute workflow and measure timing
echo "🚀 Triggering workflow execution..."
START_TIME=$(date +%s)

EXECUTION_RESPONSE=$(curl -s -w "\nHTTP_STATUS_CODE:%{http_code}" -X POST "$BASE_URL/api/v1/$WORKFLOW_ID/trigger" \
  -H "Content-Type: application/json" \
  -d '{"test_data": "minimal_test"}')

EXEC_STATUS=$(echo "$EXECUTION_RESPONSE" | grep "HTTP_STATUS_CODE:" | cut -d: -f2)
EXEC_BODY=$(echo "$EXECUTION_RESPONSE" | grep -v "HTTP_STATUS_CODE:")

END_TIME=$(date +%s)
EXECUTION_TIME=$((END_TIME - START_TIME))

echo "⏱️  Workflow execution completed in: ${EXECUTION_TIME}s"
echo "Execution status: $EXEC_STATUS"
echo "Response: $EXEC_BODY"

# Extract execution ID for monitoring
EXECUTION_ID=$(echo "$EXEC_BODY" | python3 -c "import sys, json; print(json.load(sys.stdin)['execution_id'])" 2>/dev/null || echo "unknown")

echo ""
echo "📊 Monitoring execution: $EXECUTION_ID"

# Give the execution some time to complete
echo "⏳ Waiting 8 seconds for execution to complete..."
sleep 8

# Check final results
echo "🔍 Checking final job status..."
sqlite3 data/swisspipe.db "SELECT id, status, error_message FROM job_queue WHERE execution_id = '$EXECUTION_ID';" || echo "Could not query job status"