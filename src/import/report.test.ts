import { describe, expect, it } from "vitest";
import { countOutcomes, fileName, listedFiles, reportNote, type FileReport } from "./report";

const imported: FileReport = {
  path: "/rides/a.fit",
  outcome: { kind: "imported", activityId: 3, overlaps: [] },
};
const duplicate: FileReport = { path: "/rides/b.fit", outcome: { kind: "duplicate", activityId: 1 } };
const notCycling: FileReport = { path: "/rides/c.fit", outcome: { kind: "notCycling" } };
const unreadable: FileReport = {
  path: "/rides/d.fit",
  outcome: { kind: "unreadable", reason: "garbled" },
};
const failed: FileReport = { path: "/rides/e.fit", outcome: { kind: "failed", reason: "disk full" } };

describe("countOutcomes", () => {
  it("counts each outcome, imported first and failed last", () => {
    const files = [failed, duplicate, imported, unreadable, duplicate, notCycling, imported, imported];

    expect(countOutcomes({ files })).toEqual([
      { kind: "imported", count: 3 },
      { kind: "duplicate", count: 2 },
      { kind: "notCycling", count: 1 },
      { kind: "unreadable", count: 1 },
      { kind: "failed", count: 1 },
    ]);
  });

  it("leaves out the outcomes no file had", () => {
    expect(countOutcomes({ files: [duplicate, unreadable, duplicate] })).toEqual([
      { kind: "duplicate", count: 2 },
      { kind: "unreadable", count: 1 },
    ]);
  });

  it("counts nothing when the run met no file", () => {
    expect(countOutcomes({ files: [] })).toEqual([]);
  });
});

describe("listedFiles", () => {
  it("lists every file but the Duplicates, in the order of the run", () => {
    const files = [unreadable, duplicate, imported, notCycling, duplicate, failed];

    expect(listedFiles({ files })).toEqual([unreadable, imported, notCycling, failed]);
  });
});

describe("reportNote", () => {
  it("says no file was found when the run met none", () => {
    expect(reportNote({ files: [] })).toBe("nothingFound");
  });

  it("says every file was already imported when the run met only Duplicates", () => {
    expect(reportNote({ files: [duplicate, duplicate] })).toBe("allDuplicates");
  });

  it("says nothing when the report lists a file", () => {
    expect(reportNote({ files: [duplicate, notCycling] })).toBeUndefined();
  });
});

describe("fileName", () => {
  it("keeps the last segment of a Unix path", () => {
    expect(fileName("/home/athlete/rides/morning.fit")).toBe("morning.fit");
  });

  it("keeps the last segment of a Windows path", () => {
    expect(fileName("C:\\Users\\athlete\\rides\\morning.FIT")).toBe("morning.FIT");
  });

  it("ignores a trailing separator", () => {
    expect(fileName("/home/athlete/rides/")).toBe("rides");
  });

  it("keeps a bare name as it is", () => {
    expect(fileName("morning.fit")).toBe("morning.fit");
  });

  it("keeps a path without any name as it is", () => {
    expect(fileName("/")).toBe("/");
  });
});
