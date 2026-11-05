<script setup lang="ts">
import { useI18n } from "vue-i18n";
import ImportReport from "./components/ImportReport.vue";
import { importPaths, pickFiles, pickFolders } from "./import/commands";
import { useImport } from "./import/useImport";

const { t } = useI18n();
const { running, report, error, pick, dismiss } = useImport(importPaths);

const BUTTON =
  "rounded-md px-3 py-1.5 text-sm font-medium focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-neutral-500 disabled:cursor-not-allowed disabled:opacity-50";
</script>

<template>
  <div class="flex h-screen flex-col bg-neutral-50 text-neutral-900 dark:bg-neutral-900 dark:text-neutral-100">
    <header
      class="flex items-center justify-between gap-4 border-b border-neutral-200 px-6 py-4 dark:border-neutral-800"
    >
      <h1 class="text-lg font-semibold">{{ t("app.name") }}</h1>
      <div class="flex gap-2">
        <button
          type="button"
          :class="BUTTON"
          class="bg-neutral-900 text-white enabled:hover:bg-neutral-700 dark:bg-neutral-100 dark:text-neutral-900 dark:enabled:hover:bg-neutral-300"
          :disabled="running"
          @click="pick(() => pickFiles(t('import.fitFiles')))"
        >
          {{ t("import.files") }}
        </button>
        <button
          type="button"
          :class="BUTTON"
          class="border border-neutral-300 enabled:hover:bg-neutral-100 dark:border-neutral-700 dark:enabled:hover:bg-neutral-800"
          :disabled="running"
          @click="pick(pickFolders)"
        >
          {{ t("import.folder") }}
        </button>
      </div>
    </header>
    <main
      class="flex flex-1 flex-col items-center gap-4 overflow-y-auto p-6"
      :class="report && !running ? 'justify-start' : 'justify-center'"
    >
      <p v-if="error" role="alert" class="text-red-700 dark:text-red-400">
        {{ t("import.error", { reason: error }) }}
      </p>
      <p v-if="running" role="status" class="text-neutral-500 dark:text-neutral-400">{{ t("import.running") }}</p>
      <ImportReport v-else-if="report" :report="report" @close="dismiss" />
      <p v-else class="text-neutral-500 dark:text-neutral-400">{{ t("activities.empty") }}</p>
    </main>
  </div>
</template>
