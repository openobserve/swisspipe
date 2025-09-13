<template>
  <div v-if="visible" class="fixed inset-0 z-50 flex flex-col">
    <!-- Background overlay -->
    <div class="absolute inset-0 bg-black bg-opacity-90"></div>

    <!-- Full screen modal panel -->
    <div class="relative flex flex-col h-full bg-slate-800">
      <!-- Header -->
      <div class="bg-slate-700 px-6 py-4 border-b border-slate-600 flex-shrink-0">
        <div class="flex items-center justify-between">
          <h3 class="text-lg font-medium text-white">
            Workflow JSON
          </h3>
          <div class="flex items-center space-x-2">
            <button
              @click="copyToClipboard"
              class="bg-blue-600 hover:bg-blue-700 text-white px-3 py-1 rounded text-sm transition-colors"
            >
              {{ copied ? 'Copied!' : 'Copy' }}
            </button>
            <button
              @click="downloadJson"
              class="bg-green-600 hover:bg-green-700 text-white px-3 py-1 rounded text-sm transition-colors"
            >
              Download
            </button>
            <button
              @click="$emit('close')"
              class="text-gray-400 hover:text-gray-200 transition-colors"
            >
              <XMarkIcon class="h-6 w-6" />
            </button>
          </div>
        </div>
      </div>

      <!-- Content -->
      <div class="flex-1 bg-slate-800 px-6 py-4 overflow-hidden">
        <CodeEditor
          v-model="formattedJson"
          language="json"
          :readonly="true"
          @save="() => {}"
        />
      </div>

      <!-- Footer -->
      <div class="bg-slate-700 px-6 py-4 border-t border-slate-600 flex-shrink-0">
        <div class="flex justify-end">
          <button
            @click="$emit('close')"
            class="bg-gray-600 hover:bg-gray-700 text-white px-4 py-2 rounded-md font-medium transition-colors"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { XMarkIcon } from '@heroicons/vue/24/outline'
import CodeEditor from './CodeEditor.vue'

interface Props {
  visible: boolean
  jsonData: any
}

interface Emits {
  (e: 'close'): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

const copied = ref(false)

const formattedJson = computed(() => {
  if (!props.jsonData) return ''
  try {
    return JSON.stringify(props.jsonData, null, 2)
  } catch (error) {
    console.error('Error formatting JSON:', error)
    return 'Error formatting JSON'
  }
})

async function copyToClipboard() {
  try {
    await navigator.clipboard.writeText(formattedJson.value)
    copied.value = true
    setTimeout(() => {
      copied.value = false
    }, 2000)
  } catch (error) {
    console.error('Failed to copy to clipboard:', error)
  }
}

function downloadJson() {
  try {
    const dataStr = formattedJson.value
    const dataBlob = new Blob([dataStr], { type: 'application/json' })
    const url = URL.createObjectURL(dataBlob)
    
    // Create filename from workflow name and timestamp
    const workflowName = props.jsonData?.name || 'workflow'
    const timestamp = new Date().toISOString().replace(/[:.]/g, '-')
    const filename = `${workflowName}_${timestamp}.json`
    
    // Create download link and trigger download
    const link = document.createElement('a')
    link.href = url
    link.download = filename
    document.body.appendChild(link)
    link.click()
    
    // Cleanup
    document.body.removeChild(link)
    URL.revokeObjectURL(url)
  } catch (error) {
    console.error('Failed to download JSON:', error)
  }
}

// Keyboard event handler
function handleKeyDown(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    emit('close')
  }
}

// Add/remove keyboard event listener based on modal visibility
watch(() => props.visible, (newVisible) => {
  if (newVisible) {
    document.addEventListener('keydown', handleKeyDown)
    copied.value = false
  } else {
    document.removeEventListener('keydown', handleKeyDown)
    copied.value = false
  }
})

// Cleanup on component unmount
onUnmounted(() => {
  document.removeEventListener('keydown', handleKeyDown)
})
</script>

<style scoped>
/* Modal backdrop animation */
.fixed {
  backdrop-filter: blur(4px);
}
</style>