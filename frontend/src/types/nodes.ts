import type { Edge } from '@vue-flow/core'

export interface WorkflowNode {
  id: string
  type: NodeTypeString
  position: { x: number; y: number }
  data: {
    label: string
    description?: string
    config: NodeConfig
    status?: NodeStatus
    isTracing?: boolean
    executionStatus?: string
    executionDuration?: number
    executionError?: string
    executionInput?: unknown
    executionOutput?: unknown
  }
}

export interface WorkflowEdge extends Omit<Edge, 'data'> {
  id: string
  source: string
  target: string
  sourceHandle?: string
  targetHandle?: string
  data?: {
    condition_result?: boolean
  }
}

export type NodeTypeString = 'trigger' | 'condition' | 'transformer' | 'app' | 'email' | 'delay'

export type NodeStatus = 'ready' | 'running' | 'completed' | 'error'

export type NodeConfig = TriggerConfig | ConditionConfig | TransformerConfig | AppConfig | EmailConfig | DelayConfig

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

export interface EmailConfig {
  type: 'email'
  smtp_config: string
  from: EmailAddress
  to: EmailAddress[]
  cc?: EmailAddress[]
  bcc?: EmailAddress[]
  subject: string
  template_type: 'html' | 'text'
  body_template: string
  text_body_template?: string
  attachments?: EmailAttachment[]
  priority: 'critical' | 'high' | 'normal' | 'low'
  delivery_receipt: boolean
  read_receipt: boolean
  queue_if_rate_limited: boolean
  max_queue_wait_minutes: number
  bypass_rate_limit: boolean
}

export interface DelayConfig {
  type: 'delay'
  duration: number
  unit: DelayUnit
}

export type DelayUnit = 'Seconds' | 'Minutes' | 'Hours' | 'Days'

export interface EmailAddress {
  email: string
  name?: string
}

export interface EmailAttachment {
  filename: string
  content_type: string
  data: string
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