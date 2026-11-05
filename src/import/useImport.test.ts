import { describe, expect, it, vi } from "vitest";
import type { ImportReport } from "./report";
import { useImport } from "./useImport";

const report: ImportReport = {
  files: [{ path: "/rides/a.fit", outcome: { kind: "notCycling" } }],
};

/** An importer whose run ends only when the test says so. */
function pendingImporter() {
  let finish: (report: ImportReport) => void = () => {};
  let fail: (reason: unknown) => void = () => {};
  const importer = vi.fn(
    (_paths: string[]) =>
      new Promise<ImportReport>((resolve, reject) => {
        finish = resolve;
        fail = reject;
      }),
  );
  return { importer, finish: (r: ImportReport) => finish(r), fail: (e: unknown) => fail(e) };
}

describe("run", () => {
  it("imports the paths and keeps the Import Report", async () => {
    const importer = vi.fn(async (_paths: string[]) => report);
    const { running, report: shown, error, run } = useImport(importer);

    await run(["/rides/a.fit", "/rides/2026"]);

    expect(importer).toHaveBeenCalledWith(["/rides/a.fit", "/rides/2026"]);
    expect(shown.value).toEqual(report);
    expect(error.value).toBeNull();
    expect(running.value).toBe(false);
  });

  it("is running until the import answers", async () => {
    const { importer, finish } = pendingImporter();
    const { running, run } = useImport(importer);

    const done = run(["/rides/a.fit"]);
    expect(running.value).toBe(true);

    finish(report);
    await done;
    expect(running.value).toBe(false);
  });

  it("does nothing without paths", async () => {
    const importer = vi.fn(async (_paths: string[]) => report);
    const { running, report: shown, run } = useImport(importer);

    await run([]);

    expect(importer).not.toHaveBeenCalled();
    expect(running.value).toBe(false);
    expect(shown.value).toBeNull();
  });

  it("starts no second run while one is going on", async () => {
    const { importer, finish } = pendingImporter();
    const { run } = useImport(importer);

    const first = run(["/rides/a.fit"]);
    await run(["/rides/b.fit"]);
    finish(report);
    await first;

    expect(importer).toHaveBeenCalledTimes(1);
    expect(importer).toHaveBeenCalledWith(["/rides/a.fit"]);
  });

  it("keeps the reason when the import cannot run", async () => {
    const importer = vi.fn(async (_paths: string[]) => {
      throw "the database is unavailable";
    });
    const { running, report: shown, error, run } = useImport(importer);

    await run(["/rides/a.fit"]);

    expect(error.value).toBe("the database is unavailable");
    expect(shown.value).toBeNull();
    expect(running.value).toBe(false);
  });

  it("forgets the previous report and error when a new run starts", async () => {
    const { importer, finish, fail } = pendingImporter();
    const { report: shown, error, run } = useImport(importer);
    const first = run(["/rides/a.fit"]);
    finish(report);
    await first;
    const second = run(["/rides/b.fit"]);
    fail("the database is unavailable");
    await second;

    const third = run(["/rides/c.fit"]);

    expect(shown.value).toBeNull();
    expect(error.value).toBeNull();
    finish(report);
    await third;
  });
});

describe("pick", () => {
  it("imports the paths the Athlete picked", async () => {
    const importer = vi.fn(async (_paths: string[]) => report);
    const { report: shown, pick } = useImport(importer);

    await pick(async () => ["/rides/a.fit"]);

    expect(importer).toHaveBeenCalledWith(["/rides/a.fit"]);
    expect(shown.value).toEqual(report);
  });

  it("does nothing when the Athlete cancels", async () => {
    const importer = vi.fn(async (_paths: string[]) => report);
    const { pick } = useImport(importer);

    await pick(async () => []);

    expect(importer).not.toHaveBeenCalled();
  });

  it("keeps the reason when the picker fails", async () => {
    const importer = vi.fn(async (_paths: string[]) => report);
    const { error, pick } = useImport(importer);

    await pick(async () => {
      throw "no dialog";
    });

    expect(error.value).toBe("no dialog");
    expect(importer).not.toHaveBeenCalled();
  });

  it("opens no picker while a run is going on", async () => {
    const { importer, finish } = pendingImporter();
    const picker = vi.fn(async () => ["/rides/b.fit"]);
    const { run, pick } = useImport(importer);

    const first = run(["/rides/a.fit"]);
    await pick(picker);
    finish(report);
    await first;

    expect(picker).not.toHaveBeenCalled();
  });
});

describe("dismiss", () => {
  it("closes the Import Report", async () => {
    const { report: shown, run, dismiss } = useImport(async () => report);
    await run(["/rides/a.fit"]);

    dismiss();

    expect(shown.value).toBeNull();
  });
});
