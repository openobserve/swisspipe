<template>
  <div class="email-recipient-list space-y-2">
    <div
      v-for="(recipient, index) in localRecipients"
      :key="index"
      class="flex items-center space-x-2"
    >
      <input
        v-model="recipient.email"
        @input="onEmailInputDebug"
        @blur="onFieldBlur"
        placeholder="email@example.com or {{ workflow.data.email }}"
        type="text"
        class="flex-1 px-3 py-2 bg-gray-700 border border-gray-600 rounded-md text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
      />
      <input
        v-model="recipient.name"
        placeholder="Name (optional)"
        class="w-40 px-3 py-2 bg-gray-700 border border-gray-600 rounded-md text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
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
import { ref, watch } from 'vue'
import type { EmailAddress } from '../../types/nodes'

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

const localRecipients = ref<EmailAddress[]>([...props.modelValue])
const showSuggestions = ref(false)

const commonVariables = [
  { path: '{{ workflow.data.user_email }}', description: 'User email from workflow data' },
  { path: '{{ workflow.data.admin_email }}', description: 'Admin email from workflow data' },
  { path: '{{ workflow.data.email }}', description: 'Generic email from workflow data' },
  { path: '{{ workflow.data.user_name }}', description: 'User name from workflow data' },
  { path: '{{ workflow.metadata.created_by }}', description: 'Workflow creator' }
]

// Ensure at least one recipient if not allowing empty
if (localRecipients.value.length === 0 && !props.allowEmpty) {
  localRecipients.value.push({ email: '', name: '' })
}

// Watch for external changes (without deep watching) - but don't override if user is actively typing
let userIsTyping = false
watch(
  () => props.modelValue,
  (newValue) => {
    console.log('EmailRecipientList props watcher triggered:', {
      newValue: JSON.stringify(newValue, null, 2),
      currentLocal: JSON.stringify(localRecipients.value, null, 2),
      userIsTyping
    })
    
    // Only update if user is not actively typing
    if (!userIsTyping) {
      localRecipients.value = [...newValue]
    }
  }
)

// Debounced emit function
let emitTimeout: ReturnType<typeof setTimeout> | null = null
const emitUpdate = () => {
  if (emitTimeout) clearTimeout(emitTimeout)
  emitTimeout = setTimeout(() => {
    console.log('EmailRecipientList emitting update:', JSON.stringify(localRecipients.value, null, 2))
    emit('update:modelValue', [...localRecipients.value])
  }, 400) // Increased debounce to avoid partial data capture
}

// Debug function to track input changes
const onEmailInputDebug = (event: Event) => {
  userIsTyping = true
  const target = event.target as HTMLInputElement
  console.log('EmailRecipientList input event:', {
    inputValue: target.value,
    recipientsData: JSON.stringify(localRecipients.value, null, 2)
  })
}

// Immediate update on blur to avoid truncation issues
const onFieldBlur = () => {
  userIsTyping = false  // User finished typing
  console.log('EmailRecipientList field blur - emitting immediate update:', JSON.stringify(localRecipients.value, null, 2))
  emit('update:modelValue', [...localRecipients.value])
}

// Emit changes with debouncing
watch(
  localRecipients,
  () => emitUpdate(),
  { deep: true }
)

function addRecipient() {
  localRecipients.value.push({ email: '', name: '' })
}

function removeRecipient(index: number) {
  if (localRecipients.value.length > 1 || props.allowEmpty) {
    localRecipients.value.splice(index, 1)
  }
}

function insertVariable(variablePath: string) {
  // Find the last focused email input and insert the variable
  // For simplicity, we'll add it to the last recipient's email field
  if (localRecipients.value.length > 0) {
    const lastRecipient = localRecipients.value[localRecipients.value.length - 1]
    if (!lastRecipient.email) {
      lastRecipient.email = variablePath
    } else {
      // Add a new recipient with the variable
      localRecipients.value.push({ email: variablePath, name: '' })
    }
  }
}
</script>

<style scoped>
.email-recipient-list {
  @apply space-y-2;
}
</style>