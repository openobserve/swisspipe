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
import type { WorkflowNodeData, TransformerConfig } from '../../types/nodes'

interface Props {
  data: WorkflowNodeData
}

const props = defineProps<Props>()

function getTransformerType() {
  const config = props.data.config as TransformerConfig
  if (props.data.transformer_type) {
    return props.data.transformer_type
  }
  if (config?.type) {
    return config.type
  }
  return 'Data Transform'
}
</script>