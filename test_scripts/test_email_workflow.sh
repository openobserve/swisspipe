#!/bin/bash

# Test script to reproduce email duplication issue
set -e

BASE_URL="http://localhost:3750"

echo "🧪 Testing email workflow duplication issue..."

# Create a workflow with email node
echo "Creating workflow with email node..."
WORKFLOW_RESPONSE=$(curl -s -X POST "$BASE_URL/api/admin/v1/workflows" \
  -u admin:admin \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Email Test Workflow",
    "description": "Test workflow to reproduce email duplication",
    "nodes": [
      {
        "name": "Trigger",
        "node_type": {
          "Trigger": {
            "http_method": "POST",
            "path": "/email-test"
          }
        }
      },
      {
        "name": "Send Email",
        "node_type": {
          "Email": {
            "config": {
              "smtp_config": "default",
              "to": ["test@example.com"],
              "cc": [],
              "bcc": [],
              "subject": "Test Email - {{workflow.data.subject || \"Default Subject\"}}",
              "text_body": "This is a test email: {{workflow.data.message || \"Default message\"}}",
              "html_body": "<p>This is a test email: <strong>{{workflow.data.message || \"Default message\"}}</strong></p>",
              "priority": "Normal",
              "queue_if_rate_limited": true,
              "max_queue_wait_minutes": 60
            }
          }
        }
      }
    ],
    "edges": [
      {
        "from_node_id": "",
        "to_node_id": "",
        "condition_result": null
      }
    ]
  }')

echo "Workflow creation response: $WORKFLOW_RESPONSE"

# Extract workflow ID and set up edges with proper node IDs
WORKFLOW_ID=$(echo "$WORKFLOW_RESPONSE" | jq -r '.id // empty')
if [ -z "$WORKFLOW_ID" ]; then
  echo "❌ Failed to create workflow"
  exit 1
fi

TRIGGER_NODE_ID=$(echo "$WORKFLOW_RESPONSE" | jq -r '.nodes[0].id // empty')
EMAIL_NODE_ID=$(echo "$WORKFLOW_RESPONSE" | jq -r '.nodes[1].id // empty')

if [ -z "$TRIGGER_NODE_ID" ] || [ -z "$EMAIL_NODE_ID" ]; then
  echo "❌ Failed to extract node IDs"
  exit 1
fi

echo "✅ Workflow created successfully: $WORKFLOW_ID"
echo "   Trigger Node ID: $TRIGGER_NODE_ID"
echo "   Email Node ID: $EMAIL_NODE_ID"

# Update workflow to set proper edges
echo "Updating workflow with proper edges..."
UPDATE_RESPONSE=$(curl -s -X PUT "$BASE_URL/api/admin/v1/workflows/$WORKFLOW_ID" \
  -u admin:admin \
  -H "Content-Type: application/json" \
  -d "{
    \"name\": \"Email Test Workflow\",
    \"description\": \"Test workflow to reproduce email duplication\",
    \"nodes\": [
      {
        \"id\": \"$TRIGGER_NODE_ID\",
        \"name\": \"Trigger\",
        \"node_type\": {
          \"Trigger\": {
            \"http_method\": \"POST\",
            \"path\": \"/email-test\"
          }
        }
      },
      {
        \"id\": \"$EMAIL_NODE_ID\",
        \"name\": \"Send Email\",
        \"node_type\": {
          \"Email\": {
            \"config\": {
              \"smtp_config\": \"default\",
              \"to\": [\"test@example.com\"],
              \"cc\": [],
              \"bcc\": [],
              \"subject\": \"Test Email - {{workflow.data.subject || \\\"Default Subject\\\"}}\",
              \"text_body\": \"This is a test email: {{workflow.data.message || \\\"Default message\\\"}}\",
              \"html_body\": \"<p>This is a test email: <strong>{{workflow.data.message || \\\"Default message\\\"}}</strong></p>\",
              \"priority\": \"Normal\",
              \"queue_if_rate_limited\": true,
              \"max_queue_wait_minutes\": 60
            }
          }
        }
      }
    ],
    \"edges\": [
      {
        \"from_node_id\": \"$TRIGGER_NODE_ID\",
        \"to_node_id\": \"$EMAIL_NODE_ID\",
        \"condition_result\": null
      }
    ]
  }")

echo "Update response: $UPDATE_RESPONSE"

# Test the workflow by triggering it
echo "🚀 Triggering workflow execution..."
for i in {1..2}; do
  echo "Execution $i..."
  EXECUTION_RESPONSE=$(curl -s -X POST "$BASE_URL/api/v1/$WORKFLOW_ID/trigger" \
    -H "Content-Type: application/json" \
    -d '{
      "subject": "Test Email #'$i'",
      "message": "This is test execution number '$i'"
    }')

  echo "Execution $i response: $EXECUTION_RESPONSE"

  # Wait a moment between executions
  sleep 2
done

echo "🔍 Checking email audit log for duplicates..."
sqlite3 data/swisspipe.db "SELECT execution_id, node_id, to_emails, subject, status, sent_at FROM email_audit_log ORDER BY sent_at;"

echo "🔍 Checking email queue for duplicates..."
sqlite3 data/swisspipe.db "SELECT id, execution_id, node_id, status, queued_at, processed_at, sent_at FROM email_queue ORDER BY queued_at;"

echo "✅ Test completed!"