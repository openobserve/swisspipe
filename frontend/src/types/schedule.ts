export interface Schedule {
  id: string
  workflow_id: string
  trigger_node_id: string
  schedule_name?: string
  cron_expression: string
  timezone: string
  test_payload: Record<string, unknown>
  enabled: boolean
  start_date?: string
  end_date?: string
  last_execution_time?: string
  next_execution_time?: string
  execution_count: number
  failure_count: number
  created_at: string
  updated_at: string
}

export interface ScheduleConfig {
  schedule_name?: string
  cron_expression: string
  timezone: string
  test_payload: Record<string, unknown>
  enabled: boolean
  start_date?: string
  end_date?: string
}

export interface CronValidationResponse {
  valid: boolean
  next_executions: string[]
}

export interface ScheduleFormData {
  schedule_name: string
  cron_expression: string
  timezone: string
  test_payload: string // JSON string
  enabled: boolean
  start_date: string
  end_date: string
}
