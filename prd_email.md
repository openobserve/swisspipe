# Email Node Type - Product Requirements Document

## Overview

This PRD outlines the implementation of an "Email" node type for SwissPipe workflows that enables sending emails via SMTP. The email node will support template-based content, dynamic data substitution, and flexible SMTP configuration.

## Problem Statement

**Current Gap:**
- SwissPipe workflows cannot send email notifications or reports
- No way to communicate workflow results to external stakeholders
- Missing critical communication capability for automation workflows

**Business Need:**
- Send workflow completion notifications
- Deliver reports and data exports via email
- Alert on workflow failures or important events
- Integrate with existing email infrastructure

## Success Metrics

- Email node successfully sends emails via SMTP within 30 seconds
- Support for HTML and plain text email formats
- Dynamic template rendering with workflow data
- 99%+ email delivery success rate
- Secure credential management via environment variables

## Requirements

### 1. Backend Email Node Implementation

#### 1.1 Node Configuration Schema

**Email Node Configuration:**
```json
{
  "type": "email",
  "id": "email-001",
  "name": "Send Notification Email",
  "config": {
    "smtp_config": "default", // References env config or can be "custom"
    "from": {
      "email": "noreply@company.com",
      "name": "SwissPipe Workflow" // Optional display name
    },
    "to": [
      {
        "email": "{{ workflow.data.user_email }}",
        "name": "{{ workflow.data.user_name }}" // Optional
      },
      {
        "email": "admin@company.com",
        "name": "Admin Team"
      }
    ],
    "cc": [ // Optional
      {
        "email": "manager@company.com", 
        "name": "Manager"
      }
    ],
    "bcc": [ // Optional
      {
        "email": "audit@company.com"
      }
    ],
    "subject": "Workflow {{ workflow.name }} completed - {{ workflow.status }}",
    "template_type": "html", // "html" or "text"
    "body_template": "<!DOCTYPE html><html><body><h1>Workflow Results</h1><p>Status: {{ workflow.status }}</p><p>Data: {{ workflow.data | json }}</p></body></html>",
    "text_body_template": "Workflow Results\nStatus: {{ workflow.status }}\nData: {{ workflow.data | json }}", // Optional fallback
    "attachments": [ // Optional
      {
        "filename": "report.json",
        "content_type": "application/json",
        "data": "{{ workflow.output | json }}" // Base64 encoded or template
      }
    ],
    "priority": "normal", // "low", "normal", "high"
    "delivery_receipt": false,
    "read_receipt": false
  }
}
```

#### 1.2 SMTP Configuration via Environment Variables

**Primary SMTP Configuration (default):**
```bash
# Primary SMTP server configuration
SMTP_HOST=smtp.gmail.com
SMTP_PORT=587
SMTP_SECURITY=tls  # "none", "tls", "ssl"
SMTP_USERNAME=notifications@company.com
SMTP_PASSWORD=app-password-here
SMTP_FROM_EMAIL=noreply@company.com
SMTP_FROM_NAME="SwissPipe Notifications"

# Connection settings
SMTP_TIMEOUT_SECONDS=30
SMTP_CONNECTION_POOL_SIZE=5
SMTP_MAX_RETRIES=3
SMTP_RETRY_DELAY_SECONDS=5
```

**Multiple SMTP Configurations (optional):**
```bash
# Alternative SMTP configurations
SMTP_MARKETING_HOST=smtp.sendgrid.com
SMTP_MARKETING_PORT=587
SMTP_MARKETING_SECURITY=tls
SMTP_MARKETING_USERNAME=apikey
SMTP_MARKETING_PASSWORD=sendgrid-api-key
SMTP_MARKETING_FROM_EMAIL=marketing@company.com

SMTP_ALERTS_HOST=smtp.mailgun.org
SMTP_ALERTS_PORT=587
SMTP_ALERTS_SECURITY=tls
SMTP_ALERTS_USERNAME=postmaster@mg.company.com
SMTP_ALERTS_PASSWORD=mailgun-password
SMTP_ALERTS_FROM_EMAIL=alerts@company.com
```

#### 1.3 Email Service Implementation

**Core Email Service Structure:**
```rust
pub struct EmailService {
    smtp_configs: HashMap<String, SmtpConfig>,
    template_engine: TemplateEngine,
    connection_pool: SmtpConnectionPool,
}

pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub security: SmtpSecurity, // None, Tls, Ssl
    pub username: Option<String>,
    pub password: Option<String>,
    pub from_email: String,
    pub from_name: Option<String>,
    pub timeout_seconds: u64,
    pub max_retries: u32,
    pub retry_delay_seconds: u64,
}

pub struct EmailMessage {
    pub from: EmailAddress,
    pub to: Vec<EmailAddress>,
    pub cc: Vec<EmailAddress>,
    pub bcc: Vec<EmailAddress>,
    pub subject: String,
    pub html_body: Option<String>,
    pub text_body: Option<String>,
    pub attachments: Vec<EmailAttachment>,
    pub priority: EmailPriority,
    pub delivery_receipt: bool,
    pub read_receipt: bool,
}

pub struct EmailAddress {
    pub email: String,
    pub name: Option<String>,
}

pub struct EmailAttachment {
    pub filename: String,
    pub content_type: String,
    pub data: Vec<u8>,
}
```

**Acceptance Criteria:**
- ✅ Load SMTP configurations from environment variables on startup
- ✅ Support multiple named SMTP configurations (default, marketing, alerts, etc.)
- ✅ Secure password/token storage and handling
- ✅ Connection pooling for SMTP connections
- ✅ Retry mechanism with exponential backoff for failed sends
- ✅ Template rendering with Handlebars or similar engine
- ✅ Support for HTML and plain text email bodies
- ✅ File attachment support with proper MIME encoding
- ✅ Email validation and sanitization
- ✅ Comprehensive error handling and logging

#### 1.4 Template Engine Integration

**Template Features:**
- Dynamic data substitution using workflow context
- Support for loops, conditionals, and filters
- JSON formatting helpers
- Date/time formatting
- HTML escaping for security
- Fallback to plain text if HTML rendering fails

**Template Context Variables:**
```json
{
  "workflow": {
    "id": "execution-id-123",
    "name": "Data Processing Workflow",
    "status": "completed",
    "started_at": "2025-01-15T10:30:00Z",
    "completed_at": "2025-01-15T10:35:00Z",
    "data": { "user_id": 123, "processed_items": 456 },
    "output": { "results": [...] },
    "error": null
  },
  "node": {
    "id": "email-001",
    "name": "Send Notification Email",
    "previous_outputs": { "node1": {...}, "node2": {...} }
  },
  "system": {
    "timestamp": "2025-01-15T10:35:00Z",
    "hostname": "swisspipe-prod-1"
  }
}
```

### 2. Frontend Email Node Editor

#### 2.1 Node Configuration UI

**Email Node Editor Components:**

1. **SMTP Configuration Selector**
   - Dropdown to select SMTP config: "default", "marketing", "alerts", "custom"
   - Read-only display of selected SMTP server details (host, port, security)
   - Option to test SMTP connection

2. **Email Recipients Section**
   - **From Address**: Email and optional display name fields
   - **To Recipients**: Dynamic list with email/name pairs, template support
   - **CC Recipients**: Optional, collapsible section
   - **BCC Recipients**: Optional, collapsible section
   - Template syntax helper with available variables

3. **Email Content Section**
   - **Subject Line**: Text input with template support
   - **Content Type**: Toggle between "HTML" and "Plain Text"
   - **Body Template**: 
     - Vue-based rich text editor (vue-email-editor) for HTML mode with template variable insertion
     - Monaco editor integration (@monaco-editor/vue) for plain text mode
     - Live preview component with reactive sample data
     - Vue.js reactive template validation and syntax highlighting
   - **Attachments**: File upload or dynamic content specification

4. **Email Options Section**
   - Priority selector: Low, Normal, High
   - Delivery receipt checkbox
   - Read receipt checkbox

5. **Template Variables Panel**
   - Searchable list of available template variables
   - Click to insert into templates
   - Examples and documentation

**Acceptance Criteria:**
- ✅ Intuitive drag-and-drop email node creation
- ✅ Real-time template syntax validation
- ✅ Live preview of email content with sample data
- ✅ SMTP connection testing from UI
- ✅ Template variable auto-completion
- ✅ Responsive design for various screen sizes
- ✅ Comprehensive field validation and error messages

#### 2.2 Email Node Visual Design

**Node Appearance:**
- **Icon**: Envelope/mail icon with distinctive color
- **Node Color**: Professional blue gradient (#2196F3 to #1976D2)
- **Status Indicators**: 
  - Green checkmark for successful sends
  - Red X for failed sends
  - Orange warning for partial failures (some recipients failed)
  - Loading spinner during send operation

**Connection Points:**
- **Input**: Single input for workflow data
- **Output**: 
  - Success output (green) - when all emails sent successfully  
  - Error output (red) - when email sending fails
  - Data passthrough - forwards input data to next nodes

### 3. Email Node Execution Logic

#### 3.1 Execution Flow

```
1. Receive workflow data from previous node
2. Validate email node configuration
3. Render email templates with workflow context
4. Validate rendered email addresses and content
5. Connect to configured SMTP server
6. Send email with retry mechanism
7. Log delivery status and any errors
8. Return execution result to workflow engine
```

#### 3.2 Error Handling

**Error Categories:**
- **Configuration Errors**: Invalid SMTP settings, missing templates
- **Template Errors**: Template rendering failures, missing variables
- **Validation Errors**: Invalid email addresses, content too large
- **SMTP Errors**: Connection failures, authentication issues
- **Delivery Errors**: Recipient rejections, temporary failures

**Error Response Structure:**
```json
{
  "success": false,
  "error": {
    "type": "smtp_connection_error",
    "message": "Failed to connect to SMTP server: Connection timeout",
    "details": {
      "smtp_host": "smtp.example.com",
      "smtp_port": 587,
      "timeout_seconds": 30
    },
    "retry_possible": true
  },
  "partial_success": {
    "successful_recipients": ["user1@example.com"],
    "failed_recipients": [
      {
        "email": "invalid@domain.invalid",
        "error": "Invalid recipient address"
      }
    ]
  }
}
```

### 4. Security & Compliance

#### 4.1 Security Measures

**Credential Security:**
- SMTP passwords stored as environment variables only
- No credentials logged or exposed in error messages  
- Secure connection handling (TLS/SSL support)
- Input sanitization to prevent injection attacks

**Email Security:**
- HTML template sanitization to prevent XSS
- Email address validation and sanitization
- Rate limiting to prevent spam/abuse
- Attachment size and type restrictions

**Compliance Features:**
- Optional email tracking and delivery receipts
- Audit logging of all email sends
- Data retention policies for email logs
- GDPR-compliant data handling

#### 4.2 Environment Variable Security

```bash
# Required environment variables (no defaults for security)
SMTP_HOST=smtp.example.com
SMTP_PORT=587
SMTP_SECURITY=tls
SMTP_USERNAME=user@example.com
SMTP_PASSWORD=secure-password-here

# Optional security settings
SMTP_ALLOWED_DOMAINS="example.com,company.org"  # Restrict recipient domains
SMTP_MAX_RECIPIENTS=50                          # Prevent bulk spam
SMTP_MAX_ATTACHMENT_SIZE_MB=25                  # Attachment size limit
SMTP_RATE_LIMIT_PER_MINUTE=60                  # Rate limiting
```

### 4.3 Rate Limiting & Queue Management

**Problem Statement:**
When workflows send more emails than the configured rate limit allows, the system needs to handle the excess gracefully without failing workflows or violating SMTP provider limits.

**Rate Limiting Strategy:**
SwissPipe implements a **Queue + Background Processor** approach for optimal workflow performance and SMTP compliance.

#### **Implementation Approach:**

**1. Token Bucket Rate Limiter:**
```rust
pub struct EmailRateLimiter {
    tokens: u32,
    capacity: u32,
    refill_rate: u32,          // tokens per minute
    last_refill: Instant,
}
```

**2. Database-Backed Email Queue System:**
```rust
pub struct EmailQueueService {
    db: Arc<DatabaseConnection>,
    rate_limiter: EmailRateLimiter,
}

// Queue management via database operations
impl EmailQueueService {
    pub async fn enqueue_email(&self, email: QueuedEmailRequest) -> Result<String, EmailError>;
    pub async fn dequeue_next_email(&self) -> Result<Option<QueuedEmail>, EmailError>;
    pub async fn mark_processing(&self, id: &str) -> Result<(), EmailError>;
    pub async fn mark_sent(&self, id: &str, smtp_message_id: &str) -> Result<(), EmailError>;
    pub async fn mark_failed(&self, id: &str, error: &str) -> Result<(), EmailError>;
    pub async fn cleanup_expired(&self) -> Result<u32, EmailError>;
}
```

**3. Database-Driven Queue Processor:**
- Dedicated tokio task queries database for pending emails
- Processes emails in priority order using SQL `ORDER BY priority DESC, queued_at ASC`
- Uses database transactions for atomic queue operations
- Automatic cleanup of expired/failed emails via scheduled queries
- Persistent queue survives application restarts

#### **Queue Behavior Options:**

**Node Configuration:**
```json
{
  "type": "email",
  "config": {
    "priority": "normal",              // "critical", "high", "normal", "low"
    "queue_if_rate_limited": true,     // Queue or fail immediately
    "max_queue_wait_minutes": 30,      // Max time to wait in queue
    "bypass_rate_limit": false         // Only for critical system emails
  }
}
```

**Database Queue Flow:**
1. **Email Request Received** → Check rate limiter tokens
2. **Tokens Available** → Send immediately, consume token
3. **Rate Limited + Queue Enabled** → Insert to `email_queue` table with priority, return success
4. **Rate Limited + Queue Disabled** → Return error immediately
5. **Background Processor** → Polls database for queued emails using optimized queries:
   ```sql
   SELECT * FROM email_queue 
   WHERE status = 'queued' 
     AND (scheduled_at IS NULL OR scheduled_at <= unixepoch('subsec') * 1000000)
   ORDER BY priority DESC, queued_at ASC 
   LIMIT 1
   ```

#### **Environment Variables:**

```bash
# Rate Limiting Configuration
SMTP_RATE_LIMIT_PER_MINUTE=60         # Max emails per minute
SMTP_BURST_LIMIT=10                   # Allow short bursts above rate limit
SMTP_QUEUE_MAX_SIZE=1000              # Maximum queued emails
SMTP_QUEUE_TIMEOUT_MINUTES=60         # How long emails stay in queue
SMTP_QUEUE_CLEANUP_INTERVAL_MINUTES=5 # How often to clean expired emails

# Priority Settings
SMTP_PRIORITY_BYPASS_RATE_LIMIT=false # Allow critical emails to bypass limits
SMTP_CRITICAL_BURST_LIMIT=5          # Additional burst for critical emails
```

#### **Priority Levels:**

| Priority | Behavior | Use Case |
|----------|----------|----------|
| **Critical** | Can bypass rate limits (if configured) | System alerts, security notifications |
| **High** | Front of queue, processed first | Important business emails |
| **Normal** | Standard queue position | Regular workflow notifications |
| **Low** | Back of queue, processed last | Bulk notifications, reports |

#### **Error Handling:**

**Queue Full:**
```json
{
  "success": false,
  "error": "Email queue is full. Try again later or reduce email volume.",
  "queue_stats": {
    "current_size": 1000,
    "max_size": 1000,
    "estimated_wait_minutes": 45
  }
}
```

**Queue Timeout:**
```json
{
  "success": false,
  "error": "Email expired in queue after 60 minutes without sending.",
  "queue_wait_minutes": 60
}
```

#### **Monitoring & Observability:**

**Database Queue Statistics Endpoint:**
```http
GET /api/v1/email/queue/stats

{
  "queue_size": 23,
  "rate_limit_per_minute": 60,
  "tokens_available": 15,
  "next_refill_seconds": 30,
  "priority_breakdown": {
    "critical": 0,
    "high": 3,
    "normal": 15,
    "low": 5
  },
  "average_wait_minutes": 12,
  "emails_sent_last_minute": 45,
  "emails_queued_last_minute": 67,
  "processing_count": 2,
  "failed_last_hour": 3,
  "expired_last_hour": 1
}
```

**Database Queries for Statistics:**
```sql
-- Queue size by priority
SELECT priority, COUNT(*) as count 
FROM email_queue 
WHERE status = 'queued' 
GROUP BY priority;

-- Average wait time
SELECT AVG((unixepoch('subsec') * 1000000 - queued_at) / 60000000.0) as avg_wait_minutes
FROM email_queue 
WHERE status = 'processing' OR status = 'queued';

-- Recent activity counts
SELECT 
  COUNT(CASE WHEN status = 'sent' AND sent_at > (unixepoch('subsec') - 60) * 1000000 THEN 1 END) as sent_last_minute,
  COUNT(CASE WHEN queued_at > (unixepoch('subsec') - 60) * 1000000 THEN 1 END) as queued_last_minute
FROM email_queue;
```

**Database-Backed Metrics & Logging:**
- Real-time queue depth metrics via database queries
- Email send rate tracking through `email_audit_log` table  
- Rate limit hit frequency from queue insertion patterns
- Queue timeout detection via scheduled cleanup queries
- Per-priority queue performance tracking with SQL analytics
- Database-driven alerting on queue overflow conditions

#### **Acceptance Criteria:**

- ✅ **Rate limiting never causes workflow failures**
- ✅ **Database-backed email queue handles bursts up to configured limits**
- ✅ **Priority emails are processed first via SQL ORDER BY**
- ✅ **Real-time queue statistics available via database queries**
- ✅ **Scheduled database cleanup of expired queued emails**
- ✅ **Configurable rate limits and queue behavior via environment variables**
- ✅ **Graceful degradation when database queue reaches capacity limits**
- ✅ **Background database polling doesn't impact workflow performance**
- ✅ **Queue persistence survives application restarts**
- ✅ **Atomic queue operations using database transactions**

## Technical Implementation

### 5. Database Schema Updates

**Email Audit Log Table:**
```sql
CREATE TABLE email_audit_log (
    id TEXT PRIMARY KEY, -- UUIDv7
    execution_id TEXT NOT NULL,
    node_id TEXT NOT NULL,
    smtp_config TEXT NOT NULL, -- which SMTP config was used
    from_email TEXT NOT NULL,
    to_emails TEXT NOT NULL, -- JSON array
    cc_emails TEXT, -- JSON array, optional
    bcc_emails TEXT, -- JSON array, optional
    subject TEXT NOT NULL,
    email_size_bytes INTEGER NOT NULL,
    attachment_count INTEGER DEFAULT 0,
    status TEXT NOT NULL, -- 'sent', 'failed', 'partial'
    error_message TEXT, -- if failed
    smtp_message_id TEXT, -- SMTP server response ID
    sent_at INTEGER, -- Unix epoch microseconds
    created_at INTEGER DEFAULT (unixepoch('subsec') * 1000000),
    FOREIGN KEY (execution_id) REFERENCES workflow_executions(id)
);

CREATE INDEX idx_email_audit_execution_id ON email_audit_log(execution_id);
CREATE INDEX idx_email_audit_status ON email_audit_log(status);
CREATE INDEX idx_email_audit_sent_at ON email_audit_log(sent_at);
```

**Email Queue Table:**
```sql
CREATE TABLE email_queue (
    id TEXT PRIMARY KEY, -- UUIDv7
    execution_id TEXT, -- Optional reference to workflow execution
    node_id TEXT, -- Optional reference to email node
    smtp_config TEXT NOT NULL, -- which SMTP config to use
    priority TEXT NOT NULL DEFAULT 'normal', -- 'critical', 'high', 'normal', 'low'
    email_config TEXT NOT NULL, -- JSON of EmailConfig
    template_context TEXT NOT NULL, -- JSON of TemplateContext
    status TEXT NOT NULL DEFAULT 'queued', -- 'queued', 'processing', 'sent', 'failed', 'expired'
    queued_at INTEGER DEFAULT (unixepoch('subsec') * 1000000), -- Unix epoch microseconds
    scheduled_at INTEGER, -- Unix epoch microseconds, when to send (for delayed emails)
    processed_at INTEGER, -- Unix epoch microseconds, when processing started
    sent_at INTEGER, -- Unix epoch microseconds, when email was sent
    max_wait_minutes INTEGER DEFAULT 60, -- Maximum time to wait in queue
    retry_count INTEGER DEFAULT 0, -- Number of send attempts
    max_retries INTEGER DEFAULT 3, -- Maximum retry attempts
    error_message TEXT, -- Last error if failed
    created_at INTEGER DEFAULT (unixepoch('subsec') * 1000000), -- Unix epoch microseconds
    updated_at INTEGER DEFAULT (unixepoch('subsec') * 1000000), -- Unix epoch microseconds
    FOREIGN KEY (execution_id) REFERENCES workflow_executions(id),
    FOREIGN KEY (node_id) REFERENCES nodes(id)
);

-- Performance indices for email queue operations
CREATE INDEX idx_email_queue_status_priority ON email_queue(status, priority DESC, queued_at ASC);
CREATE INDEX idx_email_queue_scheduled_at ON email_queue(scheduled_at);
CREATE INDEX idx_email_queue_execution_id ON email_queue(execution_id);
CREATE INDEX idx_email_queue_queued_at ON email_queue(queued_at);
CREATE INDEX idx_email_queue_status ON email_queue(status);
CREATE INDEX idx_email_queue_priority ON email_queue(priority);
```

### 6. Dependencies & Libraries

**Backend Dependencies:**
```toml
[dependencies]
# Email libraries
lettre = "0.11" # SMTP client library
handlebars = "4.0" # Template engine
mime = "0.3" # MIME type handling
base64 = "0.21" # Attachment encoding

# Validation
validator = "0.16" # Email validation
html-escape = "0.2" # HTML sanitization

# Async/performance
tokio = { version = "1.0", features = ["full"] }

# Rate limiting (in-memory token bucket)
governor = "0.6" # Token bucket rate limiter
tokio-util = { version = "0.7", features = ["time"] }
```

**Frontend Dependencies:**
```json
{
  "dependencies": {
    "@monaco-editor/vue": "^4.5.0",
    "vue-email-editor": "^1.0.0",
    "vue-codemirror": "^6.1.1",
    "handlebars": "^4.7.8",
    "dompurify": "^3.0.0",
    "@vueuse/core": "^10.5.0",
    "vue-draggable-plus": "^0.2.0"
  }
}
```

### 6.1 Vue.js Frontend Implementation Details

**Vue.js Email Node Editor Components:**

```vue
<template>
  <div class="email-node-editor">
    <!-- SMTP Configuration -->
    <EmailSmtpConfig 
      v-model:config="emailConfig.smtp_config"
      @test-connection="testSmtpConnection"
    />
    
    <!-- Recipients Section -->
    <EmailRecipientsSection 
      v-model:from="emailConfig.from"
      v-model:to="emailConfig.to"
      v-model:cc="emailConfig.cc"
      v-model:bcc="emailConfig.bcc"
      :template-variables="availableVariables"
    />
    
    <!-- Email Content Editor -->
    <EmailContentEditor
      v-model:subject="emailConfig.subject"
      v-model:body="emailConfig.body_template"
      v-model:type="emailConfig.template_type"
      :template-variables="availableVariables"
      :preview-data="sampleWorkflowData"
    />
    
    <!-- Live Preview -->
    <EmailPreview
      :config="emailConfig"
      :workflow-data="sampleWorkflowData"
      @refresh="refreshPreview"
    />
  </div>
</template>

<script setup>
import { ref, computed, watch } from 'vue'
import { useVuelidate } from '@vuelidate/core'
import { required, email } from '@vuelidate/validators'

// Email configuration reactive data
const emailConfig = ref({
  smtp_config: 'default',
  from: { email: '', name: '' },
  to: [],
  cc: [],
  bcc: [],
  subject: '',
  body_template: '',
  template_type: 'html'
})

// Template variables available for insertion
const availableVariables = computed(() => [
  'workflow.name',
  'workflow.status', 
  'workflow.data.*',
  'workflow.output.*',
  'node.previous_outputs.*'
])

// Validation rules
const rules = {
  from: { 
    email: { required, email },
    name: { required }
  },
  to: {
    $each: {
      email: { required, email }
    }
  },
  subject: { required },
  body_template: { required }
}

const v$ = useVuelidate(rules, emailConfig)
</script>
```

**Key Vue.js Components:**
- **EmailSmtpConfig.vue**: SMTP configuration selector with connection testing
- **EmailRecipientsSection.vue**: Dynamic recipient management with template support  
- **EmailContentEditor.vue**: Rich text editor with Monaco integration
- **EmailPreview.vue**: Real-time email preview with sample data
- **TemplateVariableHelper.vue**: Searchable variable insertion panel

**Vue.js Reactive Features:**
- Real-time validation with @vuelidate/core
- Reactive template preview updates
- Drag-and-drop recipient management (vue-draggable-plus)
- Auto-completion for template variables using @vueuse/core
- Reactive SMTP connection testing with loading states

## Configuration Examples

### 7. Environment Variable Examples

**Development Configuration:**
```bash
# Development SMTP (using Gmail)
SMTP_HOST=smtp.gmail.com
SMTP_PORT=587
SMTP_SECURITY=tls
SMTP_USERNAME=devteam@company.com
SMTP_PASSWORD=gmail-app-password
SMTP_FROM_EMAIL=dev-swisspipe@company.com
SMTP_FROM_NAME="SwissPipe Development"
SMTP_MAX_RECIPIENTS=10
```

**Production Configuration:**
```bash
# Production SMTP (using corporate mail server)
SMTP_HOST=mail.company.com
SMTP_PORT=465
SMTP_SECURITY=ssl
SMTP_USERNAME=swisspipe@company.com
SMTP_PASSWORD=secure-production-password
SMTP_FROM_EMAIL=notifications@company.com
SMTP_FROM_NAME="SwissPipe Production"
SMTP_CONNECTION_POOL_SIZE=10
SMTP_MAX_RECIPIENTS=100
SMTP_MAX_ATTACHMENT_SIZE_MB=50

# Multiple SMTP configurations for different purposes
SMTP_ALERTS_HOST=smtp.pagerduty.com
SMTP_ALERTS_PORT=587
SMTP_ALERTS_SECURITY=tls
SMTP_ALERTS_USERNAME=alerts@company.com
SMTP_ALERTS_PASSWORD=alert-smtp-password
SMTP_ALERTS_FROM_EMAIL=critical-alerts@company.com
```

## Migration Strategy

### Phase 1: Backend Implementation (Week 1-2)
1. Implement EmailService with SMTP configuration loading
2. Create email node type and execution logic
3. Add template engine integration
4. Implement email audit logging
5. Add comprehensive error handling and retry logic

### Phase 2: Frontend Implementation (Week 3-4)  
1. Design and implement email node editor UI
2. Add SMTP configuration management
3. Create template editor with live preview
4. Implement email node visual design
5. Add template variable helpers and auto-completion

### Phase 3: Testing & Polish (Week 5)
1. End-to-end testing with various SMTP providers
2. Performance testing with high email volumes
3. Security testing and vulnerability assessment
4. Documentation and user guides
5. Production deployment preparation

## Success Criteria

- [ ] **Backend email node successfully sends emails via multiple SMTP providers**
- [ ] **Template engine renders dynamic content with workflow data**
- [ ] **Frontend editor provides intuitive email configuration interface**
- [ ] **SMTP configurations load securely from environment variables**
- [ ] **Email delivery success rate >99% under normal conditions**
- [ ] **Comprehensive error handling for all failure scenarios**
- [ ] **Email audit logging captures all send attempts**
- [ ] **Security measures prevent spam and unauthorized usage**
- [ ] **Performance supports high-volume email sending (100+ emails/minute)**
- [ ] **Complete documentation and examples for users**

## Risks & Mitigation

**Risk:** SMTP provider rate limiting or blocking
**Mitigation:** Implement connection pooling, rate limiting, and multiple SMTP provider support

**Risk:** Email deliverability issues (spam filters)
**Mitigation:** Proper SMTP authentication, SPF/DKIM setup guidance, content best practices

**Risk:** Template injection vulnerabilities  
**Mitigation:** Input sanitization, HTML escaping, template sandboxing

**Risk:** Credential exposure in logs/errors
**Mitigation:** Secure credential handling, redacted error messages, audit logging

**Risk:** Email abuse for spam
**Mitigation:** Rate limiting, recipient validation, audit trails, admin controls

## Timeline

**Estimated Duration:** 5 weeks

- Week 1: Backend SMTP service and basic email sending
- Week 2: Template engine, error handling, and audit logging  
- Week 3: Frontend email node editor and configuration UI
- Week 4: Template editor, live preview, and visual polish
- Week 5: Testing, documentation, and production readiness

## Dependencies

- Backend: lettre SMTP library, handlebars template engine
- Frontend: Monaco editor for template editing, email preview components
- Infrastructure: SMTP server access and credentials
- Security: Input validation and sanitization libraries
- Testing: SMTP testing tools and mock servers