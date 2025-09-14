<template>
  <div class="flex flex-col h-full">
    <!-- 2 Column Grid -->
    <div class="grid grid-cols-2 gap-4 h-[600px]">
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
      <div class="flex flex-col border-2 border-amber-500/30 rounded-lg p-3 bg-amber-500/5">
        <div class="flex items-center justify-between mb-2">
          <label class="block text-sm font-medium text-gray-300">JavaScript Code</label>
          <button
            @click="showAIAssistant = true"
            class="text-xs bg-gradient-to-r from-amber-600 to-orange-600 hover:from-amber-500 hover:to-orange-500 text-white px-2 py-1 rounded transition-all duration-200 flex items-center space-x-1 shadow-sm hover:shadow"
            title="AI Assistant - Generate condition code from prompt"
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
            @run="executeCondition"
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
              <p id="ai-modal-description" class="text-sm text-gray-400">Describe what your condition should check</p>
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
                {{ inputData !== '{}' ? Object.keys(JSON.parse(inputData)).length + ' fields' : 'No data' }}
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
                  🤔 What should this condition check?
                </label>
              </div>
              <textarea
                v-model="aiPrompt"
                placeholder="Example: Check if user is active and has sufficient balance, or if the request comes from a specific country"
                class="flex-1 w-full bg-slate-700/80 border border-slate-500 text-gray-100 px-4 py-3 rounded-lg focus:outline-none focus:ring-2 focus:ring-amber-500 focus:border-amber-500 resize-none text-sm leading-relaxed transition-all shadow-inner"
                :disabled="aiGenerating"
                aria-label="Describe what your condition should check"
              />
              <div class="text-xs text-gray-400 mt-2">
                {{ aiPrompt.length }}/500 characters
              </div>
            </div>
          </div>
        </div>

        <!-- Tips Section -->
        <div class="px-6 pb-4">
          <div class="bg-gradient-to-r from-amber-900/20 to-orange-900/20 border border-amber-600/30 rounded-lg p-4">
            <div class="flex items-start space-x-3">
              <div class="w-6 h-6 bg-amber-500/20 rounded-full flex items-center justify-center mt-0.5">
                <svg class="h-3 w-3 text-amber-400" fill="currentColor" viewBox="0 0 20 20">
                  <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z" clip-rule="evenodd" />
                </svg>
              </div>
              <div class="text-xs text-amber-100">
                <p class="font-semibold mb-2 text-amber-200">💡 Pro Tips for Better Conditions:</p>
                <div class="grid grid-cols-3 gap-4">
                  <div>
                    <p class="font-medium text-amber-300">🎯 Be Clear</p>
                    <p class="text-amber-200/80">Describe the exact check needed</p>
                  </div>
                  <div>
                    <p class="font-medium text-amber-300">⚖️ Return Boolean</p>
                    <p class="text-amber-200/80">Always return true or false</p>
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
            :aria-label="aiGenerating ? 'Generating code, please wait' : 'Generate condition code from your prompt'"
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
import { ref, watch, onMounted, computed } from 'vue'
import CodeEditor from '../common/CodeEditor.vue'
import { useWorkflowStore } from '../../stores/workflows'
import { apiClient } from '../../services/api'
import { useToast } from '../../composables/useToast'

interface Props {
  modelValue: {
    script?: string
  }
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

interface APIError {
  response?: {
    data?: {
      error?: string
    }
  }
  message?: string
}

// AI Configuration Constants
const AI_PROVIDERS = {
  anthropic: {
    name: 'Anthropic Claude',
    models: [
      { id: 'claude-3-5-sonnet-20241022', name: 'Claude 3.5 Sonnet' },
      { id: 'claude-3-sonnet-20240229', name: 'Claude 3 Sonnet' },
      { id: 'claude-3-haiku-20240307', name: 'Claude 3 Haiku' }
    ]
  }
} as const

const DEFAULT_AI_CONFIG = {
  provider: 'anthropic' as keyof typeof AI_PROVIDERS,
  model: 'claude-3-5-sonnet-20241022',
  maxTokens: 4000,
  temperature: 0.1
} as const

const props = defineProps<Props>()
const emit = defineEmits<Emits>()
const workflowStore = useWorkflowStore()
const toast = useToast()

const localConfig = ref({ script: '', ...props.modelValue })
const selectedExecutionId = ref('')
const pastExecutions = ref<WorkflowExecution[]>([])
const inputData = ref('{}')
const loading = ref(false)
const runLoading = ref(false)

// AI Assistant state
const showAIAssistant = ref(false)
const aiPrompt = ref('')
const aiGenerating = ref(false)
const aiProvider = ref<keyof typeof AI_PROVIDERS>(DEFAULT_AI_CONFIG.provider)
const aiModel = ref(DEFAULT_AI_CONFIG.model)
const aiMaxTokens = ref(DEFAULT_AI_CONFIG.maxTokens)
const aiTemperature = ref(DEFAULT_AI_CONFIG.temperature)

const currentWorkflowId = computed(() => workflowStore.currentWorkflow?.id)

const inputDataForContext = computed(() => {
  if (inputData.value === '{}' || !inputData.value.trim()) {
    return 'No input data selected. Please select an execution from the dropdown above to see sample data.'
  }
  return inputData.value
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
    return
  }

  try {
    const execution = await apiClient.getExecution(selectedExecutionId.value)
    if (execution.input_data) {
      inputData.value = JSON.stringify(execution.input_data, null, 2)
    }
  } catch (error) {
    console.error('Error fetching execution details:', error)
  }
}

async function executeCondition(script: string) {
  if (!script.trim()) {
    toast.error('No Script', 'No condition script provided')
    return
  }

  if (!inputData.value || inputData.value === '{}') {
    toast.error('No Input Data', 'No input data selected. Please select an execution first.')
    return
  }

  runLoading.value = true
  try {
    let parsedInput
    try {
      parsedInput = JSON.parse(inputData.value)
    } catch {
      toast.error('Invalid JSON', 'Input data is not valid JSON format')
      return
    }

    // Use the API client to execute the condition script
    const result = await apiClient.executeScript(script, parsedInput)

    // For conditions, we expect a boolean result
    const conditionResult = Boolean(result)
    toast.success('Condition Result', `Condition evaluated to: ${conditionResult}`, 3000)

  } catch (error) {
    console.error('Error executing condition:', error)

    // Handle API client errors with improved type safety
    if (error && typeof error === 'object' && 'response' in error) {
      const apiError = error as APIError
      toast.error('Script Error', apiError.response?.data?.error || apiError.message || 'Unknown error')
    } else {
      toast.error('Script Error', error instanceof Error ? error.message : String(error))
    }
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

// Extract JavaScript condition function from AI response
function extractGeneratedCode(rawCode: string): string {
  if (!rawCode || typeof rawCode !== 'string') {
    throw new Error('No code content received from AI')
  }

  let generatedCode = rawCode.trim()

  // Extract code from markdown blocks first (most common case)
  const markdownMatch = generatedCode.match(/```(?:javascript|js)?\s*\n?([\s\S]*?)\n?```/)
  if (markdownMatch && markdownMatch[1]) {
    generatedCode = markdownMatch[1].trim()
  }

  // Find the condition function using manual brace matching
  const functionStartMatch = generatedCode.match(/function\s+condition\s*\([^)]*\)\s*\{/)
  if (functionStartMatch) {
    const startIndex = generatedCode.indexOf(functionStartMatch[0])
    const openBraceIndex = startIndex + functionStartMatch[0].lastIndexOf('{')
    let braceCount = 0
    let functionEnd = -1
    let inString = false
    let stringChar = ''

    // Parse character by character to find matching closing brace
    for (let i = openBraceIndex; i < generatedCode.length; i++) {
      const char = generatedCode[i]
      const prevChar = i > 0 ? generatedCode[i - 1] : ''

      // Handle string literals to avoid counting braces inside strings
      if ((char === '"' || char === "'" || char === '`') && prevChar !== '\\') {
        if (!inString) {
          inString = true
          stringChar = char
        } else if (char === stringChar) {
          inString = false
          stringChar = ''
        }
      }

      // Count braces only outside of string literals
      if (!inString) {
        if (char === '{') {
          braceCount++
        } else if (char === '}') {
          braceCount--
          if (braceCount === 0) {
            functionEnd = i + 1
            break
          }
        }
      }
    }

    if (functionEnd > -1) {
      generatedCode = generatedCode.substring(startIndex, functionEnd).trim()
    }
  } else {
    // Fallback: try to wrap content if no complete function found
    const bodyMatch = generatedCode.match(/\{([\s\S]*)\}$/)
    if (bodyMatch) {
      generatedCode = `function condition(event) {\n    ${bodyMatch[1].trim()}\n}`
    } else if (!generatedCode.startsWith('function condition')) {
      generatedCode = `function condition(event) {\n    ${generatedCode}\n    return true;\n}`
    }
  }

  // Validate the result contains a condition function
  if (!generatedCode.includes('function condition')) {
    throw new Error('Generated code does not contain a valid condition function')
  }

  return generatedCode
}

async function generateCode() {
  if (!aiPrompt.value.trim()) return

  aiGenerating.value = true
  try {
    // Create the context for the AI
    const contextData = inputData.value !== '{}' ? inputData.value : null
    const systemPrompt = `You are a JavaScript code generator for conditional logic functions.
Generate a condition function that follows this exact format:

function condition(event) {
    // Access the event data
    // actual data will be available in event.data

    // Your generated condition logic here based on the user's requirements
    // Must return a boolean (true or false)

    return true; // or false based on condition evaluation
}

Rules:
1. Always use the exact function signature shown above
2. The input data is available in event.data
3. Always return a boolean value (true or false)
4. Include helpful comments explaining the condition logic
5. Handle edge cases and validate data when appropriate
6. Use modern JavaScript features appropriately
7. Be specific and implement exactly what the user requested
8. Consider null/undefined values and provide safe defaults`

    const userPrompt = `The user wants to create a condition that checks:
"${aiPrompt.value}"

${contextData ? `Here is sample input data to work with:
${contextData}

Please generate a condition function that works with this data structure and implements the user's requirements.` : 'Please generate a condition function that implements the user\'s requirements. Since no sample data is available, make reasonable assumptions about the data structure.'}

Remember to return the complete condition function with the exact signature specified and ensure it returns a boolean value.`

    // Call the AI API with user-selected configuration using apiClient
    const result = await apiClient.generateCode({
      system_prompt: systemPrompt,
      user_prompt: userPrompt,
      model: aiModel.value,
      max_tokens: aiMaxTokens.value,
      temperature: aiTemperature.value
    })

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
    toast.success('Code Generated', 'AI successfully generated your condition function!')

  } catch (error) {
    console.error('AI code generation failed:', error)

    // Show user-friendly error with toast notification
    const errorTitle = 'Code Generation Failed'
    let errorMessage = 'Unable to generate code. '

    if (error instanceof Error) {
      if (error.message.includes('fetch') || error.message.includes('network')) {
        errorMessage = 'Please check your network connection and try again.'
      } else if (error.message.includes('ANTHROPIC_API_KEY') || error.message.includes('401')) {
        errorMessage = 'AI service is not properly configured. Please contact your administrator.'
      } else if (error.message.includes('429')) {
        errorMessage = 'Too many requests. Please wait a moment and try again.'
      } else if (error.message.includes('condition function')) {
        errorMessage = error.message // Use our custom extraction error message
      } else {
        errorMessage = `${error.message}. Please try rephrasing your request.`
      }
    }

    toast.error(errorTitle, errorMessage, 5000)
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

// Watch for workflow changes to refetch executions
watch(currentWorkflowId, (newWorkflowId) => {
  if (newWorkflowId) {
    pastExecutions.value = []
    selectedExecutionId.value = ''
    inputData.value = '{}'
    fetchPastExecutions()
  }
}, { immediate: true })

onMounted(() => {
  if (currentWorkflowId.value) {
    fetchPastExecutions()
  }
})
</script>