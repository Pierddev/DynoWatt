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
    /// In metres.
    pub distance_m: Option<f64>,
    /// In metres.
    pub elevation_gain_m: Option<f64>,
    /// Mean power over the Moving Time, zero-watt coasting included.
    pub average_power_w: Option<f64>,
    /// Mean heart rate over the Moving Time.
    pub average_hr_bpm: Option<f64>,
}

/// Computes the Activity Summary of a decoded Activity. Distance and
/// elevation gain are the device's when it wrote them, else computed from the
/// Samples; the averages are always recomputed from the Samples, so they stay
/// consistent with the metrics built on them (MMP, NP).
pub fn summarize(timeline: &Timeline, device: &DeviceTotals) -> ActivitySummary {
    let samples = || timeline.seconds.iter().flatten();
    ActivitySummary {
        version: SUMMARY_VERSION,
        elapsed_time_s: seconds(timeline.seconds.len()),
        moving_time_s: seconds(samples().count()),
        // The cumulative distance of the last Sample that has one.
        distance_m: device_or(
            device.distance_m,
            samples().filter_map(|s| s.distance_m).next_back(),
        ),
        elevation_gain_m: device_or(
            device.elevation_gain_m,
            elevation_gain(samples().filter_map(|s| s.altitude_m)),
        ),
        // A second without a value is a sensor dropout, left out rather than
        // counted as zero.
        average_power_w: mean(samples().filter_map(|s| s.power_w).map(f64::from)),
        average_hr_bpm: mean(samples().filter_map(|s| s.heart_rate_bpm).map(f64::from)),
    }
}

/// The device's total, else the one computed from the Samples. A device
/// total of zero counts as missing: a trainer without speed sensor writes a
/// zero distance, which must read "—", not "0 km".
fn device_or(device: Option<f64>, computed: Option<f64>) -> Option<f64> {
    device.filter(|&total| total > 0.0).or(computed)
}

/// Rise, in metres, that confirms a climb. Barometric and GPS altitude waver
/// by a metre or so from one second to the next; adding up every rise would
/// count that noise as climbing.
const CLIMB_HYSTERESIS_M: f64 = 3.0;

/// The elevation gain of `altitudes`, in time order: the sum of the climbs.
/// A climb starts once the altitude has risen more than
/// [`CLIMB_HYSTERESIS_M`] above the lowest point before it, and ends once it
/// has dropped more than that below its top. `None` without altitude.
fn elevation_gain(mut altitudes: impl Iterator<Item = f64>) -> Option<f64> {
    // The top of the current climb, or the lowest point since the last one.
    let mut reference = altitudes.next()?;
    let mut climbing = false;
    let mut gain = 0.0;
    for altitude in altitudes {
        if climbing {
            if altitude > reference {
                gain += altitude - reference;
                reference = altitude;
            } else if altitude < reference - CLIMB_HYSTERESIS_M {
                climbing = false;
                reference = altitude;
            }
        } else if altitude < reference {
            reference = altitude;
        } else if altitude > reference + CLIMB_HYSTERESIS_M {
            climbing = true;
            gain += altitude - reference;
            reference = altitude;
        }
    }
    Some(gain)
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
