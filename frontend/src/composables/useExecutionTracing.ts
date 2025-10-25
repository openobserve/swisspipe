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
          isTracing: true,
          tracingExecutionId: tracingExecution.value?.id
        }
      } else {
        node.data = {
          ...node.data,
          executionStatus: undefined,
          executionDuration: undefined,
          executionError: undefined,
          executionInput: null,
          executionOutput: null,
          isTracing: true,
          tracingExecutionId: tracingExecution.value?.id
        }
      }
    })
    
    updateEdgeExecutionStyles()
  }

  function updateEdgeExecutionStyles() {
    const executedNodeIds = new Set(executionSteps.value.map(step => step.node_id))

    // Build a map of condition results by inferring from execution path
    const conditionResults = new Map<string, boolean>()

    nodeStore.nodes.forEach(node => {
      if (node.type === 'condition' && executedNodeIds.has(node.id)) {
        // Find edges from this condition node
        const trueEdges = nodeStore.edges.filter(e =>
          e.source === node.id && e.data?.condition_result === true
        )
        const falseEdges = nodeStore.edges.filter(e =>
          e.source === node.id && e.data?.condition_result === false
        )

        // Check which path was taken based on execution order
        // Get the execution step for this condition node to find when it executed
        const conditionStep = executionSteps.value.find(s => s.node_id === node.id)
        if (!conditionStep) {
          return
        }

        // Find which immediate successor executed first after this condition
        type TargetInfo = {nodeId: string, time: number}
        let firstTrueTarget: TargetInfo | null = null
        let firstFalseTarget: TargetInfo | null = null

        trueEdges.forEach(e => {
          const targetStep = executionSteps.value.find(s => s.node_id === e.target)
          if (targetStep && targetStep.created_at > conditionStep.created_at) {
            if (!firstTrueTarget || targetStep.created_at < firstTrueTarget.time) {
              firstTrueTarget = { nodeId: e.target, time: targetStep.created_at }
            }
          }
        })

        falseEdges.forEach(e => {
          const targetStep = executionSteps.value.find(s => s.node_id === e.target)
          if (targetStep && targetStep.created_at > conditionStep.created_at) {
            if (!firstFalseTarget || targetStep.created_at < firstFalseTarget.time) {
              firstFalseTarget = { nodeId: e.target, time: targetStep.created_at }
            }
          }
        })

        // Determine which path was taken: the one that executed first after this condition
        const hasTrue = firstTrueTarget !== null
        const hasFalse = firstFalseTarget !== null

        if (hasTrue && hasFalse) {
          // Both paths have successors - use the one that executed first
          const trueTime = firstTrueTarget!.time
          const falseTime = firstFalseTarget!.time
          const trueFirst = trueTime < falseTime
          conditionResults.set(node.id, trueFirst)
        } else if (hasTrue) {
          conditionResults.set(node.id, true)
        } else if (hasFalse) {
          conditionResults.set(node.id, false)
        }
      }
    })

    nodeStore.edges.forEach((edge: Edge) => {
      const sourceNode = nodeStore.nodes.find(n => n.id === edge.source)
      const targetNode = nodeStore.nodes.find(n => n.id === edge.target)

      if (sourceNode && targetNode) {
        let isExecutionPath = false

        // Check if both nodes executed
        const bothExecuted = executedNodeIds.has(sourceNode.id) && executedNodeIds.has(targetNode.id)

        if (bothExecuted) {
          // For condition nodes, check if the edge matches the condition result
          if (sourceNode.type === 'condition') {
            const conditionResult = conditionResults.get(sourceNode.id)
            const edgeConditionResult = edge.data?.condition_result

            // Only highlight if condition result matches edge type
            if (conditionResult !== undefined && edgeConditionResult !== undefined) {
              isExecutionPath = conditionResult === edgeConditionResult
            } else if (edgeConditionResult === undefined) {
              // Unconditional edge from condition node (shouldn't normally happen)
              isExecutionPath = true
            }
          } else {
            // Non-condition nodes: highlight if both executed
            isExecutionPath = true
          }
        }

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
    // Future: Add animation effects for execution path
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
        isTracing: false,
        tracingExecutionId: undefined
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