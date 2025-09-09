<template>
  <span
    class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium"
    :class="statusClasses"
  >
    <div :class="dotClasses" class="w-1.5 h-1.5 rounded-full mr-1.5"></div>
    {{ statusText }}
  </span>
</template>

<script setup lang="ts">
import { computed } from 'vue'

interface Props {
  status: string
}

const props = defineProps<Props>()

const statusConfig = computed(() => {
  switch (props.status.toLowerCase()) {
    case 'completed':
      return {
        text: 'Completed',
        badgeClass: 'bg-green-100 text-green-800',
        dotClass: 'bg-green-500'
      }
    case 'failed':
      return {
        text: 'Failed',
        badgeClass: 'bg-red-100 text-red-800',
        dotClass: 'bg-red-500'
      }
    case 'running':
      return {
        text: 'Running',
        badgeClass: 'bg-blue-100 text-blue-800',
        dotClass: 'bg-blue-500 animate-pulse'
      }
    case 'pending':
      return {
        text: 'Pending',
        badgeClass: 'bg-yellow-100 text-yellow-800',
        dotClass: 'bg-yellow-500'
      }
    case 'cancelled':
      return {
        text: 'Cancelled',
        badgeClass: 'bg-gray-100 text-gray-800',
        dotClass: 'bg-gray-500'
      }
    default:
      return {
        text: props.status,
        badgeClass: 'bg-gray-100 text-gray-800',
        dotClass: 'bg-gray-500'
      }
  }
})

const statusText = computed(() => statusConfig.value.text)
const statusClasses = computed(() => statusConfig.value.badgeClass)
const dotClasses = computed(() => statusConfig.value.dotClass)
</script>