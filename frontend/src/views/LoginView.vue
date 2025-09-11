<template>
  <div class="min-h-screen flex items-center justify-center bg-gray-50 py-12 px-4 sm:px-6 lg:px-8">
    <div class="max-w-md w-full space-y-8">
      <div>
        <div class="mx-auto h-12 w-auto flex items-center justify-center">
          <svg class="h-10 w-10 text-blue-600" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
        </div>
        <h2 class="mt-6 text-center text-3xl font-extrabold text-gray-900">
          Sign in to Swisspipe
        </h2>
        <p class="mt-2 text-center text-sm text-gray-600">
          Workflow automation platform
        </p>
      </div>
      <form class="mt-8 space-y-6" @submit.prevent="handleLogin">
        <input type="hidden" name="remember" value="true" />
        <div class="rounded-md shadow-sm -space-y-px">
          <div>
            <label for="username" class="sr-only">Username</label>
            <input
              id="username"
              v-model="username"
              name="username"
              type="text"
              autocomplete="username"
              required
              class="appearance-none rounded-none relative block w-full px-3 py-2 border border-gray-300 placeholder-gray-500 text-gray-900 rounded-t-md focus:outline-none focus:ring-blue-500 focus:border-blue-500 focus:z-10 sm:text-sm"
              placeholder="Username"
              :disabled="isLoading"
            />
          </div>
          <div>
            <label for="password" class="sr-only">Password</label>
            <input
              id="password"
              v-model="password"
              name="password"
              type="password"
              autocomplete="current-password"
              required
              class="appearance-none rounded-none relative block w-full px-3 py-2 border border-gray-300 placeholder-gray-500 text-gray-900 rounded-b-md focus:outline-none focus:ring-blue-500 focus:border-blue-500 focus:z-10 sm:text-sm"
              placeholder="Password"
              :disabled="isLoading"
            />
          </div>
        </div>

        <div v-if="errorMessage" class="text-red-600 text-sm text-center">
          {{ errorMessage }}
        </div>

        <div>
          <button
            type="submit"
            class="group relative w-full flex justify-center py-2 px-4 border border-transparent text-sm font-medium rounded-md text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50 disabled:cursor-not-allowed"
            :disabled="isLoading"
          >
            <span class="absolute left-0 inset-y-0 flex items-center pl-3">
              <svg
                v-if="!isLoading"
                class="h-5 w-5 text-blue-500 group-hover:text-blue-400"
                xmlns="http://www.w3.org/2000/svg"
                viewBox="0 0 20 20"
                fill="currentColor"
                aria-hidden="true"
              >
                <path
                  fill-rule="evenodd"
                  d="M5 9V7a5 5 0 0110 0v2a2 2 0 012 2v5a2 2 0 01-2 2H5a2 2 0 01-2-2v-5a2 2 0 012-2zm8-2v2H7V7a3 3 0 016 0z"
                  clip-rule="evenodd"
                />
              </svg>
              <div
                v-else
                class="animate-spin rounded-full h-5 w-5 border-b-2 border-white"
              ></div>
            </span>
            {{ isLoading ? 'Signing in...' : 'Sign in' }}
          </button>
        </div>
      </form>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '../stores/auth'

const router = useRouter()
const authStore = useAuthStore()

const username = ref('')
const password = ref('')
const errorMessage = ref('')
const isLoading = ref(false)

const handleLogin = async () => {
  if (!username.value || !password.value) {
    errorMessage.value = 'Please enter both username and password'
    return
  }

  isLoading.value = true
  errorMessage.value = ''

  try {
    const success = await authStore.login(username.value, password.value)
    
    if (success) {
      router.push('/workflows')
    } else {
      errorMessage.value = 'Invalid username or password'
    }
  } catch (error) {
    errorMessage.value = 'An error occurred during login'
    console.error('Login error:', error)
  } finally {
    isLoading.value = false
  }
}
</script>