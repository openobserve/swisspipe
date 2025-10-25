import { ref } from 'vue'
import { useVueFlow } from '@vue-flow/core'
import { useNodeStore } from '../stores/nodes'
import { deepClone } from '../utils/comparison'
import { v4 as uuidv4 } from 'uuid'
import type { Node } from '@vue-flow/core'
import type { WorkflowExecution } from '../types/execution'

export function useVueFlowInteraction() {
  const nodeStore = useNodeStore()
  const { project } = useVueFlow()
  
  const selectedEdgeId = ref<string | null>(null)

  function onNodeClick(event: { node: Node }, tracingExecution: { value: WorkflowExecution | null } | null, onInspectNode: (node: unknown) => void) {
    // If we're in tracing mode, always show the NodeInspector instead of properties panel
    if (tracingExecution?.value) {
      onInspectNode(event.node.data)
      return
    }
    
    // Normal mode - open properties panel
    nodeStore.setSelectedNode(event.node.id)
    selectedEdgeId.value = null
  }

  function onEdgeClick(event: { edge: { id: string } }) {
    selectedEdgeId.value = event.edge.id
    nodeStore.setSelectedNode(null)
  }

  function onPaneClick() {
    nodeStore.setSelectedNode(null)
    selectedEdgeId.value = null
  }

  function onConnect(params: { source: string; target: string; sourceHandle?: string | null; targetHandle?: string | null }) {
    const edge = {
      id: `edge-${params.source}-${params.target}`,
      source: params.source,
      target: params.target,
      sourceHandle: params.sourceHandle || undefined,
      targetHandle: params.targetHandle || undefined,
      data: {}
    }
    nodeStore.addEdge(edge)
  }

  function onNodesDelete(event: { nodes: Node[] }) {
    const triggerNodes = event.nodes.filter((node) => node.type === 'trigger')

    if (triggerNodes.length > 0) {
      console.warn('Cannot delete trigger nodes')
      // Prevent deletion by not calling the actual delete
      // The VueFlow library will handle this based on the event
      return
    }
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
      
      const nodeId = uuidv4()
      // Generate 12-digit random number for unique naming
      const randomSuffix = Math.floor(Math.random() * 1000000000000).toString().padStart(12, '0')
      
      const newNode = {
        id: nodeId,
        type: nodeTypeData.type,
        position,
        data: {
          label: `${nodeTypeData.label} ${randomSuffix}`,
          description: nodeTypeData.description,
          config: deepClone(nodeTypeData.defaultConfig),
          status: 'ready' as const
        }
      }
      
      nodeStore.addNode(newNode)
    } catch (error) {
      console.error('Error parsing dropped node data:', error)
    }
  }

  function handleKeyDown(event: KeyboardEvent) {
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

    if (event.key === 'Delete' || event.key === 'Backspace') {
      event.preventDefault()
      
      if (selectedEdgeId.value) {
        nodeStore.deleteEdge(selectedEdgeId.value)
        selectedEdgeId.value = null
        return
      }
      
      if (nodeStore.selectedNode) {
        const selectedNodeData = nodeStore.getNodeById(nodeStore.selectedNode)
        if (selectedNodeData) {
          if (selectedNodeData.type === 'trigger') {
            console.warn('Cannot delete trigger nodes')
            return false
          } else {
            nodeStore.deleteNode(selectedNodeData.id)
          }
        }
      }
    }
  }

  return {
    selectedEdgeId,
    onNodeClick,
    onEdgeClick,
    onPaneClick,
    onConnect,
    onNodesDelete,
    onDrop,
    handleKeyDown
  }
}