import { describe, expect, it } from "vitest";
import { i18n } from "../i18n";
import { chartsOf, formatValue, tickLabel, type ChannelKey, type Telemetry } from "./telemetry";

const ride: Telemetry = {
  timeS: [0, 1, 2],
  powerW: [210, null, 0],
  heartRateBpm: [140, null, 141],
  cadenceRpm: [90, null, 0],
  altitudeM: [512.3, null, 512.5],
  speedMS: [10, null, 0],
};

describe("chartsOf", () => {
  it("stacks power, heart rate, cadence, altitude and speed", () => {
    expect(chartsOf(ride).map((chart) => chart.key)).toEqual([
      "power",
      "heartRate",
      "cadence",
      "altitude",
      "speed",
    ]);
  });

  it("charts each channel's values, a Gap staying null", () => {
    const [power, heartRate, cadence, altitude] = chartsOf(ride);

    expect(power.values).toEqual([210, null, 0]);
    expect(heartRate.values).toEqual([140, null, 141]);
    expect(cadence.values).toEqual([90, null, 0]);
    expect(altitude.values).toEqual([512.3, null, 512.5]);
  });

  it("charts the speed in km/h", () => {
    const speed = chartsOf(ride).find((chart) => chart.key === "speed")!;

    expect(speed.values).toEqual([36, null, 0]);
  });

  it("has no chart for a channel the device had no sensor for", () => {
    const indoor = { ...ride, heartRateBpm: null, altitudeM: null, speedMS: null };

    expect(chartsOf(indoor).map((chart) => chart.key)).toEqual(["power", "cadence"]);
  });

  it("has no chart at all when no channel was recorded", () => {
    const bare = { ...ride, powerW: null, heartRateBpm: null, cadenceRpm: null, altitudeM: null, speedMS: null };

    expect(chartsOf(bare)).toEqual([]);
  });

  it("still charts a channel recorded as zeros", () => {
    expect(chartsOf({ ...ride, powerW: [0, 0, 0] })[0]).toMatchObject({ key: "power", values: [0, 0, 0] });
  });

  it("starts the power, cadence and speed axes at zero, and fits heart rate and altitude", () => {
    expect(Object.fromEntries(chartsOf(ride).map((chart) => [chart.key, chart.fromZero]))).toEqual({
      power: true,
      heartRate: false,
      cadence: true,
      altitude: false,
      speed: true,
    });
  });
});

describe("formatValue", () => {
  it.each<[ChannelKey, number, string]>([
    ["power", 245.4, "245 W"],
    ["power", 1_234, "1,234 W"],
    ["heartRate", 141.5, "142 bpm"],
    ["cadence", 90, "90 rpm"],
    ["altitude", 512.3, "512 m"],
    ["speed", 32.44, "32.4 km/h"],
    ["speed", 30, "30.0 km/h"],
  ])("shows a %s of %d as %s", (key, value, shown) => {
    expect(formatValue(key, value, i18n.global)).toBe(shown);
  });

  it("shows zero as zero, not as missing", () => {
    expect(formatValue("power", 0, i18n.global)).toBe("0 W");
  });

  it("shows a missing value as a dash", () => {
    expect(formatValue("heartRate", null, i18n.global)).toBe("—");
  });
});

describe("tickLabel", () => {
  it.each([
    [0, 600, "0:00"],
    [3_600, 600, "1:00"],
    [5_400, 60, "1:30"],
    [18_000, 3_600, "5:00"],
    [5_430, 30, "1:30:30"],
    [59, 1, "0:00:59"],
  ])("labels %i seconds as %s when ticks are %i seconds apart", (seconds, step, label) => {
    expect(tickLabel(seconds, step)).toBe(label);
  });
});
