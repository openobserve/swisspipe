import { v4 as uuidv4 } from 'uuid'
import type { NodeTypeDefinition, WorkflowNode, WorkflowEdge } from '../types/nodes'
import { useNodeStore } from '../stores/nodes'
import { useToast } from './useToast'

export function useNodeCreation() {
  const nodeStore = useNodeStore()
  const { success, error: errorToast } = useToast()

  /**
   * Check if a position collides with existing nodes
   */
  function hasCollision(x: number, y: number, nodeWidth = 200, nodeHeight = 70): boolean {
    return nodeStore.nodes.some(node => {
      const nodeX = node.position.x
      const nodeY = node.position.y
      const nodeW = 200 // Approximate node width
      const nodeH = 70  // Approximate node height

      // Check for bounding box intersection
      return !(
        x + nodeWidth < nodeX ||
        x > nodeX + nodeW ||
        y + nodeHeight < nodeY ||
        y > nodeY + nodeH
      )
    })
  }

  /**
   * Find a clear position below the source node
   */
  function findClearPosition(sourceNode: WorkflowNode): { x: number; y: number } {
    const sourceX = sourceNode.position.x
    const sourceY = sourceNode.position.y
    const nodeHeight = 70
    const verticalGap = 100

    let targetX = sourceX
    let targetY = sourceY + nodeHeight + verticalGap

    // Try up to 3 times vertically
    for (let attempt = 0; attempt < 3; attempt++) {
      if (!hasCollision(targetX, targetY)) {
        return { x: targetX, y: targetY }
      }
      targetY += 150 // Offset by 150px
    }

    // If all vertical attempts failed, try horizontal offset
    targetX = sourceX + 200 // Offset horizontally
    targetY = sourceY + nodeHeight + verticalGap

    return { x: targetX, y: targetY }
  }

  /**
   * Get the appropriate output handle for a node
   */
  function getOutputHandle(node: WorkflowNode): string | undefined {
    switch (node.type) {
      case 'condition':
        // For condition nodes, default to 'true' handle
        return 'true'
      case 'trigger':
      case 'transformer':
      case 'http-request':
      case 'openobserve':
      case 'email':
      case 'delay':
      case 'anthropic':
      case 'human-in-loop':
        // These nodes have a default bottom handle
        return undefined
      default:
        return undefined
    }
  }

  /**
   * Check if an edge would be valid
   */
  function isValidEdge(sourceId: string, targetId: string, sourceHandle?: string): boolean {
    // Check for self-loops
    if (sourceId === targetId) {
      return false
    }

    // Check if edge already exists
    const edgeExists = nodeStore.edges.some(edge =>
      edge.source === sourceId &&
      edge.target === targetId &&
      edge.sourceHandle === sourceHandle
    )

    if (edgeExists) {
      return false
    }

    // Check for cycles (simplified - just check for direct back-edge)
    const wouldCreateCycle = nodeStore.edges.some(edge =>
      edge.source === targetId && edge.target === sourceId
    )

    if (wouldCreateCycle) {
      return false
    }

    return true
  }

  /**
   * Create a new node below the source node with automatic connection
   */
  function createConnectedNode(
    sourceNodeId: string,
    nodeType: NodeTypeDefinition,
    sourceHandle?: string
  ): { node: WorkflowNode; edge: WorkflowEdge } | null {
    // Find the source node
    const sourceNode = nodeStore.getNodeById(sourceNodeId)
    if (!sourceNode) {
      errorToast('Source node not found')
      return null
    }

    // Find a clear position
    const position = findClearPosition(sourceNode)

    // Create unique ID and label
    const nodeId = uuidv4()
    const randomSuffix = Math.floor(Math.random() * 1000000000000).toString().padStart(12, '0')

    // Create the new node
    const newNode: WorkflowNode = {
      id: nodeId,
      type: nodeType.type,
      position,
      data: {
        label: `${nodeType.label} ${randomSuffix}`,
        description: nodeType.description,
        config: nodeType.defaultConfig,
        status: 'ready' as const
      }
    }

    // Use provided source handle, or determine from node type if not provided
    const effectiveSourceHandle = sourceHandle !== undefined ? sourceHandle : getOutputHandle(sourceNode)

    // Validate edge
    if (!isValidEdge(sourceNodeId, nodeId, effectiveSourceHandle)) {
      errorToast('Cannot create connection: would create invalid workflow')
      return null
    }

    // Create the edge
    const edgeId = uuidv4()
    const newEdge: WorkflowEdge = {
      id: edgeId,
      source: sourceNodeId,
      target: nodeId,
      sourceHandle: effectiveSourceHandle,
      targetHandle: undefined
    }

    return { node: newNode, edge: newEdge }
  }

  /**
   * Add node and edge to the workflow
   */
  function addConnectedNodeToWorkflow(sourceNodeId: string, nodeType: NodeTypeDefinition, sourceHandle?: string): boolean {
    try {
      const result = createConnectedNode(sourceNodeId, nodeType, sourceHandle)
      if (!result) return false

      // Add both node and edge to the store
      nodeStore.addNode(result.node)
      nodeStore.addEdge(result.edge)

      success(`Added ${nodeType.label}`)
      return true
    } catch (error) {
      console.error('Error creating connected node:', error)
      errorToast('Failed to create node')
      return false
    }
  }

  return {
    createConnectedNode,
    addConnectedNodeToWorkflow,
    findClearPosition,
    hasCollision,
    isValidEdge
  }
}
