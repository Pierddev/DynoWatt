import { invoke } from "@tauri-apps/api/core";
import type { Telemetry } from "./telemetry";

/** The Samples of the Activity `id`, decoded again from its Source File. */
export function activityTelemetry(id: number): Promise<Telemetry> {
  return invoke<Telemetry>("activity_telemetry", { id });
}
