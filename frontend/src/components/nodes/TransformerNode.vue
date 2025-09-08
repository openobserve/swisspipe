<template>
  <div class="node-transformer px-4 py-3 rounded-lg shadow-2xl min-w-[180px] border-2 border-purple-400/30">
    <div class="flex items-center justify-between">
      <div class="flex-1">
        <div class="text-sm font-medium">{{ data.label || 'Transformer' }}</div>
        <div class="text-xs text-purple-200 opacity-80">{{ getTransformerType() }}</div>
      </div>
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

function getTransformerType() {
  if (props.data.config?.transformer_type) {
    return props.data.config.transformer_type
  }
  if (props.data.config?.type) {
    return props.data.config.type
  }
  return 'Data Transform'
}
</script>

<style scoped>
.node-transformer {
  background: rgba(139, 92, 246, 0.12);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border: 1px solid rgba(139, 92, 246, 0.25);
  box-shadow: 
    0 8px 32px rgba(139, 92, 246, 0.15),
    inset 0 1px 0 rgba(255, 255, 255, 0.1);
  transition: all 0.3s ease;
}

.node-transformer:hover {
  background: rgba(139, 92, 246, 0.18);
  box-shadow: 
    0 12px 40px rgba(139, 92, 246, 0.25),
    inset 0 1px 0 rgba(255, 255, 255, 0.15);
  transform: translateY(-1px);
}
</style>