<script setup lang="ts">
import { computed, onMounted, useTemplateRef, watch } from "vue";
import { useI18n } from "vue-i18n";
import { toRow, type ListedActivity } from "../activities/list";
import { activityTelemetry } from "../telemetry/commands";
import { useTelemetry } from "../telemetry/useTelemetry";
import ActivityType from "./ActivityType.vue";
import DeleteActivity from "./DeleteActivity.vue";
import TelemetryCharts from "./TelemetryCharts.vue";

const props = defineProps<{ activity: ListedActivity }>();
const emit = defineEmits<{ back: []; deleted: [] }>();
const i18n = useI18n();
const { t } = i18n;

const row = computed(() => toRow(props.activity, i18n));
const { telemetry, error, load } = useTelemetry(activityTelemetry);
watch(
  () => props.activity.id,
  (id) => void load(id),
  { immediate: true },
);

// Keyboard users land on the way back.
const backButton = useTemplateRef<HTMLButtonElement>("backButton");
onMounted(() => backButton.value?.focus());

const FIGURES = ["elapsedTime", "movingTime", "distance", "elevationGain", "averagePower", "averageHeartRate"] as const;
</script>

<template>
  <article aria-labelledby="activity-title" class="flex w-full max-w-5xl flex-col gap-4">
    <div class="flex items-center justify-between gap-4">
      <button
        ref="backButton"
        type="button"
        class="rounded-md px-2 py-1 text-sm font-medium text-neutral-600 hover:bg-neutral-100 focus-visible:outline-2 focus-visible:outline-neutral-500 dark:text-neutral-300 dark:hover:bg-neutral-800"
        @click="emit('back')"
      >
        <span aria-hidden="true">←</span> {{ t("detail.back") }}
      </button>
      <DeleteActivity :activity-id="activity.id" @deleted="emit('deleted')" />
    </div>
    <header class="flex flex-wrap items-center gap-x-3 gap-y-1">
      <!-- One line, so a space separates the date from the time when read or copied. -->
      <h2 id="activity-title" class="text-lg font-semibold">{{ row.date }} <span class="ml-1 font-normal text-neutral-500 tabular-nums dark:text-neutral-400">{{ row.time }}</span></h2>
      <ActivityType :is-indoor="row.isIndoor" :label="row.type" />
    </header>
    <dl
      class="grid grid-cols-3 gap-px overflow-hidden rounded-lg border border-neutral-200 bg-neutral-200 md:grid-cols-6 dark:border-neutral-700 dark:bg-neutral-700"
    >
      <div v-for="figure in FIGURES" :key="figure" class="bg-white px-4 py-3 dark:bg-neutral-800">
        <dt class="text-xs font-medium text-neutral-500 dark:text-neutral-400">
          {{ t(`activities.column.${figure}`) }}
        </dt>
        <dd class="mt-1 text-lg font-semibold whitespace-nowrap tabular-nums">{{ row[figure] }}</dd>
      </div>
    </dl>
    <p v-if="error" role="alert" class="text-red-700 dark:text-red-400">{{ t("detail.loadError", { reason: error }) }}</p>
    <p v-else-if="!telemetry" role="status" class="text-neutral-500 dark:text-neutral-400">{{ t("detail.loading") }}</p>
    <TelemetryCharts v-else :telemetry="telemetry" />
  </article>
</template>
