/**
 * One Activity as the `list_activities` command returns it, with its Activity
 * Summary (see `activities::ListedActivity` in Rust). A missing metric is
 * `null`, never `0`.
 */
export interface ListedActivity {
  id: number;
  /** The wall-clock time where the ride started, `YYYY-MM-DDTHH:MM:SS`, no offset. */
  startLocal: string;
  isIndoor: boolean;
  movingTimeS: number;
  distanceM: number | null;
  elevationGainM: number | null;
  averagePowerW: number | null;
  averageHrBpm: number | null;
}

/** One line of the activity list, every figure ready to show. */
export interface ActivityRow {
  id: number;
  date: string;
  time: string;
  isIndoor: boolean;
  type: string;
  movingTime: string;
  distance: string;
  elevationGain: string;
  averagePower: string;
  averageHeartRate: string;
}

/** The vue-i18n functions a row is formatted with: `useI18n()` or `i18n.global`. */
export interface Formatter {
  t(key: string): string;
  t(key: string, named: Record<string, unknown>): string;
  d(value: Date, key: string): string;
  n(value: number, key: string): string;
}

/** Formats `activity` for the list, in metric units, "—" for a missing figure. */
export function toRow(activity: ListedActivity, { t, d, n }: Formatter): ActivityRow {
  // Read as UTC and formatted in UTC (see the `date` and `time` formats), the
  // wall-clock time comes out unchanged.
  const start = new Date(`${activity.startLocal}Z`);
  const figure = (value: number | null, unit: string, format: string) =>
    value === null ? t("activities.missing") : t(`activities.unit.${unit}`, { value: n(value, format) });
  return {
    id: activity.id,
    date: d(start, "date"),
    time: d(start, "time"),
    isIndoor: activity.isIndoor,
    type: t(activity.isIndoor ? "activities.indoor" : "activities.road"),
    movingTime: formatDuration(activity.movingTimeS),
    distance: figure(activity.distanceM === null ? null : activity.distanceM / 1000, "km", "km"),
    elevationGain: figure(activity.elevationGainM, "m", "whole"),
    averagePower: figure(activity.averagePowerW, "watts", "whole"),
    averageHeartRate: figure(activity.averageHrBpm, "bpm", "whole"),
  };
}

/** `seconds` as `h:mm:ss`, hours unpadded: `0:45:07`, `12:03:00`. */
export function formatDuration(seconds: number): string {
  const pad = (value: number) => String(value).padStart(2, "0");
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  return `${hours}:${pad(minutes)}:${pad(seconds % 60)}`;
}
