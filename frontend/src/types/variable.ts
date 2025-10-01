export type VariableType = 'text' | 'secret'

export interface Variable {
  id: string
  name: string
  value_type: VariableType
  value: string
  description?: string
  created_at: number
  updated_at: number
}

export interface CreateVariableRequest {
  name: string
  value_type: VariableType
  value: string
  description?: string
}

export interface UpdateVariableRequest {
  value: string
  description?: string
}

export interface VariablesListResponse {
  variables: Variable[]
}

export interface ValidateVariableNameResponse {
  valid: boolean
  error?: string
}
