<template>
  <BaseNode
    node-type="app"
    :data="data"
    :node-id="nodeId"
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
import type { WorkflowNodeData, AppConfig } from '../../types/nodes'

interface Props {
  data: WorkflowNodeData
  nodeId: string
}

const props = defineProps<Props>()

function getAppType() {
  const config = props.data.config as AppConfig
  if (config?.app_type) {
    return config.app_type
  }
  if (config?.type) {
    return config.type
  }
  return 'App'
}
</script>