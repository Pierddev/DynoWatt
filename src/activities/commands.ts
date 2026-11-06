import { invoke } from "@tauri-apps/api/core";
import type { ListedActivity } from "./list";

/** Lists the Athlete's Activities with their Activity Summary, newest first. */
export function listActivities(): Promise<ListedActivity[]> {
  return invoke<ListedActivity[]>("list_activities");
}

/** Deletes the Activity `id` together with its Source File in the Library: both, or neither. */
export function deleteActivity(id: number): Promise<void> {
  return invoke<void>("delete_activity", { id });
}
