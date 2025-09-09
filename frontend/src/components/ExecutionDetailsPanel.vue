<template>
  <!-- Slide-out panel overlay -->
  <div 
    v-if="executionStore.showSidePanel" 
    class="fixed inset-0 z-50 overflow-hidden"
  >
    <!-- Background overlay -->
    <div 
      class="absolute inset-0 bg-black/30 backdrop-blur-sm transition-opacity"
      @click="executionStore.closeSidePanel"
    ></div>
    
    <!-- Side panel -->
    <div class="absolute right-0 top-0 h-full w-[90%] glass-strong shadow-2xl transform transition-transform flex flex-col">
      <!-- Header -->
      <div class="flex items-center justify-between p-6 border-b border-slate-700/50 flex-shrink-0">
        <h2 class="text-lg font-semibold text-white">Execution Details</h2>
        <button 
          @click="executionStore.closeSidePanel"
          class="text-gray-400 hover:text-white transition-colors"
        >
          <XMarkIcon class="h-6 w-6" />
        </button>
      </div>
      
      <!-- Content -->
      <div v-if="executionStore.selectedExecution" class="flex-1 overflow-y-auto p-6 space-y-6">
        <!-- Basic Info, Timing, and Input Data -->
        <div class="grid grid-cols-3 gap-6">
          <!-- Basic Information -->
          <div>
            <h3 class="text-sm font-medium text-gray-200 mb-3">Basic Information</h3>
            <div class="space-y-2">
              <div>
                <span class="text-xs text-gray-400">Execution ID</span>
                <div class="text-sm text-white font-mono">{{ executionStore.selectedExecution.id }}</div>
              </div>
              <div>
                <span class="text-xs text-gray-400">Workflow ID</span>
                <div class="text-sm text-white font-mono">{{ executionStore.selectedExecution.workflow_id }}</div>
              </div>
              <div>
                <span class="text-xs text-gray-400">Status</span>
                <div class="mt-1">
                  <span 
                    class="px-2 py-1 text-xs leading-4 font-semibold rounded-full"
                    :class="getStatusColorClass(executionStore.selectedExecution.status)"
                  >
                    {{ executionStore.selectedExecution.status }}
                  </span>
                </div>
              </div>
              <div v-if="executionStore.selectedExecution.current_node_name">
                <span class="text-xs text-gray-400">Current Node</span>
                <div class="text-sm text-white">{{ executionStore.selectedExecution.current_node_name }}</div>
              </div>
            </div>
          </div>

          <!-- Timing -->
          <div>
            <h3 class="text-sm font-medium text-gray-200 mb-3">Timing</h3>
            <div class="space-y-2">
              <div>
                <span class="text-xs text-gray-400">Created</span>
                <div class="text-sm text-white">{{ executionStore.formatTimestamp(executionStore.selectedExecution.created_at) }}</div>
              </div>
              <div v-if="executionStore.selectedExecution.started_at">
                <span class="text-xs text-gray-400">Started</span>
                <div class="text-sm text-white">{{ executionStore.formatTimestamp(executionStore.selectedExecution.started_at) }}</div>
              </div>
              <div v-if="executionStore.selectedExecution.completed_at">
                <span class="text-xs text-gray-400">Completed</span>
                <div class="text-sm text-white">{{ executionStore.formatTimestamp(executionStore.selectedExecution.completed_at) }}</div>
              </div>
              <div>
                <span class="text-xs text-gray-400">Duration</span>
                <div class="text-sm text-white">{{ executionStore.formatDuration(executionStore.selectedExecution.started_at, executionStore.selectedExecution.completed_at) }}</div>
              </div>
            </div>
          </div>

          <!-- Input Data -->
          <div>
            <h3 class="text-sm font-medium text-gray-200 mb-3">Input Data</h3>
            <div v-if="executionStore.selectedExecution.input_data" class="bg-slate-800/80 border border-slate-600/70 rounded-md p-4 max-h-48 overflow-y-auto">
              <pre class="text-xs text-gray-100 whitespace-pre-wrap font-mono leading-relaxed">{{ formatJson(executionStore.selectedExecution.input_data) }}</pre>
            </div>
            <div v-else class="bg-slate-800/80 border border-slate-600/70 rounded-md p-4 text-center">
              <span class="text-gray-400 text-xs">No input data</span>
            </div>
          </div>
        </div>

        <!-- Error Message -->
        <div v-if="executionStore.selectedExecution.error_message">
          <h3 class="text-sm font-medium text-gray-200 mb-3">Error</h3>
          <div class="p-4 bg-red-900/30 border border-red-600/60 rounded-md">
            <p class="text-sm text-red-100 font-medium leading-relaxed">{{ executionStore.selectedExecution.error_message }}</p>
          </div>
        </div>


        <!-- Output Data -->
        <div v-if="executionStore.selectedExecution.output_data">
          <h3 class="text-sm font-medium text-gray-200 mb-3">Output Data</h3>
          <div class="bg-slate-800/80 border border-slate-600/70 rounded-md p-4">
            <pre class="text-sm text-gray-100 whitespace-pre-wrap font-mono leading-relaxed">{{ formatJson(executionStore.selectedExecution.output_data) }}</pre>
          </div>
        </div>

        <!-- Steps -->
        <div>
          <h3 class="text-sm font-medium text-gray-200 mb-3">Execution Steps</h3>
          <div v-if="executionStore.executionSteps.length === 0" class="text-sm text-gray-400 text-center py-4">
            No steps recorded yet
          </div>
          <div v-else class="bg-slate-800/80 border border-slate-600/70 rounded-md overflow-hidden">
            <div class="overflow-x-auto">
              <table class="min-w-full divide-y divide-slate-600">
                <thead class="bg-slate-700/50">
                  <tr>
                    <th class="px-4 py-2 text-left text-xs font-medium text-gray-300 uppercase tracking-wider">
                      Node Name
                    </th>
                    <th class="px-4 py-2 text-left text-xs font-medium text-gray-300 uppercase tracking-wider">
                      Status
                    </th>
                    <th class="px-4 py-2 text-left text-xs font-medium text-gray-300 uppercase tracking-wider">
                      Created
                    </th>
                    <th class="px-4 py-2 text-left text-xs font-medium text-gray-300 uppercase tracking-wider">
                      Started
                    </th>
                    <th class="px-4 py-2 text-left text-xs font-medium text-gray-300 uppercase tracking-wider">
                      Completed
                    </th>
                    <th class="px-4 py-2 text-left text-xs font-medium text-gray-300 uppercase tracking-wider">
                      Duration
                    </th>
                    <th class="px-4 py-2 text-left text-xs font-medium text-gray-300 uppercase tracking-wider">
                      Error
                    </th>
                  </tr>
                </thead>
                <tbody class="divide-y divide-slate-600/50">
                  <tr 
                    v-for="step in executionStore.executionSteps" 
                    :key="step.id"
                    class="hover:bg-slate-700/30 transition-colors cursor-pointer"
                    @click="openStepModal(step)"
                  >
                    <td class="px-4 py-2 text-sm font-medium text-white">
                      {{ step.node_name }}
                    </td>
                    <td class="px-4 py-2 text-sm">
                      <span 
                        class="px-2 py-1 text-xs leading-4 font-semibold rounded-full"
                        :class="getStepStatusColorClass(step.status)"
                      >
                        {{ step.status }}
                      </span>
                    </td>
                    <td class="px-4 py-2 text-xs text-gray-200 font-mono">
                      {{ executionStore.formatTimestamp(step.created_at) }}
                    </td>
                    <td class="px-4 py-2 text-xs text-gray-200 font-mono">
                      {{ step.started_at ? executionStore.formatTimestamp(step.started_at) : '-' }}
                    </td>
                    <td class="px-4 py-2 text-xs text-gray-200 font-mono">
                      {{ step.completed_at ? executionStore.formatTimestamp(step.completed_at) : '-' }}
                    </td>
                    <td class="px-4 py-2 text-xs text-gray-200 font-mono">
                      {{ step.started_at && step.completed_at ? executionStore.formatDuration(step.started_at, step.completed_at) : '-' }}
                    </td>
                    <td class="px-4 py-2 text-xs text-red-300 max-w-md">
                      <div class="whitespace-pre-wrap break-words">{{ step.error_message || '-' }}</div>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>

        <!-- Actions -->
        <div class="flex space-x-3 pt-4 border-t border-slate-700/50">
          <button
            v-if="executionStore.selectedExecution.status === 'running' || executionStore.selectedExecution.status === 'pending'"
            @click="cancelExecution"
            class="flex-1 bg-red-600 hover:bg-red-700 text-white px-4 py-2 rounded-md font-medium transition-colors"
          >
            Cancel Execution
          </button>
          <button
            @click="refreshExecution"
            class="flex-1 bg-primary-600 hover:bg-primary-700 text-white px-4 py-2 rounded-md font-medium transition-colors"
          >
            Refresh
          </button>
        </div>
      </div>
    </div>

    <!-- Step Details Modal -->
    <div 
      v-if="showStepModal" 
      class="fixed inset-0 z-60 overflow-hidden"
    >
      <!-- Background overlay -->
      <div 
        class="absolute inset-0 bg-black/50 backdrop-blur-sm transition-opacity"
        @click="closeStepModal"
      ></div>
      
      <!-- Modal panel -->
      <div class="absolute inset-4 glass-strong shadow-2xl transform transition-transform flex flex-col rounded-lg">
        <!-- Header -->
        <div class="flex items-center justify-between p-6 border-b border-slate-700/50 flex-shrink-0">
          <h2 class="text-lg font-semibold text-white">Step Details: {{ selectedStep?.node_name }}</h2>
          <button 
            @click="closeStepModal"
            class="text-gray-400 hover:text-white transition-colors"
          >
            <XMarkIcon class="h-6 w-6" />
          </button>
        </div>
        
        <!-- Content -->
        <div v-if="selectedStep" class="flex-1 overflow-y-auto p-6 space-y-6">
          <!-- Basic Step Info -->
          <div class="grid grid-cols-2 gap-8">
            <!-- Step Information -->
            <div>
              <h3 class="text-sm font-medium text-gray-200 mb-3">Step Information</h3>
              <div class="space-y-2">
                <div>
                  <span class="text-xs text-gray-400">Step ID</span>
                  <div class="text-sm text-white font-mono">{{ selectedStep.id }}</div>
                </div>
                <div>
                  <span class="text-xs text-gray-400">Node Name</span>
                  <div class="text-sm text-white">{{ selectedStep.node_name }}</div>
                </div>
                <div>
                  <span class="text-xs text-gray-400">Status</span>
                  <div class="mt-1">
                    <span 
                      class="px-2 py-1 text-xs leading-4 font-semibold rounded-full"
                      :class="getStepStatusColorClass(selectedStep.status)"
                    >
                      {{ selectedStep.status }}
                    </span>
                  </div>
                </div>
              </div>
            </div>

            <!-- Timing -->
            <div>
              <h3 class="text-sm font-medium text-gray-200 mb-3">Timing</h3>
              <div class="space-y-2">
                <div>
                  <span class="text-xs text-gray-400">Created</span>
                  <div class="text-sm text-white">{{ executionStore.formatTimestamp(selectedStep.created_at) }}</div>
                </div>
                <div v-if="selectedStep.started_at">
                  <span class="text-xs text-gray-400">Started</span>
                  <div class="text-sm text-white">{{ executionStore.formatTimestamp(selectedStep.started_at) }}</div>
                </div>
                <div v-if="selectedStep.completed_at">
                  <span class="text-xs text-gray-400">Completed</span>
                  <div class="text-sm text-white">{{ executionStore.formatTimestamp(selectedStep.completed_at) }}</div>
                </div>
                <div>
                  <span class="text-xs text-gray-400">Duration</span>
                  <div class="text-sm text-white">{{ selectedStep.started_at && selectedStep.completed_at ? executionStore.formatDuration(selectedStep.started_at, selectedStep.completed_at) : '-' }}</div>
                </div>
              </div>
            </div>
          </div>

          <!-- Error Message -->
          <div v-if="selectedStep.error_message">
            <h3 class="text-sm font-medium text-gray-200 mb-3">Error</h3>
            <div class="p-4 bg-red-900/30 border border-red-600/60 rounded-md">
              <p class="text-sm text-red-100 font-medium leading-relaxed whitespace-pre-wrap">{{ selectedStep.error_message }}</p>
            </div>
          </div>

          <!-- Input and Output Data Side by Side -->
          <div v-if="selectedStep.input_data || selectedStep.output_data" class="grid grid-cols-2 gap-6">
            <!-- Input Data -->
            <div v-if="selectedStep.input_data">
              <h3 class="text-sm font-medium text-gray-200 mb-3">Input Data</h3>
              <div class="bg-slate-800/80 border border-slate-600/70 rounded-md p-4">
                <pre class="text-sm text-gray-100 whitespace-pre-wrap font-mono leading-relaxed">{{ formatJson(selectedStep.input_data) }}</pre>
              </div>
            </div>
            <div v-else>
              <h3 class="text-sm font-medium text-gray-200 mb-3">Input Data</h3>
              <div class="bg-slate-800/80 border border-slate-600/70 rounded-md p-4 text-center">
                <span class="text-gray-400">No input data</span>
              </div>
            </div>

            <!-- Output Data -->
            <div v-if="selectedStep.output_data">
              <h3 class="text-sm font-medium text-gray-200 mb-3">Output Data</h3>
              <div class="bg-slate-800/80 border border-slate-600/70 rounded-md p-4">
                <pre class="text-sm text-gray-100 whitespace-pre-wrap font-mono leading-relaxed">{{ formatJson(selectedStep.output_data) }}</pre>
              </div>
            </div>
            <div v-else>
              <h3 class="text-sm font-medium text-gray-200 mb-3">Output Data</h3>
              <div class="bg-slate-800/80 border border-slate-600/70 rounded-md p-4 text-center">
                <span class="text-gray-400">No output data</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { XMarkIcon } from '@heroicons/vue/24/outline'
import { useExecutionStore } from '../stores/executions'
import type { ExecutionStatus, StepStatus, ExecutionStep } from '../types/execution'

const executionStore = useExecutionStore()

// Modal state
const showStepModal = ref(false)
const selectedStep = ref<ExecutionStep | null>(null)

// Handle Escape key to close panel or modal
function handleKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    if (showStepModal.value) {
      closeStepModal()
    } else if (executionStore.showSidePanel) {
      executionStore.closeSidePanel()
    }
  }
}

// Add/remove event listener when component mounts/unmounts
onMounted(() => {
  document.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  document.removeEventListener('keydown', handleKeydown)
})

function getStatusColorClass(status: ExecutionStatus): string {
  switch (status) {
    case 'pending':
      return 'bg-gray-900 text-gray-300'
    case 'running':
      return 'bg-blue-900 text-blue-300'
    case 'completed':
      return 'bg-green-900 text-green-300'
    case 'failed':
      return 'bg-red-900 text-red-300'
    case 'cancelled':
      return 'bg-yellow-900 text-yellow-300'
    default:
      return 'bg-gray-900 text-gray-300'
  }
}

function getStepStatusColorClass(status: StepStatus): string {
  switch (status) {
    case 'pending':
      return 'bg-gray-900 text-gray-300'
    case 'running':
      return 'bg-blue-900 text-blue-300'
    case 'completed':
      return 'bg-green-900 text-green-300'
    case 'failed':
      return 'bg-red-900 text-red-300'
    case 'skipped':
      return 'bg-yellow-900 text-yellow-300'
    default:
      return 'bg-gray-900 text-gray-300'
  }
}

function formatJson(data: any): string {
  if (typeof data === 'string') {
    try {
      return JSON.stringify(JSON.parse(data), null, 2)
    } catch {
      return data
    }
  }
  return JSON.stringify(data, null, 2)
}

async function cancelExecution() {
  if (!executionStore.selectedExecution) return
  
  try {
    await executionStore.cancelExecution(executionStore.selectedExecution.id)
  } catch (error) {
    console.error('Failed to cancel execution:', error)
  }
}

async function refreshExecution() {
  if (!executionStore.selectedExecution) return
  
  try {
    await executionStore.fetchExecution(executionStore.selectedExecution.id)
    await executionStore.fetchExecutionSteps(executionStore.selectedExecution.id)
    await executionStore.fetchExecutionLogs(executionStore.selectedExecution.id)
  } catch (error) {
    console.error('Failed to refresh execution:', error)
  }
}

function openStepModal(step: ExecutionStep) {
  selectedStep.value = step
  showStepModal.value = true
}

function closeStepModal() {
  showStepModal.value = false
  selectedStep.value = null
}
</script>