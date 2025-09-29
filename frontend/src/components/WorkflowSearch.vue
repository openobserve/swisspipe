<template>
  <div class="workflow-search">
    <div class="relative">
      <input
        ref="inputRef"
        v-model="searchQuery"
        type="text"
        :placeholder="placeholder"
        class="w-full px-3 py-2 pr-10 bg-gray-700 border border-gray-600 rounded-md text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
        @input="onInput"
        @focus="showDropdown = true"
        @blur="onBlur"
        @keydown="handleKeyDown"
      />

      <!-- Search Icon -->
      <div class="absolute inset-y-0 right-0 flex items-center pr-3 pointer-events-none">
        <MagnifyingGlassIcon class="h-4 w-4 text-gray-400" />
      </div>

      <!-- Loading spinner -->
      <div v-if="loading" class="absolute inset-y-0 right-8 flex items-center pr-3">
        <div class="w-4 h-4 border-2 border-blue-500 border-t-transparent rounded-full animate-spin"></div>
      </div>
    </div>

    <!-- Dropdown Results -->
    <div
      v-if="showDropdown && (results.length > 0 || (searchQuery && !loading))"
      class="absolute z-50 w-full mt-1 bg-gray-800 border border-gray-600 rounded-md shadow-lg max-h-60 overflow-y-auto"
    >
      <div v-if="loading && searchQuery" class="px-3 py-2 text-gray-400 text-sm">
        Searching...
      </div>

      <div v-else-if="results.length === 0 && searchQuery" class="px-3 py-2 text-gray-400 text-sm">
        No workflows found for "{{ searchQuery }}"
      </div>

      <div v-else-if="results.length > 0">
        <button
          v-for="(workflow, index) in results"
          :key="workflow.id"
          :class="[
            'w-full text-left px-3 py-2 hover:bg-gray-700 focus:bg-gray-700 focus:outline-none',
            index === selectedIndex ? 'bg-gray-700' : ''
          ]"
          @click="selectWorkflow(workflow)"
          @mouseenter="selectedIndex = index"
        >
          <div class="text-white font-medium">{{ workflow.name }}</div>
          <div v-if="workflow.description" class="text-gray-400 text-sm truncate">
            {{ workflow.description }}
          </div>
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { MagnifyingGlassIcon } from '@heroicons/vue/24/outline'
import { debounce } from '../utils/debounce'
import apiClient, { type WorkflowSearchResult } from '../services/api'

interface Props {
  modelValue?: WorkflowSearchResult | null
  placeholder?: string
}

interface Emits {
  (e: 'update:modelValue', value: WorkflowSearchResult | null): void
  (e: 'select', workflow: WorkflowSearchResult): void
}

const props = withDefaults(defineProps<Props>(), {
  placeholder: 'Search workflows...'
})

const emit = defineEmits<Emits>()

// Refs
const inputRef = ref<HTMLInputElement>()
const searchQuery = ref('')
const results = ref<WorkflowSearchResult[]>([])
const loading = ref(false)
const showDropdown = ref(false)
const selectedIndex = ref(-1)

// Watch for external model changes
watch(() => props.modelValue, (newValue) => {
  if (newValue) {
    searchQuery.value = newValue.name
  } else {
    searchQuery.value = ''
  }
}, { immediate: true })

// Debounced search function
const debouncedSearch = debounce(async (query: string) => {
  if (!query.trim()) {
    results.value = []
    loading.value = false
    return
  }

  loading.value = true
  try {
    const data = await apiClient.searchWorkflows(query)
    results.value = data || []
  } catch (error) {
    console.error('Search error:', error)
    results.value = []
  } finally {
    loading.value = false
    selectedIndex.value = -1
  }
}, 300)

function onInput() {
  debouncedSearch(searchQuery.value)
}

function selectWorkflow(workflow: WorkflowSearchResult) {
  searchQuery.value = workflow.name
  showDropdown.value = false
  selectedIndex.value = -1
  emit('update:modelValue', workflow)
  emit('select', workflow)
  inputRef.value?.blur()
}

function onBlur() {
  // Delay hiding dropdown to allow click on results
  setTimeout(() => {
    showDropdown.value = false
    selectedIndex.value = -1
  }, 150)
}

function handleKeyDown(event: KeyboardEvent) {
  if (!showDropdown.value || results.value.length === 0) return

  switch (event.key) {
    case 'ArrowDown':
      event.preventDefault()
      selectedIndex.value = Math.min(selectedIndex.value + 1, results.value.length - 1)
      break
    case 'ArrowUp':
      event.preventDefault()
      selectedIndex.value = Math.max(selectedIndex.value - 1, -1)
      break
    case 'Enter':
      event.preventDefault()
      if (selectedIndex.value >= 0 && selectedIndex.value < results.value.length) {
        selectWorkflow(results.value[selectedIndex.value])
      }
      break
    case 'Escape':
      event.preventDefault()
      showDropdown.value = false
      selectedIndex.value = -1
      inputRef.value?.blur()
      break
  }
}

function clearSelection() {
  searchQuery.value = ''
  showDropdown.value = false
  selectedIndex.value = -1
  results.value = []
  emit('update:modelValue', null)
}

// Expose methods for parent components
defineExpose({
  focus: () => inputRef.value?.focus(),
  clear: clearSelection
})
</script>

<style scoped>
.workflow-search {
  @apply relative;
}
</style>