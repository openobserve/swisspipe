<template>
  <div class="email-content-editor">
    <div class="flex items-center justify-between mb-2">
      <div class="flex items-center space-x-2">
        <span class="text-sm text-gray-300">
          {{ contentType === 'html' ? 'HTML Editor' : 'Text Editor' }}
        </span>
        <button
          v-if="contentType === 'html'"
          @click="showPreview = !showPreview"
          class="px-2 py-1 bg-gray-600 hover:bg-gray-500 rounded text-xs text-white transition-colors"
        >
          {{ showPreview ? 'Hide Preview' : 'Show Preview' }}
        </button>
      </div>
      
      <button
        @click="showVariableHelper = !showVariableHelper"
        class="px-2 py-1 bg-blue-600 hover:bg-blue-700 rounded text-xs text-white transition-colors"
      >
        {{ showVariableHelper ? 'Hide' : 'Show' }} Variables
      </button>
    </div>

    <div class="grid" :class="showPreview && contentType === 'html' ? 'grid-cols-2 gap-4' : 'grid-cols-1'">
      <!-- Editor -->
      <div class="relative">
        <textarea
          v-if="contentType === 'text'"
          v-model="localContent"
          @input="onInput"
          @blur="onBlur"
          :rows="Math.max(6, height / 20)"
          placeholder="Enter your email template here. Use {{ event.data.field }} for dynamic content."
          class="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded-md text-white focus:outline-none focus:ring-2 focus:ring-blue-500 font-mono text-sm"
        />
        
        <div
          v-else
          class="border border-gray-600 rounded-md overflow-hidden"
        >
          <!-- Simple HTML editor using textarea for now -->
          <!-- In a real implementation, you'd use @monaco-editor/vue here -->
          <div class="bg-gray-800 px-3 py-1 border-b border-gray-600">
            <div class="flex space-x-2">
              <button
                @click="insertHtmlTag('h1')"
                class="px-2 py-1 bg-gray-600 hover:bg-gray-500 rounded text-xs text-white"
              >
                H1
              </button>
              <button
                @click="insertHtmlTag('p')"
                class="px-2 py-1 bg-gray-600 hover:bg-gray-500 rounded text-xs text-white"
              >
                P
              </button>
              <button
                @click="insertHtmlTag('strong')"
                class="px-2 py-1 bg-gray-600 hover:bg-gray-500 rounded text-xs text-white"
              >
                Bold
              </button>
              <button
                @click="insertVariable('{{ event.name }}')"
                class="px-2 py-1 bg-blue-600 hover:bg-blue-700 rounded text-xs text-white"
              >
                + Variable
              </button>
            </div>
          </div>
          
          <textarea
            v-model="localContent"
            @input="onInput"
            @blur="onBlur"
            :rows="Math.max(8, height / 20)"
            placeholder="<!DOCTYPE html>
<html>
<body>
  <h1>{{ event.name }}</h1>
  <p>Status: {{ event.status }}</p>
  <p>Data: {{ event.data  }}</p>
</body>
</html>"
            class="w-full px-3 py-2 bg-gray-700 border-0 text-white focus:outline-none focus:ring-2 focus:ring-blue-500 font-mono text-sm resize-none"
            style="min-height: 200px"
          />
        </div>
      </div>

      <!-- Preview (HTML only) -->
      <div v-if="showPreview && contentType === 'html'" class="border border-gray-600 rounded-md">
        <div class="bg-gray-800 px-3 py-1 border-b border-gray-600">
          <span class="text-xs text-gray-300">Preview</span>
        </div>
        <div 
          class="p-3 bg-white text-black min-h-48 overflow-auto"
          v-html="renderedPreview"
        />
      </div>
    </div>

    <!-- Variable Helper Panel -->
    <div v-if="showVariableHelper" class="mt-4 p-3 bg-gray-700 rounded-md">
      <div class="text-sm font-medium text-gray-300 mb-2">Template Variables</div>
      <div class="grid grid-cols-2 gap-2 text-xs">
        <button
          v-for="variable in templateVariables"
          :key="variable.path"
          @click="insertVariable(variable.path)"
          class="px-2 py-1 bg-gray-600 hover:bg-gray-500 rounded text-left text-white transition-colors"
          :title="variable.description"
        >
          <div class="font-mono">{{ variable.path }}</div>
          <div class="text-gray-400 text-xs">{{ variable.description }}</div>
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, computed } from 'vue'

interface Props {
  modelValue: string
  contentType: 'html' | 'text'
  height?: number
}

interface Emits {
  (e: 'update:modelValue', value: string): void
  (e: 'input'): void
  (e: 'blur'): void
}

const props = withDefaults(defineProps<Props>(), {
  height: 300
})

const emit = defineEmits<Emits>()

const localContent = ref(props.modelValue)
const showPreview = ref(false)
const showVariableHelper = ref(false)

const templateVariables = [
  { path: '{{ event.name }}', description: 'Workflow name' },
  { path: '{{ event.status }}', description: 'Execution status' },
  { path: '{{ event.id }}', description: 'Execution ID' },
  { path: '{{ event.data }}', description: 'Raw workflow data' },
  { path: '{{ json event.data  }}', description: 'Formatted JSON data' },
  { path: '{{ event.metadata }}', description: 'Workflow metadata' },
  { path: '{{ event.headers }}', description: 'HTTP headers' },
  { path: '{{ node.id }}', description: 'Current node ID' },
  { path: '{{ system.timestamp }}', description: 'Current timestamp' },
  { path: '{{ system.hostname }}', description: 'Server hostname' }
]

// Simple preview renderer (in real app, use proper template engine)
const renderedPreview = computed(() => {
  let content = localContent.value
  
  // Replace common template variables with sample data
  content = content.replace(/\{\{\s*workflow\.name\s*\}\}/g, 'Sample Workflow')
  content = content.replace(/\{\{\s*workflow\.status\s*\}\}/g, 'completed')
  content = content.replace(/\{\{\s*workflow\.id\s*\}\}/g, 'exec-123-456')
  content = content.replace(/\{\{\s*workflow\.data\s*\|\s*json\s*\}\}/g, '{"user": "john", "count": 42}')
  content = content.replace(/\{\{\s*system\.timestamp\s*\}\}/g, new Date().toISOString())
  content = content.replace(/\{\{\s*system\.hostname\s*\}\}/g, 'swisspipe-server')
  
  return content
})

// Watch for external changes
// Watch for external changes - but don't override if user is actively typing
let userIsTyping = false
watch(
  () => props.modelValue,
  (newValue) => {
    console.log('EmailContentEditor props watcher triggered:', {
      newValue,
      currentLocal: localContent.value,
      userIsTyping
    })
    
    // Only update if user is not actively typing
    if (!userIsTyping) {
      localContent.value = newValue
    }
  }
)

// Debounced emit function for content changes
let contentTimeout: ReturnType<typeof setTimeout> | null = null
const emitContentUpdate = () => {
  if (contentTimeout) clearTimeout(contentTimeout)
  contentTimeout = setTimeout(() => {
    emit('update:modelValue', localContent.value)
  }, 200) // Longer debounce for content editing
}

// Input and blur handlers
const onInput = () => {
  userIsTyping = true
  emit('input')
}

const onBlur = () => {
  userIsTyping = false
  emit('blur')
}

// Emit changes with debouncing
watch(
  localContent,
  () => emitContentUpdate()
)

function insertVariable(variable: string) {
  const textarea = document.querySelector('textarea') as HTMLTextAreaElement
  if (textarea) {
    const start = textarea.selectionStart
    const end = textarea.selectionEnd
    const text = localContent.value
    localContent.value = text.substring(0, start) + variable + text.substring(end)
    
    // Move cursor after inserted variable
    setTimeout(() => {
      textarea.selectionStart = textarea.selectionEnd = start + variable.length
      textarea.focus()
    }, 0)
  } else {
    // Fallback: append to end
    localContent.value += variable
  }
}

function insertHtmlTag(tag: string) {
  const openTag = `<${tag}>`
  const closeTag = `</${tag}>`
  const textarea = document.querySelector('textarea') as HTMLTextAreaElement
  
  if (textarea) {
    const start = textarea.selectionStart
    const end = textarea.selectionEnd
    const selectedText = localContent.value.substring(start, end)
    const replacement = `${openTag}${selectedText || 'content'}${closeTag}`
    
    localContent.value = 
      localContent.value.substring(0, start) + 
      replacement + 
      localContent.value.substring(end)
    
    setTimeout(() => {
      textarea.selectionStart = start + openTag.length
      textarea.selectionEnd = start + openTag.length + (selectedText || 'content').length
      textarea.focus()
    }, 0)
  }
}
</script>

<style scoped>
.email-content-editor textarea {
  resize: vertical;
  min-height: 150px;
}
</style>