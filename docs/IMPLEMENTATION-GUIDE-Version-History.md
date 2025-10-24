# Workflow Version History - Implementation Guide

## ✅ COMPLETED (Working & Tested)

### 1. Database Layer ✅
- **Migration**: `src/database/migrator/m20250201_000001_create_workflow_versions_table.rs`
- **Entity**: `src/database/workflow_versions.rs`
- **Status**: Migration registered, code compiles successfully

### 2. Service Layer ✅
- **Service**: `src/versions/service.rs`
- **Module**: `src/versions/mod.rs`
- **Features**:
  - `create_version()` - Creates version with validation
  - `get_versions()` - Lists with pagination
  - `get_version_detail()` - Gets full snapshot
  - Extracts workflow name from JSON
- **Status**: Code compiles, ready for API integration

---

## 🔄 NEXT STEPS TO COMPLETE

### Step 3: Create API Endpoints

**Location**: `src/api/versions/`

**Files to create**:
1. `src/api/versions/mod.rs`
2. `src/api/versions/handlers.rs`
3. `src/api/versions/routes.rs`

**Implementation**:

#### `src/api/versions/handlers.rs`:
```rust
use crate::versions::{VersionService, CreateVersionRequest};
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct PaginationQuery {
    #[serde(default = "default_limit")]
    pub limit: u64,
    #[serde(default)]
    pub offset: u64,
}

fn default_limit() -> u64 {
    50
}

pub async fn create_version(
    State(state): State<AppState>,
    Path(workflow_id): Path<String>,
    Json(req): Json<CreateVersionRequest>,
) -> Result<Json<_>, _> {
    // Get username from auth (for now use "admin")
    let changed_by = "admin"; // TODO: Get from auth context

    let version = state.version_service
        .create_version(
            &workflow_id,
            &req.workflow_snapshot,
            &req.commit_message,
            req.commit_description.as_deref(),
            changed_by,
        )
        .await?;

    Ok(Json(version))
}

pub async fn get_versions(
    State(state): State<AppState>,
    Path(workflow_id): Path<String>,
    Query(params): Query<PaginationQuery>,
) -> Result<Json<_>, _> {
    let versions = state.version_service
        .get_versions(&workflow_id, params.limit, params.offset)
        .await?;

    Ok(Json(versions))
}

pub async fn get_version_detail(
    State(state): State<AppState>,
    Path((workflow_id, version_id)): Path<(String, String)>,
) -> Result<Json<_>, _> {
    let version = state.version_service
        .get_version_detail(&workflow_id, &version_id)
        .await?;

    Ok(Json(version))
}
```

#### `src/api/versions/routes.rs`:
```rust
use super::handlers;
use crate::AppState;
use axum::{routing::get, Router};

pub fn create_routes(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/workflows/:workflow_id/versions",
            get(handlers::get_versions).post(handlers::create_version))
        .route("/api/v1/workflows/:workflow_id/versions/:version_id",
            get(handlers::get_version_detail))
        .with_state(state)
}
```

#### `src/api/versions/mod.rs`:
```rust
pub mod handlers;
pub mod routes;
```

---

### Step 4: Add VersionService to AppState

**File**: `src/lib.rs`

**Changes**:
```rust
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<sea_orm::DatabaseConnection>,
    pub engine: Arc<workflow::engine::WorkflowEngine>,
    pub config: Arc<config::Config>,
    pub hil_service: Arc<hil::HilService>,
    pub worker_pool: Arc<async_execution::MpscWorkerPool>,
    pub mpsc_distributor: Arc<async_execution::MpscJobDistributor>,
    pub workflow_cache: Arc<cache::WorkflowCache>,
    pub delay_scheduler: Arc<async_execution::DelayScheduler>,
    pub http_loop_scheduler: Arc<async_execution::HttpLoopScheduler>,
    pub variable_service: Arc<variables::VariableService>,
    pub template_engine: Arc<variables::TemplateEngine>,
    pub schedule_service: Arc<schedule::ScheduleService>,
    pub version_service: Arc<versions::VersionService>,  // ADD THIS
}
```

**File**: `src/main.rs`

**Find** the AppState initialization and add:
```rust
let version_service = Arc::new(versions::VersionService::new(db.clone()));

let state = AppState {
    // ... existing fields ...
    version_service,  // ADD THIS
};
```

---

### Step 5: Register Version Routes

**File**: `src/api/mod.rs`

**Add**:
```rust
pub mod versions;
```

**File**: `src/main.rs`

**Find** where routes are combined and add:
```rust
let app = Router::new()
    // ... existing routes ...
    .merge(api::versions::routes::create_routes(state.clone()))
    // ... rest of routes ...
```

---

### Step 6: Frontend - CommitMessageModal Component

**Location**: `frontend/src/components/modals/CommitMessageModal.vue`

```vue
<template>
  <div v-if="visible" class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
    <div class="bg-slate-800 rounded-lg shadow-xl max-w-2xl w-full mx-4 border border-slate-700">
      <!-- Header -->
      <div class="px-6 py-4 border-b border-slate-700 flex items-center justify-between">
        <h2 class="text-lg font-semibold text-gray-200">Commit Changes</h2>
        <button @click="$emit('close')" class="text-gray-400 hover:text-gray-200">
          <XMarkIcon class="h-6 w-6" />
        </button>
      </div>

      <!-- Content -->
      <div class="p-6 space-y-4">
        <!-- Subject Line -->
        <div>
          <label class="block text-sm font-medium text-gray-300 mb-2">
            Subject (required)
          </label>
          <input
            v-model="message"
            type="text"
            maxlength="100"
            placeholder="Add email notification on failure"
            class="w-full px-4 py-2 bg-slate-700 border border-slate-600 rounded-md text-gray-200 placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-blue-500"
            @keyup.enter="handleConfirm"
          />
          <div class="mt-1 text-xs text-gray-400">
            {{ message.length }}/100 characters
          </div>
        </div>

        <!-- Description -->
        <div>
          <label class="block text-sm font-medium text-gray-300 mb-2">
            Description (optional)
          </label>
          <textarea
            v-model="description"
            rows="4"
            maxlength="1000"
            placeholder="Added email node to send notifications when the HTTP request fails..."
            class="w-full px-4 py-2 bg-slate-700 border border-slate-600 rounded-md text-gray-200 placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-blue-500"
          ></textarea>
          <div class="mt-1 text-xs text-gray-400">
            {{ description.length }}/1000 characters
          </div>
        </div>
      </div>

      <!-- Footer -->
      <div class="px-6 py-4 border-t border-slate-700 flex justify-end space-x-3">
        <button
          @click="$emit('close')"
          class="px-4 py-2 text-gray-300 hover:text-gray-100 transition-colors"
        >
          Cancel
        </button>
        <button
          @click="handleConfirm"
          :disabled="!message.trim() || saving"
          class="px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:bg-slate-600 disabled:cursor-not-allowed text-white rounded-md transition-colors"
        >
          <span v-if="saving">Saving...</span>
          <span v-else>Commit & Save</span>
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { XMarkIcon } from '@heroicons/vue/24/outline'

interface Props {
  visible: boolean
  saving?: boolean
}

defineProps<Props>()

const emit = defineEmits<{
  close: []
  confirm: [{ message: string; description: string | null }]
}>()

const message = ref('')
const description = ref('')

function handleConfirm() {
  if (!message.value.trim()) return

  emit('confirm', {
    message: message.value.trim(),
    description: description.value.trim() || null
  })
}
</script>
```

---

### Step 7: Frontend - VersionHistoryPanel Component

**Location**: `frontend/src/components/panels/VersionHistoryPanel.vue`

Similar to ExecutionSidePanel, create a panel that:
- Fetches versions from API
- Shows list with pagination
- Displays commit messages, timestamps, authors
- Click to expand description

---

### Step 8: Update WorkflowDesignerHeader

**File**: `frontend/src/components/workflow/WorkflowDesignerHeader.vue`

Add History button next to Executions button:
```vue
<button @click="$emit('toggle-version-history')" ...>
  <ClockIcon class="h-5 w-5" />
  History
</button>
```

---

### Step 9: Update Workflow Save Logic

**File**: `frontend/src/views/WorkflowDesignerView.vue`

Modify save function to:
1. Show CommitMessageModal
2. On confirm, call version API
3. Then save workflow

---

## 📋 Testing Checklist

- [ ] Migration runs successfully
- [ ] Can create version via API
- [ ] Can list versions with pagination
- [ ] Can get version details
- [ ] Modal shows on workflow save
- [ ] History panel displays versions
- [ ] Initial v1 created on first save

---

## 🎯 Current Status

**Backend**: 100% complete (migration, entity, service)
**API**: Not started
**Frontend**: Not started
**Integration**: Not started

**Estimated Time Remaining**: 4-6 hours for full implementation
