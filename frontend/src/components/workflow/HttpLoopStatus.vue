<template>
  <div class="bg-slate-800 rounded-lg p-4 border border-slate-700">
    <div class="flex items-center justify-between mb-3">
      <h3 class="text-sm font-medium text-gray-200">HTTP Loop Status</h3>
      <span
        :class="statusBadgeClass"
        class="px-2 py-1 rounded text-xs font-medium"
      >
        {{ status.status }}
      </span>
    </div>

    <div class="space-y-3">
      <!-- Progress Bar -->
      <div v-if="status.max_iterations" class="space-y-2">
        <div class="flex justify-between text-sm">
          <span class="text-gray-400">Progress</span>
          <span class="text-gray-200">
            {{ status.current_iteration }} / {{ status.max_iterations }}
          </span>
        </div>
        <div class="w-full bg-slate-700 rounded-full h-2">
          <div
            class="bg-primary-600 h-2 rounded-full transition-all duration-300"
            :style="{ width: `${progressPercentage}%` }"
          ></div>
        </div>
        <div class="text-right">
          <span class="text-xs text-gray-400">{{ progressPercentage }}%</span>
        </div>
      </div>

      <!-- Infinite Loop Progress -->
      <div v-else class="space-y-2">
        <div class="flex justify-between text-sm">
          <span class="text-gray-400">Iterations</span>
          <span class="text-gray-200">{{ status.current_iteration }}</span>
        </div>
        <div class="w-full bg-slate-700 rounded-full h-2">
          <div class="bg-primary-600 h-2 rounded-full animate-pulse"></div>
        </div>
        <div class="text-right">
          <span class="text-xs text-gray-400">∞ (infinite loop)</span>
        </div>
      </div>

      <!-- Status Details -->
      <div class="grid grid-cols-2 gap-4 text-sm">
        <div class="space-y-2">
          <div class="flex justify-between">
            <span class="text-gray-400">Next Execution:</span>
            <span class="text-gray-200">{{ formatNextExecution() }}</span>
          </div>

          <div class="flex justify-between">
            <span class="text-gray-400">Started:</span>
            <span class="text-gray-200">{{ formatStartTime() }}</span>
          </div>

          <div class="flex justify-between">
            <span class="text-gray-400">Duration:</span>
            <span class="text-gray-200">{{ formatDuration() }}</span>
          </div>
        </div>

        <div class="space-y-2">
          <div class="flex justify-between">
            <span class="text-gray-400">Failures:</span>
            <span :class="status.consecutive_failures > 0 ? 'text-red-400' : 'text-gray-200'">
              {{ status.consecutive_failures }}
            </span>
          </div>

          <div v-if="status.last_response_status" class="flex justify-between">
            <span class="text-gray-400">Last Status:</span>
            <span
              :class="getStatusColor(status.last_response_status)"
              class="font-medium"
            >
              {{ status.last_response_status }}
            </span>
          </div>

          <div v-if="status.termination_reason" class="flex justify-between">
            <span class="text-gray-400">Reason:</span>
            <span class="text-gray-200">{{ status.termination_reason }}</span>
          </div>
        </div>
      </div>

      <!-- Success Rate (if completed) -->
      <div v-if="status.status === 'completed'" class="bg-slate-700 rounded p-3">
        <div class="flex justify-between items-center">
          <span class="text-gray-400 text-sm">Success Rate</span>
          <span class="text-lg font-bold" :class="getSuccessRateColor()">
            {{ Math.round(successRate * 100) }}%
          </span>
        </div>
        <div class="w-full bg-slate-600 rounded-full h-2 mt-2">
          <div
            class="h-2 rounded-full transition-all duration-300"
            :class="getSuccessRateBarColor()"
            :style="{ width: `${successRate * 100}%` }"
          ></div>
        </div>
      </div>

      <!-- Action Buttons -->
      <div v-if="status.status === 'running'" class="flex gap-2 pt-2 border-t border-slate-700">
        <button
          @click="emit('pause-loop', props.status.loop_id)"
          class="flex-1 px-3 py-2 bg-yellow-600 hover:bg-yellow-700 text-white text-xs rounded-md transition-colors"
        >
          Pause Loop
        </button>
        <button
          @click="emit('stop-loop', props.status.loop_id)"
          class="flex-1 px-3 py-2 bg-red-600 hover:bg-red-700 text-white text-xs rounded-md transition-colors"
        >
          Stop Loop
        </button>
      </div>

      <div v-else-if="status.status === 'failed'" class="flex gap-2 pt-2 border-t border-slate-700">
        <button
          @click="emit('retry-loop', props.status.loop_id)"
          class="flex-1 px-3 py-2 bg-primary-600 hover:bg-primary-700 text-white text-xs rounded-md transition-colors"
        >
          Retry Loop
        </button>
      </div>

      <!-- Iteration History (collapsible) -->
      <div v-if="false" class="pt-2 border-t border-slate-700">
        <button
          @click="showHistory = !showHistory"
          class="flex items-center justify-between w-full text-sm text-gray-400 hover:text-gray-200 transition-colors"
        >
          <span>Iteration History ({{ parsedHistory.length }})</span>
          <span class="transform transition-transform" :class="showHistory ? 'rotate-180' : ''">
            ▼
          </span>
        </button>

        <div v-if="showHistory" class="mt-3 max-h-40 overflow-y-auto space-y-2">
          <div
            v-for="(iteration, index) in parsedHistory.slice(-10)"
            :key="index"
            class="bg-slate-700 rounded p-2 text-xs"
          >
            <div class="flex justify-between items-center">
              <span class="text-gray-300">#{{ iteration.iteration }}</span>
              <div class="flex items-center gap-2">
                <span
                  :class="getStatusColor(iteration.status_code)"
                  class="font-medium"
                >
                  {{ iteration.status_code }}
                </span>
                <span class="text-gray-400">{{ formatIterationTime(iteration.timestamp) }}</span>
              </div>
            </div>
            <div v-if="iteration.duration_ms" class="text-gray-400 mt-1">
              Duration: {{ iteration.duration_ms }}ms
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import type { LoopStatus } from '../../types/nodes'

interface Props {
  status: LoopStatus
}

interface Emits {
  (e: 'pause-loop', loopId: string): void
  (e: 'stop-loop', loopId: string): void
  (e: 'retry-loop', loopId: string): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

const showHistory = ref(false)

const statusBadgeClass = computed(() => {
  switch (props.status.status) {
    case 'running':
      return 'bg-blue-600 text-white animate-pulse'
    case 'completed':
      return 'bg-green-600 text-white'
    case 'failed':
      return 'bg-red-600 text-white'
    default:
      return 'bg-gray-600 text-white'
  }
})

const progressPercentage = computed(() => {
  if (!props.status.max_iterations) return 0
  return Math.min(100, Math.round((props.status.current_iteration / props.status.max_iterations) * 100))
})

const successRate = computed(() => {
  return props.status.success_rate || 0
})

// Since iteration_history is no longer provided by backend, return empty array with proper typing
interface IterationHistory {
  iteration: number
  status_code: number
  timestamp: number
  duration_ms?: number
}

const parsedHistory = computed((): IterationHistory[] => {
  return []
})

function formatNextExecution(): string {
  if (!props.status.next_execution_at) return 'N/A'

  const nextTime = new Date(props.status.next_execution_at / 1000) // Convert from microseconds
  const now = new Date()

  if (nextTime < now) return 'Overdue'

  const diffMs = nextTime.getTime() - now.getTime()
  const diffMinutes = Math.round(diffMs / (1000 * 60))

  if (diffMinutes < 60) return `in ${diffMinutes}m`

  const diffHours = Math.round(diffMinutes / 60)
  if (diffHours < 24) return `in ${diffHours}h`

  const diffDays = Math.round(diffHours / 24)
  return `in ${diffDays}d`
}

function formatStartTime(): string {
  const startTime = new Date(props.status.loop_started_at / 1000)
  return startTime.toLocaleDateString('en-US', {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit'
  })
}

function formatDuration(): string {
  const startTime = new Date(props.status.loop_started_at / 1000)
  const now = new Date()
  const diffMs = now.getTime() - startTime.getTime()

  const hours = Math.floor(diffMs / (1000 * 60 * 60))
  const minutes = Math.floor((diffMs % (1000 * 60 * 60)) / (1000 * 60))

  if (hours > 0) {
    return `${hours}h ${minutes}m`
  }
  return `${minutes}m`
}

function formatIterationTime(timestamp: number): string {
  const time = new Date(timestamp / 1000)
  return time.toLocaleTimeString('en-US', {
    hour12: false,
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit'
  })
}

function getStatusColor(statusCode: number): string {
  if (statusCode >= 200 && statusCode < 300) return 'text-green-400'
  if (statusCode >= 300 && statusCode < 400) return 'text-yellow-400'
  if (statusCode >= 400 && statusCode < 500) return 'text-orange-400'
  if (statusCode >= 500) return 'text-red-400'
  return 'text-gray-400'
}

function getSuccessRateColor(): string {
  const rate = successRate.value
  if (rate >= 0.9) return 'text-green-400'
  if (rate >= 0.7) return 'text-yellow-400'
  if (rate >= 0.5) return 'text-orange-400'
  return 'text-red-400'
}

function getSuccessRateBarColor(): string {
  const rate = successRate.value
  if (rate >= 0.9) return 'bg-green-600'
  if (rate >= 0.7) return 'bg-yellow-600'
  if (rate >= 0.5) return 'bg-orange-600'
  return 'bg-red-600'
}
</script>