<template>
  <div class="space-y-4">
    <div>
      <label for="merge-strategy-select" class="block text-sm font-medium text-gray-300 mb-2">
        Input Merge Strategy
        <span class="text-xs text-gray-500 ml-2">
          (How to handle multiple inputs from predecessor nodes)
        </span>
      </label>

      <select
        id="merge-strategy-select"
        :value="strategyType"
        @change="handleStrategyChange"
        aria-describedby="merge-strategy-description"
        class="w-full bg-slate-700 border border-slate-600 text-gray-100 px-3 py-2 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
      >
        <option value="WaitForAll">Wait for All REACHABLE Predecessors</option>
        <option value="FirstWins">First Input Wins</option>
        <option value="TimeoutBased">Timeout Based</option>
      </select>

      <p id="merge-strategy-description" class="text-xs text-gray-500 mt-1">
        {{ STRATEGY_DESCRIPTIONS[strategyType] }}
      </p>
    </div>

    <!-- Timeout configuration (only shown for TimeoutBased) -->
    <div v-if="strategyType === 'TimeoutBased'" class="pl-4 border-l-2 border-slate-600">
      <label for="merge-timeout-input" class="block text-sm font-medium text-gray-300 mb-2">
        Timeout (seconds)
      </label>
      <input
        id="merge-timeout-input"
        type="number"
        :value="timeoutValue"
        @input="handleTimeoutChange"
        @blur="handleTimeoutBlur"
        min="1"
        max="86400"
        step="1"
        aria-describedby="merge-timeout-description"
        :aria-invalid="!!validationError"
        :class="[
          'w-full px-3 py-2 rounded-md focus:outline-none focus:ring-2',
          validationError
            ? 'bg-red-900/20 border-2 border-red-500 text-red-200 focus:ring-red-500'
            : 'bg-slate-700 border border-slate-600 text-gray-100 focus:ring-primary-500'
        ]"
        placeholder="e.g., 30"
      />
      <p
        v-if="validationError"
        class="text-xs text-red-400 mt-1"
        role="alert"
      >
        {{ validationError }}
      </p>
      <p
        v-else
        id="merge-timeout-description"
        class="text-xs text-gray-500 mt-1"
      >
        Wait up to this many seconds for all inputs, then execute with whatever was received (1-86400 seconds / 24 hours max)
      </p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import type { InputMergeStrategy } from '../../types/nodes'

interface Props {
  modelValue?: InputMergeStrategy
}

interface Emits {
  (e: 'update:modelValue', value: InputMergeStrategy): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

// Constants
const DEFAULT_TIMEOUT = 30
const MIN_TIMEOUT = 1
const MAX_TIMEOUT = 86400 // 24 hours in seconds

const STRATEGY_DESCRIPTIONS: Record<string, string> = {
  WaitForAll: 'Wait for all reachable predecessor nodes to complete before executing. Skips predecessors on untaken conditional paths (default for multiple inputs)',
  FirstWins: 'Execute immediately on first input, ignore subsequent inputs (default for single input)',
  TimeoutBased: 'Wait up to the specified timeout for inputs, then execute with whatever was received'
}

type StrategyType = 'WaitForAll' | 'FirstWins' | 'TimeoutBased'

// State
const validationError = ref<string>('')

// Extract strategy type from the union type
const strategyType = computed<StrategyType>(() => {
  if (!props.modelValue) return 'WaitForAll'

  if ('WaitForAll' in props.modelValue) return 'WaitForAll'
  if ('FirstWins' in props.modelValue) return 'FirstWins'
  if ('TimeoutBased' in props.modelValue) return 'TimeoutBased'

  return 'WaitForAll'
})

// Extract timeout value for TimeoutBased strategy
const timeoutValue = computed<number>(() => {
  if (!props.modelValue) return DEFAULT_TIMEOUT
  if ('TimeoutBased' in props.modelValue) {
    return props.modelValue.TimeoutBased || DEFAULT_TIMEOUT
  }
  return DEFAULT_TIMEOUT
})

function handleStrategyChange(event: Event) {
  const target = event.target as HTMLSelectElement
  const newType = target.value as StrategyType

  validationError.value = ''

  let newStrategy: InputMergeStrategy

  switch (newType) {
    case 'WaitForAll':
      newStrategy = { WaitForAll: null }
      break
    case 'FirstWins':
      newStrategy = { FirstWins: null }
      break
    case 'TimeoutBased':
      newStrategy = { TimeoutBased: timeoutValue.value }
      break
    default:
      newStrategy = { WaitForAll: null }
  }

  emit('update:modelValue', newStrategy)
}

function validateTimeout(timeout: number): string | null {
  if (isNaN(timeout)) {
    return 'Please enter a valid number'
  }
  if (timeout < MIN_TIMEOUT) {
    return `Timeout must be at least ${MIN_TIMEOUT} second`
  }
  if (timeout > MAX_TIMEOUT) {
    return `Timeout cannot exceed ${MAX_TIMEOUT} seconds (24 hours)`
  }
  if (!Number.isInteger(timeout)) {
    return 'Timeout must be a whole number'
  }
  return null
}

function handleTimeoutChange(event: Event) {
  const target = event.target as HTMLInputElement
  const timeout = parseInt(target.value, 10)

  const error = validateTimeout(timeout)
  validationError.value = error || ''

  // Only emit valid values
  if (!error) {
    emit('update:modelValue', { TimeoutBased: timeout })
  }
}

function handleTimeoutBlur(event: Event) {
  const target = event.target as HTMLInputElement
  const value = target.value.trim()

  // If empty on blur, restore default
  if (!value) {
    validationError.value = ''
    emit('update:modelValue', { TimeoutBased: DEFAULT_TIMEOUT })
    return
  }

  const timeout = parseInt(value, 10)
  const error = validateTimeout(timeout)

  if (error) {
    // Restore to last valid value on blur if invalid
    validationError.value = error
    // Reset to default after a moment
    setTimeout(() => {
      validationError.value = ''
      emit('update:modelValue', { TimeoutBased: DEFAULT_TIMEOUT })
    }, 2000)
  }
}
</script>
