<template>
  <!-- Modal Backdrop -->
  <div
    v-if="selectedNodeData"
    class="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center p-4"
    @click.self="handleClose"
  >
    <!-- Modal Content -->
    <div class="bg-slate-800 rounded-xl border border-slate-700 w-[90vw] h-[90vh] overflow-hidden shadow-2xl">
      <!-- Modal Header -->
      <div class="flex items-center justify-between p-6 border-b border-slate-700/50">
        <div class="flex items-center space-x-4">
          
          <h2 class="text-xl font-semibold text-white">Node Properties</h2>
          <div class="flex items-center space-x-3">
            <div
              class="w-4 h-4 rounded-full"
              :style="{ backgroundColor: nodeTypeDefinition?.color || '#6b7280' }"
            ></div>
            <span class="text-sm font-medium text-gray-300">{{ nodeTypeDefinition?.label || 'Node' }}</span>
          </div>
          <button
            v-if="selectedNodeData?.type !== 'trigger'"
            @click="deleteNode"
            class="bg-red-600 hover:bg-red-700 text-white px-3 py-1.5 rounded text-sm font-medium transition-colors flex items-center space-x-2"
          >
            <svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
            </svg>
            <span>Delete</span>
          </button>
        </div>
        <button
          @click="handleClose"
          class="text-gray-400 hover:text-gray-200 transition-colors p-2 rounded-md hover:bg-slate-700/30"
          aria-label="Close"
        >
          <XMarkIcon class="h-5 w-5" />
        </button>
      </div>

      <!-- Modal Body -->
      <div class="p-6 h-[calc(90vh-120px)] flex flex-col">

    <!-- Node Basic Info -->
    <div class="mb-6">
      
      <div class="grid grid-cols-[30%_70%] gap-4">
        <div>
          <label class="block text-sm font-medium text-gray-300 mb-2">Node Name</label>
          <input
            v-model="localNodeData.label"
            type="text"
            :readonly="selectedNodeData?.type === 'trigger'"
            :class="[
              'w-full border text-gray-100 px-3 py-2 rounded-md focus:outline-none',
              selectedNodeData?.type === 'trigger' 
                ? 'bg-slate-800 border-slate-700 text-gray-400 cursor-not-allowed' 
                : 'bg-slate-700 border-slate-600 focus:ring-2 focus:ring-primary-500'
            ]"
          />
          <p v-if="selectedNodeData?.type === 'trigger'" class="text-xs text-gray-500 mt-1">
            Trigger node name is always "Start" and cannot be changed
          </p>
        </div>
        
        <div>
          <label class="block text-sm font-medium text-gray-300 mb-2">Description</label>
          <textarea
            v-model="localNodeData.description"
            rows="2"
            class="w-full bg-slate-700 border border-slate-600 text-gray-100 px-3 py-2 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
          ></textarea>
        </div>
      </div>
    </div>

    <!-- Node-specific Configuration -->
    <div class="flex-1 min-h-0">
      <!-- Trigger Node Configuration -->
      <div v-if="selectedNodeData.type === 'trigger'">
        <h3 class="text-sm font-semibold text-gray-300 mb-3">Trigger Configuration</h3>
        <TriggerConfig
          v-model="localNodeData.config"
          @update="updateNodeData"
        />
      </div>

      <!-- Condition Node Configuration -->
      <div v-if="selectedNodeData.type === 'condition'">
        <h3 class="text-sm font-semibold text-gray-300 mb-3">Condition Configuration</h3>
        <ConditionConfig
          v-model="localNodeData.config"
          @update="updateNodeData"
        />
      </div>

      <!-- Transformer Node Configuration -->
      <div v-if="selectedNodeData.type === 'transformer'" class="h-full flex flex-col">
        <h3 class="text-sm font-semibold text-gray-300 mb-3">Transformer Configuration</h3>
        <div class="flex-1 min-h-0">
          <TransformerConfig
            v-model="localNodeData.config"
            @update="updateNodeData"
          />
        </div>
      </div>

      <!-- HTTP Request Node Configuration -->
      <div v-if="selectedNodeData.type === 'http-request'">
        <h3 class="text-sm font-semibold text-gray-300 mb-3">HTTP Request Configuration</h3>
        <div class="space-y-4">
          <HttpRequestConfig
            v-model="localNodeData.config"
            @update="updateNodeData"
          />
          <CommonConfigFields
            v-model="localNodeData.config"
            @update="updateNodeData"
          />
        </div>
      </div>

      <!-- OpenObserve Node Configuration -->
      <div v-if="selectedNodeData.type === 'openobserve'">
        <h3 class="text-sm font-semibold text-gray-300 mb-3">OpenObserve Configuration</h3>
        <div class="space-y-4">
          <OpenObserveConfig
            v-model="localNodeData.config"
            @update="updateNodeData"
          />
          <CommonConfigFields
            v-model="localNodeData.config"
            @update="updateNodeData"
          />
        </div>
      </div>


      <!-- Email Node Configuration -->
      <div v-if="selectedNodeData.type === 'email'">
        <h3 class="text-sm font-semibold text-gray-300 mb-3">Email Configuration</h3>
        <EmailConfig
          v-model="localNodeData.config"
          @update="updateNodeData"
        />
      </div>

      <!-- Delay Node Configuration -->
      <div v-if="selectedNodeData.type === 'delay'">
        <h3 class="text-sm font-semibold text-gray-300 mb-3">Delay Configuration</h3>
        <DelayConfig
          v-model="localNodeData.config"
          @update="updateNodeData"
        />
      </div>
    </div>


      </div> <!-- Modal Body -->
    </div> <!-- Modal Content -->
  </div> <!-- Modal Backdrop -->
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { XMarkIcon } from '@heroicons/vue/24/outline'
import { useNodeStore } from '../../stores/nodes'
import CommonConfigFields from '../common/CommonConfigFields.vue'
import TriggerConfig from '../app-configs/TriggerConfig.vue'
import ConditionConfig from '../app-configs/ConditionConfig.vue'
import TransformerConfig from '../app-configs/TransformerConfig.vue'
import DelayConfig from '../app-configs/DelayConfig.vue'
import HttpRequestConfig from '../app-configs/HttpRequestConfig.vue'
import OpenObserveConfig from '../app-configs/OpenObserveConfig.vue'
import EmailConfig from '../email-configs/EmailConfig.vue'
import { debugLog } from '../../utils/debug'

const nodeStore = useNodeStore()

const selectedNodeData = computed(() => nodeStore.selectedNodeData)
const nodeTypeDefinition = computed(() => 
  selectedNodeData.value ? nodeStore.nodeTypeByType(selectedNodeData.value.type) : null
)

interface NodeData {
  label: string
  description: string
  config: any
  status: string
}

const localNodeData = ref<NodeData>({} as NodeData)

// Watch for selected node changes
watch(selectedNodeData, (newNode) => {
  if (newNode) {
    localNodeData.value = JSON.parse(JSON.stringify(newNode.data))
  }
}, { immediate: true })

function updateNodeData() {
  if (selectedNodeData.value) {
    debugLog.component('NodePropertiesPanel', 'updateNodeData', {
      nodeId: selectedNodeData.value.id,
      nodeType: selectedNodeData.value.type,
      hasLocalData: !!localNodeData.value,
      isEmailNode: selectedNodeData.value.type === 'email',
      labelChange: localNodeData.value.label !== selectedNodeData.value.data.label
    })
    
    if (selectedNodeData.value.type === 'email') {
      debugLog.component('NodePropertiesPanel', 'email-node-update', {
        hasEmailConfig: !!localNodeData.value.config,
        hasFrom: !!(localNodeData.value.config as any)?.from,
        hasTo: !!(localNodeData.value.config as any)?.to,
        toCount: (localNodeData.value.config as any)?.to?.length || 0,
        hasCC: !!(localNodeData.value.config as any)?.cc,
        ccCount: (localNodeData.value.config as any)?.cc?.length || 0,
        hasBCC: !!(localNodeData.value.config as any)?.bcc,
        bccCount: (localNodeData.value.config as any)?.bcc?.length || 0
      })
    }
    
    // Get the current node from store
    const currentNode = nodeStore.getNodeById(selectedNodeData.value.id)
    if (currentNode) {
      // Create a completely new node object to force Vue reactivity
      const updatedNode = {
        ...currentNode,
        data: { ...localNodeData.value }
      }
      
      // Force reactivity by replacing the entire node
      nodeStore.updateNode(selectedNodeData.value.id, updatedNode)
      
    }
  }
}


function handleClose() {
  // Update node data before closing
  updateNodeData()
  // Close the modal
  nodeStore.setSelectedNode(null)
}

function deleteNode() {
  if (selectedNodeData.value) {
    const success = nodeStore.deleteNode(selectedNodeData.value.id)
    if (!success) {
      // Could show a toast notification here in the future
      console.warn('Node deletion prevented: Cannot delete trigger nodes')
    }
  }
}


</script>