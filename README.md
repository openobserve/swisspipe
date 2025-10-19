# SwissPipe Workflow Engine

A high-performance single node workflow engine built with Rust and Axum that processes data through configurable DAG-based workflows.

# Why SwissPipe
Swisspipe is geared for semi-technical people who have very high performance requirements where many other established workflow engines like n8n, dify etc do not work. With under 30 MB memory and negligible CPU you can have thousands of workflows running.

Swisspipe can also replace segment.com for use cases and native segment.com compatible endpoints are available as well that can receive data from segment.com SDKs.

## Features

- **DAG-based Workflows**: Define complex data processing flows using directed acyclic graphs
- **JavaScript Integration**: Use JavaScript for transformers and conditions via QuickJS
- **Multiple Node Types**: Support for HTTP requests, email, OpenObserve, delays, Conditions, Human in Loop, and more
- **Async Execution**: Background job processing with worker pools and queue management
- **Retry Logic**: Configurable exponential backoff for failed operations
- **Comprehensive APIs**: Complete workflow management and execution monitoring
- **High Performance**: Built with Rust and Axum for optimal performance
- **Single Binary**: Self-contained deployment with SQLite database
- **Email System**: SMTP integration with templating and queue management
- **Delay Scheduling**: Built-in delay nodes with resumption capabilities

## Screenshots

### Sample workflow
![Sample Workflow](./screenshots/sample_workflow.png)

### Node list
![Node List](./screenshots/node_list.png)

### Condition node

![Condition node](./screenshots/condition_node.png)


## Quick Start

### Binaries

Download binaries from https://github.com/openobserve/swisspipe/releases and run.

```shell
SP_USERNAME=admin SP_PASSWORD=admin ./swisspipe 
```

### Docker

Docker immges are available at: https://gallery.ecr.aws/zinclabs/swisspipe

```shell
docker run \
  -e SP_USERNAME=admin \
  -e SP_PASSWORD=admin \
  -p 3700:3700 \
  -v $(pwd)/data:/app/data \
  public.ecr.aws/zinclabs/swisspipe:latest
```

The server will start on `http://localhost:3700`

## Configuration

Environment variables:

- `SP_USERNAME`: Admin username for API access (default: admin)
- `SP_PASSWORD`: Admin password for API access (default: admin)  
- `DATABASE_URL`: SQLite/Postgres database URL (default: sqlite:data/swisspipe.db?mode=rwc). Use of postgres is recommended in production environments. e.g.
  - DATABASE_URL=sqlite:data/swisspipe.db?mode=rwc
  - DATABASE_URL=postgres://user:password@db1.hhhgvhlkgg.us-east-2.rds.amazonaws.com:5432/swisspipe
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
8. **Human In Loop**: Approval/Denial of workflow by a human.
9. **Anthropic**: Make a request to Anthropic's LLM.

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

### Trigger the Workflow

```bash
curl -X POST http://localhost:3700/api/v1/{workflow-id}/trigger \
  -H "Content-Type: application/json" \
  -d '{"message": "Hello World", "value": 42}'
```

## Architecture

- **Database**: SQLite/Postgres with SeaORM for data persistence
- **JavaScript Runtime**: QuickJS via rquickjs for safe script execution
- **HTTP Client**: reqwest for external API calls
- **Web Framework**: Axum for high-performance HTTP server
- **Authentication**: Basic Auth for management APIs, UUID-based for ingestion

