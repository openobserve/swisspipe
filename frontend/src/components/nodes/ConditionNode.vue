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
      <!-- True handle with click wrapper -->
      <div
        class="absolute bottom-0"
        :style="{ left: '30%', transform: 'translateX(-50%)' }"
        @click="onHandleClick($event, 'true')"
      >
        <Handle
          id="true"
          type="source"
          :position="Position.Bottom"
          :style="{ position: 'relative', left: '0', cursor: 'pointer' }"
        />
      </div>
      <!-- False handle with click wrapper -->
      <div
        class="absolute bottom-0"
        :style="{ left: '70%', transform: 'translateX(-50%)' }"
        @click="onHandleClick($event, 'false')"
      >
        <Handle
          id="false"
          type="source"
          :position="Position.Bottom"
          :style="{ position: 'relative', left: '0', cursor: 'pointer' }"
        />
      </div>
    </template>
  </BaseNode>
</template>

<script setup lang="ts">
import { inject } from 'vue'
import { Handle, Position } from '@vue-flow/core'
import BaseNode from './BaseNode.vue'
import type { WorkflowNodeData, ConditionConfig } from '../../types/nodes'

interface Props {
  data: WorkflowNodeData
  nodeId: string
}

const props = defineProps<Props>()

// Inject the handle click handler from the parent
const onHandleClickInjected = inject<(nodeId: string, sourceHandle: string | undefined, event: MouseEvent) => void>('onHandleClick')

function onHandleClick(event: MouseEvent, handleId: string) {
  console.log('🔵 ConditionNode handle clicked:', handleId, props.nodeId)

  // Stop event from bubbling to node click
  event.stopPropagation()
  event.preventDefault()

  if (!onHandleClickInjected) {
    console.error('❌ onHandleClickInjected is not available')
    return
  }

  if (!props.nodeId) {
    console.error('❌ nodeId prop is not available')
    return
  }

  console.log('✅ Calling injected handler')
  onHandleClickInjected(props.nodeId, handleId, event)
}

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