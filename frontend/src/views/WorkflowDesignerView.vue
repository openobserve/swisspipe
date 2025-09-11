<template>
  <div class="h-screen text-gray-100 flex flex-col">
    <!-- Header -->
    <header class="glass-dark border-b border-slate-700/50 flex-shrink-0">
      <div class="px-6 py-4 flex items-center justify-between">
        <div class="flex items-center space-x-4">
          <button
            @click="navigateBack"
            class="text-gray-400 hover:text-gray-200 transition-colors"
          >
            <ArrowLeftIcon class="h-6 w-6" />
          </button>
          <input
            v-model="workflowName"
            @blur="updateWorkflowName"
            class="bg-transparent text-xl font-medium text-white focus:outline-none focus:bg-white/5 focus:backdrop-blur-sm px-2 py-1 rounded transition-all duration-200"
            placeholder="Workflow Name"
          />
        </div>
        <div class="flex items-center space-x-3">
          <button
            @click="saveWorkflow"
            :disabled="saving"
            class="bg-green-600 hover:bg-green-700 disabled:bg-gray-600 text-white px-4 py-2 rounded-md font-medium transition-colors"
          >
            {{ saving ? 'Saving...' : 'Save' }}
          </button>
          <button
            @click="showJsonView"
            class="bg-indigo-600 hover:bg-indigo-700 text-white px-4 py-2 rounded-md font-medium transition-colors"
          >
            JSON View
          </button>
          <button
            @click="resetWorkflow"
            class="bg-gray-600 hover:bg-gray-700 text-white px-4 py-2 rounded-md font-medium transition-colors"
          >
            Reset
          </button>
          <button
            @click="toggleNodeLibrary"
            class="bg-purple-600 hover:bg-purple-700 text-white px-4 py-2 rounded-md font-medium transition-colors flex items-center space-x-2"
            title="Node Library"
          >
            <Squares2X2Icon class="h-4 w-4" />
            <span>Node Library</span>
          </button>
          <button
            @click="toggleExecutionsPanel"
            class="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded-md font-medium transition-colors flex items-center space-x-2"
          >
            <ClockIcon class="h-4 w-4" />
            <span>Executions</span>
          </button>
          <div class="flex items-center space-x-3 ml-4 border-l border-gray-600 pl-4">
            <span class="text-sm text-gray-300">
              {{ authStore.user?.username }}
            </span>
            <button
              @click="handleLogout"
              class="text-gray-300 hover:text-white px-3 py-2 rounded-md text-sm font-medium transition-colors"
            >
              Logout
            </button>
          </div>
        </div>
      </div>
    </header>

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

      <!-- Canvas Area -->
      <div
        class="flex-1 relative"
        @drop="onDrop"
        @dragover.prevent
        @dragenter.prevent
      >
        <VueFlow
          v-model:nodes="nodeStore.nodes"
          v-model:edges="nodeStore.edges"
          @node-click="handleNodeClick"
          @edge-click="onEdgeClick"
          @pane-click="onPaneClick"
          @connect="onConnect"
          @nodes-initialized="onNodesInitialized"
          @nodes-delete="onNodesDelete"
          class="vue-flow-dark"
          :default-viewport="{ zoom: 1 }"
          :min-zoom="0.2"
          :max-zoom="4"
          :delete-key-code="null"
        >
          <Background pattern-color="#a7abb0" :gap="20" />
          <Controls />
          
          <!-- Custom Node Types -->
          <template #node-trigger="{ data }">
            <TriggerNode :data="data" />
          </template>
          <template #node-condition="{ data }">
            <ConditionNode :data="data" />
          </template>
          <template #node-transformer="{ data }">
            <TransformerNode :data="data" />
          </template>
          <template #node-webhook="{ data }">
            <WebhookNode :data="data" />
          </template>
          <template #node-openobserve="{ data }">
            <OpenObserveNode :data="data" />
          </template>
          <template #node-app="{ data }">
            <AppNode :data="data" />
          </template>
          <template #node-email="{ data }">
            <EmailNode :data="data" />
          </template>
          <template #node-delay="{ data }">
            <DelayNode :data="data" />
          </template>
        </VueFlow>
      </div>

      <!-- Properties Panel (slide out when node selected) -->
      <div
        v-if="nodeStore.selectedNode && nodeStore.selectedNodeData"
        class="w-[768px] border-l border-slate-700/50 flex-shrink-0 overflow-y-auto bg-transparent"
        style="background: none !important;"
      >
        <NodePropertiesPanel />
      </div>

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
    <div
      v-if="!nodeStore.validation.isValid || nodeStore.validation.warnings.length"
      class="fixed bottom-4 right-4 max-w-md"
    >
      <div
        v-if="!nodeStore.validation.isValid"
        class="glass-dark bg-red-900/50 border border-red-700/50 text-red-100 px-4 py-3 rounded-lg mb-2 shadow-2xl"
      >
        <h4 class="font-medium">Validation Errors:</h4>
        <ul class="mt-1 text-sm">
          <li v-for="error in nodeStore.validation.errors" :key="error">
            • {{ error }}
          </li>
        </ul>
      </div>
      
      <div
        v-if="nodeStore.validation.warnings.length"
        class="glass-dark bg-yellow-900/50 border border-yellow-700/50 text-yellow-100 px-4 py-3 rounded-lg shadow-2xl"
      >
        <h4 class="font-medium">Warnings:</h4>
        <ul class="mt-1 text-sm">
          <li v-for="warning in nodeStore.validation.warnings" :key="warning">
            • {{ warning }}
          </li>
        </ul>
      </div>
    </div>

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
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, watch, ref, computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { VueFlow } from '@vue-flow/core'
import { Controls } from '@vue-flow/controls'
import { Background } from '@vue-flow/background'
import { ArrowLeftIcon, ClockIcon, Squares2X2Icon } from '@heroicons/vue/24/outline'
import { useWorkflowStore } from '../stores/workflows'
import { useNodeStore } from '../stores/nodes'
import { useAuthStore } from '../stores/auth'
import NodeLibraryModal from '../components/panels/NodeLibraryModal.vue'
import NodePropertiesPanel from '../components/panels/NodePropertiesPanel.vue'
import ExecutionSidePanel from '../components/panels/ExecutionSidePanel.vue'
import NodeInspector from '../components/panels/NodeInspector.vue'
import JsonViewModal from '../components/common/JsonViewModal.vue'
import TriggerNode from '../components/nodes/TriggerNode.vue'
import ConditionNode from '../components/nodes/ConditionNode.vue'
import TransformerNode from '../components/nodes/TransformerNode.vue'
import WebhookNode from '../components/nodes/WebhookNode.vue'
import OpenObserveNode from '../components/nodes/OpenObserveNode.vue'
import AppNode from '../components/nodes/AppNode.vue'
import EmailNode from '../components/nodes/EmailNode.vue'
import DelayNode from '../components/nodes/DelayNode.vue'
import { useWorkflowData } from '../composables/useWorkflowData'
import { useExecutionTracing } from '../composables/useExecutionTracing'
import { useVueFlowInteraction } from '../composables/useVueFlowInteraction'
import { usePanelState } from '../composables/usePanelState'

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

const {
  selectedEdgeId,
  onNodeClick,
  onEdgeClick,
  onPaneClick,
  onConnect,
  onNodesDelete,
  onDrop,
  handleKeyDown
} = useVueFlowInteraction()

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
  
  document.addEventListener('keydown', handleKeyDown)
})

onUnmounted(() => {
  // Remove keyboard event listener
  document.removeEventListener('keydown', handleKeyDown)
})

watch(() => workflowStore.currentWorkflow, (workflow) => {
  if (workflow) {
    workflowName.value = workflow.name
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

function handleInspectNode(node: any) {
  inspectedNode.value = node
  showNodeInspector.value = true
}

function handleNodeClick(event: any) {
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

function handleAddNode(nodeType: any) {
  // Add node at the center of the viewport
  const centerPosition = {
    x: 400,
    y: 300
  }
  
  // Create a unique ID for the node
  const nodeId = `node-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`
  
  // Create the node data
  const newNode = {
    id: nodeId,
    type: nodeType.type,
    position: centerPosition,
    data: {
      label: nodeType.label,
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

.vue-flow-dark {
  background: #1a1a2e;
}

.vue-flow-dark .vue-flow__minimap {
  background-color: #16213e;
}

.vue-flow-dark .vue-flow__controls {
  button {
    background-color: #374151;
    border-color: #4b5563;
    color: #d1d5db;
  }
  
  button:hover {
    background-color: #4b5563;
  }
}
</style>