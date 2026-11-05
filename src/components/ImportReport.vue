<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import {
  countOutcomes,
  fileName,
  listedFiles,
  reportNote,
  type ImportReport,
  type OutcomeKind,
} from "../import/report";

const props = defineProps<{ report: ImportReport }>();
const emit = defineEmits<{ close: [] }>();
const { t } = useI18n();

const counts = computed(() => countOutcomes(props.report));
const files = computed(() => listedFiles(props.report));
const note = computed(() => reportNote(props.report));

const TONE: Record<OutcomeKind, string> = {
  imported: "text-emerald-700 dark:text-emerald-400",
  duplicate: "text-neutral-600 dark:text-neutral-300",
  notCycling: "text-amber-700 dark:text-amber-400",
  unreadable: "text-red-700 dark:text-red-400",
  failed: "text-red-700 dark:text-red-400",
};
</script>

<template>
  <section
    aria-labelledby="import-report-title"
    class="w-full max-w-5xl rounded-lg border border-neutral-200 bg-white dark:border-neutral-700 dark:bg-neutral-800"
  >
    <header
      class="flex items-center justify-between gap-4 border-b border-neutral-200 px-4 py-3 dark:border-neutral-700"
    >
      <h2 id="import-report-title" class="font-semibold">{{ t("importReport.title") }}</h2>
      <button
        type="button"
        class="rounded-md px-2 py-1 text-sm font-medium hover:bg-neutral-100 focus-visible:outline-2 focus-visible:outline-neutral-500 dark:hover:bg-neutral-700"
        @click="emit('close')"
      >
        {{ t("importReport.close") }}
      </button>
    </header>

    <ul v-if="counts.length" class="flex flex-wrap gap-2 px-4 pt-3" :aria-label="t('importReport.counts')">
      <li
        v-for="{ kind, count } in counts"
        :key="kind"
        class="rounded-full border border-neutral-200 px-2.5 py-0.5 text-sm font-medium dark:border-neutral-600"
        :class="TONE[kind]"
      >
        {{ t(`importReport.count.${kind}`, count) }}
      </li>
    </ul>

    <p v-if="note" class="px-4 py-3 text-sm text-neutral-600 dark:text-neutral-300">
      {{ t(`importReport.note.${note}`) }}
    </p>

    <ul
      v-if="files.length"
      class="mt-3 divide-y divide-neutral-200 border-t border-neutral-200 dark:divide-neutral-700 dark:border-neutral-700"
    >
      <li v-for="(file, index) in files" :key="index" class="px-4 py-3">
        <div class="flex items-baseline justify-between gap-4">
          <span class="min-w-0 truncate font-medium">{{ fileName(file.path) }}</span>
          <span class="shrink-0 text-sm font-medium" :class="TONE[file.outcome.kind]">
            {{ t(`importReport.outcome.${file.outcome.kind}`) }}
          </span>
        </div>
        <p class="truncate text-xs text-neutral-500 dark:text-neutral-400" :title="file.path">{{ file.path }}</p>
        <p v-if="'reason' in file.outcome" class="mt-1 text-sm text-neutral-700 dark:text-neutral-300">
          {{ file.outcome.reason }}
        </p>
        <div
          v-if="file.outcome.kind === 'imported' && file.outcome.overlaps.length"
          role="note"
          class="mt-2 rounded-md bg-amber-50 px-3 py-2 text-sm text-amber-900 dark:bg-amber-400/10 dark:text-amber-200"
        >
          <p>{{ t("importReport.overlaps", file.outcome.overlaps.length) }}</p>
          <ul class="mt-1 font-mono text-xs">
            <li v-for="path in file.outcome.overlaps" :key="path">{{ path }}</li>
          </ul>
        </div>
      </li>
    </ul>
  </section>
</template>
