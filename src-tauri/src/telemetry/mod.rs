//! An Activity's Samples as the detail charts read them: one column per
//! channel along the one-second timeline (ADR 0002), re-decoded from the
//! Source File each time the Activity is opened (ADR 0001).

#[cfg(test)]
mod tests;

use std::path::Path;

use rusqlite::Connection;
use serde::Serialize;

use crate::db::{self, ActivityId};
use crate::decoder::{self, Sample, Timeline};

/// The Samples of an Activity, one column per channel. Entry `i` of a column
/// is second `time_s[i]` of the timeline; `None` there is a Gap, or a second
/// the sensor did not record. A whole channel is `None` when no Sample has a
/// value: the device had no such sensor, so its chart is hidden rather than
/// drawn as a flat zero line.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Telemetry {
    /// Seconds since the first Sample, Gaps included: 0, 1, 2, …
    pub time_s: Vec<u32>,
    pub power_w: Option<Vec<Option<u16>>>,
    pub heart_rate_bpm: Option<Vec<Option<u8>>>,
    pub cadence_rpm: Option<Vec<Option<u8>>>,
    /// Rounded to 0.1 m.
    pub altitude_m: Option<Vec<Option<f64>>>,
    /// Rounded to 0.001 m/s.
    pub speed_m_s: Option<Vec<Option<f64>>>,
}

/// The telemetry of the Activity `id`, decoded again from its Source File in
/// `library`. The error says why it cannot be, for the Athlete to read.
pub fn of_activity(conn: &Connection, library: &Path, id: ActivityId) -> Result<Telemetry, String> {
    let activity = db::find_activity(conn, id)
        .map_err(|e| format!("cannot find the Activity: {e}"))?
        .ok_or_else(|| format!("no Activity has the id {}", id.0))?;
    let decoded = decoder::decode_file(&library.join(&activity.record.source_path))?;
    Ok(columns(&decoded.timeline))
}

/// Lays `timeline` out in columns. Altitude and speed are rounded to about
/// the resolution FIT records them at (0.2 m, 0.001 m/s): their interpolated
/// and rescaled values would otherwise print with seventeen digits and
/// double the size of a long ride's JSON.
pub fn columns(timeline: &Timeline) -> Telemetry {
    let seconds = &timeline.seconds;
    Telemetry {
        time_s: (0..).take(seconds.len()).collect(),
        power_w: column(seconds, |s| s.power_w),
        heart_rate_bpm: column(seconds, |s| s.heart_rate_bpm),
        cadence_rpm: column(seconds, |s| s.cadence_rpm),
        altitude_m: column(seconds, |s| s.altitude_m.map(|v| round(v, 10.0))),
        speed_m_s: column(seconds, |s| s.speed_m_s.map(|v| round(v, 1000.0))),
    }
}

/// One channel's value for each second, `None` when no second has one.
fn column<T>(
    seconds: &[Option<Sample>],
    value: impl Fn(&Sample) -> Option<T>,
) -> Option<Vec<Option<T>>> {
    let column: Vec<Option<T>> = seconds
        .iter()
        .map(|second| second.as_ref().and_then(&value))
        .collect();
    column.iter().any(Option::is_some).then_some(column)
}

/// `value` to the nearest `1 / per_unit`: tenths when `per_unit` is 10.
fn round(value: f64, per_unit: f64) -> f64 {
    (value * per_unit).round() / per_unit
}
