<template>
  <div class="email-recipient-list space-y-2">
    <div
      v-for="(recipient, index) in localRecipients"
      :key="index"
      class="flex items-center space-x-2"
    >
      <input
        v-model="recipient.email"
        @input="(event) => onEmailInputDebug(event, index)"
        @blur="(event) => onFieldBlur(event, index)"
        placeholder="email@example.com or {{ event.data.email }}"
        type="text"
        class="w-1/2 px-3 py-2 bg-gray-700 border border-gray-600 rounded-md text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
      />
      <input
        v-model="recipient.name"
        @input="(event) => onNameInputDebug(event, index)"
        @blur="(event) => onFieldBlur(event, index)"
        placeholder="Name (optional)"
        class="w-1/2 px-3 py-2 bg-gray-700 border border-gray-600 rounded-md text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
      />
      <button
        @click="removeRecipient(index)"
        :disabled="localRecipients.length === 1 && !allowEmpty"
        class="px-2 py-2 bg-red-600 hover:bg-red-700 disabled:bg-gray-600 disabled:cursor-not-allowed rounded-md text-white transition-colors"
        title="Remove recipient"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
        </svg>
      </button>
    </div>

    <button
      @click="addRecipient"
      class="flex items-center space-x-2 px-3 py-2 bg-blue-600 hover:bg-blue-700 rounded-md text-white transition-colors"
    >
      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6" />
      </svg>
      <span>Add Recipient</span>
    </button>

    <!-- Template Variable Suggestions -->
    <div v-if="showSuggestions" class="mt-2 p-2 bg-gray-700 rounded-md">
      <div class="text-xs font-medium text-gray-300 mb-1">Common Template Variables:</div>
      <div class="flex flex-wrap gap-1">
        <button
          v-for="variable in commonVariables"
          :key="variable.path"
          @click="insertVariable(variable.path)"
          class="px-2 py-1 bg-gray-600 hover:bg-gray-500 rounded text-xs text-white transition-colors"
          :title="variable.description"
        >
          {{ variable.path }}
        </button>
      </div>
    </div>

    <button
      @click="showSuggestions = !showSuggestions"
      class="text-sm text-blue-400 hover:text-blue-300 transition-colors"
    >
      {{ showSuggestions ? 'Hide' : 'Show' }} Template Variables
    </button>
  </div>
</template>

<script setup lang="ts">
import type { EmailAddress } from '../../types/nodes'
import { useEmailRecipients } from '../../composables/useEmailRecipients'

interface Props {
  modelValue: EmailAddress[]
  allowEmpty?: boolean
}

interface Emits {
  (e: 'update:modelValue', value: EmailAddress[]): void
}

const props = withDefaults(defineProps<Props>(), {
  allowEmpty: true
})

const emit = defineEmits<Emits>()

// Use the email recipients composable for all business logic
const {
  localRecipients,
  showSuggestions,
  commonVariables,
  onEmailInputDebug,
  onNameInputDebug,
  onFieldBlur,
  addRecipient,
  removeRecipient,
  insertVariable
} = useEmailRecipients(props, emit)
</script>

<style scoped>
.email-recipient-list {
  @apply space-y-2;
}
</style>