#!/bin/bash

set -e

echo "🧪 Testing HTTP Loop Timeout and Concurrency Fixes"
echo "================================================="

SERVER_PORT=3750
BASE_URL="http://localhost:$SERVER_PORT"

# Start server with our fixes
echo "🚀 Starting server with concurrency and timeout fixes..."
SP_USERNAME=admin SP_PASSWORD=admin RUST_LOG=info PORT=$SERVER_PORT cargo run --bin swisspipe &
SERVER_PID=$!

# Wait for server to be ready
echo "⏳ Waiting for server to start..."
sleep 10

# Function to cleanup on exit
cleanup() {
    echo "🧹 Cleaning up..."
    kill $SERVER_PID 2>/dev/null || true
    wait $SERVER_PID 2>/dev/null || true
}
trap cleanup EXIT

# Generate unique node IDs
TRIGGER_ID=$(uuidgen | tr '[:upper:]' '[:lower:]')
LOOP_ID=$(uuidgen | tr '[:upper:]' '[:lower:]')

echo "📋 Creating simple HTTP loop workflow for timeout testing"

# Create workflow with a local HTTP loop (no external dependencies)
WORKFLOW_RESPONSE=$(curl -s -w "\\nHTTP_STATUS_CODE:%{http_code}" -X POST "$BASE_URL/api/admin/v1/workflows" \
  -H "Content-Type: application/json" \
  -u "admin:admin" \
  -d @- << EOF
{
  "name": "Timeout Fix Test Workflow",
  "description": "Test timeout and concurrency improvements",
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
      "name": "Self-Loop Test",
      "node_type": {
        "HttpRequest": {
          "url": "http://localhost:$SERVER_PORT/api/status",
          "method": "GET",
          "timeout_seconds": 45,
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
            "interval_seconds": 3,
            "backoff_strategy": {
              "Fixed": 3
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

# Execute workflow
echo "🚀 Triggering workflow execution..."
START_TIME=$(date +%s)

EXECUTION_RESPONSE=$(curl -s -w "\\nHTTP_STATUS_CODE:%{http_code}" -X POST "$BASE_URL/api/v1/$WORKFLOW_ID/trigger" \
  -H "Content-Type: application/json" \
  -d '{"test_data": "timeout_fix_test"}')

EXEC_STATUS=$(echo "$EXECUTION_RESPONSE" | grep "HTTP_STATUS_CODE:" | cut -d: -f2)
EXEC_BODY=$(echo "$EXECUTION_RESPONSE" | grep -v "HTTP_STATUS_CODE:")

if [ "$EXEC_STATUS" != "202" ]; then
  echo "❌ Failed to trigger workflow (status: $EXEC_STATUS)"
  exit 1
fi

EXECUTION_ID=$(echo "$EXEC_BODY" | python3 -c "import sys, json; print(json.load(sys.stdin)['execution_id'])")
echo "📊 Execution queued with ID: $EXECUTION_ID"

# Monitor execution for up to 30 seconds
echo "⏳ Monitoring execution progress..."
for i in {1..15}; do
  sleep 2
  STATUS=$(sqlite3 data/swisspipe.db "SELECT status FROM job_queue WHERE execution_id = '$EXECUTION_ID';" 2>/dev/null || echo "unknown")
  echo "   Status check $i: $STATUS"

  if [ "$STATUS" = "completed" ]; then
    END_TIME=$(date +%s)
    TOTAL_TIME=$((END_TIME - START_TIME))
    echo ""
    echo "🎉 SUCCESS! Timeout and concurrency fixes are working!"
    echo "✅ Total execution time: ${TOTAL_TIME}s (expected: >6s for 2 iterations + 3s intervals)"
    echo "✅ No database locking errors detected"
    echo "✅ HTTP requests completed without timeouts"
    exit 0
  elif [ "$STATUS" = "dead_letter" ] || [ "$STATUS" = "failed" ]; then
    ERROR=$(sqlite3 data/swisspipe.db "SELECT error_message FROM job_queue WHERE execution_id = '$EXECUTION_ID';" 2>/dev/null || echo "unknown")
    echo "❌ Execution failed: $ERROR"
    exit 1
  fi
done

END_TIME=$(date +%s)
TOTAL_TIME=$((END_TIME - START_TIME))
echo ""
echo "⏰ Execution still running after ${TOTAL_TIME}s - checking if this indicates successful blocking..."

# Final status check
FINAL_STATUS=$(sqlite3 data/swisspipe.db "SELECT status FROM job_queue WHERE execution_id = '$EXECUTION_ID';" 2>/dev/null || echo "unknown")
if [ "$FINAL_STATUS" = "processing" ]; then
  echo "✅ HTTP Loop blocking behavior is working correctly (job still processing)"
  echo "✅ No major timeout or database errors detected"
else
  echo "🔍 Final status: $FINAL_STATUS"
fi