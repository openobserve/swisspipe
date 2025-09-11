# SwissPipe Frontend

A modern, dark-themed Vue.js 3 frontend for the SwissPipe workflow engine built with TypeScript, Tailwind CSS, and Vue Flow.

## Features

### 🎨 **Modern Dark UI**
- Professional dark theme with high contrast
- Responsive design for desktop and mobile
- Tailwind CSS with custom color palette
- Clean, minimalist interface matching the design specs

### 📊 **Workflow Management**
- **Table-based workflow list** with search and filtering
- **Real-time workflow status** indicators
- **Bulk operations** for multiple workflows
- **Quick actions** (edit, duplicate, delete, run)

### 🔧 **Visual Workflow Designer**
- **Vue Flow integration** for professional DAG editing
- **Drag-and-drop node creation** from left panel library
- **Node categories**: Triggers (Blue), Transformers (Purple), Logic (Yellow), Apps (Green)
- **Real-time validation** with error and warning indicators
- **Properties panel** that slides out when nodes are selected

### 💻 **Code Editor Integration**
- **Monaco Editor** (VS Code engine) for JavaScript editing
- **Syntax highlighting** and auto-completion
- **TypeScript definitions** for workflow events
- **Custom dark theme** matching the overall design
- **Keyboard shortcuts** (Ctrl+S to save, Shift+Alt+F to format)

### 🔄 **State Management**
- **Pinia stores** for workflows and nodes
- **Reactive data flow** with real-time updates
- **Error handling** and loading states
- **Client-side validation** and form management

## Technology Stack

### Core Technologies
- **Vue.js 3** with Composition API
- **TypeScript** for type safety
- **Tailwind CSS** for styling
- **Vue Flow** for workflow visualization
- **Pinia** for state management
- **Vue Router** for navigation

### Key Libraries
- **@vue-flow/core** - DAG visualization
- **@vue-flow/controls** - Pan, zoom, fit view controls
- **@vue-flow/background** - Grid background patterns
- **@vue-flow/minimap** - Workflow overview minimap
- **@vue-flow/node-toolbar** - Node manipulation toolbar
- **Monaco Editor** - Code editing with VS Code features
- **@heroicons/vue** - Beautiful SVG icons
- **@headlessui/vue** - Accessible UI components
- **Axios** - HTTP client for API communication

## Getting Started

### Prerequisites
- Node.js 18+ 
- npm or yarn
- SwissPipe backend running on port 3700

### Installation
```bash
cd frontend
npm install
```

### Development
```bash
npm run dev
```
The application will be available at `http://localhost:5173`

### Build for Production
```bash
npm run build
```

## Project Structure

```
src/
├── components/
│   ├── nodes/           # Vue Flow node components
│   │   ├── TriggerNode.vue
│   │   ├── ConditionNode.vue
│   │   ├── TransformerNode.vue
│   │   └── AppNode.vue
│   ├── panels/          # UI panels
│   │   ├── NodeLibraryModal.vue
│   │   └── NodePropertiesPanel.vue
│   └── common/          # Reusable components
│       └── CodeEditor.vue
├── stores/              # Pinia stores
│   ├── workflows.ts
│   └── nodes.ts
├── services/            # API and external services
│   └── api.ts
├── types/              # TypeScript type definitions
│   ├── workflow.ts
│   ├── nodes.ts
│   └── api.ts
├── views/              # Page components
│   ├── WorkflowListView.vue
│   └── WorkflowDesignerView.vue
└── assets/             # Styles and static assets
    └── main.css
```

## Key Features Implemented

### Workflow List Page (`/workflows`)
- ✅ Professional data table with sortable columns
- ✅ Real-time search and filtering
- ✅ Create new workflow modal
- ✅ Status indicators and metadata display
- ✅ Quick action buttons (edit, duplicate, delete)

### Workflow Designer (`/workflows/:id`)
- ✅ Full-screen design interface
- ✅ Fixed left node library panel with categorized nodes
- ✅ Vue Flow canvas with dark theme
- ✅ Slide-out properties panel for node configuration
- ✅ Real-time validation with error/warning display
- ✅ Save/Reset workflow functionality

### Node Library Panel
- ✅ Four node categories with color coding
- ✅ Drag-and-drop functionality
- ✅ Descriptive text for each node type
- ✅ Visual indicators (colored dots)

### Node Components
- ✅ **Trigger Node**: HTTP endpoint (blue theme)
- ✅ **Condition Node**: Decision logic (yellow theme) with true/false handles
- ✅ **Transformer Node**: Data processing (purple theme) 
- ✅ **App Node**: External integrations (green theme)

### Properties Panel
- ✅ Context-sensitive configuration based on selected node
- ✅ Node-specific form fields and settings
- ✅ Monaco code editor integration for JavaScript
- ✅ Real-time validation and error display

### Monaco Code Editor
- ✅ Full VS Code editing experience
- ✅ JavaScript syntax highlighting and auto-completion
- ✅ TypeScript definitions for workflow events
- ✅ Dark theme matching application design
- ✅ Format document and save shortcuts

## API Integration

The frontend connects to the SwissPipe backend API:
- **Base URL**: `http://localhost:3700`
- **Authentication**: Basic Auth for management endpoints
- **Workflow Management**: CRUD operations for workflows
- **Workflow Execution**: Trigger endpoints for running workflows

## Design System

### Colors
- **Primary**: Blue (#3b82f6) - Used for triggers and primary actions
- **Secondary**: Purple (#8b5cf6) - Used for transformers  
- **Warning**: Amber (#f59e0b) - Used for conditions/logic
- **Success**: Green (#10b981) - Used for apps/integrations
- **Dark Theme**: Custom dark palette for backgrounds and text

### Typography
- **Font Family**: Inter (clean, modern sans-serif)
- **Code Font**: JetBrains Mono (for Monaco editor)

## Usage

### 1. Access the Application
Navigate to `http://localhost:5173` in your browser

### 2. Workflow Management
- View all workflows in a sortable table
- Search workflows by name or description
- Create new workflows with the "Create Workflow" button
- Click on any workflow row to open the designer

### 3. Visual Workflow Designer
- **Left Panel**: Drag nodes from the library onto the canvas
- **Canvas**: Connect nodes by dragging from connection handles
- **Right Panel**: Click any node to configure its properties
- **Code Editing**: Click "Click to edit JavaScript code" for conditions/transformers

### 4. Node Types
- **Trigger (Blue)**: HTTP endpoints that start workflows
- **Condition (Yellow)**: Decision points with true/false branches
- **Transformer (Purple)**: Data processing and modification
- **App (Green)**: External system integrations

## Future Enhancements

- [ ] **WebSocket Integration** for real-time workflow execution monitoring
- [ ] **Workflow Templates** and marketplace
- [ ] **Advanced Validation** with visual indicators on canvas
- [ ] **Keyboard Shortcuts** for workflow operations
- [ ] **Workflow Versioning** and history
- [ ] **Collaborative Editing** with operational transforms
- [ ] **Mobile Optimization** with touch gestures
- [ ] **PWA Support** for offline functionality

## Contributing

1. Follow Vue 3 Composition API patterns
2. Use TypeScript for all new components
3. Maintain the dark theme consistency
4. Add proper error handling and loading states
5. Write descriptive commit messages
6. Test on both desktop and mobile viewports

## License

This project is part of the SwissPipe workflow engine.