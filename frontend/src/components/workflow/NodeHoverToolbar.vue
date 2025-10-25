<template>
  <Transition
    enter-active-class="transition-opacity duration-200 ease-out"
    leave-active-class="transition-opacity duration-150 ease-in"
    enter-from-class="opacity-0"
    enter-to-class="opacity-100"
    leave-from-class="opacity-100"
    leave-to-class="opacity-0"
  >
    <div
      v-if="visible"
      ref="toolbarRef"
      class="absolute z-50 flex items-center"
      :style="toolbarStyle"
    >
      <!-- Toolbar Container -->
      <div class="glass-medium rounded-lg border border-slate-600/50 shadow-xl overflow-visible">
        <!-- Search Input -->
        <div class="relative">
          <input
            ref="searchInputRef"
            v-model="searchQuery"
            type="text"
            placeholder="Search nodes..."
            class="w-96 px-3 py-2 bg-slate-700/50 text-white text-sm placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500 rounded-lg"
            @focus="onSearchFocus"
            @blur="onSearchBlur"
            @keydown="handleKeyDown"
            aria-label="Search node types"
            aria-autocomplete="list"
            :aria-expanded="showResults"
            aria-controls="node-search-results"
          />

          <!-- Search Icon -->
          <div class="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none">
            <svg class="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
            </svg>
          </div>
        </div>

        <!-- Typeahead Results Dropdown -->
        <Transition
          enter-active-class="transition-all duration-150 ease-out"
          leave-active-class="transition-all duration-100 ease-in"
          enter-from-class="opacity-0 -translate-y-2"
          enter-to-class="opacity-100 translate-y-0"
          leave-from-class="opacity-100 translate-y-0"
          leave-to-class="opacity-0 -translate-y-2"
        >
          <div
            v-if="showResults && filteredNodes.length > 0"
            id="node-search-results"
            role="listbox"
            class="absolute top-full mt-1 w-full max-h-64 overflow-y-auto bg-slate-800 rounded-lg border border-slate-600 shadow-2xl"
          >
            <div
              v-for="(nodeType, index) in filteredNodes"
              :key="nodeType.type"
              role="option"
              :aria-selected="index === selectedIndex"
              class="flex items-center px-3 py-2 cursor-pointer transition-colors hover:bg-slate-600/30"
              :class="{
                'bg-slate-600/50': index === selectedIndex
              }"
              @click="selectNode(nodeType)"
              @mouseenter="selectedIndex = index"
            >
              <!-- Node Icon/Color Indicator -->
              <div
                class="w-3 h-3 rounded-full mr-2 flex-shrink-0"
                :style="{ backgroundColor: nodeType.color }"
              />

              <!-- Node Info -->
              <div class="flex-1 min-w-0">
                <div class="text-sm font-medium text-white" v-html="highlightMatch(nodeType.label)" />
                <div class="text-xs text-gray-400 truncate">{{ nodeType.description }}</div>
              </div>
            </div>
          </div>
        </Transition>

        <!-- No Results Message -->
        <div
          v-if="showResults && searchQuery && filteredNodes.length === 0"
          class="absolute top-full mt-1 w-full bg-slate-800 rounded-lg border border-slate-600 shadow-2xl p-3"
        >
          <p class="text-sm text-gray-400 text-center">No nodes found</p>
        </div>
      </div>
    </div>
  </Transition>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick, type CSSProperties } from 'vue'
import type { NodeTypeDefinition } from '../../types/nodes'

interface Props {
  visible: boolean
  nodeId: string
  position: { x: number; y: number }
  nodeTypes: NodeTypeDefinition[]
}

interface Emits {
  (e: 'createNode', nodeType: NodeTypeDefinition): void
  (e: 'dismiss'): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

// Refs
const toolbarRef = ref<HTMLElement | null>(null)
const searchInputRef = ref<HTMLInputElement | null>(null)
const searchQuery = ref('')
const selectedIndex = ref(0)
const searchFocused = ref(false)

// Computed
const showResults = computed(() => searchFocused.value || searchQuery.value.length > 0)

const filteredNodes = computed(() => {
  // Exclude trigger nodes - they should only be added once per workflow
  const availableNodeTypes = props.nodeTypes.filter(nodeType => nodeType.type !== 'trigger')

  if (!searchQuery.value) {
    return availableNodeTypes
  }

  const query = searchQuery.value.toLowerCase()
  return availableNodeTypes.filter(nodeType =>
    nodeType.label.toLowerCase().includes(query) ||
    nodeType.description.toLowerCase().includes(query) ||
    nodeType.type.toLowerCase().includes(query)
  )
})

const toolbarStyle = computed<CSSProperties>(() => {
  return {
    left: `${props.position.x}px`,
    top: `${props.position.y}px`,
  }
})

// Watchers
watch(() => props.visible, (newVisible) => {
  if (newVisible) {
    // Auto-focus the search input when toolbar becomes visible
    nextTick(() => {
      searchInputRef.value?.focus()
    })
  } else {
    // Reset state when toolbar is hidden
    searchQuery.value = ''
    selectedIndex.value = 0
    searchFocused.value = false
  }
})

watch(filteredNodes, () => {
  // Reset selected index when filtered results change
  selectedIndex.value = 0
})

// Methods
function highlightMatch(text: string): string {
  if (!searchQuery.value) return text

  const query = searchQuery.value.toLowerCase()
  const index = text.toLowerCase().indexOf(query)

  if (index === -1) return text

  const before = text.substring(0, index)
  const match = text.substring(index, index + searchQuery.value.length)
  const after = text.substring(index + searchQuery.value.length)

  return `${before}<span class="bg-blue-500/30 text-blue-300">${match}</span>${after}`
}

function selectNode(nodeType: NodeTypeDefinition) {
  emit('createNode', nodeType)
  emit('dismiss')
}

function handleKeyDown(event: KeyboardEvent) {
  switch (event.key) {
    case 'ArrowDown':
      event.preventDefault()
      selectedIndex.value = Math.min(selectedIndex.value + 1, filteredNodes.value.length - 1)
      break
    case 'ArrowUp':
      event.preventDefault()
      selectedIndex.value = Math.max(selectedIndex.value - 1, 0)
      break
    case 'Enter':
      event.preventDefault()
      if (filteredNodes.value[selectedIndex.value]) {
        selectNode(filteredNodes.value[selectedIndex.value])
      }
      break
    case 'Escape':
      event.preventDefault()
      emit('dismiss')
      break
  }
}

function onSearchFocus() {
  searchFocused.value = true
}

function onSearchBlur() {
  // Delay to allow click events on results to fire
  setTimeout(() => {
    searchFocused.value = false
  }, 200)
}
</script>
