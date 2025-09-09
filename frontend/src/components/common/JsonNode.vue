<template>
  <div class="json-node">
    <!-- Object -->
    <div v-if="isObject" class="json-object">
      <div class="json-line" :style="{ paddingLeft: `${level * 16}px` }">
        <button
          v-if="hasChildren"
          @click="toggle"
          class="json-toggle"
          :class="{ 'collapsed': isCollapsed }"
        >
          <ChevronRightIcon class="h-3 w-3" />
        </button>
        <span v-else class="json-toggle-placeholder"></span>
        
        <span v-if="showKey" class="json-key">{{ displayKey }}: </span>
        <span class="json-brace">{{ isCollapsed ? (isArray ? '[...]' : '{...}') : (isArray ? '[' : '{') }}</span>
        <span v-if="isCollapsed && hasChildren" class="json-count">{{ childrenCount }} {{ isArray ? 'items' : 'properties' }}</span>
      </div>
      
      <div v-if="!isCollapsed && hasChildren">
        <JsonNode
          v-for="(value, key) in data"
          :key="key"
          :data="value"
          :path="`${path}.${key}`"
          :level="level + 1"
          :object-key="key"
          :collapsed="collapsed"
        />
        <div class="json-line" :style="{ paddingLeft: `${level * 16}px` }">
          <span class="json-toggle-placeholder"></span>
          <span class="json-brace">{{ isArray ? ']' : '}' }}</span>
        </div>
      </div>
    </div>
    
    <!-- Primitive values -->
    <div v-else class="json-primitive">
      <div class="json-line" :style="{ paddingLeft: `${level * 16}px` }">
        <span class="json-toggle-placeholder"></span>
        <span v-if="showKey" class="json-key">{{ displayKey }}: </span>
        <span :class="valueClass">{{ displayValue }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { ChevronRightIcon } from '@heroicons/vue/24/outline'

interface Props {
  data: any
  path: string
  level: number
  objectKey?: string | number
  collapsed?: boolean
}

const props = defineProps<Props>()

const isCollapsed = ref(props.collapsed)

const isObject = computed(() => {
  return props.data !== null && typeof props.data === 'object'
})

const isArray = computed(() => {
  return Array.isArray(props.data)
})

const hasChildren = computed(() => {
  if (!isObject.value) return false
  return Object.keys(props.data).length > 0
})

const childrenCount = computed(() => {
  if (!isObject.value) return 0
  return Object.keys(props.data).length
})

const showKey = computed(() => {
  return props.objectKey !== undefined && props.level > 0
})

const displayKey = computed(() => {
  if (typeof props.objectKey === 'string') {
    return `"${props.objectKey}"`
  }
  return props.objectKey
})

const displayValue = computed(() => {
  if (props.data === null) return 'null'
  if (props.data === undefined) return 'undefined'
  if (typeof props.data === 'string') return `"${props.data}"`
  if (typeof props.data === 'boolean') return props.data.toString()
  if (typeof props.data === 'number') return props.data.toString()
  return String(props.data)
})

const valueClass = computed(() => {
  const baseClass = 'json-value'
  
  if (props.data === null) return `${baseClass} json-null`
  if (props.data === undefined) return `${baseClass} json-undefined`
  if (typeof props.data === 'string') return `${baseClass} json-string`
  if (typeof props.data === 'boolean') return `${baseClass} json-boolean`
  if (typeof props.data === 'number') return `${baseClass} json-number`
  
  return baseClass
})

function toggle() {
  isCollapsed.value = !isCollapsed.value
}
</script>

<style scoped>
.json-node {
  font-family: inherit;
}

.json-line {
  display: flex;
  align-items: center;
  line-height: 1.4;
  min-height: 18px;
}

.json-toggle {
  width: 16px;
  height: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: none;
  border: none;
  color: #64748b; /* slate-500 */
  cursor: pointer;
  padding: 0;
  margin-right: 4px;
  border-radius: 2px;
  transition: all 0.15s ease;
}

.json-toggle:hover {
  background: rgba(100, 116, 139, 0.2);
  color: #94a3b8; /* slate-400 */
}

.json-toggle.collapsed {
  transform: rotate(0deg);
}

.json-toggle:not(.collapsed) {
  transform: rotate(90deg);
}

.json-toggle-placeholder {
  width: 20px;
  height: 16px;
}

.json-key {
  color: #7dd3fc; /* sky-300 */
  margin-right: 4px;
}

.json-brace {
  color: #cbd5e1; /* slate-300 */
  font-weight: 500;
}

.json-count {
  color: #64748b; /* slate-500 */
  font-style: italic;
  margin-left: 8px;
  font-size: 11px;
}

.json-value {
  word-break: break-all;
}

.json-string {
  color: #86efac; /* green-300 */
}

.json-number {
  color: #fbbf24; /* amber-400 */
}

.json-boolean {
  color: #c084fc; /* purple-400 */
  font-weight: 500;
}

.json-null {
  color: #64748b; /* slate-500 */
  font-style: italic;
}

.json-undefined {
  color: #64748b; /* slate-500 */
  font-style: italic;
}
</style>