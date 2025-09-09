<template>
  <div class="email-template-variables">
    <div class="mb-3">
      <input
        v-model="searchQuery"
        placeholder="Search variables..."
        class="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded-md text-white text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
      />
    </div>

    <div class="space-y-3 max-h-64 overflow-y-auto">
      <div v-for="category in filteredCategories" :key="category.name">
        <h4 class="text-sm font-medium text-gray-300 mb-2">{{ category.name }}</h4>
        <div class="grid grid-cols-1 gap-1">
          <button
            v-for="variable in category.variables"
            :key="variable.path"
            @click="copyVariable(variable.path)"
            class="group flex items-start justify-between p-2 bg-gray-700 hover:bg-gray-600 rounded text-left transition-colors"
            :title="`Click to copy: ${variable.path}`"
          >
            <div class="flex-1 min-w-0">
              <div class="font-mono text-sm text-white">{{ variable.path }}</div>
              <div class="text-xs text-gray-400 mt-1">{{ variable.description }}</div>
              <div v-if="variable.example" class="text-xs text-green-400 mt-1">
                Example: {{ variable.example }}
              </div>
            </div>
            <div class="ml-2 opacity-0 group-hover:opacity-100 transition-opacity">
              <svg class="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
              </svg>
            </div>
          </button>
        </div>
      </div>
    </div>

    <!-- Copy notification -->
    <div 
      v-if="showCopyNotification"
      class="fixed top-4 right-4 bg-green-600 text-white px-4 py-2 rounded-md shadow-lg z-50"
    >
      Variable copied to clipboard!
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'

interface TemplateVariable {
  path: string
  description: string
  example?: string
}

interface VariableCategory {
  name: string
  variables: TemplateVariable[]
}

const searchQuery = ref('')
const showCopyNotification = ref(false)

const variableCategories: VariableCategory[] = [
  {
    name: 'Workflow Data',
    variables: [
      { 
        path: '{{ event.data }}', 
        description: 'Raw workflow data object', 
        example: '{"user_id": 123}' 
      },
      { 
        path: '{{ event.data  }}', 
        description: 'Formatted JSON data', 
        example: '{\n  "user_id": 123\n}' 
      },
      { 
        path: '{{ event.data.user_email }}', 
        description: 'User email from data', 
        example: 'user@example.com' 
      },
      { 
        path: '{{ event.data.user_name }}', 
        description: 'User name from data', 
        example: 'John Doe' 
      }
    ]
  },
  {
    name: 'Workflow Metadata',
    variables: [
      { 
        path: '{{ event.metadata }}', 
        description: 'Workflow metadata object' 
      },
      { 
        path: '{{ event.metadata.created_by }}', 
        description: 'Who created the workflow', 
        example: 'admin@company.com' 
      },
      { 
        path: '{{ event.headers }}', 
        description: 'HTTP headers from trigger' 
      },
      { 
        path: '{{ event.condition_results }}', 
        description: 'Results from condition nodes' 
      }
    ]
  },
  {
    name: 'Node Information',
    variables: [
      { 
        path: '{{ node.id }}', 
        description: 'Current email node ID', 
        example: 'email-node-001' 
      }
    ]
  },
  {
    name: 'System Information',
    variables: [
      { 
        path: '{{ system.timestamp }}', 
        description: 'Current timestamp in ISO format', 
        example: '2025-01-15T10:30:00Z' 
      },
      { 
        path: '{{ system.hostname }}', 
        description: 'Server hostname', 
        example: 'swisspipe-prod-1' 
      }
    ]
  },
  {
    name: 'Template Helpers',
    variables: [
      { 
        path: '{{ json variable  }}', 
        description: 'Format any variable as pretty JSON' 
      },
      { 
        path: '{{ timestamp | date_format: "%Y-%m-%d" }}', 
        description: 'Format date with custom format', 
        example: '2025-01-15' 
      },
      { 
        path: '{{ text | upper }}', 
        description: 'Convert text to uppercase', 
        example: 'HELLO WORLD' 
      },
      { 
        path: '{{ text | lower }}', 
        description: 'Convert text to lowercase', 
        example: 'hello world' 
      },
      { 
        path: '{{ html | escape_html }}', 
        description: 'Escape HTML characters for safety' 
      }
    ]
  }
]

const filteredCategories = computed(() => {
  if (!searchQuery.value.trim()) {
    return variableCategories
  }

  const query = searchQuery.value.toLowerCase()
  return variableCategories
    .map(category => ({
      ...category,
      variables: category.variables.filter(variable => 
        variable.path.toLowerCase().includes(query) ||
        variable.description.toLowerCase().includes(query) ||
        variable.example?.toLowerCase().includes(query)
      )
    }))
    .filter(category => category.variables.length > 0)
})

async function copyVariable(variablePath: string) {
  try {
    await navigator.clipboard.writeText(variablePath)
    showCopyNotification.value = true
    setTimeout(() => {
      showCopyNotification.value = false
    }, 2000)
  } catch (err) {
    console.warn('Failed to copy to clipboard:', err)
    // Fallback: try to select the text
    const textArea = document.createElement('textarea')
    textArea.value = variablePath
    document.body.appendChild(textArea)
    textArea.select()
    try {
      document.execCommand('copy')
      showCopyNotification.value = true
      setTimeout(() => {
        showCopyNotification.value = false
      }, 2000)
    } catch (fallbackErr) {
      console.error('Fallback copy failed:', fallbackErr)
    }
    document.body.removeChild(textArea)
  }
}
</script>

<style scoped>
.email-template-variables {
  @apply space-y-2;
}

.email-template-variables h4 {
  @apply text-sm font-medium text-gray-300 border-b border-gray-700 pb-1;
}
</style>