import { describe, expect, it, vi } from "vitest";
import { useDeletion } from "./useDeletion";

/** A deleter whose answers come only when the test says so. */
function pendingDeleter() {
  const calls: { resolve: () => void; reject: (reason: unknown) => void }[] = [];
  const deleter = vi.fn((_id: number) => new Promise<void>((resolve, reject) => calls.push({ resolve, reject })));
  return { deleter, calls };
}

describe("useDeletion", () => {
  it("is idle before any deletion", () => {
    const { deleting, error } = useDeletion(vi.fn(async () => {}));

    expect(deleting.value).toBe(false);
    expect(error.value).toBeNull();
  });

  it("deletes the Activity", async () => {
    const deleter = vi.fn(async (_id: number) => {});
    const { deleting, error, remove } = useDeletion(deleter);

    expect(await remove(7)).toBe(true);

    expect(deleter).toHaveBeenCalledWith(7);
    expect(deleting.value).toBe(false);
    expect(error.value).toBeNull();
  });

  it("is deleting until the command answers", async () => {
    const { deleter, calls } = pendingDeleter();
    const { deleting, remove } = useDeletion(deleter);

    const removed = remove(7);

    expect(deleting.value).toBe(true);
    calls[0].resolve();
    await removed;
    expect(deleting.value).toBe(false);
  });

  it("keeps the reason of a failed deletion", async () => {
    const { deleting, error, remove } = useDeletion(async () => {
      throw "cannot remove 2026/ride.fit: Permission denied";
    });

    expect(await remove(7)).toBe(false);

    expect(error.value).toBe("cannot remove 2026/ride.fit: Permission denied");
    expect(deleting.value).toBe(false);
  });

  it("ignores a deletion asked while another is going on", async () => {
    const { deleter, calls } = pendingDeleter();
    const { remove } = useDeletion(deleter);

    const first = remove(7);
    expect(await remove(8)).toBe(false);

    expect(deleter).toHaveBeenCalledTimes(1);
    calls[0].resolve();
    expect(await first).toBe(true);
  });

  it("forgets the last failure on a new attempt", async () => {
    const { deleter, calls } = pendingDeleter();
    const { error, remove } = useDeletion(deleter);
    const failed = remove(7);
    calls[0].reject("database is locked");
    await failed;

    const retried = remove(7);

    expect(error.value).toBeNull();
    calls[1].resolve();
    expect(await retried).toBe(true);
  });

  it("forgets the last failure once dismissed", async () => {
    const { error, remove, dismiss } = useDeletion(async () => {
      throw "database is locked";
    });
    await remove(7);

    dismiss();

    expect(error.value).toBeNull();
  });
});
