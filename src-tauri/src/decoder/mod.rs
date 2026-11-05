//! Turns a Source File into DynoWatt's own types. The only module that knows
//! about `fitparser` (ADR 0003): swapping decoder touches nothing else.

#[cfg(test)]
pub(crate) mod fixtures;
#[cfg(test)]
mod tests;
mod timeline;

use std::collections::HashSet;
use std::fmt;
use std::ops::RangeInclusive;

use chrono::{DateTime, Local, NaiveDate, Offset, TimeZone, Utc};
use fitparser::de::{DecodeOption, from_bytes_with_options};
use fitparser::profile::MesgNum;
use fitparser::{FitDataRecord, Value};
use sha2::{Digest, Sha256};

use crate::db::FitFileId;
use timeline::{TimelineError, Timer};

pub use timeline::{Sample, Timeline};

/// What a Source File holds, in DynoWatt's terms.
#[derive(Debug, Clone, PartialEq)]
pub struct DecodedActivity {
    /// First key of Duplicate detection.
    pub fit_file_id: FitFileId,
    /// SHA-256 of the whole file, lowercase hex: the second key of Duplicate
    /// detection.
    pub content_hash: String,
    /// Ridden on a home trainer.
    pub is_indoor: bool,
    /// Offset from UTC, in seconds, of the local time where the ride happened.
    pub utc_offset_s: i32,
    /// What the device computed for the ride; unknown without a `session`
    /// message.
    pub device_totals: DeviceTotals,
    /// The Samples, one per second, with Gaps; starts at the Activity's start.
    pub timeline: Timeline,
}

/// The totals a device computed for the ride and wrote in its `session`
/// message. Each is `None` when the device did not write it.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DeviceTotals {
    /// `total_distance`, in metres.
    pub distance_m: Option<f64>,
    /// `total_ascent`, in metres.
    pub elevation_gain_m: Option<f64>,
}

/// Why a Source File cannot become an Activity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// Not a readable FIT file: failed CRC, truncated or garbled data, no
    /// timestamped record, or records spread over more than a week.
    Unreadable(String),
    /// A readable file recording something other than cycling, or no sport
    /// at all.
    NotCycling,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::Unreadable(reason) => write!(f, "unreadable FIT file: {reason}"),
            DecodeError::NotCycling => f.write_str("not a cycling activity"),
        }
    }
}

impl std::error::Error for DecodeError {}

/// Decodes the bytes of a Source File. When the file does not say where the
/// ride happened, the machine's time zone stands in.
pub fn decode(bytes: &[u8]) -> Result<DecodedActivity, DecodeError> {
    decode_in(bytes, &Local)
}

/// [`decode`], with `machine` as the time zone to fall back on.
fn decode_in(bytes: &[u8], machine: &impl TimeZone) -> Result<DecodedActivity, DecodeError> {
    // Enums come back as their FIT numbers, not as names that may change
    // between profile versions.
    let options = HashSet::from([DecodeOption::ReturnNumericEnumValues]);
    let messages = from_bytes_with_options(bytes, &options)
        .map_err(|e| DecodeError::Unreadable(e.to_string()))?;
    let timeline =
        timeline::resample(records(&messages), &timer_events(&messages)).map_err(|e| {
            DecodeError::Unreadable(match e {
                TimelineError::NoRecord => "no timestamped record".to_owned(),
                TimelineError::TooLong => format!(
                    "records span more than {} days",
                    timeline::MAX_TIMELINE_S / 86_400
                ),
            })
        })?;
    let (sport, sub_sport) = sport(&messages).ok_or(DecodeError::NotCycling)?;
    if sport != CYCLING {
        return Err(DecodeError::NotCycling);
    }
    let utc_offset_s = recorded_utc_offset(&messages).unwrap_or_else(|| {
        machine
            .offset_from_utc_datetime(&timeline.start_utc.naive_utc())
            .fix()
            .local_minus_utc()
    });
    Ok(DecodedActivity {
        fit_file_id: file_id(&messages),
        content_hash: content_hash(bytes),
        is_indoor: sub_sport.is_some_and(|s| INDOOR_SUB_SPORTS.contains(&s)),
        utc_offset_s,
        device_totals: device_totals(&messages),
        timeline,
    })
}

/// The totals of the first session, the one whose sport makes the file a
/// ride.
fn device_totals(messages: &[FitDataRecord]) -> DeviceTotals {
    messages
        .iter()
        .find(|m| m.kind() == MesgNum::Session)
        .map(|m| DeviceTotals {
            distance_m: float(m, "total_distance"),
            elevation_gain_m: float(m, "total_ascent"),
        })
        .unwrap_or_default()
}

/// FIT counts time from 1989-12-31T00:00:00, in UTC for a `date_time` and in
/// local time for a `local_date_time`.
const FIT_EPOCH_UNIX: i64 = 631_065_600;

/// The offsets time zones use, from UTC−12:00 to UTC+14:00.
const UTC_OFFSETS_S: RangeInclusive<i64> = -12 * 3600..=14 * 3600;

/// Where the ride happened, from the `activity` message: its local timestamp
/// minus its UTC one, rounded to the quarter hour all time zones use, since a
/// device may write the two a second apart. `None` when the file does not
/// say, or says something no time zone uses.
fn recorded_utc_offset(messages: &[FitDataRecord]) -> Option<i32> {
    let activity = messages.iter().find(|m| m.kind() == MesgNum::Activity)?;
    let utc = int(activity, "timestamp")? - FIT_EPOCH_UNIX;
    let local = int(activity, "local_timestamp")? - fitparser_local_epoch()?;
    let offset = (local - utc + 450).div_euclid(900) * 900;
    UTC_OFFSETS_S
        .contains(&offset)
        .then(|| i32::try_from(offset).ok())?
}

/// fitparser reads a `local_date_time` as counting from the FIT epoch in the
/// machine's time zone; this is that epoch, as a Unix timestamp.
fn fitparser_local_epoch() -> Option<i64> {
    let epoch = NaiveDate::from_ymd_opt(1989, 12, 31)?.and_hms_opt(0, 0, 0)?;
    Some(Local.from_local_datetime(&epoch).single()?.timestamp())
}

/// The `record` messages that carry a timestamp, with their values.
fn records(messages: &[FitDataRecord]) -> Vec<(DateTime<Utc>, Sample)> {
    messages
        .iter()
        .filter(|m| m.kind() == MesgNum::Record)
        .filter_map(|m| {
            let sample = Sample {
                power_w: int(m, "power").and_then(|v| v.try_into().ok()),
                heart_rate_bpm: int(m, "heart_rate").and_then(|v| v.try_into().ok()),
                cadence_rpm: int(m, "cadence").and_then(|v| v.try_into().ok()),
                // Older devices write 16-bit `altitude` and `speed`, which
                // fitparser expands into the `enhanced_` fields.
                altitude_m: float(m, "enhanced_altitude"),
                speed_m_s: float(m, "enhanced_speed"),
                distance_m: float(m, "distance"),
            };
            Some((instant(m, "timestamp")?, sample))
        })
        .collect()
}

/// FIT `event` number of the timer, and the `event_type` numbers that start
/// it and stop it (stop, stop_all, stop_disable, stop_disable_all).
const TIMER_EVENT: i64 = 0;
const TIMER_START: i64 = 0;
const TIMER_STOPS: [i64; 4] = [1, 4, 8, 9];

/// The timer starts and stops, auto-pause included.
fn timer_events(messages: &[FitDataRecord]) -> Vec<(DateTime<Utc>, Timer)> {
    messages
        .iter()
        .filter(|m| m.kind() == MesgNum::Event && int(m, "event") == Some(TIMER_EVENT))
        .filter_map(|m| {
            let event = match int(m, "event_type")? {
                TIMER_START => Timer::Start,
                t if TIMER_STOPS.contains(&t) => Timer::Stop,
                _ => return None,
            };
            Some((instant(m, "timestamp")?, event))
        })
        .collect()
}

/// FIT `sport` number of cycling, whatever the terrain. `e_biking` is a sport
/// of its own and is rejected.
const CYCLING: i64 = 2;

/// FIT `sub_sport` numbers of rides on a home trainer: spin, indoor_cycling
/// and virtual_activity.
const INDOOR_SUB_SPORTS: [i64; 3] = [5, 6, 58];

/// The sport and sub-sport of the first session, or of the first `sport`
/// message when the file has no session.
fn sport(messages: &[FitDataRecord]) -> Option<(i64, Option<i64>)> {
    [MesgNum::Session, MesgNum::Sport].iter().find_map(|kind| {
        messages
            .iter()
            .filter(|m| m.kind() == *kind)
            .find_map(|m| Some((int(m, "sport")?, int(m, "sub_sport"))))
    })
}

fn content_hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn file_id(messages: &[FitDataRecord]) -> FitFileId {
    messages
        .iter()
        .find(|m| m.kind() == MesgNum::FileId)
        .map(|m| FitFileId {
            manufacturer: int(m, "manufacturer").and_then(|v| v.try_into().ok()),
            serial_number: int(m, "serial_number").and_then(|v| v.try_into().ok()),
            time_created: instant(m, "time_created"),
        })
        .unwrap_or_default()
}

fn value<'a>(message: &'a FitDataRecord, name: &str) -> Option<&'a Value> {
    message
        .fields()
        .iter()
        .find(|f| f.name() == name)
        .map(|f| f.value())
}

fn int(message: &FitDataRecord, name: &str) -> Option<i64> {
    value(message, name).and_then(|v| v.try_into().ok())
}

fn float(message: &FitDataRecord, name: &str) -> Option<f64> {
    value(message, name).and_then(|v| v.clone().try_into().ok())
}

/// A FIT `date_time` field, which counts UTC seconds.
fn instant(message: &FitDataRecord, name: &str) -> Option<DateTime<Utc>> {
    match value(message, name) {
        Some(Value::Timestamp(t)) => Some(t.to_utc()),
        _ => None,
    }
}
