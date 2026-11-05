//! The Activity Summary: the headline figures of an Activity, from its
//! Samples and the totals its device computed.

#[cfg(test)]
mod tests;

use crate::decoder::{DeviceTotals, Timeline};

/// Version of the rules below, stored with each summary. Bump it whenever
/// they change, so summaries computed by older rules can be recomputed from
/// their Source File.
pub const SUMMARY_VERSION: u32 = 1;

/// The headline figures of an Activity. A figure whose sensor was missing is
/// `None`, never `0`: a ride without a power meter has no Average Power.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActivitySummary {
    /// The [`SUMMARY_VERSION`] that computed it.
    pub version: u32,
    /// Elapsed Time, in seconds: every second of the timeline, Gaps included.
    pub elapsed_time_s: u32,
    /// Moving Time, in seconds: Elapsed Time minus the Gaps.
    pub moving_time_s: u32,
    pub distance_m: Option<f64>,
    pub elevation_gain_m: Option<f64>,
    /// Mean power over the Moving Time, zero-watt coasting included.
    pub average_power_w: Option<f64>,
    /// Mean heart rate over the Moving Time.
    pub average_hr_bpm: Option<f64>,
}

/// Computes the Activity Summary of a decoded Activity. Distance and
/// elevation gain are the device's; the averages are recomputed from the
/// Samples, so they stay consistent with the metrics built on them (MMP, NP).
pub fn summarize(timeline: &Timeline, device: &DeviceTotals) -> ActivitySummary {
    let samples = || timeline.seconds.iter().flatten();
    ActivitySummary {
        version: SUMMARY_VERSION,
        elapsed_time_s: seconds(timeline.seconds.len()),
        moving_time_s: seconds(samples().count()),
        distance_m: device.distance_m,
        elevation_gain_m: device.elevation_gain_m,
        // A second without a value is a sensor dropout, left out rather than
        // counted as zero.
        average_power_w: mean(samples().filter_map(|s| s.power_w).map(f64::from)),
        average_hr_bpm: mean(samples().filter_map(|s| s.heart_rate_bpm).map(f64::from)),
    }
}

/// The mean of `values`; `None` when there is none.
fn mean(values: impl Iterator<Item = f64>) -> Option<f64> {
    let (sum, count) = values.fold((0.0, 0_u32), |(sum, count), v| (sum + v, count + 1));
    (count > 0).then(|| sum / f64::from(count))
}

/// A number of seconds on a timeline, which never spans more than a week.
fn seconds(count: usize) -> u32 {
    u32::try_from(count).expect("a timeline of at most a week")
}
