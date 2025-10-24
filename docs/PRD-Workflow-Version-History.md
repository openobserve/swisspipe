# Product Requirements Document: Workflow Version History

## Overview
Implement a comprehensive version history system for workflows that tracks all changes with commit-style messages, similar to Git. This provides audit trail, accountability, and the ability to understand the evolution of workflows over time.

## Problem Statement
Currently, when users modify and save workflows, there is no record of:
- What changes were made
- Why the changes were made
- Who made the changes
- When the changes were made

This lack of tracking makes it difficult to:
- Debug issues introduced by recent changes
- Understand the reasoning behind workflow design decisions
- Maintain accountability in team environments
- Comply with audit requirements

## Goals
1. **Mandatory Change Documentation**: Require users to document changes before saving workflows
2. **Complete Audit Trail**: Store full history of all workflow modifications
3. **Easy History Access**: Provide intuitive UI to view and understand change history
4. **Accountability**: Track which user made which changes
5. **Diff Visualization**: Show what actually changed between versions (future enhancement)

## Non-Goals (Out of Scope)
- Workflow version rollback/restore functionality (can be added later)
- Branch/merge functionality like Git
- Collaborative editing with conflict resolution
- Diff visualization in the initial release (future enhancement)

## User Stories

### US1: Save with Commit Message
**As a** workflow designer
**I want to** be required to enter a commit message when saving a workflow
**So that** I document my changes and the reasoning behind them

**Acceptance Criteria:**
- When clicking "Save" button, a modal appears asking for a commit message
- Commit message has a subject line (required, max 100 chars) and optional description (max 1000 chars)
- Save button in modal is disabled until subject line is filled
- User can cancel and return to editing
- After successful save with message, user receives confirmation

### US2: View Version History
**As a** workflow designer
**I want to** view the complete change history of a workflow
**So that** I can understand how the workflow evolved over time

**Acceptance Criteria:**
- "History" button is visible next to "Executions" button in workflow designer header
- Clicking "History" opens a side panel showing version history
- Each version shows: version number, timestamp, author, commit message
- Versions are listed in reverse chronological order (newest first)
- Panel shows pagination for workflows with many versions

### US3: View Version Details
**As a** workflow designer
**I want to** see detailed information about a specific version
**So that** I can understand exactly what changed

**Acceptance Criteria:**
- Clicking on a version in history shows expanded details
- Details include: full commit message description, timestamp, author
- Future: Show diff of what changed (nodes added/removed/modified)

## Database Schema

### New Table: `workflow_versions`

```sql
CREATE TABLE workflow_versions (
    id TEXT PRIMARY KEY,                    -- UUID v7 for version ID
    workflow_id TEXT NOT NULL,              -- Reference to workflows table
    version_number INTEGER NOT NULL,        -- Sequential version number (1, 2, 3...)

    -- Complete workflow snapshot as JSON
    workflow_snapshot TEXT NOT NULL,        -- Complete workflow JSON (name, description, nodes, edges)

    -- Version metadata
    commit_message TEXT NOT NULL,           -- Commit subject line (max 100 chars)
    commit_description TEXT,                -- Optional detailed description (max 1000 chars)
    changed_by TEXT NOT NULL,               -- Username who made the change

    -- Timestamps
    created_at INTEGER NOT NULL,            -- Microsecond timestamp

    -- Constraints
    FOREIGN KEY (workflow_id) REFERENCES workflows(id) ON DELETE CASCADE,
    UNIQUE(workflow_id, version_number)
);

-- Indices for performance
CREATE INDEX idx_workflow_versions_workflow_id ON workflow_versions(workflow_id);
CREATE INDEX idx_workflow_versions_created_at ON workflow_versions(created_at);
```

**Workflow Snapshot JSON Format:**

The snapshot uses the **exact same JSON format** as:
- JSON View in the workflow designer
- Exported workflow files
- Imported workflow files

```json
{
  "name": "User Onboarding",
  "description": "Workflow description",
  "nodes": [
    {
      "id": "node-123",
      "name": "Start",
      "node_type": {
        "Trigger": {
          "methods": ["GET", "POST"]
        }
      },
      "position_x": 100,
      "position_y": 100
    }
  ],
  "edges": [
    {
      "from_node_id": "node-123",
      "to_node_id": "node-456",
      "condition_result": null,
      "source_handle_id": null
    }
  ]
}
```

**Benefits of storing complete JSON:**
- **Simplicity**: Single column contains entire workflow state
- **Consistency**: Matches the structure already used in `workflows` table
- **Flexibility**: Easy to add new workflow fields without schema changes
- **Restoration**: Can directly restore a previous version by importing the JSON
- **Compatibility**: JSON format is already understood by frontend and backend
- **Export/Import Ready**: Versions can be exported as workflow files
- **JSON View Compatible**: Versions display correctly in JSON view modal

### Table: `workflows` (modifications)
No schema changes needed. The `workflows` table continues to store the current/latest version.

## API Endpoints

### 1. Create Workflow Version (Called on Save)
**Endpoint:** `POST /api/v1/workflows/:workflow_id/versions`

**Request Body:**
```json
{
  "commit_message": "Add email notification on failure",
  "commit_description": "Added email node to send notifications when the HTTP request fails. Connected to the failure path of the condition node.",
  "workflow_snapshot": {
    "name": "User Onboarding",
    "description": "Workflow description",
    "nodes": [...],
    "edges": [...]
  }
}
```

**Response:**
```json
{
  "version_id": "019a1234-5678-7abc-8def-9012345678ab",
  "version_number": 15,
  "created_at": 1234567890123456
}
```

**Business Logic:**
- Validate commit message is not empty and within length limits
- Get current max version_number for workflow and increment by 1
- Create new version record with snapshot of current workflow state
- Include username from authentication context

### 2. Get Version History
**Endpoint:** `GET /api/v1/workflows/:workflow_id/versions`

**Query Parameters:**
- `limit` (default: 50, max: 100)
- `offset` (default: 0)

**Response:**
```json
{
  "versions": [
    {
      "id": "019a1234-5678-7abc-8def-9012345678ab",
      "workflow_id": "019a1234-0000-7abc-8def-000000000000",
      "version_number": 15,
      "commit_message": "Add email notification on failure",
      "commit_description": "Added email node...",
      "changed_by": "admin",
      "created_at": 1234567890123456,
      "workflow_name": "User Onboarding"
    },
    {
      "id": "019a1234-5678-7abc-8def-9012345678aa",
      "workflow_id": "019a1234-0000-7abc-8def-000000000000",
      "version_number": 14,
      "commit_message": "Fix timeout configuration",
      "commit_description": null,
      "changed_by": "john_doe",
      "created_at": 1234567890123450,
      "workflow_name": "User Onboarding"
    }
  ],
  "total": 15,
  "limit": 50,
  "offset": 0
}
```

**Implementation Note:** The `workflow_name` field is extracted from the `workflow_snapshot` JSON during the query for display purposes. The full snapshot is NOT returned in the list view for performance reasons.

### 3. Get Specific Version Details
**Endpoint:** `GET /api/v1/workflows/:workflow_id/versions/:version_id`

**Response:**
```json
{
  "id": "019a1234-5678-7abc-8def-9012345678ab",
  "workflow_id": "019a1234-0000-7abc-8def-000000000000",
  "version_number": 15,
  "workflow_snapshot": {
    "name": "User Onboarding",
    "description": "Workflow description",
    "nodes": [
      {
        "id": "node-123",
        "name": "Start",
        "node_type": {
          "Trigger": {
            "methods": ["GET", "POST"]
          }
        },
        "position_x": 100,
        "position_y": 100
      }
    ],
    "edges": [
      {
        "from_node_id": "node-123",
        "to_node_id": "node-456",
        "condition_result": null,
        "source_handle_id": null
      }
    ]
  },
  "commit_message": "Add email notification on failure",
  "commit_description": "Added email node to send notifications...",
  "changed_by": "admin",
  "created_at": 1234567890123456
}
```

**Note:** The `workflow_snapshot` field contains the complete workflow JSON that can be directly used to restore/import the workflow.

## Frontend Components

### 1. Commit Message Modal (`CommitMessageModal.vue`)
**Location:** `frontend/src/components/modals/CommitMessageModal.vue`

**Props:**
- `visible: boolean` - Show/hide modal
- `saving: boolean` - Show loading state during save

**Emits:**
- `close` - User cancels
- `confirm` - User confirms with message: `{ message: string, description?: string }`

**UI Design:**
```
┌─────────────────────────────────────────────────┐
│  Commit Changes                            [X]  │
├─────────────────────────────────────────────────┤
│                                                 │
│  Subject (required)                             │
│  ┌─────────────────────────────────────────┐   │
│  │ Add email notification on failure       │   │
│  └─────────────────────────────────────────┘   │
│  Character count: 34/100                        │
│                                                 │
│  Description (optional)                         │
│  ┌─────────────────────────────────────────┐   │
│  │ Added email node to send notifications  │   │
│  │ when the HTTP request fails. Connected  │   │
│  │ to the failure path of the condition... │   │
│  └─────────────────────────────────────────┘   │
│  Character count: 120/1000                      │
│                                                 │
│            [Cancel]  [Commit & Save]            │
└─────────────────────────────────────────────────┘
```

**Validation:**
- Subject is required, 1-100 characters
- Description is optional, max 1000 characters
- "Commit & Save" button disabled until subject is valid

### 2. Version History Panel (`VersionHistoryPanel.vue`)
**Location:** `frontend/src/components/panels/VersionHistoryPanel.vue`

**Props:**
- `workflow-id: string` - Current workflow ID

**Emits:**
- `close` - User closes panel

**UI Design:**
```
┌─────────────────────────────────────────────────┐
│  Version History                      [↻]  [X]  │
├─────────────────────────────────────────────────┤
│                                                 │
│  ┌─────────────────────────────────────────┐   │
│  │ v15  20s ago          @admin            │   │
│  │ Add email notification on failure       │   │
│  │ ▼ Added email node to send...          │   │
│  └─────────────────────────────────────────┘   │
│                                                 │
│  ┌─────────────────────────────────────────┐   │
│  │ v14  2h ago           @john_doe         │   │
│  │ Fix timeout configuration               │   │
│  └─────────────────────────────────────────┘   │
│                                                 │
│  ┌─────────────────────────────────────────┐   │
│  │ v13  1d ago           @admin            │   │
│  │ Update transformer script               │   │
│  └─────────────────────────────────────────┘   │
│                                                 │
│                                                 │
│                  [Load More]                    │
└─────────────────────────────────────────────────┘
```

**Features:**
- Each version shows version number, relative time, author, commit message
- Clicking version expands to show full description
- Pagination with "Load More" button
- Refresh button to reload history
- Close button to hide panel

### 3. History Button in Designer Header
**Location:** Modify `frontend/src/components/workflow/WorkflowDesignerHeader.vue`

**Changes:**
- Add "History" button next to "Executions" button
- Same styling as Executions button
- Icon: Clock/history icon
- Emits: `@toggle-version-history`

## Backend Services

### Service: `VersionService`
**Location:** `src/database/workflow_versions.rs`

**Methods:**
```rust
impl VersionService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self;

    // Create new version when workflow is saved
    // Takes complete workflow JSON snapshot
    pub async fn create_version(
        &self,
        workflow_id: &str,
        workflow_snapshot: &str,  // Complete workflow JSON as string
        commit_message: &str,
        commit_description: Option<&str>,
        changed_by: &str,
    ) -> Result<WorkflowVersion>;

    // Get version history with pagination
    // Returns list with basic info (no full snapshot)
    pub async fn get_versions(
        &self,
        workflow_id: &str,
        limit: u32,
        offset: u32,
    ) -> Result<VersionHistoryResponse>;

    // Get specific version details
    // Returns full workflow snapshot for the version
    pub async fn get_version(
        &self,
        workflow_id: &str,
        version_id: &str,
    ) -> Result<WorkflowVersionDetail>;

    // Get latest version number for a workflow
    async fn get_latest_version_number(
        &self,
        workflow_id: &str,
    ) -> Result<i32>;
}
```

**Note:** The `workflow_snapshot` parameter is the serialized JSON string of the complete workflow (name, description, nodes, edges). This simplifies the API and allows easy storage/retrieval.

## User Flow

### Flow 0: Initial Workflow Creation
1. User creates a new workflow in designer
2. User adds initial nodes and edges
3. User clicks "Save" button for the first time
4. System shows commit message modal
5. User enters commit subject (or system pre-fills "Initial workflow creation")
6. User clicks "Commit & Save"
7. System:
   - Saves workflow to `workflows` table
   - **Automatically creates v1** in `workflow_versions` table with the commit message
   - Shows success notification
8. Modal closes and workflow now has version history starting at v1

**Note:** Every workflow starts with v1 upon first save. This ensures complete audit trail from creation.

### Flow 1: Saving Workflow with Commit Message
1. User makes changes to workflow in designer
2. User clicks "Save" button
3. System shows commit message modal
4. User enters commit subject (required) and description (optional)
5. User clicks "Commit & Save"
6. System:
   - Saves workflow to `workflows` table (existing logic)
   - Creates version snapshot in `workflow_versions` table (increments version number)
   - Shows success notification
7. Modal closes and user continues editing

### Flow 2: Viewing Version History
1. User clicks "History" button in designer header
2. System fetches version history via API
3. Side panel slides in showing version list
4. User scrolls through versions
5. User clicks on a version to expand details
6. User clicks "Load More" for older versions
7. User closes panel to return to editing

## Success Metrics
- **Adoption:** >80% of workflow saves include meaningful commit messages (>10 chars)
- **Usage:** >50% of users view version history at least once
- **Quality:** Average commit message length >30 characters
- **Compliance:** 100% of workflow changes have audit trail

## Security & Privacy
- **Authentication:** Only authenticated users can create versions
- **Authorization:** Only users with workflow access can view version history
- **Data Retention:** Versions are kept indefinitely (no automatic cleanup)
- **Cascading Delete:** When workflow is deleted, all versions are deleted (CASCADE)

## Performance Considerations
- **Version Creation:** Should not significantly slow down workflow save (<200ms overhead)
- **History Loading:** Paginated queries with proper indexing for fast retrieval
- **Storage:** JSON snapshots will grow over time, but acceptable for most use cases
  - Example: 50KB per version × 100 versions = 5MB per workflow (reasonable)

## Future Enhancements (V2)
1. **Diff Visualization:** Show visual diff between two versions
2. **Version Restore:** Roll back to previous version
3. **Version Comparison:** Compare any two versions side-by-side
4. **Export History:** Download version history as CSV/JSON
5. **Version Tags:** Mark specific versions as "stable" or "production"
6. **Change Statistics:** Show metrics like "Added 3 nodes, removed 1 edge"

## Implementation Decisions

### ✅ Decision 1: Automatic v1 Creation
**Question:** Should we automatically create a version when workflow is created (v1 = initial creation)?
**Decision:** **YES** - Every workflow will automatically create v1 upon first save with the user's commit message.
**Rationale:** Ensures complete audit trail from creation. Users document their initial design intent.

### Decision 2: Version Retention
**Question:** Should we limit how many versions to keep per workflow?
**Decision:** **NO LIMIT** initially. Add cleanup configuration later if needed.
**Rationale:** Storage is cheap, audit trail is valuable. Can add retention policies in V2 if needed.

### Decision 3: Search Functionality
**Question:** Should version history be searchable by commit message?
**Decision:** **NOT IN V1**. Add search in future enhancement.
**Rationale:** Pagination handles most use cases. Search adds complexity that can wait for V2.

## Technical Implementation Notes

### Migration Strategy
1. Create new `workflow_versions` table via migration
2. No backfill of existing workflows (history starts from feature release)
3. Add version service to application startup
4. Update workflow save endpoint to call version service

### Error Handling
- If version creation fails, workflow save should still succeed (log error)
- If version fetch fails, show error in history panel but don't block designer
- Validate commit message length on both frontend and backend

### Testing Strategy
1. **Unit Tests:** Service methods for version CRUD operations
2. **Integration Tests:** API endpoints with authentication
3. **E2E Tests:** Complete save flow with commit message modal
4. **Load Tests:** Ensure performance with 100+ versions per workflow
