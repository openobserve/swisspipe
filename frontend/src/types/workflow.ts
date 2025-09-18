export interface Node {
  id: string
  name: string
  node_type: NodeType
  position_x: number
  position_y: number
}

export interface Edge {
  id: string
  from_node_id: string
  to_node_id: string
  condition_result?: boolean
}

export interface Workflow {
  id: string
  name: string
  description?: string
  start_node_id: string
  endpoint_url: string
  enabled: boolean
  created_at: string
  updated_at: string
  nodes: Node[]
  edges: Edge[]
}

export interface WorkflowListResponse {
  workflows: Workflow[]
}

export interface CreateWorkflowRequest {
  name: string
  description?: string
  nodes: NodeRequest[]
  edges: EdgeRequest[]
}

export interface NodeRequest {
  id: string
  name: string
  node_type: NodeType
  position_x?: number
  position_y?: number
}

export interface EdgeRequest {
  from_node_id: string
  to_node_id: string
  condition_result?: boolean
}

export type NodeType = TriggerNode | ConditionNode | TransformerNode | HttpRequestNode | OpenObserveNode | AppNode | EmailNode | DelayNode | AnthropicNode

export interface TriggerNode {
  Trigger: {
    methods: HttpMethod[]
  }
}

export interface ConditionNode {
  Condition: {
    script: string
  }
}

export interface TransformerNode {
  Transformer: {
    script: string
  }
}

export interface HttpRequestNode {
  HttpRequest: {
    url: string
    method: HttpMethod
    timeout_seconds: number
    failure_action: FailureAction
    retry_config: RetryConfig
    headers: Record<string, string>
  }
}

export interface OpenObserveNode {
  OpenObserve: {
    url: string
    authorization_header: string
    timeout_seconds: number
    failure_action: FailureAction
    retry_config: RetryConfig
  }
}

// Legacy support for old App nodes
export interface AppNode {
  App: {
    app_type: AppType
    url: string
    method: HttpMethod
    timeout_seconds: number
    failure_action: FailureAction
    retry_config: RetryConfig
  }
}

export interface EmailNode {
  Email: {
    config: {
      smtp_config: string
      from: { email: string; name?: string }
      to: { email: string; name?: string }[]
      cc?: { email: string; name?: string }[]
      bcc?: { email: string; name?: string }[]
      subject: string
      template_type: 'html' | 'text'
      body_template: string
      text_body_template?: string
      attachments?: { filename: string; content_type: string; data: string }[]
      priority: 'critical' | 'high' | 'normal' | 'low'
      delivery_receipt: boolean
      read_receipt: boolean
      queue_if_rate_limited: boolean
      max_queue_wait_minutes: number
      bypass_rate_limit: boolean
    }
  }
}

export interface DelayNode {
  Delay: {
    duration: number
    unit: DelayUnit
  }
}

export interface AnthropicNode {
  Anthropic: {
    model: string
    max_tokens: number
    temperature: number
    system_prompt?: string
    user_prompt: string
    timeout_seconds: number
    failure_action: FailureAction
    retry_config: RetryConfig
  }
}

export type DelayUnit = 'Seconds' | 'Minutes' | 'Hours' | 'Days'

export type AppType = 'HttpRequest' | { OpenObserve: { url: string, authorization_header: string } }

export type FailureAction = 'Continue' | 'Stop' | 'Retry'

export interface RetryConfig {
  max_attempts: number
  initial_delay_ms: number
  max_delay_ms: number
  backoff_multiplier: number
}

export type HttpMethod = 'GET' | 'POST' | 'PUT' | 'DELETE'

export interface WorkflowExecution {
  id: string
  workflow_id: string
  status: ExecutionStatus
  started_at: string
  completed_at?: string
  result?: unknown
  error?: string
}

export type ExecutionStatus = 'running' | 'completed' | 'failed'

export interface ExecutionEvent {
  workflow_id: string
  execution_id: string
  node_name: string
  status: ExecutionStatus
  timestamp: string
  data?: unknown
}