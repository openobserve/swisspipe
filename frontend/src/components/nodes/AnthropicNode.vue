<template>
  <BaseNode
    node-type="anthropic"
    :data="data"
    :subtitle="getAnthropicSummary()"
    default-label="Anthropic"
  >
    <template #handles>
      <!-- Connection handles with custom styling -->
      <Handle
        type="target"
        :position="Position.Top"
        :style="{ background: '#ddd' }"
      />
      <Handle
        type="source"
        :position="Position.Bottom"
        :style="{ background: '#ddd' }"
      />

      <!-- Sparkles icon -->
      <div class="absolute top-2 right-2 text-amber-500">
        <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
          <path fill-rule="evenodd" d="M5 2a1 1 0 011 1v1h1a1 1 0 110 2H6v1a1 1 0 11-2 0V6H3a1 1 0 110-2h1V3a1 1 0 011-1zm0 10a1 1 0 011 1v1h1a1 1 0 110 2H6v1a1 1 0 11-2 0v-1H3a1 1 0 110-2h1v-1a1 1 0 011-1zM12 2a1 1 0 01.967.742L14.146 7.2 17.5 8.134a1 1 0 010 1.732L14.146 10.8l-1.179 4.458a1 1 0 01-1.934 0L9.854 10.8 6.5 9.866a1 1 0 010-1.732L9.854 7.2l1.179-4.458A1 1 0 0112 2z" clip-rule="evenodd" />
        </svg>
      </div>
    </template>
  </BaseNode>
</template>

<script setup lang="ts">
import { Handle, Position } from '@vue-flow/core'
import BaseNode from './BaseNode.vue'
import type { AnthropicConfig } from '../../types/nodes'

interface Props {
  data: {
    label: string
    description?: string
    status?: string
    config: AnthropicConfig
    isTracing?: boolean
    executionStatus?: string
    executionDuration?: number
    executionError?: string
  }
}

const props = defineProps<Props>()

function getAnthropicSummary(): string {
  const config = props.data.config
  if (!config.model) {
    return 'Not configured'
  }

  // Extract model display name
  const modelName = config.model.includes('claude-3-5-sonnet') ? 'Claude 3.5 Sonnet' :
                   config.model.includes('claude-3-5-haiku') ? 'Claude 3.5 Haiku' :
                   config.model.includes('claude-3-opus') ? 'Claude 3 Opus' :
                   config.model.includes('claude-3-sonnet') ? 'Claude 3 Sonnet' :
                   config.model.includes('claude-3-haiku') ? 'Claude 3 Haiku' :
                   config.model

  return `${modelName} (${config.max_tokens} tokens)`
}
</script>

<style scoped>
.node-anthropic {
  font-family: 'Inter', sans-serif;
}
</style>