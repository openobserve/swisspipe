import { ref, computed } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useWorkflowStore } from '../stores/workflows'
import { useNodeStore } from '../stores/nodes'
import type { WorkflowNode } from '../types/nodes'
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
  const saving = ref(false)

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
    }
    // Note: Start/trigger nodes are now auto-created by the backend
  }

  async function updateWorkflowName() {
    if (workflowStore.currentWorkflow && workflowName.value !== workflowStore.currentWorkflow.name) {
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
        return {
          from_node_id: edge.source,
          to_node_id: edge.target,
          condition_result: edge.sourceHandle === 'true' ? true : edge.sourceHandle === 'false' ? false : undefined
        }
      })

      const workflowData = {
        name: workflowName.value || workflowStore.currentWorkflow.name,
        description: workflowStore.currentWorkflow.description,
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
    saving,
    navigateBack,
    loadWorkflowData,
    updateWorkflowName,
    saveWorkflow,
    resetWorkflow
  }
}

