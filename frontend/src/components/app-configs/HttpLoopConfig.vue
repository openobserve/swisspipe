<template>
  <div class="mt-6 border-t border-slate-600 pt-4">
    <div class="flex items-center justify-between mb-4">
      <h4 class="text-sm font-medium text-gray-300">Loop Configuration</h4>
      <button
        @click="toggleLoop"
        :class="[
          'px-3 py-1 text-xs rounded-full transition-colors',
          isLoopEnabled
            ? 'bg-primary-600 text-white'
            : 'bg-slate-600 text-gray-300 hover:bg-slate-500'
        ]"
      >
        {{ isLoopEnabled ? 'Enabled' : 'Disabled' }}
      </button>
    </div>

    <div v-if="isLoopEnabled" class="space-y-4">
      <!-- Basic Loop Settings -->
      <div class="grid grid-cols-2 gap-4">
        <div>
          <label class="block text-sm text-gray-300 mb-1">Max Iterations</label>
          <input
            :value="loopConfig.max_iterations || ''"
            @input="updateLoopConfig('max_iterations', ($event.target as HTMLInputElement).value ? parseInt(($event.target as HTMLInputElement).value) : undefined)"
            type="number"
            min="1"
            placeholder="Unlimited"
            class="w-full bg-slate-700 border border-slate-600 text-gray-100 px-3 py-2 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
          />
          <p class="text-xs text-gray-400 mt-1">Leave empty for unlimited</p>
        </div>

        <div>
          <label class="block text-sm text-gray-300 mb-1">Interval (seconds)</label>
          <input
            :value="loopConfig.interval_seconds"
            @input="updateLoopConfig('interval_seconds', parseInt(($event.target as HTMLInputElement).value) || 60)"
            type="number"
            min="1"
            class="w-full bg-slate-700 border border-slate-600 text-gray-100 px-3 py-2 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
          />
        </div>
      </div>

      <!-- Backoff Strategy -->
      <div>
        <label class="block text-sm text-gray-300 mb-2">Backoff Strategy</label>
        <select
          :value="backoffType"
          @change="updateBackoffType(($event.target as HTMLSelectElement).value as 'Fixed' | 'Exponential')"
          class="w-full bg-slate-700 border border-slate-600 text-gray-100 px-3 py-2 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
        >
          <option value="Fixed">Fixed Interval</option>
          <option value="Exponential">Exponential Backoff</option>
        </select>
      </div>

      <!-- Backoff Configuration -->
      <div v-if="backoffType === 'Fixed'" class="bg-slate-800 rounded-md p-3 border border-slate-700">
        <div>
          <label class="block text-sm text-gray-300 mb-1">Fixed Interval (seconds)</label>
          <input
            :value="getFixedInterval()"
            @input="updateFixedInterval(parseInt(($event.target as HTMLInputElement).value) || 60)"
            type="number"
            min="1"
            class="w-full bg-slate-700 border border-slate-600 text-gray-100 px-3 py-2 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
          />
        </div>
      </div>

      <div v-else-if="backoffType === 'Exponential'" class="bg-slate-800 rounded-md p-3 border border-slate-700">
        <div class="grid grid-cols-3 gap-3">
          <div>
            <label class="block text-sm text-gray-300 mb-1">Base (seconds)</label>
            <input
              :value="getExponentialConfig().base"
              @input="updateExponentialConfig('base', parseInt(($event.target as HTMLInputElement).value) || 1)"
              type="number"
              min="1"
              class="w-full bg-slate-700 border border-slate-600 text-gray-100 px-3 py-2 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
            />
          </div>
          <div>
            <label class="block text-sm text-gray-300 mb-1">Multiplier</label>
            <input
              :value="getExponentialConfig().multiplier"
              @input="updateExponentialConfig('multiplier', parseFloat(($event.target as HTMLInputElement).value) || 2.0)"
              type="number"
              step="0.1"
              min="1"
              class="w-full bg-slate-700 border border-slate-600 text-gray-100 px-3 py-2 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
            />
          </div>
          <div>
            <label class="block text-sm text-gray-300 mb-1">Max (seconds)</label>
            <input
              :value="getExponentialConfig().max"
              @input="updateExponentialConfig('max', parseInt(($event.target as HTMLInputElement).value) || 300)"
              type="number"
              min="1"
              class="w-full bg-slate-700 border border-slate-600 text-gray-100 px-3 py-2 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
            />
          </div>
        </div>
        <p class="text-xs text-gray-400 mt-2">
          Exponential backoff: delay = base × multiplier^iteration (capped at max)
        </p>
      </div>

      <!-- Single Termination Condition -->
      <div>
        <label class="block text-sm font-medium text-gray-300 mb-3">Termination Condition</label>

        <div class="bg-slate-800 rounded-md p-3 border border-slate-700">
          <!-- JavaScript function editor and action selector side by side -->
          <div class="grid grid-cols-3 gap-4">
            <!-- Code editor (left side, takes 2/3 width) -->
            <div class="col-span-2 border-2 border-slate-600 rounded-lg p-3 bg-slate-800/50">
              <label class="block text-xs text-gray-400 mb-1">Termination Function</label>
              <div class="h-64">
                <CodeEditor
                  :modelValue="loopConfig.termination_condition?.script || ''"
                  @update:modelValue="updateTerminationCondition('script', $event)"
                  language="javascript"
                />
              </div>
              <div class="mt-1 space-y-1">
                <p class="text-xs text-gray-500">
                  Function receives event object with data.metadata containing numeric loop values (http_status, loop_iteration, consecutive_failures, elapsed_seconds)
                </p>
                <div v-if="scriptValidation && !scriptValidation.isValid" class="text-xs text-amber-400 bg-amber-900/20 px-2 py-1 rounded border border-amber-800">
                  <div class="flex items-center gap-1">
                    <svg class="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
                      <path fill-rule="evenodd" d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z" clip-rule="evenodd" />
                    </svg>
                    <span class="font-medium">JavaScript Validation Issues:</span>
                  </div>
                  <ul class="mt-1 ml-4 list-disc list-inside space-y-0.5">
                    <li v-for="(error, index) in scriptValidation.errors" :key="index">{{ error }}</li>
                  </ul>
                </div>
              </div>
            </div>

            <!-- Action selector (right side, takes 1/3 width) -->
            <div>
              <label class="block text-xs text-gray-400 mb-1">Action</label>
              <select
                :value="loopConfig.termination_condition?.action || 'Success'"
                @change="updateTerminationCondition('action', ($event.target as HTMLSelectElement).value)"
                class="w-full bg-slate-700 border border-slate-600 text-gray-100 px-2 py-1 rounded text-sm focus:outline-none focus:ring-2 focus:ring-primary-500"
              >
                <option value="Success">Success</option>
                <option value="Failure">Failure</option>
                <option value="Stop">Stop</option>
              </select>
            </div>
          </div>
        </div>
      </div>

      <!-- Quick Setup Templates -->
      <div class="border-t border-slate-700 pt-4">
        <label class="block text-sm font-medium text-gray-300 mb-2">Quick Setup Templates</label>
        <div class="flex gap-2 flex-wrap">
          <button
            @click="applyTemplate('customer-onboarding')"
            class="px-3 py-2 bg-slate-700 hover:bg-slate-600 text-gray-300 text-xs rounded-md transition-colors"
          >
            Customer Onboarding
          </button>
          <button
            @click="applyTemplate('health-monitoring')"
            class="px-3 py-2 bg-slate-700 hover:bg-slate-600 text-gray-300 text-xs rounded-md transition-colors"
          >
            Health Monitoring
          </button>
          <button
            @click="applyTemplate('data-sync')"
            class="px-3 py-2 bg-slate-700 hover:bg-slate-600 text-gray-300 text-xs rounded-md transition-colors"
          >
            Data Sync
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import CodeEditor from '../common/CodeEditor.vue'
import type { LoopConfig, BackoffStrategy, TerminationCondition } from '../../types/nodes'

// Computed property for script validation
const scriptValidation = computed(() => {
  const script = loopConfig.value.termination_condition?.script
  if (!script) return null
  return validateJavaScriptFunction(script)
})

interface Props {
  modelValue?: LoopConfig
}

interface Emits {
  (e: 'update:modelValue', value: LoopConfig | undefined): void
  (e: 'update'): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

const isLoopEnabled = computed(() => props.modelValue !== undefined)

const loopConfig = computed(() => {
  return props.modelValue || getDefaultLoopConfig()
})

const backoffType = computed(() => {
  if ('Fixed' in loopConfig.value.backoff_strategy) return 'Fixed'
  if ('Exponential' in loopConfig.value.backoff_strategy) return 'Exponential'
  return 'Fixed'
})

function getDefaultLoopConfig(): LoopConfig {
  return {
    interval_seconds: 60,
    backoff_strategy: { Fixed: 60 },
    termination_condition: {
      script: `function condition(event) {
  // Return true to terminate loop
  // Use event.data.metadata for numeric values
  return event.data.status === 'completed';
}`,
      action: 'Success'
    }
  }
}

function toggleLoop() {
  if (isLoopEnabled.value) {
    emit('update:modelValue', undefined)
  } else {
    emit('update:modelValue', getDefaultLoopConfig())
  }
  emit('update')
}

function updateLoopConfig(key: keyof LoopConfig, value: unknown) {
  if (!props.modelValue) return

  const updated = { ...props.modelValue, [key]: value }
  emit('update:modelValue', updated)
  emit('update')
}

function updateBackoffType(type: 'Fixed' | 'Exponential') {
  if (!props.modelValue) return

  let newStrategy: BackoffStrategy
  if (type === 'Fixed') {
    newStrategy = { Fixed: loopConfig.value.interval_seconds }
  } else {
    newStrategy = { Exponential: { base: 30, multiplier: 1.5, max: 300 } }
  }

  updateLoopConfig('backoff_strategy', newStrategy)
}

function getFixedInterval(): number {
  const strategy = loopConfig.value.backoff_strategy
  return 'Fixed' in strategy ? strategy.Fixed : 60
}

function updateFixedInterval(value: number) {
  updateLoopConfig('backoff_strategy', { Fixed: value })
}

function getExponentialConfig() {
  const strategy = loopConfig.value.backoff_strategy
  if ('Exponential' in strategy) {
    return strategy.Exponential
  }
  return { base: 30, multiplier: 1.5, max: 300 }
}

function updateExponentialConfig(key: 'base' | 'multiplier' | 'max', value: number) {
  const current = getExponentialConfig()
  const updated = { ...current, [key]: value }
  updateLoopConfig('backoff_strategy', { Exponential: updated })
}

function updateTerminationCondition(key: keyof TerminationCondition, value: string) {
  const currentCondition = loopConfig.value.termination_condition || {
    script: `function condition(event) {
  // Return true to terminate loop
  return event.data.status === 'completed';
}`,
    action: 'Success'
  }

  const updatedCondition = { ...currentCondition, [key]: value }

  // Basic JavaScript validation for script changes
  if (key === 'script') {
    const validationResult = validateJavaScriptFunction(value)
    if (!validationResult.isValid) {
      console.warn('JavaScript validation warning:', validationResult.errors.join(', '))
      // Still allow the update but log warnings
    }
  }

  updateLoopConfig('termination_condition', updatedCondition)
}

function validateJavaScriptFunction(script: string): { isValid: boolean, errors: string[] } {
  const errors: string[] = []

  // Basic validation checks
  if (!script.trim()) {
    return { isValid: false, errors: ['Script cannot be empty'] }
  }

  // Check for function declaration
  if (!script.includes('function condition(event)')) {
    errors.push('Script must contain "function condition(event)" declaration')
  }

  // Check for return statement
  if (!script.includes('return')) {
    errors.push('Function must contain at least one return statement')
  }

  // Check for basic syntax issues (simple validation)
  const openBraces = (script.match(/\{/g) || []).length
  const closeBraces = (script.match(/\}/g) || []).length
  if (openBraces !== closeBraces) {
    errors.push('Mismatched braces - check function syntax')
  }

  // Check for common dangerous patterns
  const dangerousPatterns = [
    'eval(',
    'Function(',
    'setTimeout(',
    'setInterval(',
    'XMLHttpRequest',
    'fetch(',
    'import(',
    'require('
  ]

  for (const pattern of dangerousPatterns) {
    if (script.includes(pattern)) {
      errors.push(`Potentially unsafe pattern detected: ${pattern}`)
    }
  }

  // Check for proper event access patterns
  if (script.includes('event.') && !script.includes('event.data')) {
    errors.push('Consider using event.data to access response data')
  }

  return {
    isValid: errors.length === 0,
    errors
  }
}


function applyTemplate(template: string) {
  let config: LoopConfig

  switch (template) {
    case 'customer-onboarding':
      config = {
        max_iterations: 72,
        interval_seconds: 3600,
        backoff_strategy: { Fixed: 3600 },
        termination_condition: {
          script: `function condition(event) {
  // Terminate successfully if data is ingested, fail after 3 consecutive failures
  if (event.data.has_ingested_data === true) return true;
  if (event.data.metadata.consecutive_failures >= 3) return true;
  return false;
}`,
          action: 'Success'
        }
      }
      break

    case 'health-monitoring':
      config = {
        interval_seconds: 30,
        backoff_strategy: { Exponential: { base: 30, multiplier: 1.5, max: 300 } },
        termination_condition: {
          script: `function condition(event) {
  // Succeed on HTTP 200, fail if running too long
  if (event.data.metadata.http_status === 200) return true;
  if (event.data.metadata.elapsed_seconds > 3600) return true;
  return false;
}`,
          action: 'Success'
        }
      }
      break

    case 'data-sync':
      config = {
        max_iterations: 5,
        interval_seconds: 1,
        backoff_strategy: { Exponential: { base: 1, multiplier: 2.0, max: 30 } },
        termination_condition: {
          script: `function condition(event) {
  // Succeed when sync is completed, fail on client errors
  if (event.data.sync_status === "completed") return true;
  if (event.data.metadata.http_status >= 400 && event.data.metadata.http_status < 500) return true;
  return false;
}`,
          action: 'Success'
        }
      }
      break

    default:
      return
  }

  emit('update:modelValue', config)
  emit('update')
}
</script>