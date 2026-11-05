import { describe, expect, it, vi } from "vitest";
import type { ListedActivity } from "./list";
import { useActivities } from "./useActivities";

function activity(id: number): ListedActivity {
  return {
    id,
    startLocal: "2026-03-14T08:00:00",
    isIndoor: false,
    movingTimeS: 3600,
    distanceM: 30_000,
    elevationGainM: 400,
    averagePowerW: 200,
    averageHrBpm: 140,
  };
}

/** A lister whose answers come only when the test says so, in any order. */
function pendingLister() {
  const calls: { resolve: (list: ListedActivity[]) => void; reject: (reason: unknown) => void }[] = [];
  const lister = vi.fn(
    () => new Promise<ListedActivity[]>((resolve, reject) => calls.push({ resolve, reject })),
  );
  return { lister, calls };
}

describe("useActivities", () => {
  it("has no list before the first load", () => {
    const { activities, error } = useActivities(vi.fn(async () => []));

    expect(activities.value).toBeNull();
    expect(error.value).toBeNull();
  });

  it("loads the Activities", async () => {
    const { activities, error, load } = useActivities(async () => [activity(2), activity(1)]);

    await load();

    expect(activities.value).toEqual([activity(2), activity(1)]);
    expect(error.value).toBeNull();
  });

  it("loads an empty list as empty, not as missing", async () => {
    const { activities, load } = useActivities(async () => []);

    await load();

    expect(activities.value).toEqual([]);
  });

  it("keeps the list shown when a load fails", async () => {
    const lister = vi.fn(async () => [activity(1)]);
    const { activities, error, load } = useActivities(lister);
    await load();

    lister.mockRejectedValueOnce("database locked");
    await load();

    expect(error.value).toBe("database locked");
    expect(activities.value).toEqual([activity(1)]);
  });

  it("forgets a failure once a load succeeds", async () => {
    const lister = vi.fn(async () => [activity(1)]);
    lister.mockRejectedValueOnce("database locked");
    const { activities, error, load } = useActivities(lister);
    await load();

    await load();

    expect(error.value).toBeNull();
    expect(activities.value).toEqual([activity(1)]);
  });

  it("ignores an earlier load that answers last", async () => {
    const { lister, calls } = pendingLister();
    const { activities, error, load } = useActivities(lister);
    const first = load();
    const second = load();

    calls[1].resolve([activity(2), activity(1)]);
    await second;
    calls[0].resolve([activity(1)]);
    await first;

    expect(activities.value).toEqual([activity(2), activity(1)]);
    expect(error.value).toBeNull();
  });

  it("ignores the failure of an earlier load that answers last", async () => {
    const { lister, calls } = pendingLister();
    const { activities, error, load } = useActivities(lister);
    const first = load();
    const second = load();

    calls[1].resolve([activity(1)]);
    await second;
    calls[0].reject("database locked");
    await first;

    expect(activities.value).toEqual([activity(1)]);
    expect(error.value).toBeNull();
  });
});
