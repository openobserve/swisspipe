<template>
  <div class="fixed bottom-4 left-1/2 transform -translate-x-1/2 z-50 space-y-2">
    <TransitionGroup
      enter-active-class="transform ease-out duration-300 transition"
      enter-from-class="translate-y-4 opacity-0"
      enter-to-class="translate-y-0 opacity-100"
      leave-active-class="transition ease-in duration-100"
      leave-from-class="opacity-100"
      leave-to-class="opacity-0"
    >
      <div
        v-for="toast in toasts"
        :key="toast.id"
        :class="[
          'w-[600px] shadow-lg rounded-lg pointer-events-auto ring-1 ring-black ring-opacity-5 overflow-hidden',
          getToastClasses(toast.type)
        ]"
      >
        <div class="p-4">
          <div class="flex items-start">
            <div class="flex-shrink-0">
              <CheckCircleIcon
                v-if="toast.type === 'success'"
                class="h-6 w-6 text-green-400"
                aria-hidden="true"
              />
              <XCircleIcon
                v-else-if="toast.type === 'error'"
                class="h-6 w-6 text-red-400"
                aria-hidden="true"
              />
              <ExclamationTriangleIcon
                v-else-if="toast.type === 'warning'"
                class="h-6 w-6 text-yellow-400"
                aria-hidden="true"
              />
              <InformationCircleIcon
                v-else
                class="h-6 w-6 text-blue-400"
                aria-hidden="true"
              />
            </div>
            <div class="ml-3 w-0 flex-1 pt-0.5">
              <p class="text-sm font-medium text-gray-100">
                {{ toast.title }}<span v-if="toast.message" class="font-normal text-gray-300">: {{ toast.message }}</span>
              </p>
            </div>
            <div class="ml-4 flex-shrink-0 flex">
              <button
                @click="removeToast(toast.id)"
                class="rounded-md inline-flex text-gray-400 hover:text-gray-200 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500"
              >
                <span class="sr-only">Close</span>
                <XMarkIcon class="h-5 w-5" aria-hidden="true" />
              </button>
            </div>
          </div>
        </div>
      </div>
    </TransitionGroup>
  </div>
</template>

<script setup lang="ts">
import {
  CheckCircleIcon,
  XCircleIcon,
  ExclamationTriangleIcon,
  InformationCircleIcon,
  XMarkIcon,
} from '@heroicons/vue/24/outline'
import { useToast, type ToastType } from '../../composables/useToast'

const { toasts, removeToast } = useToast()

function getToastClasses(type: ToastType) {
  const baseClasses = 'bg-slate-800 border'
  
  switch (type) {
    case 'success':
      return `${baseClasses} border-green-500/50`
    case 'error':
      return `${baseClasses} border-red-500/50`
    case 'warning':
      return `${baseClasses} border-yellow-500/50`
    case 'info':
    default:
      return `${baseClasses} border-blue-500/50`
  }
}
</script>