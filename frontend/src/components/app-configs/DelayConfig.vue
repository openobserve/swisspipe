<template>
  <div class="space-y-4">
    <div>
      <label class="block text-sm font-medium text-gray-300 mb-2">Duration</label>
      <input
        :value="localConfig.duration || 1"
        @input="updateDuration"
        @blur="$emit('update')"
        type="number"
        min="1"
        class="w-full bg-slate-700 border border-slate-600 text-gray-100 px-3 py-2 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
      />
    </div>
    
    <div>
      <label class="block text-sm font-medium text-gray-300 mb-2">Unit</label>
      <select
        :value="localConfig.unit || 'Minutes'"
        @change="updateUnit"
        class="w-full bg-slate-700 border border-slate-600 text-gray-100 px-3 py-2 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
      >
        <option value="Seconds">Seconds</option>
        <option value="Minutes">Minutes</option>
        <option value="Hours">Hours</option>
        <option value="Days">Days</option>
      </select>
    </div>
    
    <div class="text-sm text-gray-400">
      The workflow will pause for {{ localConfig.duration || 1 }} {{ (localConfig.unit || 'Minutes').toLowerCase() }} before continuing.
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'

interface Props {
  modelValue: {
    duration?: number
    unit?: string
  }
}

interface Emits {
  (e: 'update:modelValue', value: Props['modelValue']): void
  (e: 'update'): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

const localConfig = ref({ ...props.modelValue })

watch(() => props.modelValue, (newValue) => {
  localConfig.value = { ...newValue }
}, { deep: true })

function updateDuration(event: Event) {
  const target = event.target as HTMLInputElement
  localConfig.value.duration = parseInt(target.value) || 1
  emit('update:modelValue', localConfig.value)
}

function updateUnit(event: Event) {
  const target = event.target as HTMLSelectElement
  localConfig.value.unit = target.value
  emit('update:modelValue', localConfig.value)
  emit('update')
}
</script>