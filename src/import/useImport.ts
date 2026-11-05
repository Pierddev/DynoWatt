import { ref } from "vue";
import type { ImportReport } from "./report";

/** Imports the Source Files at `paths`, files or folders, into the Library. */
export type Importer = (paths: string[]) => Promise<ImportReport>;

/** Asks the Athlete for paths to import; none when they cancel. */
export type Picker = () => Promise<string[]>;

/** A drag-and-drop over the window, as the webview reports it. */
export type DragDrop =
  | { type: "enter"; paths: string[] }
  | { type: "over" }
  | { type: "drop"; paths: string[] }
  | { type: "leave" };

/** The state of the import: one run at a time, then its Import Report. */
export function useImport(importPaths: Importer) {
  const running = ref(false);
  const dragging = ref(false);
  const report = ref<ImportReport | null>(null);
  const error = ref<string | null>(null);

  /** Imports `paths`, unless there are none or a run is going on. */
  async function run(paths: string[]): Promise<void> {
    if (running.value || paths.length === 0) return;
    running.value = true;
    report.value = null;
    error.value = null;
    try {
      report.value = await importPaths(paths);
    } catch (e) {
      error.value = String(e);
    } finally {
      running.value = false;
    }
  }

  /** Imports what the Athlete picks with `picker`. */
  async function pick(picker: Picker): Promise<void> {
    if (running.value) return;
    let paths: string[];
    try {
      paths = await picker();
    } catch (e) {
      error.value = String(e);
      return;
    }
    await run(paths);
  }

  /** Follows a drag over the window and imports what is dropped on it. */
  function onDragDrop(event: DragDrop): void {
    if (event.type === "drop") {
      dragging.value = false;
      void run(event.paths);
    } else {
      dragging.value = event.type !== "leave" && !running.value;
    }
  }

  /** Closes the Import Report. */
  function dismiss(): void {
    report.value = null;
  }

  return { running, dragging, report, error, run, pick, onDragDrop, dismiss };
}
