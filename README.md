# SwissPipe Workflow Engine

A high-performance workflow engine built with Rust and Axum that processes data through configurable DAG-based workflows.

## Features

- **DAG-based Workflows**: Define complex data processing flows using directed acyclic graphs
- **JavaScript Integration**: Use JavaScript for transformers and conditions via QuickJS
- **Multiple Node Types**: Support for HTTP requests, email, OpenObserve, delays, and more
- **Async Execution**: Background job processing with worker pools and queue management
- **Retry Logic**: Configurable exponential backoff for failed operations
- **Comprehensive APIs**: Complete workflow management and execution monitoring
- **High Performance**: Built with Rust and Axum for optimal performance
- **Single Binary**: Self-contained deployment with SQLite database
- **Email System**: SMTP integration with templating and queue management
- **Delay Scheduling**: Built-in delay nodes with resumption capabilities

## Quick Start

### Prerequisites

- Rust 1.70+
- SQLite (embedded)

### Installation

1. Clone the repository:
   ```bash
   git clone <repository-url>
   cd swisspipe
   ```

2. Copy environment configuration:
   ```bash
   cp .env.example .env
   ```

3. Build and run:
   ```bash
   cargo run
   ```

The server will start on `http://localhost:3700`

## Configuration

Environment variables:

- `SP_USERNAME`: Admin username for API access (default: admin)
- `SP_PASSWORD`: Admin password for API access (default: admin)  
- `DATABASE_URL`: SQLite database URL (default: sqlite:data/swisspipe.db?mode=rwc)
- `PORT`: Server port (default: 3700)

## API Endpoints

All API endpoints support JSON request/response format. Admin endpoints require Basic Auth (username/password), while workflow execution endpoints use UUID-based authentication.

### Workflow Management APIs (Admin Auth Required)

#### Workflow CRUD Operations
- **GET** `/api/admin/v1/workflows` - List all workflows
- **POST** `/api/admin/v1/workflows` - Create a new workflow  
- **GET** `/api/admin/v1/workflows/{id}` - Get specific workflow details
- **PUT** `/api/admin/v1/workflows/{id}` - Update existing workflow
- **DELETE** `/api/admin/v1/workflows/{id}` - Delete workflow

#### Execution Management APIs
- **GET** `/api/admin/v1/executions` - Get all executions with optional filters
  - Query parameters: `limit`, `offset`, `workflow_id`, `status`
- **GET** `/api/admin/v1/executions/{execution_id}` - Get execution details
- **GET** `/api/admin/v1/executions/{execution_id}/status` - Get execution status (lightweight)
- **GET** `/api/admin/v1/executions/{execution_id}/steps` - Get execution steps
- **GET** `/api/admin/v1/executions/{execution_id}/logs` - Get execution logs
- **POST** `/api/admin/v1/executions/{execution_id}/cancel` - Cancel execution
- **GET** `/api/admin/v1/executions/stats` - Get worker pool statistics

### Workflow Execution APIs (UUID-based Auth)

#### Trigger Workflow Execution
- **GET** `/api/v1/{workflow_id}/trigger` - Trigger workflow with query parameters
- **POST** `/api/v1/{workflow_id}/trigger` - Trigger workflow with JSON body
- **PUT** `/api/v1/{workflow_id}/trigger` - Trigger workflow with JSON body (alternative)
- **POST** `/api/v1/{workflow_id}/json_array` - Trigger workflow with JSON array

All execution endpoints return HTTP 202 (Accepted) with execution details:
```json
{
  "status": "accepted",
  "execution_id": "uuid",
  "message": "Workflow execution has been queued"
}
```

## Workflow Structure

### Node Types

1. **Trigger**: Entry point for HTTP requests (GET/POST/PUT methods)
2. **Condition**: JavaScript-based decision points for flow control
3. **Transformer**: JavaScript-based data modification and filtering
4. **HttpRequest**: HTTP requests to external endpoints (replaces Webhook)
5. **OpenObserve**: Log ingestion to OpenObserve platform
6. **Email**: Send emails via SMTP with templating support
7. **Delay**: Schedule workflow execution delays with resumption capability

### JavaScript Functions

#### Transformers
```javascript
function transformer(event) {
   // Process the event data
   event.data.processed = true;
   return event; // Return null to drop event
}
```

#### Conditions
```javascript
function condition(event) {
   // Evaluate condition
   return event.data.value > 100;
}
```

## Example Usage

### Create a Simple Workflow

```bash
curl -X POST http://localhost:3700/api/admin/v1/workflows \
  -H "Content-Type: application/json" \
  -H "Authorization: Basic YWRtaW46YWRtaW4=" \
  -d '{
    "name": "Simple HTTP Request Flow",
    "description": "Transform data and send to external endpoint",
    "nodes": [
      {
        "id": "transform-node-id", 
        "name": "transform",
        "node_type": {
          "Transformer": {
            "script": "function transformer(event) { event.data.timestamp = Date.now(); return event; }"
          }
        },
        "position_x": 300,
        "position_y": 100
      },
      {
        "id": "http-node-id",
        "name": "webhook",
        "node_type": {
          "HttpRequest": {
            "url": "https://webhook.site/your-unique-url",
            "method": "Post",
            "timeout_seconds": 30,
            "failure_action": "Stop",
            "retry_config": {
              "max_attempts": 3,
              "initial_delay_ms": 100,
              "max_delay_ms": 5000,
              "backoff_multiplier": 2.0
            },
            "headers": {}
          }
        },
        "position_x": 500,
        "position_y": 100
      }
    ],
    "edges": [
      {
        "from_node_id": "{START_NODE_ID}",
        "to_node_id": "transform-node-id",
        "condition_result": null
      },
      {
        "from_node_id": "transform-node-id", 
        "to_node_id": "http-node-id",
        "condition_result": null
      }
    ]
  }'
```

**Note**: The start/trigger node is automatically created by the system. You only need to define your business logic nodes. The start node ID is auto-generated and returned in the response. Use the returned `start_node_id` in your edges to connect from the trigger node to your first business logic node.

### Trigger the Workflow

```bash
curl -X POST http://localhost:3700/api/v1/{workflow-id}/trigger \
  -H "Content-Type: application/json" \
  -d '{"message": "Hello World", "value": 42}'
```

## Architecture

- **Database**: SQLite with SeaORM for data persistence
- **JavaScript Runtime**: QuickJS via rquickjs for safe script execution
- **HTTP Client**: reqwest for external API calls
- **Web Framework**: Axum for high-performance HTTP server
- **Authentication**: Basic Auth for management APIs, UUID-based for ingestion

## Development

### Build

```bash
cargo build
```

### Run Tests

```bash
cargo test
```

### Run with Logs

```bash
RUST_LOG=info cargo run
```

## License

[Add your license here]