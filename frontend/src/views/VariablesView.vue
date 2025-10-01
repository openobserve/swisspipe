<template>
  <div class="flex flex-col h-full">
    <HeaderComponent title="Environment Variables" />

    <!-- Error Message -->
    <div
      v-if="variableStore.error"
      class="mx-6 mt-4 p-4 bg-red-900/20 border border-red-500 rounded-md text-red-300"
    >
      {{ variableStore.error }}
      <button
        @click="variableStore.clearError"
        class="ml-4 text-red-400 hover:text-red-300"
      >
        Dismiss
      </button>
    </div>

    <!-- Search and Actions Bar -->
    <div class="flex items-center justify-between px-6 py-4">
      <div class="flex items-center space-x-4">
        <input
          v-model="variableStore.searchQuery"
          type="text"
          placeholder="Search variables..."
          class="bg-slate-700 border border-slate-600 text-gray-100 px-4 py-2 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500 w-64"
        />
        <div class="text-sm text-gray-400">
          {{ variableStore.variableCount }} variables
          ({{ variableStore.secretCount }} secrets, {{ variableStore.textCount }} text)
        </div>
      </div>
      <button
        @click="openCreateModal"
        class="bg-primary-600 hover:bg-primary-700 text-white px-4 py-2 rounded-md transition-colors flex items-center space-x-2"
      >
        <span>+</span>
        <span>Add Variable</span>
      </button>
    </div>

    <!-- Variables Table -->
    <div class="flex-1 overflow-auto px-6 pb-6">
      <div
        v-if="variableStore.loading && variableStore.variables.length === 0"
        class="flex items-center justify-center h-64"
      >
        <div class="text-gray-400">Loading variables...</div>
      </div>

      <div
        v-else-if="variableStore.filteredVariables.length === 0"
        class="flex items-center justify-center h-64"
      >
        <div class="text-center text-gray-400">
          <div class="text-lg mb-2">No variables found</div>
          <div class="text-sm">
            {{ variableStore.searchQuery ? 'Try a different search' : 'Create your first variable to get started' }}
          </div>
        </div>
      </div>

      <table v-else class="w-full bg-slate-800 rounded-lg overflow-hidden">
        <thead class="bg-slate-700">
          <tr>
            <th class="px-6 py-3 text-left text-xs font-medium text-gray-300 uppercase tracking-wider">
              Name
            </th>
            <th class="px-6 py-3 text-left text-xs font-medium text-gray-300 uppercase tracking-wider">
              Type
            </th>
            <th class="px-6 py-3 text-left text-xs font-medium text-gray-300 uppercase tracking-wider">
              Value
            </th>
            <th class="px-6 py-3 text-left text-xs font-medium text-gray-300 uppercase tracking-wider">
              Description
            </th>
            <th class="px-6 py-3 text-right text-xs font-medium text-gray-300 uppercase tracking-wider">
              Actions
            </th>
          </tr>
        </thead>
        <tbody class="divide-y divide-slate-600/50">
          <tr
            v-for="variable in variableStore.filteredVariables"
            :key="variable.id"
            class="hover:bg-slate-700/50 transition-colors"
          >
            <td class="px-6 py-4 whitespace-nowrap">
              <div class="font-mono text-sm text-primary-400">{{ variable.name }}</div>
              <div class="text-xs text-gray-500 font-mono">&#123;&#123; env.{{ variable.name }} &#125;&#125;</div>
            </td>
            <td class="px-6 py-4 whitespace-nowrap">
              <span
                :class="[
                  'px-2 py-1 text-xs rounded-full',
                  variable.value_type === 'secret'
                    ? 'bg-yellow-900/30 text-yellow-300'
                    : 'bg-blue-900/30 text-blue-300'
                ]"
              >
                {{ variable.value_type }}
              </span>
            </td>
            <td class="px-6 py-4">
              <span class="font-mono text-sm text-gray-300">{{ variable.value }}</span>
            </td>
            <td class="px-6 py-4">
              <span class="text-sm text-gray-400">{{
                variable.description || '-'
              }}</span>
            </td>
            <td class="px-6 py-4 whitespace-nowrap text-right text-sm font-medium">
              <div class="flex items-center justify-end space-x-2">
                <button
                  @click="openEditModal(variable)"
                  class="text-primary-400 hover:text-primary-300 transition-colors"
                  title="Edit"
                >
                  <svg class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
                  </svg>
                </button>
                <button
                  @click="confirmDelete(variable)"
                  class="text-red-400 hover:text-red-300 transition-colors"
                  title="Delete"
                >
                  <svg class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                  </svg>
                </button>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Create/Edit Modal -->
    <div
      v-if="showModal"
      class="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center p-4"
      @click.self="closeModal"
    >
      <div class="bg-slate-800 rounded-lg border border-slate-700 w-full max-w-2xl shadow-2xl">
        <div class="flex items-center justify-between p-6 border-b border-slate-700">
          <h3 class="text-lg font-semibold text-white">
            {{ editingVariable ? 'Edit Variable' : 'Add Variable' }}
          </h3>
          <button
            @click="closeModal"
            class="text-gray-400 hover:text-gray-200 transition-colors"
          >
            <svg class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <div class="p-6 space-y-4">
          <!-- Name Field -->
          <div>
            <label class="block text-sm font-medium text-gray-300 mb-2">
              Variable Name <span class="text-red-400">*</span>
            </label>
            <input
              v-model="form.name"
              :disabled="!!editingVariable"
              type="text"
              placeholder="API_KEY"
              class="w-full bg-slate-700 border border-slate-600 text-gray-100 px-4 py-2 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500 disabled:opacity-50 disabled:cursor-not-allowed font-mono"
              @input="form.name = form.name.toUpperCase()"
            />
            <div class="mt-1 text-xs text-gray-400">
              Uppercase letters, numbers, and underscores only
            </div>
            <div v-if="editingVariable" class="mt-1 text-xs text-gray-400">
              Template: <span class="font-mono text-primary-400">&#123;&#123; env.{{ form.name }} &#125;&#125;</span>
            </div>
            <div v-else class="mt-1 text-xs text-gray-400">
              Preview: <span class="font-mono text-primary-400">&#123;&#123; env.{{ form.name || 'VARIABLE_NAME' }} &#125;&#125;</span>
            </div>
          </div>

          <!-- Type Field (only for create) -->
          <div v-if="!editingVariable">
            <label class="block text-sm font-medium text-gray-300 mb-2">
              Type <span class="text-red-400">*</span>
            </label>
            <div class="flex space-x-4">
              <label class="flex items-center space-x-2 cursor-pointer">
                <input
                  v-model="form.value_type"
                  type="radio"
                  value="text"
                  class="text-primary-600 focus:ring-primary-500"
                />
                <span class="text-gray-300">Text</span>
              </label>
              <label class="flex items-center space-x-2 cursor-pointer">
                <input
                  v-model="form.value_type"
                  type="radio"
                  value="secret"
                  class="text-primary-600 focus:ring-primary-500"
                />
                <span class="text-gray-300">Secret (encrypted)</span>
              </label>
            </div>
          </div>

          <!-- Value Field -->
          <div>
            <label class="block text-sm font-medium text-gray-300 mb-2">
              Value <span class="text-red-400">*</span>
            </label>
            <div class="relative">
              <input
                v-model="form.value"
                :type="showValue ? 'text' : 'password'"
                placeholder="Enter value..."
                class="w-full bg-slate-700 border border-slate-600 text-gray-100 px-4 py-2 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500 font-mono"
              />
              <button
                v-if="form.value_type === 'secret'"
                @click="showValue = !showValue"
                class="absolute right-3 top-2.5 text-gray-400 hover:text-gray-200"
                type="button"
              >
                <svg v-if="showValue" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l3.59 3.59m0 0A9.953 9.953 0 0112 5c4.478 0 8.268 2.943 9.543 7a10.025 10.025 0 01-4.132 5.411m0 0L21 21" />
                </svg>
                <svg v-else class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
                </svg>
              </button>
            </div>
          </div>

          <!-- Description Field -->
          <div>
            <label class="block text-sm font-medium text-gray-300 mb-2">
              Description
            </label>
            <textarea
              v-model="form.description"
              placeholder="Optional description..."
              rows="3"
              class="w-full bg-slate-700 border border-slate-600 text-gray-100 px-4 py-2 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500 resize-none"
            />
          </div>
        </div>

        <div class="flex items-center justify-end space-x-3 p-6 border-t border-slate-700">
          <button
            @click="closeModal"
            class="px-4 py-2 text-gray-300 hover:text-white border border-slate-600 hover:border-slate-500 rounded-md transition-colors"
            :disabled="submitting"
          >
            Cancel
          </button>
          <button
            @click="saveVariable"
            :disabled="!isFormValid || submitting"
            class="px-4 py-2 bg-primary-600 hover:bg-primary-700 disabled:bg-gray-600 text-white rounded-md transition-colors"
          >
            {{ submitting ? 'Saving...' : (editingVariable ? 'Update' : 'Create') }}
          </button>
        </div>
      </div>
    </div>

    <!-- Delete Confirmation Modal -->
    <div
      v-if="showDeleteModal"
      class="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center p-4"
      @click.self="closeDeleteModal"
    >
      <div class="bg-slate-800 rounded-lg border border-slate-700 w-full max-w-md shadow-2xl">
        <div class="p-6">
          <h3 class="text-lg font-semibold text-white mb-4">Delete Variable</h3>
          <p class="text-gray-300 mb-2">
            Are you sure you want to delete the variable
            <span class="font-mono text-primary-400">{{ variableToDelete?.name }}</span>?
          </p>
          <p class="text-gray-400 text-sm">
            This action cannot be undone. Any workflows using this variable will fail during execution.
          </p>
        </div>

        <div class="flex items-center justify-end space-x-3 p-6 border-t border-slate-700">
          <button
            @click="closeDeleteModal"
            class="px-4 py-2 text-gray-300 hover:text-white border border-slate-600 hover:border-slate-500 rounded-md transition-colors"
            :disabled="submitting"
          >
            Cancel
          </button>
          <button
            @click="deleteVariable"
            :disabled="submitting"
            class="px-4 py-2 bg-red-600 hover:bg-red-700 disabled:bg-gray-600 text-white rounded-md transition-colors"
          >
            {{ submitting ? 'Deleting...' : 'Delete' }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useVariableStore } from '../stores/variables'
import HeaderComponent from '../components/HeaderComponent.vue'
import type { Variable, CreateVariableRequest, UpdateVariableRequest } from '../types/variable'

const variableStore = useVariableStore()

const showModal = ref(false)
const showDeleteModal = ref(false)
const editingVariable = ref<Variable | null>(null)
const variableToDelete = ref<Variable | null>(null)
const submitting = ref(false)
const showValue = ref(false)

const form = ref({
  name: '',
  value_type: 'text' as 'text' | 'secret',
  value: '',
  description: ''
})

const isFormValid = computed(() => {
  return form.value.name.trim() !== '' && form.value.value.trim() !== ''
})

function openCreateModal() {
  editingVariable.value = null
  form.value = {
    name: '',
    value_type: 'text',
    value: '',
    description: ''
  }
  showValue.value = false
  showModal.value = true
}

function openEditModal(variable: Variable) {
  editingVariable.value = variable
  form.value = {
    name: variable.name,
    value_type: variable.value_type,
    value: '', // Don't pre-fill value for security
    description: variable.description || ''
  }
  showValue.value = false
  showModal.value = true
}

function closeModal() {
  showModal.value = false
  editingVariable.value = null
}

async function saveVariable() {
  if (!isFormValid.value) return

  submitting.value = true
  try {
    if (editingVariable.value) {
      // Update existing variable
      const updateData: UpdateVariableRequest = {
        value: form.value.value,
        description: form.value.description || undefined
      }
      await variableStore.updateVariable(editingVariable.value.id, updateData)
    } else {
      // Create new variable
      const createData: CreateVariableRequest = {
        name: form.value.name,
        value_type: form.value.value_type,
        value: form.value.value,
        description: form.value.description || undefined
      }
      await variableStore.createVariable(createData)
    }
    closeModal()
  } catch (error) {
    console.error('Error saving variable:', error)
  } finally {
    submitting.value = false
  }
}

function confirmDelete(variable: Variable) {
  variableToDelete.value = variable
  showDeleteModal.value = true
}

function closeDeleteModal() {
  showDeleteModal.value = false
  variableToDelete.value = null
}

async function deleteVariable() {
  if (!variableToDelete.value) return

  submitting.value = true
  try {
    await variableStore.deleteVariable(variableToDelete.value.id)
    closeDeleteModal()
  } catch (error) {
    console.error('Error deleting variable:', error)
  } finally {
    submitting.value = false
  }
}

onMounted(() => {
  variableStore.fetchVariables()
})
</script>
