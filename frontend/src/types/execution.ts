export type ExecutionStatus = 'pending' | 'running' | 'completed' | 'failed' | 'cancelled'

export type StepStatus = 'pending' | 'running' | 'completed' | 'failed' | 'skipped'

export interface WorkflowExecution {
  id: string
  workflow_id: string
  workflow_name?: string
  status: ExecutionStatus
  current_node_id?: string
  input_data?: unknown
  output_data?: unknown
  error_message?: string
  started_at?: number
  completed_at?: number
  duration_ms?: number
  created_at: number
  updated_at: number
}

export interface ExecutionStep {
  id: string
  execution_id: string
  node_id: string
  node_name: string
  status: StepStatus
  input_data?: unknown
  output_data?: unknown
  error_message?: string
  started_at?: number
  completed_at?: number
  created_at: number
}

export interface ExecutionListResponse {
  executions: WorkflowExecution[]
  count: number
}

export interface ExecutionStepsResponse {
  execution_id: string
  steps: ExecutionStep[]
}

