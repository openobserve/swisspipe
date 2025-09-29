/**
 * Error handling utilities for robust component behavior
 */

import type { EmailConfig } from '../types/nodes'
import { debugLog } from './debug'

export interface ValidationError {
  field: string
  message: string
  code: string
}

export interface ValidationResult {
  isValid: boolean
  errors: ValidationError[]
}

/**
 * Email configuration validation
 */
export function validateEmailConfig(config: Partial<EmailConfig>): ValidationResult {
  const errors: ValidationError[] = []

  // SMTP config and from email are now handled by environment variables
  // No validation needed for these fields

  // Validate to recipients
  if (!config.to || config.to.length === 0) {
    errors.push({
      field: 'to',
      message: 'At least one recipient is required',
      code: 'REQUIRED'
    })
  } else {
    config.to.forEach((recipient, index) => {
      if (!recipient.email) {
        errors.push({
          field: `to[${index}].email`,
          message: 'Recipient email address is required',
          code: 'REQUIRED'
        })
      } else if (!isValidEmail(recipient.email) && !isTemplateVariable(recipient.email)) {
        errors.push({
          field: `to[${index}].email`,
          message: 'Recipient email must be a valid email address or template variable',
          code: 'INVALID_FORMAT'
        })
      }
    })
  }

  // Validate CC recipients if provided
  if (config.cc) {
    config.cc.forEach((recipient, index) => {
      if (recipient.email && !isValidEmail(recipient.email) && !isTemplateVariable(recipient.email)) {
        errors.push({
          field: `cc[${index}].email`,
          message: 'CC email must be a valid email address or template variable',
          code: 'INVALID_FORMAT'
        })
      }
    })
  }

  // Validate BCC recipients if provided
  if (config.bcc) {
    config.bcc.forEach((recipient, index) => {
      if (recipient.email && !isValidEmail(recipient.email) && !isTemplateVariable(recipient.email)) {
        errors.push({
          field: `bcc[${index}].email`,
          message: 'BCC email must be a valid email address or template variable',
          code: 'INVALID_FORMAT'
        })
      }
    })
  }

  // Validate subject
  if (!config.subject?.trim()) {
    errors.push({
      field: 'subject',
      message: 'Email subject is required',
      code: 'REQUIRED'
    })
  }

  // Validate body template
  if (!config.body_template?.trim()) {
    errors.push({
      field: 'body_template',
      message: 'Email body template is required',
      code: 'REQUIRED'
    })
  }


  return {
    isValid: errors.length === 0,
    errors
  }
}

/**
 * Basic email validation
 */
function isValidEmail(email: string): boolean {
  const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/
  return emailRegex.test(email)
}

/**
 * Check if string is a template variable
 */
function isTemplateVariable(value: string): boolean {
  return /\{\{\s*[\w.]+\s*\}\}/.test(value)
}

/**
 * Safe async operation wrapper
 */
export async function safeAsync<T>(
  operation: () => Promise<T>,
  context: string
): Promise<{ success: true; data: T } | { success: false; error: string }> {
  try {
    const data = await operation()
    return { success: true, data }
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : 'Unknown error'
    debugLog.error(`Safe async operation failed: ${context}`, error)
    return { success: false, error: errorMessage }
  }
}

/**
 * Safe sync operation wrapper
 */
export function safeSync<T>(
  operation: () => T,
  context: string,
  fallback: T
): T {
  try {
    return operation()
  } catch (error) {
    debugLog.error(`Safe sync operation failed: ${context}`, error)
    return fallback
  }
}

/**
 * Component error boundary utility
 */
export function withErrorHandling<T extends (...args: unknown[]) => unknown>(
  fn: T,
  context: string,
  onError?: (error: Error) => void
): T {
  return ((...args: unknown[]) => {
    try {
      return fn(...args)
    } catch (error) {
      const err = error instanceof Error ? error : new Error('Unknown error')
      debugLog.error(`Error in ${context}`, err)
      onError?.(err)
      throw err
    }
  }) as T
}