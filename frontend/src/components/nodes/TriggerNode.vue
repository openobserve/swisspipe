<template>
  <BaseNode
    node-type="trigger"
    :data="data"
    :node-id="nodeId"
    :handles="[{ type: 'source', position: Position.Bottom }]"
    default-label="Trigger"
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