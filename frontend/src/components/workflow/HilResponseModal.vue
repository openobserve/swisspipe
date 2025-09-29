<template>
  <!-- Modal Backdrop -->
  <div
    v-if="isVisible"
    class="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center p-4"
    @click.self="handleClose"
  >
    <!-- Modal Content -->
    <div class="bg-white rounded-xl shadow-2xl w-full max-w-2xl max-h-[90vh] overflow-hidden">
      <!-- Modal Header -->
      <div class="flex items-center justify-between p-6 border-b border-gray-200">
        <div class="flex items-center space-x-3">
          <div class="w-8 h-8 bg-red-100 rounded-full flex items-center justify-center">
            <svg class="w-5 h-5 text-red-600" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
            </svg>
          </div>
          <div>
            <h2 class="text-xl font-semibold text-gray-900">Human Review Required</h2>
            <p class="text-sm text-gray-600">{{ task?.title || 'Review and Decision Required' }}</p>
          </div>
        </div>
        <button
          @click="handleClose"
          class="text-gray-400 hover:text-gray-600 transition-colors p-2 rounded-md hover:bg-gray-100"
          aria-label="Close"
        >
          <svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      <!-- Modal Body -->
      <div class="p-6 max-h-[60vh] overflow-y-auto">
        <!-- Task Details -->
        <div v-if="task" class="space-y-6">
          <!-- Description -->
          <div v-if="task.description" class="bg-blue-50 border border-blue-200 rounded-md p-4">
            <h3 class="text-sm font-medium text-blue-800 mb-2">Instructions</h3>
            <p class="text-sm text-blue-700 whitespace-pre-wrap">{{ task.description }}</p>
          </div>

          <!-- Event Data -->
          <div v-if="eventData" class="bg-gray-50 border border-gray-200 rounded-md p-4">
            <h3 class="text-sm font-medium text-gray-800 mb-2">Event Data</h3>
            <pre class="text-xs text-gray-600 bg-white p-3 rounded border overflow-x-auto">{{ eventDataJson }}</pre>
          </div>

          <!-- Required Fields -->
          <div v-if="task.required_fields && task.required_fields.length > 0">
            <h3 class="text-sm font-medium text-gray-700 mb-3">Required Information</h3>
            <div class="space-y-3">
              <div v-for="field in task.required_fields" :key="field" class="space-y-1">
                <label class="block text-sm font-medium text-gray-700">{{ field }}</label>
                <input
                  v-model="requiredFieldValues[field]"
                  type="text"
                  class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                  :placeholder="`Enter ${field}...`"
                />
              </div>
            </div>
          </div>

          <!-- Additional Comments -->
          <div>
            <label class="block text-sm font-medium text-gray-700 mb-2">
              Comments <span class="text-gray-500">(Optional)</span>
            </label>
            <textarea
              v-model="comments"
              rows="3"
              class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
              placeholder="Add any additional comments or reasoning..."
            />
          </div>

          <!-- Additional Data -->
          <div>
            <label class="block text-sm font-medium text-gray-700 mb-2">
              Additional Data <span class="text-gray-500">(Optional JSON)</span>
            </label>
            <textarea
              v-model="additionalData"
              rows="4"
              class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500 font-mono text-sm"
              placeholder='{"key": "value", "notes": "Additional structured data"}'
            />
            <p v-if="dataValidationError" class="text-xs text-red-600 mt-1">
              {{ dataValidationError }}
            </p>
          </div>

          <!-- Validation Errors -->
          <div v-if="validationErrors.length > 0" class="bg-red-50 border border-red-200 rounded-md p-3">
            <h4 class="text-sm font-medium text-red-800 mb-2">Please fix the following issues:</h4>
            <ul class="text-sm text-red-700 space-y-1">
              <li v-for="error in validationErrors" :key="error" class="flex items-center">
                <span class="w-1 h-1 bg-red-400 rounded-full mr-2"></span>
                {{ error }}
              </li>
            </ul>
          </div>

          <!-- Timeout Info -->
          <div v-if="task.timeout_at" class="bg-yellow-50 border border-yellow-200 rounded-md p-3">
            <div class="flex items-center">
              <svg class="w-4 h-4 text-yellow-600 mr-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                  d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
              <p class="text-sm text-yellow-800">
                <strong>Timeout:</strong> {{ formatTimeout(task.timeout_at) }}
              </p>
            </div>
            <p class="text-xs text-yellow-700 mt-1">
              Default action: {{ task.timeout_action || 'denied' }}
            </p>
          </div>
        </div>

        <!-- Loading State -->
        <div v-else class="flex items-center justify-center py-8">
          <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
          <span class="ml-3 text-gray-600">Loading task details...</span>
        </div>
      </div>

      <!-- Modal Footer -->
      <div class="flex items-center justify-between px-6 py-4 border-t border-gray-200 bg-gray-50">
        <div class="flex items-center space-x-2 text-sm text-gray-500">
          <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
              d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
          <span>Task ID: {{ task?.id || 'Unknown' }}</span>
        </div>

        <div class="flex items-center space-x-3">
          <button
            @click="handleClose"
            class="px-4 py-2 text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
            :disabled="isSubmitting"
          >
            Cancel
          </button>
          <button
            @click="submitDecision('denied')"
            class="px-4 py-2 text-white bg-red-600 border border-transparent rounded-md hover:bg-red-700 focus:outline-none focus:ring-2 focus:ring-red-500 focus:ring-offset-2 disabled:opacity-50"
            :disabled="isSubmitting || validationErrors.length > 0"
          >
            <span v-if="isSubmitting && submittingDecision === 'denied'" class="flex items-center">
              <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-white mr-2"></div>
              Denying...
            </span>
            <span v-else>Deny</span>
          </button>
          <button
            @click="submitDecision('approved')"
            class="px-4 py-2 text-white bg-green-600 border border-transparent rounded-md hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-green-500 focus:ring-offset-2 disabled:opacity-50"
            :disabled="isSubmitting || validationErrors.length > 0"
          >
            <span v-if="isSubmitting && submittingDecision === 'approved'" class="flex items-center">
              <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-white mr-2"></div>
              Approving...
            </span>
            <span v-else>Approve</span>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'

interface HilTask {
  id: string
  execution_id: string
  node_id: string
  node_execution_id: string
  workflow_id: string
  title: string
  description?: string
  status: string
  timeout_at?: string
  timeout_action?: string
  required_fields?: string[]
  metadata?: Record<string, any>
  response_data?: any
  response_received_at?: string
  created_at: string
  updated_at: string
}

interface Props {
  isVisible: boolean
  task: HilTask | null
  eventData?: any
  webhookUrl?: string
}

interface Emits {
  (e: 'close'): void
  (e: 'submitted', response: { decision: string; success: boolean; error?: string }): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

// Form data
const requiredFieldValues = ref<Record<string, string>>({})
const comments = ref('')
const additionalData = ref('')
const dataValidationError = ref('')
const isSubmitting = ref(false)
const submittingDecision = ref<string | null>(null)

// Computed
const eventDataJson = computed(() => {
  if (!props.eventData) return ''
  try {
    return JSON.stringify(props.eventData, null, 2)
  } catch {
    return 'Invalid JSON data'
  }
})

const validationErrors = computed(() => {
  const errors: string[] = []

  // Check required fields
  if (props.task?.required_fields) {
    for (const field of props.task.required_fields) {
      if (!requiredFieldValues.value[field]?.trim()) {
        errors.push(`${field} is required`)
      }
    }
  }

  // Check additional data JSON format
  if (additionalData.value.trim() && dataValidationError.value) {
    errors.push('Additional data must be valid JSON')
  }

  return errors
})

// Methods
const handleClose = () => {
  if (!isSubmitting.value) {
    emit('close')
  }
}

const validateAdditionalData = () => {
  if (!additionalData.value.trim()) {
    dataValidationError.value = ''
    return
  }

  try {
    JSON.parse(additionalData.value)
    dataValidationError.value = ''
  } catch {
    dataValidationError.value = 'Invalid JSON format'
  }
}

const submitDecision = async (decision: 'approved' | 'denied') => {
  if (validationErrors.value.length > 0 || isSubmitting.value) {
    return
  }

  if (!props.task || !props.webhookUrl) {
    emit('submitted', { decision, success: false, error: 'Missing task or webhook URL' })
    return
  }

  isSubmitting.value = true
  submittingDecision.value = decision

  try {
    // Prepare response data
    const responseData: Record<string, any> = {
      ...requiredFieldValues.value
    }

    // Add additional structured data if provided
    if (additionalData.value.trim()) {
      try {
        const parsedData = JSON.parse(additionalData.value)
        Object.assign(responseData, parsedData)
      } catch {
        // Already validated above, but just in case
        throw new Error('Invalid additional data JSON')
      }
    }

    // Prepare request parameters
    const params = new URLSearchParams({
      decision,
      ...(comments.value.trim() && { comments: comments.value }),
      ...(Object.keys(responseData).length > 0 && { data: JSON.stringify(responseData) }),
    })

    // Make API request
    const response = await fetch(`/api/v1/hil/${props.task.node_execution_id}/respond?${params}`, {
      method: 'GET',
      credentials: 'include'
    })

    if (!response.ok) {
      const errorText = await response.text()
      let errorMessage = `HTTP ${response.status}`
      try {
        const errorJson = JSON.parse(errorText)
        errorMessage = errorJson.error || errorMessage
      } catch {
        // Use status text if JSON parsing fails
        errorMessage = response.statusText || errorMessage
      }
      throw new Error(errorMessage)
    }

    const result = await response.json()

    emit('submitted', { decision, success: true })
    emit('close')

  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : 'Unknown error occurred'
    emit('submitted', { decision, success: false, error: errorMessage })
  } finally {
    isSubmitting.value = false
    submittingDecision.value = null
  }
}

const formatTimeout = (timeoutAt: string) => {
  try {
    const date = new Date(timeoutAt)
    const now = new Date()
    const diff = date.getTime() - now.getTime()

    if (diff <= 0) {
      return 'Expired'
    }

    const hours = Math.floor(diff / (1000 * 60 * 60))
    const minutes = Math.floor((diff % (1000 * 60 * 60)) / (1000 * 60))

    if (hours > 24) {
      const days = Math.floor(hours / 24)
      return `${days} day${days > 1 ? 's' : ''} remaining`
    } else if (hours > 0) {
      return `${hours}h ${minutes}m remaining`
    } else {
      return `${minutes} minute${minutes > 1 ? 's' : ''} remaining`
    }
  } catch {
    return 'Invalid timeout'
  }
}

// Reset form when task changes
watch(() => props.task, (newTask) => {
  if (newTask) {
    // Initialize required fields
    requiredFieldValues.value = {}
    if (newTask.required_fields) {
      for (const field of newTask.required_fields) {
        requiredFieldValues.value[field] = ''
      }
    }

    // Reset other fields
    comments.value = ''
    additionalData.value = ''
    dataValidationError.value = ''
    isSubmitting.value = false
    submittingDecision.value = null
  }
}, { immediate: true })

// Watch additional data for validation
watch(additionalData, validateAdditionalData)
</script>