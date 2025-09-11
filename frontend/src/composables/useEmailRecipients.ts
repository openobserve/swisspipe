/**
 * Email recipients list composable - focused state management
 */

import { ref, watch, computed } from 'vue'
import type { EmailAddress } from '../types/nodes'
import { shallowArrayEqual, shallowEqual } from '../utils/comparison'
import { debugLog } from '../utils/debug'
import { safeSync } from '../utils/error-handling'

interface EmailRecipientsProps {
  modelValue: EmailAddress[]
  allowEmpty?: boolean
}

interface EmailRecipientsEmits {
  (e: 'update:modelValue', value: EmailAddress[]): void
}

export function useEmailRecipients(props: EmailRecipientsProps, emit: EmailRecipientsEmits) {
  // User interaction state
  const userIsTyping = ref(false)
  const showSuggestions = ref(false)

  // Template variables for suggestions
  const commonVariables = [
    { path: '{{ event.data.user_email }}', description: 'User email from workflow data' },
    { path: '{{ event.data.admin_email }}', description: 'Admin email from workflow data' },
    { path: '{{ event.data.email }}', description: 'Generic email from workflow data' },
    { path: '{{ event.data.user_name }}', description: 'User name from workflow data' },
    { path: '{{ event.metadata.created_by }}', description: 'Workflow creator' }
  ]

  // Local recipients with safe initialization
  const localRecipients = ref<EmailAddress[]>(
    safeSync(
      () => [...(props.modelValue || [])],
      'EmailRecipients initialization',
      []
    )
  )

  // Ensure at least one recipient if not allowing empty
  if (localRecipients.value.length === 0 && !props.allowEmpty) {
    localRecipients.value.push({ email: '', name: '' })
  }

  // Efficient prop changes watcher - no JSON.stringify!
  watch(
    () => props.modelValue,
    (newValue, oldValue) => {
      debugLog.component('EmailRecipientList', 'props changed', {
        hasActualChange: !shallowArrayEqual(newValue || [], oldValue || []),
        userIsTyping: userIsTyping.value
      })

      // Skip if no actual change or user is typing
      if (shallowArrayEqual(newValue || [], oldValue || []) || userIsTyping.value) {
        return
      }

      // Update local recipients safely
      localRecipients.value = safeSync(
        () => [...(newValue || [])],
        'EmailRecipients props sync',
        localRecipients.value
      )
    }
  )

  // Emit updates
  function emitUpdate() {
    debugLog.component('EmailRecipientList', 'emitting update')
    emit('update:modelValue', [...localRecipients.value])
  }

  // Input event handlers
  function handleInputStart(field: string, index: number, value?: any) {
    userIsTyping.value = true
    debugLog.interaction('EmailRecipientList', `input-start:${field}[${index}]`, value)
  }

  function handleInputEnd(field: string, index: number) {
    userIsTyping.value = false
    debugLog.interaction('EmailRecipientList', `input-end:${field}[${index}]`)
    emitUpdate()
  }

  // Specific field handlers
  function onEmailInputDebug(event: Event, index?: number) {
    const target = event.target as HTMLInputElement
    handleInputStart('email', index || 0, target.value)
  }

  function onNameInputDebug(event: Event, index?: number) {
    const target = event.target as HTMLInputElement
    handleInputStart('name', index || 0, target.value)
  }

  function onFieldBlur(event: FocusEvent, index?: number) {
    handleInputEnd('field', index || 0)
  }

  // Recipient management
  function addRecipient() {
    localRecipients.value.push({ email: '', name: '' })
    debugLog.component('EmailRecipientList', 'recipient added')
    emitUpdate()
  }

  function removeRecipient(index: number) {
    if (localRecipients.value.length > 1 || props.allowEmpty) {
      localRecipients.value.splice(index, 1)
      debugLog.component('EmailRecipientList', 'recipient removed', { index })
      emitUpdate()
    }
  }

  function insertVariable(variablePath: string) {
    if (localRecipients.value.length > 0) {
      const lastRecipient = localRecipients.value[localRecipients.value.length - 1]
      if (!lastRecipient.email) {
        lastRecipient.email = variablePath
      } else {
        // Add a new recipient with the variable
        localRecipients.value.push({ email: variablePath, name: '' })
      }
      debugLog.component('EmailRecipientList', 'variable inserted', { variablePath })
      emitUpdate()
    }
  }


  return {
    // State
    localRecipients,
    userIsTyping: computed(() => userIsTyping.value),
    showSuggestions,
    commonVariables,
    
    // Methods
    emitUpdate,
    onEmailInputDebug,
    onNameInputDebug,
    onFieldBlur,
    addRecipient,
    removeRecipient,
    insertVariable
  }
}