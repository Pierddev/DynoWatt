<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { toRow } from "../activities/list";
import type { SkippedActivity } from "../activities/recompute";

const props = defineProps<{ skipped: SkippedActivity[] }>();
const emit = defineEmits<{ open: [id: number]; close: [] }>();
const i18n = useI18n();
const { t } = i18n;

const rows = computed(() => props.skipped.map(({ activity, reason }) => ({ row: toRow(activity, i18n), reason })));
</script>

<template>
  <section
    aria-labelledby="recompute-report-title"
    class="w-full max-w-5xl rounded-lg border border-neutral-200 bg-white dark:border-neutral-700 dark:bg-neutral-800"
  >
    <header
      class="flex items-center justify-between gap-4 border-b border-neutral-200 px-4 py-3 dark:border-neutral-700"
    >
      <h2 id="recompute-report-title" class="font-semibold">{{ t("recompute.title") }}</h2>
      <button
        type="button"
        class="rounded-md px-2 py-1 text-sm font-medium hover:bg-neutral-100 focus-visible:outline-2 focus-visible:outline-neutral-500 dark:hover:bg-neutral-700"
        @click="emit('close')"
      >
        {{ t("recompute.close") }}
      </button>
    </header>

    <p class="px-4 py-3 text-sm text-amber-700 dark:text-amber-400">{{ t("recompute.body") }}</p>

    <ul class="divide-y divide-neutral-200 border-t border-neutral-200 dark:divide-neutral-700 dark:border-neutral-700">
      <li v-for="{ row, reason } in rows" :key="row.id" class="px-4 py-3">
        <!-- Opens the Activity, where it can be deleted. One line, so a space separates the date from the time. -->
        <button
          type="button"
          class="rounded-sm text-left hover:underline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-neutral-500"
          @click="emit('open', row.id)"
        >
          <span class="font-medium">{{ row.date }}</span> <span class="ml-1 text-neutral-500 tabular-nums dark:text-neutral-400">{{ row.time }}</span>
        </button>
        <p class="mt-1 text-sm [overflow-wrap:anywhere] text-neutral-700 dark:text-neutral-300">{{ reason }}</p>
      </li>
    </ul>
  </section>
</template>
