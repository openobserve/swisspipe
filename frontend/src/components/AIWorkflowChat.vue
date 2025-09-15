<template>
  <div
    v-if="showChat"
    class="fixed inset-0 bg-black/30 backdrop-blur-sm flex items-center justify-center z-50"
    @click.self="closeChat"
  >
    <div class="glass-strong rounded-lg w-full max-w-2xl max-h-[80vh] shadow-2xl flex flex-col">
      <!-- Header -->
      <div class="flex items-center justify-between p-4 border-b border-slate-600/50">
        <div class="flex items-center space-x-3">
          <div class="w-10 h-10 bg-gradient-to-r from-purple-500 to-pink-500 rounded-lg flex items-center justify-center">
            <svg class="w-6 h-6 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"></path>
            </svg>
          </div>
          <div>
            <h2 class="text-lg font-medium text-white">AI Workflow Assistant</h2>
            <p class="text-sm text-gray-400">Describe what you want to automate</p>
          </div>
        </div>
        <button
          @click="closeChat"
          class="text-gray-400 hover:text-gray-300 transition-colors"
        >
          <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
          </svg>
        </button>
      </div>

      <!-- Chat Messages -->
      <div class="flex-1 p-4 overflow-y-auto min-h-[300px] max-h-[400px]">
        <div v-if="messages.length === 0" class="text-center text-gray-400 mt-8">
          <div class="w-16 h-16 bg-gradient-to-r from-purple-500/20 to-pink-500/20 rounded-full flex items-center justify-center mx-auto mb-4">
            <svg class="w-8 h-8 text-purple-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z"></path>
            </svg>
          </div>
          <p class="mb-2">Tell me what workflow you'd like to create!</p>
          <p class="text-sm">For example: "Send email alerts when my API receives data" or "Transform JSON data and send to a webhook"</p>
        </div>

        <div v-for="(message, index) in messages" :key="index" class="mb-4">
          <div v-if="message.type === 'user'" class="flex justify-end">
            <div class="bg-primary-600 text-white rounded-lg px-4 py-2 max-w-[80%]">
              {{ message.content }}
            </div>
          </div>
          <div v-else class="flex justify-start">
            <div class="bg-slate-700 text-gray-100 rounded-lg px-4 py-2 max-w-[80%]">
              <div v-if="message.type === 'ai'" class="whitespace-pre-wrap">{{ message.content }}</div>
              <div v-else-if="message.type === 'workflow'" class="space-y-2">
                <p class="text-green-400 font-medium">✓ Workflow created successfully!</p>
                <p class="text-sm">{{ message.content }}</p>
                <button
                  @click="navigateToWorkflow(message.workflowId!)"
                  class="mt-2 bg-primary-600 hover:bg-primary-700 text-white px-3 py-1 rounded text-sm transition-colors"
                >
                  Open Workflow
                </button>
              </div>
              <div v-else-if="message.type === 'error'" class="text-red-400">
                {{ message.content }}
              </div>
            </div>
          </div>
        </div>

        <div v-if="isGenerating" class="flex justify-start mb-4">
          <div class="bg-slate-700 text-gray-100 rounded-lg px-4 py-2">
            <div class="flex items-center space-x-2">
              <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-purple-400"></div>
              <span>Creating your workflow...</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Input Area -->
      <div class="p-4 border-t border-slate-600/50">
        <form @submit.prevent="sendMessage" class="flex space-x-3">
          <input
            v-model="currentMessage"
            type="text"
            placeholder="Describe your workflow needs..."
            class="flex-1 glass border border-slate-600/50 text-gray-100 px-4 py-2 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
            :disabled="isGenerating"
          />
          <button
            type="submit"
            :disabled="!currentMessage.trim() || isGenerating"
            class="bg-primary-600 hover:bg-primary-700 disabled:bg-gray-600 text-white px-4 py-2 rounded-md font-medium transition-colors"
          >
            Send
          </button>
        </form>
        <p class="text-xs text-gray-500 mt-2">
          Tip: Be specific about data sources, actions, and conditions for better results.
        </p>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useWorkflowStore } from '../stores/workflows'
import { apiClient } from '../services/api'

interface ChatMessage {
  type: 'user' | 'ai' | 'workflow' | 'error'
  content: string
  workflowId?: string
}

const props = defineProps<{
  showChat: boolean
}>()

const emit = defineEmits<{
  close: []
}>()

const router = useRouter()
const workflowStore = useWorkflowStore()

const messages = ref<ChatMessage[]>([])
const currentMessage = ref('')
const isGenerating = ref(false)

function closeChat() {
  emit('close')
  // Reset chat state when closing
  messages.value = []
  currentMessage.value = ''
  isGenerating.value = false
}

async function sendMessage() {
  if (!currentMessage.value.trim() || isGenerating.value) return

  const userMessage = currentMessage.value.trim()
  messages.value.push({ type: 'user', content: userMessage })
  currentMessage.value = ''
  isGenerating.value = true

  try {
    // Call the AI service to generate workflow
    const response = await apiClient.generateWorkflow({ prompt: userMessage })

    if (response.success && response.workflow_id) {
      messages.value.push({
        type: 'workflow',
        content: `Created "${response.workflow_name}"`,
        workflowId: response.workflow_id
      })

      // Refresh workflows list
      await workflowStore.fetchWorkflows()
    } else {
      messages.value.push({
        type: 'error',
        content: response.error || 'Failed to create workflow. Please try again with more specific requirements.'
      })
    }
  } catch (error) {
    console.error('AI workflow generation error:', error)
    messages.value.push({
      type: 'error',
      content: 'Sorry, I encountered an error. Please try again.'
    })
  } finally {
    isGenerating.value = false
  }
}


function navigateToWorkflow(workflowId: string) {
  router.push(`/workflows/${workflowId}`)
  closeChat()
}
</script>