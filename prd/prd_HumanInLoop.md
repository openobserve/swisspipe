# PRD: Human in the Loop (HIL) Node

## Overview
The Human in the Loop (HIL) node enables workflow automation to pause and wait for human interaction before proceeding. It provides a flexible, composable approach that leverages existing SwissPipe nodes for notifications while maintaining a clean separation of concerns.

## Core Functionality

### Node Behavior
1. **Delegation Pattern**: HIL node delegates notification responsibilities to existing SwissPipe nodes (Email, HTTP Request, Webhook, etc.)
2. **Blocking Execution**: After delegating notification, the HIL node blocks workflow execution until human response is received
3. **Unique Webhook Endpoint**: Each HIL node instance gets a unique webhook URL for receiving human responses
4. **Triple Output Routing**: Routes workflow execution through three distinct paths (Approved/Denied/Notification)

### Configuration Parameters

#### Required Fields
- `notification_node_id`: ID of the node connected to the **Notification handle (blue)** responsible for sending notifications
- `title`: Human-readable title for the approval task
- `description`: Detailed instructions/context for the human reviewer

#### Optional Fields
- `timeout_seconds`: Auto-decision timeout (null = no timeout)
- `timeout_action`: Action on timeout ("approved" | "denied")
- `required_fields`: Array of fields human must provide
- `metadata`: Additional context data for notifications

#### Example Configuration
```json
{
  "notification_node_id": "email-notification-123",
  "title": "Approve Marketing Campaign",
  "description": "Please review the attached campaign materials and approve for launch",
  "timeout_seconds": 86400,
  "timeout_action": "denied",
  "required_fields": ["approval_decision", "comments"],
  "metadata": {
    "priority": "high",
    "department": "marketing"
  }
}
```

## Technical Implementation

### Webhook URL Structure
```
GET /api/v1/hil/{hil_node_execution_id}/respond?decision={approved|denied}&data={json_data}
```

**Parameters:**
- `decision`: Required. "approved" or "denied"
- `data`: Optional. JSON-encoded additional data from human
- `comments`: Optional. Human-provided comments

**Note:** `hil_node_execution_id` is unique per workflow execution instance, allowing multiple parallel executions of the same workflow to have distinct HIL tasks.

### Database Schema

#### New Entity: `human_in_loop_tasks` using SeaORM

**Migration File**: `src/database/migrator/m20250125_000001_create_human_in_loop_tasks_table.rs`

```rust
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(HumanInLoopTasks::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(HumanInLoopTasks::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(HumanInLoopTasks::ExecutionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HumanInLoopTasks::NodeId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HumanInLoopTasks::NodeExecutionId)
                            .uuid()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(HumanInLoopTasks::WorkflowId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HumanInLoopTasks::Title)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HumanInLoopTasks::Description)
                            .text(),
                    )
                    .col(
                        ColumnDef::new(HumanInLoopTasks::Status)
                            .string()
                            .not_null()
                            .default("pending"),
                    )
                    .col(
                        ColumnDef::new(HumanInLoopTasks::TimeoutAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(HumanInLoopTasks::TimeoutAction)
                            .string(),
                    )
                    .col(
                        ColumnDef::new(HumanInLoopTasks::RequiredFields)
                            .json(),
                    )
                    .col(
                        ColumnDef::new(HumanInLoopTasks::Metadata)
                            .json(),
                    )
                    .col(
                        ColumnDef::new(HumanInLoopTasks::ResponseData)
                            .json(),
                    )
                    .col(
                        ColumnDef::new(HumanInLoopTasks::ResponseReceivedAt)
                            .timestamp_with_time_zone(),
                    )
                    .col(
                        ColumnDef::new(HumanInLoopTasks::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(HumanInLoopTasks::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_hil_execution_id")
                            .from(HumanInLoopTasks::Table, HumanInLoopTasks::ExecutionId)
                            .to(WorkflowExecutions::Table, WorkflowExecutions::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_hil_workflow_id")
                            .from(HumanInLoopTasks::Table, HumanInLoopTasks::WorkflowId)
                            .to(Workflows::Table, Workflows::Id),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(HumanInLoopTasks::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum HumanInLoopTasks {
    Table,
    Id,
    ExecutionId,
    NodeId,
    NodeExecutionId,
    WorkflowId,
    Title,
    Description,
    Status,
    TimeoutAt,
    TimeoutAction,
    RequiredFields,
    Metadata,
    ResponseData,
    ResponseReceivedAt,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum WorkflowExecutions {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Workflows {
    Table,
    Id,
}
```

**Entity Definition**: `src/database/human_in_loop_tasks.rs`

```rust
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "human_in_loop_tasks")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub execution_id: Uuid,
    pub node_id: String,
    pub node_execution_id: Uuid,
    pub workflow_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub timeout_at: Option<DateTimeWithTimeZone>,
    pub timeout_action: Option<String>,
    pub required_fields: Option<Json>,
    pub metadata: Option<Json>,
    pub response_data: Option<Json>,
    pub response_received_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::database::workflow_executions::Entity",
        from = "Column::ExecutionId",
        to = "crate::database::workflow_executions::Column::Id"
    )]
    WorkflowExecution,
    #[sea_orm(
        belongs_to = "crate::database::entities::Entity",
        from = "Column::WorkflowId",
        to = "crate::database::entities::Column::Id"
    )]
    Workflow,
}

impl Related<crate::database::workflow_executions::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WorkflowExecution.def()
    }
}

impl Related<crate::database::entities::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Workflow.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
```

### Workflow Execution Flow

1. **HIL Node Reached**
   - Create entry in `human_in_loop_tasks` table
   - Generate unique webhook URL
   - Inject webhook URL and task details into event data

2. **Notification Path Execution (Blue Handle)**
   - **Immediate execution**: Notification path triggers first
   - Event flows to notification node (Email, HTTP, Webhook, etc.)
   - Event data includes:
     - Original workflow data
     - HIL task details (title, description, metadata)
     - Unique webhook URL for human response
     - Timeout information
     - Required fields specification

3. **HIL Node Blocking State**
   - After notification path completes, HIL node enters "waiting" state
   - Main workflow execution pauses at this node
   - Optional timeout timer starts
   - Notification workflow continues independently

4. **Human Response Received**
   - Human accesses webhook URL (via email link, dashboard, etc.)
   - Response updates `human_in_loop_tasks` database
   - HIL node resumes main workflow execution

5. **Response Path Routing**
   - **Approved Response**: Event flows through Approved path (Green Handle)
   - **Denied Response**: Event flows through Denied path (Red Handle)
   - Event data includes original workflow data plus human response

6. **Timeout Handling**
   - If timeout expires, execute configured timeout action
   - Resume workflow through appropriate path (Approved/Denied) based on `timeout_action`

### Three-Path Execution Model
- **Notification Path**: Executes immediately and independently
- **Approved Path**: Executes only after human approval or timeout approval
- **Denied Path**: Executes only after human denial or timeout denial

## API Endpoints

### Respond to HIL Task
```http
GET /api/v1/hil/{hil_node_execution_id}/respond
```

**Query Parameters:**
- `decision` (required): "approved" | "denied"
- `data` (optional): JSON string with additional human input
- `comments` (optional): Human comments

**Response:**
```json
{
  "status": "success",
  "message": "Response recorded successfully",
  "task": {
    "id": "uuid",
    "title": "Task title",
    "status": "approved",
    "response_received_at": "2025-01-15T10:30:00Z"
  }
}
```

### List Pending HIL Tasks (Admin)
```http
GET /api/admin/v1/hil/tasks?status=pending
```

## Node Outputs

The HIL node has three distinct output handles, each serving a specific purpose in the workflow:

### 1. Approved Path (Green Handle)
- **Trigger**: Human responds with "approved" or timeout_action="approved"
- **Purpose**: Continue workflow execution for approved requests
- **Event Data**:
  - Original workflow data
  - Human response data
  - Approval timestamp and metadata
  - Human-provided comments/additional fields

### 2. Denied Path (Red Handle)
- **Trigger**: Human responds with "denied" or timeout_action="denied"
- **Purpose**: Route to denial handling/cleanup workflows
- **Event Data**:
  - Original workflow data
  - Denial reason/comments
  - Denial timestamp and metadata
  - Human-provided additional data

### 3. Notification Path (Blue Handle)
- **Trigger**: Immediate execution when HIL node is reached (before blocking)
- **Purpose**: Delegate notification responsibilities to downstream nodes
- **Event Data**:
  - Original workflow data
  - HIL task details (title, description, metadata)
  - Unique webhook URL for human response
  - Timeout information
  - Required fields specification

### Output Handle Execution Flow
1. **Notification Path executes first**: Immediately when HIL node is reached
2. **HIL node blocks**: Waits for human response after notification is sent
3. **Approved/Denied Path executes**: Based on human decision or timeout action

## Frontend Integration

### ExecutionsView Enhancement
Add HIL task status display:
- Show "Waiting for Human Input" status
- Display task title and description
- Show time elapsed / time remaining
- Link to respond (if user has permission)
- **Three-Path Status Tracking**:
  - Track notification path completion independently
  - Show "Notification Sent" status when blue handle path completes
  - Show "Waiting for Response" when HIL blocks for human input
  - Display final routing (Approved/Denied path) when response received

### New Component: HIL Task Response Form
Simple form for human response:
- Display task context and original workflow data
- **Two Response Options**: Approval/Denial buttons (not three - notification is automatic)
- Optional comment field
- Additional data collection fields based on `required_fields` configuration
- Real-time validation for required fields

### Workflow Designer Updates
- **Handle Connection Logic**: Support three distinct output connections from HIL nodes
- **Visual Feedback**: Show all three handles with color coding and labels
- **Connection Validation**: Ensure notification handle connects to appropriate notification nodes
- **Edge Rendering**: Display three different colored edges from HIL node

## Configuration in Workflow Designer

### Node Properties Panel
- **Notification Node**: Dropdown to select existing node for notifications
- **Task Details**: Title and description fields
- **Timeout Settings**: Enable/disable timeout with duration and default action
- **Required Fields**: Define what data human must provide

### Visual Representation
- **Node Design**: Distinct icon (person + clock symbol) with red theme
- **Three Output Handles**:
  - **Green Handle (Left)**: "Approved" path - for successful approvals
  - **Red Handle (Center)**: "Denied" path - for rejections and denials
  - **Blue Handle (Right)**: "Notification" path - for immediate notification delegation
- **Handle Labels**: Color-coded text labels below each handle for clarity
- **Visual Indicators**:
  - Timeout indicator (orange dot) when timeout is configured
  - Pending status animation (pulsing yellow) during human wait state
  - User icon (person symbol) in top-right corner
- **Status Color Coding**:
  - Pending=yellow (pulsing animation)
  - Approved=green (border highlight)
  - Denied=red (border highlight)
- **Node Hover Effects**: Red-themed hover with elevation and shadow

## Security & Access Control

### Authentication
- Webhook URLs include secure tokens
- Optional: Require user authentication for sensitive approvals
- Audit logging of all human responses

### Permissions
- Node configuration requires workflow edit permissions
- Response permissions can be role-based or user-specific
- Admin can view/manage all pending HIL tasks

## Error Handling & Edge Cases

### Notification Failures
- If notification node fails, HIL task remains in pending state
- Admin can manually retry notifications or resolve tasks

### Duplicate Responses
- Webhook endpoint handles idempotency
- Only first valid response is accepted

### System Restarts
- Pending HIL tasks survive system restarts
- Timeout timers resume correctly after restart

## Success Metrics

### Performance
- HIL task creation time < 100ms
- Webhook response time < 200ms
- Database queries optimized for large task volumes

### User Experience
- Clear task presentation in notification messages
- Simple, error-free response interface
- Appropriate timeout defaults (24 hours suggested)

### System Integration
- Seamless integration with existing node types
- No breaking changes to current workflow engine
- Backward compatibility with existing workflows

## Future Enhancements

### Phase 2 Considerations
- Multi-approver support (require N of M approvers)
- Escalation chains (auto-reassign after partial timeout)
- Rich form builders for complex human input
- Mobile-optimized response interface
- Integration with external approval systems (JIRA, ServiceNow)

This PRD provides a solid foundation for implementing HIL nodes while maintaining SwissPipe's architectural principles of composability and reusability.