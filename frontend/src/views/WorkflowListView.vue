<template>
  <div class="min-h-screen text-gray-100">
    <!-- Header -->
    <HeaderComponent />

    <!-- Main Content -->
    <main class="p-6">
      <!-- Search and Filters -->
      <div class="mb-6 flex items-center justify-between">
        <div class="flex items-center space-x-4">
          <div class="relative">
            <input
              v-model="workflowStore.searchTerm"
              type="text"
              placeholder="Search workflows..."
              class="glass border border-slate-600/50 text-gray-100 px-4 py-2 pl-10 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-primary-500/50 w-64"
            />
            <MagnifyingGlassIcon class="h-5 w-5 text-gray-400 absolute left-3 top-2.5" />
          </div>
          <!-- Status Filter Toggle -->
          <div class="flex items-center bg-slate-800/50 rounded-lg p-1 border border-slate-600/50">
            <button
              @click="statusFilter = 'all'"
              :class="statusFilter === 'all'
                ? 'bg-primary-600 text-white'
                : 'text-gray-300 hover:text-white'"
              class="px-3 py-1 rounded-md text-sm font-medium transition-colors"
            >
              All
            </button>
            <button
              @click="statusFilter = 'enabled'"
              :class="statusFilter === 'enabled'
                ? 'bg-green-600 text-white'
                : 'text-gray-300 hover:text-white'"
              class="px-3 py-1 rounded-md text-sm font-medium transition-colors"
            >
              Enabled
            </button>
            <button
              @click="statusFilter = 'disabled'"
              :class="statusFilter === 'disabled'
                ? 'bg-red-600 text-white'
                : 'text-gray-300 hover:text-white'"
              class="px-3 py-1 rounded-md text-sm font-medium transition-colors"
            >
              Disabled
            </button>
          </div>
        </div>
        <div class="flex items-center space-x-4">
          <div class="text-sm text-gray-400">
            {{ filteredWorkflows.length }} of {{ workflowStore.workflowCount }} workflows
          </div>
          <!-- AI Assistant Button -->
          <button
            @click="showAIChat = true"
            class="bg-gradient-to-r from-purple-600 to-pink-600 hover:from-purple-700 hover:to-pink-700 text-white px-4 py-2 rounded-md font-medium transition-all duration-200 flex items-center space-x-2 shadow-lg"
            title="AI Workflow Assistant"
          >
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"></path>
            </svg>
            <span>AI Assistant</span>
          </button>
          <button
            @click="showImportModal = true"
            class="bg-green-600 hover:bg-green-700 text-white px-4 py-2 rounded-md font-medium transition-colors flex items-center space-x-2"
            title="Import Workflow from JSON"
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M9 19l3 3m0 0l3-3m-3 3V10"></path>
            </svg>
            <span>Import Workflow</span>
          </button>
          <button
              @click="showCreateModal = true"
              class="bg-primary-600 hover:bg-primary-700 text-white px-4 py-2 rounded-md font-medium transition-colors"
            >
              Create Workflow
            </button>
        </div>
      </div>

      <!-- Workflows Table -->
      <div class="glass-medium rounded-lg shadow-2xl overflow-hidden flex flex-col max-h-[calc(100vh-200px)]">
        <div v-if="workflowStore.loading" class="p-8 text-center">
          <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-500 mx-auto"></div>
          <p class="mt-2 text-gray-400">Loading workflows...</p>
        </div>

        <div v-else-if="workflowStore.error" class="p-8 text-center">
          <p class="text-red-400">{{ workflowStore.error }}</p>
          <button
            @click="workflowStore.fetchWorkflows()"
            class="mt-4 bg-primary-600 hover:bg-primary-700 text-white px-4 py-2 rounded-md"
          >
            Retry
          </button>
        </div>

        <div v-else-if="filteredWorkflows.length === 0" class="p-8 text-center">
          <p class="text-gray-400">
            {{ statusFilter === 'all' ? 'No workflows found' : `No ${statusFilter} workflows found` }}
          </p>
          <p v-if="statusFilter !== 'all' && workflowStore.workflowCount > 0" class="text-sm text-gray-500 mt-2">
            Try switching to "All" to see all workflows
          </p>
        </div>

        <div v-else class="flex-1 overflow-auto">
          <table class="min-w-full">
            <thead class="sticky top-0 z-10 bg-slate-800/90 backdrop-blur-sm border-b border-slate-600/50">
              <tr v-for="headerGroup in table.getHeaderGroups()" :key="headerGroup.id">
                <th
                  v-for="header in headerGroup.headers"
                  :key="header.id"
                  :class="[
                    'px-6 py-3 text-left text-xs font-medium text-gray-300 uppercase tracking-wider',
                    header.id === 'actions' ? 'text-right' : '',
                    header.column.getCanSort() ? 'cursor-pointer hover:bg-slate-600/30 transition-colors select-none' : ''
                  ]"
                  @click="header.column.getToggleSortingHandler()?.($event)"
                >
                  <div class="flex items-center" :class="header.id === 'actions' ? 'justify-end' : 'justify-start'">
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
                class="hover:bg-white/5 transition-all duration-200 cursor-pointer backdrop-blur-sm"
                @click="navigateToDesigner(row.original.id)"
              >
                <td
                  v-for="cell in row.getVisibleCells()"
                  :key="cell.id"
                  :class="[
                    'px-6 py-4',
                    cell.column.id === 'name' ? 'whitespace-nowrap' : '',
                    cell.column.id === 'enabled' ? 'whitespace-nowrap' : '',
                    cell.column.id === 'created_at' ? 'whitespace-nowrap' : '',
                    cell.column.id === 'updated_at' ? 'whitespace-nowrap' : '',
                    cell.column.id === 'actions' ? 'whitespace-nowrap text-right text-sm font-medium' : ''
                  ]"
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
    </main>

    <!-- Create Workflow Modal -->
    <div
      v-if="showCreateModal"
      class="fixed inset-0 bg-black/30 backdrop-blur-sm flex items-center justify-center z-50"
      @click.self="showCreateModal = false"
    >
      <div class="glass-strong rounded-lg p-6 w-full max-w-md shadow-2xl">
        <h2 class="text-lg font-medium text-white mb-4">Create New Workflow</h2>
        <form @submit.prevent="createWorkflow">
          <div class="mb-4">
            <label class="block text-sm font-medium text-gray-300 mb-2">Name</label>
            <input
              v-model="newWorkflow.name"
              type="text"
              required
              class="w-full glass border border-slate-600/50 text-gray-100 px-3 py-2 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
            />
          </div>
          <div class="mb-4">
            <label class="block text-sm font-medium text-gray-300 mb-2">Description</label>
            <textarea
              v-model="newWorkflow.description"
              rows="3"
              class="w-full glass border border-slate-600/50 text-gray-100 px-3 py-2 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
            ></textarea>
          </div>
          <div class="flex justify-end space-x-3">
            <button
              type="button"
              @click="showCreateModal = false"
              class="px-4 py-2 text-sm font-medium text-gray-300 hover:text-white transition-colors"
            >
              Cancel
            </button>
            <button
              type="submit"
              :disabled="!newWorkflow.name || creating"
              class="bg-primary-600 hover:bg-primary-700 disabled:bg-gray-600 text-white px-4 py-2 rounded-md font-medium transition-colors"
            >
              {{ creating ? 'Creating...' : 'Create' }}
            </button>
          </div>
        </form>
      </div>
    </div>

    <!-- Import Workflow Modal -->
    <div
      v-if="showImportModal"
      class="fixed inset-0 bg-black/30 backdrop-blur-sm flex items-center justify-center z-50"
      @click.self="showImportModal = false"
    >
      <div class="glass-strong rounded-lg p-6 w-full max-w-lg shadow-2xl">
        <h2 class="text-lg font-medium text-white mb-4">Import Workflow from JSON</h2>

        <!-- File Upload Area -->
        <div class="mb-6">
          <div
            @click="triggerFileInput"
            @dragover.prevent
            @drop.prevent="handleFileDrop"
            class="border-2 border-dashed border-slate-600/50 rounded-lg p-6 text-center cursor-pointer hover:border-primary-500/50 transition-colors"
          >
            <div v-if="!importFile" class="text-gray-400">
              <svg class="mx-auto h-12 w-12 text-gray-500 mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M9 19l3 3m0 0l3-3m-3 3V10"></path>
              </svg>
              <p class="text-sm">
                <span class="font-medium text-primary-400">Click to upload</span> or drag and drop
              </p>
              <p class="text-xs text-gray-500">JSON files only</p>
            </div>
            <div v-else class="text-green-400">
              <svg class="mx-auto h-8 w-8 mb-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"></path>
              </svg>
              <p class="text-sm font-medium">{{ importFile.name }}</p>
              <p class="text-xs text-gray-400">{{ (importFile.size / 1024).toFixed(1) }} KB</p>
            </div>
          </div>
          <input
            ref="fileInput"
            type="file"
            accept=".json"
            @change="handleFileSelect"
            class="hidden"
          />
        </div>

        <!-- Workflow Name Override -->
        <div v-if="importFile" class="mb-4">
          <label class="block text-sm font-medium text-gray-300 mb-2">
            Workflow Name
            <span class="text-xs text-gray-500">(optional - will use name from JSON if not specified)</span>
          </label>
          <input
            v-model="importWorkflowName"
            type="text"
            placeholder="Leave empty to use name from JSON file"
            class="w-full glass border border-slate-600/50 text-gray-100 px-3 py-2 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
          />
        </div>

        <!-- Error Display -->
        <div v-if="importError" class="mb-4 p-3 bg-red-900/30 border border-red-600/50 rounded-md">
          <p class="text-red-400 text-sm">{{ importError }}</p>
        </div>

        <!-- Preview Info -->
        <div v-if="workflowPreview" class="mb-4 p-3 bg-slate-800/50 border border-slate-600/50 rounded-md">
          <h4 class="text-sm font-medium text-white mb-2">Workflow Preview:</h4>
          <p class="text-sm text-gray-300"><span class="font-medium">Name:</span> {{ workflowPreview.name }}</p>
          <p class="text-sm text-gray-300"><span class="font-medium">Description:</span> {{ workflowPreview.description || 'No description' }}</p>
          <p class="text-sm text-gray-300"><span class="font-medium">Nodes:</span> {{ workflowPreview.nodes?.length || 0 }}</p>
          <p class="text-sm text-gray-300"><span class="font-medium">Edges:</span> {{ workflowPreview.edges?.length || 0 }}</p>
        </div>

        <div class="flex justify-end space-x-3">
          <button
            type="button"
            @click="cancelImport"
            class="px-4 py-2 text-sm font-medium text-gray-300 hover:text-white transition-colors"
          >
            Cancel
          </button>
          <button
            @click="importWorkflow"
            :disabled="!importFile || importing"
            class="bg-green-600 hover:bg-green-700 disabled:bg-gray-600 text-white px-4 py-2 rounded-md font-medium transition-colors"
          >
            {{ importing ? 'Importing...' : 'Import Workflow' }}
          </button>
        </div>
      </div>
    </div>

    <!-- Disable Confirmation Modal -->
    <div
      v-if="showDisableModal"
      class="fixed inset-0 bg-black/30 backdrop-blur-sm flex items-center justify-center z-50"
      @click.self="cancelDisable"
    >
      <div class="glass-strong rounded-lg p-6 w-full max-w-md shadow-2xl">
        <h2 class="text-lg font-medium text-white mb-4">Disable Workflow</h2>
        <p class="text-gray-300 mb-6">
          Are you sure you want to disable "{{ workflowToDisable?.name }}"? This will reject all ingestion requests with HTTP 403. You can re-enable it later.
        </p>
        <div class="flex justify-end space-x-3">
          <button
            type="button"
            @click="cancelDisable"
            :disabled="togglingStatus === workflowToDisable?.id"
            class="px-4 py-2 text-sm font-medium text-gray-300 hover:text-white transition-colors"
          >
            Cancel
          </button>
          <button
            @click="confirmDisable"
            :disabled="togglingStatus === workflowToDisable?.id"
            class="bg-red-600 hover:bg-red-700 disabled:bg-gray-600 text-white px-4 py-2 rounded-md font-medium transition-colors"
          >
            {{ togglingStatus === workflowToDisable?.id ? 'Disabling...' : 'Disable' }}
          </button>
        </div>
      </div>
    </div>

    <!-- AI Workflow Chat -->
    <AIWorkflowChat
      :show-chat="showAIChat"
      @close="showAIChat = false"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed, h } from 'vue'
import { useRouter } from 'vue-router'
import {
  MagnifyingGlassIcon,
  PencilIcon,
  ChevronDoubleRightIcon
} from '@heroicons/vue/24/outline'
import {
  useVueTable,
  getCoreRowModel,
  getSortedRowModel,
  type ColumnDef,
  type SortingState,
  FlexRender
} from '@tanstack/vue-table'
import { useWorkflowStore } from '../stores/workflows'
import { useExecutionStore } from '../stores/executions'
import HeaderComponent from '../components/HeaderComponent.vue'
import AIWorkflowChat from '../components/AIWorkflowChat.vue'
import type { Workflow, CreateWorkflowRequest, NodeType, RetryConfig } from '../types/workflow'
import { formatDate } from '../utils/formatting'

const router = useRouter()
const workflowStore = useWorkflowStore()
const executionStore = useExecutionStore()

const showCreateModal = ref(false)
const showImportModal = ref(false)
const showDisableModal = ref(false)
const showAIChat = ref(false)
const creating = ref(false)
const importing = ref(false)
const togglingStatus = ref<string | null>(null)
const workflowToDisable = ref<Workflow | null>(null)
const statusFilter = ref<'all' | 'enabled' | 'disabled'>('enabled')
const newWorkflow = ref({
  name: '',
  description: ''
})

// Import workflow state
const importFile = ref<File | null>(null)
const importWorkflowName = ref('')
const importError = ref('')
const workflowPreview = ref<{ name: string; description?: string; nodes: unknown[]; edges: unknown[] } | null>(null)
const fileInput = ref<HTMLInputElement | null>(null)

// Local filtered workflows computed property (must be defined before table)
const filteredWorkflows = computed(() => {
  let result = workflowStore.filteredWorkflows

  // Apply status filter
  if (statusFilter.value === 'enabled') {
    result = result.filter(workflow => workflow.enabled)
  } else if (statusFilter.value === 'disabled') {
    result = result.filter(workflow => !workflow.enabled)
  }

  return result
})

// Table state for TanStack Table
const sorting = ref<SortingState>([
  {
    id: 'updated_at',
    desc: true // Default sort by last modified descending
  }
])

// Table columns definition
const columns = computed<ColumnDef<Workflow>[]>(() => [
  {
    accessorKey: 'name',
    header: 'Name',
    cell: (info) => h('div', {
      class: 'text-sm font-medium text-white'
    }, info.getValue() as string)
  },
  {
    accessorKey: 'description',
    header: 'Description',
    cell: (info) => {
      const description = info.getValue() as string | undefined
      return h('div', {
        class: 'text-sm text-gray-300 max-w-xs truncate'
      }, description || 'No description')
    }
  },
  {
    accessorKey: 'enabled',
    header: 'Status',
    cell: (info) => {
      const enabled = info.getValue() as boolean
      return h('span', {
        class: `px-2 inline-flex text-xs leading-5 font-semibold rounded-full ${
          enabled
            ? 'bg-green-900 text-green-200'
            : 'bg-red-900 text-red-200'
        }`
      }, enabled ? 'Enabled' : 'Disabled')
    }
  },
  {
    accessorKey: 'created_at',
    header: 'Created',
    cell: (info) => h('span', {
      class: 'text-sm text-gray-300'
    }, formatDate(info.getValue() as string))
  },
  {
    accessorKey: 'updated_at',
    header: 'Last Modified',
    cell: (info) => h('span', {
      class: 'text-sm text-gray-300'
    }, formatDate(info.getValue() as string))
  },
  {
    id: 'actions',
    header: 'Actions',
    cell: (info) => {
      const workflow = info.row.original
      return h('div', {
        class: 'flex items-center justify-end space-x-2'
      }, [
        h('button', {
          class: 'text-blue-400 hover:text-blue-300 transition-colors',
          title: 'View Executions',
          onClick: (e: Event) => {
            e.stopPropagation()
            navigateToExecutions(workflow)
          }
        }, [
          h(ChevronDoubleRightIcon, { class: 'h-5 w-5' })
        ]),
        h('button', {
          class: 'text-primary-400 hover:text-primary-300 transition-colors',
          title: 'Edit',
          onClick: (e: Event) => {
            e.stopPropagation()
            navigateToDesigner(workflow.id)
          }
        }, [
          h(PencilIcon, { class: 'h-5 w-5' })
        ]),
        h('button', {
          class: `transition-colors ${
            workflow.enabled
              ? 'text-yellow-400 hover:text-yellow-300'
              : 'text-green-400 hover:text-green-300'
          }`,
          title: workflow.enabled ? 'Disable' : 'Enable',
          disabled: togglingStatus.value === workflow.id,
          onClick: (e: Event) => {
            e.stopPropagation()
            toggleWorkflowStatus(workflow)
          }
        }, [
          workflow.enabled
            ? h('svg', {
                class: 'h-5 w-5',
                fill: 'none',
                stroke: 'currentColor',
                viewBox: '0 0 24 24'
              }, [
                h('path', {
                  'stroke-linecap': 'round',
                  'stroke-linejoin': 'round',
                  'stroke-width': '2',
                  d: 'M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728L5.636 5.636m12.728 12.728L5.636 5.636'
                })
              ])
            : h('svg', {
                class: 'h-5 w-5',
                fill: 'none',
                stroke: 'currentColor',
                viewBox: '0 0 24 24'
              }, [
                h('path', {
                  'stroke-linecap': 'round',
                  'stroke-linejoin': 'round',
                  'stroke-width': '2',
                  d: 'M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z'
                })
              ])
        ])
      ])
    },
    enableSorting: false,
  }
])

// Create table instance
const table = useVueTable({
  get data() {
    return filteredWorkflows.value
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

onMounted(() => {
  workflowStore.fetchWorkflows()
})


function navigateToDesigner(workflowId: string) {
  router.push(`/workflows/${workflowId}`)
}

function navigateToExecutions(workflow: Workflow) {
  // Set the workflow name filter in the execution store
  executionStore.workflowNameFilter = workflow.name
  // Navigate to executions view
  router.push('/executions')
}

async function createWorkflow() {
  if (!newWorkflow.value.name) return
  
  creating.value = true
  try {
    const workflow = await workflowStore.createWorkflow({
      name: newWorkflow.value.name,
      description: newWorkflow.value.description || undefined,
      nodes: [],
      edges: []
    })
    
    showCreateModal.value = false
    newWorkflow.value = { name: '', description: '' }
    navigateToDesigner(workflow.id)
  } catch (error) {
    console.error('Failed to create workflow:', error)
  } finally {
    creating.value = false
  }
}


async function toggleWorkflowStatus(workflow: Workflow) {
  if (togglingStatus.value === workflow.id) return

  if (workflow.enabled) {
    // Show confirmation modal for disable action
    workflowToDisable.value = workflow
    showDisableModal.value = true
  } else {
    // Enable immediately without confirmation
    togglingStatus.value = workflow.id
    try {
      await workflowStore.enableWorkflow(workflow.id)
    } catch (error) {
      console.error('Failed to enable workflow:', error)
    } finally {
      togglingStatus.value = null
    }
  }
}

async function confirmDisable() {
  if (!workflowToDisable.value || togglingStatus.value === workflowToDisable.value.id) return

  togglingStatus.value = workflowToDisable.value.id
  try {
    await workflowStore.disableWorkflow(workflowToDisable.value.id)
    showDisableModal.value = false
    workflowToDisable.value = null
  } catch (error) {
    console.error('Failed to disable workflow:', error)
  } finally {
    togglingStatus.value = null
  }
}

function cancelDisable() {
  showDisableModal.value = false
  workflowToDisable.value = null
}

// Import workflow functions
function triggerFileInput() {
  fileInput.value?.click()
}

function handleFileSelect(event: Event) {
  const target = event.target as HTMLInputElement
  if (target.files && target.files.length > 0) {
    processFile(target.files[0])
  }
}

function handleFileDrop(event: DragEvent) {
  event.preventDefault()
  if (event.dataTransfer?.files && event.dataTransfer.files.length > 0) {
    processFile(event.dataTransfer.files[0])
  }
}

async function processFile(file: File) {
  // Reset state
  importError.value = ''
  workflowPreview.value = null

  // Validate file type
  if (!file.name.toLowerCase().endsWith('.json')) {
    importError.value = 'Please select a JSON file'
    return
  }

  // Validate file size (limit to 10MB)
  if (file.size > 10 * 1024 * 1024) {
    importError.value = 'File size must be less than 10MB'
    return
  }

  try {
    const text = await file.text()
    const workflowData = JSON.parse(text)

    // Basic validation of workflow structure
    if (!workflowData || typeof workflowData !== 'object') {
      throw new Error('Invalid JSON structure')
    }

    // Check for required fields
    if (!workflowData.name || typeof workflowData.name !== 'string') {
      throw new Error('Workflow must have a name field')
    }

    if (!Array.isArray(workflowData.nodes)) {
      throw new Error('Workflow must have a nodes array')
    }

    if (!Array.isArray(workflowData.edges)) {
      throw new Error('Workflow must have an edges array')
    }

    // Set the file and preview
    importFile.value = file
    workflowPreview.value = workflowData
    importWorkflowName.value = '' // Reset name override

  } catch (error) {
    importError.value = `Failed to parse JSON: ${(error as Error).message}`
  }
}

async function importWorkflow() {
  if (!importFile.value || !workflowPreview.value) return

  importing.value = true
  try {
    const workflowData = { ...workflowPreview.value }

    // Use name override if provided
    if (importWorkflowName.value.trim()) {
      workflowData.name = importWorkflowName.value.trim()
    }

    // Function to transform frontend node configuration to backend NodeType enum
    const transformNodeType = (node: { type: string; data?: { config?: unknown } }): NodeType => {
      const config = (node.data?.config || {}) as Record<string, unknown>

      switch (node.type) {
        case 'trigger':
          return {
            Trigger: {
              methods: (config.methods as string[]) || ['Get', 'Post', 'Put']
            }
          } as NodeType

        case 'condition':
          return {
            Condition: {
              script: (config.script as string) || 'function condition(event) { return true; }'
            }
          } as NodeType

        case 'transformer':
          return {
            Transformer: {
              script: (config.script as string) || 'function transformer(event) { return event; }'
            }
          } as NodeType

        case 'http-request':
          return {
            HttpRequest: {
              url: (config.url as string) || 'https://example.com',
              method: (config.method as string) || 'Post',
              timeout_seconds: (config.timeout_seconds as number) || 30,
              failure_action: (config.failure_action as string) || 'Stop',
              headers: (config.headers as Record<string, string>) || {},
              retry_config: (config.retry_config as RetryConfig) || {
                max_attempts: 3,
                initial_delay_ms: 100,
                max_delay_ms: 5000,
                backoff_multiplier: 2
              },
              loop_config: (config.loop_config as Record<string, unknown>) || undefined
            }
          } as NodeType

        case 'email':
          return {
            Email: {
              config: {
                to: (config.to as Array<{ email: string; name?: string }>) || [],
                cc: (config.cc as Array<{ email: string; name?: string }>) || undefined,
                bcc: (config.bcc as Array<{ email: string; name?: string }>) || undefined,
                subject: (config.subject as string) || 'SwissPipe Workflow Notification',
                template_type: (config.template_type as 'html' | 'text') || 'html',
                body_template: (config.body_template as string) || '<p>Workflow completed successfully.</p>',
                text_body_template: (config.text_body_template as string) || undefined,
                attachments: (config.attachments as Array<{ filename: string; content_type: string; data: string }>) || undefined
              }
            }
          } as NodeType

        case 'delay':
          return {
            Delay: {
              duration: (config.duration as number) || 1,
              unit: (config.unit as string) || 'Seconds'
            }
          } as NodeType

        case 'openobserve':
          return {
            OpenObserve: {
              url: (config.url as string) || (config.endpoint as string) || 'https://api.openobserve.ai',
              authorization_header: (config.authorization_header as string) || '',
              timeout_seconds: (config.timeout_seconds as number) || 30,
              failure_action: (config.failure_action as string) || 'Stop',
              retry_config: (config.retry_config as RetryConfig) || {
                max_attempts: 3,
                initial_delay_ms: 100,
                max_delay_ms: 5000,
                backoff_multiplier: 2
              }
            }
          } as NodeType

        case 'human-in-loop':
          return {
            HumanInLoop: {
              title: (config.title as string) || 'Human Action Required',
              description: (config.description as string) || 'Please review and take action',
              timeout_seconds: (config.timeout_seconds as number) || ((config.timeout_minutes as number) ? ((config.timeout_minutes as number) * 60) : undefined),
              timeout_action: (config.timeout_action as string) || undefined,
              required_fields: (config.required_fields as string[]) || undefined,
              metadata: (config.metadata as Record<string, unknown>) || undefined
            }
          } as NodeType

        case 'anthropic':
          return {
            Anthropic: {
              model: (config.model as string) || 'claude-3-sonnet-20240229',
              max_tokens: (config.max_tokens as number) || 1000,
              temperature: (config.temperature as number) || 0.7,
              system_prompt: (config.system_prompt as string) || undefined,
              user_prompt: (config.user_prompt as string) || 'Process this data',
              timeout_seconds: (config.timeout_seconds as number) || 60,
              failure_action: (config.failure_action as string) || 'Stop',
              retry_config: (config.retry_config as RetryConfig) || {
                max_attempts: 3,
                initial_delay_ms: 100,
                max_delay_ms: 5000,
                backoff_multiplier: 2
              }
            }
          } as NodeType

        default:
          // Fallback - return a simple trigger
          return {
            Trigger: {
              methods: ['Get', 'Post', 'Put']
            }
          } as NodeType
      }
    }

    // Define type for imported workflow data
    type ImportNode = { id: string; data?: { label?: string; config?: unknown }; position?: { x?: number; y?: number }; type: string }
    type ImportEdge = { source: string; target: string; sourceHandle?: string }

    // Transform nodes to match backend NodeRequest structure
    const transformedNodes = ((workflowData.nodes || []) as ImportNode[]).map((node) => ({
      id: node.id,
      name: node.data?.label || `Node ${node.id}`,
      node_type: transformNodeType(node),
      position_x: node.position?.x || 0,
      position_y: node.position?.y || 0
    }))

    // Transform edges to match backend EdgeRequest structure
    const transformedEdges = ((workflowData.edges || []) as ImportEdge[]).map((edge) => ({
      from_node_id: edge.source,
      to_node_id: edge.target,
      condition_result: edge.sourceHandle === 'true' ? true : edge.sourceHandle === 'false' ? false : undefined,
      source_handle_id: edge.sourceHandle || undefined
    }))

    // Ensure the workflow has required fields
    const workflowPayload: CreateWorkflowRequest = {
      name: workflowData.name,
      description: workflowData.description || undefined,
      nodes: transformedNodes,
      edges: transformedEdges
    }

    const workflow = await workflowStore.createWorkflow(workflowPayload)

    // Close modal and navigate to designer
    showImportModal.value = false
    resetImportState()
    navigateToDesigner(workflow.id)

  } catch (error) {
    importError.value = `Failed to import workflow: ${(error as Error).message}`
  } finally {
    importing.value = false
  }
}

function cancelImport() {
  showImportModal.value = false
  resetImportState()
}

function resetImportState() {
  importFile.value = null
  importWorkflowName.value = ''
  importError.value = ''
  workflowPreview.value = null
  if (fileInput.value) {
    fileInput.value.value = ''
  }
}
</script>