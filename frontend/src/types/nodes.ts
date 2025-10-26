import type { Edge } from '@vue-flow/core'

export interface WorkflowNode {
  id: string
  type: NodeTypeString
  position: { x: number; y: number }
  data: WorkflowNodeData
}

export interface WorkflowNodeData {
  label: string
  description?: string
  config: NodeConfig
  input_merge_strategy?: InputMergeStrategy
  status?: NodeStatus
  isTracing?: boolean
  tracingExecutionId?: string
  executionStatus?: string
  executionDuration?: number
  executionError?: string
  executionInput?: unknown
  executionOutput?: unknown
  name?: string
  app_type?: string
  condition_type?: string
  transformer_type?: string
  type?: string
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

export type NodeTypeString = 'trigger' | 'condition' | 'transformer' | 'http-request' | 'openobserve' | 'app' | 'email' | 'delay' | 'anthropic' | 'human-in-loop'

export type NodeStatus = 'ready' | 'running' | 'completed' | 'error'

export type NodeConfig = TriggerConfig | ConditionConfig | TransformerConfig | HttpRequestConfig | OpenObserveConfig | AppConfig | EmailConfig | DelayConfig | AnthropicConfig | HumanInLoopConfig

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

export interface HttpRequestConfig {
  type: 'http-request'
  url: string
  method: string
  timeout_seconds: number
  failure_action: FailureAction
  headers?: Record<string, string>
  retry_config: {
    max_attempts: number
    initial_delay_ms: number
    max_delay_ms: number
    backoff_multiplier: number
  }
  loop_config?: LoopConfig
}

export interface LoopConfig {
  max_iterations?: number
  interval_seconds: number
  backoff_strategy: BackoffStrategy
  termination_condition?: TerminationCondition
}

export type BackoffStrategy =
  | { Fixed: number }
  | { Exponential: { base: number; multiplier: number; max: number } }

export interface TerminationCondition {
  script: string            // JavaScript function: function condition(event) { return boolean; }
  action: TerminationAction // Success | Failure | Stop
}

export type TerminationAction = 'Success' | 'Failure' | 'Stop'

export interface LoopStatus {
  loop_id: string
  execution_step_id: string
  current_iteration: number
  max_iterations?: number
  next_execution_at?: number
  consecutive_failures: number
  loop_started_at: number
  last_response_status?: number
  last_response_body?: string
  status: string
  termination_reason?: string
  created_at: number
  updated_at: number
  success_rate?: number
}

export interface OpenObserveConfig {
  type: 'openobserve'
  url: string
  authorization_header: string
  timeout_seconds: number
  failure_action: FailureAction
  retry_config: {
    max_attempts: number
    initial_delay_ms: number
    max_delay_ms: number
    backoff_multiplier: number
  }
}

export interface EmailConfig {
  type: 'email'
  to: EmailAddress[]
  cc?: EmailAddress[]
  bcc?: EmailAddress[]
  reply_to?: EmailAddress
  subject: string
  template_type: 'html' | 'text'
  body_template: string
  text_body_template?: string
  attachments?: EmailAttachment[]
}

export interface DelayConfig {
  type: 'delay'
  duration: number
  unit: DelayUnit
}

export interface AnthropicConfig {
  type: 'anthropic'
  model: string
  max_tokens: number
  temperature: number
  system_prompt?: string
  user_prompt: string
  timeout_seconds: number
  failure_action: FailureAction
  retry_config: {
    max_attempts: number
    initial_delay_ms: number
    max_delay_ms: number
    backoff_multiplier: number
  }
}

export interface HumanInLoopConfig {
  type: 'human-in-loop'
  title: string
  description?: string
  timeout_seconds?: number
  timeout_action?: string
  required_fields?: string[]
  metadata?: Record<string, unknown>
}

// Legacy support for old App nodes
export interface AppConfig {
  type: 'app'
  app_type: string
  url: string
  method: string
  timeout_seconds: number
  failure_action: string
  headers: Record<string, string>
  openobserve_url?: string
  authorization_header?: string
  retry_config: {
    max_attempts: number
    initial_delay_ms: number
    max_delay_ms: number
    backoff_multiplier: number
  }
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

// Input Merge Strategy types matching Rust backend
export type InputMergeStrategy =
  | { WaitForAll: null }
  | { FirstWins: null }
  | { TimeoutBased: number }

export interface InputMergeStrategyOption {
  type: 'WaitForAll' | 'FirstWins' | 'TimeoutBased'
  label: string
  description: string
  timeout?: number
}