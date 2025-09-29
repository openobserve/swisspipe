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

        <div v-else class="flex-1 flex flex-col min-h-0">
          <!-- Fixed Header -->
          <div class="overflow-x-auto bg-slate-800/80 border-b border-slate-600/50">
            <table class="min-w-full">
              <thead class="glass-dark">
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
            </table>
          </div>

          <!-- Scrollable Body -->
          <div class="flex-1 overflow-auto">
            <div class="overflow-x-auto">
              <table class="min-w-full">
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
  PencilIcon
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
import HeaderComponent from '../components/HeaderComponent.vue'
import AIWorkflowChat from '../components/AIWorkflowChat.vue'
import type { Workflow } from '../types/workflow'
import { formatDate } from '../utils/formatting'

const router = useRouter()
const workflowStore = useWorkflowStore()

const showCreateModal = ref(false)
const showDisableModal = ref(false)
const showAIChat = ref(false)
const creating = ref(false)
const togglingStatus = ref<string | null>(null)
const workflowToDisable = ref<Workflow | null>(null)
const statusFilter = ref<'all' | 'enabled' | 'disabled'>('enabled')
const newWorkflow = ref({
  name: '',
  description: ''
})

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
</script>