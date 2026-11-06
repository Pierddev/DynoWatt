import { invoke } from "@tauri-apps/api/core";
import { listen, type Event } from "@tauri-apps/api/event";
import { describe, expect, it, vi } from "vitest";
import { deleteActivity, onRecomputeFinished, recomputeStatus } from "./commands";
import type { RecomputeStatus } from "./recompute";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(async () => undefined) }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));

describe("deleteActivity", () => {
  it("names the Activity as the Rust command expects", async () => {
    await deleteActivity(7);

    expect(invoke).toHaveBeenCalledWith("delete_activity", { id: 7 });
  });
});

describe("recomputeStatus", () => {
  it("asks the Rust command", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({ kind: "running" });

    await expect(recomputeStatus()).resolves.toEqual({ kind: "running" });
    expect(invoke).toHaveBeenCalledWith("recompute_status");
  });
});

describe("onRecomputeFinished", () => {
  it("hands over the status the Rust event carries", async () => {
    const stop = () => {};
    vi.mocked(listen).mockResolvedValueOnce(stop);
    const handler = vi.fn();

    await expect(onRecomputeFinished(handler)).resolves.toBe(stop);

    const [name, listener] = vi.mocked(listen).mock.calls[0];
    expect(name).toBe("recompute-finished");
    const payload: RecomputeStatus = { kind: "failed", reason: "the database is unavailable" };
    listener({ event: name, id: 1, payload } as Event<unknown>);
    expect(handler).toHaveBeenCalledWith(payload);
  });
});
