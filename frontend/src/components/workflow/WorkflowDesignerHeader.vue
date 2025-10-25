<template>
  <header class="glass-dark border-b border-slate-700/50 flex-shrink-0">
    <div class="px-6 py-4 flex items-center justify-between">
      <div class="flex items-center space-x-4">
        <button
          @click="$emit('navigate-back')"
          class="text-gray-400 hover:text-gray-200 transition-colors"
        >
          <ArrowLeftIcon class="h-6 w-6" />
        </button>
        <div class="flex flex-col space-y-1">
          <input
            :value="workflowName"
            @input="updateName"
            @blur="$emit('update-workflow-name')"
            class="bg-transparent text-xl font-medium text-white focus:outline-none focus:bg-white/5 focus:backdrop-blur-sm px-2 py-1 rounded transition-all duration-200"
            placeholder="Workflow Name"
          />
          <input
            :value="workflowDescription"
            @input="updateDescription"
            @blur="$emit('update-workflow-description')"
            class="bg-transparent text-sm text-gray-400 focus:outline-none focus:bg-white/5 focus:backdrop-blur-sm px-2 py-1 rounded transition-all duration-200"
            placeholder="Add a description..."
          />
        </div>
      </div>
      <div class="flex items-center space-x-3">
        <button
          @click="$emit('save-workflow')"
          :disabled="saving"
          class="bg-green-600 hover:bg-green-700 disabled:bg-gray-600 text-white px-4 py-2 rounded-md font-medium transition-colors"
        >
          {{ saving ? 'Saving...' : 'Save' }}
        </button>
        <button
          @click="$emit('show-json-view')"
          class="bg-indigo-600 hover:bg-indigo-700 text-white px-4 py-2 rounded-md font-medium transition-colors"
        >
          JSON View
        </button>
        <button
          @click="$emit('reset-workflow')"
          class="bg-gray-600 hover:bg-gray-700 text-white px-4 py-2 rounded-md font-medium transition-colors"
        >
          Reset
        </button>
        <!-- <button
          @click="$emit('toggle-ai-chat')"
          class="bg-orange-600 hover:bg-orange-700 text-white px-4 py-2 rounded-md font-medium transition-colors flex items-center space-x-2"
          title="AI Assistant"
        >
          <SparklesIcon class="h-4 w-4" />
          <span>AI Assistant</span>
        </button> -->
        <button
          @click="$emit('toggle-executions-panel')"
          class="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded-md font-medium transition-colors flex items-center space-x-2"
        >
          <ClockIcon class="h-4 w-4" />
          <span>Executions</span>
        </button>
        <button
          @click="$emit('toggle-version-history')"
          class="bg-slate-600 hover:bg-slate-700 text-white px-4 py-2 rounded-md font-medium transition-colors flex items-center space-x-2"
        >
          <svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
          <span>History</span>
        </button>
        <div class="flex items-center space-x-3 ml-4 border-l border-gray-600 pl-4">
          <span class="text-sm text-gray-300">
            {{ authStore.user?.username }}
          </span>
          <button
            @click="$emit('logout')"
            class="text-gray-300 hover:text-white px-3 py-2 rounded-md text-sm font-medium transition-colors"
          >
            Logout
          </button>
        </div>
      </div>
    </div>
  </header>
</template>

<script setup lang="ts">
import { ArrowLeftIcon, ClockIcon } from '@heroicons/vue/24/outline'
import { useAuthStore } from '../../stores/auth'

interface Props {
  workflowName: string
  workflowDescription?: string
  saving: boolean
}

interface Emits {
  (e: 'navigate-back'): void
  (e: 'update:workflowName', value: string): void
  (e: 'update:workflowDescription', value: string): void
  (e: 'update-workflow-name'): void
  (e: 'update-workflow-description'): void
  (e: 'save-workflow'): void
  (e: 'show-json-view'): void
  (e: 'reset-workflow'): void
  (e: 'toggle-node-library'): void
  (e: 'toggle-ai-chat'): void
  (e: 'toggle-executions-panel'): void
  (e: 'toggle-version-history'): void
  (e: 'logout'): void
}

defineProps<Props>()
const emit = defineEmits<Emits>()

const authStore = useAuthStore()

function updateName(event: Event) {
  const target = event.target as HTMLInputElement
  emit('update:workflowName', target.value)
}

function updateDescription(event: Event) {
  const target = event.target as HTMLInputElement
  emit('update:workflowDescription', target.value)
}
</script>
