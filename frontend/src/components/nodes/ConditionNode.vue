<template>
  <div class="node-condition px-4 py-3 rounded-lg shadow-2xl min-w-[180px] border-2 border-amber-400/30">
    <div class="flex items-center justify-between mb-2">
      <div class="flex-1">
        <div class="text-sm font-medium">{{ data.label || 'Condition' }}</div>
        <div class="text-xs text-amber-200 opacity-80">{{ getConditionType() }}</div>
      </div>
    </div>
    
    <!-- True/False labels -->
    <div class="flex justify-between text-xs">
      <span class="text-green-200">T</span>
      <span class="text-red-200">F</span>
    </div>
    
    <!-- Connection handles -->
    <Handle
      type="target"
      :position="Position.Top"
    />
    <Handle
      id="true"
      type="source"
      :position="Position.Bottom"
      :style="{ left: '30%' }"
    />
    <Handle
      id="false"
      type="source"
      :position="Position.Bottom"
      :style="{ left: '70%' }"
    />
  </div>
</template>

<script setup lang="ts">
import { Handle, Position } from '@vue-flow/core'

interface Props {
  data: {
    label: string
    description?: string
    status?: string
    config: any
  }
}

const props = defineProps<Props>()

function getConditionType() {
  if (props.data.config?.condition_type) {
    return props.data.config.condition_type
  }
  if (props.data.config?.type) {
    return props.data.config.type
  }
  return 'If/Then'
}
</script>

<style scoped>
.node-condition {
  background: rgba(245, 158, 11, 0.12);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border: 1px solid rgba(245, 158, 11, 0.25);
  box-shadow: 
    0 8px 32px rgba(245, 158, 11, 0.15),
    inset 0 1px 0 rgba(255, 255, 255, 0.1);
  position: relative;
  transition: all 0.3s ease;
}

.node-condition:hover {
  background: rgba(245, 158, 11, 0.18);
  box-shadow: 
    0 12px 40px rgba(245, 158, 11, 0.25),
    inset 0 1px 0 rgba(255, 255, 255, 0.15);
  transform: translateY(-1px);
}
</style>