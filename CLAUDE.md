# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

### Rust Backend
- **Build**: `cargo build`
- **Run**: `cargo run` (default port 3700) or `cargo run --bin swisspipe`
- **Run with custom port**: `PORT=3701 cargo run --bin swisspipe`
- **Run with logging**: `RUST_LOG=info cargo run` or `RUST_LOG=debug cargo run`
- **Test**: `cargo test`
- **Check types**: `cargo check`
- **Linting**: `cargo clippy`

### Vue.js Frontend
- **Development server**: `cd frontend && npm run dev`
- **Build**: `cd frontend && npm run build`
- **Type check**: `cd frontend && npm run type-check`
- **Lint**: `cd frontend && npm run lint`

### Environment Setup
- Copy `.env.example` to `.env` for configuration
- Default database: SQLite at `data/swisspipe.db`
- Default credentials: admin/admin (configurable via `SP_USERNAME`/`SP_PASSWORD`)

## Architecture Overview

### Core Components
- **Workflow Engine** (`src/workflow/`): DAG-based workflow processing with JavaScript integration
- **Async Execution** (`src/async_execution/`): Background job processing with worker pools and cleanup services
- **Database Layer** (`src/database/`): SeaORM-based SQLite persistence with migrations
- **API Layer** (`src/api/`): Axum REST endpoints for workflow management and data ingestion
- **Email System** (`src/email/`): Email notifications with queue and audit logging

### Key Modules
- **Workflow Engine**: Processes DAGs with Trigger, Condition, Transformer, and App nodes
- **JavaScript Runtime**: QuickJS via rquickjs for safe script execution in transformers/conditions
- **HTTP Client**: Configurable retry logic with exponential backoff
- **Authentication**: Basic Auth for management APIs, UUID-based for workflow ingestion
- **Worker Pool**: Asynchronous job execution with configurable retention and cleanup

### Database Schema
- `workflows`: Workflow definitions with JSON node/edge structures
- `workflow_executions`: Execution tracking with performance metrics
- `workflow_execution_steps`: Per-node execution details
- `job_queue`: Async job queue with status tracking
- `email_queue` & `email_audit_log`: Email system tables

### Frontend Architecture
- **Vue 3** with TypeScript and Composition API
- **Vue Flow**: Visual workflow designer with drag-and-drop
- **Monaco Editor**: Code editing for JavaScript transformers/conditions
- **Tailwind CSS**: Utility-first styling
- **Pinia**: State management

## Workflow Node Types

### Trigger Nodes
- Entry points for HTTP requests (GET/POST/PUT)
- Accessible via `/api/v1/{workflow_id}/ep` endpoints

### Condition Nodes
- JavaScript functions returning boolean for flow control
- Function signature: `function condition(event) { return boolean; }`

### Transformer Nodes  
- JavaScript functions for data modification
- Function signature: `function transformer(event) { return event; }`
- Return `null` to drop events from the workflow

### App Nodes
- **Webhook**: HTTP requests to external endpoints
- **OpenObserve**: Log ingestion to OpenObserve platform
- **Email**: Send emails via SMTP with templating

## Testing and Scripts

### Test Scripts
- `./test_workflow.sh`: Basic workflow creation and execution testing
- `./test_conditional_workflow.sh`: Conditional workflow testing

### Special Environment Variables
- `SP_EXECUTION_RETENTION_HOURS`: Control execution data retention (default varies)
- `SP_EXECUTION_RETENTION_COUNT`: Alternative retention by count limit
- `SP_CLEANUP_INTERVAL_MINUTES`: Cleanup service frequency
- `SP_DANGEROUS_HEADERS`: Headers to exclude from logging (comma-separated)

## Development Patterns

### Error Handling
- Uses `thiserror` for structured error types
- Comprehensive error context in workflow execution
- Email system has dedicated error types in `src/email/error.rs`

### Async Processing
- Tokio-based async runtime throughout
- Background worker pools for non-blocking execution
- Rate limiting via `governor` crate

### Security
- Basic Auth for admin endpoints
- UUID-based authentication for workflow ingestion
- Input validation via `validator` crate
- HTML escaping for email templates

### Database Operations  
- SeaORM with automatic migrations
- Connection pooling handled by SeaORM
- SQLite with WAL mode for better concurrency

## Frontend Development

### Key Components
- **WorkflowDesigner**: Main visual editor using Vue Flow
- **NodeInspector**: Property editor with Monaco integration
- **WorkflowList**: Management interface for workflows

### State Management
- Workflow data managed through Pinia stores
- Node library with predefined templates
- Real-time execution status updates