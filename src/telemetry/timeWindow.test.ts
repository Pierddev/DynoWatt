import { describe, expect, it } from "vitest";
import { dragSeconds, isWhole, NARROWEST_S, pan, wheelFactor, whole, zoom } from "./timeWindow";

describe("whole", () => {
  it("spans the timeline from the first Sample to the last", () => {
    expect(whole(18_000)).toEqual({ min: 0, max: 18_000 });
  });
});

describe("isWhole", () => {
  it("tells the whole timeline from a part of it", () => {
    expect(isWhole({ min: 0, max: 1_000 }, 1_000)).toBe(true);
    expect(isWhole({ min: 0, max: 999 }, 1_000)).toBe(false);
    expect(isWhole({ min: 1, max: 1_000 }, 1_000)).toBe(false);
  });
});

describe("zoom", () => {
  it("zooms in around the focus, which keeps its place", () => {
    expect(zoom({ min: 0, max: 1_000 }, 250, 0.5, 1_000)).toEqual({ min: 125, max: 625 });
  });

  it("zooms out around the focus", () => {
    expect(zoom({ min: 300, max: 500 }, 450, 2, 1_000)).toEqual({ min: 150, max: 550 });
  });

  it("stops at the start of the timeline", () => {
    expect(zoom({ min: 0, max: 200 }, 50, 2, 1_000)).toEqual({ min: 0, max: 400 });
  });

  it("stops at the end of the timeline", () => {
    expect(zoom({ min: 800, max: 1_000 }, 950, 2, 1_000)).toEqual({ min: 600, max: 1_000 });
  });

  it("never zooms out past the whole timeline", () => {
    expect(zoom({ min: 100, max: 900 }, 500, 4, 1_000)).toEqual({ min: 0, max: 1_000 });
  });

  it(`never zooms in to less than ${NARROWEST_S} seconds`, () => {
    expect(zoom({ min: 100, max: 140 }, 120, 0.5, 1_000)).toEqual({ min: 105, max: 135 });
  });

  it(`shows the whole of a ride shorter than ${NARROWEST_S} seconds`, () => {
    expect(zoom({ min: 0, max: 20 }, 10, 0.5, 20)).toEqual({ min: 0, max: 20 });
  });

  it("keeps a ride of a single Sample at its only second", () => {
    expect(zoom({ min: 0, max: 0 }, 0, 0.5, 0)).toEqual({ min: 0, max: 0 });
  });
});

describe("pan", () => {
  it("moves the window later or earlier, keeping its width", () => {
    expect(pan({ min: 100, max: 200 }, 50, 1_000)).toEqual({ min: 150, max: 250 });
    expect(pan({ min: 100, max: 200 }, -50, 1_000)).toEqual({ min: 50, max: 150 });
  });

  it("stops at the start of the timeline", () => {
    expect(pan({ min: 100, max: 200 }, -150, 1_000)).toEqual({ min: 0, max: 100 });
  });

  it("stops at the end of the timeline", () => {
    expect(pan({ min: 850, max: 950 }, 100, 1_000)).toEqual({ min: 900, max: 1_000 });
  });

  it("cannot move the whole timeline", () => {
    expect(pan({ min: 0, max: 1_000 }, 50, 1_000)).toEqual({ min: 0, max: 1_000 });
  });
});

describe("dragSeconds", () => {
  it("brings earlier seconds into view when dragged right, later ones when dragged left", () => {
    expect(dragSeconds(100, { min: 0, max: 600 }, 1_000)).toBe(-60);
    expect(dragSeconds(-100, { min: 600, max: 1_200 }, 1_000)).toBe(60);
  });
});

describe("wheelFactor", () => {
  it("zooms out when turned down and in when turned up, 300 px doubling the window", () => {
    expect(wheelFactor(0)).toBe(1);
    expect(wheelFactor(300)).toBe(2);
    expect(wheelFactor(-300)).toBe(0.5);
    expect(wheelFactor(100)).toBeCloseTo(1.26, 2);
  });
});
