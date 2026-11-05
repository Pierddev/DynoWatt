import { ref } from "vue";
import type { ListedActivity } from "./list";

/** Lists the Athlete's Activities, newest first. */
export type Lister = () => Promise<ListedActivity[]>;

/** The activity list, `null` until it first loads. */
export function useActivities(listActivities: Lister) {
  const activities = ref<ListedActivity[] | null>(null);
  const error = ref<string | null>(null);
  let latest = 0;

  /** Loads the list again. A failure keeps the list shown; only the latest load counts. */
  async function load(): Promise<void> {
    const call = ++latest;
    try {
      const listed = await listActivities();
      if (call !== latest) return;
      activities.value = listed;
      error.value = null;
    } catch (e) {
      if (call === latest) error.value = String(e);
    }
  }

  return { activities, error, load };
}
