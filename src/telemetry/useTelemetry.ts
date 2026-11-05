import { ref, shallowRef } from "vue";
import type { Telemetry } from "./telemetry";

/** The telemetry of the Activity `id`, decoded again from its Source File. */
export type TelemetryLoader = (id: number) => Promise<Telemetry>;

/** The telemetry of the Activity the detail shows: `null` while it loads or once it has failed. */
export function useTelemetry(loadTelemetry: TelemetryLoader) {
  // Shallow: thousands of Samples per channel, replaced whole, never edited.
  const telemetry = shallowRef<Telemetry | null>(null);
  const error = ref<string | null>(null);
  let latest = 0;

  /** Loads the telemetry of the Activity `id` in place of the last one. Only the latest load counts. */
  async function load(id: number): Promise<void> {
    const call = ++latest;
    telemetry.value = null;
    error.value = null;
    try {
      const loaded = await loadTelemetry(id);
      if (call === latest) telemetry.value = loaded;
    } catch (e) {
      if (call === latest) error.value = String(e);
    }
  }

  return { telemetry, error, load };
}
