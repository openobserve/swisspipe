import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import apiClient from '../services/api'
import type { 
  WorkflowExecution, 
  ExecutionStep,
  ExecutionStatus 
} from '../types/execution'

export const useExecutionStore = defineStore('executions', () => {
  // State
  const executions = ref<WorkflowExecution[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)
  const searchTerm = ref('')
  const selectedExecution = ref<WorkflowExecution | null>(null)
  const executionSteps = ref<ExecutionStep[]>([])
  const showSidePanel = ref(false)

  // Computed
  const filteredExecutions = computed(() => {
    if (!searchTerm.value) return executions.value
    
    const search = searchTerm.value.toLowerCase()
    return executions.value.filter(execution => 
      execution.id.toLowerCase().includes(search) ||
      execution.workflow_id.toLowerCase().includes(search) ||
      execution.status.toLowerCase().includes(search) ||
      execution.current_node_id?.toLowerCase().includes(search)
    )
  })

  const executionCount = computed(() => executions.value.length)

  // Actions
  async function fetchExecutions() {
    loading.value = true
    error.value = null
    try {
      const response = await apiClient.getExecutions(50) // Get last 50 executions
      executions.value = response.executions
    } catch (err: unknown) {
      error.value = (err as Error).message || 'Failed to fetch executions'
      console.error('Error fetching executions:', err)
    } finally {
      loading.value = false
    }
  }

  async function fetchExecution(executionId: string) {
    try {
      const execution = await apiClient.getExecution(executionId)
      selectedExecution.value = execution
      return execution
    } catch (err: unknown) {
      error.value = (err as Error).message || 'Failed to fetch execution details'
      console.error('Error fetching execution:', err)
      throw err
    }
  }

  async function fetchExecutionSteps(executionId: string) {
    try {
      console.log('Fetching execution steps for:', executionId)

      // Validate executionId
      if (!executionId || typeof executionId !== 'string') {
        throw new Error('Invalid execution ID provided')
      }

      const response = await apiClient.getExecutionSteps(executionId)
      console.log('Received execution steps:', response.steps)

      // Validate response structure
      if (!response || !Array.isArray(response.steps)) {
        console.warn('Invalid response structure from getExecutionSteps:', response)
        executionSteps.value = []
        return []
      }

      executionSteps.value = response.steps
      return response.steps
    } catch (err: unknown) {
      console.error('Error fetching execution steps:', err, 'for execution:', executionId)
      error.value = (err as Error).message || 'Failed to fetch execution steps'
      executionSteps.value = [] // Reset to empty array on error
      throw err
    }
  }


  async function cancelExecution(executionId: string) {
    try {
      await apiClient.cancelExecution(executionId)
      // Update local state
      const execution = executions.value.find(e => e.id === executionId)
      if (execution) {
        execution.status = 'cancelled'
      }
      if (selectedExecution.value?.id === executionId) {
        selectedExecution.value.status = 'cancelled'
      }
    } catch (err: unknown) {
      error.value = (err as Error).message || 'Failed to cancel execution'
      console.error('Error cancelling execution:', err)
      throw err
    }
  }

  function openExecutionDetails(execution: WorkflowExecution) {
    try {
      console.log('Opening execution details for:', execution.id)

      // Validate execution object
      if (!execution || !execution.id) {
        throw new Error('Invalid execution object provided')
      }

      selectedExecution.value = execution
      showSidePanel.value = true

      // Load steps for the selected execution - wrapped in setTimeout to ensure it runs after current tick
      setTimeout(() => {
        fetchExecutionSteps(execution.id).catch((err) => {
          console.error('Failed to load execution steps asynchronously:', err)
          // Don't set global error since panel is already open
        })
      }, 0)
    } catch (err: unknown) {
      console.error('Error in openExecutionDetails:', err)
      error.value = 'Failed to open execution details: ' + (err as Error).message
    }
  }

  function closeSidePanel() {
    showSidePanel.value = false
    selectedExecution.value = null
    executionSteps.value = []
  }

  function getStatusColor(status: ExecutionStatus): string {
    switch (status) {
      case 'pending':
        return 'bg-gray-100 text-gray-800'
      case 'running':
        return 'bg-blue-100 text-blue-800'
      case 'completed':
        return 'bg-green-100 text-green-800'
      case 'failed':
        return 'bg-red-100 text-red-800'
      case 'cancelled':
        return 'bg-yellow-100 text-yellow-800'
      default:
        return 'bg-gray-100 text-gray-800'
    }
  }

  function formatTimestamp(timestamp: number | undefined): string {
    if (!timestamp) return 'N/A'
    return new Date(timestamp / 1000).toLocaleString()
  }

  function formatDuration(startedAt?: number, completedAt?: number): string {
    if (!startedAt) return 'N/A'
    if (!completedAt) return 'Running...'
    
    const durationMs = (completedAt - startedAt) / 1000
    if (durationMs < 1000) return `${Math.round(durationMs)}ms`
    if (durationMs < 60000) return `${Math.round(durationMs / 1000)}s`
    return `${Math.round(durationMs / 60000)}m`
  }

  return {
    // State
    executions,
    loading,
    error,
    searchTerm,
    selectedExecution,
    executionSteps,
    showSidePanel,
    
    // Computed
    filteredExecutions,
    executionCount,
    
    // Actions
    fetchExecutions,
    fetchExecution,
    fetchExecutionSteps,
    cancelExecution,
    openExecutionDetails,
    closeSidePanel,
    getStatusColor,
    formatTimestamp,
    formatDuration
  }
})