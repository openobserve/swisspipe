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
- **GET/POST/PUT /api/v1/{workflow_uuid}/trigger** - Trigger workflow execution
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

### Required Variables
```bash
# Authentication (Required)
SP_USERNAME=admin                    # Admin username for basic authentication
SP_PASSWORD=admin                    # Admin password for basic authentication
```

### Core Application Settings
```bash
# Database Configuration
DATABASE_URL=sqlite:data/swisspipe.db?mode=rwc  # Database connection string (SQLite or PostgreSQL)
PORT=3700                           # Server port number

# Worker Pool Configuration
WORKER_COUNT=5                      # Number of async workers in the pool
JOB_POLL_INTERVAL_MS=1000          # Interval between job queue polling (milliseconds)
JOB_CLAIM_TIMEOUT_SECONDS=300      # Timeout for job claim operations (seconds)
WORKER_HEALTH_CHECK_INTERVAL_SECONDS=30        # Interval for worker health checks (seconds)
JOB_CLAIM_CLEANUP_INTERVAL_SECONDS=600         # Interval for cleaning up stale job claims (seconds)
```

### Execution and Cleanup Settings
```bash
# Workflow Execution Settings
SP_EXECUTION_RETENTION_COUNT=1000   # Number of workflow execution records to retain
SP_CLEANUP_INTERVAL_MINUTES=1       # Interval for cleanup service operations (minutes)
SP_WORKFLOW_MAX_RETRIES=0           # Maximum number of retries for failed workflow executions
```

### HTTP Loop Configuration
```bash
# HTTP Loop Scheduler Settings
SP_HTTP_LOOP_SCHEDULER_INTERVAL_SECONDS=5      # Scheduler interval for HTTP loop operations (seconds)
SP_HTTP_LOOP_MAX_HISTORY_ENTRIES=100           # Maximum number of history entries for HTTP loops
SP_HTTP_LOOP_DEFAULT_TIMEOUT_SECONDS=30        # Default timeout for HTTP loop requests (seconds)
SP_HTTP_LOOP_MAX_RESPONSE_SIZE_BYTES=10485760  # Maximum HTTP response size in bytes (10MB)
SP_HTTP_LOOP_MAX_ITERATION_TIMEOUT_SECONDS=120 # Maximum timeout for HTTP loop iterations (seconds)
```

### Email/SMTP Configuration (Optional)
```bash
# SMTP Server Settings (Required for email functionality)
SMTP_HOST=smtp.gmail.com            # SMTP server hostname (required for email)
SMTP_PORT=587                       # SMTP server port
SMTP_SECURITY=tls                   # SMTP security mode (none, tls, ssl)
SMTP_USERNAME=your-email@gmail.com  # SMTP authentication username (optional)
SMTP_PASSWORD=your-password         # SMTP authentication password (optional)
SMTP_FROM_EMAIL=noreply@yourcompany.com  # Default "from" email address (required for email)
SMTP_FROM_NAME="SwissPipe System"   # Default "from" name for emails (optional)

# SMTP Performance Settings
SMTP_TIMEOUT_SECONDS=30             # SMTP connection timeout (seconds)
SMTP_MAX_RETRIES=3                  # Maximum retries for failed email sends
SMTP_RETRY_DELAY_SECONDS=5          # Delay between email retry attempts (seconds)
SMTP_RATE_LIMIT_PER_MINUTE=60       # Email rate limit (emails per minute)
SMTP_BURST_LIMIT=10                 # Burst limit for email sending
```

### Google OAuth Configuration (Optional)
```bash
# OAuth Integration (Optional - if not set, OAuth is disabled)
GOOGLE_OAUTH_CLIENT_ID=your-client-id.googleusercontent.com     # Google OAuth client ID
GOOGLE_OAUTH_CLIENT_SECRET=your-client-secret                   # Google OAuth client secret
GOOGLE_OAUTH_ALLOWED_DOMAINS=yourcompany.com,anotherdomain.com  # Comma-separated allowed domains
GOOGLE_OAUTH_REDIRECT_URL=http://localhost:3700/auth/google/callback  # OAuth redirect URL
```

### Security Settings
```bash
# Security and Validation
SP_DANGEROUS_HEADERS=authorization,x-api-key,x-secret  # Headers to exclude from logging (comma-separated)
COOKIE_SECURE=false                 # Enable secure cookies (set to "true" for HTTPS)
```

### External Integrations
```bash
# AI Services (Optional)
ANTHROPIC_API_KEY=sk-ant-api03-...  # API key for Anthropic Claude integration

# System Information
HOSTNAME=your-server-hostname       # System hostname for email templates
```

### Runtime Logging
```bash
# Standard Rust logging (not directly used but commonly set)
RUST_LOG=info                       # Controls logging level (error, warn, info, debug, trace)
# Examples:
# RUST_LOG=debug                    # Debug level for all modules
# RUST_LOG=info,swisspipe=debug     # Info for all, debug for swisspipe
# RUST_LOG=error,swisspipe=info     # Error for all, info for swisspipe
```

### Minimal Production Configuration Example
```bash
# Minimal required variables for production deployment
SP_USERNAME=your-admin-username
SP_PASSWORD=your-secure-password
DATABASE_URL=postgresql://user:password@localhost:5432/swisspipe
PORT=3700
RUST_LOG=info
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