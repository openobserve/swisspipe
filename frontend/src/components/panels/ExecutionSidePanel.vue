<template>
  <div class="h-full flex flex-col">
    <!-- Header -->
    <div class="px-4 py-3 border-b border-slate-700/50 flex items-center justify-between">
      <h2 class="text-lg font-semibold text-gray-200">Executions</h2>
      <div class="flex items-center space-x-2">
        <button
          @click="refreshExecutions"
          :disabled="loading"
          class="text-gray-400 hover:text-gray-200 transition-colors p-1 rounded"
          :class="{ 'animate-spin': refreshing }"
          title="Refresh executions"
        >
          <ArrowPathIcon class="h-5 w-5" />
        </button>
        <button
          @click="$emit('close')"
          class="text-gray-400 hover:text-gray-200 transition-colors"
        >
          <XMarkIcon class="h-6 w-6" />
        </button>
      </div>
    </div>

    <!-- Filter Controls -->
    <div class="px-4 py-2 border-b border-slate-700/50 bg-slate-800/30">
      <div class="flex items-center space-x-3">
        <label class="text-sm font-medium text-gray-300">Status:</label>
        <select
          v-model="statusFilter"
          @change="onStatusFilterChange"
          class="bg-slate-700 border border-slate-600 text-gray-200 text-sm rounded-md px-2 py-1 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
        >
          <option value="">All Statuses</option>
          <option value="pending">Pending</option>
          <option value="running">Running</option>
          <option value="completed">Completed</option>
          <option value="failed">Failed</option>
          <option value="cancelled">Cancelled</option>
        </select>
        <button
          v-if="statusFilter"
          @click="clearFilter"
          class="text-xs text-gray-400 hover:text-gray-300 underline"
        >
          Clear
        </button>
      </div>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-hidden flex flex-col">
      <!-- Loading State -->
      <div v-if="loading" class="p-4 text-center text-gray-400">
        <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600 mx-auto mb-2"></div>
        <p>Loading executions...</p>
      </div>

      <!-- Error State -->
      <div v-else-if="error" class="p-4 text-center text-red-400">
        <p>{{ error }}</p>
        <button
          @click="fetchExecutions"
          class="mt-2 text-blue-400 hover:text-blue-300 underline"
        >
          Retry
        </button>
      </div>

      <!-- Empty State -->
      <div v-else-if="executions.length === 0" class="p-4 text-center text-gray-400">
        <p>No executions found for this workflow.</p>
      </div>

      <!-- Executions List -->
      <div v-else class="flex-1 overflow-y-auto">
        <div class="divide-y divide-slate-700/50">
          <div
            v-for="execution in executions"
            :key="execution.id"
            @click="selectExecution(execution)"
            class="p-4 cursor-pointer hover:bg-slate-700/30 transition-colors border-l-4 border-transparent hover:border-blue-500"
            :class="{
              'bg-slate-700/50 border-blue-500': selectedExecutionId === execution.id
            }"
          >
            <!-- Execution Header -->
            <div class="flex items-center justify-between mb-2">
              <div class="flex items-center space-x-2">
                <span class="font-mono text-sm text-gray-300">
                  {{ execution.id.substring(0, 8) }}
                </span>
                <StatusBadge :status="execution.status" />
              </div>
              <span class="text-xs text-gray-400">
                {{ formatRelativeTime(execution.started_at) }}
              </span>
            </div>

            <!-- Execution Details -->
            <div class="text-sm text-gray-400">
              <div class="flex justify-between">
                <span>Duration:</span>
                <span>{{ formatDuration(execution.duration_ms) }}</span>
              </div>
              <div v-if="execution.error_message" class="mt-1 text-red-400 truncate">
                {{ execution.error_message }}
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Pagination -->
      <div v-if="totalPages > 1" class="px-4 py-3 border-t border-slate-700/50">
        <div class="flex items-center justify-between text-sm">
          <button
            @click="previousPage"
            :disabled="currentPage === 1"
            class="px-3 py-1 bg-slate-600 hover:bg-slate-500 disabled:bg-slate-800 disabled:text-gray-500 text-white rounded transition-colors"
          >
            Previous
          </button>
          <span class="text-gray-400">
            Page {{ currentPage }} of {{ totalPages }}
          </span>
          <button
            @click="nextPage"
            :disabled="currentPage === totalPages"
            class="px-3 py-1 bg-slate-600 hover:bg-slate-500 disabled:bg-slate-800 disabled:text-gray-500 text-white rounded transition-colors"
          >
            Next
          </button>
        </div>
      </div>
    </div>

    <!-- Auto-refresh indicator -->
    <div v-if="autoRefresh" class="px-4 py-2 border-t border-slate-700/50 text-xs text-gray-500 flex items-center space-x-2">
      <div class="w-2 h-2 bg-green-500 rounded-full animate-pulse"></div>
      <span>Auto-refreshing...</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { XMarkIcon, ArrowPathIcon } from '@heroicons/vue/24/outline'
import StatusBadge from '../common/StatusBadge.vue'
import { apiClient } from '../../services/api'

interface Execution {
  id: string
  workflow_id: string
  status: string
  started_at: number | null
  completed_at: number | null
  duration_ms: number | null
  error_message: string | null
}

interface Props {
  workflowId: string
}

const props = defineProps<Props>()
const emit = defineEmits(['close', 'trace-execution'])

// State
const executions = ref<Execution[]>([])
const loading = ref(false)
const error = ref<string | null>(null)
const selectedExecutionId = ref<string | null>(null)
const currentPage = ref(1)
const totalExecutions = ref(0)
const pageSize = 20
const autoRefresh = ref(true)
const statusFilter = ref<string>('')
const refreshing = ref(false)

// Computed
const totalPages = computed(() => Math.ceil(totalExecutions.value / pageSize))

// Auto-refresh interval
let refreshInterval: NodeJS.Timeout | null = null

onMounted(() => {
  fetchExecutions()
  if (autoRefresh.value) {
    startAutoRefresh()
  }
})

onUnmounted(() => {
  stopAutoRefresh()
})

async function fetchExecutions() {
  if (!props.workflowId) return
  
  loading.value = true
  error.value = null
  
  try {
    const offset = (currentPage.value - 1) * pageSize
    const status = statusFilter.value || undefined
    const data = await apiClient.getExecutionsByWorkflow(props.workflowId, pageSize, offset, status)
    executions.value = data.executions || []
    totalExecutions.value = data.count || 0
  } catch (err: any) {
    error.value = err.message || 'Unknown error occurred'
    console.error('Failed to fetch executions:', err)
  } finally {
    loading.value = false
  }
}

function selectExecution(execution: Execution) {
  selectedExecutionId.value = execution.id
  emit('trace-execution', execution)
}

function previousPage() {
  if (currentPage.value > 1) {
    currentPage.value--
    fetchExecutions()
  }
}

function nextPage() {
  if (currentPage.value < totalPages.value) {
    currentPage.value++
    fetchExecutions()
  }
}

function startAutoRefresh() {
  refreshInterval = setInterval(() => {
    fetchExecutions()
  }, 30000) // Refresh every 30 seconds
}

function stopAutoRefresh() {
  if (refreshInterval) {
    clearInterval(refreshInterval)
    refreshInterval = null
  }
}

function formatRelativeTime(timestamp: number | null): string {
  if (!timestamp) return 'N/A'
  
  const now = Date.now() * 1000 // Convert to microseconds
  const diff = now - timestamp
  const seconds = Math.floor(diff / 1000000)
  const minutes = Math.floor(seconds / 60)
  const hours = Math.floor(minutes / 60)
  const days = Math.floor(hours / 24)
  
  if (days > 0) return `${days}d ago`
  if (hours > 0) return `${hours}h ago`
  if (minutes > 0) return `${minutes}m ago`
  return `${seconds}s ago`
}

function formatDuration(durationMs: number | null): string {
  if (!durationMs) return 'N/A'
  
  if (durationMs < 1000) return `${durationMs}ms`
  if (durationMs < 60000) return `${(durationMs / 1000).toFixed(1)}s`
  return `${(durationMs / 60000).toFixed(1)}m`
}

function onStatusFilterChange() {
  currentPage.value = 1 // Reset to first page when filter changes
  fetchExecutions()
}

function clearFilter() {
  statusFilter.value = ''
  currentPage.value = 1
  fetchExecutions()
}

async function refreshExecutions() {
  refreshing.value = true
  try {
    await fetchExecutions()
  } finally {
    // Keep spinning animation for at least 500ms for visual feedback
    setTimeout(() => {
      refreshing.value = false
    }, 500)
  }
}
</script>