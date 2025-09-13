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

function getConditionType() {
  if (props.data.config?.condition_type) {
    return props.data.config.condition_type
  }
  if (props.data.config?.type) {
    return props.data.config.type
  }
  return 'If/Then'
}
</script>