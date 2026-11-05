import { describe, expect, it, vi } from "vitest";
import type { Telemetry } from "./telemetry";
import { useTelemetry } from "./useTelemetry";

function telemetry(powerW: number): Telemetry {
  return {
    timeS: [0, 1],
    powerW: [powerW, null],
    heartRateBpm: null,
    cadenceRpm: null,
    altitudeM: null,
    speedMS: null,
  };
}

/** A loader whose answers come only when the test says so, in any order. */
function pendingLoader() {
  const calls: { resolve: (telemetry: Telemetry) => void; reject: (reason: unknown) => void }[] = [];
  const loader = vi.fn(
    (_id: number) => new Promise<Telemetry>((resolve, reject) => calls.push({ resolve, reject })),
  );
  return { loader, calls };
}

describe("useTelemetry", () => {
  it("has no telemetry before it loads", () => {
    const { telemetry: shown, error } = useTelemetry(vi.fn(async () => telemetry(200)));

    expect(shown.value).toBeNull();
    expect(error.value).toBeNull();
  });

  it("loads the telemetry of the Activity", async () => {
    const loader = vi.fn(async (_id: number) => telemetry(200));
    const { telemetry: shown, error, load } = useTelemetry(loader);

    await load(7);

    expect(loader).toHaveBeenCalledWith(7);
    expect(shown.value).toEqual(telemetry(200));
    expect(error.value).toBeNull();
  });

  it("keeps the reason a load failed", async () => {
    const { telemetry: shown, error, load } = useTelemetry(async () => {
      throw "the Source File is missing";
    });

    await load(7);

    expect(error.value).toBe("the Source File is missing");
    expect(shown.value).toBeNull();
  });

  it("forgets the last telemetry while the next one loads", async () => {
    const { loader, calls } = pendingLoader();
    const { telemetry: shown, load } = useTelemetry(loader);
    const first = load(7);
    calls[0].resolve(telemetry(200));
    await first;

    void load(8);

    expect(shown.value).toBeNull();
  });

  it("forgets a failure while the next load runs", async () => {
    const { loader, calls } = pendingLoader();
    const { error, load } = useTelemetry(loader);
    const first = load(7);
    calls[0].reject("database locked");
    await first;

    void load(7);

    expect(error.value).toBeNull();
  });

  it("ignores an earlier load that answers last", async () => {
    const { loader, calls } = pendingLoader();
    const { telemetry: shown, error, load } = useTelemetry(loader);
    const first = load(7);
    const second = load(8);

    calls[1].resolve(telemetry(250));
    await second;
    calls[0].resolve(telemetry(200));
    await first;

    expect(shown.value).toEqual(telemetry(250));
    expect(error.value).toBeNull();
  });

  it("ignores the failure of an earlier load that answers last", async () => {
    const { loader, calls } = pendingLoader();
    const { telemetry: shown, error, load } = useTelemetry(loader);
    const first = load(7);
    const second = load(8);

    calls[1].resolve(telemetry(250));
    await second;
    calls[0].reject("database locked");
    await first;

    expect(shown.value).toEqual(telemetry(250));
    expect(error.value).toBeNull();
  });
});
