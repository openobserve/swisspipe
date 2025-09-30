#!/bin/bash

# Test script to verify custom start node functionality

echo "Testing custom start node workflow creation..."

# Start the server in background
echo "Starting swisspipe server..."
RUST_LOG=info PORT=3800 cargo run --bin swisspipe &
SERVER_PID=$!

# Wait for server to start
sleep 3

# Test 1: Create workflow with custom start node (frontend controls everything)
echo "Test 1: Creating workflow with frontend-provided start node..."
curl -X POST http://localhost:3800/api/workflows \
  -H "Content-Type: application/json" \
  -H "Authorization: Basic YWRtaW46YWRtaW4=" \
  -d '{
    "name": "Frontend Controlled Workflow",
    "description": "Frontend provides UUIDs for all nodes including start node",
    "start_node_id": "custom-trigger-123",
    "nodes": [
      {
        "id": "custom-trigger-123",
        "name": "API Webhook",
        "node_type": {
          "Trigger": {
            "methods": ["Post"]
          }
        },
        "position_x": 100,
        "position_y": 50
      },
      {
        "id": "transform-node-456",
        "name": "Process Data",
        "node_type": {
          "Transformer": {
            "script": "function transformer(event) { return {...event, data: {...event.data, processed: true, timestamp: new Date().toISOString()}}; }"
          }
        },
        "position_x": 300,
        "position_y": 100
      }
    ],
    "edges": [
      {
        "from_node_id": "custom-trigger-123",
        "to_node_id": "transform-node-456"
      }
    ]
  }' | jq '.'

echo -e "\n"

# Test 2: Create workflow without start_node_id (backward compatibility)
echo "Test 2: Creating workflow with auto-generated start node (backward compatibility)..."
curl -X POST http://localhost:3800/api/workflows \
  -H "Content-Type: application/json" \
  -H "Authorization: Basic YWRtaW46YWRtaW4=" \
  -d '{
    "name": "Auto Start Node Test",
    "description": "Testing backward compatibility",
    "nodes": [
      {
        "id": "transform-node-789",
        "name": "Transform Data",
        "node_type": {
          "Transformer": {
            "script": "function transformer(event) { return {...event, data: {...event.data, auto_start: true}}; }"
          }
        },
        "position_x": 300,
        "position_y": 100
      }
    ],
    "edges": []
  }' | jq '.'

echo -e "\n"

# Test 3: Create workflow with existing trigger node (no start_node_id specified)
echo "Test 3: Creating workflow with existing trigger node (auto-detection)..."
curl -X POST http://localhost:3800/api/workflows \
  -H "Content-Type: application/json" \
  -H "Authorization: Basic YWRtaW46YWRtaW4=" \
  -d '{
    "name": "Auto Detect Start Node Test",
    "description": "Testing auto-detection of trigger node",
    "nodes": [
      {
        "id": "existing-trigger-999",
        "name": "Existing Trigger",
        "node_type": {
          "Trigger": {
            "methods": ["Post"]
          }
        },
        "position_x": 150,
        "position_y": 75
      },
      {
        "id": "condition-node-888",
        "name": "Check Condition",
        "node_type": {
          "Condition": {
            "script": "function condition(event) { return event.data.active === true; }"
          }
        },
        "position_x": 300,
        "position_y": 150
      }
    ],
    "edges": [
      {
        "from_node_id": "existing-trigger-999",
        "to_node_id": "condition-node-888"
      }
    ]
  }' | jq '.'

# Clean up - kill the server
echo -e "\nStopping server..."
kill $SERVER_PID
wait $SERVER_PID 2>/dev/null

echo "Test completed!"