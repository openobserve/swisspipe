/**
 * Email configuration composable - centralized state management and business logic
 */

import { ref, watch, computed, onMounted, type Ref } from 'vue'
import type { EmailConfig } from '../types/nodes'
import { emailConfigEqual, deepClone } from '../utils/comparison'
import { debugLog } from '../utils/debug'
import { validateEmailConfig, safeSync, type ValidationResult } from '../utils/error-handling'
import { apiClient } from '../services/api'

interface EmailConfigProps {
  modelValue: EmailConfig
}

interface EmailConfigEmits {
  (e: 'update:modelValue', value: EmailConfig): void
}

export function useEmailConfig(props: EmailConfigProps, emit: EmailConfigEmits) {
  // Validation state
  const validationErrors = ref<ValidationResult>({ isValid: true, errors: [] })

  // Local config with safe initialization using deep clone for data isolation
  const localConfig = ref<EmailConfig>(safeSync(
    () => {
      const cloned = deepClone(props.modelValue)
      // Ensure cc and bcc are always arrays after cloning
      return {
        ...cloned,
        cc: cloned.cc || [],
        bcc: cloned.bcc || []
      }
    },
    'EmailConfig initialization',
    {
      ...deepClone(props.modelValue),
      cc: [],
      bcc: [],
      to: props.modelValue.to || []
    }
  ))

  // Computed validation state
  const isValid = computed(() => validationErrors.value.isValid)
  const hasErrors = computed(() => validationErrors.value.errors.length > 0)
  
  // Computed properties to ensure recipient arrays are always defined and trigger updates
  const toRecipients = computed({
    get: () => localConfig.value.to || [],
    set: (value) => { localConfig.value.to = value }
  })
  
  const ccRecipients = computed({
    get: () => localConfig.value.cc || [],
    set: (value) => { localConfig.value.cc = value }
  })
  
  const bccRecipients = computed({
    get: () => localConfig.value.bcc || [],
    set: (value) => { localConfig.value.bcc = value }
  })

  // Efficient prop changes watcher - no JSON.stringify!
  watch(
    () => props.modelValue,
    (newValue, oldValue) => {
      debugLog.component('EmailConfig', 'props changed', {
        hasActualChange: !emailConfigEqual(newValue, oldValue)
      })

      // Skip if no actual change
      if (emailConfigEqual(newValue, oldValue)) {
        return
      }

      // Update local config safely with deep clone for data isolation
      localConfig.value = safeSync(
        () => {
          const cloned = deepClone(newValue)
          return {
            ...cloned,
            cc: cloned.cc || [],
            bcc: cloned.bcc || []
          }
        },
        'EmailConfig props sync',
        localConfig.value
      )

      // Validate the new config
      validateConfig()
    }
  )

  // Validate configuration
  function validateConfig() {
    validationErrors.value = safeSync(
      () => validateEmailConfig(localConfig.value),
      'EmailConfig validation',
      { isValid: true, errors: [] }
    )
    
    debugLog.component('EmailConfig', 'validation complete', {
      isValid: validationErrors.value.isValid,
      errorCount: validationErrors.value.errors.length
    })
  }

  // Emit updates with validation
  function emitUpdate() {
    debugLog.component('EmailConfig', 'emitting update')
    
    // Validate before emitting
    validateConfig()
    
    if (isValid.value) {
      emit('update:modelValue', { ...localConfig.value })
    } else {
      debugLog.component('EmailConfig', 'update blocked due to validation errors', validationErrors.value.errors)
    }
  }


  // Watch for non-input changes (dropdowns, checkboxes) using efficient comparison
  const nonInputFields = computed(() => [
    localConfig.value.smtp_config,
    localConfig.value.template_type,
    localConfig.value.priority,
    localConfig.value.max_queue_wait_minutes,
    localConfig.value.queue_if_rate_limited,
    localConfig.value.delivery_receipt,
    localConfig.value.read_receipt
  ])

  watch(
    nonInputFields,
    (newValues, oldValues) => {
      if (oldValues && !arraysEqual(newValues, oldValues)) {
        debugLog.component('EmailConfig', 'non-input field changed')
        emitUpdate()
      }
    }
  )

  // Watch for recipient list changes - simplified approach
  watch(
    () => [localConfig.value.to, localConfig.value.cc, localConfig.value.bcc],
    () => {
      debugLog.component('EmailConfig', 'recipient lists changed')
      emitUpdate()
    },
    { deep: true }
  )

  // Watch for input field changes (from, subject, body templates)
  watch(
    () => [
      localConfig.value.from,
      localConfig.value.subject,
      localConfig.value.body_template,
      localConfig.value.text_body_template
    ],
    () => {
      debugLog.component('EmailConfig', 'input fields changed')
      emitUpdate()
    },
    { deep: true }
  )

  // Utility function for array comparison
  function arraysEqual<T>(a: T[], b: T[]): boolean {
    return a.length === b.length && a.every((val, i) => val === b[i])
  }

  // Function to apply default settings from database
  async function applyDefaultSettings() {
    try {
      const { defaultFromEmail, defaultFromName } = await apiClient.getDefaultEmailSettings()

      // Only apply defaults if current values are empty or match the hardcoded defaults
      const shouldApplyEmailDefault = !localConfig.value.from.email ||
        localConfig.value.from.email === 'noreply@company.com' ||
        localConfig.value.from.email === ''

      const shouldApplyNameDefault = !localConfig.value.from.name ||
        localConfig.value.from.name === 'SwissPipe Workflow' ||
        localConfig.value.from.name === ''

      if (shouldApplyEmailDefault && defaultFromEmail) {
        debugLog.component('EmailConfig', 'Applying default from email', { defaultFromEmail })
        localConfig.value.from.email = defaultFromEmail
      }

      if (shouldApplyNameDefault && defaultFromName) {
        debugLog.component('EmailConfig', 'Applying default from name', { defaultFromName })
        localConfig.value.from.name = defaultFromName
      }

      // Emit update if any defaults were applied
      if ((shouldApplyEmailDefault && defaultFromEmail) || (shouldApplyNameDefault && defaultFromName)) {
        emitUpdate()
      }
    } catch (error) {
      debugLog.component('EmailConfig', 'Failed to fetch default settings', { error })
      // Continue without defaults if fetching fails
    }
  }

  // Initialize validation
  validateConfig()

  // Apply default settings on mount
  onMounted(() => {
    applyDefaultSettings()
  })

  return {
    // State
    localConfig,
    toRecipients,
    ccRecipients,
    bccRecipients,
    validationErrors: readonly(validationErrors),
    isValid,
    hasErrors,
    
    // Methods
    emitUpdate,
    validateConfig
  }
}

// Helper to make refs readonly
function readonly<T>(ref: Ref<T>) {
  return computed(() => ref.value)
}