<template>
  <BaseNode
    node-type="http-request"
    :data="data"
    subtitle="HTTP Request"
    :handles="[
      { type: 'target', position: Position.Top },
      { type: 'source', position: Position.Bottom }
    ]"
    default-label="HTTP Request"
  />
</template>

<script setup lang="ts">
import { Position } from '@vue-flow/core'
import BaseNode from './BaseNode.vue'

interface Props {
  data: {
    label: string
    description?: string
    status?: string
    config: unknown
    isTracing?: boolean
    executionStatus?: string
    executionDuration?: number
    executionError?: string
  }
}

defineProps<Props>()
</script>