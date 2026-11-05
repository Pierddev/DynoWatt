<script setup lang="ts">
import uPlot from "uplot";
import "uplot/dist/uPlot.min.css";
import { computed, nextTick, onBeforeUnmount, onMounted, ref, shallowRef, useId, watch } from "vue";
import { useI18n } from "vue-i18n";
import { formatDuration } from "../activities/list";
import { chartsOf, formatValue, tickLabel, type ChannelKey, type Chart, type Telemetry } from "../telemetry/telemetry";
import { dragSeconds, isWhole, pan, wheelFactor, whole, zoom, type TimeWindow } from "../telemetry/timeWindow";

const props = defineProps<{ telemetry: Telemetry }>();
const i18n = useI18n();
const { t } = i18n;

/** Each channel's line, light then dark: the Tailwind 600 and 400 shades. */
const COLOURS: Record<ChannelKey, [light: string, dark: string]> = {
  power: ["#d97706", "#fbbf24"],
  heartRate: ["#dc2626", "#f87171"],
  cadence: ["#0284c7", "#38bdf8"],
  altitude: ["#059669", "#34d399"],
  speed: ["#7c3aed", "#a78bfa"],
};
/** Axis labels and grid lines: neutral 500 and 200 when light, 400 and 700 when dark. */
const AXIS = { light: { text: "#737373", grid: "#e5e5e5" }, dark: { text: "#a3a3a3", grid: "#404040" } };
const HEIGHT_PX = 150;
/** One width for every value axis, so the plots line up under the shared cursor. */
const VALUE_AXIS_PX = 52;
/** The spacings, in seconds, the time axis may put between its ticks. */
const TICK_STEPS = [1, 2, 5, 10, 15, 30, 60, 120, 300, 600, 900, 1800, 3600, 7200, 10800];

const charts = computed(() => chartsOf(props.telemetry));
const end = computed(() => props.telemetry.timeS[props.telemetry.timeS.length - 1] ?? 0);
const view = shallowRef<TimeWindow>(whole(end.value));
const zoomed = computed(() => !isWhole(view.value, end.value));
const dragging = ref(false);
/** The Sample under the cursor, `null` when the cursor is off the charts. */
const cursor = ref<number | null>(null);
const cursorTime = computed(() =>
  cursor.value === null ? null : formatDuration(props.telemetry.timeS[cursor.value]),
);

const syncKey = useId();
const darkScheme = window.matchMedia("(prefers-color-scheme: dark)");
const plotElements: HTMLDivElement[] = [];
let plots: uPlot[] = [];
let frame = 0;
const resizes = new ResizeObserver(() => {
  for (const [i, plot] of plots.entries()) plot.setSize({ width: plotElements[i].clientWidth, height: HEIGHT_PX });
});

/** The value axis of power, cadence and speed: from zero to a little above the highest value shown. */
const fromZero: uPlot.Range.Function = (_plot, _min, max) => [
  0,
  max > 0 ? (uPlot.rangeNum(0, max, 0.1, true)[1] ?? max) : 1,
];

function options(chart: Chart, width: number, dark: boolean): uPlot.Options {
  const axis = dark ? AXIS.dark : AXIS.light;
  const colour = COLOURS[chart.key][dark ? 1 : 0];
  const grid = { stroke: axis.grid, width: 1 };
  return {
    width,
    height: HEIGHT_PX,
    legend: { show: false },
    select: { show: false, left: 0, top: 0, width: 0, height: 0 },
    cursor: {
      y: false,
      sync: { key: syncKey, setSeries: false },
      // Dragging pans and a double click shows the whole ride: see `bind`.
      drag: { x: false, y: false, setScale: false },
      bind: { dblclick: () => null },
    },
    scales: { x: { time: false }, y: chart.fromZero ? { range: fromZero } : {} },
    axes: [
      {
        stroke: axis.text,
        grid,
        ticks: grid,
        space: 60,
        incrs: TICK_STEPS,
        values: (_plot, splits, _axis, _space, step) => splits.map((seconds) => tickLabel(seconds, step)),
      },
      { stroke: axis.text, grid, ticks: grid, size: VALUE_AXIS_PX },
    ],
    series: [
      {},
      {
        stroke: colour,
        width: 1.5,
        // The altitude profile reads as terrain.
        fill: chart.key === "altitude" ? `${colour}33` : undefined,
        points: { show: false },
      },
    ],
    hooks: {
      setCursor: [
        (plot) => {
          cursor.value = plot.cursor.idx ?? null;
        },
      ],
    },
  };
}

/** Shows `next` in every chart, at most once per frame however often the wheel turns. */
function show(next: TimeWindow) {
  if (next.min === view.value.min && next.max === view.value.max) return;
  view.value = next;
  frame ||= requestAnimationFrame(() => {
    frame = 0;
    for (const plot of plots) plot.setScale("x", view.value);
  });
}

/**
 * Ctrl + wheel (a pinch on a touchpad) zooms around the pointer, a sideways
 * swipe or dragging pans, a double click shows the whole ride. A plain wheel
 * still scrolls the page: the stack of charts is taller than the window.
 */
function bind(plot: uPlot) {
  const over = plot.over;
  over.addEventListener(
    "wheel",
    (event) => {
      if (event.ctrlKey || event.metaKey) {
        event.preventDefault();
        const focus = plot.posToVal(event.clientX - over.getBoundingClientRect().left, "x");
        show(zoom(view.value, focus, wheelFactor(event.deltaY), end.value));
      } else if (Math.abs(event.deltaX) > Math.abs(event.deltaY)) {
        event.preventDefault();
        show(pan(view.value, dragSeconds(-event.deltaX, view.value, over.clientWidth), end.value));
      }
    },
    { passive: false },
  );
  over.addEventListener("pointerdown", (down) => {
    if (down.button !== 0) return;
    over.setPointerCapture(down.pointerId);
    const from = view.value;
    dragging.value = true;
    const move = (event: PointerEvent) =>
      show(pan(from, dragSeconds(event.clientX - down.clientX, from, over.clientWidth), end.value));
    const stop = () => {
      dragging.value = false;
      over.removeEventListener("pointermove", move);
      over.removeEventListener("pointerup", stop);
      over.removeEventListener("pointercancel", stop);
    };
    over.addEventListener("pointermove", move);
    over.addEventListener("pointerup", stop);
    over.addEventListener("pointercancel", stop);
  });
  over.addEventListener("dblclick", () => show(whole(end.value)));
}

function build() {
  const dark = darkScheme.matches;
  plots = charts.value.map((chart, i) => {
    const element = plotElements[i];
    const plot = new uPlot(options(chart, element.clientWidth, dark), [props.telemetry.timeS, chart.values], element);
    plot.setScale("x", view.value);
    bind(plot);
    return plot;
  });
  for (const element of plotElements.slice(0, plots.length)) resizes.observe(element);
}

function destroy() {
  resizes.disconnect();
  for (const plot of plots) plot.destroy();
  plots = [];
  cursor.value = null;
}

/** The charts drawn again, in the colours of the scheme or for other telemetry. */
function rebuild() {
  destroy();
  build();
}

onMounted(build);
darkScheme.addEventListener("change", rebuild);
watch(
  () => props.telemetry,
  async () => {
    view.value = whole(end.value);
    await nextTick();
    rebuild();
  },
);
onBeforeUnmount(() => {
  cancelAnimationFrame(frame);
  darkScheme.removeEventListener("change", rebuild);
  destroy();
});
</script>

<template>
  <section
    :aria-label="t('telemetry.title')"
    class="rounded-lg border border-neutral-200 bg-white p-4 dark:border-neutral-700 dark:bg-neutral-800"
  >
    <p v-if="!charts.length" class="text-sm text-neutral-500 dark:text-neutral-400">{{ t("telemetry.none") }}</p>
    <template v-else>
      <div class="flex min-h-7 items-center justify-between gap-4 text-sm">
        <p class="text-neutral-500 tabular-nums dark:text-neutral-400">
          <template v-if="cursorTime">{{ t("telemetry.time", { value: cursorTime }) }}</template>
        </p>
        <div class="flex items-center gap-3">
          <p class="text-xs text-neutral-500 dark:text-neutral-400">{{ t("telemetry.hint") }}</p>
          <button
            v-if="zoomed"
            type="button"
            class="rounded-md border border-neutral-300 px-2 py-1 text-xs font-medium hover:bg-neutral-100 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-neutral-500 dark:border-neutral-600 dark:hover:bg-neutral-700"
            @click="show(whole(end))"
          >
            {{ t("telemetry.reset") }}
          </button>
        </div>
      </div>
      <figure v-for="(chart, i) in charts" :key="chart.key" class="mt-3">
        <figcaption class="flex items-baseline justify-between text-sm" :style="{ paddingLeft: `${VALUE_AXIS_PX}px` }">
          <span class="font-medium">{{ t(`telemetry.channel.${chart.key}`) }}</span>
          <span v-if="cursor !== null" class="font-medium tabular-nums">
            {{ formatValue(chart.key, chart.values[cursor], i18n) }}
          </span>
        </figcaption>
        <div
          :ref="(element) => (plotElements[i] = element as HTMLDivElement)"
          role="img"
          :aria-label="t('telemetry.chart', { channel: t(`telemetry.channel.${chart.key}`) })"
          class="select-none"
          :class="dragging ? 'cursor-grabbing' : zoomed ? 'cursor-grab' : ''"
        />
      </figure>
    </template>
  </section>
</template>
