<template>
  <div class="flex flex-col h-full bg-slate-900">
    <!-- Header -->
    <div class="glass-dark border-b border-slate-700/50 flex-shrink-0">
      <div class="px-6 py-4">
        <div class="flex items-center justify-between">
          <div>
            <h1 class="text-2xl font-bold text-white flex items-center">
              <svg class="h-8 w-8 text-primary-400 mr-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
              </svg>
              API Documentation
            </h1>
            <p class="text-sm text-gray-400 mt-1">Comprehensive SwissPipe API reference with interactive examples</p>
          </div>
          <div class="flex items-center space-x-3">
            <a
              :href="openApiSpecUrl"
              target="_blank"
              class="flex items-center space-x-2 bg-primary-600/20 hover:bg-primary-600/30 text-primary-300 hover:text-primary-200 border border-primary-600/30 hover:border-primary-600/50 px-4 py-2 rounded-md text-sm font-medium transition-all duration-200 backdrop-blur-sm"
            >
              <svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
              </svg>
              <span>Download OpenAPI Spec</span>
            </a>
            <button
              @click="reloadDocumentation"
              :disabled="isLoading"
              class="flex items-center space-x-2 bg-slate-600/20 hover:bg-slate-600/30 text-slate-300 hover:text-slate-200 border border-slate-600/30 hover:border-slate-600/50 px-4 py-2 rounded-md text-sm font-medium transition-all duration-200 backdrop-blur-sm disabled:opacity-50"
            >
              <svg class="h-4 w-4" :class="{ 'animate-spin': isLoading }" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
              </svg>
              <span>{{ isLoading ? 'Loading...' : 'Refresh' }}</span>
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Quick Navigation -->
    <div class="bg-slate-800/50 border-b border-slate-700/30 px-6 py-3 flex-shrink-0">
      <div class="flex items-center space-x-1 text-sm">
        <span class="text-gray-400">Quick links:</span>
        <button
          v-for="link in quickLinks"
          :key="link.id"
          @click="navigateToSection(link.hash)"
          class="px-3 py-1 text-gray-300 hover:text-primary-400 hover:bg-primary-900/20 rounded-md transition-colors"
        >
          {{ link.name }}
        </button>
      </div>
    </div>

    <!-- Documentation Content -->
    <div class="flex-1 overflow-hidden bg-white">
      <!-- Loading State -->
      <div v-show="isLoading" class="flex items-center justify-center h-full">
        <div class="text-center">
          <div class="inline-block animate-spin rounded-full h-12 w-12 border-b-2 border-primary-500"></div>
          <p class="mt-4 text-gray-600">Loading API documentation...</p>
        </div>
      </div>

      <!-- Error State -->
      <div v-show="hasError" class="flex items-center justify-center h-full">
        <div class="text-center max-w-md">
          <div class="text-red-500 text-6xl mb-4">❌</div>
          <h3 class="text-xl font-semibold text-gray-800 mb-2">Failed to Load Documentation</h3>
          <p class="text-gray-600 mb-4">{{ errorMessage }}</p>
          <button
            @click="reloadDocumentation"
            class="bg-primary-600 hover:bg-primary-700 text-white px-4 py-2 rounded-md transition-colors"
          >
            Try Again
          </button>
        </div>
      </div>

      <!-- Redoc Documentation -->
      <div
        id="redoc-container"
        class="h-full w-full overflow-y-auto bg-white"
        ref="redocContainer"
        v-show="!isLoading && !hasError"
      ></div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick } from 'vue'

// Reactive state
const isLoading = ref(true)
const hasError = ref(false)
const errorMessage = ref('')
const redocContainer = ref<HTMLElement | null>(null)

// Configuration
const openApiSpecUrl = '/openapi.yaml'
const quickLinks = [
  { id: 'health', name: '🏥 Health', hash: '#tag/Health' },
  { id: 'ingestion', name: '📥 Ingestion', hash: '#tag/Workflow-Ingestion' },
  { id: 'workflows', name: '🔄 Workflows', hash: '#tag/Admin---Workflows' },
  { id: 'executions', name: '📊 Executions', hash: '#tag/Admin---Executions' },
  { id: 'ai', name: '🤖 AI', hash: '#tag/Admin---AI' },
  { id: 'auth', name: '🔐 Auth', hash: '#tag/Authentication' },
]

// Load Redoc library dynamically with fallback URLs
function loadRedocScript(): Promise<void> {
  return new Promise((resolve, reject) => {
    // Check if Redoc is already loaded
    if ((window as any).Redoc) {
      resolve()
      return
    }

    // List of CDN URLs to try (in order of preference)
    const cdnUrls = [
      'https://cdn.redoc.ly/redoc/2.1.3/bundles/redoc.standalone.js',
      'https://cdn.jsdelivr.net/npm/redoc@2.1.3/bundles/redoc.standalone.js',
      'https://unpkg.com/redoc@2.1.3/bundles/redoc.standalone.js'
    ]

    let currentIndex = 0

    const tryLoadScript = () => {
      if (currentIndex >= cdnUrls.length) {
        reject(new Error('Failed to load Redoc library from all CDN sources. Please check your internet connection.'))
        return
      }

      const script = document.createElement('script')
      script.src = cdnUrls[currentIndex]

      // Set up a timeout for each script loading attempt
      const timeout = setTimeout(() => {
        script.onload = null
        script.onerror = null
        console.warn(`Timeout loading Redoc from ${cdnUrls[currentIndex]}, trying next CDN...`)
        currentIndex++
        tryLoadScript()
      }, 10000) // 10 second timeout per CDN

      script.onload = () => {
        clearTimeout(timeout)
        // Double-check that Redoc is available
        if ((window as any).Redoc) {
          resolve()
        } else {
          currentIndex++
          tryLoadScript()
        }
      }

      script.onerror = () => {
        clearTimeout(timeout)
        console.warn(`Failed to load Redoc from ${cdnUrls[currentIndex]}, trying next CDN...`)
        currentIndex++
        tryLoadScript()
      }

      // Add to document head
      document.head.appendChild(script)
    }

    tryLoadScript()
  })
}

// Initialize Redoc documentation
async function initializeRedoc() {
  try {
    isLoading.value = true
    hasError.value = false

    // Wait for next tick to ensure DOM is ready
    await nextTick()

    // Give the template a moment to render
    await new Promise(resolve => setTimeout(resolve, 100))

    // Ensure container exists
    if (!redocContainer.value) {
      console.error('Redoc container ref:', redocContainer.value)
      console.error('Available elements:', document.querySelector('#redoc-container'))
      throw new Error('Documentation container not found')
    }

    // Load Redoc library
    await loadRedocScript()

    // Initialize Redoc with default light theme
    const redocOptions = {
      scrollYOffset: 0,
      hideDownloadButton: false,
      disableSearch: false,
      expandResponses: '200,201',
      jsonSampleExpandLevel: 2,
      hideSingleRequestSampleTab: true,
      menuToggle: true,
      nativeScrollbars: false,
      pathInMiddlePanel: true,
      requiredPropsFirst: true,
      sortPropsAlphabetically: true,
      expandSingleSchemaField: true,
      showObjectSchemaExamples: true,
      showExtensions: true,
      hideSchemaPattern: true
    }

    // Initialize Redoc
    ;(window as any).Redoc.init(openApiSpecUrl, redocOptions, redocContainer.value)

    isLoading.value = false
  } catch (error) {
    console.error('Failed to initialize API documentation:', error)

    // If Redoc failed to load, try to show a simple fallback
    if (error instanceof Error && error.message.includes('Failed to load Redoc library')) {
      try {
        await showFallbackDocumentation()
        isLoading.value = false
        return
      } catch (fallbackError) {
        console.error('Fallback documentation also failed:', fallbackError)
      }
    }

    hasError.value = true
    isLoading.value = false
    errorMessage.value = error instanceof Error ? error.message : 'Unknown error occurred'
  }
}

// Show fallback documentation when Redoc fails to load
async function showFallbackDocumentation() {
  if (!redocContainer.value) {
    throw new Error('Container not available for fallback')
  }

  // Create fallback content
  redocContainer.value.innerHTML = `
    <div style="padding: 2rem; max-width: 800px; margin: 0 auto; font-family: system-ui, -apple-system, sans-serif;">
      <div style="background: #f8fafc; border: 1px solid #e2e8f0; border-radius: 8px; padding: 1.5rem; margin-bottom: 2rem;">
        <h2 style="color: #1e293b; margin: 0 0 1rem 0;">⚠️ Redoc Library Unavailable</h2>
        <p style="color: #475569; margin: 0;">The interactive documentation library couldn't be loaded. You can still access the raw OpenAPI specification:</p>
      </div>

      <div style="text-align: center; margin: 2rem 0;">
        <a href="${openApiSpecUrl}"
           target="_blank"
           style="display: inline-flex; items-center: gap: 0.5rem; background: #3b82f6; color: white; text-decoration: none; padding: 0.75rem 1.5rem; border-radius: 6px; font-weight: 500;">
          📄 View OpenAPI Specification
        </a>
      </div>

      <div style="background: white; border: 1px solid #e2e8f0; border-radius: 8px; padding: 1.5rem;">
        <h3 style="color: #1e293b; margin: 0 0 1rem 0;">SwissPipe API Overview</h3>
        <div style="color: #475569; line-height: 1.6;">
          <p><strong>Base URL:</strong> <code style="background: #f1f5f9; padding: 0.2rem 0.4rem; border-radius: 3px;">http://localhost:3700</code></p>

          <h4 style="color: #374151; margin: 1.5rem 0 0.5rem 0;">Main API Categories:</h4>
          <ul style="margin: 0; padding-left: 1.5rem;">
            <li><strong>Health Check:</strong> <code>/health</code></li>
            <li><strong>Workflow Ingestion:</strong> <code>/api/v1/{workflow_id}/trigger</code></li>
            <li><strong>Admin - Workflows:</strong> <code>/api/admin/v1/workflows</code></li>
            <li><strong>Admin - Executions:</strong> <code>/api/admin/v1/executions</code></li>
            <li><strong>Script Testing:</strong> <code>/api/admin/v1/script</code></li>
            <li><strong>AI Integration:</strong> <code>/api/admin/v1/ai</code></li>
            <li><strong>Settings:</strong> <code>/api/admin/v1/settings</code></li>
            <li><strong>Authentication:</strong> <code>/auth</code></li>
          </ul>

          <h4 style="color: #374151; margin: 1.5rem 0 0.5rem 0;">Authentication:</h4>
          <ul style="margin: 0; padding-left: 1.5rem;">
            <li><strong>Admin APIs:</strong> Basic Authentication (username/password)</li>
            <li><strong>Workflow Ingestion:</strong> UUID-based authentication</li>
            <li><strong>Google OAuth:</strong> Available for web interface</li>
          </ul>

          <div style="background: #fef3c7; border: 1px solid #fbbf24; border-radius: 6px; padding: 1rem; margin-top: 1.5rem;">
            <p style="margin: 0; color: #92400e;"><strong>💡 Tip:</strong> For the full interactive documentation experience, please ensure you have internet access to load the Redoc library, or try refreshing the page.</p>
          </div>
        </div>
      </div>
    </div>
  `
}

// Navigate to specific section in documentation
function navigateToSection(hash: string) {
  const element = document.querySelector(hash)
  if (element) {
    element.scrollIntoView({ behavior: 'smooth', block: 'start' })
  }
}

// Reload documentation
async function reloadDocumentation() {
  // Clear the container
  if (redocContainer.value) {
    redocContainer.value.innerHTML = ''
  }

  // Re-initialize
  await initializeRedoc()
}

// Lifecycle hooks
onMounted(async () => {
  await initializeRedoc()
})

onUnmounted(() => {
  // Clean up any event listeners or resources if needed
})
</script>

<style scoped>
/* Custom scrollbar for the documentation container */
#redoc-container ::-webkit-scrollbar {
  width: 8px;
}

#redoc-container ::-webkit-scrollbar-track {
  background: #f1f5f9;
}

#redoc-container ::-webkit-scrollbar-thumb {
  background: #cbd5e1;
  border-radius: 4px;
}

#redoc-container ::-webkit-scrollbar-thumb:hover {
  background: #94a3b8;
}

/* Ensure Redoc takes full height and allows scrolling */
:deep(.redoc-wrap) {
  height: 100% !important;
  overflow-y: auto !important;
}

/* Fix scrolling for main content area */
:deep(.redoc-wrap > div) {
  height: 100% !important;
  overflow-y: auto !important;
}

:deep([role="main"]) {
  height: auto !important;
  max-height: none !important;
  overflow-y: auto !important;
}

/* Ensure content panels can scroll */
:deep(.content-panel) {
  overflow-y: auto !important;
  max-height: none !important;
}

:deep(.api-content) {
  overflow-y: auto !important;
  max-height: none !important;
}

/* Additional scrolling fixes for Redoc specific classes */
:deep(.menu-content) {
  overflow-y: auto !important;
}

:deep(.redoc-summary) {
  overflow-y: visible !important;
}

/* Fix for three-panel layout scrolling */
:deep(.redoc-wrap .api-content) {
  overflow-y: auto !important;
  height: 100vh !important;
}

:deep(.redoc-wrap .menu-content) {
  overflow-y: auto !important;
  height: 100vh !important;
}

/* Basic menu styling for better integration */
:deep(.menu-content .menu-item-label) {
  font-size: 14px !important;
}

/* Hide the Redoc title since we have our own */
:deep(.api-info .api-info-title) {
  display: none !important;
}
</style>