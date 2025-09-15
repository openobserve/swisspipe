<template>
  <BaseNode
    node-type="condition"
    :data="data"
    :subtitle="getConditionType()"
    default-label="Condition"
  >
    <template #content>
      <!-- True/False labels -->
      <div class="flex justify-between text-xs">
        <span class="text-green-200">True</span>
        <span class="text-red-200">False</span>
      </div>
    </template>
    
    <template #handles>
      <!-- Connection handles -->
      <Handle
        type="target"
        :position="Position.Top"
      />
      <Handle
        id="true"
        type="source"
        :position="Position.Bottom"
        :style="{ left: '30%' }"
      />
      <Handle
        id="false"
        type="source"
        :position="Position.Bottom"
        :style="{ left: '70%' }"
      />
    </template>
  </BaseNode>
</template>

<script setup lang="ts">
import { Handle, Position } from '@vue-flow/core'
import BaseNode from './BaseNode.vue'
import type { WorkflowNodeData, ConditionConfig } from '../../types/nodes'

interface Props {
  data: WorkflowNodeData
}

const props = defineProps<Props>()

function getConditionType() {
  const config = props.data.config as ConditionConfig
  if (props.data.condition_type) {
    return props.data.condition_type
  }
  if (config?.type) {
    return config.type
  }
  return 'If/Then'
}
</script>