import type { Node, Edge } from '@vue-flow/core'

export interface WorkflowNode {
  id: string
  type: NodeTypeString
  position: { x: number; y: number }
  data: {
    label: string
    description?: string
    config: NodeConfig
    status?: NodeStatus
  }
}

export interface WorkflowEdge extends Edge {
  id: string
  source: string
  target: string
  sourceHandle?: string
  targetHandle?: string
  data?: {
    condition_result?: boolean
  }
}

export type NodeTypeString = 'trigger' | 'condition' | 'transformer' | 'app'

export type NodeStatus = 'ready' | 'running' | 'completed' | 'error'

export type NodeConfig = TriggerConfig | ConditionConfig | TransformerConfig | AppConfig

export interface TriggerConfig {
  type: 'trigger'
  methods: string[]
}

export interface ConditionConfig {
  type: 'condition'
  script: string
}

export interface TransformerConfig {
  type: 'transformer'
  script: string
}

export interface AppConfig {
  type: 'app'
  app_type: AppType
  url: string
  method: string
  timeout_seconds: number
  failure_action: FailureAction
  headers?: Record<string, string>
  openobserve_url?: string
  authorization_header?: string
  stream_name?: string
  retry_config: {
    max_attempts: number
    initial_delay_ms: number
    max_delay_ms: number
    backoff_multiplier: number
  }
}

export type AppType = 'Webhook' | { OpenObserve: { url: string, authorization_header: string } }
export type FailureAction = 'Continue' | 'Stop' | 'Retry'

export interface NodeTypeDefinition {
  type: NodeTypeString
  label: string
  description: string
  color: string
  icon: string
  defaultConfig: NodeConfig
}

export interface ValidationState {
  isValid: boolean
  errors: string[]
  warnings: string[]
}