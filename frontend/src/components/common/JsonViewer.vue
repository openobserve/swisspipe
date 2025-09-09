<template>
  <div class="json-viewer">
    <pre v-if="!data || (typeof data === 'object' && Object.keys(data).length === 0)" class="text-gray-500 italic">{{ emptyMessage }}</pre>
    <div v-else class="json-content">
      <JsonNode 
        :data="data" 
        :path="'root'" 
        :level="0"
        :collapsed="collapsed"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import JsonNode from './JsonNode.vue'

interface Props {
  data: unknown
  collapsed?: boolean
  theme?: 'light' | 'dark'
}

const props = withDefaults(defineProps<Props>(), {
  collapsed: false,
  theme: 'dark'
})

const emptyMessage = computed(() => {
  if (!props.data) return 'null'
  if (typeof props.data === 'object' && Object.keys(props.data).length === 0) {
    return Array.isArray(props.data) ? '[]' : '{}'
  }
  return ''
})
</script>

<style scoped>
.json-viewer {
  font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
  font-size: 13px;
  line-height: 1.4;
}

.json-content {
  color: #e2e8f0; /* slate-200 */
}

pre {
  margin: 0;
  padding: 12px;
  background: rgba(30, 41, 59, 0.5); /* slate-800 */
  border-radius: 6px;
  border: 1px solid rgba(71, 85, 105, 0.3); /* slate-600 */
}
</style>