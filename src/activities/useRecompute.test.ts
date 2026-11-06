import { describe, expect, it, vi } from "vitest";
import type { RecomputeStatus } from "./recompute";
import { useRecompute } from "./useRecompute";

const SKIPPED = { activityId: 2, reason: "cannot read 2026/b.fit" };
const DONE: RecomputeStatus = { kind: "done", recomputed: [1], skipped: [SKIPPED] };

/** A recompute whose status the test sets, and whose end it announces. */
function fakeRecompute(status: () => Promise<RecomputeStatus>) {
  const calls: string[] = [];
  const stop = vi.fn();
  let handler: ((status: RecomputeStatus) => void) | undefined;
  const readStatus = vi.fn(() => {
    calls.push("read");
    return status();
  });
  const onFinished = vi.fn(async (onEnd: (status: RecomputeStatus) => void) => {
    calls.push("listen");
    handler = onEnd;
    return stop;
  });
  const recomputed = vi.fn();
  const recompute = useRecompute(readStatus, onFinished, recomputed);
  const finish = (end: RecomputeStatus) => handler?.(end);
  return { ...recompute, calls, stop, recomputed, finish };
}

describe("useRecompute", () => {
  it("knows nothing before it follows the run", () => {
    const { running, skipped, error } = fakeRecompute(async () => DONE);

    expect(running.value).toBe(false);
    expect(skipped.value).toEqual([]);
    expect(error.value).toBeNull();
  });

  it("listens for the end before it asks, and resolves to the way to stop", async () => {
    const { follow, calls, stop } = fakeRecompute(async () => ({ kind: "running" }));

    await expect(follow()).resolves.toBe(stop);
    expect(calls).toEqual(["listen", "read"]);
  });

  it("shows the run going on", async () => {
    const { running, skipped, follow, recomputed } = fakeRecompute(async () => ({ kind: "running" }));

    await follow();

    expect(running.value).toBe(true);
    expect(skipped.value).toEqual([]);
    expect(recomputed).not.toHaveBeenCalled();
  });

  it("takes the end from the event", async () => {
    const { running, skipped, error, follow, finish, recomputed } = fakeRecompute(async () => ({ kind: "running" }));
    await follow();

    finish(DONE);

    expect(running.value).toBe(false);
    expect(skipped.value).toEqual([SKIPPED]);
    expect(error.value).toBeNull();
    expect(recomputed).toHaveBeenCalledOnce();
  });

  it("takes the end from the status when the event came first", async () => {
    const { running, skipped, follow, recomputed } = fakeRecompute(async () => DONE);

    await follow();

    expect(running.value).toBe(false);
    expect(skipped.value).toEqual([SKIPPED]);
    expect(recomputed).toHaveBeenCalledOnce();
  });

  it("takes in the end once when it comes twice", async () => {
    let answer: ((status: RecomputeStatus) => void) | undefined;
    const { running, skipped, follow, finish, recomputed } = fakeRecompute(
      () => new Promise((resolve) => (answer = resolve)),
    );
    const following = follow();
    await vi.waitFor(() => expect(answer).toBeDefined());

    finish(DONE);
    finish({ kind: "done", recomputed: [3], skipped: [] });
    answer?.({ kind: "running" });
    await following;

    expect(running.value).toBe(false);
    expect(skipped.value).toEqual([SKIPPED]);
    expect(recomputed).toHaveBeenCalledOnce();
  });

  it("leaves the list as it is when nothing was recomputed", async () => {
    const { skipped, follow, recomputed } = fakeRecompute(async () => ({
      kind: "done",
      recomputed: [],
      skipped: [SKIPPED],
    }));

    await follow();

    expect(skipped.value).toEqual([SKIPPED]);
    expect(recomputed).not.toHaveBeenCalled();
  });

  it("reports a run that failed", async () => {
    const { running, skipped, error, follow, recomputed } = fakeRecompute(async () => ({
      kind: "failed",
      reason: "the database is unavailable",
    }));

    await follow();

    expect(running.value).toBe(false);
    expect(skipped.value).toEqual([]);
    expect(error.value).toBe("the database is unavailable");
    expect(recomputed).not.toHaveBeenCalled();
  });

  it("reports a status it cannot read", async () => {
    const { running, error, follow } = fakeRecompute(() => Promise.reject("state not managed"));

    await follow();

    expect(running.value).toBe(false);
    expect(error.value).toBe("state not managed");
  });

  it("closes the notice of the skipped Activities", async () => {
    const { skipped, error, follow, dismiss } = fakeRecompute(async () => DONE);
    await follow();

    dismiss();

    expect(skipped.value).toEqual([]);
    expect(error.value).toBeNull();
  });
});
