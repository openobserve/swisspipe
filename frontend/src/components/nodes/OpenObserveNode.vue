<template>
  <BaseNode
    node-type="openobserve"
    :data="data"
    :node-id="nodeId"
    subtitle="Log Ingestion"
    :handles="[
      { type: 'target', position: Position.Top },
      { type: 'source', position: Position.Bottom }
    ]"
    default-label="OpenObserve"
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
  nodeId: string
}

defineProps<Props>()
</script>