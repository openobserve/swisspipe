# Product Requirements Document: Environment Variables

## 1. Overview

### 1.1 Purpose
Enable users to define reusable variables (environment variables/secrets) in a centralized Settings menu that can be securely referenced throughout workflows using template syntax. This feature allows for better configuration management, security, and reusability across workflows.

### 1.2 Goals
- Centralize configuration management for API keys, tokens, and other sensitive data
- Enable secure storage and usage of credentials without hardcoding in workflows
- Provide template syntax for variable interpolation in workflow nodes
- Support both public configuration values and sensitive secrets
- Reduce duplication and improve maintainability of workflows

### 1.3 Non-Goals
- Integration with external secret management systems (HashiCorp Vault, AWS Secrets Manager) - Phase 2
- Variable versioning or history - Phase 2
- Per-workflow variable overrides - Phase 2
- Variable scoping by user/team - Phase 2

---

## 2. User Stories

### 2.1 Primary User Stories

**US-1: Define Variables in Settings**
- As a workflow administrator
- I want to define variables (API keys, URLs, tokens) in a Settings page
- So that I can reuse them across multiple workflows without duplication

**US-2: Use Variables in HTTP Request Nodes**
- As a workflow designer
- I want to reference variables using `{{ env.VARIABLE_NAME }}` syntax in HTTP request URLs and headers
- So that I don't have to hardcode API endpoints and authentication tokens

**US-3: Use Variables in Email Templates**
- As a workflow designer
- I want to reference variables in email subject and body templates
- So that I can customize emails with environment-specific values

**US-4: Secure Sensitive Variables**
- As a security-conscious administrator
- I want to mark certain variables as "secret" so their values are masked in the UI
- So that sensitive credentials are not accidentally exposed

**US-5: Update Variables Without Workflow Changes**
- As an operations engineer
- I want to update variable values in Settings without modifying workflows
- So that I can rotate credentials or change configurations easily

### 2.2 Secondary User Stories

**US-6: View Variable Usage**
- As a workflow administrator
- I want to see which workflows use a specific variable
- So that I understand the impact of changing or deleting it

**US-7: Validate Variable References**
- As a workflow designer
- I want to be warned when I reference undefined variables
- So that I can catch configuration errors before runtime

---

## 3. Functional Requirements

### 3.1 Variable Management (Settings UI)

#### 3.1.1 Variable List View
- **FR-1.1**: Display a table/list of all defined variables
- **FR-1.2**: Show columns: Name, Type (Text/Secret), Description, Last Modified, Actions
- **FR-1.3**: Support search/filter functionality for variables
- **FR-1.4**: Display usage count (number of workflows using the variable)
- **FR-1.5**: Provide "Add Variable" button

#### 3.1.2 Create Variable
- **FR-2.1**: Variable name must follow naming convention: `[A-Z0-9_]+` (uppercase, numbers, underscores)
- **FR-2.2**: Variable name must be unique across all variables
- **FR-2.3**: Support variable types:
  - `Text`: Regular configuration value (visible in UI)
  - `Secret`: Sensitive value (masked in UI after creation)
- **FR-2.4**: Required fields: Name, Type, Value
- **FR-2.5**: Optional fields: Description (up to 500 characters)
- **FR-2.6**: Validate variable name format before creation
- **FR-2.7**: Show real-time preview of template usage: `{{ env.VARIABLE_NAME }}`

#### 3.1.3 Edit Variable
- **FR-3.1**: Allow editing variable value and description
- **FR-3.2**: Variable name cannot be changed after creation (to prevent breaking workflows)
- **FR-3.3**: For secrets, show masked value with "Show/Hide" toggle
- **FR-3.4**: Show "Update" and "Cancel" buttons
- **FR-3.5**: Display warning if variable is used in workflows

#### 3.1.4 Delete Variable
- **FR-4.1**: Show confirmation dialog before deletion
- **FR-4.2**: Display warning if variable is currently used in any workflows
- **FR-4.3**: List workflows that use the variable in confirmation dialog
- **FR-4.4**: Prevent deletion if variable is used (with option to force delete)

### 3.2 Variable Interpolation (Template Engine)

#### 3.2.1 Template Syntax
- **FR-5.1**: Use Handlebars-style syntax: `{{ env.VARIABLE_NAME }}`
- **FR-5.2**: Support variable references in:
  - HTTP Request: URL, Headers (values)
  - Email: Subject, Body Template, Text Body Template
  - Transformer: Initial input (optionally)
  - Anthropic: System Prompt, User Prompt
  - OpenObserve: URL, Authorization Header
  - Delay: (no interpolation needed)
- **FR-5.3**: Support nested/complex templates: `https://{{ env.API_HOST }}/api/v1/{{ env.RESOURCE }}`
- **FR-5.4**: Escape special characters in variable values during interpolation
- **FR-5.5**: Fail workflow execution if referenced variable is undefined (with clear error message)

#### 3.2.2 Variable Resolution
- **FR-6.1**: Variables are resolved at workflow execution time (not design time)
- **FR-6.2**: Variables must be available throughout entire workflow execution
- **FR-6.3**: Variable values are immutable during a single workflow execution
- **FR-6.4**: Log warning if template contains undefined variable reference

#### 3.2.3 UI Indicators
- **FR-7.1**: Highlight `{{ env.VARIABLE_NAME }}` syntax in code editors (Monaco/text inputs)
- **FR-7.2**: Show autocomplete suggestions for available variables when typing `{{ env.`
- **FR-7.3**: Show tooltip on hover with variable description and current value (masked for secrets)
- **FR-7.4**: Indicate undefined variables with red underline/warning icon

### 3.3 Security Requirements

#### 3.3.1 Secret Storage
- **FR-8.1**: Secret variables must be encrypted at rest in the database
- **FR-8.2**: Use AES-256-GCM encryption for secret values
- **FR-8.3**: Store encryption key separately from database (environment variable or key management)
- **FR-8.4**: Secrets are decrypted only during workflow execution (not at design time)

#### 3.3.2 Secret Display
- **FR-9.1**: Mask secret values in UI as `••••••••` after creation
- **FR-9.2**: Provide "Show" button to temporarily reveal secret value
- **FR-9.3**: Secret values should not appear in workflow execution logs
- **FR-9.4**: Redact secret values in error messages (replace with `[REDACTED]`)

#### 3.3.3 Access Control
- **FR-10.1**: Only authenticated admin users can view/edit variables
- **FR-10.2**: API endpoints for variables require admin authentication
- **FR-10.3**: Audit log all variable create/update/delete operations

### 3.4 Database Schema

#### 3.4.1 Variables Table
```sql
CREATE TABLE environment_variables (
    id VARCHAR(36) PRIMARY KEY,
    name VARCHAR(255) UNIQUE NOT NULL,
    value_type VARCHAR(20) NOT NULL, -- 'text' or 'secret'
    value TEXT NOT NULL, -- Encrypted for secrets
    description TEXT,
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL,
    created_by VARCHAR(255), -- Future: user tracking
    INDEX idx_name (name)
);
```

### 3.5 API Endpoints

#### 3.5.1 Variable Management APIs
```
GET    /api/admin/v1/variables              # List all variables (secrets masked)
GET    /api/admin/v1/variables/:id          # Get variable details
POST   /api/admin/v1/variables              # Create new variable
PUT    /api/admin/v1/variables/:id          # Update variable
DELETE /api/admin/v1/variables/:id          # Delete variable
POST   /api/admin/v1/variables/validate     # Validate variable name format
GET    /api/admin/v1/variables/:id/usage    # Get workflows using this variable
```

#### 3.5.2 Request/Response Formats

**Create Variable Request:**
```json
{
  "name": "API_KEY_OPENAI",
  "value_type": "secret",
  "value": "sk-proj-abc123...",
  "description": "OpenAI API key for AI features"
}
```

**List Variables Response:**
```json
{
  "variables": [
    {
      "id": "uuid-1",
      "name": "API_KEY_OPENAI",
      "value_type": "secret",
      "value": "••••••••",
      "description": "OpenAI API key for AI features",
      "usage_count": 3,
      "created_at": 1234567890,
      "updated_at": 1234567890
    },
    {
      "id": "uuid-2",
      "name": "API_HOST",
      "value_type": "text",
      "value": "https://api.example.com",
      "description": "Production API host",
      "usage_count": 5,
      "created_at": 1234567890,
      "updated_at": 1234567890
    }
  ]
}
```

---

## 4. Technical Architecture

### 4.1 Backend Components

#### 4.1.1 Variable Service (`src/variables/`)
- `service.rs`: CRUD operations for variables
- `encryption.rs`: Encryption/decryption for secret values
- `template_engine.rs`: Variable interpolation logic using Handlebars
- `validation.rs`: Variable name validation

#### 4.1.2 Template Engine Integration
- Use `handlebars` crate for template rendering
- Register custom helper: `{{ env.VARIABLE_NAME }}`
- Integrate into workflow execution pipeline
- Pre-load all variables into Handlebars context before execution

#### 4.1.3 Encryption Service
- Use `aes-gcm` crate for encryption
- Load encryption key from `SP_ENCRYPTION_KEY` environment variable
- Generate key if not provided (warn in logs for production)
- Implement key rotation support (Phase 2)

### 4.2 Frontend Components

#### 4.2.1 Variables Settings Page (`frontend/src/views/VariablesView.vue`)
- List/table view of all variables
- Create/Edit/Delete modals
- Search and filter functionality
- Usage indicator per variable

#### 4.2.2 Variable Editor Component (`frontend/src/components/VariableEditor.vue`)
- Form for creating/editing variables
- Real-time validation
- Secret visibility toggle
- Template preview

#### 4.2.3 Template Input Component (`frontend/src/components/common/TemplateInput.vue`)
- Monaco editor with syntax highlighting for `{{ env.* }}`
- Autocomplete for variable names
- Validation indicators for undefined variables
- Inline variable value preview (on hover)

#### 4.2.4 Store Updates
- Add `variables` store (`frontend/src/stores/variables.ts`)
- Fetch variables on app initialization
- Cache variables in memory for autocomplete

### 4.3 Integration Points

#### 4.3.1 Workflow Execution
```rust
// Pseudo-code flow
1. Load all environment variables from database
2. Build Handlebars context with variables
3. For each node execution:
   a. Resolve templates in node configuration (URL, headers, etc.)
   b. Replace {{ env.VARIABLE_NAME }} with actual values
   c. Execute node with resolved values
4. Handle template resolution errors gracefully
```

#### 4.3.2 Node Configuration Updates
Update node config structs to indicate which fields support templating:
- `HttpRequestConfig`: `url`, `headers` values
- `EmailConfig`: `subject`, `body_template`, `text_body_template`
- `AnthropicConfig`: `system_prompt`, `user_prompt`
- `OpenObserveConfig`: `url`, `authorization_header`

---

## 5. User Interface Design

### 5.1 Settings Menu Update
Add "Environment Variables" menu item in Settings navigation:
```
Settings
├── General
├── Email Configuration
├── Environment Variables  ← NEW
└── API Documentation
```

### 5.2 Variables List Page

```
┌─────────────────────────────────────────────────────────────────┐
│ Environment Variables                                    [+ Add] │
├─────────────────────────────────────────────────────────────────┤
│ [Search variables...]                                    🔍      │
├──────────┬──────────┬────────────────────────┬─────────┬────────┤
│ Name     │ Type     │ Description            │ Usage   │ Actions│
├──────────┼──────────┼────────────────────────┼─────────┼────────┤
│ API_HOST │ Text     │ Production API host    │ 5 flows │ ✏️ 🗑️  │
│ API_KEY  │ Secret   │ API authentication key │ 3 flows │ ✏️ 🗑️  │
│ DB_URL   │ Secret   │ Database connection    │ 1 flow  │ ✏️ 🗑️  │
└──────────┴──────────┴────────────────────────┴─────────┴────────┘
```

### 5.3 Add/Edit Variable Modal

```
┌───────────────────────────────────────────────┐
│ Add Environment Variable              [✕]     │
├───────────────────────────────────────────────┤
│ Variable Name*                                │
│ ┌───────────────────────────────────────────┐ │
│ │ API_KEY_STRIPE                           │ │
│ └───────────────────────────────────────────┘ │
│ Template: {{ env.API_KEY_STRIPE }}            │
│                                               │
│ Type*                                         │
│ ⦿ Text     ◯ Secret                          │
│                                               │
│ Value*                                        │
│ ┌───────────────────────────────────────────┐ │
│ │ sk_test_abc123...              [👁️ Show] │ │
│ └───────────────────────────────────────────┘ │
│                                               │
│ Description                                   │
│ ┌───────────────────────────────────────────┐ │
│ │ Stripe API key for payment processing    │ │
│ └───────────────────────────────────────────┘ │
│                                               │
│                      [Cancel]  [Save Variable]│
└───────────────────────────────────────────────┘
```

### 5.4 Node Configuration with Template Support

Example: HTTP Request Node
```
┌────────────────────────────────────────────────┐
│ URL*                                           │
│ ┌────────────────────────────────────────────┐ │
│ │ {{ env.API_HOST }}/users/{{ data.userId }} │ │ ← Template highlighting
│ └────────────────────────────────────────────┘ │
│                                                │
│ Headers                                        │
│ ┌─────────────┬──────────────────────────────┐ │
│ │ Key         │ Value                        │ │
│ ├─────────────┼──────────────────────────────┤ │
│ │ Authorization│ Bearer {{ env.API_TOKEN }}  │ │ ← Template highlighting
│ │ Content-Type│ application/json             │ │
│ └─────────────┴──────────────────────────────┘ │
└────────────────────────────────────────────────┘
```

---

## 6. Error Handling

### 6.1 Variable Resolution Errors

**Error Case 1: Undefined Variable**
- **Scenario**: Workflow references `{{ env.UNKNOWN_VAR }}`
- **Behavior**:
  - Fail workflow execution immediately
  - Set execution status to "failed"
  - Error message: "Variable 'UNKNOWN_VAR' is not defined in environment variables"
  - Log error with node ID and variable name

**Error Case 2: Invalid Template Syntax**
- **Scenario**: Malformed template `{{ env.API_KEY`
- **Behavior**:
  - Fail workflow execution
  - Error message: "Invalid template syntax in [node_name]: {{ env.API_KEY"
  - Suggest correct syntax

**Error Case 3: Encryption Key Missing**
- **Scenario**: Secret variable exists but encryption key not configured
- **Behavior**:
  - Fail application startup
  - Log critical error: "SP_ENCRYPTION_KEY environment variable not set"
  - Refuse to start application

### 6.2 UI Validation Errors

**Validation 1: Invalid Variable Name**
- Show error: "Variable name must contain only uppercase letters, numbers, and underscores"
- Examples: `API_KEY` ✓, `api-key` ✗, `API KEY` ✗

**Validation 2: Duplicate Variable Name**
- Show error: "Variable 'API_KEY' already exists"

**Validation 3: Empty Required Fields**
- Disable "Save" button until all required fields are filled

---

## 7. Testing Requirements

### 7.1 Unit Tests
- Variable CRUD operations
- Encryption/decryption functionality
- Template parsing and variable resolution
- Variable name validation
- Usage tracking

### 7.2 Integration Tests
- Workflow execution with variable interpolation
- Multiple variables in single template
- Nested variable references
- Secret masking in logs and error messages
- Variable updates reflected in subsequent executions

### 7.3 E2E Tests
- Create variable via UI → Use in workflow → Execute → Verify
- Update variable → Execute workflow → Verify new value used
- Delete unused variable → Verify deletion
- Attempt to delete used variable → Verify warning/prevention

### 7.4 Security Tests
- Verify secrets are encrypted in database
- Verify secrets are masked in UI
- Verify secrets don't appear in logs
- Verify API authentication requirements

---

## 8. Migration & Deployment

### 8.1 Database Migration
- Create `environment_variables` table
- Add indexes for performance
- Generate encryption key if not exists (with warning)

### 8.2 Backward Compatibility
- Existing workflows without variables continue to work
- Template syntax is opt-in (workflows without `{{ }}` are unaffected)
- No breaking changes to existing APIs

### 8.3 Configuration
New environment variables:
- `SP_ENCRYPTION_KEY`: Required for secret encryption (32-byte hex string)
- Auto-generate and warn if not provided in development
- Require explicit configuration in production

---

## 9. Success Metrics

### 9.1 Adoption Metrics
- Number of variables created per workspace
- Percentage of workflows using variables
- Average number of variables per workflow

### 9.2 Quality Metrics
- Variable resolution error rate (should be < 1%)
- API response time for variable operations (< 100ms p95)
- Zero security incidents related to secret exposure

### 9.3 Usability Metrics
- Time to create first variable (target: < 30 seconds)
- User satisfaction with variable management (target: > 4/5)

---

## 10. Future Enhancements (Phase 2)

### 10.1 Advanced Features
- **Variable Groups/Namespaces**: Organize variables by environment (dev/staging/prod)
- **Variable Scoping**: Limit variable access by workflow, user, or team
- **Variable Versioning**: Track history of variable value changes
- **Import/Export**: Bulk import variables from JSON/ENV files
- **External Secret Providers**: Integration with HashiCorp Vault, AWS Secrets Manager

### 10.2 Template Enhancements
- **Conditional Templates**: `{{ #if env.FEATURE_FLAG }}...{{ /if }}`
- **Default Values**: `{{ env.API_KEY | default: "fallback" }}`
- **Transformation Helpers**: `{{ env.API_KEY | base64 }}`, `{{ env.URL | uppercase }}`

### 10.3 Developer Experience
- **CLI Tool**: Manage variables via command line
- **Workflow Testing**: Test workflows with mock variable values
- **Variable Validation**: Type checking for variables (string, number, URL, etc.)

---

## 11. Open Questions

1. **Q**: Should we support per-workflow variable overrides?
   **A**: Not in Phase 1. Consider for Phase 2 based on user feedback.

2. **Q**: How to handle key rotation for encrypted secrets?
   **A**: Phase 2 feature. For now, require manual re-entry of secrets if key changes.

3. **Q**: Should variables be environment-specific (dev/staging/prod)?
   **A**: Not in Phase 1. Single global namespace. Consider namespaces in Phase 2.

4. **Q**: Should we support variable references in JavaScript code (transformers/conditions)?
   **A**: Not in Phase 1. JavaScript can access workflow event data. Variables are for configuration only.

---

## 12. Appendices

### 12.1 Example Use Cases

**Use Case 1: Multi-Environment API Configuration**
```
Variables:
- API_HOST = https://api.production.com
- API_KEY = sk_prod_abc123...
- TIMEOUT_SECONDS = 30

HTTP Request Node:
- URL: {{ env.API_HOST }}/users
- Headers: { "Authorization": "Bearer {{ env.API_KEY }}" }
- Timeout: {{ env.TIMEOUT_SECONDS }}
```

**Use Case 2: Email Branding**
```
Variables:
- COMPANY_NAME = Acme Corp
- SUPPORT_EMAIL = support@acme.com
- LOGO_URL = https://cdn.acme.com/logo.png

Email Node:
- Subject: Update from {{ env.COMPANY_NAME }}
- Body:
  <img src="{{ env.LOGO_URL }}" />
  <p>Contact us at {{ env.SUPPORT_EMAIL }}</p>
```

**Use Case 3: Feature Flags**
```
Variables:
- ENABLE_ADVANCED_ANALYTICS = true
- ENABLE_EMAIL_NOTIFICATIONS = false

(Future: Use in conditional logic)
```

### 12.2 Security Considerations

1. **Encryption at Rest**: All secret variables encrypted with AES-256-GCM
2. **Encryption Key Storage**: Store in environment variable, never in code/database
3. **Access Control**: Admin-only access to variable management
4. **Audit Logging**: Log all variable modifications with timestamp and user
5. **Secret Redaction**: Mask secrets in all logs and error messages
6. **HTTPS Only**: Enforce HTTPS for all API calls involving variables
7. **Rate Limiting**: Implement rate limits on variable APIs to prevent brute force

### 12.3 Implementation Phases

**Phase 1 (MVP)**:
- ✅ Variable CRUD in Settings
- ✅ Template interpolation in HTTP & Email nodes
- ✅ Secret encryption
- ✅ Basic UI with masked secrets

**Phase 2 (Enhanced)**:
- Variable usage tracking
- Autocomplete in editors
- Syntax highlighting
- Usage warnings on delete

**Phase 3 (Advanced)**:
- Variable namespaces/environments
- External secret provider integration
- Advanced templating (conditionals, transforms)
- Workflow testing with mock variables

---

**Document Version**: 1.0
**Last Updated**: 2025-01-30
**Status**: Draft - Ready for Review
**Authors**: Product Team
**Reviewers**: Engineering, Security, Product Management
