# SwissPipe Workflow Engine

A high-performance workflow engine built with Rust and Axum that processes data through configurable DAG-based workflows.

## Features

- **DAG-based Workflows**: Define complex data processing flows using directed acyclic graphs
- **JavaScript Integration**: Use JavaScript for transformers and conditions via QuickJS
- **Multiple App Types**: Support for webhooks and OpenObserve integration
- **Retry Logic**: Configurable exponential backoff for failed HTTP requests
- **REST API**: Complete CRUD operations for workflow management
- **High Performance**: Built with Rust and Axum for optimal performance
- **Single Binary**: Self-contained deployment with SQLite database

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

### Workflow Management (requires Basic Auth)

- `GET /workflows` - List all workflows
- `POST /workflows` - Create a new workflow
- `GET /workflows/{id}` - Get specific workflow
- `PUT /workflows/{id}` - Update workflow
- `DELETE /workflows/{id}` - Delete workflow

### Data Ingestion (UUID-based auth)

- `GET/POST/PUT /api/v1/{workflow_id}/ep` - Trigger workflow execution
- `POST /api/v1/{workflow_id}/json_array` - Accept JSON array data

## Workflow Structure

### Node Types

1. **Trigger**: Entry point for HTTP requests
2. **Condition**: JavaScript-based decision points
3. **Transformer**: JavaScript-based data modification
4. **App**: External system integration (Webhook/OpenObserve)

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
curl -X POST http://localhost:3700/workflows \
  -H "Content-Type: application/json" \
  -H "Authorization: Basic YWRtaW46YWRtaW4=" \
  -d '{
    "name": "Simple Webhook Flow",
    "description": "Transform data and send to webhook",
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
            "script": "function transformer(event) { event.data.timestamp = Date.now(); return event; }"
          }
        }
      },
      {
        "name": "webhook",
        "node_type": {
          "App": {
            "app_type": "Webhook",
            "url": "https://webhook.site/your-unique-url",
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
```

### Trigger the Workflow

```bash
curl -X POST http://localhost:3700/api/v1/{workflow-id}/ep \
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