import { ref, computed } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useWorkflowStore } from '../stores/workflows'
import { useNodeStore } from '../stores/nodes'
import type { WorkflowNode } from '../types/nodes'
import type { Workflow } from '../types/workflow'
import { debugLog } from '../utils/debug'
import { useToast } from './useToast'
import { 
  convertApiNodeTypeToVueFlowType,
  getNodeDescription,
  convertApiNodeConfigToVueFlowConfig,
  convertNodeToApiType
} from '../utils/nodeConverters'

export function useWorkflowData() {
  const router = useRouter()
  const route = useRoute()
  const workflowStore = useWorkflowStore()
  const nodeStore = useNodeStore()
  const toast = useToast()

  const workflowId = computed(() => route.params.id as string)
  const workflowName = ref('')
  const workflowDescription = ref('')
  const saving = ref(false)

  const normalizeName = (name: string) => name.trim()
  const normalizeDescriptionInput = (description: string) => description.trim()

  const normalizeDescriptionForRequest = (description: string) => {
    const trimmed = normalizeDescriptionInput(description)
    return trimmed === '' ? undefined : trimmed
  }

  const applyWorkflowMetadataUpdate = (update: { name?: string; description?: string | undefined; forceDescriptionUpdate?: boolean }) => {
    const currentWorkflow = workflowStore.currentWorkflow
    if (!currentWorkflow) {
      return
    }

    const shouldUpdateName = update.name !== undefined
    const shouldUpdateDescription = update.forceDescriptionUpdate || update.description !== undefined

    const updatedWorkflow: Workflow = {
      ...currentWorkflow,
      ...(shouldUpdateName ? { name: update.name as string } : {}),
      ...(shouldUpdateDescription ? { description: update.description } : {})
    }

    workflowStore.setCurrentWorkflow(updatedWorkflow)

    const index = workflowStore.workflows.findIndex(w => w.id === updatedWorkflow.id)
    if (index !== -1) {
      workflowStore.workflows.splice(index, 1, {
        ...workflowStore.workflows[index],
        ...(shouldUpdateName ? { name: updatedWorkflow.name } : {}),
        ...(shouldUpdateDescription ? { description: updatedWorkflow.description } : {})
      })
    }
  }

  function navigateBack() {
    router.push('/workflows')
  }

  function loadWorkflowData() {
    const workflow = workflowStore.currentWorkflow
    
    if (workflow && workflow.nodes && workflow.edges) {
      // Convert API nodes to VueFlow format
      workflow.nodes.forEach((node, index) => {
        const nodeType = convertApiNodeTypeToVueFlowType(node.node_type)
        const vueFlowNode: WorkflowNode = {
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
        // Use node IDs for edge connections
        const sourceNode = workflow.nodes.find(n => n.id === edge.from_node_id)
        const targetNode = workflow.nodes.find(n => n.id === edge.to_node_id)

        if (sourceNode && targetNode) {
          // Map source_handle_id from API to VueFlow sourceHandle
          let sourceHandle = undefined
          if (edge.source_handle_id) {
            sourceHandle = edge.source_handle_id
          } else if (edge.condition_result === true) {
            sourceHandle = 'true'
          } else if (edge.condition_result === false) {
            sourceHandle = 'false'
          }

          const vueFlowEdge = {
            id: edge.id,
            source: sourceNode.id,
            target: targetNode.id,
            sourceHandle,
            targetHandle: undefined,
            data: {}
          }
          nodeStore.addEdge(vueFlowEdge)
        }
      })
    }
    // Note: Start/trigger nodes are now auto-created by the backend
  }

  async function updateWorkflowName() {
    const currentWorkflow = workflowStore.currentWorkflow
    if (!currentWorkflow) {
      return
    }

    const trimmedName = normalizeName(workflowName.value)
    if (!trimmedName) {
      workflowName.value = currentWorkflow.name
      return
    }

    if (trimmedName !== workflowName.value) {
      workflowName.value = trimmedName
    }

    if (trimmedName !== currentWorkflow.name) {
      applyWorkflowMetadataUpdate({ name: trimmedName })
    }
  }

  async function updateWorkflowDescription() {
    const currentWorkflow = workflowStore.currentWorkflow
    if (!currentWorkflow) {
      return
    }

    const trimmedDescription = normalizeDescriptionInput(workflowDescription.value)

    if (trimmedDescription !== workflowDescription.value) {
      workflowDescription.value = trimmedDescription
    }

    const normalizedForStore = trimmedDescription === '' ? undefined : trimmedDescription
    const currentDescription = currentWorkflow.description

    if (normalizedForStore !== currentDescription) {
      applyWorkflowMetadataUpdate({ description: normalizedForStore, forceDescriptionUpdate: true })
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
        debugLog.transform('node-to-api', {
          nodeId: node.id,
          nodeType: node.type,
          hasData: !!node.data,
          hasConfig: !!node.data.config
        })
        
        const apiNode = {
          id: node.id,
          name: node.type === 'trigger' ? 'Start' : (node.data.label || node.id),
          node_type: convertNodeToApiType(node),
          position_x: node.position.x,
          position_y: node.position.y
        }
        
        // Debug logging for email nodes specifically
        if (node.type === 'email') {
          debugLog.component('useWorkflowData', 'email-node-conversion', {
            nodeId: node.id,
            hasConfig: !!node.data.config,
            hasFrom: !!(node.data.config as unknown as Record<string, unknown>)?.from,
            hasTo: !!(node.data.config as unknown as Record<string, unknown>)?.to,
            toCount: Array.isArray((node.data.config as unknown as Record<string, unknown>)?.to) ? ((node.data.config as unknown as Record<string, unknown>).to as unknown[]).length : 0
          })
        }
        
        return apiNode
      })

      // Convert Vue Flow edges to API format
      const apiEdges = nodeStore.edges.map(edge => {
        // Map VueFlow sourceHandle to API source_handle_id
        const result: {
          from_node_id: string
          to_node_id: string
          condition_result?: boolean
          source_handle_id?: string
        } = {
          from_node_id: edge.source,
          to_node_id: edge.target
        }

        if (edge.sourceHandle) {
          // Handle HIL node handles (approved, denied, notification)
          if (['approved', 'denied', 'notification'].includes(edge.sourceHandle)) {
            result.source_handle_id = edge.sourceHandle
          }
          // Handle condition node handles (true, false)
          else if (edge.sourceHandle === 'true') {
            result.condition_result = true
          } else if (edge.sourceHandle === 'false') {
            result.condition_result = false
          }
          // Handle other specific handles
          else {
            result.source_handle_id = edge.sourceHandle
          }
        } else {
          // Check if source is HIL node and infer missing handle
          const sourceNode = nodeStore.nodes.find(n => n.id === edge.source)
          if (sourceNode && sourceNode.type === 'human-in-loop') {
            // Check which HIL handles are already used by other edges
            const otherHilEdges = nodeStore.edges.filter(e =>
              e.source === edge.source &&
              e.id !== edge.id &&
              e.sourceHandle
            )
            const usedHandles = otherHilEdges.map(e => e.sourceHandle)

            // Infer the missing handle (should be approved if not used)
            if (!usedHandles.includes('approved')) {
              result.source_handle_id = 'approved'
              console.log('Inferred missing approved handle for HIL edge:', edge.id)
            } else if (!usedHandles.includes('denied')) {
              result.source_handle_id = 'denied'
              console.log('Inferred missing denied handle for HIL edge:', edge.id)
            } else if (!usedHandles.includes('notification')) {
              result.source_handle_id = 'notification'
              console.log('Inferred missing notification handle for HIL edge:', edge.id)
            }
          }
        }

        return result
      })

      const trimmedWorkflowName = normalizeName(workflowName.value)
      const workflowData = {
        name: trimmedWorkflowName || workflowStore.currentWorkflow.name,
        description: normalizeDescriptionForRequest(workflowDescription.value),
        nodes: apiNodes.filter(node => {
          // Filter out trigger nodes since they are auto-created by the backend
          const nodeInStore = nodeStore.nodes.find(n => n.id === node.id)
          return nodeInStore?.type !== 'trigger'
        }),
        edges: apiEdges
      }

      await workflowStore.updateWorkflow(workflowStore.currentWorkflow.id, workflowData as unknown as Parameters<typeof workflowStore.updateWorkflow>[1])
      toast.success('Workflow Saved', 'Your workflow has been saved successfully.')
    } catch (error) {
      console.error('Failed to save workflow:', error)
      toast.error('Save Failed', error instanceof Error ? error.message : 'Failed to save workflow. Please try again.')
    } finally {
      saving.value = false
    }
  }

  function resetWorkflow() {
    nodeStore.clearWorkflow()
    loadWorkflowData()
  }

  return {
    workflowId,
    workflowName,
    workflowDescription,
    saving,
    navigateBack,
    loadWorkflowData,
    updateWorkflowName,
    updateWorkflowDescription,
    saveWorkflow,
    resetWorkflow
  }
}
