# Samples are resampled onto a one-second timeline, and Gaps stay explicit

Devices do not reliably record at 1 Hz: "smart recording" writes a point every 1–8 s, and auto-pause leaves holes. When a Source File is decoded, its points are placed on a regular one-second timeline, and seconds with no data are kept as Gaps (missing values), never filled with zeros or interpolated across a pause. A fixed 1 s step makes MMP, W'bal and synchronised charts simple index arithmetic, while explicit Gaps defer the "what does a pause mean?" question to each calculation: W'bal should recover during a café stop, but MMP must not treat it as a zero-watt effort.

## Consequences

- Every downstream calculation must handle Gaps deliberately; none may assume a contiguous series.
- Short irregular intervals inside a moving stretch (smart recording) need a fill rule distinct from real Gaps; the threshold separating the two is a decoding parameter, not a domain constant.
