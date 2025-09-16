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