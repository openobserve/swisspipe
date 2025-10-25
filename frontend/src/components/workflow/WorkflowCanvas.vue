<template>
  <div
    class="absolute inset-0"
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
      :connection-mode="'loose'"
      :connect-on-click="false"
    >
      <Background pattern-color="#a7abb0" :gap="20" />
      <Controls />
      
      <!-- Custom Node Types -->
      <template #node-trigger="nodeProps">
        <TriggerNode :data="nodeProps.data" :node-id="nodeProps.id" />
      </template>
      <template #node-condition="nodeProps">
        <ConditionNode :data="nodeProps.data" :node-id="nodeProps.id" />
      </template>
      <template #node-transformer="nodeProps">
        <TransformerNode :data="nodeProps.data" :node-id="nodeProps.id" />
      </template>
      <template #node-http-request="nodeProps">
        <HttpRequestNode
          :data="nodeProps.data"
          :node-id="nodeProps.id"
          @pause-loop="$emit('pause-loop', $event)"
          @stop-loop="$emit('stop-loop', $event)"
          @retry-loop="$emit('retry-loop', $event)"
        />
      </template>
      <template #node-openobserve="nodeProps">
        <OpenObserveNode :data="nodeProps.data" :node-id="nodeProps.id" />
      </template>
      <template #node-app="nodeProps">
        <AppNode :data="nodeProps.data" :node-id="nodeProps.id" />
      </template>
      <template #node-email="nodeProps">
        <EmailNode :data="nodeProps.data" :node-id="nodeProps.id" />
      </template>
      <template #node-delay="nodeProps">
        <DelayNode :data="nodeProps.data" :node-id="nodeProps.id" />
      </template>
      <template #node-anthropic="nodeProps">
        <AnthropicNode :data="nodeProps.data" :node-id="nodeProps.id" />
      </template>
      <template #node-human-in-loop="nodeProps">
        <HumanInLoopNode :data="nodeProps.data" :node-id="nodeProps.id" />
      </template>
    </VueFlow>
  </div>
</template>

<script setup lang="ts">
import { provide } from 'vue'
import { VueFlow } from '@vue-flow/core'
import type { Node, Edge, Connection, NodeMouseEvent, EdgeMouseEvent } from '@vue-flow/core'
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
import AnthropicNode from '../nodes/AnthropicNode.vue'
import HumanInLoopNode from '../nodes/HumanInLoopNode.vue'

interface Props {
  nodes: Node[]
  edges: Edge[]
  onHandleClick?: (nodeId: string, sourceHandle: string | undefined, event: MouseEvent) => void
}

interface Emits {
  (e: 'update:nodes', value: Node[]): void
  (e: 'update:edges', value: Edge[]): void
  (e: 'node-click', event: NodeMouseEvent): void
  (e: 'edge-click', event: EdgeMouseEvent): void
  (e: 'pane-click', event: MouseEvent): void
  (e: 'connect', event: Connection): void
  (e: 'nodes-initialized'): void
  (e: 'nodes-delete', event: { nodes: Node[] }): void
  (e: 'drop', event: DragEvent): void
  (e: 'pause-loop', loopId: string): void
  (e: 'stop-loop', loopId: string): void
  (e: 'retry-loop', loopId: string): void
}

const props = defineProps<Props>()
defineEmits<Emits>()

// Provide the handle click function to child node components
if (props.onHandleClick) {
  provide('onHandleClick', props.onHandleClick)
}
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