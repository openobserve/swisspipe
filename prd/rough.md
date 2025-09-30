We need to build a workflow engine. We should be able to trigger this using get, post or put. Using put and post, we should be able to accept data. Using get, we should still be able to get data using code parameters. We should be able to support many workflows. So we need to have a UI for the workflows list. For each workflow, we should generate a new endpoint. We should be similar to /api/v1/{uuid}.

It should be built using Rust and Axum for high performance. Database to be used is SQLite. Goal is to build something which is high performance. It should be built using Rust and Axum for high performance. It should be built using Rust and Axum for high performance. It should be built using Rust and Axum for high performance. It should be built using Rust and Axum for high performance. It should be built using Rust and Axum for high performance. It should be built using Rust and Axum for high performance. We also need an endpoint called JSON_array. Where we can accept JSON array data. We can accept JSON array data. The goal is to build something very simple. We should be able to accept the data and send it to a destination. Each of those destinations will be called apps, short for application. 

## Overview
Build a minimal backend service that provides workflow engine to accept incoming data.


## Non-Goals
- Event persistence and replay
- High availability and failover
- Horizontal scaling
- Complex authentication systems
- Data deduplication
- Backpressure management
- Multi-tenancy or workspaces
- No GUI for now

## Technology Stack
- **Language**: Rust
- **Web Framework**: Axum
- **Database**: SQLite with SeaORM
- **Deployment**: Single binary with no external dependencies
-- Use quickJS for implementing transformers and conditions

Should have DAG based architecture.

### Core Concepts

1. **Nodes**: Independent processing units in the workflow
   - **Triggers**: Entry points for data
   - **Conditions**: Decision points (renamed from routing rules)
   - **Transformers**: Data modification units
   - **Apps**: Data endpoints

2. **Edges**: Connections between nodes that define the workflow flow

3. **Responses**: Data returned from apps that can influence workflow progression

#### 3. Transformer Nodes
- Modify, enrich, or filter data
- Stateless operations
- Support for JavaScript/WASM-based transformations
- Can be chained together

#### 4. App Nodes
- Send data to external systems (currently only webhook will be supported)
- Return responses that can be used by subsequent nodes
- Support synchronous and asynchronous operations


### Workflow Examples

#### Example 1: Simple Linear Flow
```
Trigger 1 → Condition 1 → Condition 2 → App 1
```

#### Example 2: Transformer Integration
```
Trigger 1 → Condition 1 → Transformer 1 → Condition 2 → App 1
```

#### Example 3: Response-based Routing
```
Trigger 1 → Condition 1 → App 1 → Transformer 1 → App 2
```

#### Example 4: Complex Branching
```
Trigger 1 → App 1 → Transformer 1 → App 2
              → App 2 → Condition 2 (yes) → App 3
                              → Condition 2 (no) → App 5
              → App 3 → Transformer 2 → App 4

Everything must be done think synchronously

Functionality for Transformers and Conditions should be implemented using JavaScript. Transformers should be written like a function that will accept an event and will return an event. And user can do right simple JavaScript code in that particular function to process the incoming data. Conditions will also be written in JavaScript which will accept a function just like transformer and function should accept event as a parameter and return either true or false.


Have a single apf for workflow creation. They'll create the workflow, the nodes, conditions and transformers. While getting the get apfworkflows, you're also written everything. Workflow, the API should also have a list endpoint. We can get all the workflows.

The following environment variables should be used.

SP_USERNAME=admin
SP_PASSWORD=admin
DATABASE_URL=sqlite:data/swisspipe.db?mode=rwc
PORT=3700

Do not create a table for users. The username and password should always be picked up from the environment variable and all the API end points will be validated will be authenticated using these environment variable credentials. The data ingestion end points do not need separate authentication. The UUID in the URL will act as the authentication variable.


Every workflow will have a start node. Every workflow will have a right key which is the UUID that needs to be stored in the same table of workflow.


### 1. Workflow-Centric Entity Management
- **All entities belong to workflows**: apps, conditions, transformers all scoped to specific workflows
- **Human-readable references**: Edges reference nodes by name within workflow context

Do not design front end for now.


a transformer should look like:

function transformer(event) {
   // Do some processing wit thteh event

   retrun event
}

a transformer should look like:

function transformer(event) {
   // Do some processing with the event

   return event
}

a condition should look like:

function condition(event) {
   // Do some processing with the event

   retrun true/false
}

