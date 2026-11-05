<script setup lang="ts">
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { computed, nextTick, onBeforeUnmount, onMounted, ref, useTemplateRef } from "vue";
import { useI18n } from "vue-i18n";
import { listActivities } from "./activities/commands";
import { useActivities } from "./activities/useActivities";
import ActivityDetail from "./components/ActivityDetail.vue";
import ActivityList from "./components/ActivityList.vue";
import ImportReport from "./components/ImportReport.vue";
import { importPaths, pickFiles, pickFolders } from "./import/commands";
import { useImport } from "./import/useImport";

const { t } = useI18n();
const { activities, error: listError, load } = useActivities(listActivities);
const { running, dragging, report, error, pick, onDragDrop, dismiss } = useImport(async (paths) => {
  const imported = await importPaths(paths);
  // The list shows what the run imported along with its Import Report.
  void load();
  return imported;
});
void load();

// The Activity whose detail shows in place of the list.
const opened = ref<number | null>(null);
const openedActivity = computed(() => activities.value?.find(({ id }) => id === opened.value) ?? null);
const main = useTemplateRef<HTMLElement>("main");
let listScroll = 0;

async function open(id: number) {
  listScroll = main.value?.scrollTop ?? 0;
  opened.value = id;
  await nextTick();
  main.value?.scrollTo({ top: 0 });
}

/** Back to the list, where it was left, the opened Activity focused. */
async function back() {
  const id = opened.value;
  opened.value = null;
  await nextTick();
  main.value?.scrollTo({ top: listScroll });
  document.getElementById(`activity-${id}`)?.focus();
}

// Centred only while the main area holds a short message, not a list or a report.
const centred = computed(() => !activities.value?.length && (running.value || !report.value));

// Files dropped anywhere on the window: the webview catches the drop, not the DOM.
let stopDragDrop: (() => void) | undefined;
onMounted(async () => {
  stopDragDrop = await getCurrentWebview().onDragDropEvent(({ payload }) => onDragDrop(payload));
});
onBeforeUnmount(() => stopDragDrop?.());

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
      ref="main"
      class="flex flex-1 flex-col items-center gap-4 overflow-y-auto p-6"
      :class="centred ? 'justify-center' : 'justify-start'"
    >
      <p v-if="error" role="alert" class="text-red-700 dark:text-red-400">
        {{ t("import.error", { reason: error }) }}
      </p>
      <p v-if="listError" role="alert" class="text-red-700 dark:text-red-400">
        {{ t("activities.loadError", { reason: listError }) }}
      </p>
      <p v-if="running" role="status" class="text-neutral-500 dark:text-neutral-400">{{ t("import.running") }}</p>
      <ImportReport v-else-if="report" :report="report" @close="dismiss" />
      <ActivityDetail v-if="openedActivity" :activity="openedActivity" @back="back" />
      <ActivityList v-else-if="activities?.length" :activities="activities" @open="open" />
      <div v-else-if="activities && !running && !report" class="text-center text-neutral-500 dark:text-neutral-400">
        <p>{{ t("activities.empty") }}</p>
        <p class="mt-1 text-sm">{{ t("import.dropHint") }}</p>
      </div>
    </main>
    <div
      v-if="dragging"
      class="pointer-events-none fixed inset-3 z-10 flex items-center justify-center rounded-xl border-2 border-dashed border-neutral-400 bg-neutral-50/90 text-lg font-medium dark:border-neutral-500 dark:bg-neutral-900/90"
    >
      {{ t("import.dropHere") }}
    </div>
  </div>
</template>
