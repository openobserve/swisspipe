<template>
  <BaseNode
    node-type="app"
    :data="data"
    :subtitle="getAppType()"
    :handles="[
      { type: 'target', position: Position.Top },
      { type: 'source', position: Position.Bottom }
    ]"
    default-label="App"
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

const props = defineProps<Props>()

function getAppType() {
  if (props.data.config?.app_type) {
    return props.data.config.app_type
  }
  if (props.data.config?.type) {
    return props.data.config.type
  }
  return 'App'
}
</script>