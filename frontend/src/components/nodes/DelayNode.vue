<template>
  <BaseNode
    node-type="delay"
    :data="data"
    :subtitle="getDelaySummary()"
    default-label="Delay"
  >
    <template #handles>
      <!-- Connection handles with custom styling -->
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
      
      <!-- Clock icon -->
      <div class="absolute top-2 right-2 text-gray-400">
        <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
          <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm1-12a1 1 0 10-2 0v4a1 1 0 00.293.707l2.828 2.829a1 1 0 101.415-1.415L11 9.586V6z" clip-rule="evenodd" />
        </svg>
      </div>
    </template>
  </BaseNode>
</template>

<script setup lang="ts">
import { Handle, Position } from '@vue-flow/core'
import BaseNode from './BaseNode.vue'
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

function getDelaySummary(): string {
  const config = props.data.config
  if (!config.duration || !config.unit) {
    return 'Not configured'
  }
  
  const unitDisplay = config.unit.toLowerCase()
  return `${config.duration} ${unitDisplay}`
}

</script>

<style scoped>
.node-delay {
  font-family: 'Inter', sans-serif;
}
</style>