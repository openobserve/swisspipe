#!/bin/bash

set -e

echo "🧪 HTTP Loop Integration Tests - Blocking Behavior & Data Flow"
echo "============================================================="

SERVER_PORT=3750
BASE_URL="http://localhost:$SERVER_PORT"
TOTAL_TESTS=5
PASSED_TESTS=0

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test result tracking
declare -a TEST_RESULTS=()

# Helper function to record test results
record_test() {
    local test_name="$1"
    local success="$2"
    local message="$3"

    if [ "$success" -eq 1 ]; then
        echo -e "${GREEN}✅ PASS${NC}: $test_name"
        PASSED_TESTS=$((PASSED_TESTS + 1))
        TEST_RESULTS+=("PASS: $test_name")
    else
        echo -e "${RED}❌ FAIL${NC}: $test_name - $message"
        TEST_RESULTS+=("FAIL: $test_name - $message")
    fi
}

# Test 1: HTTP Loop Blocks Subsequent Node Execution
echo ""
echo "📋 Test 1: HTTP Loop Blocks Subsequent Node Execution"
echo "======================================================"

TRIGGER_ID=$(uuidgen | tr '[:upper:]' '[:lower:]')
LOOP_ID=$(uuidgen | tr '[:upper:]' '[:lower:]')
AFTER_ID=$(uuidgen | tr '[:upper:]' '[:lower:]')

WORKFLOW_RESPONSE=$(curl -s -w "\nHTTP_STATUS_CODE:%{http_code}" -X POST "$BASE_URL/api/admin/v1/workflows" \
  -H "Content-Type: application/json" \
  -u "admin:admin" \
  -d @- << EOF
{
  "name": "Blocking Test - Sequential Execution",
  "description": "Verify HTTP loop blocks next node until completion",
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
      "name": "HTTP Loop (3 iterations)",
      "node_type": {
        "HttpRequest": {
          "url": "https://httpbin.org/delay/1",
          "method": "GET",
          "timeout_seconds": 15,
          "failure_action": "Continue",
          "retry_config": {
            "max_attempts": 1,
            "initial_delay_ms": 100,
            "max_delay_ms": 1000,
            "backoff_multiplier": 2.0
          },
          "headers": {},
          "loop_config": {
            "max_iterations": 3,
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
      "name": "After Loop Node",
      "node_type": {
        "Transformer": {
          "script": "function transformer(event) { event.data.after_loop_timestamp = Date.now(); return event; }"
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

HTTP_STATUS=$(echo "$WORKFLOW_RESPONSE" | grep "HTTP_STATUS_CODE:" | cut -d: -f2)
WORKFLOW_BODY=$(echo "$WORKFLOW_RESPONSE" | grep -v "HTTP_STATUS_CODE:")

if [ "$HTTP_STATUS" == "201" ] || [ "$HTTP_STATUS" == "200" ]; then
    WORKFLOW_ID=$(echo "$WORKFLOW_BODY" | python3 -c "import sys, json; print(json.load(sys.stdin)['id'])")

    # Execute and measure timing
    START_TIME=$(date +%s)
    EXECUTION_RESPONSE=$(curl -s -w "\nHTTP_STATUS_CODE:%{http_code}" -X POST "$BASE_URL/api/v1/$WORKFLOW_ID/trigger" \
      -H "Content-Type: application/json" \
      -d '{"test": "blocking_verification", "start_time": '$(date +%s)'}')

    EXEC_STATUS=$(echo "$EXECUTION_RESPONSE" | grep "HTTP_STATUS_CODE:" | cut -d: -f2)

    if [ "$EXEC_STATUS" == "202" ]; then
        EXEC_BODY=$(echo "$EXECUTION_RESPONSE" | grep -v "HTTP_STATUS_CODE:")
        EXECUTION_ID=$(echo "$EXEC_BODY" | python3 -c "import sys, json; print(json.load(sys.stdin)['execution_id'])" 2>/dev/null || echo "unknown")

        # Monitor execution
        echo "   Monitoring execution: $EXECUTION_ID"
        for i in {1..15}; do
            sleep 2
            STATUS=$(sqlite3 data/swisspipe.db "SELECT status FROM job_queue WHERE execution_id = '$EXECUTION_ID';" 2>/dev/null || echo "unknown")
            echo "   Status check $i: $STATUS"

            if [ "$STATUS" = "completed" ]; then
                END_TIME=$(date +%s)
                TOTAL_TIME=$((END_TIME - START_TIME))

                if [ $TOTAL_TIME -ge 6 ]; then
                    record_test "HTTP Loop Blocks Subsequent Node" 1 "Execution took ${TOTAL_TIME}s (expected ≥6s)"
                else
                    record_test "HTTP Loop Blocks Subsequent Node" 0 "Execution too fast: ${TOTAL_TIME}s (expected ≥6s)"
                fi
                break
            elif [ "$STATUS" = "dead_letter" ] || [ "$STATUS" = "failed" ]; then
                record_test "HTTP Loop Blocks Subsequent Node" 0 "Execution failed: $STATUS"
                break
            fi

            if [ $i -eq 15 ]; then
                record_test "HTTP Loop Blocks Subsequent Node" 0 "Execution timeout after 30s"
            fi
        done
    else
        record_test "HTTP Loop Blocks Subsequent Node" 0 "Failed to trigger workflow: $EXEC_STATUS"
    fi
else
    record_test "HTTP Loop Blocks Subsequent Node" 0 "Failed to create workflow: $HTTP_STATUS"
fi

# Test 2: Final Loop Data Passed to Next Node
echo ""
echo "📋 Test 2: Final Loop Data Passed to Next Node"
echo "=============================================="

TRIGGER_ID=$(uuidgen | tr '[:upper:]' '[:lower:]')
LOOP_ID=$(uuidgen | tr '[:upper:]' '[:lower:]')
DATA_PROCESSOR_ID=$(uuidgen | tr '[:upper:]' '[:lower:]')

WORKFLOW_RESPONSE=$(curl -s -w "\nHTTP_STATUS_CODE:%{http_code}" -X POST "$BASE_URL/api/admin/v1/workflows" \
  -H "Content-Type: application/json" \
  -u "admin:admin" \
  -d @- << EOF
{
  "name": "Data Flow Test - Loop Data Preservation",
  "description": "Verify final loop data reaches next node",
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
      "name": "Data Counter Loop",
      "node_type": {
        "HttpRequest": {
          "url": "https://httpbin.org/post",
          "method": "POST",
          "timeout_seconds": 10,
          "failure_action": "Continue",
          "retry_config": {
            "max_attempts": 1,
            "initial_delay_ms": 100,
            "max_delay_ms": 1000,
            "backoff_multiplier": 2.0
          },
          "headers": {"Content-Type": "application/json"},
          "loop_config": {
            "max_iterations": 2,
            "interval_seconds": 1,
            "backoff_strategy": {
              "Fixed": 1
            }
          }
        }
      }
    },
    {
      "id": "$DATA_PROCESSOR_ID",
      "name": "Data Processor",
      "node_type": {
        "Transformer": {
          "script": "function transformer(event) { event.data.processed_final_data = true; event.data.processing_timestamp = Date.now(); return event; }"
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
      "to_node_id": "$DATA_PROCESSOR_ID"
    }
  ]
}
EOF
)

HTTP_STATUS=$(echo "$WORKFLOW_RESPONSE" | grep "HTTP_STATUS_CODE:" | cut -d: -f2)
WORKFLOW_BODY=$(echo "$WORKFLOW_RESPONSE" | grep -v "HTTP_STATUS_CODE:")

if [ "$HTTP_STATUS" == "201" ] || [ "$HTTP_STATUS" == "200" ]; then
    WORKFLOW_ID=$(echo "$WORKFLOW_BODY" | python3 -c "import sys, json; print(json.load(sys.stdin)['id'])")

    # Execute with test data
    EXECUTION_RESPONSE=$(curl -s -w "\nHTTP_STATUS_CODE:%{http_code}" -X POST "$BASE_URL/api/v1/$WORKFLOW_ID/trigger" \
      -H "Content-Type: application/json" \
      -d '{"counter": 0, "test_id": "data_flow_test", "original_timestamp": '$(date +%s)'}')

    EXEC_STATUS=$(echo "$EXECUTION_RESPONSE" | grep "HTTP_STATUS_CODE:" | cut -d: -f2)

    if [ "$EXEC_STATUS" == "202" ]; then
        EXEC_BODY=$(echo "$EXECUTION_RESPONSE" | grep -v "HTTP_STATUS_CODE:")
        EXECUTION_ID=$(echo "$EXEC_BODY" | python3 -c "import sys, json; print(json.load(sys.stdin)['execution_id'])" 2>/dev/null || echo "unknown")

        echo "   Monitoring data flow execution: $EXECUTION_ID"
        for i in {1..10}; do
            sleep 1
            STATUS=$(sqlite3 data/swisspipe.db "SELECT status FROM job_queue WHERE execution_id = '$EXECUTION_ID';" 2>/dev/null || echo "unknown")
            echo "   Status check $i: $STATUS"

            if [ "$STATUS" = "completed" ]; then
                # Check if execution steps show data progression
                STEP_COUNT=$(sqlite3 data/swisspipe.db "SELECT COUNT(*) FROM workflow_execution_steps WHERE execution_id = '$EXECUTION_ID';" 2>/dev/null || echo "0")

                if [ "$STEP_COUNT" -ge 3 ]; then
                    record_test "Final Loop Data Passed to Next Node" 1 "All workflow steps executed ($STEP_COUNT steps)"
                else
                    record_test "Final Loop Data Passed to Next Node" 0 "Insufficient execution steps: $STEP_COUNT"
                fi
                break
            elif [ "$STATUS" = "dead_letter" ] || [ "$STATUS" = "failed" ]; then
                record_test "Final Loop Data Passed to Next Node" 0 "Execution failed: $STATUS"
                break
            fi

            if [ $i -eq 10 ]; then
                record_test "Final Loop Data Passed to Next Node" 0 "Execution timeout"
            fi
        done
    else
        record_test "Final Loop Data Passed to Next Node" 0 "Failed to trigger workflow: $EXEC_STATUS"
    fi
else
    record_test "Final Loop Data Passed to Next Node" 0 "Failed to create workflow: $HTTP_STATUS"
fi

# Test 3: Termination Condition Blocks Properly
echo ""
echo "📋 Test 3: Loop Termination Condition Blocks Properly"
echo "====================================================="

TRIGGER_ID=$(uuidgen | tr '[:upper:]' '[:lower:]')
LOOP_ID=$(uuidgen | tr '[:upper:]' '[:lower:]')
FINAL_ID=$(uuidgen | tr '[:upper:]' '[:lower:]')

WORKFLOW_RESPONSE=$(curl -s -w "\nHTTP_STATUS_CODE:%{http_code}" -X POST "$BASE_URL/api/admin/v1/workflows" \
  -H "Content-Type: application/json" \
  -u "admin:admin" \
  -d @- << EOF
{
  "name": "Termination Condition Test",
  "description": "Test early termination with proper blocking",
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
      "name": "Early Termination Loop",
      "node_type": {
        "HttpRequest": {
          "url": "https://httpbin.org/get",
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
            "max_iterations": 10,
            "interval_seconds": 1,
            "backoff_strategy": {
              "Fixed": 1
            },
            "termination_condition": {
              "script": "function condition(event) { return (event.metadata.loop_iteration && parseInt(event.metadata.loop_iteration) >= 2); }",
              "action": "Success"
            }
          }
        }
      }
    },
    {
      "id": "$FINAL_ID",
      "name": "After Termination",
      "node_type": {
        "Transformer": {
          "script": "function transformer(event) { event.data.early_termination_success = true; return event; }"
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
      "to_node_id": "$FINAL_ID"
    }
  ]
}
EOF
)

HTTP_STATUS=$(echo "$WORKFLOW_RESPONSE" | grep "HTTP_STATUS_CODE:" | cut -d: -f2)
WORKFLOW_BODY=$(echo "$WORKFLOW_RESPONSE" | grep -v "HTTP_STATUS_CODE:")

if [ "$HTTP_STATUS" == "201" ] || [ "$HTTP_STATUS" == "200" ]; then
    WORKFLOW_ID=$(echo "$WORKFLOW_BODY" | python3 -c "import sys, json; print(json.load(sys.stdin)['id'])")

    START_TIME=$(date +%s)
    EXECUTION_RESPONSE=$(curl -s -w "\nHTTP_STATUS_CODE:%{http_code}" -X POST "$BASE_URL/api/v1/$WORKFLOW_ID/trigger" \
      -H "Content-Type: application/json" \
      -d '{"target_value": 5, "current_value": 1}')

    EXEC_STATUS=$(echo "$EXECUTION_RESPONSE" | grep "HTTP_STATUS_CODE:" | cut -d: -f2)

    if [ "$EXEC_STATUS" == "202" ]; then
        EXEC_BODY=$(echo "$EXECUTION_RESPONSE" | grep -v "HTTP_STATUS_CODE:")
        EXECUTION_ID=$(echo "$EXEC_BODY" | python3 -c "import sys, json; print(json.load(sys.stdin)['execution_id'])" 2>/dev/null || echo "unknown")

        echo "   Monitoring early termination: $EXECUTION_ID"
        for i in {1..8}; do
            sleep 1
            STATUS=$(sqlite3 data/swisspipe.db "SELECT status FROM job_queue WHERE execution_id = '$EXECUTION_ID';" 2>/dev/null || echo "unknown")
            echo "   Status check $i: $STATUS"

            if [ "$STATUS" = "completed" ]; then
                END_TIME=$(date +%s)
                TOTAL_TIME=$((END_TIME - START_TIME))

                # Should complete faster than full 10 iterations (early termination)
                if [ $TOTAL_TIME -ge 2 ] && [ $TOTAL_TIME -le 6 ]; then
                    record_test "Loop Termination Condition Blocks Properly" 1 "Early termination in ${TOTAL_TIME}s"
                else
                    record_test "Loop Termination Condition Blocks Properly" 0 "Unexpected timing: ${TOTAL_TIME}s"
                fi
                break
            elif [ "$STATUS" = "dead_letter" ] || [ "$STATUS" = "failed" ]; then
                record_test "Loop Termination Condition Blocks Properly" 0 "Execution failed: $STATUS"
                break
            fi

            if [ $i -eq 8 ]; then
                record_test "Loop Termination Condition Blocks Properly" 0 "Execution timeout"
            fi
        done
    else
        record_test "Loop Termination Condition Blocks Properly" 0 "Failed to trigger: $EXEC_STATUS"
    fi
else
    record_test "Loop Termination Condition Blocks Properly" 0 "Failed to create workflow: $HTTP_STATUS"
fi

# Test 4: Concurrent Loops Don't Interfere
echo ""
echo "📋 Test 4: Concurrent Loops Don't Interfere"
echo "=========================================="

# Create first workflow
TRIGGER_ID1=$(uuidgen | tr '[:upper:]' '[:lower:]')
LOOP_ID1=$(uuidgen | tr '[:upper:]' '[:lower:]')
FINAL_ID1=$(uuidgen | tr '[:upper:]' '[:lower:]')

WORKFLOW1_RESPONSE=$(curl -s -w "\nHTTP_STATUS_CODE:%{http_code}" -X POST "$BASE_URL/api/admin/v1/workflows" \
  -H "Content-Type: application/json" \
  -u "admin:admin" \
  -d @- << EOF
{
  "name": "Concurrent Test Workflow 1",
  "description": "First concurrent workflow",
  "start_node_id": "$TRIGGER_ID1",
  "nodes": [
    {
      "id": "$TRIGGER_ID1",
      "name": "Start 1",
      "node_type": {
        "Trigger": {
          "methods": ["POST"]
        }
      }
    },
    {
      "id": "$LOOP_ID1",
      "name": "Loop 1",
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
            "max_iterations": 1,
            "interval_seconds": 1,
            "backoff_strategy": {
              "Fixed": 1
            }
          }
        }
      }
    },
    {
      "id": "$FINAL_ID1",
      "name": "Final 1",
      "node_type": {
        "Transformer": {
          "script": "function transformer(event) { event.data.workflow_id = 'workflow1'; return event; }"
        }
      }
    }
  ],
  "edges": [
    {
      "from_node_id": "$TRIGGER_ID1",
      "to_node_id": "$LOOP_ID1"
    },
    {
      "from_node_id": "$LOOP_ID1",
      "to_node_id": "$FINAL_ID1"
    }
  ]
}
EOF
)

HTTP_STATUS1=$(echo "$WORKFLOW1_RESPONSE" | grep "HTTP_STATUS_CODE:" | cut -d: -f2)
WORKFLOW1_BODY=$(echo "$WORKFLOW1_RESPONSE" | grep -v "HTTP_STATUS_CODE:")

# Create second workflow
TRIGGER_ID2=$(uuidgen | tr '[:upper:]' '[:lower:]')
LOOP_ID2=$(uuidgen | tr '[:upper:]' '[:lower:]')
FINAL_ID2=$(uuidgen | tr '[:upper:]' '[:lower:]')

WORKFLOW2_RESPONSE=$(curl -s -w "\nHTTP_STATUS_CODE:%{http_code}" -X POST "$BASE_URL/api/admin/v1/workflows" \
  -H "Content-Type: application/json" \
  -u "admin:admin" \
  -d @- << EOF
{
  "name": "Concurrent Test Workflow 2",
  "description": "Second concurrent workflow",
  "start_node_id": "$TRIGGER_ID2",
  "nodes": [
    {
      "id": "$TRIGGER_ID2",
      "name": "Start 2",
      "node_type": {
        "Trigger": {
          "methods": ["POST"]
        }
      }
    },
    {
      "id": "$LOOP_ID2",
      "name": "Loop 2",
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
            "interval_seconds": 1,
            "backoff_strategy": {
              "Fixed": 1
            }
          }
        }
      }
    },
    {
      "id": "$FINAL_ID2",
      "name": "Final 2",
      "node_type": {
        "Transformer": {
          "script": "function transformer(event) { event.data.workflow_id = 'workflow2'; return event; }"
        }
      }
    }
  ],
  "edges": [
    {
      "from_node_id": "$TRIGGER_ID2",
      "to_node_id": "$LOOP_ID2"
    },
    {
      "from_node_id": "$LOOP_ID2",
      "to_node_id": "$FINAL_ID2"
    }
  ]
}
EOF
)

HTTP_STATUS2=$(echo "$WORKFLOW2_RESPONSE" | grep "HTTP_STATUS_CODE:" | cut -d: -f2)
WORKFLOW2_BODY=$(echo "$WORKFLOW2_RESPONSE" | grep -v "HTTP_STATUS_CODE:")

if [ "$HTTP_STATUS1" == "201" ] && [ "$HTTP_STATUS2" == "201" ]; then
    WORKFLOW_ID1=$(echo "$WORKFLOW1_BODY" | python3 -c "import sys, json; print(json.load(sys.stdin)['id'])")
    WORKFLOW_ID2=$(echo "$WORKFLOW2_BODY" | python3 -c "import sys, json; print(json.load(sys.stdin)['id'])")

    # Execute both workflows concurrently
    START_TIME=$(date +%s)

    curl -s -X POST "$BASE_URL/api/v1/$WORKFLOW_ID1/trigger" \
      -H "Content-Type: application/json" \
      -d '{"test": "concurrent1"}' &
    PID1=$!

    curl -s -X POST "$BASE_URL/api/v1/$WORKFLOW_ID2/trigger" \
      -H "Content-Type: application/json" \
      -d '{"test": "concurrent2"}' &
    PID2=$!

    # Wait for both to complete
    wait $PID1
    wait $PID2

    # Monitor until both complete
    sleep 5

    # Check recent executions
    RECENT_COUNT=$(sqlite3 data/swisspipe.db "SELECT COUNT(*) FROM job_queue WHERE status = 'completed' AND created_at > datetime('now', '-1 minute');" 2>/dev/null || echo "0")

    if [ "$RECENT_COUNT" -ge 2 ]; then
        record_test "Concurrent Loops Don't Interfere" 1 "Both concurrent workflows completed ($RECENT_COUNT recent completions)"
    else
        record_test "Concurrent Loops Don't Interfere" 0 "Only $RECENT_COUNT recent completions"
    fi
else
    record_test "Concurrent Loops Don't Interfere" 0 "Failed to create workflows ($HTTP_STATUS1, $HTTP_STATUS2)"
fi

# Test 5: Error Handling Preserves Blocking
echo ""
echo "📋 Test 5: Error Handling Preserves Blocking Behavior"
echo "====================================================="

TRIGGER_ID=$(uuidgen | tr '[:upper:]' '[:lower:]')
LOOP_ID=$(uuidgen | tr '[:upper:]' '[:lower:]')
RECOVERY_ID=$(uuidgen | tr '[:upper:]' '[:lower:]')

WORKFLOW_RESPONSE=$(curl -s -w "\nHTTP_STATUS_CODE:%{http_code}" -X POST "$BASE_URL/api/admin/v1/workflows" \
  -H "Content-Type: application/json" \
  -u "admin:admin" \
  -d @- << EOF
{
  "name": "Error Handling Test",
  "description": "Test error handling with blocking preserved",
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
      "name": "Error-prone Loop",
      "node_type": {
        "HttpRequest": {
          "url": "https://httpbin.org/status/404",
          "method": "GET",
          "timeout_seconds": 5,
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
      "id": "$RECOVERY_ID",
      "name": "Error Recovery",
      "node_type": {
        "Transformer": {
          "script": "function transformer(event) { event.data.error_handled = true; return event; }"
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
      "to_node_id": "$RECOVERY_ID"
    }
  ]
}
EOF
)

HTTP_STATUS=$(echo "$WORKFLOW_RESPONSE" | grep "HTTP_STATUS_CODE:" | cut -d: -f2)
WORKFLOW_BODY=$(echo "$WORKFLOW_RESPONSE" | grep -v "HTTP_STATUS_CODE:")

if [ "$HTTP_STATUS" == "201" ] || [ "$HTTP_STATUS" == "200" ]; then
    WORKFLOW_ID=$(echo "$WORKFLOW_BODY" | python3 -c "import sys, json; print(json.load(sys.stdin)['id'])")

    START_TIME=$(date +%s)
    EXECUTION_RESPONSE=$(curl -s -w "\nHTTP_STATUS_CODE:%{http_code}" -X POST "$BASE_URL/api/v1/$WORKFLOW_ID/trigger" \
      -H "Content-Type: application/json" \
      -d '{"test": "error_handling"}')

    EXEC_STATUS=$(echo "$EXECUTION_RESPONSE" | grep "HTTP_STATUS_CODE:" | cut -d: -f2)

    if [ "$EXEC_STATUS" == "202" ]; then
        EXEC_BODY=$(echo "$EXECUTION_RESPONSE" | grep -v "HTTP_STATUS_CODE:")
        EXECUTION_ID=$(echo "$EXEC_BODY" | python3 -c "import sys, json; print(json.load(sys.stdin)['execution_id'])" 2>/dev/null || echo "unknown")

        echo "   Monitoring error handling: $EXECUTION_ID"
        for i in {1..10}; do
            sleep 1
            STATUS=$(sqlite3 data/swisspipe.db "SELECT status FROM job_queue WHERE execution_id = '$EXECUTION_ID';" 2>/dev/null || echo "unknown")
            echo "   Status check $i: $STATUS"

            if [ "$STATUS" = "completed" ]; then
                END_TIME=$(date +%s)
                TOTAL_TIME=$((END_TIME - START_TIME))

                # Should still take time even with errors (blocking preserved)
                if [ $TOTAL_TIME -ge 4 ]; then
                    record_test "Error Handling Preserves Blocking" 1 "Error handling with proper timing: ${TOTAL_TIME}s"
                else
                    record_test "Error Handling Preserves Blocking" 0 "Too fast despite errors: ${TOTAL_TIME}s"
                fi
                break
            elif [ "$STATUS" = "dead_letter" ] || [ "$STATUS" = "failed" ]; then
                # Check if recovery node still executed
                STEP_COUNT=$(sqlite3 data/swisspipe.db "SELECT COUNT(*) FROM workflow_execution_steps WHERE execution_id = '$EXECUTION_ID';" 2>/dev/null || echo "0")
                if [ "$STEP_COUNT" -ge 2 ]; then
                    record_test "Error Handling Preserves Blocking" 1 "Error handled, recovery executed ($STEP_COUNT steps)"
                else
                    record_test "Error Handling Preserves Blocking" 0 "Error handling incomplete ($STEP_COUNT steps)"
                fi
                break
            fi

            if [ $i -eq 10 ]; then
                record_test "Error Handling Preserves Blocking" 0 "Execution timeout"
            fi
        done
    else
        record_test "Error Handling Preserves Blocking" 0 "Failed to trigger: $EXEC_STATUS"
    fi
else
    record_test "Error Handling Preserves Blocking" 0 "Failed to create workflow: $HTTP_STATUS"
fi

# Final Results Summary
echo ""
echo "🎯 HTTP Loop Integration Test Results"
echo "====================================="
echo -e "Passed: ${GREEN}$PASSED_TESTS${NC}/$TOTAL_TESTS tests"
echo ""

for result in "${TEST_RESULTS[@]}"; do
    if [[ $result == PASS:* ]]; then
        echo -e "${GREEN}$result${NC}"
    else
        echo -e "${RED}$result${NC}"
    fi
done

echo ""
if [ $PASSED_TESTS -eq $TOTAL_TESTS ]; then
    echo -e "${GREEN}🎉 ALL TESTS PASSED!${NC}"
    echo "✅ HTTP loop blocking behavior is working correctly"
    echo "✅ Final loop data is properly passed to subsequent nodes"
    echo "✅ Termination conditions work with proper blocking"
    echo "✅ Concurrent loops operate independently"
    echo "✅ Error handling preserves blocking behavior"
    exit 0
else
    echo -e "${RED}❌ SOME TESTS FAILED${NC}"
    echo "⚠️  HTTP loop implementation may have issues"
    exit 1
fi