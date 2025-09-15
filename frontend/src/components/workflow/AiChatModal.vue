<template>
  <div class="h-full flex flex-col">
    <!-- Header -->
    <div class="flex items-center justify-between p-4 border-b border-slate-700">
      <div class="flex items-center space-x-2">
        <SparklesIcon class="h-5 w-5 text-orange-400" />
        <h3 class="text-lg font-semibold text-white">AI Assistant</h3>
      </div>
      <button
        @click="$emit('close')"
        class="text-gray-400 hover:text-gray-200 transition-colors"
      >
        <XMarkIcon class="h-5 w-5" />
      </button>
    </div>

    <!-- Chat Messages -->
    <div class="flex-1 overflow-y-auto p-4 space-y-4" ref="messagesContainer">
      <div
        v-for="message in messages"
        :key="message.id"
        class="flex"
        :class="message.role === 'user' ? 'justify-end' : 'justify-start'"
      >
        <div
          class="max-w-[80%] rounded-lg px-3 py-2 text-sm"
          :class="
            message.role === 'user'
              ? 'bg-orange-600 text-white'
              : 'bg-slate-700 text-gray-200'
          "
        >
          <p class="whitespace-pre-wrap">{{ message.content }}</p>
        </div>
      </div>

      <!-- Loading indicator -->
      <div v-if="isLoading" class="flex justify-start">
        <div class="bg-slate-700 rounded-lg px-3 py-2">
          <div class="flex items-center space-x-2">
            <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-orange-400"></div>
            <span class="text-gray-300 text-sm">AI is thinking...</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Input Area -->
    <div class="border-t border-slate-700 p-4">
      <div class="flex items-end space-x-2">
        <div class="flex-1">
          <textarea
            v-model="currentMessage"
            @keydown.enter.prevent="handleSendMessage"
            @keydown.shift.enter="addNewLine"
            placeholder="Ask me to modify your workflow..."
            class="w-full bg-slate-700 text-white placeholder-gray-400 rounded-lg px-3 py-2 text-sm resize-none focus:outline-none focus:ring-2 focus:ring-orange-500"
            rows="3"
          />
        </div>
        <button
          @click="handleSendMessage"
          :disabled="!currentMessage.trim() || isLoading"
          class="bg-orange-600 hover:bg-orange-700 disabled:bg-gray-600 text-white p-2 rounded-lg transition-colors"
        >
          <PaperAirplaneIcon class="h-4 w-4" />
        </button>
      </div>
      <div class="mt-2 text-xs text-gray-400">
        Press Enter to send, Shift+Enter for new line
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, nextTick, onMounted } from 'vue'
import { SparklesIcon, XMarkIcon, PaperAirplaneIcon } from '@heroicons/vue/24/outline'
import { useWorkflowStore } from '../../stores/workflows'
import { apiClient } from '../../services/api'

interface Message {
  id: number
  role: 'user' | 'assistant'
  content: string
  timestamp: Date
}

interface Emits {
  (e: 'close'): void
}

defineEmits<Emits>()

const workflowStore = useWorkflowStore()

const messages = ref<Message[]>([
  {
    id: 1,
    role: 'assistant',
    content: 'Hello! I\'m your AI assistant. I can help you modify your workflow by adding nodes, changing configurations, or explaining how things work. What would you like to do?',
    timestamp: new Date()
  }
])

const currentMessage = ref('')
const isLoading = ref(false)
const messagesContainer = ref<HTMLElement>()

let messageIdCounter = 2

function addNewLine() {
  currentMessage.value += '\n'
}

async function handleSendMessage() {
  if (!currentMessage.value.trim() || isLoading.value) return

  // Add user message
  const userMessage: Message = {
    id: messageIdCounter++,
    role: 'user',
    content: currentMessage.value.trim(),
    timestamp: new Date()
  }
  messages.value.push(userMessage)

  const messageContent = currentMessage.value.trim()
  currentMessage.value = ''

  // Scroll to bottom
  await scrollToBottom()

  // Set loading state
  isLoading.value = true

  try {
    // Call the AI update-workflow endpoint
    await callAiUpdateWorkflow(messageContent)
  } catch (error) {
    console.error('Error sending message to AI:', error)
    // Add error message
    messages.value.push({
      id: messageIdCounter++,
      role: 'assistant',
      content: 'Sorry, I encountered an error. Please try again.',
      timestamp: new Date()
    })
  } finally {
    isLoading.value = false
    await scrollToBottom()
  }
}

async function callAiUpdateWorkflow(userMessage: string) {
  const currentWorkflow = workflowStore.currentWorkflow
  if (!currentWorkflow?.id) {
    messages.value.push({
      id: messageIdCounter++,
      role: 'assistant',
      content: 'I need a workflow to be loaded to help you modify it. Please make sure you have a workflow open.',
      timestamp: new Date()
    })
    return
  }

  try {
    const result = await apiClient.updateWorkflowWithAI({
      workflow_id: currentWorkflow.id,
      prompt: userMessage
    })

    if (result.success) {
      let aiResponse = result.message

      if (result.changes_made && result.changes_made.length > 0) {
        aiResponse += '\n\nChanges analyzed:\n'
        result.changes_made.forEach((change: string, index: number) => {
          aiResponse += `${index + 1}. ${change}\n`
        })
      }

      messages.value.push({
        id: messageIdCounter++,
        role: 'assistant',
        content: aiResponse,
        timestamp: new Date()
      })
    } else {
      messages.value.push({
        id: messageIdCounter++,
        role: 'assistant',
        content: `Sorry, I couldn't process that request: ${result.error || result.message}`,
        timestamp: new Date()
      })
    }
  } catch (error) {
    console.error('AI API call failed:', error)
    messages.value.push({
      id: messageIdCounter++,
      role: 'assistant',
      content: 'I\'m having trouble connecting to the AI service right now. Please try again in a moment.',
      timestamp: new Date()
    })
  }
}

async function scrollToBottom() {
  await nextTick()
  if (messagesContainer.value) {
    messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight
  }
}

onMounted(() => {
  scrollToBottom()
})
</script>