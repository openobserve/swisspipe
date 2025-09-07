<template>
  <div class="space-y-4">
    <div>
      <label class="block text-sm font-medium text-gray-300 mb-2">OpenObserve URL</label>
      <input
        :value="modelValue.openobserve_url"
        @input="updateConfig('openobserve_url', ($event.target as HTMLInputElement).value)"
        @blur="$emit('update')"
        type="url"
        placeholder="https://your-openobserve-instance.com/api/org/stream/_json"
        class="w-full bg-slate-700 border border-slate-600 text-gray-100 px-3 py-2 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
      />
      <p class="text-xs text-gray-400 mt-1">
        Example: https://api.openobserve.ai/api/hsghdjfgjh3fgr3gkj/your-stream/_json
      </p>
    </div>
    
    <div>
      <label class="block text-sm font-medium text-gray-300 mb-2">Authorization Header</label>
      <input
        :value="modelValue.authorization_header"
        @input="updateConfig('authorization_header', ($event.target as HTMLInputElement).value)"
        @blur="$emit('update')"
        type="text"
        placeholder="Basic cm9vdEBleGFtcGxlLmNvbTpDb21wbGV4cGFzcyMxMjM="
        class="w-full bg-slate-700 border border-slate-600 text-gray-100 px-3 py-2 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
      />
    </div>


    <div class="bg-blue-900/20 border border-blue-700/50 p-3 rounded-md">
      <div class="flex items-start space-x-2">
        <div class="text-blue-400 mt-0.5">
          <svg class="h-4 w-4" fill="currentColor" viewBox="0 0 20 20">
            <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z" clip-rule="evenodd" />
          </svg>
        </div>
        <div>
          <p class="text-sm text-blue-300 font-medium">Data Format</p>
          <p class="text-xs text-blue-400 mt-1">
            OpenObserve expects JSON Array data. The workflow event data will be automatically sent as JSON Array payload.
          </p>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
interface OpenObserveConfig {
  openobserve_url: string
  authorization_header: string
  stream_name: string
}

interface Props {
  modelValue: OpenObserveConfig
}

interface Emits {
  (e: 'update:modelValue', value: OpenObserveConfig): void
  (e: 'update'): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

function updateConfig(key: keyof OpenObserveConfig, value: string) {
  const updated = { ...props.modelValue }
  updated[key] = value
  emit('update:modelValue', updated)
}
</script>