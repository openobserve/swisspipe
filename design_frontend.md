# SwissPipe Frontend Design Document

## Overview

This document outlines the frontend design for SwissPipe, a visual workflow engine that provides an intuitive GUI for creating, managing, and monitoring data processing workflows. The frontend will offer a modern, responsive interface built with Vue.js 3 and Vue Flow for visual workflow design.

## Design Philosophy

- **Visual-First Design**: Workflows are created and edited through an intuitive drag-and-drop interface
- **Real-time Feedback**: Live monitoring of workflow execution with WebSocket integration
- **Developer-Friendly**: Monaco editor integration for advanced code editing
- **Responsive Design**: Mobile-first approach with desktop optimization
- **Accessibility**: WCAG 2.1 AA compliant interface using HeadlessUI components

## Technology Stack

### Core Technologies

- **Vue.js 3** - Frontend framework with Composition API for better reactivity and TypeScript integration
- **Vue Flow** - Advanced DAG (Directed Acyclic Graph) visualization and editing library for workflow design
- **TypeScript** - Type safety, better developer experience, and improved code maintainability
- **Tailwind CSS** - Utility-first CSS framework for rapid UI development and consistent design system
- **Pinia** - Modern state management solution for Vue 3 with excellent TypeScript support
- **Vue Router** - Declarative routing for single-page application navigation
- **WebSocket** - Real-time communication for workflow execution monitoring and live updates

### Supporting Libraries

#### Vue Flow Ecosystem
- **@vue-flow/core** - Core Vue Flow functionality for node and edge management
- **@vue-flow/controls** - Built-in controls for pan, zoom, fit view, and fullscreen
- **@vue-flow/background** - Customizable grid and dot pattern backgrounds
- **@vue-flow/node-toolbar** - Context-sensitive node manipulation toolbar
- **@vue-flow/minimap** - Workflow overview minimap for navigation in large workflows

#### UI and UX Libraries
- **axios** - Promise-based HTTP client for API communication with interceptors
- **@heroicons/vue** - Beautiful hand-crafted SVG icons by the makers of Tailwind CSS
- **@headlessui/vue** - Completely unstyled, fully accessible UI components

#### Development and Code Editing
- **Monaco Editor** - VS Code editor for in-browser code editing of JavaScript transformers and conditions
- **@monaco-editor/vue** - Vue wrapper for Monaco Editor with proper lifecycle management

## Application Architecture

### Component Structure

```
src/
├── components/
│   ├── layout/
│   │   ├── AppHeader.vue           # Top navigation bar
│   │   ├── AppSidebar.vue          # Collapsible sidebar (when needed)
│   │   └── AppLayout.vue           # Main layout wrapper
│   ├── workflow/
│   │   ├── WorkflowList.vue        # Workflow listing with search/filter
│   │   ├── WorkflowTable.vue       # Data table for workflow listing
│   │   ├── WorkflowTableRow.vue    # Individual workflow table row
│   │   ├── WorkflowDesigner.vue    # Main workflow design interface
│   │   ├── WorkflowCanvas.vue      # Vue Flow canvas wrapper
│   │   ├── WorkflowToolbar.vue     # Workflow-level actions (save, run, etc.)
│   │   └── WorkflowDetails.vue     # Workflow metadata editing
│   ├── nodes/
│   │   ├── TriggerNode.vue         # HTTP trigger node
│   │   ├── ConditionNode.vue       # Decision/branching node
│   │   ├── TransformerNode.vue     # Data transformation node
│   │   ├── AppNode.vue             # External app integration node
│   │   └── NodeBase.vue            # Shared node component base
│   ├── panels/
│   │   ├── NodeLibraryPanel.vue    # Fixed left-side node library
│   │   ├── NodePropertiesPanel.vue # Slide-out properties panel (when node selected)
│   │   ├── CodeEditor.vue          # Monaco editor wrapper
│   │   └── WorkflowHeader.vue      # Top header with navigation and actions
│   ├── modals/
│   │   ├── CreateWorkflowModal.vue # New workflow creation
│   │   ├── DeleteConfirmModal.vue  # Deletion confirmation
│   │   └── WorkflowSettingsModal.vue # Workflow-level settings
│   └── common/
│       ├── LoadingSpinner.vue      # Loading states
│       ├── ErrorAlert.vue          # Error display component
│       ├── SuccessToast.vue        # Success notifications
│       ├── StatusBadge.vue         # Workflow status indicators
│       ├── DataTable.vue           # Reusable data table component
│       ├── TableHeader.vue         # Sortable table headers
│       ├── TablePagination.vue     # Table pagination controls
│       └── ActionDropdown.vue      # Action dropdown menus
├── stores/
│   ├── workflows.ts                # Workflow management store
│   ├── nodes.ts                    # Node state and configuration store
│   ├── execution.ts                # Workflow execution monitoring store
│   └── ui.ts                       # UI state (panels, modals, etc.)
├── types/
│   ├── workflow.ts                 # Workflow-related TypeScript types
│   ├── nodes.ts                    # Node configuration types
│   └── api.ts                      # API response types
├── services/
│   ├── api.ts                      # API client with interceptors
│   ├── websocket.ts                # WebSocket connection management
│   └── validation.ts               # Client-side validation rules
├── utils/
│   ├── constants.ts                # Application constants
│   ├── formatters.ts               # Data formatting utilities
│   └── validators.ts               # Form validation helpers
└── views/
    ├── WorkflowList.vue            # Main workflows listing page
    └── WorkflowDesigner.vue        # Workflow design interface
```

## User Interface Design

### Navigation Structure

#### Top Navigation Bar
- **Logo/Brand**: SwissPipe branding with home link
- **Primary Navigation**: Single menu item - "Workflows"
- **User Actions**: Settings, help, user profile (future enhancement)
- **Theme Toggle**: Light/dark mode switcher
- **Connection Status**: API and WebSocket connection indicators

### Main Application Views

#### 1. Workflow List View (`/workflows`)

**Layout**: Table-based layout with advanced sorting, filtering, and search capabilities

**Features**:
- **Search Bar**: Real-time search by workflow name and description
- **Filter Options**: Status (active, inactive, error), last modified, creation date
- **Data Table**: Professional table with the following columns:
  - **Name**: Workflow name with clickable link to designer
  - **Description**: Brief workflow description (truncated with tooltip)
  - **Status**: Status badge with color coding (Active/Inactive/Error/Running)
  - **Created**: Creation date with relative time (e.g., "2 days ago")
  - **Last Modified**: Last edit timestamp with user info
  - **Last Run**: Last execution time and result status
  - **Success Rate**: Percentage with visual indicator bar
  - **Actions**: Dropdown menu with edit, duplicate, delete, run options
- **Table Features**:
  - **Sortable Columns**: Click column headers to sort ascending/descending
  - **Row Selection**: Checkboxes for bulk operations
  - **Pagination**: Configurable page size (10, 25, 50, 100 rows)
  - **Responsive**: Columns hide on smaller screens (mobile-first priority)
  - **Row Hover**: Subtle highlight on row hover
  - **Context Menu**: Right-click for quick actions

**Actions**:
- **Create New Workflow**: Prominent button to start workflow creation
- **Bulk Operations**: Select multiple workflows for batch operations
- **Import/Export**: Workflow definition import/export functionality

#### 2. Workflow Designer View (`/workflows/:id`)

**Layout**: Full-screen design interface with fixed left panel and dark theme

**Top Header Bar**:
- **Back Navigation**: Arrow back button to return to workflow list
- **Workflow Title**: Editable workflow name field (e.g., "wf1")
- **Action Buttons**: Save (green) and Reset (gray) buttons on the right

**Left Node Library Panel (Fixed)**:
- **Panel Title**: "Node Library" header
- **Node Categories**: Organized in expandable sections:
  - **Triggers**: Blue colored section
    - **Trigger**: "Input data from configured sources" with blue dot indicator
  - **Transformers**: Purple colored section
    - **Transformer**: "Process and modify data" with purple dot indicator
  - **Logic**: Yellow/amber colored section
    - **Condition**: "Branch workflow based on conditions" with yellow dot indicator
  - **Apps**: Green colored section
    - **App**: "Send data to external systems" with green dot indicator
- **Dark Theme**: Charcoal/dark gray background with colored accents
- **Node Descriptions**: Each node type shows descriptive text below the name

**Main Canvas Area**:
- **Dark Canvas**: Dark blue/navy background for visual contrast
- **Node Visualization**: Nodes appear as rounded rectangles with:
  - **Color Coding**: Matches the library categories (blue for trigger, green for app)
  - **Node Labels**: Display node type and custom name (e.g., "Workflow Trigger", "app1")
  - **Status Indicators**: "Ready" status shown below node name
  - **Connection Handles**: Small circular connection points
  - **Settings Icon**: Gear icon in top-right corner of each node
- **Flow Connections**: Curved lines connecting nodes with smooth bezier curves
- **Grid Pattern**: Subtle dot grid pattern for alignment (optional)
- **Zoom Controls**: Standard Vue Flow zoom controls (not visible in image but implied)

**Visual Design Specifications**:
- **Color Scheme**: Dark theme with high contrast
  - Background: Dark navy/charcoal (`#1a1a2e` or similar)
  - Panel: Darker gray (`#16213e` or similar)
  - Accent Colors: Blue, Purple, Yellow, Green for node categories
- **Typography**: Clean sans-serif font, good contrast against dark background
- **Node Design**: Rounded rectangles with subtle shadows and colored left borders
- **Interactive Elements**: Hover states and selection indicators
- **Responsive Behavior**: Left panel remains fixed, canvas area responsive

### Node Types and Configuration

#### 1. Trigger Node
**Visual Design**: Rounded rectangle with blue accent color and dot indicator
**Node Library Description**: "Input data from configured sources"
**Canvas Appearance**: Blue themed node showing "Workflow Trigger" with "Ready" status
**Configuration Panel**:
- HTTP Methods: Multi-select checkboxes (GET, POST, PUT, DELETE)
- Endpoint URL: Auto-generated, display-only with copy button
- Description: Optional text description
- Headers: Key-value pair editor for custom headers (advanced tab)

#### 2. Condition Node
**Visual Design**: Rounded rectangle with yellow/amber accent color and dot indicator
**Node Library Description**: "Branch workflow based on conditions"
**Canvas Appearance**: Yellow themed node with condition logic display
**Configuration Panel**:
- **JavaScript Code Editor**: Monaco editor with:
  - Syntax highlighting for JavaScript
  - Auto-completion for event object properties
  - Error highlighting and validation
  - Code folding and minimap for large functions
- **Function Template**: Pre-populated with correct function signature
- **Test Data**: JSON editor for testing condition logic
- **Preview**: Live evaluation results display

#### 3. Transformer Node
**Visual Design**: Rounded rectangle with purple accent color and dot indicator
**Node Library Description**: "Process and modify data"
**Canvas Appearance**: Purple themed node with transformation display
**Configuration Panel**:
- **JavaScript Code Editor**: Similar to condition node
- **Input/Output Schema**: Visual JSON schema editors (future enhancement)
- **Test Data**: Input data editor with output preview
- **Data Mapping**: Visual field mapping interface (future enhancement)

#### 4. App Node
**Visual Design**: Rounded rectangle with green accent color and dot indicator
**Node Library Description**: "Send data to external systems"
**Canvas Appearance**: Green themed node showing app name (e.g., "app1") with "Ready" status
**Configuration Panel**:
- **App Type**: Dropdown selection (Webhook, Database, etc.)
- **Connection Settings**: 
  - URL/Endpoint configuration
  - Authentication methods (Basic, Bearer Token, API Key)
  - Timeout and retry configuration
- **Request Configuration**:
  - HTTP method selection
  - Headers editor
  - Body template with variable substitution
- **Response Handling**: 
  - Success criteria definition
  - Error handling configuration

#### Node Interaction Design
- **Settings Access**: Gear icon in top-right corner of each node for configuration
- **Connection Points**: Small circular handles for connecting nodes
- **Status Display**: Status text ("Ready", "Running", "Error") below node names
- **Visual Feedback**: Hover effects and selection states with subtle highlighting

### Code Editor Integration

#### Monaco Editor Configuration
- **Language Support**: JavaScript with TypeScript definitions
- **Theme Integration**: Matches application light/dark theme
- **Feature Set**:
  - Syntax highlighting and error detection
  - Auto-completion with custom definitions
  - Code folding and minimap
  - Find/replace functionality
  - Bracket matching and auto-closing
  - Multiple cursor support

#### Code Templates
- **Condition Functions**:
  ```javascript
  function condition(event) {
    // event.data - The input data
    // event.metadata - Request metadata
    // event.headers - HTTP headers
    
    // Return true or false
    return event.data.value > 50;
  }
  ```

- **Transformer Functions**:
  ```javascript
  function transformer(event) {
    // Modify event.data as needed
    event.data.processed = true;
    event.data.timestamp = new Date().toISOString();
    
    // Return the modified event or null to drop
    return event;
  }
  ```

## State Management

### Pinia Store Structure

#### Workflow Store (`stores/workflows.ts`)
```typescript
interface WorkflowState {
  workflows: Workflow[]
  currentWorkflow: Workflow | null
  loading: boolean
  error: string | null
  filters: WorkflowFilters
  searchTerm: string
}

// Actions: fetchWorkflows, createWorkflow, updateWorkflow, deleteWorkflow
```

#### Node Store (`stores/nodes.ts`)
```typescript
interface NodeState {
  nodes: Node[]
  edges: Edge[]
  selectedNode: string | null
  nodeTypes: NodeTypeDefinition[]
  validation: ValidationState
}

// Actions: addNode, updateNode, deleteNode, connectNodes
```

#### Execution Store (`stores/execution.ts`)
```typescript
interface ExecutionState {
  executions: WorkflowExecution[]
  realTimeData: Map<string, ExecutionEvent>
  isMonitoring: boolean
  webSocketConnected: boolean
}

// Actions: startMonitoring, stopMonitoring, fetchExecutions
```

## Real-time Features

### WebSocket Integration
- **Connection Management**: Automatic reconnection with exponential backoff
- **Event Handling**: 
  - Workflow execution started/completed/failed
  - Node execution progress
  - Real-time log streaming
  - System status updates

### Live Monitoring
- **Execution Visualization**: Highlight currently executing nodes on canvas
- **Progress Indicators**: Real-time progress bars and status updates
- **Log Stream**: Live log output in dedicated panel
- **Performance Metrics**: Execution time, throughput, error rates

## Responsive Design

### Breakpoint Strategy
- **Mobile** (< 768px): Stack panels, simplified toolbar, touch-optimized
- **Tablet** (768px - 1024px): Collapsible panels, adaptive toolbar
- **Desktop** (> 1024px): Full feature set, multi-panel layout

### Mobile Considerations
- **Touch Interactions**: Optimized node manipulation for touch screens
- **Simplified UI**: Streamlined interface with essential features
- **Gesture Support**: Pinch to zoom, pan gestures for canvas navigation
- **Responsive Panels**: Stack panels vertically on smaller screens

## Accessibility Features

### WCAG 2.1 AA Compliance
- **Keyboard Navigation**: Full keyboard support for all interactions
- **Screen Reader Support**: Proper ARIA labels and announcements
- **Color Contrast**: Minimum 4.5:1 contrast ratio for all text
- **Focus Management**: Clear focus indicators and logical tab order
- **Alternative Text**: Descriptive alt text for icons and visual elements

### Accessibility Enhancements
- **High Contrast Mode**: Optional high contrast theme
- **Reduced Motion**: Respect `prefers-reduced-motion` settings
- **Font Scaling**: Support for browser font size adjustments
- **Voice Announcements**: Screen reader announcements for state changes

## Performance Optimizations

### Vue.js Optimizations
- **Lazy Loading**: Route-based code splitting for faster initial load
- **Component Optimization**: Proper use of `v-memo` and `defineAsyncComponent`
- **Reactive Data**: Minimize reactive data and use `shallowRef` where appropriate
- **Virtual Scrolling**: For large lists of workflows or nodes

### Canvas Performance
- **Vue Flow Optimization**: Efficient node rendering with proper keys
- **Debounced Updates**: Debounce frequent operations like autosave
- **Memory Management**: Proper cleanup of WebSocket connections and timers
- **Lazy Rendering**: Only render visible nodes in large workflows

## Testing Strategy

### Unit Testing
- **Vitest**: Fast unit testing for utilities and stores
- **Vue Test Utils**: Component testing with proper mocking
- **Coverage Targets**: Minimum 80% code coverage for critical paths

### Integration Testing
- **Playwright**: End-to-end testing for complete user workflows
- **API Testing**: Mock API responses for consistent testing
- **Accessibility Testing**: Automated a11y testing with axe-core

### Visual Testing
- **Chromatic**: Visual regression testing for UI components
- **Storybook**: Component documentation and visual testing

## Deployment and Build

### Build Configuration
- **Vite**: Fast build tool with hot module replacement
- **TypeScript**: Strict type checking in build process
- **Asset Optimization**: Image optimization and bundle splitting
- **Progressive Web App**: Service worker for offline capability (future enhancement)

### Environment Configuration
- **Development**: Local API proxy, hot reload, debug tools
- **Staging**: Production-like environment for testing
- **Production**: Optimized build, CDN integration, analytics

## Security Considerations

### Client-Side Security
- **Input Sanitization**: Sanitize user input before display
- **Content Security Policy**: Strict CSP headers to prevent XSS
- **API Security**: Token-based authentication with automatic refresh
- **Code Execution**: Sandboxed JavaScript execution for security

### Data Protection
- **Local Storage**: Encrypt sensitive data in browser storage
- **Session Management**: Secure session handling with proper timeout
- **HTTPS Only**: Enforce HTTPS for all communications
- **Input Validation**: Client-side validation with server-side verification

## Future Enhancements

### Phase 1 Enhancements
- **Workflow Templates**: Pre-built workflow templates for common use cases
- **Node Marketplace**: Community-contributed custom nodes
- **Advanced Scheduling**: Cron-based workflow scheduling
- **Workflow Versioning**: Version control for workflow definitions

### Phase 2 Enhancements
- **Collaborative Editing**: Real-time collaborative workflow editing
- **Advanced Analytics**: Detailed workflow performance analytics
- **Custom Node Development**: SDK for creating custom node types
- **Workflow Sharing**: Public workflow gallery and sharing

### Phase 3 Enhancements
- **Mobile App**: Native mobile application for monitoring
- **Enterprise Features**: SSO, advanced permissions, audit trails
- **AI Integration**: AI-powered workflow optimization suggestions
- **Multi-tenant Support**: Organization and team management

## Conclusion

This frontend design provides a comprehensive, modern interface for SwissPipe that emphasizes usability, performance, and accessibility. The combination of Vue.js 3, Vue Flow, and TypeScript creates a robust foundation for a professional workflow management interface.

The modular architecture ensures maintainability and extensibility, while the responsive design guarantees a great user experience across all devices. Real-time features and comprehensive testing strategies ensure the application will perform well in production environments.

