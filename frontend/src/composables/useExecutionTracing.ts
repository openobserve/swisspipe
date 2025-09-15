import { ref } from 'vue'
import { useNodeStore } from '../stores/nodes'
import { apiClient } from '../services/api'
import type { WorkflowExecution, ExecutionStep } from '../types/execution'
import type { Node, Edge } from '@vue-flow/core'
import type { WorkflowNodeData } from '../types/nodes'
import { MarkerType } from '@vue-flow/core'

export function useExecutionTracing() {
  const nodeStore = useNodeStore()
  
  const tracingExecution = ref<WorkflowExecution | null>(null)
  const executionSteps = ref<ExecutionStep[]>([])
  const showNodeInspector = ref(false)
  const inspectedNode = ref<WorkflowNodeData | null>(null)

  async function onTraceExecution(executionData: WorkflowExecution) {
    tracingExecution.value = executionData
    
    try {
      const data = await apiClient.getExecutionSteps(executionData.id)
      executionSteps.value = data.steps || []
      
      updateNodeExecutionStates()
      animateExecutionPath()
    } catch (error) {
      console.error('Failed to fetch execution steps:', error)
    }
  }

  function updateNodeExecutionStates() {
    const nodeExecutionMap = new Map()
    
    executionSteps.value.forEach(step => {
      // Calculate duration from started_at and completed_at if available
      // Backend timestamps are in microseconds, convert to milliseconds
      let duration = undefined
      if (step.started_at && step.completed_at) {
        duration = Math.round((step.completed_at - step.started_at) / 1000) // Convert microseconds to milliseconds
      }

      // Use node_id instead of node_name for mapping
      nodeExecutionMap.set(step.node_id, {
        status: step.status,
        duration: duration,
        error: step.error_message,
        input: step.input_data,
        output: step.output_data
      })
    })
    
    nodeStore.nodes.forEach(node => {
      // Match by node ID instead of node name/label
      const executionData = nodeExecutionMap.get(node.id)
      
      if (executionData) {
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
        node.data = {
          ...node.data,
          executionStatus: undefined,
          executionDuration: undefined,
          executionError: undefined,
          executionInput: null,
          executionOutput: null,
          isTracing: true
        }
      }
    })
    
    updateEdgeExecutionStyles()
  }

  function updateEdgeExecutionStyles() {
    const executedNodeIds = new Set(executionSteps.value.map(step => step.node_id))
    
    nodeStore.edges.forEach((edge: Edge) => {
      const sourceNode = nodeStore.nodes.find(n => n.id === edge.source)
      const targetNode = nodeStore.nodes.find(n => n.id === edge.target)
      
      if (sourceNode && targetNode) {
        // Use node IDs instead of node names/labels
        const isExecutionPath = executedNodeIds.has(sourceNode.id) && executedNodeIds.has(targetNode.id)
        
        if (isExecutionPath) {
          edge.style = {
            ...edge.style,
            strokeWidth: 4,
            stroke: '#3b82f6',
            strokeDasharray: '0',
            opacity: 1,
            transition: 'all 0.3s ease'
          }
          edge.markerEnd = {
            type: MarkerType.ArrowClosed,
            color: '#3b82f6',
            width: 10,
            height: 10
          }
        } else {
          edge.style = {
            ...edge.style,
            strokeWidth: 1,
            stroke: '#6b7280',
            opacity: 0.3,
            transition: 'all 0.3s ease'
          }
          edge.markerEnd = {
            type: MarkerType.ArrowClosed,
            color: '#6b7280',
            width: 15,
            height: 15
          }
        }
      }
    })
  }

  function animateExecutionPath() {
    console.log('Animating execution path with steps:', executionSteps.value)
  }

  function clearExecutionTracing() {
    tracingExecution.value = null
    executionSteps.value = []
    showNodeInspector.value = false
    inspectedNode.value = null
    
    nodeStore.nodes.forEach((node: Node) => {
      node.data = {
        ...node.data,
        executionStatus: undefined,
        executionDuration: undefined,
        executionError: undefined,
        executionInput: null,
        executionOutput: null,
        isTracing: false
      }
    })
    
    nodeStore.edges.forEach((edge: Edge) => {
      edge.style = {
        strokeWidth: 2,
        stroke: '#6b7280',
        opacity: 1,
        transition: 'all 0.3s ease'
      }
      edge.markerEnd = {
        type: MarkerType.ArrowClosed,
        color: '#6b7280',
        width: 15,
        height: 15
      }
    })
  }

  return {
    tracingExecution,
    executionSteps,
    showNodeInspector,
    inspectedNode,
    onTraceExecution,
    clearExecutionTracing
  }
}