<template>
  <div class="space-y-4 pb-4">
    <div>
      <label class="block text-sm font-medium text-gray-300 mb-2">Model</label>
      <select
        :value="modelValue.model"
        @change="updateConfig('model', ($event.target as HTMLSelectElement).value)"
        @blur="$emit('update')"
        class="w-full bg-slate-700 border border-slate-600 text-gray-100 px-3 py-2 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
      >
        <option value="claude-3-5-sonnet-20241022">Claude 3.5 Sonnet</option>
        <option value="claude-3-5-haiku-20241022">Claude 3.5 Haiku</option>
        <option value="claude-3-opus-20240229">Claude 3 Opus</option>
        <option value="claude-3-sonnet-20240229">Claude 3 Sonnet</option>
        <option value="claude-3-haiku-20240307">Claude 3 Haiku</option>
      </select>
    </div>

    <div>
      <label class="block text-sm font-medium text-gray-300 mb-2">Max Tokens</label>
      <input
        :value="modelValue.max_tokens"
        @input="updateMaxTokens"
        @blur="$emit('update')"
        type="number"
        :min="1"
        :max="getMaxTokensForModel(modelValue.model)"
        placeholder="4000"
        class="w-full bg-slate-700 border border-slate-600 text-gray-100 px-3 py-2 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
      />
      <p class="text-xs text-gray-400 mt-1">Max tokens for {{ getModelDisplayName(modelValue.model) }}: {{ getMaxTokensForModel(modelValue.model).toLocaleString() }}</p>
    </div>

    <div>
      <label class="block text-sm font-medium text-gray-300 mb-2">Temperature</label>
      <input
        :value="modelValue.temperature"
        @input="updateConfig('temperature', parseFloat(($event.target as HTMLInputElement).value) || 0.7)"
        @blur="$emit('update')"
        type="number"
        min="0"
        max="1"
        step="0.1"
        placeholder="0.7"
        class="w-full bg-slate-700 border border-slate-600 text-gray-100 px-3 py-2 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
      />
    </div>

    <div>
      <label class="block text-sm font-medium text-gray-300 mb-2">System Prompt (optional)</label>
      <textarea
        :value="modelValue.system_prompt || ''"
        @input="updateConfig('system_prompt', ($event.target as HTMLTextAreaElement).value || undefined)"
        @blur="$emit('update')"
        rows="3"
        placeholder="You are a helpful AI assistant..."
        class="w-full bg-slate-700 border border-slate-600 text-gray-100 px-3 py-2 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
      />
    </div>

    <div>
      <label class="block text-sm font-medium text-gray-300 mb-2">User Prompt</label>
      <textarea
        :value="modelValue.user_prompt"
        @input="updateConfig('user_prompt', ($event.target as HTMLTextAreaElement).value)"
        @blur="$emit('update')"
        rows="4"
        placeholder="Analyze this data: {{ event.data }}"
        class="w-full bg-slate-700 border border-slate-600 text-gray-100 px-3 py-2 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
      />
    </div>

    <!-- Common Config Fields (timeout, failure action, retry) -->
    <CommonConfigFields
      :model-value="modelValue"
      @update:model-value="(value) => $emit('update:modelValue', value)"
      @update="$emit('update')"
    />
  </div>
</template>

<script setup lang="ts">
import CommonConfigFields from '../common/CommonConfigFields.vue'
import type { AnthropicConfig } from '../../types/nodes'

interface Props {
  modelValue: AnthropicConfig
}

interface Emits {
  (e: 'update:modelValue', value: AnthropicConfig): void
  (e: 'update'): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

function updateConfig(key: keyof AnthropicConfig, value: unknown) {
  const updated = { ...props.modelValue }
  ;(updated as Record<string, unknown>)[key] = value
  emit('update:modelValue', updated)
}

function updateMaxTokens(event: Event) {
  const target = event.target as HTMLInputElement
  const value = parseInt(target.value) || 4000
  const maxTokens = getMaxTokensForModel(props.modelValue.model)
  const clampedValue = Math.min(Math.max(1, value), maxTokens)
  updateConfig('max_tokens', clampedValue)
}

function getMaxTokensForModel(model: string): number {
  if (model.includes('claude-3-5')) return 8192
  if (model.includes('claude-3-opus')) return 4096
  if (model.includes('claude-3-sonnet')) return 4096
  if (model.includes('claude-3-haiku')) return 4096
  return 8192 // Default for newer models
}

function getModelDisplayName(model: string): string {
  if (model.includes('claude-3-5-sonnet')) return 'Claude 3.5 Sonnet'
  if (model.includes('claude-3-5-haiku')) return 'Claude 3.5 Haiku'
  if (model.includes('claude-3-opus')) return 'Claude 3 Opus'
  if (model.includes('claude-3-sonnet')) return 'Claude 3 Sonnet'
  if (model.includes('claude-3-haiku')) return 'Claude 3 Haiku'
  return model
}
</script>