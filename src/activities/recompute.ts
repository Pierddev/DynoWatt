import type { ListedActivity } from "./list";

/** An Activity the recompute left with its older figures, and why (see `recompute::Skipped` in Rust). */
export interface Skipped {
  activityId: number;
  reason: string;
}

/** Where the recompute of stale summaries started at launch stands (see `recompute::Status` in Rust). */
export type RecomputeStatus =
  | { kind: "running" }
  | { kind: "done"; recomputed: number[]; skipped: Skipped[] }
  | { kind: "failed"; reason: string };

/** A skipped Activity as the list shows it, with the reason it was skipped. */
export interface SkippedActivity {
  activity: ListedActivity;
  reason: string;
}

/** The `skipped` Activities still in `activities`, in the list's order: a deleted one is no news. */
export function stillListed(skipped: Skipped[], activities: ListedActivity[]): SkippedActivity[] {
  const reasons = new Map(skipped.map(({ activityId, reason }) => [activityId, reason]));
  return activities.flatMap((activity) => {
    const reason = reasons.get(activity.id);
    return reason === undefined ? [] : [{ activity, reason }];
  });
}
