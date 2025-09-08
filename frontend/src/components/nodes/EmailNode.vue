<template>
  <div class="node-email px-4 py-3 rounded-lg shadow-2xl min-w-[180px] border-2 border-blue-400/30">
    <div class="flex items-center justify-between mb-2">
      <div class="flex-1">
        <div class="text-sm font-medium">{{ data.label || 'Email' }}</div>
        <div class="text-xs text-blue-200 opacity-80">{{ getEmailSummary() }}</div>
      </div>
      <div class="flex items-center space-x-1">
        <!-- Priority indicator -->
        <div 
          v-if="data.config?.priority && data.config.priority !== 'normal'"
          class="w-2 h-2 rounded-full"
          :class="getPriorityColor()"
          :title="`Priority: ${data.config.priority}`"
        ></div>
        <!-- Queue indicator -->
        <div
          v-if="data.config?.queue_if_rate_limited"
          class="w-2 h-2 rounded-full bg-orange-400"
          title="Queue if rate limited"
        ></div>
      </div>
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
import { Handle, Position } from '@vue-flow/core'
import type { EmailConfig } from '../../types/nodes'

interface Props {
  data: {
    label: string
    description?: string
    status?: string
    config: EmailConfig
  }
}

const props = defineProps<Props>()

function getEmailSummary() {
  const config = props.data.config
  if (!config) return 'Email'
  
  const recipientCount = config.to?.length || 0
  const ccCount = config.cc?.length || 0
  const bccCount = config.bcc?.length || 0
  const totalRecipients = recipientCount + ccCount + bccCount
  
  const parts = []
  if (totalRecipients > 0) {
    parts.push(`${totalRecipients} recipient${totalRecipients > 1 ? 's' : ''}`)
  }
  if (config.template_type) {
    parts.push(config.template_type.toUpperCase())
  }
  
  return parts.join(' • ') || 'Email'
}

function getPriorityColor() {
  const priority = props.data.config?.priority
  switch (priority) {
    case 'critical': return 'bg-red-500'
    case 'high': return 'bg-orange-500'
    case 'low': return 'bg-gray-500'
    default: return 'bg-blue-500'
  }
}
</script>

<style scoped>
.node-email {
  background: rgba(33, 150, 243, 0.12);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border: 1px solid rgba(33, 150, 243, 0.25);
  box-shadow: 
    0 8px 32px rgba(33, 150, 243, 0.15),
    inset 0 1px 0 rgba(255, 255, 255, 0.1);
  transition: all 0.3s ease;
  position: relative;
}

.node-email:hover {
  background: rgba(33, 150, 243, 0.18);
  box-shadow: 
    0 12px 40px rgba(33, 150, 243, 0.25),
    inset 0 1px 0 rgba(255, 255, 255, 0.15);
  transform: translateY(-1px);
}
</style>