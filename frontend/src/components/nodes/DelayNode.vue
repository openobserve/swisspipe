<template>
  <div class="node-delay px-4 py-3 rounded-lg shadow-2xl min-w-[180px] border-2" :class="nodeClasses">
    <div class="flex items-center justify-between mb-2">
      <div class="flex-1">
        <div class="text-sm font-medium">{{ data.label || 'Delay' }}</div>
        <div class="text-xs text-gray-300 opacity-80">{{ getDelaySummary() }}</div>
      </div>
      <div class="flex items-center space-x-1">
        <!-- Execution status indicator -->
        <div v-if="data.isTracing && data.executionStatus" :class="statusIndicatorClasses" class="w-3 h-3 rounded-full"></div>
        <!-- Clock icon -->
        <div class="text-gray-400">
          <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
            <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm1-12a1 1 0 10-2 0v4a1 1 0 00.293.707l2.828 2.829a1 1 0 101.415-1.415L11 9.586V6z" clip-rule="evenodd" />
          </svg>
        </div>
      </div>
    </div>
    
    <div v-if="data.isTracing && data.executionStatus" class="text-xs text-gray-400 mt-1">
      <div>Status: {{ data.executionStatus }}</div>
      <div v-if="data.executionDuration">Duration: {{ formatDuration(data.executionDuration) }}</div>
    </div>
    
    <!-- Connection handles -->
    <Handle
      type="target"
      :position="Position.Top"
      :style="{ background: '#ddd' }"
    />
    <Handle
      type="source"
      :position="Position.Bottom"
      :style="{ background: '#ddd' }"
    />
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { Handle, Position } from '@vue-flow/core'
import type { DelayConfig } from '../../types/nodes'

interface Props {
  data: {
    label: string
    description?: string
    status?: string
    config: DelayConfig
    isTracing?: boolean
    executionStatus?: string
    executionDuration?: number
    executionError?: string
  }
}

const props = defineProps<Props>()

const nodeClasses = computed(() => [
  'bg-gray-800',
  'border-gray-600',
  'text-white',
  {
    'ring-2 ring-blue-400': props.data.isTracing,
    'border-red-500': props.data.executionStatus === 'failed',
    'border-green-500': props.data.executionStatus === 'completed',
    'border-yellow-500': props.data.executionStatus === 'running'
  }
])

const statusIndicatorClasses = computed(() => ({
  'bg-green-500': props.data.executionStatus === 'completed',
  'bg-red-500': props.data.executionStatus === 'failed',
  'bg-yellow-500 animate-pulse': props.data.executionStatus === 'running',
  'bg-gray-500': props.data.executionStatus === 'pending'
}))

function getDelaySummary(): string {
  const config = props.data.config
  if (!config.duration || !config.unit) {
    return 'Not configured'
  }
  
  const unitDisplay = config.unit.toLowerCase()
  return `${config.duration} ${unitDisplay}`
}

function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`
  return `${(ms / 60000).toFixed(1)}m`
}
</script>

<style scoped>
.node-delay {
  font-family: 'Inter', sans-serif;
}
</style>