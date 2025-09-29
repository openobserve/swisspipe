import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { WorkflowNode, WorkflowEdge, NodeTypeDefinition, ValidationState } from '../types/nodes'
import { DEFAULT_CONDITION_SCRIPT, DEFAULT_TRANSFORMER_SCRIPT } from '../constants/defaults'
import {
  DEFAULT_EMAIL_CONFIG,
  DEFAULT_HTTP_CONFIG,
  DEFAULT_OPENOBSERVE_CONFIG,
  DEFAULT_DELAY_CONFIG,
  DEFAULT_ANTHROPIC_CONFIG,
  DEFAULT_HUMAN_IN_LOOP_CONFIG,
  NODE_LIBRARY_DEFINITIONS
} from '../constants/nodeDefaults'

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

  // Helper function to create node type definitions
  function createNodeTypeDefinition(type: keyof typeof NODE_LIBRARY_DEFINITIONS): NodeTypeDefinition {
    const libraryDef = NODE_LIBRARY_DEFINITIONS[type]

    // Add type-specific default configurations
    switch (type) {
      case 'trigger':
        return {
          type: 'trigger',
          ...libraryDef,
          defaultConfig: {
            type: 'trigger',
            methods: ['POST']
          }
        }
      case 'condition':
        return {
          type: 'condition',
          ...libraryDef,
          defaultConfig: {
            type: 'condition',
            script: DEFAULT_CONDITION_SCRIPT
          }
        }
      case 'transformer':
        return {
          type: 'transformer',
          ...libraryDef,
          defaultConfig: {
            type: 'transformer',
            script: DEFAULT_TRANSFORMER_SCRIPT
          }
        }
      case 'http-request':
        return {
          type: 'http-request',
          ...libraryDef,
          defaultConfig: {
            type: 'http-request',
            ...DEFAULT_HTTP_CONFIG
          }
        }
      case 'openobserve':
        return {
          type: 'openobserve',
          ...libraryDef,
          defaultConfig: {
            type: 'openobserve',
            ...DEFAULT_OPENOBSERVE_CONFIG,
            // Override with node library specific URL and auth
            url: 'https://api.openobserve.ai/api/default/logs/_json',
            authorization_header: 'Basic <your-token>'
          }
        }
      case 'email':
        return {
          type: 'email',
          ...libraryDef,
          defaultConfig: {
            type: 'email',
            ...DEFAULT_EMAIL_CONFIG,
            // Override the subject to match the existing node library template
            subject: 'Workflow completed',
            // Override the body template to match the existing node library template  
            body_template: '<!DOCTYPE html><html>\n<body>\n\n<h1>Workflow Results</h1>\n<p>Data: {{json event.data}}</p>\n\n</body>\n</html>',
            text_body_template: 'Workflow Results\nData: {{json event.data}}'
          }
        }
      case 'delay':
        return {
          type: 'delay',
          ...libraryDef,
          defaultConfig: {
            type: 'delay',
            ...DEFAULT_DELAY_CONFIG
          }
        }
      case 'anthropic':
        return {
          type: 'anthropic',
          ...libraryDef,
          defaultConfig: {
            type: 'anthropic',
            ...DEFAULT_ANTHROPIC_CONFIG
          }
        }
      case 'human-in-loop':
        return {
          type: 'human-in-loop',
          ...libraryDef,
          defaultConfig: {
            ...DEFAULT_HUMAN_IN_LOOP_CONFIG
          }
        }
      default:
        // This should never happen, but TypeScript requires it
        throw new Error(`Unknown node type: ${type}`)
    }
  }

  // Node type definitions using centralized constants
  const nodeTypes = ref<NodeTypeDefinition[]>([
    createNodeTypeDefinition('trigger'),
    createNodeTypeDefinition('condition'),
    createNodeTypeDefinition('transformer'),
    createNodeTypeDefinition('http-request'),
    createNodeTypeDefinition('openobserve'),
    createNodeTypeDefinition('email'),
    createNodeTypeDefinition('delay'),
    createNodeTypeDefinition('anthropic'),
    createNodeTypeDefinition('human-in-loop')
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
      const updatedNode = { ...nodes.value[index], ...updates }
      
      // Force array reactivity by creating a new array
      const newNodes = [...nodes.value]
      newNodes[index] = updatedNode
      nodes.value = newNodes
      
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