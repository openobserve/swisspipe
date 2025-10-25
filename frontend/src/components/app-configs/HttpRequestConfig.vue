<template>
  <div class="space-y-4">
    <div>
      <label class="block text-sm font-medium text-gray-300 mb-2">HTTP Request URL</label>
      <input
        :value="modelValue.url"
        @input="updateConfig('url', ($event.target as HTMLInputElement).value)"
        @blur="$emit('update')"
        type="url"
        placeholder="https://api.example.com/endpoint"
        class="w-full bg-slate-700 border border-slate-600 text-gray-100 px-3 py-2 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
      />
    </div>
    
    <div>
      <label class="block text-sm font-medium text-gray-300 mb-2">HTTP Method</label>
      <select
        :value="modelValue.method"
        @change="updateConfig('method', ($event.target as HTMLSelectElement).value)"
        @blur="$emit('update')"
        class="w-full bg-slate-700 border border-slate-600 text-gray-100 px-3 py-2 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
      >
        <option value="Get">GET</option>
        <option value="Post">POST</option>
        <option value="Put">PUT</option>
        <option value="Patch">PATCH</option>
        <option value="Delete">DELETE</option>
      </select>
    </div>
    
    <div>
      <label class="block text-sm font-medium text-gray-300 mb-2">Headers</label>
      <div class="space-y-2">
        <div v-for="(value, key, index) in modelValue.headers || {}" :key="`header-${index}`" class="flex gap-2">
          <input
            :value="key"
            @input="updateHeaderKey($event, key)"
            @blur="$emit('update')"
            type="text"
            placeholder="Header name"
            class="flex-1 bg-slate-700 border border-slate-600 text-gray-100 px-3 py-2 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
          />
          <input
            :value="value"
            @input="updateHeaderValue(key, ($event.target as HTMLInputElement).value)"
            @blur="$emit('update')"
            type="text"
            placeholder="Header value"
            class="flex-1 bg-slate-700 border border-slate-600 text-gray-100 px-3 py-2 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
          />
          <button
            @click="removeHeader(key)"
            class="px-3 py-2 bg-red-600 hover:bg-red-700 text-white rounded-md transition-colors"
          >
            ✕
          </button>
        </div>
        <button
          @click="addHeader"
          class="w-full px-3 py-2 bg-slate-600 hover:bg-slate-500 text-gray-300 rounded-md transition-colors text-sm"
        >
          + Add Header
        </button>
      </div>
    </div>

    <!-- HTTP Loop Configuration -->
    <HttpLoopConfig
      :model-value="modelValue.loop_config"
      @update:model-value="updateLoopConfig"
      @update="$emit('update')"
    />
  </div>
</template>

<script setup lang="ts">
import HttpLoopConfig from './HttpLoopConfig.vue'
import type { HttpRequestConfig as HttpRequestConfigType, LoopConfig } from '../../types/nodes'

interface Props {
  modelValue: HttpRequestConfigType
}

interface Emits {
  (e: 'update:modelValue', value: HttpRequestConfigType): void
  (e: 'update'): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

function updateConfig(key: keyof HttpRequestConfigType, value: unknown) {
  const updated = { ...props.modelValue }
  ;(updated as Record<string, unknown>)[key] = value
  emit('update:modelValue', updated)
}

function updateHeaderKey(event: Event, oldKey: string) {
  const newKey = (event.target as HTMLInputElement).value
  if (newKey === oldKey) return
  
  const headers = { ...props.modelValue.headers }
  if (headers[oldKey] !== undefined) {
    headers[newKey] = headers[oldKey]
    delete headers[oldKey]
  }
  
  updateConfig('headers', headers)
}

function updateHeaderValue(key: string, value: string) {
  const headers = { ...props.modelValue.headers }
  headers[key] = value
  updateConfig('headers', headers)
}

function addHeader() {
  const headers = { ...(props.modelValue.headers || {}) }
  headers[''] = ''
  updateConfig('headers', headers)
}

function removeHeader(key: string) {
  const headers = { ...props.modelValue.headers }
  delete headers[key]
  updateConfig('headers', headers)
}

function updateLoopConfig(value: LoopConfig | undefined) {
  updateConfig('loop_config', value)
}
</script>