export interface AIModel {
  id: string
  name: string
}

export interface AIProvider {
  name: string
  models: AIModel[]
}

export interface AIProviders {
  [key: string]: AIProvider
}

export const AI_PROVIDERS: AIProviders = {
  anthropic: {
    name: 'Anthropic Claude',
    models: [
      { id: 'claude-3-5-sonnet-20241022', name: 'Claude 3.5 Sonnet' },
      { id: 'claude-3-sonnet-20240229', name: 'Claude 3 Sonnet' },
      { id: 'claude-3-haiku-20240307', name: 'Claude 3 Haiku' }
    ]
  }
} as const

export const DEFAULT_AI_CONFIG = {
  provider: 'anthropic' as keyof typeof AI_PROVIDERS,
  model: 'claude-3-5-sonnet-20241022',
  maxTokens: 4000,
  temperature: 0.1,
  executionsLimit: 20
} as const

export interface AIGenerationRequest {
  system_prompt: string
  user_prompt: string
  model: string
  max_tokens: number
  temperature: number
}

export interface AIGenerationResponse {
  response: string
}