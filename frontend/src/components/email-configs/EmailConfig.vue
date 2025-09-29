<template>
  <div class="email-config p-4 space-y-6">
    <!-- Email Recipients -->
    <div class="config-section">
      <h3 class="text-lg font-semibold text-white mb-4">Recipients</h3>

      <!-- Recipients List -->
      <div class="space-y-4">

        <!-- To Recipients -->
        <div>
          <label class="block text-sm font-medium text-gray-300 mb-2">To Recipients</label>
          <EmailRecipientList
            v-model="toRecipients"
            :allow-empty="false"
          />
        </div>

        <!-- CC Recipients -->
        <div>
          <label class="block text-sm font-medium text-gray-300 mb-2">CC Recipients (Optional)</label>
          <EmailRecipientList v-model="ccRecipients" />
        </div>

        <!-- BCC Recipients -->
        <div>
          <label class="block text-sm font-medium text-gray-300 mb-2">BCC Recipients (Optional)</label>
          <EmailRecipientList v-model="bccRecipients" />
        </div>
      </div>
    </div>

    <!-- Email Content -->
    <div class="config-section">
      <h3 class="text-lg font-semibold text-white mb-4">Email Content</h3>
      
      <div class="space-y-4">
        <!-- Subject -->
        <div>
          <label class="block text-sm font-medium text-gray-300 mb-2">Subject</label>
          <input
            v-model="localConfig.subject"
            placeholder="Email subject with {{ event.name }} variables"
            class="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded-md text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
        </div>

        <!-- Template Type -->
        <div>
          <label class="block text-sm font-medium text-gray-300 mb-2">Content Type</label>
          <div class="flex space-x-4">
            <label class="flex items-center">
              <input
                v-model="localConfig.template_type"
                type="radio"
                value="html"
                class="mr-2 text-blue-600"
              />
              <span class="text-white">HTML</span>
            </label>
            <label class="flex items-center">
              <input
                v-model="localConfig.template_type"
                type="radio"
                value="text"
                class="mr-2 text-blue-600"
              />
              <span class="text-white">Plain Text</span>
            </label>
          </div>
        </div>

        <!-- Body Template -->
        <div>
          <label class="block text-sm font-medium text-gray-300 mb-2">
            {{ localConfig.template_type === 'html' ? 'HTML' : 'Text' }} Body Template
          </label>
          <EmailContentEditor
            v-model="localConfig.body_template"
            :content-type="localConfig.template_type"
            :height="300"
          />
        </div>

        <!-- Text Fallback (for HTML emails) -->
        <div v-if="localConfig.template_type === 'html'">
          <label class="block text-sm font-medium text-gray-300 mb-2">
            Text Fallback (Optional)
          </label>
          <textarea
            v-model="localConfig.text_body_template"
            placeholder="Plain text version for email clients that don't support HTML"
            rows="4"
            class="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded-md text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
        </div>
      </div>
    </div>

  </div>
</template>

<script setup lang="ts">
import type { EmailConfig } from '../../types/nodes'
import { useEmailConfig } from '../../composables/useEmailConfig'
import EmailRecipientList from './EmailRecipientList.vue'
import EmailContentEditor from './EmailContentEditor.vue'

interface Props {
  modelValue: EmailConfig
}

interface Emits {
  (e: 'update:modelValue', value: EmailConfig): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

// Use the email config composable for all business logic
const {
  localConfig,
  toRecipients,
  ccRecipients,
  bccRecipients
} = useEmailConfig(props, emit)
</script>

<style scoped>
.config-section {
  @apply bg-gray-800 rounded-lg p-4 border border-gray-700;
}

.config-section h3 {
  @apply border-b border-gray-700 pb-2 mb-4;
}
</style>