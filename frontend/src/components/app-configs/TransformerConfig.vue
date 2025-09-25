<template>
  <div class="space-y-4 overflow-y-scroll h-screen max-h-[550px]">
    <!-- 3 Column Grid -->
    <div class="grid grid-cols-3 gap-4 h-full">
      <!-- Input Column -->
      <div class="flex flex-col border-2 border-blue-500/30 rounded-lg p-3 bg-blue-500/5">
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
      <div class="flex flex-col border-2 border-purple-500/30 rounded-lg p-3 bg-purple-500/5">
        <div class="flex items-center justify-between mb-2">
          <label class="block text-sm font-medium text-gray-300">JavaScript Code</label>
          <button
            @click="showAIAssistant = true"
            class="text-xs bg-gradient-to-r from-amber-600 to-orange-600 hover:from-amber-500 hover:to-orange-500 text-white px-2 py-1 rounded transition-all duration-200 flex items-center space-x-1 shadow-sm hover:shadow"
            title="AI Assistant - Generate code from prompt"
          >
            <svg class="h-3 w-3" fill="currentColor" viewBox="0 0 20 20">
              <path fill-rule="evenodd" d="M5 2a1 1 0 011 1v1h1a1 1 0 110 2H6v1a1 1 0 11-2 0V6H3a1 1 0 110-2h1V3a1 1 0 011-1zm0 10a1 1 0 011 1v1h1a1 1 0 110 2H6v1a1 1 0 11-2 0v-1H3a1 1 0 110-2h1v-1a1 1 0 011-1zM12 2a1 1 0 01.967.742L14.146 7.2 17.5 8.134a1 1 0 010 1.732L14.146 10.8l-1.179 4.458a1 1 0 01-1.934 0L9.854 10.8 6.5 9.866a1 1 0 010-1.732L9.854 7.2l1.179-4.458A1 1 0 0112 2z" clip-rule="evenodd" />
            </svg>
            <span>AI</span>
          </button>
        </div>
        <div class="flex-1 min-h-0">
          <CodeEditor
            v-model="localConfig.script"
            :language="'javascript'"
            :show-run-button="true"
            :show-save-button="false"
            :run-loading="runLoading"
            @update:modelValue="onScriptChange"
            @save="$emit('update')"
            @run="executeScript"
          />
        </div>
      </div>

      <!-- Output Column -->
      <div class="flex flex-col border-2 border-green-500/30 rounded-lg p-3 bg-green-500/5">
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

    <!-- AI Assistant Modal -->
    <div
      v-if="showAIAssistant"
      class="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center p-4"
      role="dialog"
      aria-modal="true"
      aria-labelledby="ai-modal-title"
      aria-describedby="ai-modal-description"
      @click.self="cancelAIAssistant"
    >
      <div class="bg-slate-800 rounded-xl border border-slate-700 w-full max-w-6xl shadow-2xl">
        <!-- Modal Header -->
        <div class="flex items-center justify-between p-6 border-b border-slate-700/50">
          <div class="flex items-center space-x-3">
            <div class="w-8 h-8 bg-gradient-to-r from-amber-600 to-orange-600 rounded-lg flex items-center justify-center">
              <svg class="h-4 w-4 text-white" fill="currentColor" viewBox="0 0 20 20">
                <path fill-rule="evenodd" d="M5 2a1 1 0 011 1v1h1a1 1 0 110 2H6v1a1 1 0 11-2 0V6H3a1 1 0 110-2h1V3a1 1 0 011-1zm0 10a1 1 0 011 1v1h1a1 1 0 110 2H6v1a1 1 0 11-2 0v-1H3a1 1 0 110-2h1v-1a1 1 0 011-1zM12 2a1 1 0 01.967.742L14.146 7.2 17.5 8.134a1 1 0 010 1.732L14.146 10.8l-1.179 4.458a1 1 0 01-1.934 0L9.854 10.8 6.5 9.866a1 1 0 010-1.732L9.854 7.2l1.179-4.458A1 1 0 0112 2z" clip-rule="evenodd" />
              </svg>
            </div>
            <div>
              <h3 id="ai-modal-title" class="text-lg font-semibold text-white">AI Code Assistant</h3>
              <p id="ai-modal-description" class="text-sm text-gray-400">Describe what you want your transformer to do</p>
            </div>
          </div>
          <button
            @click="cancelAIAssistant"
            class="text-gray-400 hover:text-gray-200 transition-colors p-2 rounded-md hover:bg-slate-700/30"
            aria-label="Close AI Assistant modal"
          >
            <svg class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <!-- Modal Body -->
        <div class="p-6 grid grid-cols-5 gap-6 h-[32rem]">
          <!-- Left Column - Input Data (wider) -->
          <div class="col-span-3 flex flex-col">
            <div class="flex items-center justify-between mb-3">
              <label class="block text-sm font-medium text-gray-200">
                📄 Current Input Data
              </label>
              <div class="text-xs text-gray-400">
                {{ inputFieldsText }}
              </div>
            </div>
            <div class="flex-1 min-h-0">
              <CodeEditor
                :model-value="inputDataForContext"
                :language="'json'"
                :readonly="true"
                :show-format-button="false"
                :show-save-button="false"
                :show-run-button="false"
              />
            </div>
          </div>

          <!-- Right Column - Configuration and Prompt -->
          <div class="col-span-2 flex flex-col space-y-4">
            <!-- AI Configuration -->
            <div class="bg-slate-900/60 border border-slate-600/60 rounded-lg p-4 space-y-3">
              <div class="flex items-center space-x-2 mb-3">
                <div class="w-4 h-4 bg-gradient-to-r from-amber-500 to-orange-500 rounded-sm"></div>
                <h4 class="text-sm font-medium text-gray-200">⚙️ AI Configuration</h4>
              </div>
              <div class="space-y-3">
                <div>
                  <label class="block text-xs font-medium text-gray-300 mb-1.5">🤖 Provider</label>
                  <select
                    v-model="aiProvider"
                    class="w-full bg-slate-700/80 border border-slate-500 text-gray-100 px-3 py-1.5 rounded-md text-xs focus:outline-none focus:ring-2 focus:ring-amber-500 focus:border-amber-500 transition-all"
                    :disabled="aiGenerating"
                  >
                    <option v-for="(provider, key) in AI_PROVIDERS" :key="key" :value="key">
                      {{ provider.name }}
                    </option>
                  </select>
                </div>
                <div>
                  <label class="block text-xs font-medium text-gray-300 mb-1.5">🧠 Model</label>
                  <select
                    v-model="aiModel"
                    class="w-full bg-slate-700/80 border border-slate-500 text-gray-100 px-3 py-1.5 rounded-md text-xs focus:outline-none focus:ring-2 focus:ring-amber-500 focus:border-amber-500 transition-all"
                    :disabled="aiGenerating"
                  >
                    <option v-for="model in availableModels" :key="model.id" :value="model.id">
                      {{ model.name }}
                    </option>
                  </select>
                </div>
                <div class="grid grid-cols-2 gap-3">
                  <div>
                    <label class="block text-xs font-medium text-gray-300 mb-1.5">🎯 Max Tokens</label>
                    <input
                      v-model.number="aiMaxTokens"
                      type="number"
                      min="100"
                      max="8000"
                      step="100"
                      class="w-full bg-slate-700/80 border border-slate-500 text-gray-100 px-3 py-1.5 rounded-md text-xs focus:outline-none focus:ring-2 focus:ring-amber-500 focus:border-amber-500 transition-all"
                      :disabled="aiGenerating"
                    />
                  </div>
                  <div>
                    <label class="block text-xs font-medium text-gray-300 mb-1.5">🌡️ Temperature</label>
                    <input
                      v-model.number="aiTemperature"
                      type="number"
                      min="0"
                      max="1"
                      step="0.1"
                      class="w-full bg-slate-700/80 border border-slate-500 text-gray-100 px-3 py-1.5 rounded-md text-xs focus:outline-none focus:ring-2 focus:ring-amber-500 focus:border-amber-500 transition-all"
                      :disabled="aiGenerating"
                    />
                  </div>
                </div>
              </div>
            </div>

            <!-- Prompt Input -->
            <div class="flex-1 flex flex-col">
              <div class="flex items-center space-x-2 mb-3">
                <label class="block text-sm font-medium text-gray-200">
                  💬 What should this transformer do?
                </label>
              </div>
              <textarea
                v-model="aiPrompt"
                placeholder="Example: Extract the 'name' field and convert it to uppercase, add a 'processed' timestamp, and filter out any items where status is 'inactive'"
                class="flex-1 w-full bg-slate-700/80 border border-slate-500 text-gray-100 px-4 py-3 rounded-lg focus:outline-none focus:ring-2 focus:ring-amber-500 focus:border-amber-500 resize-none text-sm leading-relaxed transition-all shadow-inner"
                :disabled="aiGenerating"
                aria-label="Describe what your transformer should do"
              />
              <div class="text-xs text-gray-400 mt-2">
                {{ aiPrompt.length }}/500 characters
              </div>
            </div>
          </div>
        </div>

        <!-- Tips Section (moved to bottom) -->
        <div class="px-6 pb-4">
          <div class="bg-gradient-to-r from-amber-900/20 to-orange-900/20 border border-amber-600/30 rounded-lg p-4">
            <div class="flex items-start space-x-3">
              <div class="w-6 h-6 bg-amber-500/20 rounded-full flex items-center justify-center mt-0.5">
                <svg class="h-3 w-3 text-amber-400" fill="currentColor" viewBox="0 0 20 20">
                  <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z" clip-rule="evenodd" />
                </svg>
              </div>
              <div class="text-xs text-amber-100">
                <p class="font-semibold mb-2 text-amber-200">💡 Pro Tips for Better Results:</p>
                <div class="grid grid-cols-3 gap-4">
                  <div>
                    <p class="font-medium text-amber-300">🎯 Be Specific</p>
                    <p class="text-amber-200/80">Describe exact transformations needed</p>
                  </div>
                  <div>
                    <p class="font-medium text-amber-300">✅ Add Validation</p>
                    <p class="text-amber-200/80">Mention data validation rules</p>
                  </div>
                  <div>
                    <p class="font-medium text-amber-300">🔍 Reference Fields</p>
                    <p class="text-amber-200/80">Use field names from input data</p>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- Modal Footer -->
        <div class="flex items-center justify-end space-x-3 p-6 border-t border-slate-700/50">
          <button
            @click="cancelAIAssistant"
            class="px-4 py-2 text-gray-300 hover:text-white border border-slate-600 hover:border-slate-500 rounded-md transition-colors"
            :disabled="aiGenerating"
          >
            Cancel
          </button>
          <button
            @click="generateCode"
            :disabled="!aiPrompt.trim() || aiGenerating"
            class="px-4 py-2 bg-gradient-to-r from-amber-600 to-orange-600 hover:from-amber-500 hover:to-orange-500 disabled:from-gray-600 disabled:to-gray-600 text-white rounded-md transition-all duration-200 flex items-center space-x-2"
            :aria-label="aiGenerating ? 'Generating code, please wait' : 'Generate code from your prompt'"
          >
            <svg v-if="aiGenerating" class="animate-spin h-4 w-4" fill="none" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
            </svg>
            <svg v-else class="h-4 w-4" fill="currentColor" viewBox="0 0 20 20">
              <path fill-rule="evenodd" d="M12 2a1 1 0 01.967.742L14.146 7.2 17.5 8.134a1 1 0 010 1.732L14.146 10.8l-1.179 4.458a1 1 0 01-1.934 0L9.854 10.8 6.5 9.866a1 1 0 010-1.732L9.854 7.2l1.179-4.458A1 1 0 0112 2z" clip-rule="evenodd" />
            </svg>
            <span>{{ aiGenerating ? 'Generating...' : 'Generate Code' }}</span>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, computed, onUnmounted } from 'vue'
import CodeEditor from '../common/CodeEditor.vue'
import { useWorkflowStore } from '../../stores/workflows'
import { useNodeStore } from '../../stores/nodes'
import { apiClient } from '../../services/api'
import { useToast } from '../../composables/useToast'
import { AI_PROVIDERS, DEFAULT_AI_CONFIG, type AIGenerationRequest } from '../../config/ai'
import { getUserFriendlyErrorMessage } from '../../utils/errors'
import { extractGeneratedCode } from '../../utils/codeValidation'

interface Props {
  modelValue: {
    script?: string
  }
  nodeId?: string
}

interface Emits {
  (e: 'update:modelValue', value: { script?: string }): void
  (e: 'update'): void
}

interface WorkflowExecution {
  id: string
  workflow_id: string
  status: string
  created_at: number
  completed_at?: number
  input_data?: unknown
}


const props = defineProps<Props>()
const emit = defineEmits<Emits>()
const workflowStore = useWorkflowStore()
const nodeStore = useNodeStore()
const toast = useToast()

const localConfig = ref({ script: '', ...props.modelValue })
const selectedExecutionId = ref('')
const pastExecutions = ref<WorkflowExecution[]>([])
const inputData = ref('{}')
const outputData = ref('{}')
const loading = ref(false)
const runLoading = ref(false)
const abortController = ref<AbortController | null>(null)

// AI Assistant state
const showAIAssistant = ref(false)
const aiPrompt = ref('')
const aiGenerating = ref(false)
const aiProvider = ref<keyof typeof AI_PROVIDERS>(DEFAULT_AI_CONFIG.provider)
const aiModel = ref<string>(DEFAULT_AI_CONFIG.model)
const aiMaxTokens = ref(DEFAULT_AI_CONFIG.maxTokens)
const aiTemperature = ref(DEFAULT_AI_CONFIG.temperature)

const currentWorkflowId = computed(() => workflowStore.currentWorkflow?.id)

const inputDataForContext = computed(() => {
  if (inputData.value === '{}' || !inputData.value.trim()) {
    return 'No input data selected. Please select an execution from the dropdown above to see sample data.'
  }
  return inputData.value
})

const inputFieldsText = computed(() => {
  try {
    if (inputData.value === '{}' || !inputData.value.trim()) {
      return 'No data'
    }
    const parsed = JSON.parse(inputData.value)
    const fieldCount = Object.keys(parsed).length
    return `${fieldCount} field${fieldCount === 1 ? '' : 's'}`
  } catch {
    return 'Invalid JSON'
  }
})

const parsedInputData = computed(() => {
  try {
    return inputData.value !== '{}' ? JSON.parse(inputData.value) : null
  } catch {
    return null
  }
})

const availableModels = computed(() => {
  return AI_PROVIDERS[aiProvider.value].models
})

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

  // Cancel any existing request
  if (abortController.value) {
    abortController.value.abort()
  }

  abortController.value = new AbortController()
  loading.value = true

  try {
    const data = await apiClient.getExecutionsByWorkflow(
      currentWorkflowId.value,
      DEFAULT_AI_CONFIG.executionsLimit
    )
    pastExecutions.value = data.executions || []
  } catch (error) {
    if (!abortController.value?.signal.aborted) {
      console.error('Error fetching past executions:', error)
      toast.error('Error', 'Failed to fetch past executions')
    }
  } finally {
    if (!abortController.value?.signal.aborted) {
      loading.value = false
    }
  }
}

interface ExecutionStep {
  node_id: string
  output_data?: unknown
}

interface WorkflowEdge {
  source: string
  target: string
  data?: {
    condition_result?: unknown
  }
}

interface PredecessorOutput {
  nodeId: string
  conditionResult: unknown
  data: unknown
}

async function findImmediatePredecessorOutput(nodeId: string, executionSteps: ExecutionStep[]) {
  // Use nodeStore.edges for real-time working state (includes unsaved changes)
  const workingEdges = nodeStore.edges

  if (!workingEdges) {
    console.log('No edges found in node store')
    return null
  }

  // Find all edges leading TO this node
  const incomingEdges = (workingEdges as WorkflowEdge[]).filter(e => e.target === nodeId)

  console.log(`Found ${incomingEdges.length} incoming edges for node ${nodeId}`)
  console.log('Incoming edges:', incomingEdges)
  console.log('Total edges in node store:', workingEdges.length || 0)
  console.log('Total nodes in node store:', nodeStore.nodes?.length || 0)

  if (incomingEdges.length === 0) {
    return null // No predecessors
  }

  // Get output data from immediate predecessors only
  const predecessorOutputs: PredecessorOutput[] = []
  for (const edge of incomingEdges) {
    const predStep = executionSteps.find(step => step.node_id === edge.source)
    if (predStep?.output_data) {
      predecessorOutputs.push({
        nodeId: edge.source,
        conditionResult: edge.data?.condition_result || null,
        data: predStep.output_data
      })
    }
  }

  if (predecessorOutputs.length === 0) {
    return null // Predecessors exist but no output data
  }

  // Handle multiple predecessors
  return selectBestImmediatePredecessor(predecessorOutputs)
}

function selectBestImmediatePredecessor(predecessorOutputs: PredecessorOutput[]) {
  if (predecessorOutputs.length === 1) {
    // Single predecessor: use directly
    return predecessorOutputs[0].data
  }

  // Multiple predecessors: create merged structure for parallel processing
  // or select first available for condition branches
  if (predecessorOutputs.some(p => p.conditionResult !== null)) {
    // This is after condition branches - use any available path
    return predecessorOutputs[0].data
  } else {
    // This is after parallel processing - create merged array
    return predecessorOutputs.map(output => output.data)
  }
}

async function onExecutionSelect() {
  if (!selectedExecutionId.value) {
    inputData.value = '{}'
    outputData.value = '{}'
    return
  }

  try {
    // Always fetch execution steps first to get step-level data
    const stepsResponse = await apiClient.getExecutionSteps(selectedExecutionId.value)

    // Find the specific transformer step using nodeId if available
    let transformerStep = null
    if (props.nodeId) {
      console.log(`Looking for transformer step with node_id: ${props.nodeId}`)
      transformerStep = stepsResponse.steps.find(step => step.node_id === props.nodeId)
    } else {
      console.log('Falling back to name-based transformer step search')
      transformerStep = stepsResponse.steps.find(step =>
        step.node_name.toLowerCase().includes('transformer') ||
        step.node_name.toLowerCase().includes('transform')
      )
    }

    if (transformerStep) {
      // Use step-level input data (output from previous node)
      if (transformerStep.input_data) {
        inputData.value = JSON.stringify(transformerStep.input_data, null, 2)
        console.log('Using step-level input data for transformer')
      } else {
        inputData.value = JSON.stringify({ info: 'No input data available for this transformer step' }, null, 2)
      }

      // Use step-level output data
      if (transformerStep.output_data) {
        outputData.value = JSON.stringify(transformerStep.output_data, null, 2)
      } else {
        outputData.value = JSON.stringify({ info: 'No output data available for this transformer step' }, null, 2)
      }
    } else {
      // Enhanced fallback: try to get immediate predecessor output
      console.log('No transformer step found, looking for immediate predecessor output')

      if (props.nodeId) {
        const predecessorOutput = await findImmediatePredecessorOutput(props.nodeId, stepsResponse.steps)

        if (predecessorOutput) {
          // Use immediate predecessor's output as input data
          inputData.value = JSON.stringify(predecessorOutput, null, 2)
          outputData.value = JSON.stringify({
            info: 'Test data from immediate predecessor node. Output will be generated when you run the script.'
          }, null, 2)
          console.log('Using immediate predecessor output as input data for transformer')
        } else {
          // Show informational message when no predecessor data available
          const incomingEdges = (nodeStore.edges as WorkflowEdge[] | undefined)?.filter(e => e.target === props.nodeId) || []

          inputData.value = JSON.stringify({
            info: `This transformer node '${props.nodeId}' was not present during this execution.`,
            suggestion: 'Please run a new workflow execution to see realistic input data for testing.',
            reason: incomingEdges.length === 0
              ? 'No immediate predecessor nodes found.'
              : 'Immediate predecessor nodes have no output data.'
          }, null, 2)
          outputData.value = JSON.stringify({
            info: 'No execution data available for this node'
          }, null, 2)
        }
      } else {
        // Legacy fallback for name-based matching
        console.log('No transformer step found, falling back to workflow-level data')
        const execution = await apiClient.getExecution(selectedExecutionId.value)

        if (execution.input_data) {
          inputData.value = JSON.stringify(execution.input_data, null, 2)
        } else {
          inputData.value = '{}'
        }

        outputData.value = JSON.stringify({
          info: 'No transformer steps found in this execution'
        }, null, 2)
      }
    }
  } catch (error) {
    console.error('Error fetching execution details:', error)
    inputData.value = JSON.stringify({ error: 'Failed to load execution data' }, null, 2)
    outputData.value = JSON.stringify({ error: 'Failed to load execution data' }, null, 2)
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
    if (!parsedInputData.value) {
      outputData.value = JSON.stringify({ error: 'Invalid input JSON format' }, null, 2)
      toast.error('Error', 'Input data is not valid JSON')
      return
    }

    // Use the API client to execute the script
    const result = await apiClient.executeScript(script, parsedInputData.value)
    outputData.value = JSON.stringify(result, null, 2)

  } catch (error) {
    console.error('Error executing script:', error)

    const errorMessage = getUserFriendlyErrorMessage(error)
    outputData.value = JSON.stringify({
      error: 'Script execution failed',
      details: errorMessage
    }, null, 2)

    toast.error('Execution Failed', errorMessage)
  } finally {
    runLoading.value = false
  }
}

// AI Assistant functions
function cancelAIAssistant() {
  showAIAssistant.value = false
  aiPrompt.value = ''
  aiGenerating.value = false
}


async function generateCode() {
  if (!aiPrompt.value.trim()) return

  aiGenerating.value = true
  try {
    // Create the context for the AI
    const contextData = inputData.value !== '{}' ? inputData.value : null
    const systemPrompt = `You are a JavaScript code generator for data transformation functions.
Generate a transformer function that follows this exact format:

function transformer(event) {
    // Access and modify the event data
    // actual data will be available in event.data

    // Your generated code here based on the user's requirements

    return event; // Return modified event or null to drop
}

Rules:
1. Always use the exact function signature shown above
2. The input data is available in event.data
3. Always return the modified event object or null to drop the event
4. Include helpful comments explaining the transformations
5. Handle edge cases and validate data when appropriate
6. Use modern JavaScript features appropriately
7. Be specific and implement exactly what the user requested`

    const userPrompt = `The user wants to transform data with the following requirements:
"${aiPrompt.value}"

${contextData ? `Here is sample input data to work with:
${contextData}

Please generate a transformer function that works with this data structure and implements the user's requirements.` : 'Please generate a transformer function that implements the user\'s requirements. Since no sample data is available, make reasonable assumptions about the data structure.'}

Remember to return the complete transformer function with the exact signature specified.`

    // Call the AI API with user-selected configuration using apiClient
    const request: AIGenerationRequest = {
      system_prompt: systemPrompt,
      user_prompt: userPrompt,
      model: aiModel.value,
      max_tokens: aiMaxTokens.value,
      temperature: aiTemperature.value
    }

    const result = await apiClient.generateCode(request)

    // Extract and clean up the generated code
    const rawCode = result.response || ''
    const generatedCode = extractGeneratedCode(rawCode)

    // Update the script in the editor
    localConfig.value.script = generatedCode
    onScriptChange(generatedCode)

    // Close the modal
    showAIAssistant.value = false
    aiPrompt.value = ''

    // Show success message
    toast.success('Code Generated', 'AI successfully generated your transformer function!')

  } catch (error) {
    console.error('AI code generation failed:', error)

    const errorMessage = error instanceof Error && error.message.includes('transformer function')
      ? error.message // Use validation error message directly
      : getUserFriendlyErrorMessage(error)

    toast.error('Code Generation Failed', errorMessage, 5000)
  } finally {
    aiGenerating.value = false
  }
}

// Watch for AI provider changes to update model selection
watch(aiProvider, (newProvider) => {
  // Reset to first available model when provider changes
  const availableModelsList = AI_PROVIDERS[newProvider].models
  if (availableModelsList.length > 0) {
    aiModel.value = availableModelsList[0].id
  }
})

// Cleanup on unmount
onUnmounted(() => {
  if (abortController.value) {
    abortController.value.abort()
  }
})

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
</script>