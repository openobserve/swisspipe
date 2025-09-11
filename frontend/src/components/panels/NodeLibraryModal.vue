<template>
  <div class="flex flex-col h-full max-h-[80vh]">
    <!-- Header -->
    <div class="flex items-center justify-between p-6 border-b border-slate-700/50">
      <h2 class="text-xl font-semibold text-white">Node Library</h2>
      <button
        @click="$emit('close')"
        class="text-gray-400 hover:text-gray-200 transition-colors p-2 rounded-md hover:bg-slate-700/30"
        aria-label="Close"
      >
        <XMarkIcon class="h-5 w-5" />
      </button>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-y-auto p-6">
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">

        <!-- Transformers Section -->
        <div class="space-y-4">
          <h3 class="text-sm font-medium text-gray-300 mb-3 sticky top-0 bg-slate-800 py-2">Transformers</h3>
          <div class="space-y-3">
            <div
              v-for="nodeType in transformerNodes"
              :key="nodeType.type"
              class="flex items-start space-x-3 p-4 glass hover:glass-medium rounded-lg transition-all duration-300 border-l-4 group"
              :style="{ borderLeftColor: nodeType.color }"
            >
              <div
                class="w-3 h-3 rounded-full flex-shrink-0 mt-1"
                :style="{ backgroundColor: nodeType.color }"
              ></div>
              <div class="flex-1 min-w-0">
                <h4 class="text-sm font-medium text-white">{{ nodeType.label }}</h4>
                <p class="text-xs text-gray-400 mt-1">{{ nodeType.description }}</p>
              </div>
              <button
                @click="addNode(nodeType)"
                class="opacity-0 group-hover:opacity-100 transition-opacity bg-green-600 hover:bg-green-700 text-white rounded-full w-8 h-8 flex items-center justify-center text-lg font-bold"
                title="Add node"
              >
                +
              </button>
            </div>
          </div>
        </div>

        <!-- Logic Section -->
        <div class="space-y-4">
          <h3 class="text-sm font-medium text-gray-300 mb-3 sticky top-0 bg-slate-800 py-2">Logic</h3>
          <div class="space-y-3">
            <div
              v-for="nodeType in logicNodes"
              :key="nodeType.type"
              class="flex items-start space-x-3 p-4 glass hover:glass-medium rounded-lg transition-all duration-300 border-l-4 group"
              :style="{ borderLeftColor: nodeType.color }"
            >
              <div
                class="w-3 h-3 rounded-full flex-shrink-0 mt-1"
                :style="{ backgroundColor: nodeType.color }"
              ></div>
              <div class="flex-1 min-w-0">
                <h4 class="text-sm font-medium text-white">{{ nodeType.label }}</h4>
                <p class="text-xs text-gray-400 mt-1">{{ nodeType.description }}</p>
              </div>
              <button
                @click="addNode(nodeType)"
                class="opacity-0 group-hover:opacity-100 transition-opacity bg-green-600 hover:bg-green-700 text-white rounded-full w-8 h-8 flex items-center justify-center text-lg font-bold"
                title="Add node"
              >
                +
              </button>
            </div>
          </div>
        </div>

        <!-- Apps Section -->
        <div class="space-y-4">
          <h3 class="text-sm font-medium text-gray-300 mb-3 sticky top-0 bg-slate-800 py-2">Apps</h3>
          <div class="space-y-3">
            <div
              v-for="nodeType in appNodes"
              :key="nodeType.type"
              class="flex items-start space-x-3 p-4 glass hover:glass-medium rounded-lg transition-all duration-300 border-l-4 group"
              :style="{ borderLeftColor: nodeType.color }"
            >
              <div
                class="w-3 h-3 rounded-full flex-shrink-0 mt-1"
                :style="{ backgroundColor: nodeType.color }"
              ></div>
              <div class="flex-1 min-w-0">
                <h4 class="text-sm font-medium text-white">{{ nodeType.label }}</h4>
                <p class="text-xs text-gray-400 mt-1">{{ nodeType.description }}</p>
              </div>
              <button
                @click="addNode(nodeType)"
                class="opacity-0 group-hover:opacity-100 transition-opacity bg-green-600 hover:bg-green-700 text-white rounded-full w-8 h-8 flex items-center justify-center text-lg font-bold"
                title="Add node"
              >
                +
              </button>
            </div>
          </div>
        </div>

        <!-- Communication Section -->
        <div class="space-y-4">
          <h3 class="text-sm font-medium text-gray-300 mb-3 sticky top-0 bg-slate-800 py-2">Communication</h3>
          <div class="space-y-3">
            <div
              v-for="nodeType in communicationNodes"
              :key="nodeType.type"
              class="flex items-start space-x-3 p-4 glass hover:glass-medium rounded-lg transition-all duration-300 border-l-4 group"
              :style="{ borderLeftColor: nodeType.color }"
            >
              <div
                class="w-3 h-3 rounded-full flex-shrink-0 mt-1"
                :style="{ backgroundColor: nodeType.color }"
              ></div>
              <div class="flex-1 min-w-0">
                <h4 class="text-sm font-medium text-white">{{ nodeType.label }}</h4>
                <p class="text-xs text-gray-400 mt-1">{{ nodeType.description }}</p>
              </div>
              <button
                @click="addNode(nodeType)"
                class="opacity-0 group-hover:opacity-100 transition-opacity bg-green-600 hover:bg-green-700 text-white rounded-full w-8 h-8 flex items-center justify-center text-lg font-bold"
                title="Add node"
              >
                +
              </button>
            </div>
          </div>
        </div>

      </div>
    </div>

    <!-- Footer -->
    <div class="border-t border-slate-700/50 p-4 bg-slate-800/50">
      <p class="text-xs text-gray-400 text-center">
        Click the + button next to any node type to add it to your workflow
      </p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { XMarkIcon } from '@heroicons/vue/24/outline'
import { useNodeStore } from '../../stores/nodes'
import type { NodeTypeDefinition } from '../../types/nodes'

const emit = defineEmits<{
  close: []
  'add-node': [nodeType: NodeTypeDefinition]
}>()

const nodeStore = useNodeStore()

// Computed properties to categorize nodes
const transformerNodes = computed(() => 
  nodeStore.nodeTypes.filter(type => type.type === 'transformer')
)
const logicNodes = computed(() => 
  nodeStore.nodeTypes.filter(type => type.type === 'condition' || type.type === 'delay')
)
const appNodes = computed(() => 
  nodeStore.nodeTypes.filter(type => type.type === 'webhook' || type.type === 'openobserve')
)
const communicationNodes = computed(() => 
  nodeStore.nodeTypes.filter(type => type.type === 'email')
)

function addNode(nodeType: NodeTypeDefinition) {
  emit('add-node', nodeType)
}
</script>