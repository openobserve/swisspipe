import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { apiClient } from '../services/api'
import type { Variable, CreateVariableRequest, UpdateVariableRequest } from '../types/variable'

export const useVariableStore = defineStore('variables', () => {
  // State
  const variables = ref<Variable[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)
  const searchQuery = ref('')

  // Computed
  const filteredVariables = computed(() => {
    if (!searchQuery.value) {
      return variables.value
    }
    const query = searchQuery.value.toLowerCase()
    return variables.value.filter(
      (v) =>
        v.name.toLowerCase().includes(query) ||
        v.description?.toLowerCase().includes(query)
    )
  })

  const variableCount = computed(() => variables.value.length)

  const secretCount = computed(
    () => variables.value.filter((v) => v.value_type === 'secret').length
  )

  const textCount = computed(
    () => variables.value.filter((v) => v.value_type === 'text').length
  )

  // Actions
  async function fetchVariables() {
    loading.value = true
    error.value = null
    try {
      const response = await apiClient.getVariables()
      variables.value = response.variables
    } catch (err: unknown) {
      error.value = (err as Error).message || 'Failed to fetch variables'
      console.error('Error fetching variables:', err)
    } finally {
      loading.value = false
    }
  }

  async function createVariable(data: CreateVariableRequest): Promise<Variable> {
    loading.value = true
    error.value = null
    try {
      const variable = await apiClient.createVariable(data)
      variables.value.push(variable)
      return variable
    } catch (err: unknown) {
      error.value = (err as Error).message || 'Failed to create variable'
      console.error('Error creating variable:', err)
      throw err
    } finally {
      loading.value = false
    }
  }

  async function updateVariable(
    id: string,
    data: UpdateVariableRequest
  ): Promise<Variable> {
    loading.value = true
    error.value = null
    try {
      const variable = await apiClient.updateVariable(id, data)
      const index = variables.value.findIndex((v) => v.id === id)
      if (index !== -1) {
        variables.value[index] = variable
      }
      return variable
    } catch (err: unknown) {
      error.value = (err as Error).message || 'Failed to update variable'
      console.error('Error updating variable:', err)
      throw err
    } finally {
      loading.value = false
    }
  }

  async function deleteVariable(id: string): Promise<void> {
    loading.value = true
    error.value = null
    try {
      await apiClient.deleteVariable(id)
      variables.value = variables.value.filter((v) => v.id !== id)
    } catch (err: unknown) {
      error.value = (err as Error).message || 'Failed to delete variable'
      console.error('Error deleting variable:', err)
      throw err
    } finally {
      loading.value = false
    }
  }

  async function validateName(name: string): Promise<boolean> {
    try {
      const response = await apiClient.validateVariableName(name)
      return response.valid
    } catch (err: unknown) {
      console.error('Error validating variable name:', err)
      return false
    }
  }

  function clearError() {
    error.value = null
  }

  return {
    // State
    variables,
    loading,
    error,
    searchQuery,
    // Computed
    filteredVariables,
    variableCount,
    secretCount,
    textCount,
    // Actions
    fetchVariables,
    createVariable,
    updateVariable,
    deleteVariable,
    validateName,
    clearError
  }
})
