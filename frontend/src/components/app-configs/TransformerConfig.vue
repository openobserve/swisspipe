<template>
  <div class="flex flex-col h-full">
    <!-- 3 Column Grid -->
    <div class="grid grid-cols-3 gap-4 h-full">
      <!-- Input Column -->
      <div class="flex flex-col">
        <div class="mb-2">
          <div class="flex items-center justify-between mb-2">
            <label class="block text-sm font-medium text-gray-300">Past Executions</label>
            <button
              @click="fetchPastExecutions"
              :disabled="loading"
              class="text-xs bg-slate-600 hover:bg-slate-500 disabled:bg-slate-800 text-gray-300 px-2 py-1 rounded transition-colors flex items-center space-x-1"
              title="Refresh executions"
            >
              <svg 
                class="h-3 w-3" 
                :class="{ 'animate-spin': loading }"
                fill="none" 
                viewBox="0 0 24 24" 
                stroke="currentColor"
              >
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
              </svg>
              <span>Refresh</span>
            </button>
          </div>
          <select 
            v-model="selectedExecutionId"
            @change="onExecutionSelect"
            class="w-full bg-slate-700 border border-slate-600 text-gray-100 px-3 py-2 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
          >
            <option value="">Select execution...</option>
            <option v-for="execution in pastExecutions" :key="execution.id" :value="execution.id">
              {{ formatExecutionOption(execution) }}
            </option>
          </select>
        </div>
        <label class="block text-sm font-medium text-gray-300 mb-2">Input Data</label>
        <div class="flex-1 min-h-0">
          <CodeEditor
            v-model="inputData"
            :language="'json'"
            :readonly="true"
            :show-format-button="false"
            :show-save-button="false"
          />
        </div>
      </div>

      <!-- Code Column -->
      <div class="flex flex-col">
        <label class="block text-sm font-medium text-gray-300 mb-2">JavaScript Code</label>
        <div class="flex-1 min-h-0">
          <CodeEditor
            v-model="localConfig.script"
            :language="'javascript'"
            :show-run-button="true"
            :run-loading="runLoading"
            @update:modelValue="onScriptChange"
            @save="$emit('update')"
            @run="executeScript"
          />
        </div>
      </div>

      <!-- Output Column -->
      <div class="flex flex-col">
        <label class="block text-sm font-medium text-gray-300 mb-2">Output Preview</label>
        <div class="flex-1 min-h-0">
          <CodeEditor
            v-model="outputData"
            :language="'json'"
            :readonly="true"
            :show-format-button="false"
            :show-save-button="false"
          />
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted, computed } from 'vue'
import CodeEditor from '../common/CodeEditor.vue'
import { useWorkflowStore } from '../../stores/workflows'
import { apiClient } from '../../services/api'

interface Props {
  modelValue: {
    script?: string
  }
}

interface Emits {
  (e: 'update:modelValue', value: any): void
  (e: 'update'): void
}

interface WorkflowExecution {
  id: string
  workflow_id: string
  status: string
  created_at: number
  completed_at?: number
  input_data?: any
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()
const workflowStore = useWorkflowStore()

const localConfig = ref({ script: '', ...props.modelValue })
const selectedExecutionId = ref('')
const pastExecutions = ref<WorkflowExecution[]>([])
const inputData = ref('{}')
const outputData = ref('{}')
const loading = ref(false)
const runLoading = ref(false)

const currentWorkflowId = computed(() => workflowStore.currentWorkflow?.id)

watch(() => props.modelValue, (newValue) => {
  localConfig.value = { script: '', ...newValue }
}, { deep: true })

function onScriptChange(newScript: string) {
  localConfig.value.script = newScript
  emit('update:modelValue', localConfig.value)
}

function formatExecutionOption(execution: WorkflowExecution): string {
  let date = 'Invalid Date'
  
  if (execution.created_at) {
    try {
      // Convert microseconds to milliseconds by dividing by 1000
      const millisecondsTimestamp = execution.created_at / 1000
      const dateObj = new Date(millisecondsTimestamp)
      
      if (!isNaN(dateObj.getTime())) {
        date = dateObj.toLocaleDateString() + ' ' + dateObj.toLocaleTimeString()
      }
    } catch (error) {
      console.warn('Error parsing execution timestamp:', execution.created_at, error)
    }
  }
  
  return `${execution.status} - ${date}`
}

async function fetchPastExecutions() {
  if (!currentWorkflowId.value) {
    console.warn('No current workflow ID available')
    return
  }
  
  loading.value = true
  try {
    const data = await apiClient.getExecutionsByWorkflow(currentWorkflowId.value, 20)
    pastExecutions.value = data.executions || []
  } catch (error) {
    console.error('Error fetching past executions:', error)
  } finally {
    loading.value = false
  }
}

async function onExecutionSelect() {
  if (!selectedExecutionId.value) {
    inputData.value = '{}'
    outputData.value = '{}'
    return
  }

  try {
    const execution = await apiClient.getExecution(selectedExecutionId.value)
    if (execution.input_data) {
      inputData.value = JSON.stringify(execution.input_data, null, 2)
    }
    // TODO: Fetch execution steps to get transformer node output
    outputData.value = '{}'
  } catch (error) {
    console.error('Error fetching execution details:', error)
  }
}

async function executeScript(script: string) {
  if (!script.trim()) {
    outputData.value = JSON.stringify({ error: 'No script provided' }, null, 2)
    return
  }

  if (!inputData.value || inputData.value === '{}') {
    outputData.value = JSON.stringify({ error: 'No input data selected. Please select an execution first.' }, null, 2)
    return
  }

  runLoading.value = true
  try {
    let parsedInput
    try {
      parsedInput = JSON.parse(inputData.value)
    } catch (parseError) {
      outputData.value = JSON.stringify({ error: 'Invalid input JSON format' }, null, 2)
      return
    }

    // Use the API client to execute the script
    const result = await apiClient.executeScript(script, parsedInput)
    outputData.value = JSON.stringify(result, null, 2)

  } catch (error) {
    console.error('Error executing script:', error)
    
    // Handle API client errors
    if (error && typeof error === 'object' && 'response' in error) {
      const apiError = error as any
      outputData.value = JSON.stringify({ 
        error: 'Script execution failed',
        details: apiError.response?.data?.error || apiError.message || 'Unknown error'
      }, null, 2)
    } else {
      outputData.value = JSON.stringify({ 
        error: 'Script execution failed',
        details: error instanceof Error ? error.message : String(error)
      }, null, 2)
    }
  } finally {
    runLoading.value = false
  }
}

// Watch for workflow changes to refetch executions
watch(currentWorkflowId, (newWorkflowId) => {
  if (newWorkflowId) {
    pastExecutions.value = []
    selectedExecutionId.value = ''
    inputData.value = '{}'
    outputData.value = '{}'
    fetchPastExecutions()
  }
}, { immediate: true })

onMounted(() => {
  if (currentWorkflowId.value) {
    fetchPastExecutions()
  }
})
</script>