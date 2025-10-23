<template>
  <div class="space-y-4 pb-4">
    <!-- Loading State -->
    <div v-if="isLoading" class="flex items-center justify-center p-8">
      <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-500"></div>
      <span class="ml-3 text-gray-400">Loading schedule...</span>
    </div>

    <!-- Main Form -->
    <div v-else class="space-y-4">
      <!-- Schedule Name (Optional) -->
      <div>
        <label class="block text-sm font-medium text-gray-300 mb-2">
          Schedule Name (optional)
        </label>
        <input
          v-model="formData.schedule_name"
          type="text"
          placeholder="e.g., Daily Report Generation"
          class="w-full px-3 py-2 bg-slate-700 border border-gray-600 rounded-md text-white text-sm focus:outline-none focus:ring-2 focus:ring-primary-500"
        />
      </div>

      <!-- Cron Expression & Timezone -->
      <div class="grid grid-cols-2 gap-4">
        <div>
          <label class="block text-sm font-medium text-gray-300 mb-2">
            Cron Expression <span class="text-red-400">*</span>
          </label>
          <input
            v-model="formData.cron_expression"
            type="text"
            placeholder="0 */5 * * * *"
            class="w-full px-3 py-2 bg-slate-700 border border-gray-600 rounded-md text-white text-sm focus:outline-none focus:ring-2 focus:ring-primary-500 font-mono"
            @blur="validateCronExpression"
          />
          <p class="text-xs text-gray-500 mt-1">Format: second minute hour day month weekday</p>
        </div>

        <div>
          <label class="block text-sm font-medium text-gray-300 mb-2">
            Timezone <span class="text-red-400">*</span>
          </label>
          <select
            v-model="formData.timezone"
            class="w-full px-3 py-2 bg-slate-700 border border-gray-600 rounded-md text-white text-sm focus:outline-none focus:ring-2 focus:ring-primary-500"
            @change="validateCronExpression"
          >
            <option value="UTC">UTC</option>
            <option value="America/New_York">America/New_York (EST/EDT)</option>
            <option value="America/Chicago">America/Chicago (CST/CDT)</option>
            <option value="America/Denver">America/Denver (MST/MDT)</option>
            <option value="America/Los_Angeles">America/Los_Angeles (PST/PDT)</option>
            <option value="Europe/London">Europe/London</option>
            <option value="Europe/Paris">Europe/Paris</option>
            <option value="Asia/Tokyo">Asia/Tokyo</option>
            <option value="Asia/Shanghai">Asia/Shanghai</option>
            <option value="Asia/Kolkata">Asia/Kolkata</option>
            <option value="Australia/Sydney">Australia/Sydney</option>
          </select>
        </div>
      </div>

      <!-- Cron Validation Results -->
      <div v-if="cronValidation.checked" class="bg-slate-800 p-3 rounded-md">
        <div v-if="cronValidation.valid" class="text-green-400">
          <div class="flex items-center space-x-2 mb-2">
            <svg class="h-5 w-5" fill="currentColor" viewBox="0 0 20 20">
              <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clip-rule="evenodd" />
            </svg>
            <span class="font-medium">Valid Cron Expression</span>
          </div>
          <div class="text-xs text-gray-400">
            <p class="font-medium mb-1">Next 5 executions:</p>
            <ul class="space-y-1">
              <li v-for="(time, index) in cronValidation.nextExecutions" :key="index" class="font-mono">
                {{ formatExecutionTime(time) }}
              </li>
            </ul>
          </div>
        </div>
        <div v-else class="text-red-400">
          <div class="flex items-center space-x-2">
            <svg class="h-5 w-5" fill="currentColor" viewBox="0 0 20 20">
              <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z" clip-rule="evenodd" />
            </svg>
            <span class="font-medium">Invalid Cron Expression</span>
          </div>
          <p class="text-xs mt-1">{{ cronValidation.error }}</p>
        </div>
      </div>

      <!-- Common Cron Patterns -->
      <div class="bg-blue-900/20 border border-blue-700/50 p-3 rounded-md">
        <p class="text-sm text-blue-300 font-medium mb-2">Common Patterns:</p>
        <div class="grid grid-cols-2 gap-2 text-xs">
          <button
            v-for="pattern in commonPatterns"
            :key="pattern.expression"
            @click="formData.cron_expression = pattern.expression; validateCronExpression()"
            class="text-left px-2 py-1 bg-blue-800/30 hover:bg-blue-800/50 rounded transition-colors"
          >
            <code class="text-blue-300">{{ pattern.expression }}</code>
            <span class="text-gray-400 ml-2">- {{ pattern.description }}</span>
          </button>
        </div>
      </div>

      <!-- Date Range (Optional) -->
      <div class="grid grid-cols-2 gap-4">
        <div>
          <label class="block text-sm font-medium text-gray-300 mb-2">
            Start Date (optional)
          </label>
          <input
            v-model="formData.start_date"
            type="datetime-local"
            class="w-full px-3 py-2 bg-slate-700 border border-gray-600 rounded-md text-white text-sm focus:outline-none focus:ring-2 focus:ring-primary-500"
          />
          <p class="text-xs text-gray-500 mt-1">Schedule won't run before this date</p>
        </div>

        <div>
          <label class="block text-sm font-medium text-gray-300 mb-2">
            End Date (optional)
          </label>
          <input
            v-model="formData.end_date"
            type="datetime-local"
            class="w-full px-3 py-2 bg-slate-700 border border-gray-600 rounded-md text-white text-sm focus:outline-none focus:ring-2 focus:ring-primary-500"
          />
          <p class="text-xs text-gray-500 mt-1">Schedule won't run after this date</p>
        </div>
      </div>

      <!-- Test Payload -->
      <div>
        <label class="block text-sm font-medium text-gray-300 mb-2">
          Test Payload (JSON) <span class="text-red-400">*</span>
        </label>
        <div class="border border-gray-600 rounded-md overflow-hidden h-[200px]">
          <code-editor
            v-model="formData.test_payload"
            language="json"
            :show-format-button="true"
            :show-save-button="false"
            :show-run-button="false"
          />
        </div>
        <p class="text-xs text-gray-500 mt-1">This data will be sent to the workflow on each scheduled execution</p>
      </div>

      <!-- Enabled Toggle -->
      <div class="flex items-center justify-between p-3 bg-slate-800 rounded-md">
        <div>
          <p class="text-sm font-medium text-gray-300">Enable Schedule</p>
          <p class="text-xs text-gray-500">When enabled, the workflow will run according to the cron schedule</p>
        </div>
        <label class="relative inline-flex items-center cursor-pointer">
          <input
            v-model="formData.enabled"
            type="checkbox"
            class="sr-only peer"
          />
          <div class="w-11 h-6 bg-gray-600 peer-focus:outline-none peer-focus:ring-2 peer-focus:ring-primary-500 rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary-600"></div>
        </label>
      </div>

      <!-- Action Buttons -->
      <div class="flex items-center space-x-3 pt-2">
        <button
          @click="saveSchedule"
          :disabled="isSaving || !isFormValid"
          class="px-4 py-2 bg-primary-600 text-white rounded-md hover:bg-primary-700 disabled:bg-gray-600 disabled:cursor-not-allowed transition-colors text-sm font-medium flex items-center space-x-2"
        >
          <span v-if="isSaving" class="animate-spin rounded-full h-4 w-4 border-b-2 border-white"></span>
          <span>{{ isSaving ? 'Saving...' : (hasExistingSchedule ? 'Update Schedule' : 'Create Schedule') }}</span>
        </button>

        <button
          v-if="hasExistingSchedule"
          @click="deleteSchedule"
          :disabled="isDeleting"
          class="px-4 py-2 bg-red-600 text-white rounded-md hover:bg-red-700 disabled:bg-gray-600 disabled:cursor-not-allowed transition-colors text-sm font-medium"
        >
          {{ isDeleting ? 'Deleting...' : 'Delete Schedule' }}
        </button>
      </div>

      <!-- Success/Error Messages -->
      <div v-if="successMessage" class="bg-green-900/20 border border-green-700/50 p-3 rounded-md">
        <p class="text-sm text-green-300">{{ successMessage }}</p>
      </div>

      <div v-if="errorMessage" class="bg-red-900/20 border border-red-700/50 p-3 rounded-md">
        <p class="text-sm text-red-300">{{ errorMessage }}</p>
      </div>

      <!-- Schedule Info (if exists) -->
      <div v-if="hasExistingSchedule && scheduleInfo" class="bg-slate-800 p-3 rounded-md space-y-2">
        <p class="text-sm font-medium text-gray-300">Schedule Information</p>
        <div class="text-xs text-gray-400 space-y-1">
          <p><strong>Last Execution:</strong> {{ scheduleInfo.last_execution_time ? formatExecutionTime(scheduleInfo.last_execution_time) : 'Never' }}</p>
          <p><strong>Next Execution:</strong> {{ scheduleInfo.next_execution_time ? formatExecutionTime(scheduleInfo.next_execution_time) : 'Not scheduled' }}</p>
          <p><strong>Execution Count:</strong> {{ scheduleInfo.execution_count }}</p>
          <p><strong>Failure Count:</strong> {{ scheduleInfo.failure_count }}</p>
          <p><strong>Created:</strong> {{ formatExecutionTime(scheduleInfo.created_at) }}</p>
          <p><strong>Updated:</strong> {{ formatExecutionTime(scheduleInfo.updated_at) }}</p>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useWorkflowStore } from '../../stores/workflows'
import { apiClient } from '../../services/api'
import CodeEditor from '../common/CodeEditor.vue'
import type { Schedule, ScheduleFormData } from '../../types/schedule'

interface Props {
  nodeId: string
}

const props = defineProps<Props>()

const workflowStore = useWorkflowStore()

const isLoading = ref(false)
const isSaving = ref(false)
const isDeleting = ref(false)
const successMessage = ref('')
const errorMessage = ref('')

const hasExistingSchedule = ref(false)
const scheduleInfo = ref<Schedule | null>(null)

const formData = ref<ScheduleFormData>({
  schedule_name: '',
  cron_expression: '0 9 * * *', // Default: Daily at 9 AM
  timezone: 'UTC',
  test_payload: '{\n  "scheduled": true\n}',
  enabled: true,
  start_date: '',
  end_date: ''
})

const cronValidation = ref({
  checked: false,
  valid: false,
  nextExecutions: [] as string[],
  error: ''
})

const commonPatterns = [
  { expression: '0 */5 * * * *', description: 'Every 5 minutes' },
  { expression: '0 0 * * * *', description: 'Every hour' },
  { expression: '0 0 9 * * *', description: 'Daily at 9 AM' },
  { expression: '0 0 0 * * *', description: 'Daily at midnight' },
  { expression: '0 0 9 * * 1', description: 'Every Monday at 9 AM' },
  { expression: '0 0 0 1 * *', description: 'First day of month' }
]

const isFormValid = computed(() => {
  if (!formData.value.cron_expression || !formData.value.timezone) {
    return false
  }
  try {
    JSON.parse(formData.value.test_payload)
  } catch {
    return false
  }
  return cronValidation.value.valid
})

// Load existing schedule
const loadSchedule = async () => {
  if (!workflowStore.currentWorkflow?.id) {
    return
  }

  isLoading.value = true
  try {
    const schedule = await apiClient.getSchedule(workflowStore.currentWorkflow.id, props.nodeId)
    if (schedule) {
      hasExistingSchedule.value = true
      scheduleInfo.value = schedule

      formData.value = {
        schedule_name: schedule.schedule_name || '',
        cron_expression: schedule.cron_expression,
        timezone: schedule.timezone,
        test_payload: JSON.stringify(schedule.test_payload, null, 2),
        enabled: schedule.enabled,
        start_date: schedule.start_date ? schedule.start_date.substring(0, 16) : '',
        end_date: schedule.end_date ? schedule.end_date.substring(0, 16) : ''
      }

      await validateCronExpression()
    }
  } catch (error) {
    console.error('Failed to load schedule:', error)
  } finally {
    isLoading.value = false
  }
}

// Validate cron expression
const validateCronExpression = async () => {
  if (!formData.value.cron_expression || !formData.value.timezone) {
    cronValidation.value.checked = false
    return
  }

  try {
    const result = await apiClient.validateCron(
      formData.value.cron_expression,
      formData.value.timezone
    )

    cronValidation.value = {
      checked: true,
      valid: result.valid,
      nextExecutions: result.next_executions,
      error: result.valid ? '' : result.next_executions[0] || 'Invalid cron expression'
    }
  } catch (error) {
    cronValidation.value = {
      checked: true,
      valid: false,
      nextExecutions: [],
      error: 'Failed to validate cron expression'
    }
  }
}

// Save schedule
const saveSchedule = async () => {
  if (!workflowStore.currentWorkflow?.id || !isFormValid.value) {
    return
  }

  isSaving.value = true
  successMessage.value = ''
  errorMessage.value = ''

  try {
    const payload = JSON.parse(formData.value.test_payload)

    const config = {
      schedule_name: formData.value.schedule_name || undefined,
      cron_expression: formData.value.cron_expression,
      timezone: formData.value.timezone,
      test_payload: payload,
      enabled: formData.value.enabled,
      start_date: formData.value.start_date || undefined,
      end_date: formData.value.end_date || undefined
    }

    const result = await apiClient.upsertSchedule(
      workflowStore.currentWorkflow.id,
      props.nodeId,
      config
    )

    hasExistingSchedule.value = true
    scheduleInfo.value = result
    successMessage.value = `Schedule ${hasExistingSchedule.value ? 'updated' : 'created'} successfully!`

    setTimeout(() => {
      successMessage.value = ''
    }, 3000)
  } catch (error) {
    errorMessage.value = 'Failed to save schedule: ' + (error as Error).message
  } finally {
    isSaving.value = false
  }
}

// Delete schedule
const deleteSchedule = async () => {
  if (!workflowStore.currentWorkflow?.id) {
    return
  }

  if (!confirm('Are you sure you want to delete this schedule?')) {
    return
  }

  isDeleting.value = true
  successMessage.value = ''
  errorMessage.value = ''

  try {
    await apiClient.deleteSchedule(workflowStore.currentWorkflow.id, props.nodeId)

    hasExistingSchedule.value = false
    scheduleInfo.value = null
    successMessage.value = 'Schedule deleted successfully!'

    setTimeout(() => {
      successMessage.value = ''
    }, 3000)
  } catch (error) {
    errorMessage.value = 'Failed to delete schedule: ' + (error as Error).message
  } finally {
    isDeleting.value = false
  }
}

// Format execution time
const formatExecutionTime = (time: string): string => {
  try {
    return new Date(time).toLocaleString()
  } catch {
    return time
  }
}

// Watch for cron expression changes
watch(
  () => formData.value.cron_expression,
  () => {
    // Debounce validation
    const timer = setTimeout(() => {
      validateCronExpression()
    }, 500)
    return () => clearTimeout(timer)
  }
)

onMounted(() => {
  loadSchedule()
})
</script>
