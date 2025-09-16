<template>
  <div class="space-y-4 overflow-y-scroll h-screen max-h-[600px]">

    <!-- SwissPipe Native Endpoints -->
    <div class="bg-slate-800 p-4 rounded-md">
      <h4 class="text-sm font-medium text-gray-300 mb-3">📡 SwissPipe Native Endpoints</h4>
      <div v-if="isLoadingBaseUrl" class="text-xs text-gray-400 flex items-center space-x-2">
        <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-primary-500"></div>
        <span>Loading endpoint URLs...</span>
      </div>
      <div v-else class="text-xs text-gray-400 space-y-2">
        <div class="bg-slate-700 p-2 rounded">
          <p><strong>Single Event:</strong> <code class="text-green-400">{{ primaryEndpoint }}</code></p>
          <p class="text-gray-500">Methods: GET, POST, PUT</p>
        </div>
        <div class="bg-slate-700 p-2 rounded">
          <p><strong>Batch Events:</strong> <code class="text-green-400">{{ batchEndpoint }}</code></p>
          <p class="text-gray-500">Method: POST (JSON array)</p>
        </div>
      </div>
    </div>

    <!-- Segment.com Compatible Endpoints -->
    <div class="bg-purple-900/20 border border-purple-700/50 p-4 rounded-md">
      <h4 class="text-sm font-medium text-purple-300 mb-3">🔗 Segment.com Compatible Endpoints</h4>
      <div v-if="isLoadingBaseUrl" class="text-xs text-purple-200 flex items-center space-x-2">
        <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-purple-500"></div>
        <span>Loading endpoint URLs...</span>
      </div>
      <div v-else class="text-xs text-purple-200 space-y-2">
        <div class="grid grid-cols-1 gap-2">
          <div v-for="endpoint in segmentEndpoints" :key="endpoint.name" class="bg-purple-800/30 p-2 rounded">
            <div class="flex justify-between items-start">
              <div>
                <p><strong>{{ endpoint.name }}:</strong> <code class="text-purple-300">{{ endpoint.url }}</code></p>
                <p class="text-purple-400 text-xs">{{ endpoint.description }}</p>
              </div>
              <span class="text-purple-400 text-xs font-mono">POST</span>
            </div>
          </div>
        </div>
        <div class="mt-3 pt-2 border-t border-purple-700/50">
          <p class="text-purple-300 font-medium">Authentication:</p>
          <div class="mt-1 space-y-1">
            <p>• <strong>Header:</strong> <code>Authorization: Bearer {{ workflowId }}</code></p>
            <p>• <strong>Body:</strong> <code>"writeKey": "{{ workflowId }}"</code></p>
          </div>
        </div>
      </div>
    </div>

    <div class="bg-green-900/20 border border-green-700/50 p-3 rounded-md">
      <div class="flex items-start space-x-2">
        <div class="text-green-400 mt-0.5">
          <svg class="h-4 w-4" fill="currentColor" viewBox="0 0 20 20">
            <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clip-rule="evenodd" />
          </svg>
        </div>
        <div>
          <p class="text-sm text-green-300 font-medium">Ready to Use</p>
          <p class="text-xs text-green-400 mt-1">
            This trigger is the entry point for your workflow. Save the workflow to activate the endpoint.
          </p>
        </div>
      </div>
    </div>

    <div class="bg-blue-900/20 border border-blue-700/50 p-3 rounded-md">
      <div class="flex items-start space-x-2">
        <div class="text-blue-400 mt-0.5">
          <svg class="h-4 w-4" fill="currentColor" viewBox="0 0 20 20">
            <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z" clip-rule="evenodd" />
          </svg>
        </div>
        <div>
          <p class="text-sm text-blue-300 font-medium">Example Usage</p>
          <div class="text-xs text-blue-400 mt-1 space-y-2">
            <div>
              <p class="text-blue-300 font-medium mb-1">SwissPipe Native:</p>
              <p><strong>curl</strong> -X POST {{ primaryEndpoint }} \</p>
              <p class="ml-4">-H "Content-Type: application/json" \</p>
              <p class="ml-4">-d '{"key": "value"}'</p>
            </div>
            <div>
              <p class="text-purple-300 font-medium mb-1">Segment.com Compatible:</p>
              <p><strong>curl</strong> -X POST {{ segmentTrackEndpoint }} \</p>
              <p class="ml-4">-H "Authorization: Bearer {{ workflowId }}" \</p>
              <p class="ml-4">-H "Content-Type: application/json" \</p>
              <p class="ml-4">-d '{"userId": "123", "event": "Button Clicked"}'</p>
            </div>
          </div>
        </div>
      </div>
    </div>

    <div class="bg-amber-900/20 border border-amber-700/50 p-3 rounded-md">
      <div class="flex items-start space-x-2">
        <div class="text-amber-400 mt-0.5">
          <svg class="h-4 w-4" fill="currentColor" viewBox="0 0 20 20">
            <path fill-rule="evenodd" d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z" clip-rule="evenodd" />
          </svg>
        </div>
        <div>
          <p class="text-sm text-amber-300 font-medium">Important Notes</p>
          <div class="text-xs text-amber-400 mt-1 space-y-1">
            <p>• Trigger nodes cannot be deleted from workflows</p>
            <p>• Every workflow must have exactly one trigger node</p>
            <p>• All endpoints route to the same workflow execution engine</p>
            <p>• Segment.com endpoints use the workflow ID as the write key</p>
            <p>• SwissPipe native endpoints require no authentication</p>
          </div>
        </div>
      </div>
    </div>

    <!-- API Integration Tips -->
    <div class="bg-indigo-900/20 border border-indigo-700/50 p-3 rounded-md">
      <div class="flex items-start space-x-2">
        <div class="text-indigo-400 mt-0.5">
          <svg class="h-4 w-4" fill="currentColor" viewBox="0 0 20 20">
            <path fill-rule="evenodd" d="M3 4a1 1 0 011-1h12a1 1 0 110 2H4a1 1 0 01-1-1zm0 4a1 1 0 011-1h12a1 1 0 110 2H4a1 1 0 01-1-1zm0 4a1 1 0 011-1h12a1 1 0 110 2H4a1 1 0 01-1-1zm0 4a1 1 0 011-1h12a1 1 0 110 2H4a1 1 0 01-1-1z" clip-rule="evenodd" />
          </svg>
        </div>
        <div>
          <p class="text-sm text-indigo-300 font-medium">Integration Tips</p>
          <div class="text-xs text-indigo-400 mt-1 space-y-1">
            <p>• Use <strong>SwissPipe native</strong> endpoints for custom integrations</p>
            <p>• Use <strong>Segment.com compatible</strong> endpoints to drop-in replace Segment</p>
            <p>• Batch endpoints process events concurrently for better performance</p>
            <p>• All event data is available in workflow transformers and conditions</p>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, onMounted } from 'vue'
import { useWorkflowStore } from '../../stores/workflows'
import { apiClient } from '../../services/api'

interface TriggerConfig {
  type: 'trigger'
  methods: string[]
}

interface Props {
  modelValue: TriggerConfig
}

interface Emits {
  (e: 'update:modelValue', value: TriggerConfig): void
  (e: 'update'): void
}

defineProps<Props>()
defineEmits<Emits>()

const workflowStore = useWorkflowStore()
// Initialize with production fallback (where frontend and backend are served together)
const apiBaseUrl = ref(import.meta.env.DEV ? 'http://localhost:3700' : window.location.origin)
const isLoadingBaseUrl = ref(true)

const workflowId = computed(() => workflowStore.currentWorkflow?.id || '{workflow_id}')

// Load API base URL setting for external use
const loadApiBaseUrl = async () => {
  try {
    const setting = await apiClient.getSetting('api_base_url')
    if (setting.value) {
      // Use the configured API base URL if set
      apiBaseUrl.value = setting.value
    } else {
      // Fallback: in dev use localhost:3700, in production use browser origin
      apiBaseUrl.value = import.meta.env.DEV ? 'http://localhost:3700' : window.location.origin
    }
  } catch (error) {
    console.warn('Failed to load API base URL setting, using fallback:', error)
    // Fallback: in dev use localhost:3700, in production use browser origin
    apiBaseUrl.value = import.meta.env.DEV ? 'http://localhost:3700' : window.location.origin
  } finally {
    isLoadingBaseUrl.value = false
  }
}

onMounted(() => {
  loadApiBaseUrl()
})

// SwissPipe Native Endpoints
const primaryEndpoint = computed(() => `${apiBaseUrl.value}/api/v1/${workflowId.value}/trigger`)
const batchEndpoint = computed(() => `${apiBaseUrl.value}/api/v1/${workflowId.value}/json_array`)

// Segment.com Compatible Endpoints
const segmentEndpoints = computed(() => [
  {
    name: 'Track Events',
    url: `${apiBaseUrl.value}/api/v1/track`,
    description: 'Track user actions and events'
  },
  {
    name: 'Identify Users',
    url: `${apiBaseUrl.value}/api/v1/identify`,
    description: 'Identify users with traits and properties'
  },
  {
    name: 'Page Views',
    url: `${apiBaseUrl.value}/api/v1/page`,
    description: 'Track page views and navigation'
  },
  {
    name: 'Screen Views',
    url: `${apiBaseUrl.value}/api/v1/screen`,
    description: 'Track mobile app screen views'
  },
  {
    name: 'Group Users',
    url: `${apiBaseUrl.value}/api/v1/group`,
    description: 'Associate users with groups or organizations'
  },
  {
    name: 'Alias Users',
    url: `${apiBaseUrl.value}/api/v1/alias`,
    description: 'Create user aliases and merge identities'
  },
  {
    name: 'Batch Events',
    url: `${apiBaseUrl.value}/api/v1/batch`,
    description: 'Send multiple events in a single request'
  },
  {
    name: 'Import Data',
    url: `${apiBaseUrl.value}/api/v1/import`,
    description: 'Import historical data in batch format'
  }
])

const segmentTrackEndpoint = computed(() => `${apiBaseUrl.value}/api/v1/track`)

</script>