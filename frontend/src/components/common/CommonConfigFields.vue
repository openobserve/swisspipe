<template>
  <div class="space-y-4">
    <!-- Timeout Configuration -->
    <div>
      <label class="block text-sm font-medium text-gray-300 mb-2">Timeout (seconds)</label>
      <input
        :value="config.timeout_seconds"
        @input="updateTimeout"
        @blur="$emit('update')"
        type="number"
        min="1"
        max="300"
        class="w-full bg-slate-700 border border-slate-600 text-gray-100 px-3 py-2 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
      />
    </div>
    
    <!-- Failure Action Configuration -->
    <div>
      <label class="block text-sm font-medium text-gray-300 mb-2">On Failure</label>
      <select
        :value="config.failure_action"
        @change="updateFailureAction"
        class="w-full bg-slate-700 border border-slate-600 text-gray-100 px-3 py-2 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
      >
        <option value="Stop">Stop Workflow</option>
        <option value="Continue">Continue to Next Node</option>
        <option value="Retry">Retry This Node</option>
      </select>
    </div>
    
    <!-- Retry Configuration -->
    <div v-if="config.failure_action === 'Retry'">
      <label class="block text-sm font-medium text-gray-300 mb-2">Retry Attempts</label>
      <input
        :value="config.retry_config?.max_attempts || 3"
        @input="updateRetryAttempts"
        @blur="$emit('update')"
        type="number"
        min="1"
        max="10"
        class="w-full bg-slate-700 border border-slate-600 text-gray-100 px-3 py-2 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

interface Props {
  modelValue: {
    timeout_seconds?: number
    failure_action?: string
    retry_config?: {
      max_attempts?: number
    }
  }
}

interface Emits {
  (e: 'update:modelValue', value: any): void
  (e: 'update'): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

const config = computed(() => props.modelValue)

function updateTimeout(event: Event) {
  const target = event.target as HTMLInputElement
  const newConfig = {
    ...config.value,
    timeout_seconds: parseInt(target.value) || 30
  }
  emit('update:modelValue', newConfig)
}

function updateFailureAction(event: Event) {
  const target = event.target as HTMLSelectElement
  const newConfig = {
    ...config.value,
    failure_action: target.value
  }
  
  // Initialize retry_config if Retry is selected
  if (target.value === 'Retry' && !newConfig.retry_config) {
    newConfig.retry_config = { max_attempts: 3 }
  }
  
  emit('update:modelValue', newConfig)
  emit('update')
}

function updateRetryAttempts(event: Event) {
  const target = event.target as HTMLInputElement
  const newConfig = {
    ...config.value,
    retry_config: {
      ...config.value.retry_config,
      max_attempts: parseInt(target.value) || 3
    }
  }
  emit('update:modelValue', newConfig)
}
</script>