import { DEFAULT_CONDITION_SCRIPT, DEFAULT_TRANSFORMER_SCRIPT } from '../constants/defaults'
import {
  DEFAULT_RETRY_CONFIG,
  DEFAULT_EMAIL_CONFIG,
  DEFAULT_HTTP_CONFIG,
  DEFAULT_OPENOBSERVE_CONFIG,
  DEFAULT_DELAY_CONFIG,
  DEFAULT_ANTHROPIC_CONFIG,
  DEFAULT_HUMAN_IN_LOOP_CONFIG,
  NODE_TYPE_DESCRIPTIONS
} from '../constants/nodeDefaults'
import type { NodeConfig, TriggerConfig, ConditionConfig, TransformerConfig } from '../types/nodes'
import type { NodeType } from '../types/workflow'
import { debugLog } from './debug'

export type ApiNodeType = 'trigger' | 'condition' | 'transformer' | 'http-request' | 'openobserve' | 'email' | 'delay' | 'anthropic' | 'human-in-loop'

export function convertApiNodeTypeToVueFlowType(nodeType: NodeType): ApiNodeType {
  if ('Trigger' in nodeType) return 'trigger'
  if ('Condition' in nodeType) return 'condition'
  if ('Transformer' in nodeType) return 'transformer'
  if ('HttpRequest' in nodeType) return 'http-request'
  if ('OpenObserve' in nodeType) return 'openobserve'
  if ('Email' in nodeType) return 'email'
  if ('Delay' in nodeType) return 'delay'
  if ('Anthropic' in nodeType) return 'anthropic'
  if ('HumanInLoop' in nodeType) return 'human-in-loop'

  // Legacy support for old App nodes
  if ('App' in nodeType) {
    if (typeof nodeType.App.app_type === 'object' && nodeType.App.app_type.OpenObserve) {
      return 'openobserve'
    }
    return 'http-request'
  }

  return 'http-request'
}

export function getNodeDescription(nodeType: NodeType): string {
  if ('Trigger' in nodeType) return NODE_TYPE_DESCRIPTIONS.Trigger
  if ('Condition' in nodeType) return NODE_TYPE_DESCRIPTIONS.Condition
  if ('Transformer' in nodeType) return NODE_TYPE_DESCRIPTIONS.Transformer
  if ('HttpRequest' in nodeType) return NODE_TYPE_DESCRIPTIONS.HttpRequest
  if ('OpenObserve' in nodeType) return NODE_TYPE_DESCRIPTIONS.OpenObserve
  if ('Email' in nodeType) return NODE_TYPE_DESCRIPTIONS.Email
  if ('Delay' in nodeType) return NODE_TYPE_DESCRIPTIONS.Delay
  if ('Anthropic' in nodeType) return NODE_TYPE_DESCRIPTIONS.Anthropic
  if ('HumanInLoop' in nodeType) return NODE_TYPE_DESCRIPTIONS.HumanInLoop
  if ('App' in nodeType) return NODE_TYPE_DESCRIPTIONS.App
  return 'Unknown node type'
}

export function convertApiNodeConfigToVueFlowConfig(nodeType: NodeType): NodeConfig {
  if ('Trigger' in nodeType) {
    return {
      type: 'trigger',
      methods: nodeType.Trigger.methods || ['POST']
    }
  }

  if ('Condition' in nodeType) {
    return {
      type: 'condition',
      script: nodeType.Condition.script || DEFAULT_CONDITION_SCRIPT
    }
  }

  if ('Transformer' in nodeType) {
    return {
      type: 'transformer',
      script: nodeType.Transformer.script || DEFAULT_TRANSFORMER_SCRIPT
    }
  }

  if ('HttpRequest' in nodeType) {
    return {
      type: 'http-request' as const,
      ...DEFAULT_HTTP_CONFIG,
      url: nodeType.HttpRequest.url || DEFAULT_HTTP_CONFIG.url,
      method: nodeType.HttpRequest.method || DEFAULT_HTTP_CONFIG.method,
      timeout_seconds: nodeType.HttpRequest.timeout_seconds || DEFAULT_HTTP_CONFIG.timeout_seconds,
      failure_action: nodeType.HttpRequest.failure_action || DEFAULT_HTTP_CONFIG.failure_action,
      headers: nodeType.HttpRequest.headers || DEFAULT_HTTP_CONFIG.headers,
      retry_config: nodeType.HttpRequest.retry_config || DEFAULT_HTTP_CONFIG.retry_config,
      loop_config: nodeType.HttpRequest.loop_config || undefined
    }
  }

  if ('OpenObserve' in nodeType) {
    return {
      type: 'openobserve' as const,
      ...DEFAULT_OPENOBSERVE_CONFIG,
      url: nodeType.OpenObserve.url || DEFAULT_OPENOBSERVE_CONFIG.url,
      authorization_header: nodeType.OpenObserve.authorization_header || DEFAULT_OPENOBSERVE_CONFIG.authorization_header,
      timeout_seconds: nodeType.OpenObserve.timeout_seconds || DEFAULT_OPENOBSERVE_CONFIG.timeout_seconds,
      failure_action: nodeType.OpenObserve.failure_action || DEFAULT_OPENOBSERVE_CONFIG.failure_action,
      retry_config: nodeType.OpenObserve.retry_config || DEFAULT_OPENOBSERVE_CONFIG.retry_config
    }
  }

  if ('Email' in nodeType) {
    const emailConfig = nodeType.Email.config
    return {
      type: 'email' as const,
      ...DEFAULT_EMAIL_CONFIG,
      to: emailConfig.to || DEFAULT_EMAIL_CONFIG.to,
      cc: emailConfig.cc || DEFAULT_EMAIL_CONFIG.cc,
      bcc: emailConfig.bcc || DEFAULT_EMAIL_CONFIG.bcc,
      subject: emailConfig.subject || DEFAULT_EMAIL_CONFIG.subject,
      template_type: emailConfig.template_type || DEFAULT_EMAIL_CONFIG.template_type,
      body_template: emailConfig.body_template || DEFAULT_EMAIL_CONFIG.body_template,
      text_body_template: emailConfig.text_body_template,
      attachments: emailConfig.attachments || DEFAULT_EMAIL_CONFIG.attachments
    }
  }

  if ('Delay' in nodeType) {
    return {
      type: 'delay' as const,
      duration: nodeType.Delay.duration || DEFAULT_DELAY_CONFIG.duration,
      unit: nodeType.Delay.unit || DEFAULT_DELAY_CONFIG.unit
    }
  }

  if ('Anthropic' in nodeType) {
    return {
      type: 'anthropic' as const,
      model: nodeType.Anthropic.model || DEFAULT_ANTHROPIC_CONFIG.model,
      max_tokens: nodeType.Anthropic.max_tokens || DEFAULT_ANTHROPIC_CONFIG.max_tokens,
      temperature: nodeType.Anthropic.temperature || DEFAULT_ANTHROPIC_CONFIG.temperature,
      system_prompt: nodeType.Anthropic.system_prompt || DEFAULT_ANTHROPIC_CONFIG.system_prompt,
      user_prompt: nodeType.Anthropic.user_prompt || DEFAULT_ANTHROPIC_CONFIG.user_prompt,
      timeout_seconds: nodeType.Anthropic.timeout_seconds || DEFAULT_ANTHROPIC_CONFIG.timeout_seconds,
      failure_action: nodeType.Anthropic.failure_action || DEFAULT_ANTHROPIC_CONFIG.failure_action,
      retry_config: nodeType.Anthropic.retry_config || DEFAULT_ANTHROPIC_CONFIG.retry_config
    }
  }

  if ('HumanInLoop' in nodeType) {
    return {
      type: 'human-in-loop' as const,
      title: nodeType.HumanInLoop.title || DEFAULT_HUMAN_IN_LOOP_CONFIG.title,
      description: nodeType.HumanInLoop.description || DEFAULT_HUMAN_IN_LOOP_CONFIG.description,
      timeout_seconds: nodeType.HumanInLoop.timeout_seconds || DEFAULT_HUMAN_IN_LOOP_CONFIG.timeout_seconds,
      timeout_action: nodeType.HumanInLoop.timeout_action || DEFAULT_HUMAN_IN_LOOP_CONFIG.timeout_action,
      required_fields: nodeType.HumanInLoop.required_fields || DEFAULT_HUMAN_IN_LOOP_CONFIG.required_fields,
      metadata: nodeType.HumanInLoop.metadata || DEFAULT_HUMAN_IN_LOOP_CONFIG.metadata
    }
  }

  // Legacy support for old App nodes
  if ('App' in nodeType) {
    const appType = typeof nodeType.App.app_type === 'string'
      ? nodeType.App.app_type
      : (typeof nodeType.App.app_type === 'object' && nodeType.App.app_type.OpenObserve)
        ? 'OpenObserve'
        : 'HttpRequest'

    const config = {
      type: 'app' as const,
      app_type: appType,
      ...DEFAULT_HTTP_CONFIG,
      url: nodeType.App.url || DEFAULT_HTTP_CONFIG.url,
      method: nodeType.App.method || DEFAULT_HTTP_CONFIG.method,
      timeout_seconds: nodeType.App.timeout_seconds || DEFAULT_HTTP_CONFIG.timeout_seconds,
      failure_action: nodeType.App.failure_action || DEFAULT_HTTP_CONFIG.failure_action,
      headers: DEFAULT_HTTP_CONFIG.headers, // Use default headers to avoid type conflicts
      openobserve_url: '',
      authorization_header: '',
      retry_config: nodeType.App.retry_config || DEFAULT_RETRY_CONFIG
    }

    if (typeof nodeType.App.app_type === 'object' && nodeType.App.app_type.OpenObserve) {
      config.openobserve_url = nodeType.App.app_type.OpenObserve.url || ''
      config.authorization_header = nodeType.App.app_type.OpenObserve.authorization_header || ''
    }

    return config
  }

  return {
    type: 'trigger' as const,
    methods: ['POST']
  } as NodeConfig
}

export function convertNodeToApiType(node: { type: string; data: { config: NodeConfig } }) {
  switch (node.type) {
    case 'trigger':
      return {
        Trigger: {
          methods: (node.data.config as TriggerConfig).methods || ['POST']
        }
      }
      
    case 'condition':
      return {
        Condition: {
          script: (node.data.config as ConditionConfig).script || DEFAULT_CONDITION_SCRIPT
        }
      }
      
    case 'transformer':
      return {
        Transformer: {
          script: (node.data.config as TransformerConfig).script || DEFAULT_TRANSFORMER_SCRIPT
        }
      }
      
    case 'http-request':
      const httpRequestConfig = node.data.config as unknown as Record<string, unknown>
      debugLog.transform('http-config-to-api', {
        nodeId: (node as Record<string, unknown>).id || 'unknown',
        hasLoopConfig: !!httpRequestConfig.loop_config,
        loopConfigType: typeof httpRequestConfig.loop_config,
        loopConfig: httpRequestConfig.loop_config
      })

      return {
        HttpRequest: {
          url: httpRequestConfig.url || DEFAULT_HTTP_CONFIG.url,
          method: httpRequestConfig.method || DEFAULT_HTTP_CONFIG.method,
          timeout_seconds: httpRequestConfig.timeout_seconds || DEFAULT_HTTP_CONFIG.timeout_seconds,
          failure_action: httpRequestConfig.failure_action || DEFAULT_HTTP_CONFIG.failure_action,
          headers: httpRequestConfig.headers || DEFAULT_HTTP_CONFIG.headers,
          retry_config: httpRequestConfig.retry_config || DEFAULT_RETRY_CONFIG,
          loop_config: httpRequestConfig.loop_config || undefined
        }
      }
      
    case 'openobserve':
      const openobserveConfig = node.data.config as unknown as Record<string, unknown>
      return {
        OpenObserve: {
          url: openobserveConfig.url || DEFAULT_OPENOBSERVE_CONFIG.url,
          authorization_header: openobserveConfig.authorization_header || DEFAULT_OPENOBSERVE_CONFIG.authorization_header,
          timeout_seconds: openobserveConfig.timeout_seconds || DEFAULT_OPENOBSERVE_CONFIG.timeout_seconds,
          failure_action: openobserveConfig.failure_action || DEFAULT_OPENOBSERVE_CONFIG.failure_action,
          retry_config: openobserveConfig.retry_config || DEFAULT_RETRY_CONFIG
        }
      }
      
    case 'email':
      const emailConfig = node.data.config as unknown as Record<string, unknown>
      debugLog.transform('email-config-to-api', {
        nodeId: (node as Record<string, unknown>).id || 'unknown',
        hasTo: !!emailConfig.to,
        toCount: Array.isArray(emailConfig.to) ? emailConfig.to.length : 0,
        hasCC: !!emailConfig.cc,
        ccCount: Array.isArray(emailConfig.cc) ? emailConfig.cc.length : 0,
        hasBCC: !!emailConfig.bcc,
        bccCount: Array.isArray(emailConfig.bcc) ? emailConfig.bcc.length : 0
      })

      const result = {
        Email: {
          config: {
            to: emailConfig.to || DEFAULT_EMAIL_CONFIG.to,
            cc: emailConfig.cc || DEFAULT_EMAIL_CONFIG.cc,
            bcc: emailConfig.bcc || DEFAULT_EMAIL_CONFIG.bcc,
            subject: emailConfig.subject || DEFAULT_EMAIL_CONFIG.subject,
            template_type: emailConfig.template_type || DEFAULT_EMAIL_CONFIG.template_type,
            body_template: emailConfig.body_template || DEFAULT_EMAIL_CONFIG.body_template,
            text_body_template: emailConfig.text_body_template,
            attachments: emailConfig.attachments || DEFAULT_EMAIL_CONFIG.attachments
          }
        }
      }

      debugLog.transform('email-api-result', {
        hasResult: !!result,
        hasEmailConfig: !!result.Email?.config,
        finalToCount: Array.isArray(result.Email?.config?.to) ? result.Email.config.to.length : 0
      })

      return result
      
    case 'delay':
      const delayConfig = node.data.config as unknown as Record<string, unknown>
      return {
        Delay: {
          duration: delayConfig.duration || DEFAULT_DELAY_CONFIG.duration,
          unit: delayConfig.unit || DEFAULT_DELAY_CONFIG.unit
        }
      }

    case 'anthropic':
      const anthropicConfig = node.data.config as unknown as Record<string, unknown>
      return {
        Anthropic: {
          model: anthropicConfig.model || DEFAULT_ANTHROPIC_CONFIG.model,
          max_tokens: anthropicConfig.max_tokens || DEFAULT_ANTHROPIC_CONFIG.max_tokens,
          temperature: anthropicConfig.temperature || DEFAULT_ANTHROPIC_CONFIG.temperature,
          system_prompt: anthropicConfig.system_prompt || DEFAULT_ANTHROPIC_CONFIG.system_prompt,
          user_prompt: anthropicConfig.user_prompt || DEFAULT_ANTHROPIC_CONFIG.user_prompt,
          timeout_seconds: anthropicConfig.timeout_seconds || DEFAULT_ANTHROPIC_CONFIG.timeout_seconds,
          failure_action: anthropicConfig.failure_action || DEFAULT_ANTHROPIC_CONFIG.failure_action,
          retry_config: anthropicConfig.retry_config || DEFAULT_ANTHROPIC_CONFIG.retry_config
        }
      }

    case 'human-in-loop':
      const hilConfig = node.data.config as unknown as Record<string, unknown>
      return {
        HumanInLoop: {
          title: hilConfig.title || DEFAULT_HUMAN_IN_LOOP_CONFIG.title,
          description: hilConfig.description || DEFAULT_HUMAN_IN_LOOP_CONFIG.description,
          timeout_seconds: hilConfig.timeout_seconds || DEFAULT_HUMAN_IN_LOOP_CONFIG.timeout_seconds,
          timeout_action: hilConfig.timeout_action || DEFAULT_HUMAN_IN_LOOP_CONFIG.timeout_action,
          required_fields: hilConfig.required_fields || DEFAULT_HUMAN_IN_LOOP_CONFIG.required_fields,
          metadata: hilConfig.metadata || DEFAULT_HUMAN_IN_LOOP_CONFIG.metadata
        }
      }

    case 'app':
      // Legacy support for old App nodes
      const appConfig = node.data.config as unknown as Record<string, unknown>
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
          url: appConfig.url || DEFAULT_HTTP_CONFIG.url,
          method: appConfig.method || DEFAULT_HTTP_CONFIG.method,
          timeout_seconds: appConfig.timeout_seconds || DEFAULT_HTTP_CONFIG.timeout_seconds,
          failure_action: appConfig.failure_action || DEFAULT_HTTP_CONFIG.failure_action,
          headers: appConfig.headers || DEFAULT_HTTP_CONFIG.headers,
          retry_config: appConfig.retry_config || DEFAULT_RETRY_CONFIG
        }
      }
      
    default:
      throw new Error(`Unknown node type: ${node.type}`)
  }
}