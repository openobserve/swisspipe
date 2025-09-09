import { ref } from 'vue'
import { useNodeStore } from '../stores/nodes'
import { apiClient } from '../services/api'

export function useExecutionTracing() {
  const nodeStore = useNodeStore()
  
  const tracingExecution = ref<any>(null)
  const executionSteps = ref<any[]>([])
  const showNodeInspector = ref(false)
  const inspectedNode = ref<any>(null)

  async function onTraceExecution(executionData: any) {
    console.log('Tracing execution:', executionData)
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
      nodeExecutionMap.set(step.node_name, {
        status: step.status,
        duration: step.duration_ms,
        error: step.error_message,
        input: step.input_data,
        output: step.output_data
      })
    })
    
    nodeStore.nodes.forEach(node => {
      const nodeName = node.type === 'trigger' ? 'Start' : node.data.label
      const executionData = nodeExecutionMap.get(nodeName)
      
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
    const executedNodeNames = new Set(executionSteps.value.map(step => step.node_name))
    
    nodeStore.edges.forEach((edge: any) => {
      const sourceNode = nodeStore.nodes.find(n => n.id === edge.source)
      const targetNode = nodeStore.nodes.find(n => n.id === edge.target)
      
      if (sourceNode && targetNode) {
        const sourceName = sourceNode.type === 'trigger' ? 'Start' : sourceNode.data.label
        const targetName = targetNode.type === 'trigger' ? 'Start' : targetNode.data.label
        
        const isExecutionPath = executedNodeNames.has(sourceName) && executedNodeNames.has(targetName)
        
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
            type: 'arrowclosed',
            color: '#3b82f6',
            width: 20,
            height: 20
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
    console.log('Animating execution path with steps:', executionSteps.value)
  }

  function clearExecutionTracing() {
    tracingExecution.value = null
    executionSteps.value = []
    showNodeInspector.value = false
    inspectedNode.value = null
    
    nodeStore.nodes.forEach((node: any) => {
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