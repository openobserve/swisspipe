import axios, { type AxiosInstance, type AxiosResponse } from 'axios'
import type { ApiError } from '../types/api'
import type { Workflow, WorkflowListResponse, CreateWorkflowRequest } from '../types/workflow'
import type {
  WorkflowExecution,
  ExecutionListResponse,
  ExecutionStepsResponse
} from '../types/execution'

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
            // Fallback to Basic Auth for non-OAuth users
            const storedCredentials = localStorage.getItem('auth_credentials')

            if (storedCredentials) {
              config.headers.Authorization = `Basic ${storedCredentials}`
            } else {
              // Fallback to environment variables
              const username = import.meta.env.VITE_API_USERNAME
              const password = import.meta.env.VITE_API_PASSWORD

              if (username && password) {
                const token = btoa(`${username}:${password}`)
                config.headers.Authorization = `Basic ${token}`
              } else {
                // If no basic auth, try session-based (cookies will be sent with withCredentials)
                config.withCredentials = true
              }
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
  async getExecutions(limit?: number, offset?: number, workflowId?: string, status?: string): Promise<ExecutionListResponse> {
    const params = new URLSearchParams()
    if (limit) params.append('limit', limit.toString())
    if (offset) params.append('offset', offset.toString())
    if (workflowId) params.append('workflow_id', workflowId)
    if (status) params.append('status', status)
    
    const response = await this.client.get<ExecutionListResponse>(`/api/admin/v1/executions?${params}`)
    return response.data
  }

  async getExecution(executionId: string): Promise<WorkflowExecution> {
    const response = await this.client.get<WorkflowExecution>(`/api/admin/v1/executions/${executionId}`)
    return response.data
  }

  async getExecutionSteps(executionId: string): Promise<ExecutionStepsResponse> {
    const response = await this.client.get<ExecutionStepsResponse>(`/api/admin/v1/executions/${executionId}/steps`)
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

  async getDefaultEmailSettings(): Promise<{ defaultFromEmail: string; defaultFromName: string }> {
    const settings = await this.getSettings()
    const defaultFromEmail = settings.settings.find(s => s.key === 'default_from_email')?.value || ''
    const defaultFromName = settings.settings.find(s => s.key === 'default_from_name')?.value || ''
    return { defaultFromEmail, defaultFromName }
  }
}

export const apiClient = new ApiClient()
export default apiClient