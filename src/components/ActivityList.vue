<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { toRow, type ListedActivity } from "../activities/list";
import ActivityType from "./ActivityType.vue";

const props = defineProps<{ activities: ListedActivity[] }>();
const emit = defineEmits<{ open: [id: number] }>();
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
          <!-- The whole row opens the detail; its date is the button keyboards reach. -->
          <tr
            v-for="row in rows"
            :key="row.id"
            class="cursor-pointer hover:bg-neutral-50 dark:hover:bg-neutral-700/40"
            @click="emit('open', row.id)"
          >
            <td class="px-4 py-3 whitespace-nowrap">
              <button
                :id="`activity-${row.id}`"
                type="button"
                class="rounded-sm text-left focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-neutral-500"
              >
                <!-- One line, so a space separates the date from the time when read or copied. -->
                <span class="font-medium">{{ row.date }}</span> <span class="ml-1 text-neutral-500 tabular-nums dark:text-neutral-400">{{ row.time }}</span>
              </button>
            </td>
            <td class="px-4 py-3">
              <ActivityType :is-indoor="row.isIndoor" :label="row.type" />
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
