<template>
  <div class="space-y-4">
    <!-- Tabs -->
    <div class="flex border-b border-gray-700 overflow-x-auto">
      <button
        @click="activeTab = 'native'"
        :class="[
          'px-4 py-2 text-sm font-medium transition-colors whitespace-nowrap',
          activeTab === 'native'
            ? 'text-primary-400 border-b-2 border-primary-400'
            : 'text-gray-400 hover:text-gray-300'
        ]"
      >
        Native Endpoints
      </button>
      <button
        @click="activeTab = 'segment'"
        :class="[
          'px-4 py-2 text-sm font-medium transition-colors whitespace-nowrap',
          activeTab === 'segment'
            ? 'text-primary-400 border-b-2 border-primary-400'
            : 'text-gray-400 hover:text-gray-300'
        ]"
      >
        Segment Endpoints
      </button>
      <button
        @click="activeTab = 'test'"
        :class="[
          'px-4 py-2 text-sm font-medium transition-colors whitespace-nowrap',
          activeTab === 'test'
            ? 'text-primary-400 border-b-2 border-primary-400'
            : 'text-gray-400 hover:text-gray-300'
        ]"
      >
        Test
      </button>
    </div>

    <!-- Native Endpoints Tab -->
    <div v-if="activeTab === 'native'" class="space-y-4 overflow-y-auto max-h-[600px]">
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
              <p><strong>curl</strong> -X POST {{ primaryEndpoint }} \</p>
              <p class="ml-4">-H "Content-Type: application/json" \</p>
              <p class="ml-4">-d '{"user_name": "Clark Kent", "user_email": "superman@marvel.com"}'</p>
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
              <p>• No authentication required for native endpoints</p>
              <p>• Supports GET, POST, and PUT methods</p>
              <p>• Batch endpoint processes events concurrently</p>
              <p>• All event data is available in workflow nodes</p>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Segment Endpoints Tab -->
    <div v-if="activeTab === 'segment'" class="space-y-4 overflow-y-auto max-h-[600px]">
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
              <p class="text-purple-300 font-medium mb-1">Track Event:</p>
              <p><strong>curl</strong> -X POST {{ segmentTrackEndpoint }} \</p>
              <p class="ml-4">-H "Authorization: Bearer {{ workflowId }}" \</p>
              <p class="ml-4">-H "Content-Type: application/json" \</p>
              <p class="ml-4">-d '{"userId": "123", "event": "Button Clicked", "properties": {"color": "blue"}}'</p>
            </div>
          </div>
        </div>
      </div>

      <div class="bg-purple-900/20 border border-purple-700/50 p-3 rounded-md">
        <div class="flex items-start space-x-2">
          <div class="text-purple-400 mt-0.5">
            <svg class="h-4 w-4" fill="currentColor" viewBox="0 0 20 20">
              <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z" clip-rule="evenodd" />
            </svg>
          </div>
          <div>
            <p class="text-sm text-purple-300 font-medium">Drop-in Replacement</p>
            <div class="text-xs text-purple-400 mt-1 space-y-1">
              <p>• Compatible with Segment.com HTTP Tracking API</p>
              <p>• Use your workflow ID as the write key</p>
              <p>• No code changes needed for migration</p>
              <p>• Supports all standard Segment event types</p>
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
              <p>• All endpoints require Authorization header or writeKey in body</p>
              <p>• Use workflow ID as the authentication token/writeKey</p>
              <p>• Batch endpoint processes multiple events concurrently</p>
              <p>• Compatible with existing Segment.com SDK integrations</p>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Test Tab -->
    <div v-if="activeTab === 'test'" class="space-y-4 overflow-y-auto max-h-[600px]">
      <div class="bg-slate-800 p-4 rounded-md">
        <h4 class="text-sm font-medium text-gray-300 mb-3">🧪 Test Workflow</h4>

        <!-- Method Selection -->
        <div class="mb-4">
          <label class="block text-sm font-medium text-gray-300 mb-2">HTTP Method</label>
          <select
            v-model="testMethod"
            class="w-full px-3 py-2 bg-slate-700 border border-gray-600 rounded-md text-white text-sm focus:outline-none focus:ring-2 focus:ring-primary-500"
          >
            <option value="POST">POST</option>
            <option value="GET">GET</option>
            <option value="PUT">PUT</option>
          </select>
        </div>

        <!-- Test Data Editor -->
        <div class="mb-4">
          <label class="block text-sm font-medium text-gray-300 mb-2">Test Data (JSON)</label>
          <div class="border border-gray-600 rounded-md overflow-hidden" style="height: 300px;">
            <code-editor
              v-model="testData"
              language="json"
              :show-format-button="true"
              :show-save-button="false"
              :show-run-button="false"
            />
          </div>
        </div>

        <!-- Trigger Button -->
        <div class="flex items-center space-x-3">
          <button
            @click="triggerWorkflow"
            :disabled="isTriggeringWorkflow || !workflowStore.currentWorkflow?.id"
            class="px-4 py-2 bg-primary-600 text-white rounded-md hover:bg-primary-700 disabled:bg-gray-600 disabled:cursor-not-allowed transition-colors text-sm font-medium flex items-center space-x-2"
          >
            <span v-if="isTriggeringWorkflow" class="animate-spin rounded-full h-4 w-4 border-b-2 border-white"></span>
            <span>{{ isTriggeringWorkflow ? 'Triggering...' : 'Trigger Workflow' }}</span>
          </button>
          <button
            v-if="executionId"
            @click="viewExecution"
            class="px-4 py-2 bg-green-600 text-white rounded-md hover:bg-green-700 transition-colors text-sm font-medium"
          >
            View Execution
          </button>
        </div>

        <!-- Response Display -->
        <div v-if="testResponse" class="mt-4">
          <label class="block text-sm font-medium text-gray-300 mb-2">Response</label>
          <div class="bg-slate-900 p-3 rounded-md border border-gray-600">
            <div class="flex items-center justify-between mb-2">
              <span class="text-xs font-medium" :class="testResponseStatus >= 200 && testResponseStatus < 300 ? 'text-green-400' : 'text-red-400'">
                Status: {{ testResponseStatus }}
              </span>
              <button
                @click="copyResponse"
                class="text-xs text-gray-400 hover:text-white transition-colors"
              >
                Copy
              </button>
            </div>
            <pre class="text-xs text-gray-300 overflow-x-auto">{{ testResponse }}</pre>
          </div>
        </div>

        <!-- Error Display -->
        <div v-if="testError" class="mt-4 bg-red-900/20 border border-red-700/50 p-3 rounded-md">
          <p class="text-sm text-red-300 font-medium">Error</p>
          <p class="text-xs text-red-400 mt-1">{{ testError }}</p>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, onMounted } from 'vue'
import { useWorkflowStore } from '../../stores/workflows'
import { apiClient } from '../../services/api'
import { useRouter } from 'vue-router'
import CodeEditor from '../common/CodeEditor.vue'

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

const router = useRouter()
const workflowStore = useWorkflowStore()
// Initialize with production fallback (where frontend and backend are served together)
const apiBaseUrl = ref(import.meta.env.DEV ? 'http://localhost:3700' : window.location.origin)
const isLoadingBaseUrl = ref(true)

// Tab state
const activeTab = ref<'native' | 'segment' | 'test'>('native')

// Test form state
const testMethod = ref<'POST' | 'GET' | 'PUT'>('POST')
const testData = ref('{\n  "user_name": "OpenObserve",\n  "user_email": "hello@openobserve.ai"\n}')
const isTriggeringWorkflow = ref(false)
const testResponse = ref('')
const testResponseStatus = ref(0)
const testError = ref('')
const executionId = ref('')

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

// Test workflow functions
const triggerWorkflow = async () => {
  if (!workflowStore.currentWorkflow?.id) {
    testError.value = 'No workflow ID available. Please save the workflow first.'
    return
  }

  testError.value = ''
  testResponse.value = ''
  testResponseStatus.value = 0
  executionId.value = ''
  isTriggeringWorkflow.value = true

  try {
    // Validate JSON
    let payload: any
    try {
      payload = JSON.parse(testData.value)
    } catch (e) {
      testError.value = 'Invalid JSON: ' + (e as Error).message
      return
    }

    // Construct URL
    const url = `${apiBaseUrl.value}/api/v1/${workflowStore.currentWorkflow.id}/trigger`

    // Make request
    const response = await fetch(url, {
      method: testMethod.value,
      headers: {
        'Content-Type': 'application/json',
      },
      body: testMethod.value !== 'GET' ? JSON.stringify(payload) : undefined,
    })

    testResponseStatus.value = response.status

    // Try to parse response as JSON
    const responseText = await response.text()
    try {
      const responseJson = JSON.parse(responseText)
      testResponse.value = JSON.stringify(responseJson, null, 2)

      // Extract execution_id if present
      if (responseJson.execution_id) {
        executionId.value = responseJson.execution_id
      }
    } catch {
      testResponse.value = responseText
    }

    if (!response.ok) {
      testError.value = `Request failed with status ${response.status}`
    }
  } catch (error) {
    testError.value = 'Request failed: ' + (error as Error).message
  } finally {
    isTriggeringWorkflow.value = false
  }
}

const viewExecution = () => {
  if (executionId.value && workflowStore.currentWorkflow?.id) {
    router.push(`/workflows/${workflowStore.currentWorkflow.id}/executions/${executionId.value}`)
  }
}

const copyResponse = () => {
  navigator.clipboard.writeText(testResponse.value)
}

</script>