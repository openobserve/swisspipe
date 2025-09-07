export interface Workflow {
  id: string
  name: string
  description?: string
  start_node_name: string
  endpoint_url: string
  created_at: string
  updated_at: string
}

export interface WorkflowListResponse {
  workflows: Workflow[]
}

export interface CreateWorkflowRequest {
  name: string
  description?: string
  start_node_name: string
  nodes: NodeRequest[]
  edges: EdgeRequest[]
}

export interface NodeRequest {
  name: string
  node_type: NodeType
}

export interface EdgeRequest {
  from_node_name: string
  to_node_name: string
  condition_result?: boolean
}

export type NodeType = TriggerNode | ConditionNode | TransformerNode | AppNode

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

export interface AppNode {
  App: {
    app_type: string
    url: string
    method: HttpMethod
    timeout_seconds: number
    retry_config: RetryConfig
  }
}

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
  result?: any
  error?: string
}

export type ExecutionStatus = 'running' | 'completed' | 'failed'

export interface ExecutionEvent {
  workflow_id: string
  execution_id: string
  node_name: string
  status: ExecutionStatus
  timestamp: string
  data?: any
}