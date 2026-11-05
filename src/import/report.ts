/**
 * The Import Report as the `import_paths` command returns it: one line per
 * file, in the order the run met them (see `import::ImportReport` in Rust).
 */
export interface ImportReport {
  files: FileReport[];
}

/** What became of one file of the run. */
export interface FileReport {
  /** The file as the Athlete picked or dropped it, or found in a folder. */
  path: string;
  outcome: Outcome;
}

/** The one outcome of a file. Only an imported file reaches the Library. */
export type Outcome =
  /** `overlaps` holds the Library paths of the Activities it overlaps. */
  | { kind: "imported"; activityId: number; overlaps: string[] }
  | { kind: "duplicate"; activityId: number }
  | { kind: "notCycling" }
  | { kind: "unreadable"; reason: string }
  | { kind: "failed"; reason: string };

export type OutcomeKind = Outcome["kind"];

export interface OutcomeCount {
  kind: OutcomeKind;
  count: number;
}

/** The outcomes in the order the report shows their counts. */
const OUTCOME_KINDS: readonly OutcomeKind[] = [
  "imported",
  "duplicate",
  "notCycling",
  "unreadable",
  "failed",
];

/** How many files ended with each outcome, leaving out the outcomes no file had. */
export function countOutcomes(report: ImportReport): OutcomeCount[] {
  return OUTCOME_KINDS.map((kind) => ({
    kind,
    count: report.files.filter((file) => file.outcome.kind === kind).length,
  })).filter(({ count }) => count > 0);
}

/** The files the report lists one by one: Duplicates are only counted. */
export function listedFiles(report: ImportReport): FileReport[] {
  return report.files.filter((file) => file.outcome.kind !== "duplicate");
}

/** Why the report lists no file, or `undefined` when it lists some. */
export function reportNote(report: ImportReport): "nothingFound" | "allDuplicates" | undefined {
  if (report.files.length === 0) return "nothingFound";
  if (listedFiles(report).length === 0) return "allDuplicates";
  return undefined;
}

/** The last segment of `path`, whether it uses `/` or `\` separators. */
export function fileName(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? path;
}
