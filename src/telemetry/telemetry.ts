import { formatDuration, type Formatter } from "../activities/list";

/**
 * The Samples of an Activity as the `activity_telemetry` command returns them
 * (see `telemetry::Telemetry` in Rust): one column per channel along the
 * one-second timeline (ADR 0002). `null` in a column is a Gap, or a second
 * the sensor did not record; a whole column is `null` when the device had no
 * such sensor.
 */
export interface Telemetry {
  /** Seconds since the first Sample, Gaps included: 0, 1, 2, … */
  timeS: number[];
  powerW: (number | null)[] | null;
  heartRateBpm: (number | null)[] | null;
  cadenceRpm: (number | null)[] | null;
  altitudeM: (number | null)[] | null;
  speedMS: (number | null)[] | null;
}

export type ChannelKey = "power" | "heartRate" | "cadence" | "altitude" | "speed";

/** One chart of the activity detail: a channel's values along `timeS`, in the unit shown. */
export interface Chart {
  key: ChannelKey;
  values: (number | null)[];
  /** Whether the value axis starts at zero rather than fitting the values shown. */
  fromZero: boolean;
}

const KMH_PER_M_S = 3.6;

/** Each channel, in the order the charts are stacked, with the unit and number format of its values. */
const CHANNELS: {
  key: ChannelKey;
  values: (telemetry: Telemetry) => (number | null)[] | null;
  fromZero: boolean;
  unit: string;
  format: string;
}[] = [
  { key: "power", values: (t) => t.powerW, fromZero: true, unit: "watts", format: "whole" },
  { key: "heartRate", values: (t) => t.heartRateBpm, fromZero: false, unit: "bpm", format: "whole" },
  { key: "cadence", values: (t) => t.cadenceRpm, fromZero: true, unit: "rpm", format: "whole" },
  { key: "altitude", values: (t) => t.altitudeM, fromZero: false, unit: "m", format: "whole" },
  {
    key: "speed",
    values: (t) => t.speedMS?.map((v) => (v === null ? null : v * KMH_PER_M_S)) ?? null,
    fromZero: true,
    unit: "kmh",
    format: "tenths",
  },
];

/**
 * The charts `telemetry` draws, power first. A channel the device had no
 * sensor for has no chart, rather than a flat zero line; speed is in km/h.
 */
export function chartsOf(telemetry: Telemetry): Chart[] {
  return CHANNELS.flatMap(({ key, values, fromZero }) => {
    const column = values(telemetry);
    return column === null ? [] : [{ key, values: column, fromZero }];
  });
}

/** A value of the channel `key` as the readout shows it: `245 W`, `32.4 km/h`, "—" when missing. */
export function formatValue(key: ChannelKey, value: number | null, { t, n }: Formatter): string {
  if (value === null) return t("activities.missing");
  const { unit, format } = CHANNELS.find((channel) => channel.key === key)!;
  return t(`activities.unit.${unit}`, { value: n(value, format) });
}

/**
 * The time axis label at `seconds` when ticks are `step` seconds apart:
 * `h:mm`, with the seconds only when ticks fall within a minute.
 */
export function tickLabel(seconds: number, step: number): string {
  const label = formatDuration(seconds);
  return step < 60 ? label : label.slice(0, -3);
}
