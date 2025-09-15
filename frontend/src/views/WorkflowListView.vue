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
        </div>
        <div class="flex items-center space-x-4">
          <div class="text-sm text-gray-400">
            {{ workflowStore.workflowCount }} workflows
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
      <div class="glass-medium rounded-lg shadow-2xl overflow-hidden">
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

        <div v-else-if="workflowStore.filteredWorkflows.length === 0" class="p-8 text-center">
          <p class="text-gray-400">No workflows found</p>
        </div>

        <div v-else class="overflow-x-auto">
          <table class="min-w-full divide-y divide-slate-600">
            <thead class="glass-dark">
              <tr>
                <th class="px-6 py-3 text-left text-xs font-medium text-gray-300 uppercase tracking-wider">
                  Name
                </th>
                <th class="px-6 py-3 text-left text-xs font-medium text-gray-300 uppercase tracking-wider">
                  Description
                </th>
                <th class="px-6 py-3 text-left text-xs font-medium text-gray-300 uppercase tracking-wider">
                  Status
                </th>
                <th class="px-6 py-3 text-left text-xs font-medium text-gray-300 uppercase tracking-wider">
                  Created
                </th>
                <th class="px-6 py-3 text-left text-xs font-medium text-gray-300 uppercase tracking-wider">
                  Last Modified
                </th>
                <th class="px-6 py-3 text-right text-xs font-medium text-gray-300 uppercase tracking-wider">
                  Actions
                </th>
              </tr>
            </thead>
            <tbody class="divide-y divide-slate-600/50">
              <tr
                v-for="workflow in workflowStore.filteredWorkflows"
                :key="workflow.id"
                class="hover:bg-white/5 transition-all duration-200 cursor-pointer backdrop-blur-sm"
                @click="navigateToDesigner(workflow.id)"
              >
                <td class="px-6 py-4 whitespace-nowrap">
                  <div class="text-sm font-medium text-white">{{ workflow.name }}</div>
                </td>
                <td class="px-6 py-4">
                  <div class="text-sm text-gray-300 max-w-xs truncate">
                    {{ workflow.description || 'No description' }}
                  </div>
                </td>
                <td class="px-6 py-4 whitespace-nowrap">
                  <span class="px-2 inline-flex text-xs leading-5 font-semibold rounded-full bg-green-900 text-green-200">
                    Active
                  </span>
                </td>
                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-300">
                  {{ formatDate(workflow.created_at) }}
                </td>
                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-300">
                  {{ formatDate(workflow.updated_at) }}
                </td>
                <td class="px-6 py-4 whitespace-nowrap text-right text-sm font-medium">
                  <div class="flex items-center justify-end space-x-2">
                    <button
                      @click.stop="navigateToDesigner(workflow.id)"
                      class="text-primary-400 hover:text-primary-300 transition-colors"
                      title="Edit"
                    >
                      <PencilIcon class="h-5 w-5" />
                    </button>
                    <button
                      @click.stop="showDeleteModalHandler(workflow)"
                      class="text-red-400 hover:text-red-300 transition-colors"
                      title="Delete"
                    >
                      <TrashIcon class="h-5 w-5" />
                    </button>
                  </div>
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

    <!-- Delete Confirmation Modal -->
    <div
      v-if="showDeleteModal"
      class="fixed inset-0 bg-black/30 backdrop-blur-sm flex items-center justify-center z-50"
      @click.self="cancelDelete"
    >
      <div class="glass-strong rounded-lg p-6 w-full max-w-md shadow-2xl">
        <h2 class="text-lg font-medium text-white mb-4">Delete Workflow</h2>
        <p class="text-gray-300 mb-6">
          Are you sure you want to delete "{{ workflowToDelete?.name }}"? This action cannot be undone.
        </p>
        <div class="flex justify-end space-x-3">
          <button
            type="button"
            @click="cancelDelete"
            :disabled="deleting"
            class="px-4 py-2 text-sm font-medium text-gray-300 hover:text-white transition-colors"
          >
            Cancel
          </button>
          <button
            @click="confirmDelete"
            :disabled="deleting"
            class="bg-red-600 hover:bg-red-700 disabled:bg-gray-600 text-white px-4 py-2 rounded-md font-medium transition-colors"
          >
            {{ deleting ? 'Deleting...' : 'Delete' }}
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
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import {
  MagnifyingGlassIcon,
  PencilIcon,
  TrashIcon
} from '@heroicons/vue/24/outline'
import { useWorkflowStore } from '../stores/workflows'
import HeaderComponent from '../components/HeaderComponent.vue'
import AIWorkflowChat from '../components/AIWorkflowChat.vue'
import type { Workflow } from '../types/workflow'
import { formatDate } from '../utils/formatting'

const router = useRouter()
const workflowStore = useWorkflowStore()

const showCreateModal = ref(false)
const showDeleteModal = ref(false)
const showAIChat = ref(false)
const creating = ref(false)
const deleting = ref(false)
const workflowToDelete = ref<Workflow | null>(null)
const newWorkflow = ref({
  name: '',
  description: ''
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


function showDeleteModalHandler(workflow: Workflow) {
  workflowToDelete.value = workflow
  showDeleteModal.value = true
}

async function confirmDelete() {
  if (!workflowToDelete.value || deleting.value) return
  
  deleting.value = true
  try {
    await workflowStore.deleteWorkflow(workflowToDelete.value.id)
    showDeleteModal.value = false
    workflowToDelete.value = null
  } catch (error) {
    console.error('Failed to delete workflow:', error)
  } finally {
    deleting.value = false
  }
}

function cancelDelete() {
  showDeleteModal.value = false
  workflowToDelete.value = null
}
</script>