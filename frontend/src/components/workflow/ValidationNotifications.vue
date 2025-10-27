<template>
  <div
    v-if="!validation.isValid || validation.warnings.length"
    class="fixed top-20 left-4 max-w-md z-40"
  >
    <!-- Validation Errors -->
    <div
      v-if="!validation.isValid"
      class="glass-dark bg-red-900/50 border border-red-700/50 text-red-100 px-4 py-3 rounded-lg mb-2 shadow-2xl"
    >
      <h4 class="font-medium">Validation Errors:</h4>
      <ul class="mt-1 text-sm">
        <li v-for="error in validation.errors" :key="error">
          • {{ error }}
        </li>
      </ul>
    </div>

    <!-- Validation Warnings -->
    <div
      v-if="validation.warnings.length"
      class="glass-dark bg-yellow-900/50 border border-yellow-700/50 text-yellow-100 px-4 py-3 rounded-lg shadow-2xl"
    >
      <h4 class="font-medium">Warnings:</h4>
      <ul class="mt-1 text-sm">
        <li v-for="warning in validation.warnings" :key="warning">
          • {{ warning }}
        </li>
      </ul>
    </div>
  </div>
</template>

<script setup lang="ts">
interface ValidationResult {
  isValid: boolean
  errors: string[]
  warnings: string[]
}

interface Props {
  validation: ValidationResult
}

defineProps<Props>()
</script>