<template>
  <div class="space-y-6">
    <!-- Title Field -->
    <div>
      <label class="block text-sm font-medium text-gray-300 mb-2">
        Title *
      </label>
      <input
        v-model="localConfig.title"
        type="text"
        class="w-full px-3 py-2 bg-slate-700 border border-slate-600 text-gray-100 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
        placeholder="Enter a descriptive title for the human review task"
        @input="handleConfigChange"
      />
      <p class="text-xs text-gray-400 mt-1">
        Brief title that describes what needs human review
      </p>
    </div>


    <!-- Description Field -->
    <div>
      <label class="block text-sm font-medium text-gray-300 mb-2">
        Description
      </label>
      <textarea
        v-model="localConfig.description"
        rows="3"
        class="w-full px-3 py-2 bg-slate-700 border border-slate-600 text-gray-100 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
        placeholder="Provide detailed instructions for the human reviewer"
        @input="handleConfigChange"
      />
      <p class="text-xs text-gray-400 mt-1">
        Detailed instructions for the human reviewer
      </p>
    </div>

    <!-- Timeout Configuration -->
    <div class="grid grid-cols-2 gap-4">
      <div>
        <label class="block text-sm font-medium text-gray-300 mb-2">
          Timeout (seconds)
        </label>
        <input
          v-model.number="localConfig.timeout_seconds"
          type="number"
          min="1"
          class="w-full px-3 py-2 bg-slate-700 border border-slate-600 text-gray-100 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
          placeholder="e.g., 3600 (1 hour)"
          @input="handleConfigChange"
        />
        <p class="text-xs text-gray-400 mt-1">
          Leave empty for no timeout
        </p>
      </div>

      <div>
        <label class="block text-sm font-medium text-gray-300 mb-2">
          Timeout Action
        </label>
        <select
          v-model="localConfig.timeout_action"
          class="w-full px-3 py-2 bg-slate-700 border border-slate-600 text-gray-100 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
          @change="handleConfigChange"
        >
          <option value="denied">Deny (Default)</option>
          <option value="approved">Approve</option>
        </select>
        <p class="text-xs text-gray-400 mt-1">
          Action to take when timeout is reached
        </p>
      </div>
    </div>

    <!-- Required Fields -->
    <div>
      <label class="block text-sm font-medium text-gray-300 mb-2">
        Required Fields
      </label>
      <div class="space-y-2">
        <div
          v-for="(field, index) in (localConfig.required_fields || [])"
          :key="index"
          class="flex items-center space-x-2"
        >
          <input
            v-model="(localConfig.required_fields || [])[index]"
            type="text"
            class="flex-1 px-3 py-2 bg-slate-700 border border-slate-600 text-gray-100 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500"
            placeholder="Field name"
            @input="handleConfigChange"
          />
          <button
            @click="removeRequiredField(index)"
            class="px-3 py-2 text-red-400 hover:text-red-300 transition-colors"
          >
            Remove
          </button>
        </div>
        <button
          @click="addRequiredField"
          class="px-3 py-2 text-blue-400 hover:text-blue-300 text-sm transition-colors"
        >
          + Add Required Field
        </button>
      </div>
      <p class="text-xs text-gray-400 mt-1">
        Fields that must be provided in the human response
      </p>
    </div>

    <!-- Metadata -->
    <div>
      <label class="block text-sm font-medium text-gray-300 mb-2">
        Metadata (JSON)
      </label>
      <textarea
        v-model="metadataJson"
        rows="4"
        class="w-full px-3 py-2 bg-slate-700 border border-slate-600 text-gray-100 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500 font-mono text-sm"
        placeholder='{"priority": "high", "department": "finance"}'
        @input="handleMetadataChange"
      />
      <p class="text-xs text-gray-400 mt-1">
        Additional metadata to include with the HIL task (JSON format)
      </p>
      <p v-if="metadataError" class="text-xs text-red-400 mt-1">
        {{ metadataError }}
      </p>
    </div>

    <!-- Validation Errors -->
    <div v-if="validationErrors.length > 0" class="bg-red-900/20 border border-red-500/30 rounded-md p-3">
      <h4 class="text-sm font-medium text-red-300 mb-2">Configuration Issues:</h4>
      <ul class="text-sm text-red-200 space-y-1">
        <li v-for="error in validationErrors" :key="error" class="flex items-center">
          <span class="w-1 h-1 bg-red-400 rounded-full mr-2"></span>
          {{ error }}
        </li>
      </ul>
    </div>

    <!-- Help Text -->
    <div class="bg-blue-900/20 border border-blue-500/30 rounded-md p-3">
      <h4 class="text-sm font-medium text-blue-300 mb-2">How it works:</h4>
      <ul class="text-sm text-blue-200 space-y-1">
        <li>• The workflow will pause at this node until a human makes a decision</li>
        <li>• A notification will be sent via the designated notification system</li>
        <li>• Humans can respond via the provided webhook URL with "approved" or "denied"</li>
        <li>• The workflow will resume with the human decision available in event data</li>
      </ul>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch, onMounted } from 'vue'
import type { HumanInLoopConfig } from '@/types/nodes'
import { DEFAULT_HUMAN_IN_LOOP_CONFIG } from '@/constants/nodeDefaults'
import { useWorkflowStore } from '@/stores/workflows'
import { deepClone } from '@/utils/comparison'

interface Props {
  config: HumanInLoopConfig
}

interface Emits {
  (e: 'update:config', config: HumanInLoopConfig): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

const workflowStore = useWorkflowStore()
const workflow = computed(() => workflowStore.currentWorkflow)

// Local config to prevent direct mutations
const localConfig = ref<HumanInLoopConfig>(deepClone(props.config))
const metadataJson = ref<string>('')
const metadataError = ref<string>('')

// Validation
const validationErrors = computed(() => {
  const errors: string[] = []

  if (!localConfig.value.title.trim()) {
    errors.push('Title is required')
  }


  if (localConfig.value.timeout_seconds && localConfig.value.timeout_seconds <= 0) {
    errors.push('Timeout must be greater than 0 seconds')
  }

  return errors
})


// Initialize metadata JSON display
onMounted(() => {
  if (localConfig.value.metadata && Object.keys(localConfig.value.metadata).length > 0) {
    try {
      metadataJson.value = JSON.stringify(localConfig.value.metadata, null, 2)
    } catch {
      metadataJson.value = '{}'
    }
  } else {
    metadataJson.value = '{}'
  }
})

// Handle config changes
const handleConfigChange = () => {
  emit('update:config', deepClone(localConfig.value))
}

// Handle metadata JSON changes
const handleMetadataChange = () => {
  try {
    const parsed = JSON.parse(metadataJson.value || '{}')
    localConfig.value.metadata = parsed
    metadataError.value = ''
    handleConfigChange()
  } catch {
    metadataError.value = 'Invalid JSON format'
  }
}

// Required fields management
const addRequiredField = () => {
  localConfig.value.required_fields = [...(localConfig.value.required_fields || []), '']
  handleConfigChange()
}

const removeRequiredField = (index: number) => {
  localConfig.value.required_fields = localConfig.value.required_fields?.filter((_, i) => i !== index) || []
  handleConfigChange()
}

// Watch for external config changes
watch(() => props.config, (newConfig) => {
  localConfig.value = deepClone(newConfig)

  // Update metadata JSON display
  if (newConfig.metadata && Object.keys(newConfig.metadata).length > 0) {
    try {
      metadataJson.value = JSON.stringify(newConfig.metadata, null, 2)
    } catch {
      metadataJson.value = '{}'
    }
  } else {
    metadataJson.value = '{}'
  }
}, { deep: true })

// Initialize with defaults if config is empty
if (!props.config.title) {
  localConfig.value = deepClone(DEFAULT_HUMAN_IN_LOOP_CONFIG)
  handleConfigChange()
}
</script>