<template>
  <div class="min-h-screen flex items-center justify-center bg-gray-50">
    <div class="max-w-md w-full text-center">
      <div class="mb-4">
        <div v-if="isLoading" class="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto"></div>
        <div v-else-if="error" class="text-red-600">
          <svg class="h-12 w-12 mx-auto mb-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z" />
          </svg>
        </div>
        <div v-else class="text-green-600">
          <svg class="h-12 w-12 mx-auto mb-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
          </svg>
        </div>
      </div>

      <h2 class="text-2xl font-bold text-gray-900 mb-2">
        <span v-if="isLoading">Processing Authentication...</span>
        <span v-else-if="error">Authentication Failed</span>
        <span v-else>Authentication Successful!</span>
      </h2>

      <p class="text-gray-600 mb-4">
        <span v-if="isLoading">Please wait while we complete your sign-in...</span>
        <span v-else-if="error">{{ error }}</span>
        <span v-else>Redirecting to your dashboard...</span>
      </p>

      <button
        v-if="error"
        @click="$router.push('/login')"
        class="inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md text-white bg-blue-600 hover:bg-blue-700"
      >
        Back to Login
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useAuthStore } from '../stores/auth'

const route = useRoute()
const router = useRouter()
const authStore = useAuthStore()

const isLoading = ref(true)
const error = ref('')

onMounted(async () => {
  try {
    // Check URL parameters for OAuth callback status
    const urlParams = new URLSearchParams(window.location.search)
    const errorParam = urlParams.get('error')

    if (errorParam) {
      error.value = `Authentication failed: ${errorParam}`
      isLoading.value = false
      return
    }

    // Try to refresh auth state to see if OAuth was successful
    await authStore.initializeAuth()

    if (authStore.isAuthenticated) {
      // OAuth was successful, redirect to dashboard
      setTimeout(() => {
        router.push('/workflows')
      }, 2000)
    } else {
      error.value = 'Authentication was not successful. Please try again.'
    }
  } catch (err) {
    console.error('OAuth callback error:', err)
    error.value = 'An error occurred during authentication.'
  } finally {
    isLoading.value = false
  }
})
</script>