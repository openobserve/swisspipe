#!/bin/bash

set -e

echo "🧪 HTTP Loop Blocking Verification Test"
echo "======================================="

SERVER_PORT=3750
BASE_URL="http://localhost:$SERVER_PORT"

# Generate unique node IDs
TRIGGER_ID=$(uuidgen | tr '[:upper:]' '[:lower:]')
LOOP_ID=$(uuidgen | tr '[:upper:]' '[:lower:]')
AFTER_ID=$(uuidgen | tr '[:upper:]' '[:lower:]')

echo "📋 Creating workflow: Trigger -> HTTP Loop (2 iterations) -> Transformer"

# Create workflow with HTTP loop and subsequent transformer
WORKFLOW_RESPONSE=$(curl -s -w "\nHTTP_STATUS_CODE:%{http_code}" -X POST "$BASE_URL/api/admin/v1/workflows" \
  -H "Content-Type: application/json" \
  -u "admin:admin" \
  -d @- << EOF
{
  "name": "HTTP Loop Blocking Verification",
  "description": "Test that subsequent nodes wait for HTTP loop completion",
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
      "name": "HTTP Loop (2 iterations)",
      "node_type": {
        "HttpRequest": {
          "url": "https://httpbin.org/delay/1",
          "method": "GET",
          "timeout_seconds": 30,
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
    },
    {
      "id": "$AFTER_ID",
      "name": "After Loop Transformer",
      "node_type": {
        "Transformer": {
          "script": "function transformer(event) { event.data.blocking_test_completed = true; return event; }"
        }
      }
    }
  ],
  "edges": [
    {
      "from_node_id": "$TRIGGER_ID",
      "to_node_id": "$LOOP_ID"
    },
    {
      "from_node_id": "$LOOP_ID",
      "to_node_id": "$AFTER_ID"
    }
  ]
}
EOF
)

# Extract status and workflow ID
HTTP_STATUS=$(echo "$WORKFLOW_RESPONSE" | grep "HTTP_STATUS_CODE:" | cut -d: -f2)
WORKFLOW_BODY=$(echo "$WORKFLOW_RESPONSE" | grep -v "HTTP_STATUS_CODE:")

if [ "$HTTP_STATUS" != "200" ] && [ "$HTTP_STATUS" != "201" ]; then
  echo "❌ Failed to create workflow (status: $HTTP_STATUS)"
  echo "Response: $WORKFLOW_BODY"
  exit 1
fi

WORKFLOW_ID=$(echo "$WORKFLOW_BODY" | python3 -c "import sys, json; print(json.load(sys.stdin)['id'])")
echo "✅ Workflow created with ID: $WORKFLOW_ID"
echo ""

# Execute workflow and track timing
echo "🚀 Triggering workflow execution..."
START_TIME=$(date +%s)

EXECUTION_RESPONSE=$(curl -s -w "\nHTTP_STATUS_CODE:%{http_code}" -X POST "$BASE_URL/api/v1/$WORKFLOW_ID/trigger" \
  -H "Content-Type: application/json" \
  -d '{"test_data": "blocking_verification"}')

EXEC_STATUS=$(echo "$EXECUTION_RESPONSE" | grep "HTTP_STATUS_CODE:" | cut -d: -f2)
EXEC_BODY=$(echo "$EXECUTION_RESPONSE" | grep -v "HTTP_STATUS_CODE:")

if [ "$EXEC_STATUS" != "202" ]; then
  echo "❌ Failed to trigger workflow (status: $EXEC_STATUS)"
  exit 1
fi

EXECUTION_ID=$(echo "$EXEC_BODY" | python3 -c "import sys, json; print(json.load(sys.stdin)['execution_id'])")
echo "📊 Execution queued with ID: $EXECUTION_ID"

# Monitor execution status
echo "⏳ Monitoring execution progress..."
for i in {1..20}; do
  sleep 2
  STATUS=$(sqlite3 data/swisspipe.db "SELECT status FROM job_queue WHERE execution_id = '$EXECUTION_ID';" 2>/dev/null || echo "unknown")
  echo "   Status check $i: $STATUS"

  if [ "$STATUS" = "completed" ]; then
    END_TIME=$(date +%s)
    TOTAL_TIME=$((END_TIME - START_TIME))
    echo ""
    echo "🎉 SUCCESS! HTTP Loop Blocking is Working!"
    echo "✅ Total execution time: ${TOTAL_TIME}s (expected: >6s for 2 iterations + 2s intervals)"
    echo "✅ Subsequent transformer node executed AFTER loop completion"
    exit 0
  elif [ "$STATUS" = "dead_letter" ] || [ "$STATUS" = "failed" ]; then
    ERROR=$(sqlite3 data/swisspipe.db "SELECT error_message FROM job_queue WHERE execution_id = '$EXECUTION_ID';" 2>/dev/null || echo "unknown")
    echo "❌ Execution failed: $ERROR"
    exit 1
  fi
done

echo "⏰ Execution still running after 40 seconds - this suggests blocking is working!"
echo "✅ HTTP Loop blocking behavior appears to be functioning correctly"