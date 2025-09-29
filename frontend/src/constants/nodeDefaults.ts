export const DEFAULT_RETRY_CONFIG = {
  max_attempts: 3,
  initial_delay_ms: 100,
  max_delay_ms: 5000,
  backoff_multiplier: 2.0
}

export const DEFAULT_EMAIL_CONFIG = {
  to: [{
    email: '{{ event.data.user_email }}',
    name: '{{ event.data.user_name }}'
  }],
  cc: [],
  bcc: [],
  subject: 'Workflow {{ event.name }} completed',
  template_type: 'html' as const,
  body_template: '<!DOCTYPE html><html><body><h1>Workflow Results</h1><p>Status: {{ event.status }}</p><p>Data: {{ event.data  }}</p></body></html>',
  text_body_template: undefined,
  attachments: []
}

export const DEFAULT_HTTP_CONFIG = {
  url: 'https://httpbin.org/post',
  method: 'POST' as const,
  timeout_seconds: 30,
  failure_action: 'Stop' as const,
  headers: {},
  retry_config: DEFAULT_RETRY_CONFIG,
  loop_config: undefined
}

export const DEFAULT_OPENOBSERVE_CONFIG = {
  url: '',
  authorization_header: '',
  timeout_seconds: 30,
  failure_action: 'Stop' as const,
  retry_config: DEFAULT_RETRY_CONFIG
}

export const DEFAULT_DELAY_CONFIG = {
  duration: 5,
  unit: 'Seconds' as const
}

export const DEFAULT_ANTHROPIC_CONFIG = {
  model: 'claude-sonnet-4-5-20250929',
  max_tokens: 8192,
  temperature: 0.7,
  system_prompt: '',
  user_prompt: 'Analyze this data: {{ event.data }}',
  timeout_seconds: 60,
  failure_action: 'Stop' as const,
  retry_config: DEFAULT_RETRY_CONFIG
}

export const DEFAULT_HUMAN_IN_LOOP_CONFIG = {
  type: 'human-in-loop' as const,
  title: 'Human Review Required',
  description: 'Please review this workflow event and make a decision',
  timeout_seconds: undefined,
  timeout_action: 'denied',
  required_fields: [],
  metadata: {}
}

export const NODE_TYPE_DESCRIPTIONS = {
  Trigger: 'HTTP endpoint trigger',
  Condition: 'Conditional logic node',
  Transformer: 'Data transformation node',
  HttpRequest: 'HTTP request',
  OpenObserve: 'OpenObserve log analytics',
  Email: 'Email notification node',
  Delay: 'Workflow execution delay',
  Anthropic: 'Anthropic LLM integration',
  HumanInLoop: 'Human intervention point',
  App: 'External application node'
}

export const NODE_TYPE_MAPPINGS = {
  trigger: 'Trigger',
  condition: 'Condition',
  transformer: 'Transformer',
  'http-request': 'HttpRequest',
  openobserve: 'OpenObserve',
  email: 'Email',
  delay: 'Delay',
  anthropic: 'Anthropic',
  'human-in-loop': 'HumanInLoop',
  app: 'App'
}

// Node Library UI Definitions
export const NODE_LIBRARY_DEFINITIONS = {
  trigger: {
    label: 'Trigger',
    description: 'Input data from configured sources',
    color: '#3b82f6',
    icon: 'play'
  },
  condition: {
    label: 'Condition', 
    description: 'Branch workflow based on conditions',
    color: '#f59e0b',
    icon: 'question-mark-circle'
  },
  transformer: {
    label: 'Transformer',
    description: 'Process and modify data', 
    color: '#8b5cf6',
    icon: 'arrow-path'
  },
  'http-request': {
    label: 'HTTP Request',
    description: 'Send HTTP requests to external APIs',
    color: '#10b981', 
    icon: 'globe-alt'
  },
  openobserve: {
    label: 'OpenObserve',
    description: 'Send logs and data to OpenObserve platform',
    color: '#f97316',
    icon: 'chart-bar'
  },
  email: {
    label: 'Email',
    description: 'Send email notifications and reports',
    color: '#2196F3',
    icon: 'envelope'
  },
  delay: {
    label: 'Delay',
    description: 'Pause workflow execution for a specified duration',
    color: '#6b7280',
    icon: 'clock'
  },
  anthropic: {
    label: 'Anthropic',
    description: 'Generate content using Anthropic LLM models',
    color: '#d97706',
    icon: 'sparkles'
  },
  'human-in-loop': {
    label: 'Human in Loop',
    description: 'Pause workflow for human review and decision',
    color: '#dc2626',
    icon: 'user'
  }
}