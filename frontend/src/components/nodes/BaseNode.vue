<template>
  <div 
    :class="[`node-${nodeType}`, 'px-4 py-3 rounded-lg shadow-2xl min-w-[180px] border-2', nodeClasses]"
    :style="nodeStyles"
  >
    <div class="flex items-center justify-between mb-1">
      <div class="flex-1">
        <div class="text-sm font-medium">{{ data.label || defaultLabel }}</div>
        <div v-if="subtitle" class="text-xs opacity-80" :class="subtitleColorClass">{{ subtitle }}</div>
      </div>
      <div v-if="data.isTracing" class="flex items-center space-x-1">
        <div v-if="data.executionStatus" :class="statusIndicatorClasses" class="w-3 h-3 rounded-full"></div>
      </div>
    </div>
    
    <!-- Execution info when tracing -->
    <div v-if="data.isTracing && data.executionStatus" class="text-xs text-gray-400 mt-1">
      <div>Status: {{ data.executionStatus }}</div>
      <div v-if="data.executionDuration">Duration: {{ formatDuration(data.executionDuration) }}</div>
    </div>
    
    <!-- Custom content slot -->
    <slot name="content" />
    
    <!-- Connection handles -->
    <slot name="handles">
      <Handle
        v-for="handle in handles"
        :key="`${handle.type}-${handle.position}`"
        :type="handle.type"
        :position="handle.position"
        :id="handle.id"
      />
    </slot>
  </div>
</template>

<script setup lang="ts">
import { Handle, Position } from '@vue-flow/core'
import { computed } from 'vue'
import { formatDuration } from '../../utils/formatting'
import { getNodeTheme, type NodeType } from '../../constants/nodeThemes'

interface NodeHandle {
  type: 'source' | 'target'
  position: typeof Position[keyof typeof Position]
  id?: string
}

interface Props {
  nodeType: NodeType
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
  handles?: NodeHandle[]
  defaultLabel?: string
  subtitle?: string
}

const props = withDefaults(defineProps<Props>(), {
  handles: () => [{ type: 'source', position: Position.Bottom }],
  defaultLabel: 'Node',
  subtitle: ''
})

const theme = computed(() => getNodeTheme(props.nodeType))

const nodeClasses = computed(() => {
  const baseClass = theme.value.borderDefault
  
  if (!props.data.isTracing || !props.data.executionStatus) {
    return baseClass
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
      return baseClass
  }
})

const nodeStyles = computed(() => ({
  background: theme.value.background,
  backdropFilter: 'blur(20px)',
  WebkitBackdropFilter: 'blur(20px)',
  border: `1px solid ${theme.value.border}`,
  boxShadow: theme.value.boxShadow,
  transition: 'all 0.3s ease'
}))

const statusIndicatorClasses = computed(() => {
  if (!props.data.executionStatus) return ''
  
  switch (props.data.executionStatus) {
    case 'completed':
      return 'bg-green-500'
    case 'failed':
      return 'bg-red-500'
    case 'running':
      return 'bg-blue-500 animate-pulse'
    case 'pending':
      return 'bg-yellow-500'
    case 'skipped':
      return 'bg-gray-500'
    default:
      return 'bg-gray-500'
  }
})

const subtitleColorClass = computed(() => {
  const themeColor = theme.value.color
  switch (themeColor) {
    case 'blue': return 'text-blue-200'
    case 'green': return 'text-green-200'
    case 'amber': return 'text-amber-200'
    case 'purple': return 'text-purple-200'
    case 'orange': return 'text-orange-200'
    case 'gray': return 'text-gray-200'
    default: return 'text-gray-200'
  }
})
</script>

<style scoped>
.node-trigger:hover,
.node-condition:hover,
.node-transformer:hover,
.node-http-request:hover,
.node-openobserve:hover,
.node-email:hover,
.node-delay:hover,
.node-app:hover {
  transform: translateY(-1px);
}

.node-trigger:hover {
  background: rgba(59, 130, 246, 0.18) !important;
  box-shadow: 
    0 12px 40px rgba(59, 130, 246, 0.25),
    inset 0 1px 0 rgba(255, 255, 255, 0.15) !important;
}

.node-condition:hover {
  background: rgba(245, 158, 11, 0.18) !important;
  box-shadow: 
    0 12px 40px rgba(245, 158, 11, 0.25),
    inset 0 1px 0 rgba(255, 255, 255, 0.15) !important;
}

.node-transformer:hover {
  background: rgba(139, 92, 246, 0.18) !important;
  box-shadow: 
    0 12px 40px rgba(139, 92, 246, 0.25),
    inset 0 1px 0 rgba(255, 255, 255, 0.15) !important;
}

.node-http-request:hover,
.node-app:hover {
  background: rgba(16, 185, 129, 0.18) !important;
  box-shadow: 
    0 12px 40px rgba(16, 185, 129, 0.25),
    inset 0 1px 0 rgba(255, 255, 255, 0.15) !important;
}

.node-openobserve:hover {
  background: rgba(249, 115, 22, 0.18) !important;
  box-shadow: 
    0 12px 40px rgba(249, 115, 22, 0.25),
    inset 0 1px 0 rgba(255, 255, 255, 0.15) !important;
}

.node-email:hover {
  background: rgba(33, 150, 243, 0.18) !important;
  box-shadow: 
    0 12px 40px rgba(33, 150, 243, 0.25),
    inset 0 1px 0 rgba(255, 255, 255, 0.15) !important;
}

.node-delay:hover {
  background: rgba(107, 114, 128, 0.18) !important;
  box-shadow: 
    0 12px 40px rgba(107, 114, 128, 0.25),
    inset 0 1px 0 rgba(255, 255, 255, 0.15) !important;
}
</style>