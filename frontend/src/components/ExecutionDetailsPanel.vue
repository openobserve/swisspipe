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
              <div v-if="executionStore.selectedExecution.current_node_id">
                <span class="text-xs text-gray-400">Current Node</span>
                <div class="text-sm text-white">{{ executionStore.selectedExecution.current_node_id }}</div>
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
            <h3 class="text-sm font-medium text-gray-200 mb-3">Input Data (event)</h3>
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
                  <tr v-for="headerGroup in table.getHeaderGroups()" :key="headerGroup.id">
                    <th
                      v-for="header in headerGroup.headers"
                      :key="header.id"
                      class="px-4 py-2 text-left text-xs font-medium text-gray-300 uppercase tracking-wider cursor-pointer hover:bg-slate-600/50 transition-colors"
                      @click="header.column.getToggleSortingHandler()?.($event)"
                    >
                      <div class="flex items-center space-x-1">
                        <FlexRender
                          :render="header.column.columnDef.header"
                          :props="header.getContext()"
                        />
                        <span v-if="header.column.getIsSorted()" class="ml-1">
                          {{ header.column.getIsSorted() === 'desc' ? '↓' : '↑' }}
                        </span>
                      </div>
                    </th>
                  </tr>
                </thead>
                <tbody class="divide-y divide-slate-600/50">
                  <tr
                    v-for="row in table.getRowModel().rows"
                    :key="row.id"
                    class="hover:bg-slate-700/30 transition-colors cursor-pointer"
                    @click="openStepModal(row.original)"
                  >
                    <td
                      v-for="cell in row.getVisibleCells()"
                      :key="cell.id"
                      class="px-4 py-2 text-sm"
                    >
                      <FlexRender
                        :render="cell.column.columnDef.cell"
                        :props="cell.getContext()"
                      />
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
              <h3 class="text-sm font-medium text-gray-200 mb-3">Input Data (event)</h3>
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
import { ref, onMounted, onUnmounted, computed, h } from 'vue'
import { XMarkIcon } from '@heroicons/vue/24/outline'
import {
  useVueTable,
  getCoreRowModel,
  getSortedRowModel,
  type ColumnDef,
  type SortingState,
  FlexRender
} from '@tanstack/vue-table'
import { useExecutionStore } from '../stores/executions'
import type { ExecutionStatus, StepStatus, ExecutionStep } from '../types/execution'

const executionStore = useExecutionStore()

// Modal state
const showStepModal = ref(false)
const selectedStep = ref<ExecutionStep | null>(null)

// Table state
const sorting = ref<SortingState>([
  {
    id: 'created_at',
    desc: false // ascending by default
  }
])

// Table columns definition
const columns = computed<ColumnDef<ExecutionStep>[]>(() => [
  {
    accessorKey: 'node_name',
    header: 'Node Name',
    cell: (info) => h('span', { class: 'font-medium text-white' }, info.getValue() as string)
  },
  {
    accessorKey: 'status',
    header: 'Status',
    cell: (info) => {
      const status = info.getValue() as StepStatus
      return h('span', {
        class: `px-2 py-1 text-xs leading-4 font-semibold rounded-full ${getStepStatusColorClass(status)}`
      }, status)
    }
  },
  {
    accessorKey: 'created_at',
    header: 'Created',
    sortingFn: 'basic',
    cell: (info) => h('span', {
      class: 'text-xs text-gray-200 font-mono'
    }, executionStore.formatTimestamp(info.getValue() as number))
  },
  {
    accessorKey: 'started_at',
    header: 'Started',
    cell: (info) => {
      const startedAt = info.getValue() as number | undefined
      return h('span', {
        class: 'text-xs text-gray-200 font-mono'
      }, startedAt ? executionStore.formatTimestamp(startedAt) : '-')
    }
  },
  {
    accessorKey: 'completed_at',
    header: 'Completed',
    cell: (info) => {
      const completedAt = info.getValue() as number | undefined
      return h('span', {
        class: 'text-xs text-gray-200 font-mono'
      }, completedAt ? executionStore.formatTimestamp(completedAt) : '-')
    }
  },
  {
    id: 'duration',
    header: 'Duration',
    accessorFn: (row) => ({ started_at: row.started_at, completed_at: row.completed_at }),
    cell: (info) => {
      const { started_at, completed_at } = info.getValue() as { started_at?: number, completed_at?: number }
      return h('span', {
        class: 'text-xs text-gray-200 font-mono'
      }, started_at && completed_at ? executionStore.formatDuration(started_at, completed_at) : '-')
    }
  },
  {
    accessorKey: 'error_message',
    header: 'Error',
    cell: (info) => {
      const errorMessage = info.getValue() as string | undefined
      return h('div', {
        class: 'text-xs text-red-300 max-w-md whitespace-pre-wrap break-words'
      }, errorMessage || '-')
    }
  }
])

// Create table instance
const table = useVueTable({
  get data() {
    return executionStore.executionSteps
  },
  get columns() {
    return columns.value
  },
  state: {
    get sorting() {
      return sorting.value
    }
  },
  onSortingChange: (updaterOrValue) => {
    sorting.value = typeof updaterOrValue === 'function'
      ? updaterOrValue(sorting.value)
      : updaterOrValue
  },
  getCoreRowModel: getCoreRowModel(),
  getSortedRowModel: getSortedRowModel(),
})

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

function formatJson(data: unknown): string {
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
  } catch (error) {
    console.error('Failed to refresh execution:', error)
  }
}

function openStepModal(step: ExecutionStep) {
  try {
    console.log('Opening step modal for:', step?.id)
    if (!step || !step.id) {
      console.error('Invalid step object:', step)
      return
    }
    selectedStep.value = step
    showStepModal.value = true
  } catch (error) {
    console.error('Error in openStepModal:', error)
  }
}

function closeStepModal() {
  showStepModal.value = false
  selectedStep.value = null
}
</script>