<template>
  <BaseNode
    node-type="human-in-loop"
    :data="data"
    :subtitle="getHilSummary()"
    default-label="Human in Loop"
  >
    <template #handles>
      <!-- Input handle -->
      <Handle
        type="target"
        :position="Position.Top"
        :style="{ background: '#ddd' }"
      />

      <!-- Output handles -->
      <!-- Approved handle (left) -->
      <Handle
        type="source"
        id="approved"
        :position="Position.Bottom"
        :style="{
          background: '#10b981',
          left: '25%',
          transform: 'translateX(-50%)',
          cursor: 'pointer'
        }"
        @click="onHandleClick($event, 'approved')"
      />

      <!-- Denied handle (center) -->
      <Handle
        type="source"
        id="denied"
        :position="Position.Bottom"
        :style="{
          background: '#ef4444',
          left: '50%',
          transform: 'translateX(-50%)',
          cursor: 'pointer'
        }"
        @click="onHandleClick($event, 'denied')"
      />

      <!-- Notification handle (right) -->
      <Handle
        type="source"
        id="notification"
        :position="Position.Bottom"
        :style="{
          background: '#3b82f6',
          left: '75%',
          transform: 'translateX(-50%)',
          cursor: 'pointer'
        }"
        @click="onHandleClick($event, 'notification')"
      />

      <!-- Handle labels -->
      <div class="absolute -bottom-6 left-1/4 transform -translate-x-1/2 text-xs text-green-400 font-medium">
        Approved
      </div>
      <div class="absolute -bottom-6 left-1/2 transform -translate-x-1/2 text-xs text-red-400 font-medium">
        Denied
      </div>
      <div class="absolute -bottom-6 left-3/4 transform -translate-x-1/2 text-xs text-blue-400 font-medium">
        Notify
      </div>

      <!-- User icon -->
      <div class="absolute top-2 right-2 text-red-400">
        <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
          <path d="M10 9a3 3 0 100-6 3 3 0 000 6zm-7 9a7 7 0 1114 0H3z"/>
        </svg>
      </div>

      <!-- Timeout indicator -->
      <div
        v-if="data.config?.timeout_seconds"
        class="absolute top-2 right-8 w-2 h-2 rounded-full bg-orange-400"
        :title="`Timeout: ${data.config.timeout_seconds}s`"
      ></div>

      <!-- Pending status indicator -->
      <div
        v-if="data.executionStatus === 'pending' || data.status === 'pending'"
        class="absolute -top-1 -right-1 w-3 h-3"
      >
        <div class="w-3 h-3 bg-yellow-400 rounded-full animate-pulse"></div>
      </div>
    </template>
  </BaseNode>
</template>

<script setup lang="ts">
import { inject } from 'vue'
import { Handle, Position } from '@vue-flow/core'
import BaseNode from './BaseNode.vue'
import type { HumanInLoopConfig } from '../../types/nodes'

interface Props {
  data: {
    label: string
    description?: string
    status?: string
    config: HumanInLoopConfig
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

function onHandleClick(event: MouseEvent, handleId: string) {
  console.log('🟣 HumanInLoopNode handle clicked:', handleId, props.nodeId)

  // Stop event from bubbling to node click
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
  onHandleClickInjected(props.nodeId, handleId, event)
}

function getHilSummary(): string {
  const config = props.data.config
  if (!config.title) {
    return 'Not configured'
  }

  const timeoutText = config.timeout_seconds
    ? ` (${Math.round(config.timeout_seconds / 60)}m timeout)`
    : ' (no timeout)'

  return `${config.title}${timeoutText}`
}

</script>

<style scoped>
.node-human-in-loop {
  font-family: 'Inter', sans-serif;
}
</style>