<template>
  <div v-if="visible" class="fixed inset-0 z-50 flex items-center justify-center">
    <!-- Backdrop -->
    <div 
      class="absolute inset-0 bg-black/50 backdrop-blur-sm"
      @click="close"
    ></div>
    
    <!-- Modal Content -->
    <div class="relative max-w-6xl w-full mx-4 h-[80vh] bg-slate-900 rounded-lg shadow-2xl border border-slate-700/50 flex">
      <!-- Header -->
      <div class="absolute top-0 left-0 right-0 px-6 py-4 border-b border-slate-700/50 flex items-center justify-between">
        <div>
          <h2 class="text-xl font-semibold text-gray-200">Node Execution Details</h2>
          <p class="text-sm text-gray-400">{{ nodeData?.label || 'Unknown Node' }}</p>
        </div>
        <button
          @click="close"
          class="text-gray-400 hover:text-gray-200 transition-colors"
        >
          <XMarkIcon class="h-6 w-6" />
        </button>
      </div>
      
      <!-- Content -->
      <div class="mt-20 flex-1 flex pb-20">
        <!-- Input Panel -->
        <div class="flex-1 border-r border-slate-700/50 flex flex-col">
          <div class="px-4 py-3 bg-slate-800/50 border-b border-slate-700/50">
            <h3 class="text-sm font-medium text-gray-200 flex items-center">
              <ArrowDownIcon class="h-4 w-4 mr-2 text-blue-400" />
              Input Data
            </h3>
          </div>
          <div class="flex-1">
            <div class="h-full border border-slate-600 rounded">
              <div 
                ref="inputEditor" 
                class="h-full w-full"
              ></div>
            </div>
          </div>
        </div>
        
        <!-- Output Panel -->
        <div class="flex-1 flex flex-col">
          <div class="px-4 py-3 bg-slate-800/50 border-b border-slate-700/50">
            <h3 class="text-sm font-medium text-gray-200 flex items-center">
              <ArrowUpIcon class="h-4 w-4 mr-2 text-green-400" />
              Output Data
            </h3>
          </div>
          <div class="flex-1 relative">
            <!-- Error overlay -->
            <div v-if="hasError" class="absolute top-2 left-2 right-2 z-10 p-3 bg-red-900/90 border border-red-500/50 rounded-lg backdrop-blur-sm">
              <div class="flex items-center mb-2">
                <ExclamationTriangleIcon class="h-5 w-5 text-red-400 mr-2" />
                <span class="text-red-400 font-medium">Error</span>
              </div>
              <p class="text-red-300 text-sm">{{ nodeData?.executionError }}</p>
            </div>
            <!-- Editor container -->
            <div class="h-full border border-slate-600 rounded">
              <div 
                ref="outputEditor" 
                class="h-full w-full"
              ></div>
            </div>
          </div>
        </div>
      </div>
      
      <!-- Footer -->
      <div class="absolute bottom-0 left-0 right-0 px-6 py-4 bg-slate-800/50 border-t border-slate-700/50 rounded-b-lg">
        <div class="flex items-center justify-between text-sm">
          <div class="flex items-center space-x-4 text-gray-400">
            <div class="flex items-center">
              <div class="w-2 h-2 rounded-full mr-2" :class="statusColor"></div>
              <span>Status: {{ nodeData?.executionStatus || 'Unknown' }}</span>
            </div>
            <div v-if="nodeData?.executionDuration">
              Duration: {{ formatDuration(nodeData.executionDuration) }}
            </div>
          </div>
          <div class="flex space-x-3">
            <button
              @click="copyToClipboard('input')"
              class="px-3 py-1.5 bg-blue-600 hover:bg-blue-700 text-white rounded-md text-sm transition-colors"
            >
              Copy Input
            </button>
            <button
              @click="copyToClipboard('output')"
              class="px-3 py-1.5 bg-green-600 hover:bg-green-700 text-white rounded-md text-sm transition-colors"
            >
              Copy Output
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch, onMounted, onUnmounted, nextTick } from 'vue'
import { XMarkIcon, ArrowDownIcon, ArrowUpIcon, ExclamationTriangleIcon } from '@heroicons/vue/24/outline'
import * as monaco from 'monaco-editor'

interface Props {
  visible: boolean
  nodeData: unknown
}

const props = defineProps<Props>()
const emit = defineEmits(['close'])

// Monaco Editor refs
const inputEditor = ref<HTMLElement>()
const outputEditor = ref<HTMLElement>()
let inputMonacoEditor: monaco.editor.IStandaloneCodeEditor | null = null
let outputMonacoEditor: monaco.editor.IStandaloneCodeEditor | null = null

const inputData = computed(() => {
  if (!props.nodeData?.executionInput) return { message: 'No input data available' }
  return props.nodeData.executionInput
})

const outputData = computed(() => {
  if (hasError.value) return { message: 'Node execution failed - see error above' }
  if (!props.nodeData?.executionOutput) return { message: 'No output data available' }
  return props.nodeData.executionOutput
})

const hasError = computed(() => {
  return !!(props.nodeData?.executionError)
})

const statusColor = computed(() => {
  switch (props.nodeData?.executionStatus) {
    case 'completed':
      return 'bg-green-500'
    case 'failed':
      return 'bg-red-500'
    case 'running':
      return 'bg-blue-500'
    case 'pending':
      return 'bg-yellow-500'
    case 'skipped':
      return 'bg-gray-500'
    default:
      return 'bg-gray-500'
  }
})

function close() {
  emit('close')
}

function formatDuration(durationMs: number | null): string {
  if (!durationMs) return 'N/A'
  
  if (durationMs < 1000) return `${durationMs}ms`
  if (durationMs < 60000) return `${(durationMs / 1000).toFixed(1)}s`
  return `${(durationMs / 60000).toFixed(1)}m`
}

async function copyToClipboard(type: 'input' | 'output') {
  try {
    const editor = type === 'input' ? inputMonacoEditor : outputMonacoEditor
    if (editor) {
      // Get the formatted content from Monaco editor
      const content = editor.getValue()
      await navigator.clipboard.writeText(content)
    } else {
      // Fallback to data if editor not available
      const data = type === 'input' ? inputData.value : outputData.value
      const jsonString = JSON.stringify(data, null, 2)
      await navigator.clipboard.writeText(jsonString)
    }
    // TODO: Show success toast
    console.log(`${type} data copied to clipboard`)
  } catch (error) {
    console.error('Failed to copy to clipboard:', error)
    // TODO: Show error toast
  }
}

// Monaco Editor setup
function initializeEditors() {
  if (inputEditor.value && !inputMonacoEditor) {
    inputMonacoEditor = monaco.editor.create(inputEditor.value, {
      value: JSON.stringify(inputData.value, null, 2),
      language: 'json',
      theme: 'vs-dark',
      readOnly: true,
      minimap: { enabled: false },
      scrollBeyondLastLine: false,
      wordWrap: 'on',
      folding: true,
      lineNumbers: 'on',
      automaticLayout: true,
      fontSize: 13,
      tabSize: 2
    })
  }
  
  if (outputEditor.value && !outputMonacoEditor) {
    outputMonacoEditor = monaco.editor.create(outputEditor.value, {
      value: JSON.stringify(outputData.value, null, 2),
      language: 'json',
      theme: 'vs-dark',
      readOnly: true,
      minimap: { enabled: false },
      scrollBeyondLastLine: false,
      wordWrap: 'on',
      folding: true,
      lineNumbers: 'on',
      automaticLayout: true,
      fontSize: 13,
      tabSize: 2
    })
  }
}

function updateEditorContent() {
  if (inputMonacoEditor) {
    inputMonacoEditor.setValue(JSON.stringify(inputData.value, null, 2))
  }
  if (outputMonacoEditor) {
    outputMonacoEditor.setValue(JSON.stringify(outputData.value, null, 2))
  }
}

function disposeEditors() {
  if (inputMonacoEditor) {
    inputMonacoEditor.dispose()
    inputMonacoEditor = null
  }
  if (outputMonacoEditor) {
    outputMonacoEditor.dispose()
    outputMonacoEditor = null
  }
}

// Lifecycle hooks
onMounted(async () => {
  await nextTick()
  if (props.visible) {
    initializeEditors()
  }
})

onUnmounted(() => {
  disposeEditors()
})

// Watch for data changes
watch([inputData, outputData], () => {
  updateEditorContent()
}, { deep: true })

// Handle visibility changes
watch(() => props.visible, async (visible) => {
  if (visible) {
    await nextTick()
    initializeEditors()
    updateEditorContent()
    
    const handleEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        close()
        document.removeEventListener('keydown', handleEscape)
      }
    }
    document.addEventListener('keydown', handleEscape)
  } else {
    disposeEditors()
  }
})
</script>

<style scoped>
/* Custom scrollbar for dark theme */
.overflow-auto::-webkit-scrollbar {
  width: 8px;
}

.overflow-auto::-webkit-scrollbar-track {
  background: rgba(30, 41, 59, 0.5);
  border-radius: 4px;
}

.overflow-auto::-webkit-scrollbar-thumb {
  background: rgba(100, 116, 139, 0.5);
  border-radius: 4px;
}

.overflow-auto::-webkit-scrollbar-thumb:hover {
  background: rgba(100, 116, 139, 0.8);
}
</style>