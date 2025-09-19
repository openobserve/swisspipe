export interface APIErrorResponse {
  response?: {
    status?: number
    data?: {
      error?: string
      message?: string
    }
  }
  message?: string
  code?: string
}

export function isAPIError(error: unknown): error is APIErrorResponse {
  return typeof error === 'object' && error !== null && 'response' in error
}

export function getErrorMessage(error: unknown): string {
  if (isAPIError(error)) {
    return error.response?.data?.error ||
           error.response?.data?.message ||
           error.message ||
           'Unknown API error'
  }

  if (error instanceof Error) {
    return error.message
  }

  return String(error)
}

export function getUserFriendlyErrorMessage(error: unknown): string {
  const message = getErrorMessage(error)

  if (message.includes('fetch') || message.includes('network')) {
    return 'Please check your network connection and try again.'
  }

  if (message.includes('ANTHROPIC_API_KEY') || message.includes('401')) {
    return 'AI service is not properly configured. Please contact your administrator.'
  }

  if (message.includes('429')) {
    return 'Too many requests. Please wait a moment and try again.'
  }

  return `${message}. Please try rephrasing your request.`
}