<template>
  <div class="h-screen flex flex-col text-gray-100">
    <!-- Header -->
    <header class="glass-dark border-b border-slate-700/50 flex-shrink-0">
      <div class="px-6 py-4">
        <div class="flex items-center justify-between">
          <div class="flex items-center space-x-4">
            <h1 class="text-2xl font-bold text-white">SwissPipe</h1>
            <nav class="flex space-x-6">
              <router-link
                to="/workflows"
                class="px-3 py-2 text-sm font-medium transition-colors rounded-md"
                :class="$route.path === '/workflows' ? 'text-primary-400 bg-primary-900/20' : 'text-gray-300 hover:text-primary-400 hover:bg-primary-900/10'"
              >
                Workflows
              </router-link>
              <router-link
                to="/executions"
                class="px-3 py-2 text-sm font-medium transition-colors rounded-md"
                :class="$route.path === '/executions' ? 'text-primary-400 bg-primary-900/20' : 'text-gray-300 hover:text-primary-400 hover:bg-primary-900/10'"
              >
                Executions
              </router-link>
            </nav>
          </div>
          <div class="flex items-center space-x-4">
            <span class="text-sm text-gray-300">
              Welcome, {{ authStore.user?.username }}
            </span>
            <button
              @click="executionStore.fetchExecutions()"
              class="bg-primary-600 hover:bg-primary-700 text-white px-4 py-2 rounded-md font-medium transition-colors"
              :disabled="executionStore.loading"
            >
              <ArrowPathIcon v-if="executionStore.loading" class="h-4 w-4 animate-spin" />
              <span v-else>Refresh</span>
            </button>
            <button
              @click="handleLogout"
              class="text-gray-300 hover:text-white px-3 py-2 rounded-md text-sm font-medium transition-colors"
            >
              Logout
            </button>
          </div>
        </div>
      </div>
    </header>

    <!-- Main Content -->
    <main class="flex-1 flex flex-col p-6 min-h-0">
      <!-- Search and Filters -->
      <div class="mb-6 flex items-center justify-between flex-shrink-0">
        <div class="flex items-center space-x-4">
          <div class="relative">
            <input
              v-model="executionStore.searchTerm"
              type="text"
              placeholder="Search executions..."
              class="glass border border-slate-600/50 text-gray-100 px-4 py-2 pl-10 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-primary-500/50 w-64"
            />
            <MagnifyingGlassIcon class="h-5 w-5 text-gray-400 absolute left-3 top-2.5" />
          </div>
        </div>
        <div class="text-sm text-gray-400">
          {{ executionStore.executionCount }} executions
        </div>
      </div>

      <!-- Executions Table -->
      <div class="glass-medium rounded-lg shadow-2xl overflow-hidden flex-1 flex flex-col min-h-0">
        <div v-if="executionStore.loading" class="p-8 text-center">
          <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-500 mx-auto"></div>
          <p class="mt-2 text-gray-400">Loading executions...</p>
        </div>

        <div v-else-if="executionStore.error" class="p-8 text-center">
          <p class="text-red-400">{{ executionStore.error }}</p>
          <button
            @click="executionStore.fetchExecutions()"
            class="mt-4 bg-primary-600 hover:bg-primary-700 text-white px-4 py-2 rounded-md"
          >
            Retry
          </button>
        </div>

        <div v-else-if="executionStore.filteredExecutions.length === 0" class="p-8 text-center">
          <p class="text-gray-400">No executions found</p>
        </div>

        <div v-else class="flex-1 overflow-x-auto overflow-y-auto">
          <table class="min-w-full divide-y divide-slate-600">
            <thead class="sticky top-0 z-10 bg-slate-900">
              <tr>
                <th class="px-6 py-3 text-left text-xs font-medium text-gray-300 uppercase tracking-wider">
                  Execution ID
                </th>
                <th class="px-6 py-3 text-left text-xs font-medium text-gray-300 uppercase tracking-wider">
                  Workflow
                </th>
                <th class="px-6 py-3 text-left text-xs font-medium text-gray-300 uppercase tracking-wider">
                  Status
                </th>
                <th class="px-6 py-3 text-left text-xs font-medium text-gray-300 uppercase tracking-wider">
                  Current Node
                </th>
                <th class="px-6 py-3 text-left text-xs font-medium text-gray-300 uppercase tracking-wider">
                  Started
                </th>
                <th class="px-6 py-3 text-left text-xs font-medium text-gray-300 uppercase tracking-wider">
                  Duration
                </th>
                <th class="px-6 py-3 text-right text-xs font-medium text-gray-300 uppercase tracking-wider">
                  Actions
                </th>
              </tr>
            </thead>
            <tbody class="divide-y divide-slate-600/50">
              <tr
                v-for="execution in executionStore.filteredExecutions"
                :key="execution.id"
                class="hover:bg-white/5 transition-all duration-200 cursor-pointer backdrop-blur-sm"
                @click="executionStore.openExecutionDetails(execution)"
              >
                <td class="px-6 py-4">
                  <div class="text-sm font-medium text-white font-mono">
                    {{ execution.id }}
                  </div>
                </td>
                <td class="px-6 py-4">
                  <div class="text-sm text-gray-300 font-mono">{{ execution.workflow_id }}</div>
                </td>
                <td class="px-6 py-4 whitespace-nowrap">
                  <span 
                    class="px-2 inline-flex text-xs leading-5 font-semibold rounded-full"
                    :class="getStatusColorClass(execution.status)"
                  >
                    {{ execution.status }}
                  </span>
                </td>
                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-300">
                  {{ execution.current_node_id || '-' }}
                </td>
                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-300">
                  {{ executionStore.formatTimestamp(execution.started_at) }}
                </td>
                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-300">
                  {{ executionStore.formatDuration(execution.started_at, execution.completed_at) }}
                </td>
                <td class="px-6 py-4 whitespace-nowrap text-right text-sm font-medium">
                  <div class="flex items-center justify-end space-x-2">
                    <button
                      v-if="execution.status === 'running' || execution.status === 'pending'"
                      @click.stop="cancelExecution(execution)"
                      class="text-red-400 hover:text-red-300 transition-colors"
                      title="Cancel"
                    >
                      <StopIcon class="h-5 w-5" />
                    </button>
                    <button
                      @click.stop="executionStore.openExecutionDetails(execution)"
                      class="text-primary-400 hover:text-primary-300 transition-colors"
                      title="View Details"
                    >
                      <EyeIcon class="h-5 w-5" />
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </main>

    <!-- Execution Details Side Panel -->
    <ExecutionDetailsPanel />
  </div>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import { useRouter } from 'vue-router'
import {
  MagnifyingGlassIcon,
  EyeIcon,
  StopIcon,
  ArrowPathIcon
} from '@heroicons/vue/24/outline'
import { useExecutionStore } from '../stores/executions'
import { useAuthStore } from '../stores/auth'
import ExecutionDetailsPanel from '../components/ExecutionDetailsPanel.vue'
import type { WorkflowExecution, ExecutionStatus } from '../types/execution'

const router = useRouter()
const executionStore = useExecutionStore()
const authStore = useAuthStore()

onMounted(() => {
  executionStore.fetchExecutions()
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

async function cancelExecution(execution: WorkflowExecution) {
  try {
    await executionStore.cancelExecution(execution.id)
  } catch (error) {
    console.error('Failed to cancel execution:', error)
  }
}

function handleLogout() {
  authStore.logout()
  router.push('/login')
}
</script>