# Admin API Logging

This document describes the comprehensive logging system for admin API calls in SwissPipe.

## Overview

All admin API calls (`/api/admin/v1/*`) are automatically logged with detailed information including:

- **User identification** (basic auth username or OAuth session info)
- **Request details** (method, path, query parameters, headers, body content)
- **Response information** (status code, duration)
- **Audit trail** for successful and failed operations
- **Request body logging** with size limits (4KB max) for POST/PUT/PATCH operations

## Log Targets

The logging system uses structured logging with specific targets:

- **`admin_api`**: Debug-level logs for all admin API requests and responses
- **`admin_api_audit`**: Info/Warn-level audit trail for completed operations

## Log Format

Each log entry is structured JSON with the following format:

### Request Started Log
```json
{
  "event": "admin_api_request_started",
  "method": "POST",
  "uri": "/api/admin/v1/workflows",
  "user_id": "12345678901234567890",
  "user_identifier": "john.doe@example.com",
  "user_name": "John Doe",
  "user_email": "john.doe@example.com",
  "auth_type": "oauth",
  "request_info": {
    "method": "POST",
    "path": "/workflows",
    "query": "",
    "headers": {
      "content-type": "application/json",
      "user-agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
    },
    "body": "{\"name\":\"My Workflow\",\"description\":\"Test workflow\"}"
  },
  "timestamp": "2025-01-15T10:30:45.123Z"
}
```

### Request Completed Log
```json
{
  "event": "admin_api_request_completed",
  "method": "POST",
  "uri": "/api/admin/v1/workflows",
  "status": 201,
  "duration_ms": 45,
  "user_id": "12345678901234567890",
  "user_identifier": "john.doe@example.com",
  "user_name": "John Doe",
  "user_email": "john.doe@example.com",
  "auth_type": "oauth",
  "timestamp": "2025-01-15T10:30:45.168Z"
}
```

### Audit Trail Log
```json
{
  "event": "admin_api_operation_success",
  "method": "POST",
  "uri": "/api/admin/v1/workflows",
  "status": 201,
  "duration_ms": 45,
  "user_id": "12345678901234567890",
  "user_identifier": "john.doe@example.com",
  "user_name": "John Doe",
  "user_email": "john.doe@example.com",
  "auth_type": "oauth",
  "request_info": {
    "method": "POST",
    "path": "/workflows",
    "query": "",
    "headers": {
      "content-type": "application/json",
      "user-agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
    },
    "body": "{\"name\":\"My Workflow\",\"description\":\"Test workflow\"}"
  },
  "timestamp": "2025-01-15T10:30:45.168Z"
}
```

## User Identification

The system identifies users through database lookup and provides detailed information:

1. **OAuth users**:
   - `user_id`: Google OAuth user ID from session
   - `user_identifier`: User's email address
   - `user_name`: Full name from Google profile
   - `user_email`: Email address from Google profile
   - `auth_type`: "oauth"

2. **Basic auth users**:
   - `user_id`: "basic_{username}" format
   - `user_identifier`: Username from basic auth
   - `user_name`: null (not available)
   - `user_email`: null (not available)
   - `auth_type`: "basic_auth"

3. **Unauthenticated users**:
   - `user_id`: "unknown"
   - `user_identifier`: "anonymous"
   - `user_name`: null
   - `user_email`: null
   - `auth_type`: "none"

4. **Expired/Invalid OAuth sessions**:
   - `user_id`: First 8 characters of session ID
   - `user_identifier`: "oauth_user_expired"
   - `user_name`: null
   - `user_email`: null
   - `auth_type`: "oauth"

## Enabling Detailed Logging

To see admin API logs, set the log level to DEBUG:

```bash
RUST_LOG=debug cargo run
```

For audit trail only:

```bash
RUST_LOG=admin_api_audit=info cargo run
```

For comprehensive logging:

```bash
RUST_LOG=debug,admin_api=debug,admin_api_audit=info cargo run
```

## Example Log Output

### Successful Operation (OAuth User)
```json
{"event":"admin_api_operation_success","method":"POST","uri":"/api/admin/v1/workflows","status":201,"duration_ms":45,"user_id":"12345678901234567890","user_identifier":"john.doe@example.com","user_name":"John Doe","user_email":"john.doe@example.com","auth_type":"oauth","request_info":{"method":"POST","path":"/workflows","query":"","headers":{"content-type":"application/json","user-agent":"Mozilla/5.0"}},"timestamp":"2025-01-15T10:30:45.168Z"}
```

### Failed Operation (Basic Auth User)
```json
{"event":"admin_api_operation_failed","method":"DELETE","uri":"/api/admin/v1/workflows/123","status":404,"duration_ms":12,"user_id":"basic_admin","user_identifier":"admin","user_name":null,"user_email":null,"auth_type":"basic_auth","request_info":{"method":"DELETE","path":"/workflows/123","query":"","headers":{"user-agent":"curl/8.0.0"}},"timestamp":"2025-01-15T10:30:45.180Z"}
```

## Covered Endpoints

The middleware automatically logs all admin endpoints:

- `/api/admin/v1/workflows/*` - Workflow management
- `/api/admin/v1/executions/*` - Execution management
- `/api/admin/v1/script/*` - Script execution
- `/api/admin/v1/ai/*` - AI operations
- `/api/admin/v1/settings/*` - Settings management

## Security Notes

- **Sensitive headers** are excluded from logs (authorization, cookie, x-api-key, x-auth-token, x-forwarded-for, x-real-ip, set-cookie, www-authenticate, proxy-authorization, x-csrf-token, x-xsrf-token)
- **Passwords** in basic auth are not logged (only username)
- **Session IDs** are safely truncated with collision avoidance for short session IDs
- **Request body logging** is limited to 4KB and only captures JSON, form data, and multipart content
- **Header values** are truncated at 200 characters to prevent log bloat
- **Binary data** in request bodies is logged as metadata only (size, not content)