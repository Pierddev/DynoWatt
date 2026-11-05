<script setup lang="ts">
defineProps<{ isIndoor: boolean; label: string }>();
</script>

<template>
  <span
    class="rounded-full px-2 py-0.5 text-xs font-medium"
    :class="
      isIndoor
        ? 'bg-sky-50 text-sky-800 dark:bg-sky-400/10 dark:text-sky-300'
        : 'bg-neutral-100 text-neutral-700 dark:bg-neutral-700 dark:text-neutral-200'
    "
  >
    {{ label }}
  </span>
</template>
