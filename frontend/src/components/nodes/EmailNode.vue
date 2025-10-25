<template>
  <BaseNode
    node-type="email"
    :data="data"
    :node-id="nodeId"
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
      <div @click="onHandleClick($event)">
        <Handle
          type="source"
          :position="Position.Bottom"
          :style="{ background: '#ddd', cursor: 'pointer' }"
        />
      </div>

    </template>
  </BaseNode>
</template>

<script setup lang="ts">
import { inject } from 'vue'
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
  nodeId: string
}

const props = defineProps<Props>()

// Inject the handle click handler from the parent
const onHandleClickInjected = inject<(nodeId: string, sourceHandle: string | undefined, event: MouseEvent) => void>('onHandleClick')

function onHandleClick(event: MouseEvent) {
  console.log('📧 EmailNode handle clicked:', props.nodeId)

  event.stopPropagation()
  event.preventDefault()

  if (!onHandleClickInjected) {
    console.error('❌ onHandleClickInjected is not available')
    return
  }

  if (!props.nodeId) {
    console.error('❌ nodeId prop is not available')
    return
  }

  console.log('✅ Calling injected handler')
  onHandleClickInjected(props.nodeId, undefined, event)
}

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