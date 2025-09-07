<template>
  <div class="node-trigger px-4 py-3 rounded-lg shadow-2xl min-w-[180px] border-2 border-blue-400/30">
    <div class="flex items-center justify-between">
      <span class="text-sm font-medium">{{ data.label || 'Trigger' }}</span>
      <button class="text-white/70 hover:text-white transition-colors">
        <CogIcon class="h-4 w-4" />
      </button>
    </div>
    
    <!-- Connection handles -->
    <Handle
      type="source"
      :position="Position.Bottom"
    />
  </div>
</template>

<script setup lang="ts">
import { Handle, Position } from '@vue-flow/core'
import { CogIcon } from '@heroicons/vue/24/outline'

interface Props {
  data: {
    label: string
    description?: string
    status?: string
    config: any
  }
}

defineProps<Props>()
</script>

<style scoped>
.node-trigger {
  background: rgba(59, 130, 246, 0.12);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border: 1px solid rgba(59, 130, 246, 0.25);
  box-shadow: 
    0 8px 32px rgba(59, 130, 246, 0.15),
    inset 0 1px 0 rgba(255, 255, 255, 0.1);
  transition: all 0.3s ease;
}

.node-trigger:hover {
  background: rgba(59, 130, 246, 0.18);
  box-shadow: 
    0 12px 40px rgba(59, 130, 246, 0.25),
    inset 0 1px 0 rgba(255, 255, 255, 0.15);
  transform: translateY(-1px);
}
</style>