import { fileURLToPath, URL } from 'node:url'

import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import vueDevTools from 'vite-plugin-vue-devtools'
import monacoEditorPlugin from 'vite-plugin-monaco-editor'

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    vue(),
    vueDevTools(),
    (monacoEditorPlugin as { default: (options: Record<string, unknown>) => unknown }).default({
      languages: ['javascript', 'typescript', 'json'],
      features: ['!gotoSymbol'] // Disable features that might cause issues
    })
  ],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url))
    },
  },
  server: {
    hmr: {
      overlay: true,
    },
    watch: {
      usePolling: false,
    },
  },
  define: {
    'process.env': {}
  },
  worker: {
    format: 'es'
  }
})
