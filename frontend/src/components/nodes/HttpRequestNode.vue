<template>
  <BaseNode
    node-type="http-request"
    :data="data"
    :node-id="nodeId"
    subtitle="HTTP Request"
    :handles="[
      { type: 'target', position: Position.Top },
      { type: 'source', position: Position.Bottom }
    ]"
    default-label="HTTP Request"
  >
    <template #content>
      <!-- Loop Indicator - Only show when tracing a specific execution -->
      <div v-if="hasLoopConfig && data.isTracing" class="mt-2">
        <div class="flex items-center gap-2">
          <div class="w-2 h-2 rounded-full bg-primary-500"></div>
          <span class="text-xs text-gray-400">Loop Status</span>
        </div>

        <!-- Loop status details -->
        <div class="mt-2 p-2 bg-slate-800 rounded text-xs">
          <div v-if="loopStatus">
            <div>Status: {{ loopStatus.status }}</div>
            <div>Iteration: {{ loopStatus.current_iteration }}</div>
          </div>
          <div v-else>
            No loop data for this execution
          </div>
        </div>
      </div>
    </template>
  </BaseNode>
</template>

<script setup lang="ts">
import { computed, onUnmounted, watch } from 'vue'
import { Position } from '@vue-flow/core'
import { useRoute } from 'vue-router'
import BaseNode from './BaseNode.vue'
import { useHttpLoop } from '../../composables/useHttpLoop'
import type { HttpRequestConfig, LoopStatus } from '../../types/nodes'

interface Props {
  data: {
    label: string
    description?: string
    status?: string
    config: HttpRequestConfig
    isTracing?: boolean
    tracingExecutionId?: string
    executionStatus?: string
    executionDuration?: number
    executionError?: string
    loopStatus?: LoopStatus
  }
  nodeId: string
}

const props = defineProps<Props>()

// Get route to access workflow ID
const route = useRoute()

// Use HTTP Loop composable with execution ID from node data
const { loopStatuses, refreshLoopData, startPolling, stopPolling, setExecutionId } = useHttpLoop()

// Only fetch loop data when tracing is enabled
const shouldFetchLoopData = computed(() => {
  return hasLoopConfig.value && props.data.isTracing
})

// Loop-related computed properties
const hasLoopConfig = computed(() => {
  const result = props.data.config.loop_config !== undefined
  console.log('hasLoopConfig computed:', result, 'loop_config:', props.data.config.loop_config)
  return result
})

// Watch hasLoopConfig for changes
watch(hasLoopConfig, (newValue, oldValue) => {
  console.log('hasLoopConfig changed from', oldValue, 'to', newValue)
})

// Note: loopId computation removed as it's not used for direct lookup

const loopStatus = computed(() => {
  if (!hasLoopConfig.value) return undefined

  const workflowId = route.params.id as string
  const nodeId = props.nodeId

  console.log(`=== LOOP STATUS MATCHING DEBUG ===`)
  console.log(`Current workflowId from route: "${workflowId}"`)
  console.log(`Current nodeId from props: "${nodeId}"`)
  console.log(`Total loop statuses available: ${loopStatuses.value.size}`)

  // Find loop by execution_step_id containing our node ID
  // The execution_step_id format appears to be: "{execution_id}_{node_id}"
  for (const [loopId, loopData] of loopStatuses.value.entries()) {
    console.log(`Checking loop ${loopId}:`)
    console.log(`  execution_step_id: "${loopData.execution_step_id}"`)

    // Check if execution_step_id ends with our nodeId (after underscore)
    const endsWithNodeId = loopData.execution_step_id?.endsWith(`_${nodeId}`)
    console.log(`  ends with _${nodeId}: ${endsWithNodeId}`)

    if (loopData.execution_step_id && endsWithNodeId) {
      console.log(`✅ MATCH FOUND! Using loop status:`, loopData)
      return loopData
    }
  }

  console.log(`❌ NO MATCH FOUND`)
  return undefined
})

// Watch for tracing execution ID changes to update the HTTP loop filter
watch(() => props.data.tracingExecutionId, (newExecutionId) => {
  setExecutionId(newExecutionId)
})

// Watch for tracing changes - only fetch data when tracing is enabled
watch(shouldFetchLoopData, async (newValue, oldValue) => {
  if (newValue && !oldValue) {
    // Started tracing - set execution ID and fetch initial data and start polling
    setExecutionId(props.data.tracingExecutionId)
    try {
      await refreshLoopData()
      startPolling(5000) // Poll every 5 seconds during tracing
    } catch (error) {
      console.error('Failed to fetch loop data for node:', props.nodeId, error)
    }
  } else if (!newValue && oldValue) {
    // Stopped tracing - clear execution ID and stop polling
    setExecutionId(undefined)
    stopPolling()
  }
}, { immediate: true })

// Always stop polling when component unmounts
onUnmounted(() => {
  stopPolling()
})

</script>