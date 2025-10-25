# Environment Variables Implementation Guide

## Status: Phase 1 - Core Infrastructure Complete

### ✅ Completed Components

1. **Database Layer** ✓
   - Migration: `m20250130_000001_create_environment_variables_table.rs`
   - Entity: `src/database/environment_variables.rs`
   - Integrated into migrator

2. **Encryption Service** ✓
   - File: `src/variables/encryption.rs`
   - AES-256-GCM encryption
   - Environment key support with auto-generation
   - Full test coverage

3. **Variables Service** ✓
   - File: `src/variables/service.rs`
   - CRUD operations
   - Name validation
   - Secret masking
   - Template map loading

### 📝 Next Steps - Backend

#### 1. Create Template Engine (`src/variables/template_engine.rs`)

```rust
use handlebars::Handlebars;
use std::collections::HashMap;

pub struct TemplateEngine {
    handlebars: Handlebars<'static>,
}

impl TemplateEngine {
    pub fn new() -> Self {
        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(true); // Fail on undefined variables
        Self { handlebars }
    }

    /// Resolve template with variables
    /// Template format: "https://{{ env.API_HOST }}/api"
    pub fn resolve(&self, template: &str, variables: &HashMap<String, String>) -> Result<String, String> {
        // Create context with env namespace
        let mut context = serde_json::Map::new();
        let env_map: serde_json::Map<String, serde_json::Value> = variables
            .iter()
            .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
            .collect();

        context.insert("env".to_string(), serde_json::Value::Object(env_map));

        self.handlebars
            .render_template(template, &context)
            .map_err(|e| format!("Template resolution failed: {}", e))
    }
}
```

#### 2. Create Module File (`src/variables/mod.rs`)

```rust
pub mod encryption;
pub mod service;
pub mod template_engine;

pub use encryption::EncryptionService;
pub use service::{VariableService, CreateVariableRequest, UpdateVariableRequest, VariableResponse};
pub use template_engine::TemplateEngine;
```

#### 3. Add to `src/lib.rs`

```rust
pub mod variables;
```

#### 4. Create API Endpoints (`src/api/variables.rs`)

```rust
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put, delete},
    Router,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(get_all_variables).post(create_variable))
        .route("/:id", get(get_variable).put(update_variable).delete(delete_variable))
}

// Implement handlers using VariableService from State
```

#### 5. Update `AppState` (`src/main.rs`)

```rust
pub struct AppState {
    // ... existing fields
    variable_service: Arc<VariableService>,
    template_engine: Arc<TemplateEngine>,
}
```

#### 6. Integrate into Workflow Execution (`src/workflow/engine/executor.rs`)

```rust
// Before executing workflow:
let variables = self.variable_service.load_variables_map().await?;

// For each node with templated fields (HTTP, Email, etc.):
let resolved_url = self.template_engine.resolve(&node.url, &variables)?;
```

### 📝 Next Steps - Frontend

#### 1. Create Variables Store (`frontend/src/stores/variables.ts`)

```typescript
import { defineStore } from 'pinia'
import { ref } from 'vue'
import apiClient from '../services/api'

export const useVariableStore = defineStore('variables', () => {
  const variables = ref([])
  const loading = ref(false)

  async function fetchVariables() {
    loading.value = true
    try {
      const response = await apiClient.getVariables()
      variables.value = response.variables
    } finally {
      loading.value = false
    }
  }

  async function createVariable(data) {
    return await apiClient.createVariable(data)
  }

  // ... other methods

  return { variables, loading, fetchVariables, createVariable }
})
```

#### 2. Add API Methods (`frontend/src/services/api.ts`)

```typescript
async getVariables(): Promise<VariablesListResponse> {
  const response = await this.client.get('/api/admin/v1/variables')
  return response.data
}

async createVariable(data: CreateVariableRequest): Promise<Variable> {
  const response = await this.client.post('/api/admin/v1/variables', data)
  return response.data
}
```

#### 3. Create Variables View (`frontend/src/views/VariablesView.vue`)

**Features:**
- Table with columns: Name, Type, Description, Actions
- Search/filter
- Add Variable button → Modal
- Edit/Delete actions per row
- Secret values masked

#### 4. Create Variable Editor Modal Component

**Form Fields:**
- Name (uppercase validation)
- Type (Text/Secret radio buttons)
- Value (with show/hide for secrets)
- Description (textarea)

#### 5. Update Settings Navigation

Add "Environment Variables" menu item in Settings

#### 6. Add Template Support to Node Configs

**For HTTP Request Node:**
- URL field with template highlighting
- Header values with template support

**For Email Node:**
- Subject with templates
- Body template with templates

### 🧪 Testing Checklist

#### Backend Tests
- [ ] Encryption/decryption with various keys
- [ ] Variable CRUD operations
- [ ] Name validation (uppercase, no spaces, etc.)
- [ ] Duplicate name prevention
- [ ] Template resolution with variables
- [ ] Template resolution with undefined variables (should fail)
- [ ] Secret masking in responses

#### Integration Tests
- [ ] Create workflow with templated URL
- [ ] Execute workflow → verify variables resolved
- [ ] Update variable → execute workflow → verify new value used
- [ ] Workflow execution fails if variable undefined

#### E2E Tests
- [ ] Create variable via UI
- [ ] Use variable in workflow
- [ ] Execute workflow successfully
- [ ] Update variable value
- [ ] Re-execute workflow with new value

### 🔒 Security Checklist

- [ ] SP_ENCRYPTION_KEY environment variable configured
- [ ] Secrets encrypted in database
- [ ] Secrets masked in API responses
- [ ] Secrets not logged in execution logs
- [ ] Admin authentication required for all variable APIs
- [ ] Audit log for variable changes (future)

### 📦 Deployment Steps

1. **Set Environment Variable:**
   ```bash
   # Generate a secure 32-byte key
   openssl rand -hex 32

   # Set in environment
   export SP_ENCRYPTION_KEY=<generated_key>
   ```

2. **Run Migration:**
   ```bash
   cargo run  # Migrations auto-run on startup
   ```

3. **Verify:**
   - Check logs for "Database migrations completed"
   - Create a test variable via API
   - Verify secret is encrypted in database

### 🎯 Quick Start Guide for Users

1. **Navigate to Settings → Environment Variables**

2. **Create First Variable:**
   - Click "Add Variable"
   - Name: `API_HOST`
   - Type: Text
   - Value: `https://api.example.com`
   - Save

3. **Use in Workflow:**
   - Add HTTP Request node
   - URL: `{{ env.API_HOST }}/users`
   - Save and execute

4. **Create Secret:**
   - Name: `API_KEY`
   - Type: Secret
   - Value: `sk-abc123...`
   - Value will be masked after saving

### 📚 Template Syntax Reference

**Basic Variable:**
```
{{ env.VARIABLE_NAME }}
```

**In HTTP URL:**
```
https://{{ env.API_HOST }}/api/v1/users
```

**In Headers:**
```
Authorization: Bearer {{ env.API_TOKEN }}
X-API-Key: {{ env.SECRET_KEY }}
```

**In Email Subject:**
```
Update from {{ env.COMPANY_NAME }}
```

**In Email Body:**
```html
<p>Contact us at {{ env.SUPPORT_EMAIL }}</p>
<img src="{{ env.LOGO_URL }}" />
```

**Accessing Event Data:**
```
{{ event.data.user_id }}
{{ event.metadata.request_id }}
{{ event.headers.content-type }}
```

**Accessing Array Elements (use dot notation, not brackets):**
```
✅ CORRECT: {{ event.data.companies.0.id }}
❌ WRONG:   {{ event.data.companies[0].id }}

✅ CORRECT: {{ event.data.items.0.name }}
✅ CORRECT: {{ event.data.items.1.price }}
```

**Accessing Nested Object Properties:**
```
{{ event.data.user.profile.email }}
{{ event.data.response.data.companies.0.name }}
```

### 🐛 Troubleshooting

**Error: "SP_ENCRYPTION_KEY not set"**
- Solution: Set environment variable with 64-character hex string
- Auto-generated key is shown in logs (save it!)

**Error: "Variable 'API_KEY' is not defined"**
- Check variable name matches exactly (case-sensitive)
- Verify variable exists in Settings → Environment Variables
- Re-save workflow after creating variable

**Error: "Template resolution failed: Failed to parse template"**
- Common cause: Using bracket notation `[0]` for arrays instead of dot notation `.0`
- Solution: Replace `{{ event.data.items[0].id }}` with `{{ event.data.items.0.id }}`
- Check that all variable names are spelled correctly
- Verify the data structure matches your template path

**Error: "Decryption failed"**
- Encryption key changed
- Solution: Re-enter secret values with current key

### 🔄 Migration from Hardcoded Values

**Before:**
```json
{
  "url": "https://api.production.com/users",
  "headers": {
    "Authorization": "Bearer sk-prod-abc123"
  }
}
```

**After:**
1. Create variables:
   - `API_HOST` = `https://api.production.com`
   - `API_TOKEN` = `sk-prod-abc123` (secret)

2. Update workflow:
```json
{
  "url": "{{ env.API_HOST }}/users",
  "headers": {
    "Authorization": "Bearer {{ env.API_TOKEN }}"
  }
}
```

### 📈 Future Enhancements (Phase 2)

- Variable usage tracking (which workflows use each variable)
- Variable namespaces (dev/staging/prod)
- Import/export variables
- Audit logging
- Variable validation (URL, email, etc.)
- Autocomplete in UI editors
- Syntax highlighting for templates

---

**Last Updated:** 2025-01-30
**Implementation Status:** 30% Complete (Core Infrastructure Done)
**Next Milestone:** Complete Template Engine + API Endpoints
