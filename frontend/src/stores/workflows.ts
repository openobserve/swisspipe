import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { Workflow, CreateWorkflowRequest, WorkflowFilters } from '../types/workflow'
import type { ApiError } from '../types/api'
import { apiClient } from '../services/api'

export const useWorkflowStore = defineStore('workflows', () => {
  // State
  const workflows = ref<Workflow[]>([])
  const currentWorkflow = ref<Workflow | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)
  const filters = ref<WorkflowFilters>({
    search: '',
    status: []
  })
  const searchTerm = ref('')

  // Getters
  const filteredWorkflows = computed(() => {
    let result = workflows.value

    if (searchTerm.value) {
      const search = searchTerm.value.toLowerCase()
      result = result.filter(workflow => 
        workflow.name.toLowerCase().includes(search) ||
        workflow.description?.toLowerCase().includes(search)
      )
    }

    return result
  })

  const workflowCount = computed(() => workflows.value.length)

  // Actions
  async function fetchWorkflows() {
    loading.value = true
    error.value = null
    try {
      const response = await apiClient.getWorkflows()
      workflows.value = response.workflows
    } catch (err) {
      const apiError = err as ApiError
      error.value = apiError.message
      console.error('Failed to fetch workflows:', apiError)
    } finally {
      loading.value = false
    }
  }

  async function fetchWorkflow(id: string) {
    loading.value = true
    error.value = null
    try {
      currentWorkflow.value = await apiClient.getWorkflow(id)
    } catch (err) {
      const apiError = err as ApiError
      error.value = apiError.message
      console.error('Failed to fetch workflow:', apiError)
    } finally {
      loading.value = false
    }
  }

  async function createWorkflow(workflow: CreateWorkflowRequest) {
    loading.value = true
    error.value = null
    try {
      const newWorkflow = await apiClient.createWorkflow(workflow)
      workflows.value.push(newWorkflow)
      return newWorkflow
    } catch (err) {
      const apiError = err as ApiError
      error.value = apiError.message
      console.error('Failed to create workflow:', apiError)
      throw err
    } finally {
      loading.value = false
    }
  }

  async function updateWorkflow(id: string, workflow: CreateWorkflowRequest) {
    loading.value = true
    error.value = null
    try {
      const updatedWorkflow = await apiClient.updateWorkflow(id, workflow)
      const index = workflows.value.findIndex(w => w.id === id)
      if (index !== -1) {
        workflows.value[index] = updatedWorkflow
      }
      if (currentWorkflow.value?.id === id) {
        currentWorkflow.value = updatedWorkflow
      }
      return updatedWorkflow
    } catch (err) {
      const apiError = err as ApiError
      error.value = apiError.message
      console.error('Failed to update workflow:', apiError)
      throw err
    } finally {
      loading.value = false
    }
  }

  async function deleteWorkflow(id: string) {
    loading.value = true
    error.value = null
    try {
      await apiClient.deleteWorkflow(id)
      workflows.value = workflows.value.filter(w => w.id !== id)
      if (currentWorkflow.value?.id === id) {
        currentWorkflow.value = null
      }
    } catch (err) {
      const apiError = err as ApiError
      error.value = apiError.message
      console.error('Failed to delete workflow:', apiError)
      throw err
    } finally {
      loading.value = false
    }
  }

  function clearError() {
    error.value = null
  }

  function setCurrentWorkflow(workflow: Workflow | null) {
    currentWorkflow.value = workflow
  }

  function updateSearchTerm(term: string) {
    searchTerm.value = term
  }

  return {
    // State
    workflows,
    currentWorkflow,
    loading,
    error,
    filters,
    searchTerm,
    // Getters
    filteredWorkflows,
    workflowCount,
    // Actions
    fetchWorkflows,
    fetchWorkflow,
    createWorkflow,
    updateWorkflow,
    deleteWorkflow,
    clearError,
    setCurrentWorkflow,
    updateSearchTerm
  }
})