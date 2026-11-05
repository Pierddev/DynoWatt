import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { ImportReport } from "./report";

/** Imports the Source Files at `paths`, files or folders, and returns the Import Report. */
export function importPaths(paths: string[]): Promise<ImportReport> {
  return invoke<ImportReport>("import_paths", { paths });
}

/** Asks the Athlete for .FIT files; none when they cancel. */
export async function pickFiles(filterName: string): Promise<string[]> {
  const paths = await open({
    multiple: true,
    // Both cases: the Linux file pickers match extensions case-sensitively.
    filters: [{ name: filterName, extensions: ["fit", "FIT"] }],
  });
  return paths ?? [];
}

/** Asks the Athlete for folders to scan for .FIT files; none when they cancel. */
export async function pickFolders(): Promise<string[]> {
  return (await open({ multiple: true, directory: true })) ?? [];
}
