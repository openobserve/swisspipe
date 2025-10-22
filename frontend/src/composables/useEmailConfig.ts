/**
 * Email configuration composable - centralized state management and business logic
 */

import { ref, watch, computed, type Ref } from 'vue'
import type { EmailConfig } from '../types/nodes'
import { emailConfigEqual, deepClone } from '../utils/comparison'
import { debugLog } from '../utils/debug'
import { validateEmailConfig, safeSync, type ValidationResult } from '../utils/error-handling'

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
    localConfig.value.template_type
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

  // Watch for input field changes (subject, body templates)
  watch(
    () => [
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

  // Initialize validation
  validateConfig()

  // Default settings applied via environment variables on backend
  // onMounted(() => {
  //   applyDefaultSettings()
  // })

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