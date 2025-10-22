<template>
  <div class="h-screen text-gray-100 flex flex-col">
    <!-- Header -->
    <WorkflowDesignerHeader
      v-model:workflow-name="workflowName"
      :saving="saving"
      @navigate-back="navigateBack"
      @update-workflow-name="updateWorkflowName"
      @save-workflow="saveWorkflow"
      @show-json-view="showJsonView"
      @reset-workflow="resetWorkflow"
      @toggle-node-library="toggleNodeLibrary"
      @toggle-ai-chat="toggleAiChat"
      @toggle-executions-panel="toggleExecutionsPanel"
      @logout="handleLogout"
    />

    <!-- Main Content -->
    <div class="flex flex-1 overflow-hidden">
      <!-- Node Library Modal -->
      <Transition
        enter-active-class="transition-opacity duration-300 ease-out"
        leave-active-class="transition-opacity duration-300 ease-in"
        enter-from-class="opacity-0"
        enter-to-class="opacity-100"
        leave-from-class="opacity-100"
        leave-to-class="opacity-0"
      >
        <div 
          v-if="showNodeLibrary"
          class="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center p-4"
          @click.self="closeNodeLibrary"
        >
          <div class="bg-slate-800 rounded-xl border border-slate-700 max-w-4xl w-full max-h-[80vh] overflow-hidden shadow-2xl">
            <NodeLibraryModal @close="closeNodeLibrary" @add-node="handleAddNode" />
          </div>
        </div>
      </Transition>

      <!-- AI Chat Modal -->
      <Transition
        enter-active-class="transition-opacity duration-300 ease-out"
        leave-active-class="transition-opacity duration-300 ease-in"
        enter-from-class="opacity-0"
        enter-to-class="opacity-100"
        leave-from-class="opacity-100"
        leave-to-class="opacity-0"
      >
        <div
          v-if="showAiChat"
          class="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-end p-4"
          @click.self="closeAiChat"
        >
          <div class="bg-slate-800 rounded-xl border border-slate-700 w-96 h-[80vh] overflow-hidden shadow-2xl">
            <AiChatModal @close="closeAiChat" />
          </div>
        </div>
      </Transition>

      <!-- Canvas Area -->
      <WorkflowCanvas
        v-model:nodes="nodeStore.nodes"
        v-model:edges="nodeStore.edges"
        @node-click="handleNodeClick"
        @edge-click="onEdgeClick"
        @pane-click="onPaneClick"
        @connect="onConnect"
        @nodes-initialized="onNodesInitialized"
        @nodes-delete="onNodesDelete"
        @drop="onDrop"
        @pause-loop="pauseLoop"
        @stop-loop="stopLoop"
        @retry-loop="retryLoop"
      />


      <!-- Executions Panel (slide out when executions button clicked) -->
      <div
        v-if="showExecutionsPanel"
        class="w-[500px] border-l border-slate-700/50 flex-shrink-0 overflow-y-auto glass-medium"
      >
        <ExecutionSidePanel
          :workflow-id="workflowId"
          @close="() => { closeExecutionsPanel(); clearExecutionTracing() }"
          @trace-execution="onTraceExecution"
        />
      </div>

    </div>

    <!-- Validation Errors/Warnings -->
    <ValidationNotifications :validation="nodeStore.validation" />

    <!-- Node Inspector Modal -->
    <NodeInspector
      :visible="showNodeInspector"
      :node-data="inspectedNode"
      @close="handleCloseInspector"
    />

    <!-- JSON View Modal -->
    <JsonViewModal
      :visible="showJsonModal"
      :json-data="workflowJson"
      @close="handleCloseJsonView"
    />

    <!-- Node Properties Modal -->
    <NodePropertiesPanel />
    
    <!-- Toast Notifications -->
    <ToastContainer />

  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, watch, ref, computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { v4 as uuidv4 } from 'uuid'
import type { NodeMouseEvent } from '@vue-flow/core'
import { useWorkflowStore } from '../stores/workflows'
import { useNodeStore } from '../stores/nodes'
import { useAuthStore } from '../stores/auth'
import WorkflowDesignerHeader from '../components/workflow/WorkflowDesignerHeader.vue'
import WorkflowCanvas from '../components/workflow/WorkflowCanvas.vue'
import ValidationNotifications from '../components/workflow/ValidationNotifications.vue'
import NodeLibraryModal from '../components/panels/NodeLibraryModal.vue'
import AiChatModal from '../components/workflow/AiChatModal.vue'
import NodePropertiesPanel from '../components/panels/NodePropertiesPanel.vue'
import ExecutionSidePanel from '../components/panels/ExecutionSidePanel.vue'
import NodeInspector from '../components/panels/NodeInspector.vue'
import JsonViewModal from '../components/common/JsonViewModal.vue'
import { useWorkflowData } from '../composables/useWorkflowData'
import { useExecutionTracing } from '../composables/useExecutionTracing'
import { useVueFlowInteraction } from '../composables/useVueFlowInteraction'
import { usePanelState } from '../composables/usePanelState'
import { useHttpLoop } from '../composables/useHttpLoop'
import ToastContainer from '../components/common/ToastContainer.vue'
import type { NodeTypeDefinition, WorkflowNodeData, WorkflowNode, HttpRequestConfig } from '../types/nodes'

const route = useRoute()
const router = useRouter()
const workflowStore = useWorkflowStore()
const nodeStore = useNodeStore()
const authStore = useAuthStore()

// JSON view state
const showJsonModal = ref(false)

// Composables
const {
  workflowId,
  workflowName,
  saving,
  navigateBack,
  loadWorkflowData,
  updateWorkflowName,
  saveWorkflow,
  resetWorkflow
} = useWorkflowData()

const {
  tracingExecution,
  showNodeInspector,
  inspectedNode,
  onTraceExecution,
  clearExecutionTracing
} = useExecutionTracing()

const {
  showExecutionsPanel,
  showNodeLibrary,
  toggleExecutionsPanel,
  closeExecutionsPanel,
  toggleNodeLibrary
} = usePanelState()

// AI Chat state
const showAiChat = ref(false)
function toggleAiChat() {
  showAiChat.value = !showAiChat.value
}
function closeAiChat() {
  showAiChat.value = false
}

const {
  onNodeClick,
  onEdgeClick,
  onPaneClick,
  onConnect,
  onNodesDelete,
  onDrop,
  handleKeyDown
} = useVueFlowInteraction()


// HTTP Loop integration
const {
  loopStatuses,
  refreshLoopData,
  stopPolling,
  setExecutionId
} = useHttpLoop()

// Check if any HTTP nodes have loop configurations and refresh loop data if needed
const checkAndRefreshLoopData = async () => {
  const httpNodesWithLoops = nodeStore.nodes.filter((node: WorkflowNode) => {
    if (node.type === 'http-request') {
      const config = node.data.config as HttpRequestConfig
      return config.loop_config !== undefined && config.loop_config !== null
    }
    return false
  })

  // Only fetch loop data if there are HTTP nodes with loop configurations
  if (httpNodesWithLoops.length > 0) {
    await refreshLoopData()
  }
}

// Computed properties
const workflowJson = computed(() => {
  return {
    id: workflowId.value,
    name: workflowName.value,
    nodes: nodeStore.nodes.map(node => ({
      id: node.id,
      type: node.type,
      position: node.position,
      data: node.data
    })),
    edges: nodeStore.edges.map(edge => ({
      id: edge.id,
      source: edge.source,
      target: edge.target,
      sourceHandle: edge.sourceHandle,
      targetHandle: edge.targetHandle
    }))
  }
})

onMounted(async () => {
  nodeStore.clearWorkflow()

  if (workflowId.value) {
    await workflowStore.fetchWorkflow(workflowId.value)
    if (workflowStore.currentWorkflow) {
      workflowName.value = workflowStore.currentWorkflow.name
      loadWorkflowData()
    }
  } else {
    loadWorkflowData()
  }

  // Don't start automatic polling - load loop data only when needed
  checkAndRefreshLoopData()

  document.addEventListener('keydown', handleKeyDown)
})

onUnmounted(() => {
  // Remove keyboard event listener
  document.removeEventListener('keydown', handleKeyDown)

  // Stop HTTP loop polling
  stopPolling()
})

watch(() => workflowStore.currentWorkflow, (workflow) => {
  if (workflow) {
    workflowName.value = workflow.name
    // Check for loop data when workflow loads
    checkAndRefreshLoopData()
  }
})

// Watch for changes in nodes to refresh loop data if needed
watch(() => nodeStore.nodes, () => {
  checkAndRefreshLoopData()
}, { deep: false })

// Watch for execution tracing changes to update HTTP loop execution filtering
watch(tracingExecution, (newExecution) => {
  if (newExecution) {
    // When tracing starts, filter HTTP loops by the execution ID
    setExecutionId(newExecution.id)
  } else {
    // When tracing stops, clear the execution filter (show all loops)
    setExecutionId(undefined)
  }
})

watch(() => route.params.id, async (newId, oldId) => {
  if (newId !== oldId) {
    nodeStore.clearWorkflow()
    
    if (newId) {
      await workflowStore.fetchWorkflow(newId as string)
      if (workflowStore.currentWorkflow) {
        workflowName.value = workflowStore.currentWorkflow.name
        loadWorkflowData()
      }
    } else {
      workflowName.value = ''
      loadWorkflowData()
    }
  }
})

function onNodesInitialized() {
  console.log('Nodes initialized')
}

function handleInspectNode(node: unknown) {
  inspectedNode.value = node as WorkflowNodeData
  showNodeInspector.value = true
}

function handleNodeClick(event: NodeMouseEvent) {
  onNodeClick(event, tracingExecution, handleInspectNode)
}

function handleCloseInspector() {
  showNodeInspector.value = false
}

function showJsonView() {
  showJsonModal.value = true
}

function handleCloseJsonView() {
  showJsonModal.value = false
}

function closeNodeLibrary() {
  showNodeLibrary.value = false
}

function handleAddNode(nodeType: NodeTypeDefinition) {
  // Find the bottom-most node position
  let bottomMostY = 100 // Default starting position if no nodes exist
  
  if (nodeStore.nodes.length > 0) {
    // Find the node with the highest Y position (bottom-most)
    bottomMostY = Math.max(...nodeStore.nodes.map(node => node.position.y))
    // Add node height (~70px) + 100px gap below the bottom-most node
    bottomMostY += 70 + 100 // Node height + requested 100px gap
  }
  
  const newPosition = {
    x: 400, // Center horizontally
    y: bottomMostY
  }
  
  // Create a unique ID for the node
  const nodeId = uuidv4()
  
  // Generate 12-digit random number for unique naming
  const randomSuffix = Math.floor(Math.random() * 1000000000000).toString().padStart(12, '0')
  
  // Create the node data
  const newNode = {
    id: nodeId,
    type: nodeType.type,
    position: newPosition,
    data: {
      label: `${nodeType.label} ${randomSuffix}`,
      description: nodeType.description,
      config: nodeType.defaultConfig,
      status: 'ready' as const
    }
  }
  
  // Add the node to the store
  nodeStore.addNode(newNode)
  
  // Close the modal
  closeNodeLibrary()
}

function handleLogout() {
  authStore.logout()
  router.push('/login')
}

// Type guard for HTTP request nodes
function isHttpRequestNode(node: WorkflowNode): node is WorkflowNode & {
  data: WorkflowNodeData & { config: HttpRequestConfig }
} {
  return node.type === 'http-request'
}

// Watch for loop status changes and update node data
watch(loopStatuses, (newLoopStatuses) => {
  // Update node data with loop status information
  newLoopStatuses.forEach((loopStatus, loopId) => {
    const httpNodes = nodeStore.nodes
      .filter(isHttpRequestNode)
      .filter(node => node.data.config.loop_config !== undefined)

    // Find nodes that might be associated with this loop
    // Note: This is a simplified approach - in a real implementation,
    // you'd need a proper mapping between loop IDs and node IDs
    httpNodes.forEach(node => {
      const nodeData = node.data as WorkflowNodeData & { loopStatus?: typeof loopStatus }
      if (nodeData.loopStatus?.loop_id === loopId) {
        // Update the node's loop status
        const updatedNode = {
          ...node,
          data: {
            ...node.data,
            loopStatus
          }
        }
        nodeStore.updateNode(node.id, updatedNode)
      }
    })
  })
}, { deep: true })

</script>

<style scoped>
.slide-left-enter-active,
.slide-left-leave-active {
  transition: transform 0.3s ease;
}

.slide-left-enter-from {
  transform: translateX(100%);
}

.slide-left-leave-to {
  transform: translateX(100%);
}
</style>