import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'
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
  const workflowNameFilter = ref('')
  const selectedExecution = ref<WorkflowExecution | null>(null)
  const executionSteps = ref<ExecutionStep[]>([])
  const showSidePanel = ref(false)

  // Pagination state
  const currentPage = ref(1)
  const pageSize = ref(50)
  const totalCount = ref(0)

  // Computed - now just returns executions since filtering is done server-side
  const filteredExecutions = computed(() => {
    let filtered = executions.value

    // Apply general search term filter (client-side for non-workflow name fields)
    if (searchTerm.value) {
      const search = searchTerm.value.toLowerCase()
      filtered = filtered.filter(execution =>
        execution.id.toLowerCase().includes(search) ||
        execution.workflow_id.toLowerCase().includes(search) ||
        execution.status.toLowerCase().includes(search) ||
        execution.current_node_id?.toLowerCase().includes(search)
      )
    }

    return filtered
  })

  const executionCount = computed(() => filteredExecutions.value.length)

  // Computed pagination properties
  const totalPages = computed(() => Math.ceil(totalCount.value / pageSize.value))
  const hasNextPage = computed(() => currentPage.value < totalPages.value)
  const hasPreviousPage = computed(() => currentPage.value > 1)

  // Actions
  async function fetchExecutions() {
    loading.value = true
    error.value = null
    try {
      const offset = (currentPage.value - 1) * pageSize.value
      const response = await apiClient.getExecutions(
        pageSize.value,
        workflowNameFilter.value || undefined,
        offset
      )
      executions.value = response.executions
      // Use total_count from backend if available
      if (response.total_count !== undefined) {
        totalCount.value = response.total_count
      } else {
        // Fallback: estimate based on current page results
        if (response.executions.length < pageSize.value) {
          totalCount.value = offset + response.executions.length
        } else {
          totalCount.value = offset + response.executions.length + 1
        }
      }
    } catch (err: unknown) {
      error.value = (err as Error).message || 'Failed to fetch executions'
      console.error('Error fetching executions:', err)
    } finally {
      loading.value = false
    }
  }

  function nextPage() {
    if (hasNextPage.value) {
      currentPage.value++
      fetchExecutions()
    }
  }

  function previousPage() {
    if (hasPreviousPage.value) {
      currentPage.value--
      fetchExecutions()
    }
  }

  function goToPage(page: number) {
    if (page >= 1 && page <= totalPages.value) {
      currentPage.value = page
      fetchExecutions()
    }
  }

  function setPageSize(size: number) {
    pageSize.value = size
    currentPage.value = 1
    fetchExecutions()
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

  // Debounced function for workflow name filtering
  let workflowFilterTimeout: number | null = null
  function debouncedFetchExecutions() {
    if (workflowFilterTimeout) {
      clearTimeout(workflowFilterTimeout)
    }
    workflowFilterTimeout = setTimeout(() => {
      fetchExecutions()
    }, 300) // 300ms debounce
  }

  // Watch for workflow name filter changes and trigger server-side filtering
  watch(workflowNameFilter, (newValue, oldValue) => {
    if (newValue !== oldValue) {
      currentPage.value = 1 // Reset to first page when filter changes
      debouncedFetchExecutions()
    }
  })

  return {
    // State
    executions,
    loading,
    error,
    searchTerm,
    workflowNameFilter,
    selectedExecution,
    executionSteps,
    showSidePanel,

    // Pagination state
    currentPage,
    pageSize,
    totalCount,

    // Computed
    filteredExecutions,
    executionCount,
    totalPages,
    hasNextPage,
    hasPreviousPage,

    // Actions
    fetchExecutions,
    fetchExecution,
    fetchExecutionSteps,
    cancelExecution,
    openExecutionDetails,
    closeSidePanel,
    getStatusColor,
    formatTimestamp,
    formatDuration,

    // Pagination actions
    nextPage,
    previousPage,
    goToPage,
    setPageSize
  }
})