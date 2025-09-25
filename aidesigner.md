# AI Designer Assistant - Product Requirements Document

## Overview
Add an AI assistant feature to the SwissPipe workflow designer that provides contextual help, guidance, and workflow command execution through a collapsible chat interface.

## Problem Statement
Users building workflows in the visual designer need:
- Contextual assistance for understanding node configurations
- Help troubleshooting workflow logic
- Suggestions for optimal workflow patterns
- Learning about available node types and their capabilities
- **Ability to create and modify workflows through natural language commands**

## Solution
Implement an AI assistant button that opens a chat UI overlay, providing intelligent assistance and workflow modification capabilities through natural language commands.

## User Stories

### Primary User Stories
- As a workflow designer, I want to ask questions about node configuration so I can build workflows correctly
- As a new user, I want contextual help about workflow patterns so I can learn best practices
- As an experienced user, I want quick answers about node capabilities so I can work efficiently
- As any user, I want the chat UI to auto-collapse when I click elsewhere so it doesn't obstruct my workflow design
- **As a user, I want to create workflows using natural language commands like "create a workflow to send data to HubSpot and Active Campaign"**
- **As a user, I want to modify existing workflows by saying "add a transformer to send data to Segment.com"**

### Secondary User Stories
- As a user, I want the AI to understand my current workflow context so suggestions are relevant
- As a user, I want chat history to persist during my session so I can reference previous answers
- **As a user, I want the AI to execute workflow modifications immediately so I can see results in real-time**
- **As a user, I want to undo AI-generated workflow changes if they're not what I intended**

## Functional Requirements

### Core Features
1. **AI Assistant Button**
   - Fixed position button in the workflow designer interface
   - Accessible at all times during workflow design
   - Clear visual indicator (AI/chat icon)

2. **Collapsible Chat Interface**
   - Overlay panel that slides out from the assistant button
   - Auto-collapse when user clicks outside the chat area
   - Auto-collapse when user loses focus from chat input/interface
   - Manual collapse via close button or escape key

3. **Chat Functionality**
   - Text input for user questions
   - Streaming response display
   - Chat history within current session
   - Loading states during AI response generation

4. **Context Awareness**
   - Access to current workflow structure
   - Knowledge of selected nodes and their configurations
   - Understanding of available node types and capabilities

5. **Workflow Command Execution**
   - Natural language parsing for workflow commands
   - Real-time workflow modification capabilities
   - Command confirmation and preview before execution
   - Undo/redo support for AI-generated changes

## Technical Requirements

### Frontend Implementation
- **Component Location**: `frontend/src/components/workflow/`
- **Main Components**:
  - `AIAssistantButton.vue` - Floating action button
  - `AIChatPanel.vue` - Collapsible chat interface
  - `ChatMessage.vue` - Individual message component
  - `ChatInput.vue` - Message input component

### Integration Points
- **Workflow Designer**: Integrate with `WorkflowDesigner.vue`
- **State Management**: Extend Pinia store for chat state
- **Styling**: Use existing Tailwind CSS classes for consistency

### API Requirements
- **Chat Endpoint**: `/api/v1/ai/chat`
- **Command Execution Endpoint**: `/api/v1/ai/execute`
- **Request Format (Chat)**:
  ```json
  {
    "message": "user question or command",
    "context": {
      "workflow": "current workflow JSON",
      "selectedNode": "node ID or null"
    }
  }
  ```
- **Request Format (Command Execution)**:
  ```json
  {
    "command": "parsed command object",
    "workflowId": "current workflow ID",
    "context": {
      "workflow": "current workflow JSON"
    }
  }
  ```
- **Response Format**: Server-sent events for streaming responses
- **Command Response**: JSON with workflow modifications and confirmation prompts

### Command Processing
- **Supported Commands**:
  - `CREATE_WORKFLOW`: "create a workflow to [description]"
  - `ADD_NODE`: "add a [node type] to [target/description]"
  - `MODIFY_NODE`: "modify [node] to [changes]"
  - `DELETE_NODE`: "remove the [node description]"
  - `CONNECT_NODES`: "connect [source] to [target]"
  - `SET_CONFIGURATION`: "configure [node] with [settings]"

- **Command Examples**:
  ```json
  {
    "type": "CREATE_WORKFLOW",
    "description": "send data to HubSpot and Active Campaign",
    "nodes": [
      {"type": "webhook", "target": "hubspot"},
      {"type": "webhook", "target": "active_campaign"}
    ]
  }
  ```

### UI/UX Specifications

#### AI Assistant Button
- **Position**: Fixed bottom-right corner of workflow designer
- **Size**: 56px diameter circular button
- **Icon**: AI/robot icon or chat bubble with sparkle
- **States**: Default, hover, active (when chat is open)
- **Z-index**: Above workflow canvas but below modals

#### Chat Panel
- **Dimensions**: 400px width, max 600px height
- **Position**: Slides out from assistant button
- **Animation**: Smooth slide-in/out transition (200ms)
- **Background**: Semi-transparent backdrop with blur effect
- **Border**: Rounded corners consistent with app design

#### Auto-collapse Behavior
- **Triggers**:
  - Click outside chat panel
  - Focus lost from chat input
  - Click on workflow canvas
  - Escape key press
- **Exceptions**:
  - Don't collapse when clicking within chat panel
  - Don't collapse during AI response streaming
  - Don't collapse during command confirmation dialogs

#### Command Execution UI
- **Command Preview**: Show visual preview of workflow changes before applying
- **Confirmation Dialog**: "Apply these changes to your workflow?" with preview
- **Progress Indicators**: Show real-time status during command execution
- **Undo Button**: Easily revert AI-generated changes
- **Command Suggestions**: Auto-complete for common workflow patterns

### Responsive Design
- **Desktop**: Full-sized chat panel
- **Tablet**: Slightly smaller panel (350px width)
- **Mobile**: Full-screen overlay on small screens

## Non-Functional Requirements

### Performance
- Chat panel open/close animation must be smooth (60fps)
- AI responses should start streaming within 2 seconds
- Chat history limited to 50 messages per session
- Command execution should complete within 5 seconds
- Workflow modifications must be atomic (all or nothing)

### Accessibility
- Keyboard navigation support (Tab, Enter, Escape)
- ARIA labels for screen readers
- High contrast mode support
- Focus management when panel opens/closes

### Privacy & Security
- Chat messages not persisted beyond current session
- Workflow context sanitized before sending to AI
- No sensitive data (credentials, tokens) included in context
- Command execution requires explicit user confirmation
- Audit log for all AI-generated workflow modifications
- Rate limiting on command execution to prevent abuse

## Success Metrics
- **Engagement**: 30% of workflow designer sessions use AI assistant
- **Command Usage**: 40% of AI interactions include workflow commands
- **Efficiency**: 20% reduction in time to complete workflow setup
- **Command Success Rate**: 85% of executed commands produce intended results
- **User Satisfaction**: 4.5+ rating for AI assistant helpfulness

## Implementation Phases

### Phase 1: Core UI (Week 1)
- AI assistant button component
- Basic chat panel with manual toggle
- Auto-collapse functionality

### Phase 2: Chat Functionality (Week 2)
- Message input and display
- Chat history management
- Loading states and animations

### Phase 3: AI Integration (Week 3)
- Backend chat API endpoint
- Context awareness implementation
- Streaming response handling

### Phase 4: Command System (Week 4)
- Natural language command parsing
- Workflow modification engine
- Command execution API endpoint

### Phase 5: Command UI & Security (Week 5)
- Command preview and confirmation dialogs
- Undo/redo functionality
- Audit logging and rate limiting

### Phase 6: Polish & Testing (Week 6)
- Responsive design implementation
- Accessibility improvements
- User testing and refinement

## Future Enhancements
- Voice input/output capability
- Workflow suggestions based on patterns
- Integration with documentation search
- Multi-language support
- Persistent chat history across sessions
- **Advanced Command Features**:
  - Bulk operations ("create 5 similar workflows")
  - Conditional commands ("if data contains email, add to CRM")
  - Workflow templates from natural language descriptions
  - Integration with external service catalogs (Zapier, IFTTT patterns)
  - Smart defaults based on user's previous workflows

## Detailed Command Examples

### Example Commands and Expected Behavior

1. **"Create a workflow to send data to HubSpot and Active Campaign"**
   - Creates new workflow with trigger node
   - Adds two webhook nodes configured for HubSpot and Active Campaign
   - Connects trigger to both webhook nodes
   - Pre-fills common configuration settings

2. **"Add a transformer to send data to Segment.com"**
   - Adds transformer node after current selection
   - Adds webhook node configured for Segment
   - Creates proper connections
   - Opens configuration panel for transformer code

3. **"Add email notification when workflow fails"**
   - Adds condition node to check for errors
   - Adds email node with failure template
   - Connects error flow to email notification

4. **"Create a delay of 5 minutes before sending to CRM"**
   - Adds delay node with 5-minute configuration
   - Inserts into existing workflow flow
   - Maintains proper execution sequence