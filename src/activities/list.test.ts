import { beforeAll, describe, expect, it, vi } from "vitest";
import { i18n } from "../i18n";
import { formatDuration, toRow, type ListedActivity } from "./list";

const ride: ListedActivity = {
  id: 7,
  startLocal: "2026-03-14T08:00:00",
  isIndoor: false,
  movingTimeS: 5025,
  distanceM: 42_195,
  elevationGainM: 512.4,
  averagePowerW: 215.6,
  averageHrBpm: 141.5,
};

const row = (activity: ListedActivity) => toRow(activity, i18n.global);

describe("toRow", () => {
  // UTC+14: a start time read in the machine's time zone would move by
  // fourteen hours, and to the next day late in the evening.
  beforeAll(() => {
    vi.stubEnv("TZ", "Pacific/Kiritimati");
  });

  it("formats each figure of the Activity Summary in metric units", () => {
    expect(row(ride)).toEqual({
      id: 7,
      date: "Sat, Mar 14, 2026",
      time: "08:00",
      isIndoor: false,
      type: "Road",
      movingTime: "1:23:45",
      distance: "42.2 km",
      elevationGain: "512 m",
      averagePower: "216 W",
      averageHeartRate: "142 bpm",
    });
  });

  it("shows a ride recorded at 08:00 in another time zone at 08:00", () => {
    // The machine really is in another time zone.
    expect(new Date("2026-03-14T08:00:00Z").getHours()).not.toBe(8);

    expect(row(ride).time).toBe("08:00");
  });

  it("keeps a late ride on its own day", () => {
    const late = row({ ...ride, startLocal: "2026-03-14T23:30:59" });

    expect([late.date, late.time]).toEqual(["Sat, Mar 14, 2026", "23:30"]);
  });

  it("names an Indoor Activity", () => {
    expect(row({ ...ride, isIndoor: true })).toMatchObject({ isIndoor: true, type: "Indoor" });
  });

  it("shows a missing figure as a dash", () => {
    const bare = row({
      ...ride,
      distanceM: null,
      elevationGainM: null,
      averagePowerW: null,
      averageHrBpm: null,
    });

    expect([bare.distance, bare.elevationGain, bare.averagePower, bare.averageHeartRate]).toEqual([
      "—",
      "—",
      "—",
      "—",
    ]);
  });

  it("shows a zero figure as zero, not as missing", () => {
    const still = row({ ...ride, distanceM: 0, elevationGainM: 0, averagePowerW: 0, averageHrBpm: 0 });

    expect([still.distance, still.elevationGain, still.averagePower, still.averageHeartRate]).toEqual([
      "0.0 km",
      "0 m",
      "0 W",
      "0 bpm",
    ]);
  });

  it("rounds the distance to a tenth of a kilometre and groups thousands", () => {
    expect(row({ ...ride, distanceM: 30_549 }).distance).toBe("30.5 km");
    expect(row({ ...ride, distanceM: 1_234_567 }).distance).toBe("1,234.6 km");
    expect(row({ ...ride, elevationGainM: 2_345.5 }).elevationGain).toBe("2,346 m");
  });
});

describe("formatDuration", () => {
  it.each([
    [0, "0:00:00"],
    [59, "0:00:59"],
    [60, "0:01:00"],
    [2_707, "0:45:07"],
    [3_600, "1:00:00"],
    [43_380, "12:03:00"],
  ])("shows %i seconds as %s", (seconds, shown) => {
    expect(formatDuration(seconds)).toBe(shown);
  });
});
