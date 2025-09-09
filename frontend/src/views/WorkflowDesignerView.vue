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
          <button
            @click="toggleExecutionsPanel"
            class="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded-md font-medium transition-colors flex items-center space-x-2"
          >
            <ClockIcon class="h-4 w-4" />
            <span>Executions</span>
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
          <template #node-email="{ data }">
            <EmailNode :data="data" />
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
          @close="closeExecutionsPanel"
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
      @close="showNodeInspector = false"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useVueFlow } from '@vue-flow/core'
import { VueFlow } from '@vue-flow/core'
import { Controls } from '@vue-flow/controls'
import { Background } from '@vue-flow/background'
import { ArrowLeftIcon, ClockIcon } from '@heroicons/vue/24/outline'
import { useWorkflowStore } from '../stores/workflows'
import { useNodeStore } from '../stores/nodes'
import NodeLibraryPanel from '../components/panels/NodeLibraryPanel.vue'
import NodePropertiesPanel from '../components/panels/NodePropertiesPanel.vue'
import ExecutionSidePanel from '../components/panels/ExecutionSidePanel.vue'
import NodeInspector from '../components/panels/NodeInspector.vue'
import TriggerNode from '../components/nodes/TriggerNode.vue'
import ConditionNode from '../components/nodes/ConditionNode.vue'
import TransformerNode from '../components/nodes/TransformerNode.vue'
import AppNode from '../components/nodes/AppNode.vue'
import EmailNode from '../components/nodes/EmailNode.vue'
import { DEFAULT_CONDITION_SCRIPT, DEFAULT_TRANSFORMER_SCRIPT } from '../constants/defaults'
import { apiClient } from '../services/api'

const router = useRouter()
const route = useRoute()
const workflowStore = useWorkflowStore()
const nodeStore = useNodeStore()
const { project } = useVueFlow()

const workflowId = route.params.id as string
const workflowName = ref('')
const saving = ref(false)
const selectedEdgeId = ref<string | null>(null)
const showExecutionsPanel = ref(false)
const tracingExecution = ref<any>(null)
const executionSteps = ref<any[]>([])
const showNodeInspector = ref(false)
const inspectedNode = ref<any>(null)

// Removed nodeDataUpdateKey as it was causing performance issues with frequent re-renders

onMounted(async () => {
  // Always clear the canvas state when entering a workflow
  nodeStore.clearWorkflow()
  
  if (workflowId) {
    await workflowStore.fetchWorkflow(workflowId)
    if (workflowStore.currentWorkflow) {
      workflowName.value = workflowStore.currentWorkflow.name
      // Load workflow nodes and edges
      loadWorkflowData()
    }
  } else {
    // For new workflows, still load default starter workflow
    loadWorkflowData()
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

// Watch for route changes to clear workflow state when switching between workflows
watch(() => route.params.id, async (newId, oldId) => {
  if (newId !== oldId) {
    // Clear the canvas state when switching workflows
    nodeStore.clearWorkflow()
    
    if (newId) {
      await workflowStore.fetchWorkflow(newId as string)
      if (workflowStore.currentWorkflow) {
        workflowName.value = workflowStore.currentWorkflow.name
        loadWorkflowData()
      }
    } else {
      // For new workflows
      workflowName.value = ''
      loadWorkflowData()
    }
  }
})

function loadWorkflowData() {
  const workflow = workflowStore.currentWorkflow
  
  if (workflow && workflow.nodes && workflow.edges) {
    // Load actual workflow nodes and edges from backend
    
    // Convert API nodes to VueFlow format
    workflow.nodes.forEach((node, index) => {
      const nodeType = convertApiNodeTypeToVueFlowType(node.node_type)
      const vueFlowNode = {
        id: node.id,
        type: nodeType,
        position: { x: node.position_x || (150 + (index * 200)), y: node.position_y || (100 + (Math.floor(index / 3) * 150)) },
        data: {
          label: nodeType === 'trigger' ? 'Start' : node.name,
          description: getNodeDescription(node.node_type),
          config: convertApiNodeConfigToVueFlowConfig(node.node_type),
          status: 'ready' as const
        }
      }
      nodeStore.addNode(vueFlowNode)
    })
    
    // Convert API edges to VueFlow format
    workflow.edges.forEach(edge => {
      const sourceNode = workflow.nodes.find(n => n.name === edge.from_node_name)
      const targetNode = workflow.nodes.find(n => n.name === edge.to_node_name)
      
      if (sourceNode && targetNode) {
        const vueFlowEdge = {
          id: edge.id,
          source: sourceNode.id,
          target: targetNode.id,
          sourceHandle: edge.condition_result === true ? 'true' : edge.condition_result === false ? 'false' : undefined,
          targetHandle: undefined,
          data: {}
        }
        nodeStore.addEdge(vueFlowEdge)
      }
    })
  } else if (nodeStore.nodes.length === 0) {
    // Create default starter workflow only if no nodes exist and no workflow data
    nodeStore.addNode({
      id: 'trigger-1',
      type: 'trigger',
      position: { x: 100, y: 100 },
      data: {
        label: 'Start',
        description: 'HTTP endpoint trigger',
        config: {
          type: 'trigger',
          methods: ['POST']
        },
        status: 'ready' as const
      }
    })
  }
}

function navigateBack() {
  router.push('/workflows')
}

function onNodeClick(event: any) {
  // If we're in tracing mode and the node has execution data, show inspector
  if (tracingExecution.value && event.node.data.isTracing && event.node.data.executionStatus) {
    inspectedNode.value = event.node.data
    showNodeInspector.value = true
    return
  }
  
  // Normal mode - select node for properties panel
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
        status: 'ready' as const
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
    if (!workflowStore.currentWorkflow) {
      console.error('No current workflow to save')
      return
    }

    // Convert Vue Flow nodes to API format
    const apiNodes = nodeStore.nodes.map(node => {
      const apiNode = {
        name: node.type === 'trigger' ? 'Start' : (node.data.label || node.id),
        node_type: convertNodeToApiType(node),
        position_x: node.position.x,
        position_y: node.position.y
      }
      
      // Debug logging for email nodes specifically
      if (node.type === 'email') {
        console.log('Saving email node:', {
          nodeId: node.id,
          rawConfig: node.data.config,
          convertedApiType: apiNode.node_type
        })
      }
      
      return apiNode
    })

    // Convert Vue Flow edges to API format
    const apiEdges = nodeStore.edges.map(edge => {
      const sourceNode = nodeStore.getNodeById(edge.source)
      const targetNode = nodeStore.getNodeById(edge.target)
      return {
        from_node_name: sourceNode?.type === 'trigger' ? 'Start' : (sourceNode?.data.label || edge.source),
        to_node_name: targetNode?.type === 'trigger' ? 'Start' : (targetNode?.data.label || edge.target),
        condition_result: edge.sourceHandle === 'true' ? true : edge.sourceHandle === 'false' ? false : undefined
      }
    })

    // Set start_node_name to 'Start' (hardcoded for trigger)
    const startNodeName = 'Start'

    const workflowData = {
      name: workflowName.value || workflowStore.currentWorkflow.name,
      description: workflowStore.currentWorkflow.description,
      start_node_name: startNodeName,
      nodes: apiNodes,
      edges: apiEdges
    }

    await workflowStore.updateWorkflow(workflowStore.currentWorkflow.id, workflowData)
  } catch (error) {
    console.error('Failed to save workflow:', error)
  } finally {
    saving.value = false
  }
}

function convertApiNodeTypeToVueFlowType(nodeType: any): 'trigger' | 'condition' | 'transformer' | 'app' | 'email' {
  if (nodeType.Trigger) return 'trigger'
  if (nodeType.Condition) return 'condition'
  if (nodeType.Transformer) return 'transformer'
  if (nodeType.App) return 'app'
  if (nodeType.Email) return 'email'
  return 'app' // fallback
}

function getNodeDescription(nodeType: any): string {
  if (nodeType.Trigger) return 'HTTP endpoint trigger'
  if (nodeType.Condition) return 'Conditional logic node'
  if (nodeType.Transformer) return 'Data transformation node'
  if (nodeType.App) return 'External application node'
  if (nodeType.Email) return 'Email notification node'
  return 'Unknown node type'
}

function convertApiNodeConfigToVueFlowConfig(nodeType: any): any {
  if (nodeType.Trigger) {
    return {
      type: 'trigger',
      methods: nodeType.Trigger.methods || ['POST']
    }
  }
  if (nodeType.Condition) {
    return {
      type: 'condition',
      script: nodeType.Condition.script || DEFAULT_CONDITION_SCRIPT
    }
  }
  if (nodeType.Transformer) {
    return {
      type: 'transformer',
      script: nodeType.Transformer.script || DEFAULT_TRANSFORMER_SCRIPT
    }
  }
  if (nodeType.App) {
    const config = {
      type: 'app',
      app_type: nodeType.App.app_type || 'Webhook',
      url: nodeType.App.url || 'https://httpbin.org/post',
      method: nodeType.App.method || 'POST',
      timeout_seconds: nodeType.App.timeout_seconds || 30,
      failure_action: nodeType.App.failure_action || 'Stop',
      headers: nodeType.App.headers || {},
      openobserve_url: '',
      authorization_header: '',
      retry_config: nodeType.App.retry_config || {
        max_attempts: 3,
        initial_delay_ms: 100,
        max_delay_ms: 5000,
        backoff_multiplier: 2.0
      }
    }
    
    // Handle OpenObserve specific fields
    if (typeof nodeType.App.app_type === 'object' && nodeType.App.app_type.OpenObserve) {
      config.app_type = 'OpenObserve'
      config.openobserve_url = nodeType.App.app_type.OpenObserve.url || ''
      config.authorization_header = nodeType.App.app_type.OpenObserve.authorization_header || ''
    }
    
    return config
  }
  if (nodeType.Email) {
    const emailConfig = nodeType.Email.config
    return {
      type: 'email',
      smtp_config: emailConfig.smtp_config || 'default',
      from: emailConfig.from || {
        email: 'noreply@company.com',
        name: 'SwissPipe Workflow'
      },
      to: emailConfig.to || [{
        email: '{{ workflow.data.user_email }}',
        name: '{{ workflow.data.user_name }}'
      }],
      cc: emailConfig.cc || [],
      bcc: emailConfig.bcc || [],
      subject: emailConfig.subject || 'Workflow {{ workflow.name }} completed',
      template_type: emailConfig.template_type || 'html',
      body_template: emailConfig.body_template || '<!DOCTYPE html><html><body><h1>Workflow Results</h1><p>Status: {{ workflow.status }}</p><p>Data: {{ workflow.data | json }}</p></body></html>',
      text_body_template: emailConfig.text_body_template,
      attachments: emailConfig.attachments || [],
      priority: emailConfig.priority ? emailConfig.priority.toLowerCase() : 'normal',
      delivery_receipt: emailConfig.delivery_receipt || false,
      read_receipt: emailConfig.read_receipt || false,
      queue_if_rate_limited: emailConfig.queue_if_rate_limited !== undefined ? emailConfig.queue_if_rate_limited : true,
      max_queue_wait_minutes: emailConfig.max_queue_wait_minutes || 60,
      bypass_rate_limit: emailConfig.bypass_rate_limit || false
    }
  }
  return {}
}

function convertNodeToApiType(node: any) {
  switch (node.type) {
    case 'trigger':
      return {
        Trigger: {
          methods: node.data.config.methods || ['POST']
        }
      }
    case 'condition':
      return {
        Condition: {
          script: node.data.config.script || DEFAULT_CONDITION_SCRIPT
        }
      }
    case 'transformer':
      return {
        Transformer: {
          script: node.data.config.script || DEFAULT_TRANSFORMER_SCRIPT
        }
      }
    case 'app':
      const appConfig = node.data.config
      let app_type = appConfig.app_type || 'Webhook'
      
      // Handle OpenObserve as structured type
      if (appConfig.app_type === 'OpenObserve') {
        app_type = {
          OpenObserve: {
            url: appConfig.openobserve_url || '',
            authorization_header: appConfig.authorization_header || ''
          }
        }
      }
      
      return {
        App: {
          app_type: app_type,
          url: appConfig.url || 'https://httpbin.org/post',
          method: appConfig.method || 'POST',
          timeout_seconds: appConfig.timeout_seconds || 30,
          failure_action: appConfig.failure_action || 'Stop',
          headers: appConfig.headers || {},
          retry_config: appConfig.retry_config || {
            max_attempts: 3,
            initial_delay_ms: 100,
            max_delay_ms: 5000,
            backoff_multiplier: 2.0
          }
        }
      }
    case 'email':
      const emailConfig = node.data.config
      return {
        Email: {
          config: {
            smtp_config: emailConfig.smtp_config || 'default',
            from: emailConfig.from || {
              email: 'noreply@company.com',
              name: 'SwissPipe Workflow'
            },
            to: emailConfig.to || [{
              email: '{{ workflow.data.user_email }}',
              name: '{{ workflow.data.user_name }}'
            }],
            cc: emailConfig.cc || [],
            bcc: emailConfig.bcc || [],
            subject: emailConfig.subject || 'Workflow {{ workflow.name }} completed',
            template_type: emailConfig.template_type || 'html',
            body_template: emailConfig.body_template || '<!DOCTYPE html><html><body><h1>Workflow Results</h1><p>Status: {{ workflow.status }}</p><p>Data: {{ workflow.data | json }}</p></body></html>',
            text_body_template: emailConfig.text_body_template,
            attachments: emailConfig.attachments || [],
            priority: emailConfig.priority ? emailConfig.priority.charAt(0).toUpperCase() + emailConfig.priority.slice(1).toLowerCase() : 'Normal',
            delivery_receipt: emailConfig.delivery_receipt || false,
            read_receipt: emailConfig.read_receipt || false,
            queue_if_rate_limited: emailConfig.queue_if_rate_limited !== undefined ? emailConfig.queue_if_rate_limited : true,
            max_queue_wait_minutes: emailConfig.max_queue_wait_minutes || 60,
            bypass_rate_limit: emailConfig.bypass_rate_limit || false
          }
        }
      }
    default:
      throw new Error(`Unknown node type: ${node.type}`)
  }
}

function resetWorkflow() {
  nodeStore.clearWorkflow()
  loadWorkflowData()
}

function toggleExecutionsPanel() {
  showExecutionsPanel.value = !showExecutionsPanel.value
}

function closeExecutionsPanel() {
  showExecutionsPanel.value = false
  clearExecutionTracing()
}

async function onTraceExecution(executionData: any) {
  console.log('Tracing execution:', executionData)
  tracingExecution.value = executionData
  
  try {
    // Fetch execution steps
    const data = await apiClient.getExecutionSteps(executionData.id)
    executionSteps.value = data.steps || []
    
    // Update node states based on execution steps
    updateNodeExecutionStates()
    
    // Animate execution path
    animateExecutionPath()
  } catch (error) {
    console.error('Failed to fetch execution steps:', error)
  }
}

function updateNodeExecutionStates() {
  // Create a map of node names to their execution status
  const nodeExecutionMap = new Map()
  
  executionSteps.value.forEach(step => {
    nodeExecutionMap.set(step.node_name, {
      status: step.status,
      duration: step.duration_ms,
      error: step.error_message,
      input: step.input_data,
      output: step.output_data
    })
  })
  
  // Update each node's data with execution state
  nodeStore.nodes.forEach(node => {
    const nodeName = node.type === 'trigger' ? 'Start' : node.data.label
    const executionData = nodeExecutionMap.get(nodeName)
    
    if (executionData) {
      // Update node data with execution state
      node.data = {
        ...node.data,
        executionStatus: executionData.status,
        executionDuration: executionData.duration,
        executionError: executionData.error,
        executionInput: executionData.input,
        executionOutput: executionData.output,
        isTracing: true
      }
    } else {
      // Clear execution state if no data
      node.data = {
        ...node.data,
        executionStatus: null,
        executionDuration: null,
        executionError: null,
        executionInput: null,
        executionOutput: null,
        isTracing: true
      }
    }
  })
  
  // Update edge styles for execution path highlighting
  updateEdgeExecutionStyles()
  
  // Vue reactivity will handle the update automatically
}

function updateEdgeExecutionStyles() {
  // Get the execution path from the steps
  const executedNodeNames = new Set(executionSteps.value.map(step => step.node_name))
  
  // Update all edges to show execution path highlighting
  nodeStore.edges.forEach((edge: any) => {
    // Find source and target nodes
    const sourceNode = nodeStore.nodes.find(n => n.id === edge.source)
    const targetNode = nodeStore.nodes.find(n => n.id === edge.target)
    
    if (sourceNode && targetNode) {
      const sourceName = sourceNode.type === 'trigger' ? 'Start' : sourceNode.data.label
      const targetName = targetNode.type === 'trigger' ? 'Start' : targetNode.data.label
      
      // Check if both nodes were executed (part of execution path)
      const isExecutionPath = executedNodeNames.has(sourceName) && executedNodeNames.has(targetName)
      
      if (isExecutionPath) {
        // Highlight execution path edges
        edge.style = {
          ...edge.style,
          strokeWidth: 4,
          stroke: '#3b82f6', // Blue color for execution path
          strokeDasharray: '0', // Remove any dashed pattern
          opacity: 1,
          transition: 'all 0.3s ease'
        }
        edge.markerEnd = {
          type: 'arrowclosed',
          color: '#3b82f6',
          width: 20,
          height: 20
        }
      } else {
        // Dim non-execution edges
        edge.style = {
          ...edge.style,
          strokeWidth: 1,
          stroke: '#6b7280', // Gray color for non-execution paths
          opacity: 0.3,
          transition: 'all 0.3s ease'
        }
        edge.markerEnd = {
          type: 'arrowclosed',
          color: '#6b7280',
          width: 15,
          height: 15
        }
      }
    }
  })
}

function animateExecutionPath() {
  // TODO: Add animation to show execution flow
  // This could highlight edges in sequence to show the path taken
  console.log('Animating execution path with steps:', executionSteps.value)
}

function clearExecutionTracing() {
  tracingExecution.value = null
  executionSteps.value = []
  showNodeInspector.value = false
  inspectedNode.value = null
  
  // Clear execution state from all nodes
  nodeStore.nodes.forEach((node: any) => {
    node.data = {
      ...node.data,
      executionStatus: null,
      executionDuration: null,
      executionError: null,
      executionInput: null,
      executionOutput: null,
      isTracing: false
    }
  })
  
  // Reset edge styles to default
  nodeStore.edges.forEach((edge: any) => {
    edge.style = {
      strokeWidth: 2,
      stroke: '#6b7280',
      opacity: 1,
      transition: 'all 0.3s ease'
    }
    edge.markerEnd = {
      type: 'arrowclosed',
      color: '#6b7280',
      width: 15,
      height: 15
    }
  })
  
  // Vue reactivity will handle the update automatically
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