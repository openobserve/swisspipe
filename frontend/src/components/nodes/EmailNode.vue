<template>
  <BaseNode
    node-type="email"
    :data="data"
    :subtitle="getEmailSummary()"
    default-label="Email"
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
      
      <!-- Priority indicator -->
      <div 
        v-if="data.config?.priority && data.config.priority !== 'normal'"
        class="absolute top-2 right-8 w-2 h-2 rounded-full"
        :class="getPriorityColor()"
        :title="`Priority: ${data.config.priority}`"
      ></div>
    </template>
  </BaseNode>
</template>

<script setup lang="ts">
import { Handle, Position } from '@vue-flow/core'
import BaseNode from './BaseNode.vue'
import type { EmailConfig } from '../../types/nodes'

interface Props {
  data: {
    label: string
    description?: string
    status?: string
    config: EmailConfig
    isTracing?: boolean
    executionStatus?: string
    executionDuration?: number
    executionError?: string
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

function formatDuration(durationMs: number | null): string {
  if (!durationMs) return 'N/A'
  
  if (durationMs < 1000) return `${durationMs}ms`
  if (durationMs < 60000) return `${(durationMs / 1000).toFixed(1)}s`
  return `${(durationMs / 60000).toFixed(1)}m`
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