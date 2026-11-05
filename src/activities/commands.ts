import { invoke } from "@tauri-apps/api/core";
import type { ListedActivity } from "./list";

/** Lists the Athlete's Activities with their Activity Summary, newest first. */
export function listActivities(): Promise<ListedActivity[]> {
  return invoke<ListedActivity[]>("list_activities");
}
