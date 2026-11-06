import { computed, ref } from "vue";
import type { RecomputeStatus } from "./recompute";

/** Asks where the recompute stands. */
export type StatusReader = () => Promise<RecomputeStatus>;

/** Calls its handler with the status the recompute ends with; resolves to the way to stop. */
export type FinishWatcher = (handler: (status: RecomputeStatus) => void) => Promise<() => void>;

/**
 * Follows the recompute of stale summaries the app starts in the background.
 * Calls `recomputed` once if the run brought summaries up to date, so the
 * list can show the new figures.
 */
export function useRecompute(readStatus: StatusReader, onFinished: FinishWatcher, recomputed: () => void) {
  const status = ref<RecomputeStatus | null>(null);
  const dismissed = ref(false);

  const running = computed(() => status.value?.kind === "running");
  const skipped = computed(() => (status.value?.kind === "done" && !dismissed.value ? status.value.skipped : []));
  const error = computed(() => (status.value?.kind === "failed" ? status.value.reason : null));

  /** Takes in `next`, unless the run is already known to be over. */
  function settle(next: RecomputeStatus): void {
    if (status.value && status.value.kind !== "running") return;
    status.value = next;
    if (next.kind === "done" && next.recomputed.length > 0) recomputed();
  }

  /**
   * Starts following the recompute; resolves to the way to stop. Listens
   * before asking, so the end is never missed, though it may come twice.
   */
  async function follow(): Promise<() => void> {
    const stop = await onFinished(settle);
    try {
      settle(await readStatus());
    } catch (e) {
      settle({ kind: "failed", reason: String(e) });
    }
    return stop;
  }

  /** Closes the notice of the skipped Activities. */
  function dismiss(): void {
    dismissed.value = true;
  }

  return { running, skipped, error, follow, dismiss };
}
