<template>
  <div class="p-4">
    <h2 class="text-lg font-semibold text-white mb-6">Node Library</h2>
    
    <div class="space-y-6">

      <!-- Transformers Section -->
      <div>
        <h3 class="text-sm font-medium text-gray-300 mb-3">Transformers</h3>
        <div
          v-for="nodeType in transformerNodes"
          :key="nodeType.type"
          @dragstart="startDrag($event, nodeType)"
          draggable="true"
          class="flex items-start space-x-3 p-3 glass hover:glass-medium rounded-lg cursor-move transition-all duration-300 border-l-4"
          :style="{ borderLeftColor: nodeType.color }"
        >
          <div
            class="w-3 h-3 rounded-full flex-shrink-0 mt-0.5"
            :style="{ backgroundColor: nodeType.color }"
          ></div>
          <div class="flex-1 min-w-0">
            <h4 class="text-sm font-medium text-white">{{ nodeType.label }}</h4>
            <p class="text-xs text-gray-400 mt-1">{{ nodeType.description }}</p>
          </div>
        </div>
      </div>

      <!-- Logic Section -->
      <div>
        <h3 class="text-sm font-medium text-gray-300 mb-3">Logic</h3>
        <div
          v-for="nodeType in logicNodes"
          :key="nodeType.type"
          @dragstart="startDrag($event, nodeType)"
          draggable="true"
          class="flex items-start space-x-3 p-3 glass hover:glass-medium rounded-lg cursor-move transition-all duration-300 border-l-4"
          :style="{ borderLeftColor: nodeType.color }"
        >
          <div
            class="w-3 h-3 rounded-full flex-shrink-0 mt-0.5"
            :style="{ backgroundColor: nodeType.color }"
          ></div>
          <div class="flex-1 min-w-0">
            <h4 class="text-sm font-medium text-white">{{ nodeType.label }}</h4>
            <p class="text-xs text-gray-400 mt-1">{{ nodeType.description }}</p>
          </div>
        </div>
      </div>

      <!-- Apps Section -->
      <div>
        <h3 class="text-sm font-medium text-gray-300 mb-3">Apps</h3>
        <div
          v-for="nodeType in appNodes"
          :key="nodeType.type"
          @dragstart="startDrag($event, nodeType)"
          draggable="true"
          class="flex items-start space-x-3 p-3 glass hover:glass-medium rounded-lg cursor-move transition-all duration-300 border-l-4"
          :style="{ borderLeftColor: nodeType.color }"
        >
          <div
            class="w-3 h-3 rounded-full flex-shrink-0 mt-0.5"
            :style="{ backgroundColor: nodeType.color }"
          ></div>
          <div class="flex-1 min-w-0">
            <h4 class="text-sm font-medium text-white">{{ nodeType.label }}</h4>
            <p class="text-xs text-gray-400 mt-1">{{ nodeType.description }}</p>
          </div>
        </div>
      </div>

      <!-- Communication Section -->
      <div>
        <h3 class="text-sm font-medium text-gray-300 mb-3">Communication</h3>
        <div
          v-for="nodeType in communicationNodes"
          :key="nodeType.type"
          @dragstart="startDrag($event, nodeType)"
          draggable="true"
          class="flex items-start space-x-3 p-3 glass hover:glass-medium rounded-lg cursor-move transition-all duration-300 border-l-4"
          :style="{ borderLeftColor: nodeType.color }"
        >
          <div
            class="w-3 h-3 rounded-full flex-shrink-0 mt-0.5"
            :style="{ backgroundColor: nodeType.color }"
          ></div>
          <div class="flex-1 min-w-0">
            <h4 class="text-sm font-medium text-white">{{ nodeType.label }}</h4>
            <p class="text-xs text-gray-400 mt-1">{{ nodeType.description }}</p>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useNodeStore } from '../../stores/nodes'
import type { NodeTypeDefinition } from '../../types/nodes'

const nodeStore = useNodeStore()

const transformerNodes = computed(() => 
  nodeStore.nodeTypes.filter(type => type.type === 'transformer')
)

const logicNodes = computed(() => 
  nodeStore.nodeTypes.filter(type => type.type === 'condition' || type.type === 'delay')
)

const appNodes = computed(() => 
  nodeStore.nodeTypes.filter(type => type.type === 'app')
)

const communicationNodes = computed(() => 
  nodeStore.nodeTypes.filter(type => type.type === 'email')
)

function startDrag(event: DragEvent, nodeType: NodeTypeDefinition) {
  if (event.dataTransfer) {
    event.dataTransfer.setData('application/vueflow', JSON.stringify(nodeType))
    event.dataTransfer.effectAllowed = 'move'
  }
}
</script>

<style scoped>
/* Drag and drop styles */
.cursor-move:active {
  cursor: grabbing;
}

/* Hover effects */
.group:hover .group-hover\:visible {
  visibility: visible;
}
</style>