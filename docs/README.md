# SwissPipe API Documentation

This directory contains comprehensive API documentation for SwissPipe, a powerful workflow automation platform.

## 📁 Files

- **`openapi.yaml`** - Complete OpenAPI 3.0.3 specification with all endpoints, schemas, and examples
- **`api-documentation.html`** - Beautiful, interactive HTML documentation powered by Redoc
- **`serve-docs.py`** - Python server script for local documentation hosting
- **`README.md`** - This file

## 🚀 Quick Start

### View Documentation Locally

1. **Serve the documentation:**
   ```bash
   cd docs
   python serve-docs.py
   ```

   Or specify a custom port:
   ```bash
   python serve-docs.py 3000
   ```

2. **Open your browser** to `http://localhost:8080/api-documentation.html` (or your custom port)

3. The documentation will automatically open in your default browser!

### Alternative Methods

#### Using Python's built-in server:
```bash
cd docs
python -m http.server 8080
# Then visit: http://localhost:8080/api-documentation.html
```

#### Using Node.js:
```bash
cd docs
npx serve . -p 8080
# Then visit: http://localhost:8080/api-documentation.html
```

## 📚 Documentation Features

### 🎨 Beautiful Design
- Modern, responsive design with gradient backgrounds
- Professional typography using Inter font family
- Intuitive navigation with quick-access links
- Mobile-friendly interface

### 🔍 Comprehensive Coverage
- **60+ API endpoints** across all SwissPipe services
- **Detailed schemas** with examples and validation rules
- **Authentication guides** for Basic Auth, OAuth, and workflow tokens
- **Error responses** with detailed error codes and messages

### 🚀 Interactive Features
- **Try-it-out functionality** for testing API endpoints directly
- **Code examples** in multiple languages
- **Real-time request/response previews**
- **Expandable/collapsible sections** for easy navigation

## 📋 API Categories

### Core APIs
- **Health Check** - Service health and status monitoring
- **Workflow Ingestion** - Trigger workflow executions (public APIs)
- **Workflow Management** - CRUD operations for workflows (admin)
- **Execution Monitoring** - Real-time execution tracking (admin)

### Advanced Features
- **Script Testing** - JavaScript code execution and testing
- **AI Integration** - Claude AI code and workflow generation
- **Settings Management** - System configuration
- **Authentication** - Google OAuth and session management
- **Analytics** - Segment-compatible event tracking

### Monitoring & Operations
- **Worker Pool Statistics** - Performance and capacity metrics
- **Cleanup Statistics** - Data retention and cleanup metrics
- **Execution Details** - Step-by-step execution information

## 🔐 Authentication

### Admin APIs (`/api/admin/v1/*`)
- **Type:** HTTP Basic Authentication
- **Credentials:** Username and password (configurable via `SP_USERNAME`/`SP_PASSWORD`)
- **Usage:** All administrative operations (workflow management, monitoring, etc.)

### Workflow Ingestion (`/api/v1/*`)
- **Type:** UUID-based path authentication
- **Format:** `/api/v1/{workflow_id}/trigger`
- **Usage:** Public APIs for triggering workflow executions

### Google OAuth (UI only)
- **Type:** OAuth 2.0 with Google
- **Usage:** Web interface authentication
- **Configuration:** Optional, requires `GOOGLE_OAUTH_*` environment variables

## 🛠️ Development

### Updating the Documentation

1. **Edit the OpenAPI spec:**
   ```bash
   vim openapi.yaml
   ```

2. **The HTML documentation will automatically reflect changes** when you refresh the page

3. **Validate your OpenAPI spec:**
   ```bash
   # Using Swagger Editor online
   # Copy-paste your openapi.yaml content to: https://editor.swagger.io/

   # Or using CLI tools:
   npm install -g @apidevtools/swagger-cli
   swagger-cli validate openapi.yaml
   ```

### Custom Styling

The HTML documentation includes custom CSS for:
- Beautiful gradient backgrounds
- Professional typography
- Responsive design
- Custom color scheme matching SwissPipe branding

Edit the `<style>` section in `api-documentation.html` to customize the appearance.

## 🌐 Production Deployment

### Static Hosting
Deploy both `openapi.yaml` and `api-documentation.html` to any static hosting service:
- **GitHub Pages** - Automatic deployment from your repository
- **Netlify** - Drag-and-drop deployment
- **Vercel** - Git-based deployment
- **AWS S3** - Static website hosting

### Integration with SwissPipe
The documentation can be served directly by SwissPipe by placing the files in the `static/` directory and accessing them via the web interface.

## 📖 OpenAPI Specification Details

### Specification Version
- **OpenAPI:** 3.0.3
- **SwissPipe Version:** 0.1.0
- **Last Updated:** September 2024

### Key Features
- **Complete schema definitions** for all data models
- **Detailed parameter descriptions** with examples
- **Comprehensive error responses** with troubleshooting guides
- **Security scheme definitions** for all authentication methods
- **Server definitions** for development and production environments

### Schema Highlights
- `WorkflowEvent` - Core data structure for workflow processing
- `WorkflowExecution` - Complete execution lifecycle tracking
- `ExecutionStep` - Granular step-by-step execution details
- `Node` & `Edge` - Workflow graph components
- Authentication models for OAuth and session management

## 🤝 Contributing

To improve the documentation:

1. **For API changes:** Update the backend code and regenerate the OpenAPI spec
2. **For documentation improvements:** Edit `openapi.yaml` directly
3. **For styling changes:** Modify the CSS in `api-documentation.html`
4. **For new features:** Add new endpoints and schemas to the OpenAPI spec

## 📞 Support

- **GitHub Issues:** [Report documentation issues](https://github.com/prabhatsharma/swisspipe/issues)
- **API Questions:** Check the interactive documentation for detailed examples
- **Development:** Refer to the main project README for setup instructions

---

**Built with ❤️ using [Redoc](https://github.com/Redocly/redoc) and OpenAPI 3.0.3**