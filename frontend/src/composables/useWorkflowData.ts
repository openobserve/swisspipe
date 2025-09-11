import { ref, computed } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useWorkflowStore } from '../stores/workflows'
import { useNodeStore } from '../stores/nodes'
import { DEFAULT_CONDITION_SCRIPT, DEFAULT_TRANSFORMER_SCRIPT } from '../constants/defaults'
import type { NodeConfig, WorkflowNode } from '../types/nodes'
import { debugLog } from '../utils/debug'

export function useWorkflowData() {
  const router = useRouter()
  const route = useRoute()
  const workflowStore = useWorkflowStore()
  const nodeStore = useNodeStore()

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
        // Prefer node IDs over names for edge connections
        let sourceNode, targetNode
        
        if (edge.from_node_id && edge.to_node_id) {
          // Use node IDs if available
          sourceNode = workflow.nodes.find(n => n.id === edge.from_node_id)
          targetNode = workflow.nodes.find(n => n.id === edge.to_node_id)
        } else {
          // Fall back to names for backward compatibility
          sourceNode = workflow.nodes.find(n => n.name === edge.from_node_name)
          targetNode = workflow.nodes.find(n => n.name === edge.to_node_name)
        }
        
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
            hasFrom: !!(node.data.config as any)?.from,
            hasTo: !!(node.data.config as any)?.to,
            toCount: (node.data.config as any)?.to?.length || 0
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
          from_node_id: edge.source,
          to_node_id: edge.target,
          condition_result: edge.sourceHandle === 'true' ? true : edge.sourceHandle === 'false' ? false : undefined
        }
      })

      const startNodeName = 'Start'
      const startNodeId = nodeStore.nodes.find(n => n.type === 'trigger')?.id

      const workflowData = {
        name: workflowName.value || workflowStore.currentWorkflow.name,
        description: workflowStore.currentWorkflow.description,
        start_node_name: startNodeName,
        start_node_id: startNodeId,
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

function convertApiNodeTypeToVueFlowType(nodeType: any): 'trigger' | 'condition' | 'transformer' | 'http-request' | 'openobserve' | 'email' | 'delay' {
  if (nodeType.Trigger) return 'trigger'
  if (nodeType.Condition) return 'condition'
  if (nodeType.Transformer) return 'transformer'
  if (nodeType.HttpRequest) return 'http-request'
  if (nodeType.OpenObserve) return 'openobserve'
  if (nodeType.Email) return 'email'
  if (nodeType.Delay) return 'delay'
  // Legacy support for old App nodes
  if (nodeType.App) {
    if (typeof nodeType.App.app_type === 'object' && nodeType.App.app_type.OpenObserve) {
      return 'openobserve'
    }
    return 'http-request'
  }
  return 'http-request'
}

function getNodeDescription(nodeType: any): string {
  if (nodeType.Trigger) return 'HTTP endpoint trigger'
  if (nodeType.Condition) return 'Conditional logic node'
  if (nodeType.Transformer) return 'Data transformation node'
  if (nodeType.HttpRequest) return 'HTTP request'
  if (nodeType.OpenObserve) return 'OpenObserve log analytics'
  if (nodeType.Email) return 'Email notification node'
  if (nodeType.Delay) return 'Workflow execution delay'
  // Legacy support
  if (nodeType.App) return 'External application node'
  return 'Unknown node type'
}

function convertApiNodeConfigToVueFlowConfig(nodeType: any): NodeConfig {
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
  if (nodeType.HttpRequest) {
    return {
      type: 'http-request' as const,
      url: nodeType.HttpRequest.url || 'https://httpbin.org/post',
      method: nodeType.HttpRequest.method || 'POST',
      timeout_seconds: nodeType.HttpRequest.timeout_seconds || 30,
      failure_action: nodeType.HttpRequest.failure_action || 'Stop',
      headers: nodeType.HttpRequest.headers || {},
      retry_config: nodeType.HttpRequest.retry_config || {
        max_attempts: 3,
        initial_delay_ms: 100,
        max_delay_ms: 5000,
        backoff_multiplier: 2.0
      }
    }
  }
  if (nodeType.OpenObserve) {
    return {
      type: 'openobserve' as const,
      url: nodeType.OpenObserve.url || '',
      authorization_header: nodeType.OpenObserve.authorization_header || '',
      timeout_seconds: nodeType.OpenObserve.timeout_seconds || 30,
      failure_action: nodeType.OpenObserve.failure_action || 'Stop',
      retry_config: nodeType.OpenObserve.retry_config || {
        max_attempts: 3,
        initial_delay_ms: 100,
        max_delay_ms: 5000,
        backoff_multiplier: 2.0
      }
    }
  }
  // Legacy support for old App nodes
  if (nodeType.App) {
    const config = {
      type: 'app' as const,
      app_type: nodeType.App.app_type || 'HttpRequest',
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
      type: 'email' as const,
      smtp_config: emailConfig.smtp_config || 'default',
      from: emailConfig.from || {
        email: 'noreply@company.com',
        name: 'SwissPipe Workflow'
      },
      to: emailConfig.to || [{
        email: '{{ event.data.user_email }}',
        name: '{{ event.data.user_name }}'
      }],
      cc: emailConfig.cc || [],
      bcc: emailConfig.bcc || [],
      subject: emailConfig.subject || 'Workflow {{ event.name }} completed',
      template_type: emailConfig.template_type || 'html',
      body_template: emailConfig.body_template || '<!DOCTYPE html><html><body><h1>Workflow Results</h1><p>Status: {{ event.status }}</p><p>Data: {{ event.data  }}</p></body></html>',
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
  if (nodeType.Delay) {
    return {
      type: 'delay' as const,
      duration: nodeType.Delay.duration || 5,
      unit: nodeType.Delay.unit || 'Seconds'
    }
  }
  return {
    type: 'trigger' as const,
    methods: ['POST']
  } as NodeConfig
}

function convertNodeToApiType(node: { type: string; data: { config: any } }) {
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
    case 'http-request':
      const httpRequestConfig = node.data.config as any
      return {
        HttpRequest: {
          url: httpRequestConfig.url || 'https://httpbin.org/post',
          method: httpRequestConfig.method || 'POST',
          timeout_seconds: httpRequestConfig.timeout_seconds || 30,
          failure_action: httpRequestConfig.failure_action || 'Stop',
          headers: httpRequestConfig.headers || {},
          retry_config: httpRequestConfig.retry_config || {
            max_attempts: 3,
            initial_delay_ms: 100,
            max_delay_ms: 5000,
            backoff_multiplier: 2.0
          }
        }
      }
    case 'openobserve':
      const openobserveConfig = node.data.config as any
      return {
        OpenObserve: {
          url: openobserveConfig.url || '',
          authorization_header: openobserveConfig.authorization_header || '',
          timeout_seconds: openobserveConfig.timeout_seconds || 30,
          failure_action: openobserveConfig.failure_action || 'Stop',
          retry_config: openobserveConfig.retry_config || {
            max_attempts: 3,
            initial_delay_ms: 100,
            max_delay_ms: 5000,
            backoff_multiplier: 2.0
          }
        }
      }
    case 'app':
      // Legacy support for old App nodes
      const appConfig = node.data.config as any
      let app_type = appConfig.app_type || 'HttpRequest'
      
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
      const emailConfig = node.data.config as any
      debugLog.transform('email-config-to-api', {
        nodeId: (node as any).id || 'unknown',
        hasFrom: !!emailConfig.from,
        hasTo: !!emailConfig.to,
        toCount: emailConfig.to?.length || 0,
        hasCC: !!emailConfig.cc,
        ccCount: emailConfig.cc?.length || 0,
        hasBCC: !!emailConfig.bcc,
        bccCount: emailConfig.bcc?.length || 0
      })
      
      const result = {
        Email: {
          config: {
            smtp_config: emailConfig.smtp_config || 'default',
            from: emailConfig.from || {
              email: 'noreply@company.com',
              name: 'SwissPipe Workflow'
            },
            to: emailConfig.to || [{
              email: '{{ event.data.user_email }}',
              name: '{{ event.data.user_name }}'
            }],
            cc: emailConfig.cc || [],
            bcc: emailConfig.bcc || [],
            subject: emailConfig.subject || 'Workflow {{ event.name }} completed',
            template_type: emailConfig.template_type || 'html',
            body_template: emailConfig.body_template || '<!DOCTYPE html><html><body><h1>Workflow Results</h1><p>Status: {{ event.status }}</p><p>Data: {{ event.data  }}</p></body></html>',
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
      
      debugLog.transform('email-api-result', {
        hasResult: !!result,
        hasEmailConfig: !!result.Email?.config,
        finalToCount: result.Email?.config?.to?.length || 0,
        finalFromExists: !!result.Email?.config?.from
      })
      
      return result
    case 'delay':
      const delayConfig = node.data.config as any
      return {
        Delay: {
          duration: delayConfig.duration || 5,
          unit: delayConfig.unit || 'Seconds'
        }
      }
    default:
      throw new Error(`Unknown node type: ${node.type}`)
  }
}