<template>
  <div class="space-y-4">
    <div>
      <label class="block text-sm font-medium text-gray-300 mb-2">JavaScript Code</label>
      <div class="h-80">
        <CodeEditor
          :modelValue="localConfig.script || ''"
          :language="'javascript'"
          @update:modelValue="onScriptChange"
          @save="$emit('update')"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import CodeEditor from '../common/CodeEditor.vue'

interface Props {
  modelValue: {
    script?: string
  }
}

interface Emits {
  (e: 'update:modelValue', value: any): void
  (e: 'update'): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

const localConfig = ref({ ...props.modelValue })

watch(() => props.modelValue, (newValue) => {
  localConfig.value = { ...newValue }
}, { deep: true })

function onScriptChange(newScript: string) {
  localConfig.value.script = newScript
  emit('update:modelValue', localConfig.value)
}
</script>