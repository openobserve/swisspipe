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
            @click="resetWorkflow"
            class="bg-gray-600 hover:bg-gray-700 text-white px-4 py-2 rounded-md font-medium transition-colors"
          >
            Reset
          </button>
        </div>
      </div>
    </header>

    <!-- Main Content -->
    <div class="flex flex-1 overflow-hidden">
      <!-- Node Library Panel -->
      <div class="w-80 glass-medium border-r border-slate-700/50 flex-shrink-0 overflow-y-auto">
        <NodeLibraryPanel />
      </div>

      <!-- Canvas Area -->
      <div
        class="flex-1 relative"
        @drop="onDrop"
        @dragover.prevent
        @dragenter.prevent
      >
        <VueFlow
          :key="nodeDataUpdateKey"
          v-model:nodes="nodeStore.nodes"
          v-model:edges="nodeStore.edges"
          @node-click="onNodeClick"
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
          <template #node-app="{ data }">
            <AppNode :data="data" />
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
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useVueFlow } from '@vue-flow/core'
import { VueFlow } from '@vue-flow/core'
import { Controls } from '@vue-flow/controls'
import { Background } from '@vue-flow/background'
import { ArrowLeftIcon } from '@heroicons/vue/24/outline'
import { useWorkflowStore } from '../stores/workflows'
import { useNodeStore } from '../stores/nodes'
import NodeLibraryPanel from '../components/panels/NodeLibraryPanel.vue'
import NodePropertiesPanel from '../components/panels/NodePropertiesPanel.vue'
import TriggerNode from '../components/nodes/TriggerNode.vue'
import ConditionNode from '../components/nodes/ConditionNode.vue'
import TransformerNode from '../components/nodes/TransformerNode.vue'
import AppNode from '../components/nodes/AppNode.vue'

const router = useRouter()
const route = useRoute()
const workflowStore = useWorkflowStore()
const nodeStore = useNodeStore()
const { project } = useVueFlow()

const workflowId = route.params.id as string
const workflowName = ref('')
const saving = ref(false)
const selectedEdgeId = ref<string | null>(null)

// Key to force VueFlow re-render when node data changes
const nodeDataUpdateKey = computed(() => {
  return nodeStore.nodes.map(n => `${n.id}-${JSON.stringify(n.data)}`).join('|')
})

onMounted(async () => {
  if (workflowId) {
    await workflowStore.fetchWorkflow(workflowId)
    if (workflowStore.currentWorkflow) {
      workflowName.value = workflowStore.currentWorkflow.name
      // Load workflow nodes and edges
      loadWorkflowData()
    }
  }
  
  // Add keyboard event listener
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

function loadWorkflowData() {
  // TODO: Load actual workflow nodes and edges from backend
  // For now, create a simple starter workflow
  if (nodeStore.nodes.length === 0) {
    nodeStore.addNode({
      id: 'trigger-1',
      type: 'trigger',
      position: { x: 100, y: 100 },
      data: {
        label: 'Workflow Trigger',
        description: 'HTTP endpoint trigger',
        config: {
          type: 'trigger',
          methods: ['POST']
        },
        status: 'ready'
      }
    })
  }
}

function navigateBack() {
  router.push('/workflows')
}

function onNodeClick(event: any) {
  nodeStore.setSelectedNode(event.node.id)
  // Clear edge selection when node is selected
  selectedEdgeId.value = null
}

function onEdgeClick(event: any) {
  selectedEdgeId.value = event.edge.id
  // Clear node selection when edge is selected
  nodeStore.setSelectedNode(null)
}

function onPaneClick() {
  // Clear all selections when clicking on empty canvas
  nodeStore.setSelectedNode(null)
  selectedEdgeId.value = null
}

function onConnect(params: any) {
  const edge = {
    id: `edge-${params.source}-${params.target}`,
    source: params.source,
    target: params.target,
    sourceHandle: params.sourceHandle,
    targetHandle: params.targetHandle,
    data: {}
  }
  nodeStore.addEdge(edge)
}

function onNodesInitialized() {
  console.log('Nodes initialized')
}

function onNodesDelete(event: any) {
  // Prevent deletion of trigger nodes
  const nodesToDelete = event.nodes || []
  const triggerNodes = nodesToDelete.filter((node: any) => node.type === 'trigger')
  
  if (triggerNodes.length > 0) {
    console.warn('Cannot delete trigger nodes')
    // Return false or cancel the deletion
    event.preventDefault?.()
    return false
  }
  
  // Allow deletion of non-trigger nodes
  return true
}

function onDrop(event: DragEvent) {
  event.preventDefault()
  
  if (!event.dataTransfer) return
  
  try {
    const nodeTypeData = JSON.parse(event.dataTransfer.getData('application/vueflow'))
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect()
    const position = project({
      x: event.clientX - rect.left,
      y: event.clientY - rect.top
    })
    
    const nodeId = `${nodeTypeData.type}-${Date.now()}`
    const newNode = {
      id: nodeId,
      type: nodeTypeData.type,
      position,
      data: {
        label: nodeTypeData.label,
        description: nodeTypeData.description,
        config: { ...nodeTypeData.defaultConfig },
        status: 'ready'
      }
    }
    
    nodeStore.addNode(newNode)
  } catch (error) {
    console.error('Error parsing dropped node data:', error)
  }
}

async function updateWorkflowName() {
  if (workflowStore.currentWorkflow && workflowName.value !== workflowStore.currentWorkflow.name) {
    // TODO: Update workflow name in backend
    console.log('Update workflow name:', workflowName.value)
  }
}

async function saveWorkflow() {
  saving.value = true
  try {
    // TODO: Convert nodes and edges to API format and save
    console.log('Saving workflow:', {
      nodes: nodeStore.nodes,
      edges: nodeStore.edges
    })
  } catch (error) {
    console.error('Failed to save workflow:', error)
  } finally {
    saving.value = false
  }
}

function resetWorkflow() {
  nodeStore.clearWorkflow()
  loadWorkflowData()
}

function handleKeyDown(event: KeyboardEvent) {
  // Prevent deletion when user is typing in input fields, textareas, or contenteditable elements
  const target = event.target as HTMLElement
  if (target && (
    target.tagName === 'INPUT' ||
    target.tagName === 'TEXTAREA' ||
    target.tagName === 'SELECT' ||
    target.contentEditable === 'true' ||
    target.isContentEditable
  )) {
    return
  }

  // Handle Delete and Backspace keys
  if (event.key === 'Delete' || event.key === 'Backspace') {
    event.preventDefault()
    
    // Check if an edge is selected
    if (selectedEdgeId.value) {
      nodeStore.deleteEdge(selectedEdgeId.value)
      selectedEdgeId.value = null
      return
    }
    
    // Check if a node is selected
    if (nodeStore.selectedNode) {
      const selectedNodeData = nodeStore.getNodeById(nodeStore.selectedNode)
      if (selectedNodeData) {
        if (selectedNodeData.type === 'trigger') {
          // Prevent deletion of trigger nodes
          console.warn('Cannot delete trigger nodes')
          return false
        } else {
          // Allow deletion of non-trigger nodes
          nodeStore.deleteNode(selectedNodeData.id)
        }
      }
    }
  }
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