import { describe, expect, it } from "vitest";
import type { ListedActivity } from "./list";
import { stillListed } from "./recompute";

function activity(id: number): ListedActivity {
  return {
    id,
    startLocal: "2026-03-14T08:00:00",
    isIndoor: false,
    elapsedTimeS: 3900,
    movingTimeS: 3600,
    distanceM: 30_000,
    elevationGainM: 400,
    averagePowerW: 200,
    averageHrBpm: 140,
  };
}

describe("stillListed", () => {
  it("pairs each skipped Activity with its reason, in the list's order", () => {
    const skipped = [
      { activityId: 1, reason: "cannot read 2026/a.fit" },
      { activityId: 3, reason: "cannot decode 2026/c.fit" },
    ];

    expect(stillListed(skipped, [activity(3), activity(2), activity(1)])).toEqual([
      { activity: activity(3), reason: "cannot decode 2026/c.fit" },
      { activity: activity(1), reason: "cannot read 2026/a.fit" },
    ]);
  });

  it("leaves out a skipped Activity no longer listed", () => {
    const skipped = [
      { activityId: 1, reason: "cannot read 2026/a.fit" },
      { activityId: 9, reason: "cannot read 2026/deleted.fit" },
    ];

    expect(stillListed(skipped, [activity(2), activity(1)])).toEqual([
      { activity: activity(1), reason: "cannot read 2026/a.fit" },
    ]);
  });

  it("keeps an empty reason", () => {
    expect(stillListed([{ activityId: 1, reason: "" }], [activity(1)])).toEqual([{ activity: activity(1), reason: "" }]);
  });

  it("lists nothing when nothing was skipped", () => {
    expect(stillListed([], [activity(1)])).toEqual([]);
  });
});
