<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { toRow, type ListedActivity } from "../activities/list";

const props = defineProps<{ activities: ListedActivity[] }>();
const i18n = useI18n();
const { t } = i18n;

const rows = computed(() => props.activities.map((activity) => toRow(activity, i18n)));

const FIGURES = ["movingTime", "distance", "elevationGain", "averagePower", "averageHeartRate"] as const;
</script>

<template>
  <section aria-labelledby="activities-title" class="w-full max-w-5xl">
    <h2 id="activities-title" class="mb-3 font-semibold">{{ t("activities.title") }}</h2>
    <div
      class="overflow-x-auto rounded-lg border border-neutral-200 bg-white dark:border-neutral-700 dark:bg-neutral-800"
    >
      <table class="w-full text-sm">
        <thead
          class="border-b border-neutral-200 text-xs font-medium text-neutral-500 dark:border-neutral-700 dark:text-neutral-400"
        >
          <tr>
            <th scope="col" class="px-4 py-2.5 text-left font-medium">{{ t("activities.column.start") }}</th>
            <th scope="col" class="px-4 py-2.5 text-left font-medium">{{ t("activities.column.type") }}</th>
            <th v-for="figure in FIGURES" :key="figure" scope="col" class="px-4 py-2.5 text-right font-medium">
              {{ t(`activities.column.${figure}`) }}
            </th>
          </tr>
        </thead>
        <tbody class="divide-y divide-neutral-200 dark:divide-neutral-700">
          <tr v-for="row in rows" :key="row.id">
            <td class="px-4 py-3 whitespace-nowrap">
              <!-- One line, so a space separates the date from the time when read or copied. -->
              <span class="font-medium">{{ row.date }}</span> <span class="ml-1 text-neutral-500 tabular-nums dark:text-neutral-400">{{ row.time }}</span>
            </td>
            <td class="px-4 py-3">
              <span
                class="rounded-full px-2 py-0.5 text-xs font-medium"
                :class="
                  row.isIndoor
                    ? 'bg-sky-50 text-sky-800 dark:bg-sky-400/10 dark:text-sky-300'
                    : 'bg-neutral-100 text-neutral-700 dark:bg-neutral-700 dark:text-neutral-200'
                "
              >
                {{ row.type }}
              </span>
            </td>
            <td v-for="figure in FIGURES" :key="figure" class="px-4 py-3 text-right whitespace-nowrap tabular-nums">
              {{ row[figure] }}
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
</template>
