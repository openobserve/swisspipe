<template>
  <div class="node-app px-4 py-3 rounded-lg shadow-2xl min-w-[180px] border-2" :class="nodeClasses">
    <div class="flex items-center justify-between mb-2">
      <div class="flex-1">
        <div class="text-sm font-medium">{{ data.label || 'App' }}</div>
        <div class="text-xs text-green-200 opacity-80">{{ getAppType() }}</div>
      </div>
      <div v-if="data.isTracing" class="flex items-center space-x-1">
        <div v-if="data.executionStatus" :class="statusIndicatorClasses" class="w-3 h-3 rounded-full"></div>
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
    />
    <Handle
      type="source"
      :position="Position.Bottom"
    />
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { Handle, Position } from '@vue-flow/core'

interface Props {
  data: {
    label: string
    description?: string
    status?: string
    config: any
    isTracing?: boolean
    executionStatus?: string
    executionDuration?: number
    executionError?: string
  }
}

const props = defineProps<Props>()

const nodeClasses = computed(() => {
  // Default classes
  const baseClasses = 'border-green-400/30'
  
  if (!props.data.isTracing || !props.data.executionStatus) {
    return baseClasses
  }
  
  switch (props.data.executionStatus) {
    case 'completed':
      return 'border-green-400 bg-green-900/20'
    case 'failed':
      return 'border-red-400 bg-red-900/20'
    case 'running':
      return 'border-blue-400 bg-blue-900/20 animate-pulse'
    case 'pending':
      return 'border-yellow-400 bg-yellow-900/20'
    case 'skipped':
      return 'border-gray-400 bg-gray-900/20'
    default:
      return baseClasses
  }
})

const statusIndicatorClasses = computed(() => {
  if (!props.data.executionStatus) return ''
  
  switch (props.data.executionStatus) {
    case 'completed': return 'bg-green-400'
    case 'failed': return 'bg-red-400'
    case 'running': return 'bg-blue-400 animate-pulse'
    case 'pending': return 'bg-yellow-400'
    case 'skipped': return 'bg-gray-400'
    default: return 'bg-gray-400'
  }
})

function getAppType() {
  if (props.data.config?.app_type) {
    return props.data.config.app_type
  }
  if (props.data.config?.type) {
    return props.data.config.type
  }
  return 'App'
}

function formatDuration(durationMs: number | null): string {
  if (!durationMs) return 'N/A'
  
  if (durationMs < 1000) return `${durationMs}ms`
  if (durationMs < 60000) return `${(durationMs / 1000).toFixed(1)}s`
  return `${(durationMs / 60000).toFixed(1)}m`
}
</script>

<style scoped>
.node-app {
  background: rgba(34, 197, 94, 0.12);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border: 1px solid rgba(34, 197, 94, 0.25);
  box-shadow: 
    0 8px 32px rgba(34, 197, 94, 0.15),
    inset 0 1px 0 rgba(255, 255, 255, 0.1);
  transition: all 0.3s ease;
}

.node-app:hover {
  background: rgba(34, 197, 94, 0.18);
  box-shadow: 
    0 12px 40px rgba(34, 197, 94, 0.25),
    inset 0 1px 0 rgba(255, 255, 255, 0.15);
  transform: translateY(-1px);
}
</style>