<template>
  <div class="p-4 bg-transparent" v-if="selectedNodeData">
    <div class="flex items-center justify-between mb-4">
      <h2 class="text-lg font-semibold text-white">Node Properties</h2>
      <button
        @click="nodeStore.setSelectedNode(null)"
        class="text-gray-400 hover:text-gray-200 transition-colors"
      >
        <XMarkIcon class="h-5 w-5" />
      </button>
    </div>

    <!-- Node Basic Info -->
    <div class="mb-6">
      <div class="flex items-center space-x-3 mb-4">
        <div
          class="w-4 h-4 rounded-full"
          :style="{ backgroundColor: nodeTypeDefinition?.color || '#6b7280' }"
        ></div>
        <span class="text-sm font-medium text-gray-300">{{ nodeTypeDefinition?.label || 'Node' }}</span>
      </div>
      
      <div class="space-y-4">
        <div>
          <label class="block text-sm font-medium text-gray-300 mb-2">Node Name</label>
          <input
            v-model="localNodeData.label"
            @blur="updateNodeData"
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
            @blur="updateNodeData"
            rows="2"
            class="w-full bg-slate-700 border border-slate-600 text-gray-100 px-3 py-2 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
          ></textarea>
        </div>
      </div>
    </div>

    <!-- Node-specific Configuration -->
    <div class="space-y-6">
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
        <div>
          <label class="block text-sm font-medium text-gray-300 mb-2">JavaScript Code</label>
          <div class="h-80">
            <CodeEditor
              v-model="localNodeData.config.script"
              :language="'javascript'"
              @update:modelValue="onScriptChange"
              @save="updateNodeData"
            />
          </div>
        </div>
      </div>

      <!-- Transformer Node Configuration -->
      <div v-if="selectedNodeData.type === 'transformer'">
        <h3 class="text-sm font-semibold text-gray-300 mb-3">Transformer Configuration</h3>
        <div>
          <label class="block text-sm font-medium text-gray-300 mb-2">JavaScript Code</label>
          <div class="h-80">
            <CodeEditor
              v-model="localNodeData.config.script"
              :language="'javascript'"
              @update:modelValue="onScriptChange"
              @save="updateNodeData"
            />
          </div>
        </div>
      </div>

      <!-- App Node Configuration -->
      <div v-if="selectedNodeData.type === 'app'">
        <h3 class="text-sm font-semibold text-gray-300 mb-3">App Configuration</h3>
        <div class="space-y-4">
          <div>
            <label class="block text-sm font-medium text-gray-300 mb-2">App Type</label>
            <select
              v-model="localNodeData.config.app_type"
              @change="updateNodeData"
              class="w-full bg-slate-700 border border-slate-600 text-gray-100 px-3 py-2 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
            >
              <option value="Webhook">Webhook</option>
              <option value="OpenObserve">OpenObserve</option>
            </select>
          </div>
          
          <!-- App-specific configuration -->
          <div class="mt-4">
            <WebhookConfig
              v-if="localNodeData.config.app_type === 'Webhook'"
              v-model="localNodeData.config"
              @update="updateNodeData"
            />
            
            <OpenObserveConfig
              v-else-if="localNodeData.config.app_type === 'OpenObserve'"
              v-model="localNodeData.config"
              @update="updateNodeData"
            />
          </div>
          
          <div>
            <label class="block text-sm font-medium text-gray-300 mb-2">Timeout (seconds)</label>
            <input
              v-model.number="localNodeData.config.timeout_seconds"
              @blur="updateNodeData"
              type="number"
              min="1"
              max="300"
              class="w-full bg-slate-700 border border-slate-600 text-gray-100 px-3 py-2 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
            />
          </div>
          
          <div>
            <label class="block text-sm font-medium text-gray-300 mb-2">On Failure</label>
            <select
              v-model="localNodeData.config.failure_action"
              @change="updateNodeData"
              class="w-full bg-slate-700 border border-slate-600 text-gray-100 px-3 py-2 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
            >
              <option value="Stop">Stop Workflow</option>
              <option value="Continue">Continue to Next Node</option>
              <option value="Retry">Retry This Node</option>
            </select>
          </div>
          
          <div v-if="localNodeData.config.failure_action === 'Retry'">
            <label class="block text-sm font-medium text-gray-300 mb-2">Retry Attempts</label>
            <input
              v-model.number="localNodeData.config.retry_config.max_attempts"
              @blur="updateNodeData"
              type="number"
              min="1"
              max="10"
              class="w-full bg-slate-700 border border-slate-600 text-gray-100 px-3 py-2 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
            />
          </div>
        </div>
      </div>

      <!-- Email Node Configuration -->
      <div v-if="selectedNodeData.type === 'email'">
        <h3 class="text-sm font-semibold text-gray-300 mb-3">Email Configuration</h3>
        <EmailConfig
          v-model="localNodeData.config"
          @update:modelValue="updateNodeData"
        />
      </div>
    </div>

    <!-- Actions -->
    <div v-if="selectedNodeData?.type !== 'trigger'" class="mt-8 pt-4 border-t border-slate-600">
      <button
        @click="deleteNode"
        class="w-full bg-red-600 hover:bg-red-700 text-white px-4 py-2 rounded-md font-medium transition-colors"
      >
        Delete Node
      </button>
    </div>
    
    <!-- Info for trigger nodes -->
    <div v-else class="mt-8 pt-4 border-t border-slate-600">
      <div class="flex items-center space-x-2 text-sm text-gray-400">
        <svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
        <span>Trigger nodes cannot be deleted</span>
      </div>
    </div>

  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { XMarkIcon } from '@heroicons/vue/24/outline'
import { useNodeStore } from '../../stores/nodes'
import CodeEditor from '../common/CodeEditor.vue'
import TriggerConfig from '../app-configs/TriggerConfig.vue'
import WebhookConfig from '../app-configs/WebhookConfig.vue'
import OpenObserveConfig from '../app-configs/OpenObserveConfig.vue'
import EmailConfig from '../email-configs/EmailConfig.vue'

const nodeStore = useNodeStore()

const selectedNodeData = computed(() => nodeStore.selectedNodeData)
const nodeTypeDefinition = computed(() => 
  selectedNodeData.value ? nodeStore.nodeTypeByType(selectedNodeData.value.type) : null
)

const localNodeData = ref<any>({})

// Watch for selected node changes
watch(selectedNodeData, (newNode) => {
  if (newNode) {
    localNodeData.value = JSON.parse(JSON.stringify(newNode.data))
  }
}, { immediate: true })

function updateNodeData() {
  if (selectedNodeData.value) {
    nodeStore.updateNode(selectedNodeData.value.id, {
      data: localNodeData.value
    })
  }
}

function onScriptChange(newScript: string) {
  console.log('Script changed to:', newScript)
  if (localNodeData.value.config) {
    localNodeData.value.config.script = newScript
    console.log('Updated localNodeData.config.script:', localNodeData.value.config.script)
    // Immediately update the node store so changes are reflected in saves
    updateNodeData()
  } else {
    console.error('localNodeData.config is undefined!')
  }
}

function deleteNode() {
  if (selectedNodeData.value) {
    const success = nodeStore.deleteNode(selectedNodeData.value.id)
    if (!success) {
      // Could show a toast notification here in the future
      console.log('Node deletion prevented')
    }
  }
}


</script>