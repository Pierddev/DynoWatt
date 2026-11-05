/** The stretch of the timeline the charts show, in seconds since the first Sample. */
export interface TimeWindow {
  min: number;
  max: number;
}

/** The narrowest window the charts zoom in to, in seconds. */
export const NARROWEST_S = 30;

/** The whole timeline of a ride whose last Sample is at `end` seconds. */
export function whole(end: number): TimeWindow {
  return { min: 0, max: end };
}

/** Whether `window` shows the whole timeline ending at `end`. */
export function isWhole(window: TimeWindow, end: number): boolean {
  return window.min <= 0 && window.max >= end;
}

/**
 * `window` zoomed by `factor` around the second `focus`: narrower below 1,
 * wider above. `focus` keeps its place on screen. The window stays within
 * the timeline ending at `end`, and no narrower than `NARROWEST_S`.
 */
export function zoom(window: TimeWindow, focus: number, factor: number, end: number): TimeWindow {
  const width = window.max - window.min;
  // A ride shorter than `NARROWEST_S` shows whole: `clamp` keeps it to `end`.
  const span = clamp(width * factor, NARROWEST_S, end);
  const ratio = width > 0 ? (focus - window.min) / width : 0;
  return fit(focus - ratio * span, span, end);
}

/** `window` moved `seconds` later (earlier when negative), stopping at either end of the timeline. */
export function pan(window: TimeWindow, seconds: number, end: number): TimeWindow {
  return fit(window.min + seconds, window.max - window.min, end);
}

/**
 * The seconds a drag of `dx` pixels across a plot `widthPx` wide pans
 * `window`: dragging right brings earlier seconds into view.
 */
export function dragSeconds(dx: number, window: TimeWindow, widthPx: number): number {
  return (-dx / widthPx) * (window.max - window.min);
}

/** The zoom factor of a wheel turn of `deltaY` pixels: down zooms out, a 300 px turn doubles the window. */
export function wheelFactor(deltaY: number): number {
  return 2 ** (deltaY / 300);
}

/** The window `span` seconds wide starting at `min`, slid back within the timeline ending at `end`. */
function fit(min: number, span: number, end: number): TimeWindow {
  const start = clamp(min, 0, end - span);
  return { min: start, max: start + span };
}

/** `value` brought within `lowest` and `highest`; `highest` wins when they cross. */
function clamp(value: number, lowest: number, highest: number): number {
  return Math.min(Math.max(value, lowest), highest);
}
