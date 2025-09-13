<template>
  <div
    class="flex-1 relative"
    @drop="$emit('drop', $event)"
    @dragover.prevent
    @dragenter.prevent
  >
    <VueFlow
      :nodes="nodes"
      :edges="edges"
      @update:nodes="$emit('update:nodes', $event)"
      @update:edges="$emit('update:edges', $event)"
      @node-click="$emit('node-click', $event)"
      @edge-click="$emit('edge-click', $event)"
      @pane-click="$emit('pane-click', $event)"
      @connect="$emit('connect', $event)"
      @nodes-initialized="$emit('nodes-initialized')"
      @nodes-delete="$emit('nodes-delete', $event)"
      class="vue-flow-dark"
      :default-viewport="{ zoom: 1 }"
      :min-zoom="0.2"
      :max-zoom="4"
      :delete-key-code="null"
    >
      <Background pattern-color="#a7abb0" :gap="20" />
      <Controls />
      
      <!-- Custom Node Types -->
      <template #node-trigger="{ data }">
        <TriggerNode :data="data" />
      </template>
      <template #node-condition="{ data }">
        <ConditionNode :data="data" />
      </template>
      <template #node-transformer="{ data }">
        <TransformerNode :data="data" />
      </template>
      <template #node-http-request="{ data }">
        <HttpRequestNode :data="data" />
      </template>
      <template #node-openobserve="{ data }">
        <OpenObserveNode :data="data" />
      </template>
      <template #node-app="{ data }">
        <AppNode :data="data" />
      </template>
      <template #node-email="{ data }">
        <EmailNode :data="data" />
      </template>
      <template #node-delay="{ data }">
        <DelayNode :data="data" />
      </template>
    </VueFlow>
  </div>
</template>

<script setup lang="ts">
import { VueFlow } from '@vue-flow/core'
import { Controls } from '@vue-flow/controls'
import { Background } from '@vue-flow/background'
import TriggerNode from '../nodes/TriggerNode.vue'
import ConditionNode from '../nodes/ConditionNode.vue'
import TransformerNode from '../nodes/TransformerNode.vue'
import HttpRequestNode from '../nodes/HttpRequestNode.vue'
import OpenObserveNode from '../nodes/OpenObserveNode.vue'
import AppNode from '../nodes/AppNode.vue'
import EmailNode from '../nodes/EmailNode.vue'
import DelayNode from '../nodes/DelayNode.vue'

interface Props {
  nodes: any[]
  edges: any[]
}

interface Emits {
  (e: 'update:nodes', value: any[]): void
  (e: 'update:edges', value: any[]): void
  (e: 'node-click', event: any): void
  (e: 'edge-click', event: any): void
  (e: 'pane-click', event: any): void
  (e: 'connect', event: any): void
  (e: 'nodes-initialized'): void
  (e: 'nodes-delete', event: any): void
  (e: 'drop', event: DragEvent): void
}

defineProps<Props>()
defineEmits<Emits>()
</script>

<style scoped>
.vue-flow-dark {
  background: #1a1a2e;
}

.vue-flow-dark .vue-flow__minimap {
  background-color: #16213e;
}

.vue-flow-dark .vue-flow__controls {
  button {
    background-color: #374151;
    border-color: #4b5563;
    color: #d1d5db;
  }
  
  button:hover {
    background-color: #4b5563;
  }
}
</style>