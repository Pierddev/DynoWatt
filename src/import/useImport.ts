import { ref } from "vue";
import type { ImportReport } from "./report";

/** Imports the Source Files at `paths`, files or folders, into the Library. */
export type Importer = (paths: string[]) => Promise<ImportReport>;

/** Asks the Athlete for paths to import; none when they cancel. */
export type Picker = () => Promise<string[]>;

/** The state of the import: one run at a time, then its Import Report. */
export function useImport(importPaths: Importer) {
  const running = ref(false);
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

  /** Closes the Import Report. */
  function dismiss(): void {
    report.value = null;
  }

  return { running, report, error, run, pick, dismiss };
}
