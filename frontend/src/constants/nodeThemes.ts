export interface NodeTheme {
  color: string
  background: string
  backgroundHover: string
  border: string
  boxShadow: string
  boxShadowHover: string
  borderDefault: string
}

export const NODE_THEMES = {
  trigger: {
    color: 'blue',
    background: 'rgba(59, 130, 246, 0.12)',
    backgroundHover: 'rgba(59, 130, 246, 0.18)',
    border: 'rgba(59, 130, 246, 0.25)',
    boxShadow: '0 8px 32px rgba(59, 130, 246, 0.15), inset 0 1px 0 rgba(255, 255, 255, 0.1)',
    boxShadowHover: '0 12px 40px rgba(59, 130, 246, 0.25), inset 0 1px 0 rgba(255, 255, 255, 0.15)',
    borderDefault: 'border-blue-400/30'
  },
  condition: {
    color: 'amber',
    background: 'rgba(245, 158, 11, 0.12)',
    backgroundHover: 'rgba(245, 158, 11, 0.18)',
    border: 'rgba(245, 158, 11, 0.25)',
    boxShadow: '0 8px 32px rgba(245, 158, 11, 0.15), inset 0 1px 0 rgba(255, 255, 255, 0.1)',
    boxShadowHover: '0 12px 40px rgba(245, 158, 11, 0.25), inset 0 1px 0 rgba(255, 255, 255, 0.15)',
    borderDefault: 'border-amber-400/30'
  },
  transformer: {
    color: 'purple',
    background: 'rgba(139, 92, 246, 0.12)',
    backgroundHover: 'rgba(139, 92, 246, 0.18)',
    border: 'rgba(139, 92, 246, 0.25)',
    boxShadow: '0 8px 32px rgba(139, 92, 246, 0.15), inset 0 1px 0 rgba(255, 255, 255, 0.1)',
    boxShadowHover: '0 12px 40px rgba(139, 92, 246, 0.25), inset 0 1px 0 rgba(255, 255, 255, 0.15)',
    borderDefault: 'border-purple-400/30'
  },
  'http-request': {
    color: 'green',
    background: 'rgba(16, 185, 129, 0.12)',
    backgroundHover: 'rgba(16, 185, 129, 0.18)',
    border: 'rgba(16, 185, 129, 0.25)',
    boxShadow: '0 8px 32px rgba(16, 185, 129, 0.15), inset 0 1px 0 rgba(255, 255, 255, 0.1)',
    boxShadowHover: '0 12px 40px rgba(16, 185, 129, 0.25), inset 0 1px 0 rgba(255, 255, 255, 0.15)',
    borderDefault: 'border-green-400/30'
  },
  openobserve: {
    color: 'orange',
    background: 'rgba(249, 115, 22, 0.12)',
    backgroundHover: 'rgba(249, 115, 22, 0.18)',
    border: 'rgba(249, 115, 22, 0.25)',
    boxShadow: '0 8px 32px rgba(249, 115, 22, 0.15), inset 0 1px 0 rgba(255, 255, 255, 0.1)',
    boxShadowHover: '0 12px 40px rgba(249, 115, 22, 0.25), inset 0 1px 0 rgba(255, 255, 255, 0.15)',
    borderDefault: 'border-orange-400/30'
  },
  email: {
    color: 'blue',
    background: 'rgba(33, 150, 243, 0.12)',
    backgroundHover: 'rgba(33, 150, 243, 0.18)',
    border: 'rgba(33, 150, 243, 0.25)',
    boxShadow: '0 8px 32px rgba(33, 150, 243, 0.15), inset 0 1px 0 rgba(255, 255, 255, 0.1)',
    boxShadowHover: '0 12px 40px rgba(33, 150, 243, 0.25), inset 0 1px 0 rgba(255, 255, 255, 0.15)',
    borderDefault: 'border-blue-400/30'
  },
  delay: {
    color: 'gray',
    background: 'rgba(107, 114, 128, 0.12)',
    backgroundHover: 'rgba(107, 114, 128, 0.18)',
    border: 'rgba(107, 114, 128, 0.25)',
    boxShadow: '0 8px 32px rgba(107, 114, 128, 0.15), inset 0 1px 0 rgba(255, 255, 255, 0.1)',
    boxShadowHover: '0 12px 40px rgba(107, 114, 128, 0.25), inset 0 1px 0 rgba(255, 255, 255, 0.15)',
    borderDefault: 'border-gray-400/30'
  },
  anthropic: {
    color: 'amber',
    background: 'rgba(217, 119, 6, 0.12)',
    backgroundHover: 'rgba(217, 119, 6, 0.18)',
    border: 'rgba(217, 119, 6, 0.25)',
    boxShadow: '0 8px 32px rgba(217, 119, 6, 0.15), inset 0 1px 0 rgba(255, 255, 255, 0.1)',
    boxShadowHover: '0 12px 40px rgba(217, 119, 6, 0.25), inset 0 1px 0 rgba(255, 255, 255, 0.15)',
    borderDefault: 'border-amber-600/30'
  },
  app: {
    color: 'green',
    background: 'rgba(16, 185, 129, 0.12)',
    backgroundHover: 'rgba(16, 185, 129, 0.18)',
    border: 'rgba(16, 185, 129, 0.25)',
    boxShadow: '0 8px 32px rgba(16, 185, 129, 0.15), inset 0 1px 0 rgba(255, 255, 255, 0.1)',
    boxShadowHover: '0 12px 40px rgba(16, 185, 129, 0.25), inset 0 1px 0 rgba(255, 255, 255, 0.15)',
    borderDefault: 'border-green-400/30'
  },
  'human-in-loop': {
    color: 'red',
    background: 'rgba(220, 38, 38, 0.12)',
    backgroundHover: 'rgba(220, 38, 38, 0.18)',
    border: 'rgba(220, 38, 38, 0.25)',
    boxShadow: '0 8px 32px rgba(220, 38, 38, 0.15), inset 0 1px 0 rgba(255, 255, 255, 0.1)',
    boxShadowHover: '0 12px 40px rgba(220, 38, 38, 0.25), inset 0 1px 0 rgba(255, 255, 255, 0.15)',
    borderDefault: 'border-red-400/30'
  }
} as const

export type NodeType = keyof typeof NODE_THEMES

export function getNodeTheme(nodeType: NodeType): NodeTheme {
  return NODE_THEMES[nodeType] || NODE_THEMES.trigger
}