<template>
  <BaseNode
    node-type="transformer"
    :data="data"
    :subtitle="getTransformerType()"
    :handles="[
      { type: 'target', position: Position.Top },
      { type: 'source', position: Position.Bottom }
    ]"
    default-label="Transformer"
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