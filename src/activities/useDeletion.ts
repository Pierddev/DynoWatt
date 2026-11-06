import { ref } from "vue";

/** Deletes the Activity `id` together with its Source File: both, or neither. */
export type Deleter = (id: number) => Promise<void>;

/** Deleting an Activity, one deletion at a time. */
export function useDeletion(deleteActivity: Deleter) {
  const deleting = ref(false);
  const error = ref<string | null>(null);

  /**
   * Deletes the Activity `id`: `true` once it is gone, `false` when it failed, the reason in `error`. Ignored, and
   * `false`, while another deletion is going on.
   */
  async function remove(id: number): Promise<boolean> {
    if (deleting.value) return false;
    deleting.value = true;
    error.value = null;
    try {
      await deleteActivity(id);
      return true;
    } catch (e) {
      error.value = String(e);
      return false;
    } finally {
      deleting.value = false;
    }
  }

  /** Forgets the last failure, once the Athlete gives up on the deletion. */
  function dismiss() {
    error.value = null;
  }

  return { deleting, error, remove, dismiss };
}
