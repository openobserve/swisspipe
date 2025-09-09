import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { WorkflowNode, WorkflowEdge, NodeTypeDefinition, ValidationState } from '../types/nodes'
import { DEFAULT_CONDITION_SCRIPT, DEFAULT_TRANSFORMER_SCRIPT } from '../constants/defaults'

export const useNodeStore = defineStore('nodes', () => {
  // State
  const nodes = ref<WorkflowNode[]>([])
  const edges = ref<WorkflowEdge[]>([])
  const selectedNode = ref<string | null>(null)
  const validation = ref<ValidationState>({
    isValid: true,
    errors: [],
    warnings: []
  })

  // Node type definitions
  const nodeTypes = ref<NodeTypeDefinition[]>([
    {
      type: 'trigger',
      label: 'Trigger',
      description: 'Input data from configured sources',
      color: '#3b82f6',
      icon: 'play',
      defaultConfig: {
        type: 'trigger',
        methods: ['POST']
      }
    },
    {
      type: 'condition',
      label: 'Condition',
      description: 'Branch workflow based on conditions',
      color: '#f59e0b',
      icon: 'question-mark-circle',
      defaultConfig: {
        type: 'condition',
        script: DEFAULT_CONDITION_SCRIPT
      }
    },
    {
      type: 'transformer',
      label: 'Transformer',
      description: 'Process and modify data',
      color: '#8b5cf6',
      icon: 'arrow-path',
      defaultConfig: {
        type: 'transformer',
        script: DEFAULT_TRANSFORMER_SCRIPT
      }
    },
    {
      type: 'app',
      label: 'App',
      description: 'Send data to external systems',
      color: '#10b981',
      icon: 'cube',
      defaultConfig: {
        type: 'app',
        app_type: 'Webhook',
        url: 'https://httpbin.org/post',
        method: 'POST',
        timeout_seconds: 30,
        failure_action: 'Stop',
        headers: {},
        openobserve_url: '',
        authorization_header: '',
        stream_name: 'default',
        retry_config: {
          max_attempts: 3,
          initial_delay_ms: 100,
          max_delay_ms: 5000,
          backoff_multiplier: 2.0
        }
      }
    },
    {
      type: 'email',
      label: 'Email',
      description: 'Send email notifications and reports',
      color: '#2196F3',
      icon: 'envelope',
      defaultConfig: {
        type: 'email',
        smtp_config: 'default',
        from: {
          email: 'noreply@company.com',
          name: 'SwissPipe Workflow'
        },
        to: [{
          email: '{{ event.data.user_email }}',
          name: '{{ event.data.user_name }}'
        }],
        cc: [],
        bcc: [],
        subject: 'Workflow completed',
        template_type: 'html',
        body_template: '<!DOCTYPE html><html><body><h1>Workflow Results</h1><p>Data: {{json event.data}}</p></body></html>',
        text_body_template: 'Workflow Results\nData: {{json event.data}}',
        attachments: [],
        priority: 'normal',
        delivery_receipt: false,
        read_receipt: false,
        queue_if_rate_limited: true,
        max_queue_wait_minutes: 60,
        bypass_rate_limit: false
      }
    }
  ])

  // Getters
  const getNodeById = computed(() => (id: string) => nodes.value.find(node => node.id === id))
  const getEdgeById = computed(() => (id: string) => edges.value.find(edge => edge.id === id))
  const selectedNodeData = computed(() => selectedNode.value ? getNodeById.value(selectedNode.value) : null)
  const nodeTypeByType = computed(() => (type: string) => nodeTypes.value.find(nt => nt.type === type))

  // Actions
  function addNode(node: WorkflowNode) {
    nodes.value.push(node)
    validateWorkflow()
  }

  function updateNode(id: string, updates: Partial<WorkflowNode>) {
    const index = nodes.value.findIndex(node => node.id === id)
    if (index !== -1) {
      nodes.value[index] = { ...nodes.value[index], ...updates }
      validateWorkflow()
    }
  }

  function deleteNode(id: string) {
    const nodeToDelete = nodes.value.find(node => node.id === id)
    
    // Prevent deletion of trigger nodes
    if (nodeToDelete && nodeToDelete.type === 'trigger') {
      console.warn('Cannot delete trigger nodes')
      return false
    }
    
    nodes.value = nodes.value.filter(node => node.id !== id)
    edges.value = edges.value.filter(edge => edge.source !== id && edge.target !== id)
    if (selectedNode.value === id) {
      selectedNode.value = null
    }
    validateWorkflow()
    return true
  }

  function addEdge(edge: WorkflowEdge) {
    edges.value.push(edge)
    validateWorkflow()
  }

  function updateEdge(id: string, updates: Partial<WorkflowEdge>) {
    const index = edges.value.findIndex(edge => edge.id === id)
    if (index !== -1) {
      edges.value[index] = { ...edges.value[index], ...updates }
      validateWorkflow()
    }
  }

  function deleteEdge(id: string) {
    edges.value = edges.value.filter(edge => edge.id !== id)
    validateWorkflow()
  }

  function setSelectedNode(id: string | null) {
    selectedNode.value = id
  }

  function clearWorkflow() {
    nodes.value = []
    edges.value = []
    selectedNode.value = null
    validation.value = {
      isValid: true,
      errors: [],
      warnings: []
    }
  }

  function validateWorkflow() {
    const errors: string[] = []
    const warnings: string[] = []

    // Check for duplicate node names
    const nodeNames = new Set()
    for (const node of nodes.value) {
      if (nodeNames.has(node.data.label)) {
        errors.push(`Duplicate node name: ${node.data.label}`)
      }
      nodeNames.add(node.data.label)
    }

    // Check for orphaned nodes (no connections)
    const connectedNodes = new Set([
      ...edges.value.map(e => e.source),
      ...edges.value.map(e => e.target)
    ])
    
    for (const node of nodes.value) {
      if (!connectedNodes.has(node.id) && nodes.value.length > 1) {
        warnings.push(`Node '${node.data.label}' is not connected to any other nodes`)
      }
    }

    // Check for condition nodes without both true/false edges
    for (const node of nodes.value) {
      if (node.type === 'condition') {
        const outgoingEdges = edges.value.filter(e => e.source === node.id)
        const hasTrue = outgoingEdges.some(e => e.sourceHandle === 'true')
        const hasFalse = outgoingEdges.some(e => e.sourceHandle === 'false')
        
        if (!hasTrue) {
          warnings.push(`Condition node '${node.data.label}' has no true branch`)
        }
        if (!hasFalse) {
          warnings.push(`Condition node '${node.data.label}' has no false branch`)
        }
      }
    }

    validation.value = {
      isValid: errors.length === 0,
      errors,
      warnings
    }
  }

  return {
    // State
    nodes,
    edges,
    selectedNode,
    nodeTypes,
    validation,
    // Getters
    getNodeById,
    getEdgeById,
    selectedNodeData,
    nodeTypeByType,
    // Actions
    addNode,
    updateNode,
    deleteNode,
    addEdge,
    updateEdge,
    deleteEdge,
    setSelectedNode,
    clearWorkflow,
    validateWorkflow
  }
})