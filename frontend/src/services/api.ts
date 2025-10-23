import axios, { type AxiosInstance, type AxiosResponse } from 'axios'
import type { ApiError } from '../types/api'
import type { Workflow, WorkflowListResponse, CreateWorkflowRequest } from '../types/workflow'
import type {
  WorkflowExecution,
  ExecutionListResponse,
  ExecutionStepsResponse
} from '../types/execution'
import type { LoopStatus } from '../types/nodes'
import type {
  Variable,
  VariablesListResponse,
  CreateVariableRequest,
  UpdateVariableRequest,
  ValidateVariableNameResponse
} from '../types/variable'
import type { Schedule, ScheduleConfig, CronValidationResponse } from '../types/schedule'

// AI Code Generation types
interface GenerateCodeRequest {
  system_prompt: string
  user_prompt: string
  model?: string
  max_tokens?: number
  temperature?: number
}

interface GenerateCodeResponse {
  response: string
  usage?: unknown
}

// AI Workflow Generation types
interface GenerateWorkflowRequest {
  prompt: string
}

interface GenerateWorkflowResponse {
  success: boolean
  workflow_id?: string
  workflow_name?: string
  error?: string
}

// AI Workflow Update types
interface UpdateWorkflowRequest {
  workflow_id: string
  prompt: string
}

interface UpdateWorkflowResponse {
  success: boolean
  message: string
  workflow_name?: string
  changes_made: string[]
  error?: string
}

// Workflow Search types
interface WorkflowSearchResult {
  id: string
  name: string
  description?: string
}

// Google OAuth types
interface LoginResponse {
  success: boolean
  message: string
  user?: {
    id: string
    email: string
    name: string
    picture?: string
  }
  session_id?: string
}

// Settings types
interface SettingResponse {
  key: string
  value: string
  description?: string
  created_at: number
  updated_at: number
}

interface SettingsListResponse {
  settings: SettingResponse[]
}

class ApiClient {
  private client: AxiosInstance

  constructor() {
    // In development, use VITE_API_BASE_URL or fallback to localhost:3700
    // In production, use the same origin as the frontend (since they're served together)
    const getBaseURL = () => {
      // Check if we're in development mode
      if (import.meta.env.DEV) {
        return import.meta.env.VITE_API_BASE_URL || 'http://localhost:3700'
      }
      // In production, frontend and backend are served from the same origin
      return window.location.origin
    }

    this.client = axios.create({
      baseURL: getBaseURL(),
      timeout: 10000,
      withCredentials: true, // Always include cookies for session management
      headers: {
        'Content-Type': 'application/json'
      }
    })

    this.setupInterceptors()
  }

  private setupInterceptors() {
    // Request interceptor for auth
    this.client.interceptors.request.use(
      (config) => {
        // Add auth for admin management endpoints, AI endpoints, and OAuth endpoints
        if (config.url?.includes('/api/admin/') || config.url?.includes('/api/ai/') || config.url?.includes('/auth/')) {
          // For OAuth endpoints (/auth/user, /auth/logout), rely on cookies - no explicit headers needed
          if (config.url?.includes('/auth/')) {
            // Cookies will be sent automatically with credentials: 'include'
            config.withCredentials = true
            return config
          }

          // For admin/AI endpoints, check if user is authenticated via OAuth first
          const userInfo = JSON.parse(localStorage.getItem('oauth_user') || 'null')

          if (userInfo && userInfo.session_id) {
            // User is authenticated via OAuth - use session cookies
            config.withCredentials = true
          } else {
            // Use Basic Auth for non-OAuth users
            const storedCredentials = localStorage.getItem('auth_credentials')

            if (storedCredentials) {
              config.headers.Authorization = `Basic ${storedCredentials}`
            } else {
              // No credentials available - let the request proceed without auth
              // The backend will return 401 if auth is required
              config.withCredentials = true
            }
          }
        }
        return config
      },
      (error) => {
        return Promise.reject(error)
      }
    )

    // Response interceptor for error handling
    this.client.interceptors.response.use(
      (response: AxiosResponse) => response,
      (error) => {
        const apiError: ApiError = {
          message: error.response?.data?.message || error.message || 'An unexpected error occurred',
          status: error.response?.status || 0,
          code: error.response?.data?.code
        }
        return Promise.reject(apiError)
      }
    )
  }

  // Workflow endpoints
  async getWorkflows(): Promise<WorkflowListResponse> {
    const response = await this.client.get<WorkflowListResponse>('/api/admin/v1/workflows')
    return response.data
  }

  async getWorkflow(id: string): Promise<Workflow> {
    const response = await this.client.get<Workflow>(`/api/admin/v1/workflows/${id}`)
    return response.data
  }

  async createWorkflow(workflow: CreateWorkflowRequest): Promise<Workflow> {
    const response = await this.client.post<Workflow>('/api/admin/v1/workflows', workflow)
    return response.data
  }

  async updateWorkflow(id: string, workflow: CreateWorkflowRequest): Promise<Workflow> {
    const response = await this.client.put<Workflow>(`/api/admin/v1/workflows/${id}`, workflow)
    return response.data
  }

  async deleteWorkflow(id: string): Promise<void> {
    await this.client.delete(`/api/admin/v1/workflows/${id}`)
  }

  async enableWorkflow(id: string): Promise<void> {
    await this.client.put(`/api/admin/v1/workflows/${id}/enable`)
  }

  async searchWorkflows(searchTerm: string): Promise<WorkflowSearchResult[]> {
    const params = new URLSearchParams()
    if (searchTerm) params.append('search', searchTerm)

    const response = await this.client.get<WorkflowSearchResult[]>(`/api/admin/v1/workflows/search?${params}`)
    return response.data
  }

  // Workflow execution endpoints
  async executeWorkflow(workflowId: string, data: unknown): Promise<unknown> {
    const response = await this.client.post(`/api/v1/${workflowId}/trigger`, data)
    return response.data
  }

  async executeWorkflowArray(workflowId: string, data: unknown[]): Promise<unknown> {
    const response = await this.client.post(`/api/v1/${workflowId}/json_array`, data)
    return response.data
  }

  // Execution management endpoints
  async getExecutions(limit?: number, workflowName?: string, offset?: number, workflowId?: string, status?: string): Promise<ExecutionListResponse> {
    const params = new URLSearchParams()
    if (limit) params.append('limit', limit.toString())
    if (offset) params.append('offset', offset.toString())
    if (workflowId) params.append('workflow_id', workflowId)
    if (status) params.append('status', status)
    if (workflowName) params.append('workflow_name', workflowName)

    const response = await this.client.get<ExecutionListResponse>(`/api/admin/v1/executions?${params}`)
    return response.data
  }

  async getExecution(executionId: string): Promise<WorkflowExecution> {
    const response = await this.client.get<WorkflowExecution>(`/api/admin/v1/executions/${executionId}`)
    return response.data
  }

  async getExecutionSteps(executionId: string): Promise<ExecutionStepsResponse> {
    // Validate input
    if (!executionId || typeof executionId !== 'string' || executionId.trim() === '') {
      throw new Error('Invalid execution ID provided to getExecutionSteps')
    }

    const response = await this.client.get<ExecutionStepsResponse>(`/api/admin/v1/executions/${executionId.trim()}/steps`)

    // Validate response structure
    if (!response.data || typeof response.data !== 'object') {
      throw new Error('Invalid response from execution steps API')
    }

    // Ensure steps is always an array
    if (!Array.isArray(response.data.steps)) {
      response.data.steps = []
    }

    return response.data
  }

  async cancelExecution(executionId: string): Promise<void> {
    await this.client.post(`/api/admin/v1/executions/${executionId}/cancel`)
  }

  async getExecutionsByWorkflow(workflowId: string, limit?: number, offset?: number, status?: string): Promise<ExecutionListResponse> {
    const params = new URLSearchParams()
    params.append('workflow_id', workflowId)
    if (limit) params.append('limit', limit.toString())
    if (offset) params.append('offset', offset.toString())
    if (status) params.append('status', status)
    
    const response = await this.client.get<ExecutionListResponse>(`/api/admin/v1/executions?${params}`)
    return response.data
  }

  // Script execution endpoint
  async executeScript(script: string, input: unknown, scriptType?: 'transformer' | 'condition'): Promise<unknown> {
    const response = await this.client.post('/api/admin/v1/script/execute', {
      script,
      input,
      script_type: scriptType
    })
    return response.data
  }

  // AI Code generation endpoint
  async generateCode(request: GenerateCodeRequest): Promise<GenerateCodeResponse> {
    const response = await this.client.post<GenerateCodeResponse>('/api/admin/v1/ai/generate-code', request, {
      timeout: 120000 // 120 seconds for AI generation requests
    })
    return response.data
  }

  // AI Workflow generation endpoint
  async generateWorkflow(request: GenerateWorkflowRequest): Promise<GenerateWorkflowResponse> {
    const response = await this.client.post<GenerateWorkflowResponse>('/api/admin/v1/ai/generate-workflow', request, {
      timeout: 120000 // 120 seconds for AI generation requests
    })
    return response.data
  }

  // AI Workflow update endpoint
  async updateWorkflowWithAI(request: UpdateWorkflowRequest): Promise<UpdateWorkflowResponse> {
    const response = await this.client.post<UpdateWorkflowResponse>('/api/admin/v1/ai/update-workflow', request, {
      timeout: 120000 // 120 seconds for AI generation requests
    })
    return response.data
  }

  // Auth validation endpoint
  async validateCredentials(credentials: string): Promise<boolean> {
    try {
      const response = await this.client.get('/api/admin/v1/workflows', {
        headers: {
          'Authorization': `Basic ${credentials}`
        }
      })
      return response.status === 200
    } catch {
      return false
    }
  }

  // Google OAuth endpoints
  async getCurrentUser(): Promise<LoginResponse> {
    const response = await this.client.get<LoginResponse>('/auth/user')
    return response.data
  }

  async logout(): Promise<LoginResponse> {
    const response = await this.client.get<LoginResponse>('/auth/logout')
    return response.data
  }

  // Settings endpoints
  async getSettings(): Promise<SettingsListResponse> {
    const response = await this.client.get<SettingsListResponse>('/api/admin/v1/settings')
    return response.data
  }

  async getSetting(key: string): Promise<SettingResponse> {
    const response = await this.client.get<SettingResponse>(`/api/admin/v1/settings/${key}`)
    return response.data
  }

  async updateSetting(key: string, value: string): Promise<SettingResponse> {
    const response = await this.client.put<SettingResponse>(`/api/admin/v1/settings/${key}`, { value })
    return response.data
  }


  // HTTP Loop endpoints
  async getLoopStatus(loopId: string): Promise<LoopStatus> {
    const response = await this.client.get<LoopStatus>(`/api/admin/v1/loops/${loopId}`)
    return response.data
  }

  async getActiveLoops(execution_id?: string): Promise<LoopStatus[]> {
    const params = execution_id ? { execution_id } : {}
    const response = await this.client.get<LoopStatus[]>('/api/admin/v1/loops/active', { params })
    return response.data
  }

  async pauseLoop(loopId: string): Promise<void> {
    await this.client.post(`/api/admin/v1/loops/${loopId}/pause`)
  }

  async resumeLoop(loopId: string): Promise<void> {
    await this.client.post(`/api/admin/v1/loops/${loopId}/resume`)
  }

  async stopLoop(loopId: string): Promise<void> {
    await this.client.post(`/api/admin/v1/loops/${loopId}/stop`)
  }

  async retryLoop(loopId: string): Promise<void> {
    await this.client.post(`/api/admin/v1/loops/${loopId}/retry`)
  }

  // Environment Variables API
  async getVariables(): Promise<VariablesListResponse> {
    const response = await this.client.get<VariablesListResponse>('/api/admin/v1/variables')
    return response.data
  }

  async getVariable(id: string): Promise<Variable> {
    const response = await this.client.get<Variable>(`/api/admin/v1/variables/${id}`)
    return response.data
  }

  async createVariable(data: CreateVariableRequest): Promise<Variable> {
    const response = await this.client.post<Variable>('/api/admin/v1/variables', data)
    return response.data
  }

  async updateVariable(id: string, data: UpdateVariableRequest): Promise<Variable> {
    const response = await this.client.put<Variable>(`/api/admin/v1/variables/${id}`, data)
    return response.data
  }

  async deleteVariable(id: string): Promise<void> {
    await this.client.delete(`/api/admin/v1/variables/${id}`)
  }

  async validateVariableName(name: string): Promise<ValidateVariableNameResponse> {
    const response = await this.client.post<ValidateVariableNameResponse>(
      '/api/admin/v1/variables/validate',
      { name }
    )
    return response.data
  }

  // Schedule API
  async upsertSchedule(
    workflowId: string,
    nodeId: string,
    config: ScheduleConfig
  ): Promise<Schedule> {
    const response = await this.client.put<Schedule>(
      `/api/admin/v1/workflows/${workflowId}/triggers/${nodeId}/schedule`,
      config
    )
    return response.data
  }

  async getSchedule(workflowId: string, nodeId: string): Promise<Schedule | null> {
    try {
      const response = await this.client.get<Schedule>(
        `/api/admin/v1/workflows/${workflowId}/triggers/${nodeId}/schedule`
      )
      return response.data
    } catch (error) {
      const apiError = error as ApiError
      if (apiError.status === 404) {
        return null
      }
      throw error
    }
  }

  async updateScheduleEnabled(
    workflowId: string,
    nodeId: string,
    enabled: boolean
  ): Promise<void> {
    await this.client.patch(
      `/api/admin/v1/workflows/${workflowId}/triggers/${nodeId}/schedule`,
      { enabled }
    )
  }

  async deleteSchedule(workflowId: string, nodeId: string): Promise<void> {
    await this.client.delete(`/api/admin/v1/workflows/${workflowId}/triggers/${nodeId}/schedule`)
  }

  async validateCron(
    cronExpression: string,
    timezone: string
  ): Promise<CronValidationResponse> {
    const response = await this.client.post<CronValidationResponse>(
      '/api/admin/v1/schedules/validate',
      {
        cron_expression: cronExpression,
        timezone
      }
    )
    return response.data
  }
}

export const apiClient = new ApiClient()
export default apiClient

// Export types for use in components
export type { WorkflowSearchResult }