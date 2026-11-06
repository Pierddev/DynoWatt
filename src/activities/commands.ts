import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ListedActivity } from "./list";
import type { RecomputeStatus } from "./recompute";

/** Lists the Athlete's Activities with their Activity Summary, newest first. */
export function listActivities(): Promise<ListedActivity[]> {
  return invoke<ListedActivity[]>("list_activities");
}

/** Deletes the Activity `id` together with its Source File in the Library: both, or neither. */
export function deleteActivity(id: number): Promise<void> {
  return invoke<void>("delete_activity", { id });
}

/** Where the recompute of stale summaries started at launch stands. */
export function recomputeStatus(): Promise<RecomputeStatus> {
  return invoke<RecomputeStatus>("recompute_status");
}

/** Calls `handler` with the status the recompute ends with; resolves to the way to stop listening. */
export function onRecomputeFinished(handler: (status: RecomputeStatus) => void): Promise<() => void> {
  return listen<RecomputeStatus>("recompute-finished", ({ payload }) => handler(payload));
}
