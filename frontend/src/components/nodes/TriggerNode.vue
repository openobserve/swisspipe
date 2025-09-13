<template>
  <BaseNode
    node-type="trigger"
    :data="data"
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
    config: any
    isTracing?: boolean
    executionStatus?: string
    executionDuration?: number
    executionError?: string
  }
}

defineProps<Props>()
</script>