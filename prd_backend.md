# SwissPipe Workflow Engine - Backend PRD

## Overview
SwissPipe is a high-performance workflow engine that accepts incoming data via REST API endpoints and processes it through configurable DAG-based workflows. Each workflow generates a unique endpoint for data ingestion and processing.

## Goals
- Build a minimal, high-performance workflow engine
- Support HTTP triggers (GET, POST, PUT) with data ingestion
- Enable dynamic workflow creation and management
- Provide JavaScript-based data transformation and conditional logic
- Generate unique API endpoints per workflow

## Non-Goals
- Event persistence and replay
- High availability and failover
- Horizontal scaling
- Complex authentication systems
- Data deduplication
- Backpressure management
- Multi-tenancy or workspaces
- GUI/Frontend (not in scope for backend)

## Technology Stack
- **Language**: Rust
- **Web Framework**: Axum
- **Database**: SQLite with SeaORM
- **JavaScript Engine**: QuickJS for transformers and conditions
- **Deployment**: Single binary with no external dependencies

## Architecture

### Core Concepts

#### 1. Workflows
- Container for all processing logic
- Each workflow has a unique UUID that serves as the API endpoint identifier
- Generated endpoint format: `/api/v1/{workflow_uuid}`
- Contains a start node that begins execution

#### 2. Nodes (Processing Units)
- **Trigger**: Entry point for data (HTTP endpoints) . Exactly 1 trigger per workflow that can be triggered by multiple endpoints (GET, POST, PUT)
- **Conditions**: JavaScript-based decision points that return true/false
- **Transformers**: JavaScript-based data modification units that accept and return events
- **Apps**: Data endpoints (currently webhook support only)

#### 3. Edges
- Connections between nodes that define workflow execution flow
- Reference nodes by human-readable names within workflow context
- Enable DAG-based workflow execution

#### 4. Responses
- Data returned from app nodes that can influence subsequent workflow progression

### JavaScript Integration
- **Transformers**: Functions that accept an event parameter and return a modified event
- **Conditions**: Functions that accept an event parameter and return boolean (true/false)
- All JavaScript execution handled via QuickJS for performance and security

### Workflow Execution
- Synchronous processing model
- Starts from designated start node
- Follows edges based on condition evaluations
- Supports linear flows, branching, and complex routing patterns

## API Endpoints

### Workflow Management
- **POST /workflows** - Create new workflow
- **GET /workflows** - List all workflows
- **GET /workflows/{id}** - Get specific workflow details
- **PUT /workflows/{id}** - Update workflow
- **DELETE /workflows/{id}** - Delete workflow

### Data Ingestion
- **GET/POST/PUT /api/v1/{workflow_uuid}/ep** - Trigger workflow execution
- UUID in URL serves as authentication mechanism
- POST/PUT endpoints accept JSON payload
- GET endpoints can accept data via query parameters

### Special Endpoints
- **POST /api/v1/{workflow_uuid}/json_array** - Accept JSON array data for processing

## Authentication & Security
- Management API endpoints require HTTP Basic Auth
- Credentials sourced from environment variables:
  - `SP_USERNAME=admin`
  - `SP_PASSWORD=admin`
- Data ingestion endpoints use UUID-based authentication (no separate auth required)

## Environment Configuration
```
SP_USERNAME=admin
SP_PASSWORD=admin
DATABASE_URL=sqlite:data/swisspipe.db?mode=rwc
PORT=3700
```

## Database Design

### Key Tables
- **workflows** - Workflow definitions with UUID keys
- **nodes** - All node types (triggers, conditions, transformers, apps)
- **edges** - Connections between nodes
- No user table required (credentials from environment variables)

### Workflow-Centric Design
- All entities (apps, conditions, transformers) scoped to specific workflows
- Human-readable node references within workflow context
- UUID-based workflow identification for API endpoints

## Workflow Examples

### Simple Linear Flow
```
Trigger → Condition → App
```

### With Transformation
```
Trigger → Condition → Transformer → App
```

### Response-based Routing
```
Trigger → App → Transformer → App
```

### Complex Branching
```
Trigger → App → Transformer → App
         → App → Condition (yes) → App
                → Condition (no) → App
         → App → Transformer → App
```

## Performance Requirements
- High-performance design using Rust and Axum
- SQLite for simple, embedded storage
- Single binary deployment
- Optimized for synchronous workflow execution

## Success Metrics
- Zero external dependencies for deployment (no kafka, redis, postgres, clickhouse, etc)
- Simple API for workflow management and data ingestion