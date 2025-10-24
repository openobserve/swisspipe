<template>
  <div class="fixed top-0 right-0 h-full w-96 bg-slate-900 border-l border-slate-700 shadow-xl z-40 flex flex-col">
    <!-- Header -->
    <div class="px-4 py-3 border-b border-slate-700 flex items-center justify-between">
      <h2 class="text-lg font-semibold text-gray-200">Version History</h2>
      <div class="flex items-center space-x-2">
        <button
          @click="loadVersions"
          :disabled="loading"
          class="p-1 text-gray-400 hover:text-gray-200 transition-colors disabled:opacity-50"
          title="Refresh"
        >
          <svg class="h-5 w-5" :class="{ 'animate-spin': loading }" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
          </svg>
        </button>
        <button
          @click="$emit('close')"
          class="p-1 text-gray-400 hover:text-gray-200 transition-colors"
          title="Close"
        >
          <svg class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-y-auto p-4">
      <!-- Loading State -->
      <div v-if="loading && versions.length === 0" class="flex items-center justify-center h-32">
        <div class="text-gray-400">Loading versions...</div>
      </div>

      <!-- Error State -->
      <div v-else-if="error" class="text-red-400 text-sm p-4 bg-red-900/20 rounded-md">
        {{ error }}
      </div>

      <!-- Empty State -->
      <div v-else-if="versions.length === 0" class="flex flex-col items-center justify-center h-32 text-gray-400">
        <svg class="h-12 w-12 mb-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
        <p>No version history yet</p>
      </div>

      <!-- Version List -->
      <div v-else class="space-y-3">
        <div
          v-for="version in versions"
          :key="version.id"
          class="bg-slate-800 border border-slate-700 rounded-lg p-3 hover:border-slate-600 transition-colors"
        >
          <!-- Version Header -->
          <div class="flex items-start justify-between mb-2">
            <div class="flex items-center space-x-2">
              <span class="text-blue-400 font-mono text-sm font-semibold">v{{ version.version_number }}</span>
              <span class="text-gray-500 text-xs">•</span>
              <span class="text-gray-400 text-xs">{{ formatTime(version.created_at) }}</span>
            </div>
            <span class="text-gray-500 text-xs">@{{ version.changed_by }}</span>
          </div>

          <!-- Commit Message -->
          <div class="text-gray-200 text-sm font-medium mb-1">
            {{ version.commit_message }}
          </div>

          <!-- Commit Description (if exists) -->
          <div v-if="version.commit_description" class="mt-2">
            <button
              @click="toggleExpanded(version.id)"
              class="text-xs text-blue-400 hover:text-blue-300 transition-colors flex items-center"
            >
              <svg
                class="h-3 w-3 mr-1 transition-transform"
                :class="{ 'rotate-90': expandedVersions.has(version.id) }"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
              </svg>
              {{ expandedVersions.has(version.id) ? 'Hide' : 'Show' }} description
            </button>
            <div
              v-if="expandedVersions.has(version.id)"
              class="mt-2 text-gray-400 text-xs whitespace-pre-wrap"
            >
              {{ version.commit_description }}
            </div>
          </div>

          <!-- Workflow Name -->
          <div class="mt-2 text-gray-500 text-xs">
            Workflow: {{ version.workflow_name }}
          </div>
        </div>
      </div>

      <!-- Load More Button -->
      <div v-if="hasMore" class="mt-4 flex justify-center">
        <button
          @click="loadMore"
          :disabled="loading"
          class="px-4 py-2 bg-slate-700 hover:bg-slate-600 text-gray-200 rounded-md transition-colors disabled:opacity-50 disabled:cursor-not-allowed text-sm"
        >
          <span v-if="loading">Loading...</span>
          <span v-else>Load More</span>
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { getVersions } from '../../services/api'

interface Props {
  workflowId: string
}

interface Version {
  id: string
  workflow_id: string
  version_number: number
  commit_message: string
  commit_description: string | null
  changed_by: string
  created_at: number
  workflow_name: string
}


const props = defineProps<Props>()

defineEmits<{
  close: []
}>()

const versions = ref<Version[]>([])
const loading = ref(false)
const error = ref<string | null>(null)
const expandedVersions = ref(new Set<string>())
const total = ref(0)
const limit = 50
const offset = ref(0)

const hasMore = computed(() => versions.value.length < total.value)

async function loadVersions() {
  loading.value = true
  error.value = null
  offset.value = 0

  try {
    const response = await getVersions(props.workflowId, limit, 0)
    versions.value = response.versions
    total.value = response.total
    offset.value = response.versions.length
  } catch (e) {
    error.value = e instanceof Error ? e.message : 'Failed to load version history'
    console.error('Failed to load versions:', e)
  } finally {
    loading.value = false
  }
}

async function loadMore() {
  if (loading.value || !hasMore.value) return

  loading.value = true
  error.value = null

  try {
    const response = await getVersions(props.workflowId, limit, offset.value)
    versions.value.push(...response.versions)
    offset.value += response.versions.length
  } catch (e) {
    error.value = e instanceof Error ? e.message : 'Failed to load more versions'
    console.error('Failed to load more versions:', e)
  } finally {
    loading.value = false
  }
}

function toggleExpanded(versionId: string) {
  if (expandedVersions.value.has(versionId)) {
    expandedVersions.value.delete(versionId)
  } else {
    expandedVersions.value.add(versionId)
  }
}

function formatTime(timestamp: number): string {
  const now = Date.now() * 1000 // Convert to microseconds
  const diff = now - timestamp
  const seconds = Math.floor(diff / 1_000_000)
  const minutes = Math.floor(seconds / 60)
  const hours = Math.floor(minutes / 60)
  const days = Math.floor(hours / 24)

  if (seconds < 60) return 'just now'
  if (minutes < 60) return `${minutes}m ago`
  if (hours < 24) return `${hours}h ago`
  if (days < 7) return `${days}d ago`

  // Format as date for older entries
  const date = new Date(timestamp / 1000) // Convert microseconds to milliseconds
  return date.toLocaleDateString()
}

onMounted(() => {
  loadVersions()
})
</script>
