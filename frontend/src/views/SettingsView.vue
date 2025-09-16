<template>
  <div class="h-screen flex flex-col text-gray-100">
    <!-- Header -->
    <HeaderComponent />

    <!-- Main Content -->
    <main class="flex-1 flex flex-col p-6 min-h-0">
      <!-- Title -->
      <div class="mb-6 flex items-center justify-between flex-shrink-0">
        <div>
          <h1 class="text-2xl font-bold text-white">Settings</h1>
          <p class="text-gray-400 mt-1">Configure organization-wide settings</p>
        </div>
      </div>

      <!-- Settings Form -->
      <div class="glass-medium rounded-lg shadow-2xl p-6 max-w-2xl">
        <div v-if="loading" class="p-8 text-center">
          <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-500 mx-auto"></div>
          <p class="mt-2 text-gray-400">Loading settings...</p>
        </div>

        <div v-else-if="error" class="p-8 text-center">
          <p class="text-red-400">{{ error }}</p>
          <button
            @click="loadSettings()"
            class="mt-4 bg-primary-600 hover:bg-primary-700 text-white px-4 py-2 rounded-md"
          >
            Try Again
          </button>
        </div>

        <div v-else class="space-y-6">
          <!-- API Base URL Setting -->
          <div>
            <label for="apiBaseUrl" class="block text-sm font-medium text-gray-300 mb-2">
              API Base URL
            </label>
            <p class="text-xs text-gray-500 mb-3">
              Base URL for API endpoints that users can copy for external use. Leave empty to use the current browser origin.
            </p>
            <div class="flex space-x-3">
              <input
                id="apiBaseUrl"
                v-model="apiBaseUrlValue"
                type="url"
                placeholder="https://api.example.com"
                class="flex-1 glass border border-slate-600/50 text-gray-100 px-4 py-2 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-primary-500/50"
                :disabled="saving"
              />
              <button
                @click="saveApiBaseUrl()"
                :disabled="saving || apiBaseUrlValue === originalApiBaseUrl"
                class="bg-primary-600 hover:bg-primary-700 disabled:bg-gray-600 disabled:cursor-not-allowed text-white px-4 py-2 rounded-md font-medium transition-colors"
              >
                <span v-if="saving">Saving...</span>
                <span v-else>Save</span>
              </button>
            </div>
            <div v-if="saveMessage" class="mt-2 text-sm" :class="saveSuccess ? 'text-green-400' : 'text-red-400'">
              {{ saveMessage }}
            </div>
          </div>
        </div>
      </div>
    </main>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import HeaderComponent from '../components/HeaderComponent.vue'
import { apiClient } from '../services/api'

// Reactive state
const loading = ref(false)
const saving = ref(false)
const error = ref('')
const saveMessage = ref('')
const saveSuccess = ref(false)
const apiBaseUrlValue = ref('')
const originalApiBaseUrl = ref('')

// Load settings from API
const loadSettings = async () => {
  loading.value = true
  error.value = ''

  try {
    const settings = await apiClient.getSettings()
    const apiBaseUrlSetting = settings.settings.find(s => s.key === 'api_base_url')

    if (apiBaseUrlSetting) {
      apiBaseUrlValue.value = apiBaseUrlSetting.value
      originalApiBaseUrl.value = apiBaseUrlSetting.value
    }
  } catch (err: any) {
    console.error('Failed to load settings:', err)
    error.value = err.message || 'Failed to load settings'
  } finally {
    loading.value = false
  }
}

// Save API Base URL setting
const saveApiBaseUrl = async () => {
  if (apiBaseUrlValue.value === originalApiBaseUrl.value) {
    return
  }

  saving.value = true
  saveMessage.value = ''

  try {
    await apiClient.updateSetting('api_base_url', apiBaseUrlValue.value)
    originalApiBaseUrl.value = apiBaseUrlValue.value
    saveMessage.value = 'API Base URL saved successfully'
    saveSuccess.value = true

    // Clear success message after 3 seconds
    setTimeout(() => {
      saveMessage.value = ''
    }, 3000)
  } catch (err: any) {
    console.error('Failed to save API Base URL:', err)
    saveMessage.value = err.message || 'Failed to save API Base URL'
    saveSuccess.value = false
  } finally {
    saving.value = false
  }
}

// Load settings on component mount
onMounted(() => {
  loadSettings()
})
</script>