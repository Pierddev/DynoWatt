<script setup lang="ts">
import { useI18n } from "vue-i18n";

const { t } = useI18n();
</script>

<template>
  <div class="flex h-screen flex-col bg-neutral-50 text-neutral-900 dark:bg-neutral-900 dark:text-neutral-100">
    <header class="border-b border-neutral-200 px-6 py-4 dark:border-neutral-800">
      <h1 class="text-lg font-semibold">{{ t("app.name") }}</h1>
    </header>
    <main class="flex flex-1 items-center justify-center p-6">
      <p class="text-neutral-500">{{ t("activities.empty") }}</p>
    </main>
  </div>
</template>
