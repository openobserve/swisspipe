<template>
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
            <router-link
              to="/settings"
              class="px-3 py-2 text-sm font-medium transition-colors rounded-md"
              :class="$route.path === '/settings' ? 'text-primary-400 bg-primary-900/20' : 'text-gray-300 hover:text-primary-400 hover:bg-primary-900/10'"
            >
              Settings
            </router-link>
            <a
              href="/api-docs"
              target="_blank"
              rel="noopener noreferrer"
              class="px-3 py-2 text-sm font-medium transition-colors rounded-md flex items-center space-x-1 text-gray-300 hover:text-primary-400 hover:bg-primary-900/10"
            >
              <svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
              </svg>
              <span>API</span>
              <svg class="h-3 w-3 ml-1" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
              </svg>
            </a>
          </nav>
        </div>
        <div class="flex items-center space-x-4">
          <span class="text-sm text-gray-300">Welcome, </span>
          <span class="text-sm font-medium text-primary-300">{{ displayName }}</span>
          <button
            @click="handleLogout"
            class="bg-red-600/20 hover:bg-red-600/30 text-red-300 hover:text-red-200 border border-red-600/30 hover:border-red-600/50 px-4 py-2 rounded-md text-sm font-medium transition-all duration-200 backdrop-blur-sm"
          >
            Logout
          </button>
        </div>
      </div>
    </div>
  </header>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '../stores/auth'

const router = useRouter()
const authStore = useAuthStore()

// Display name logic: prioritize name (OAuth) over username (basic auth), fallback to email
const displayName = computed(() => {
  const user = authStore.user
  if (!user) return 'Guest'

  // For OAuth users: use name, fallback to email if no name
  if (user.name && user.name.trim()) return user.name
  if (user.email && user.email.trim()) return user.email

  // For basic auth users: use username
  if (user.username && user.username.trim()) return user.username

  // Final fallback
  return user.id || 'User'
})

function handleLogout() {
  authStore.logout()
  router.push('/login')
}
</script>