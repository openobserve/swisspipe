import './assets/main.css'

import { createApp } from 'vue'
import { createPinia } from 'pinia'

import App from './App.vue'
import router from './router'

// Configure Monaco Editor to disable web workers cleanly
import type { Environment } from 'monaco-editor'

declare global {
  interface Window {
    MonacoEnvironment?: Environment;
  }
}

// Create a silent no-op environment that prevents warnings
window.MonacoEnvironment = {
  getWorker: () => {
    // Create a minimal web worker that does nothing but satisfies Monaco's requirements
    const workerCode = `
      self.onmessage = function() {
        // Minimal worker that just acknowledges messages
      };
    `;
    const blob = new Blob([workerCode], { type: 'application/javascript' });
    return new Worker(URL.createObjectURL(blob));
  }
};

const app = createApp(App)

// Global error handler to catch and log all Vue errors
app.config.errorHandler = (error, instance, info) => {
  console.error('Vue Global Error Handler caught:', {
    error: error,
    message: error instanceof Error ? error.message : String(error),
    stack: error instanceof Error ? error.stack : undefined,
    instance: instance,
    info: info
  })

  // Check specifically for toUrl errors
  if (error instanceof Error && error.message.includes('toUrl')) {
    console.error('DETECTED toUrl ERROR:', {
      fullMessage: error.message,
      stack: error.stack,
      componentInfo: info
    })
  }
}

// Also catch unhandled promise rejections
window.addEventListener('unhandledrejection', (event) => {
  console.error('Unhandled Promise Rejection:', event.reason)
  if (event.reason instanceof Error && event.reason.message.includes('toUrl')) {
    console.error('DETECTED toUrl ERROR in Promise:', event.reason)
  }
})

app.use(createPinia())
app.use(router)

app.mount('#app')
