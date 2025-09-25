import { ref, computed, onUnmounted } from 'vue'
import { apiClient } from '../services/api'
import type { LoopStatus } from '../types/nodes'

export function useHttpLoop(executionId?: string) {
  const loopStatuses = ref<Map<string, LoopStatus>>(new Map())
  const loading = ref(false)
  const error = ref<string | null>(null)

  // Store the execution ID for filtering
  const currentExecutionId = ref<string | undefined>(executionId)

  // Polling interval for real-time updates
  let pollingInterval: number | null = null

  const activeLoops = computed(() => {
    return Array.from(loopStatuses.value.values()).filter(
      status => status.status === 'running'
    )
  })

  const getLoopStatus = (loopId: string): LoopStatus | undefined => {
    return loopStatuses.value.get(loopId)
  }

  const isLoopActive = (loopId: string): boolean => {
    const status = loopStatuses.value.get(loopId)
    return status?.status === 'running'
  }

  const fetchLoopStatus = async (loopId: string): Promise<void> => {
    try {
      loading.value = true
      error.value = null

      const status = await apiClient.getLoopStatus(loopId)
      loopStatuses.value.set(loopId, status)
    } catch (err) {
      error.value = err instanceof Error ? err.message : 'Failed to fetch loop status'
      console.error('Failed to fetch loop status:', err, { loopId })
    } finally {
      loading.value = false
    }
  }

  const fetchActiveLoops = async (): Promise<void> => {
    try {
      loading.value = true
      error.value = null

      const response = await apiClient.getActiveLoops(currentExecutionId.value)
      console.log('Active loops response:', response, 'Type:', typeof response, 'IsArray:', Array.isArray(response)) // Enhanced debug log

      // Validate and normalize the response
      let activeLoopsList: LoopStatus[] = []

      if (Array.isArray(response)) {
        activeLoopsList = response
      } else if (response && typeof response === 'object') {
        // Try different possible nested array properties
        const possibleArrayProps = ['loops', 'data', 'active_loops', 'results']

        for (const prop of possibleArrayProps) {
          if (Array.isArray((response as Record<string, unknown>)[prop])) {
            activeLoopsList = (response as Record<string, unknown>)[prop] as LoopStatus[]
            console.log(`Using response.${prop} as array data`)
            break
          }
        }

        if (activeLoopsList.length === 0) {
          // If still no array found, check if response itself might be a single item to wrap in array
          if (response && 'loop_id' in response && 'status' in response) {
            activeLoopsList = [response as LoopStatus]
            console.log('Treating single response object as array')
          } else {
            console.warn('Unexpected API response format for active loops:', response)
            activeLoopsList = []
          }
        }
      } else if (response === null || response === undefined) {
        console.log('API returned null/undefined, using empty array')
        activeLoopsList = []
      } else {
        console.warn('Unexpected API response type for active loops:', typeof response, response)
        activeLoopsList = []
      }

      console.log('Final activeLoopsList:', activeLoopsList, 'Length:', activeLoopsList.length)

      // Ensure activeLoopsList is indeed an array before calling forEach
      if (!Array.isArray(activeLoopsList)) {
        console.error('activeLoopsList is not an array after normalization:', typeof activeLoopsList, activeLoopsList)
        activeLoopsList = []
      }

      // Update the map with active loops
      const newMap = new Map<string, LoopStatus>()
      activeLoopsList.forEach(status => {
        if (status && status.loop_id) {
          newMap.set(status.loop_id, status)
        } else {
          console.warn('Invalid loop status object:', status)
        }
      })

      // Keep completed/failed loops that were previously active
      loopStatuses.value.forEach((status, id) => {
        if (status.status === 'completed' || status.status === 'failed') {
          if (!newMap.has(id)) {
            newMap.set(id, status)
          }
        }
      })

      loopStatuses.value = newMap
    } catch (err) {
      error.value = err instanceof Error ? err.message : 'Failed to fetch active loops'
      console.error('Failed to fetch active loops:', err)
    } finally {
      loading.value = false
    }
  }

  const pauseLoop = async (loopId: string): Promise<void> => {
    try {
      await apiClient.pauseLoop(loopId)
      // Refresh the specific loop status after action
      await fetchLoopStatus(loopId)
    } catch (err) {
      error.value = err instanceof Error ? err.message : 'Failed to pause loop'
      console.error('Failed to pause loop:', err, { loopId })
      throw err
    }
  }

  const resumeLoop = async (loopId: string): Promise<void> => {
    try {
      await apiClient.resumeLoop(loopId)
      // Refresh the specific loop status after action
      await fetchLoopStatus(loopId)
    } catch (err) {
      error.value = err instanceof Error ? err.message : 'Failed to resume loop'
      console.error('Failed to resume loop:', err, { loopId })
      throw err
    }
  }

  const stopLoop = async (loopId: string): Promise<void> => {
    try {
      await apiClient.stopLoop(loopId)
      // Refresh the specific loop status after action
      await fetchLoopStatus(loopId)
    } catch (err) {
      error.value = err instanceof Error ? err.message : 'Failed to stop loop'
      console.error('Failed to stop loop:', err, { loopId })
      throw err
    }
  }

  const retryLoop = async (loopId: string): Promise<void> => {
    try {
      await apiClient.retryLoop(loopId)
      // Refresh the specific loop status after action
      await fetchLoopStatus(loopId)
    } catch (err) {
      error.value = err instanceof Error ? err.message : 'Failed to retry loop'
      console.error('Failed to retry loop:', err, { loopId })
      throw err
    }
  }

  const startPolling = (intervalMs: number = 10000): void => {
    if (pollingInterval !== null) {
      clearInterval(pollingInterval)
    }

    // Start polling with initial fetch
    pollingInterval = window.setInterval(async () => {
      await fetchActiveLoops()
    }, intervalMs)

    // Also fetch immediately
    fetchActiveLoops()
  }

  const stopPolling = (): void => {
    if (pollingInterval !== null) {
      clearInterval(pollingInterval)
      pollingInterval = null
    }
  }

  // Watch for specific loop status updates - only when explicitly called
  const watchLoopStatus = (loopId: string, intervalMs: number = 2000): (() => void) => {
    let watchInterval: number | null = null

    const startWatching = () => {
      // Set up polling for this specific loop - no initial fetch
      watchInterval = window.setInterval(async () => {
        await fetchLoopStatus(loopId)
      }, intervalMs)
    }

    startWatching()

    // Return cleanup function
    return () => {
      if (watchInterval !== null) {
        clearInterval(watchInterval)
        watchInterval = null
      }
    }
  }

  // Manual refresh function for on-demand updates
  const refreshLoopData = async (): Promise<void> => {
    await fetchActiveLoops()
  }

  // Manual refresh for specific loop
  const refreshLoopStatus = async (loopId: string): Promise<void> => {
    await fetchLoopStatus(loopId)
  }

  // Update execution ID for filtering
  const setExecutionId = (executionId?: string): void => {
    currentExecutionId.value = executionId
  }

  // Cleanup on unmount
  onUnmounted(() => {
    stopPolling()
  })

  return {
    // State
    loopStatuses: computed(() => loopStatuses.value),
    loading: computed(() => loading.value),
    error: computed(() => error.value),
    activeLoops,

    // Getters
    getLoopStatus,
    isLoopActive,

    // Actions
    fetchLoopStatus,
    fetchActiveLoops,
    pauseLoop,
    resumeLoop,
    stopLoop,
    retryLoop,

    // Manual refresh functions
    refreshLoopData,
    refreshLoopStatus,

    // Execution filtering
    setExecutionId,

    // Polling (for explicit use only)
    startPolling,
    stopPolling,
    watchLoopStatus
  }
}