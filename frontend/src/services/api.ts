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

class ApiClient {
  private client: AxiosInstance

  constructor() {
    this.client = axios.create({
      baseURL: import.meta.env.VITE_API_BASE_URL || 'http://localhost:3701',
      timeout: 10000,
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
        // Add Basic Auth for admin management endpoints only
        if (config.url?.includes('/api/admin/')) {
          // Try to get credentials from localStorage first (from auth store)
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
              console.warn('API credentials not configured. Either log in or set VITE_API_USERNAME and VITE_API_PASSWORD environment variables.')
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
  async executeScript(script: string, input: unknown): Promise<unknown> {
    const response = await this.client.post('/api/admin/v1/script/execute', {
      script,
      input
    })
    return response.data
  }

  // AI Code generation endpoint
  async generateCode(request: GenerateCodeRequest): Promise<GenerateCodeResponse> {
    const response = await this.client.post<GenerateCodeResponse>('/api/v1/ai/generate-code', request, {
      timeout: 120000 // 120 seconds for AI generation requests
    })
    return response.data
  }

  // AI Workflow generation endpoint
  async generateWorkflow(request: GenerateWorkflowRequest): Promise<GenerateWorkflowResponse> {
    const response = await this.client.post<GenerateWorkflowResponse>('/api/v1/ai/generate-workflow', request, {
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
}

export const apiClient = new ApiClient()
export default apiClient