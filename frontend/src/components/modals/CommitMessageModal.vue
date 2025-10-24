<template>
  <div v-if="visible" class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
    <div class="bg-slate-800 rounded-lg shadow-xl max-w-2xl w-full mx-4 border border-slate-700">
      <!-- Header -->
      <div class="px-6 py-4 border-b border-slate-700 flex items-center justify-between">
        <h2 class="text-lg font-semibold text-gray-200">Commit Changes</h2>
        <button @click="$emit('close')" class="text-gray-400 hover:text-gray-200 transition-colors">
          <svg class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      <!-- Content -->
      <div class="p-6 space-y-4">
        <!-- Subject Line -->
        <div>
          <label class="block text-sm font-medium text-gray-300 mb-2">
            Subject (required)
          </label>
          <input
            v-model="message"
            type="text"
            maxlength="100"
            placeholder="Add email notification on failure"
            class="w-full px-4 py-2 bg-slate-700 border border-slate-600 rounded-md text-gray-200 placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-blue-500"
            @keyup.enter="handleConfirm"
            ref="messageInput"
          />
          <div class="mt-1 text-xs text-gray-400">
            {{ message.length }}/100 characters
          </div>
        </div>

        <!-- Description -->
        <div>
          <label class="block text-sm font-medium text-gray-300 mb-2">
            Description (optional)
          </label>
          <textarea
            v-model="description"
            rows="4"
            maxlength="1000"
            placeholder="Added email node to send notifications when the HTTP request fails..."
            class="w-full px-4 py-2 bg-slate-700 border border-slate-600 rounded-md text-gray-200 placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-blue-500"
          ></textarea>
          <div class="mt-1 text-xs text-gray-400">
            {{ description.length }}/1000 characters
          </div>
        </div>
      </div>

      <!-- Footer -->
      <div class="px-6 py-4 border-t border-slate-700 flex justify-end space-x-3">
        <button
          @click="$emit('close')"
          class="px-4 py-2 text-gray-300 hover:text-gray-100 transition-colors"
        >
          Cancel
        </button>
        <button
          @click="handleConfirm"
          :disabled="!message.trim() || saving"
          class="px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:bg-slate-600 disabled:cursor-not-allowed text-white rounded-md transition-colors"
        >
          <span v-if="saving">Saving...</span>
          <span v-else>Commit & Save</span>
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'

interface Props {
  visible: boolean
  saving?: boolean
}

const props = defineProps<Props>()

const emit = defineEmits<{
  close: []
  confirm: [{ message: string; description: string | null }]
}>()

const message = ref('')
const description = ref('')
const messageInput = ref<HTMLInputElement | null>(null)

function handleConfirm() {
  if (!message.value.trim()) return

  emit('confirm', {
    message: message.value.trim(),
    description: description.value.trim() || null
  })
}

// Reset form when modal is closed
watch(() => props.visible, (isVisible) => {
  if (isVisible) {
    message.value = ''
    description.value = ''
    // Auto-focus the message input when modal opens
    nextTick(() => {
      messageInput.value?.focus()
    })
  }
})
</script>
