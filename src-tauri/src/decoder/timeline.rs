//! Places records on a regular one-second timeline and decides which holes
//! between them are Gaps (ADR 0002). Knows nothing about FIT.

use std::ops::Range;

use chrono::{DateTime, Utc};

/// The values for one second of an Activity's timeline. A channel the device
/// did not record is `None`, never zero.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Sample {
    pub power_w: Option<u16>,
    pub heart_rate_bpm: Option<u8>,
    pub cadence_rpm: Option<u8>,
    pub altitude_m: Option<f64>,
    pub speed_m_s: Option<f64>,
    pub distance_m: Option<f64>,
}

/// An Activity's Samples, one per second.
#[derive(Debug, Clone, PartialEq)]
pub struct Timeline {
    /// The first record's instant: the start of the Activity.
    pub start_utc: DateTime<Utc>,
    /// One entry per second from the first record to the last, both included.
    /// `None` is a second of a Gap.
    pub seconds: Vec<Option<Sample>>,
}

/// A timer event, as the device recorded it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Timer {
    Start,
    Stop,
}

/// Longest hole, in missing seconds, filled rather than left as a Gap while
/// the timer runs: smart recording writes a point every 1 to 8 s, so a longer
/// hole is a recording dropout. A decoding parameter, not a domain constant.
const MAX_FILLED_HOLE_S: i64 = 10;

/// Longest timeline accepted, in seconds. A larger span comes from a broken
/// clock and would allocate gigabytes.
pub(super) const MAX_TIMELINE_S: i64 = 7 * 24 * 3600;

/// Why records cannot make a timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TimelineError {
    NoRecord,
    TooLong,
}

/// Builds the timeline of `records`, in any order. Holes inside a timer pause,
/// or longer than [`MAX_FILLED_HOLE_S`], are Gaps; shorter ones are filled.
pub(super) fn resample(
    mut records: Vec<(DateTime<Utc>, Sample)>,
    timer: &[(DateTime<Utc>, Timer)],
) -> Result<Timeline, TimelineError> {
    // A stable sort: records sharing a timestamp keep their file order.
    records.sort_by_key(|(at, _)| *at);
    let mut points: Vec<(i64, Sample)> = Vec::new();
    for (at, sample) in records {
        let t = at.timestamp();
        match points.last_mut() {
            Some((last_t, last)) if *last_t == t => *last = merge(last, &sample),
            _ => points.push((t, sample)),
        }
    }
    let (Some(&(first, _)), Some(&(last, _))) = (points.first(), points.last()) else {
        return Err(TimelineError::NoRecord);
    };
    if last - first > MAX_TIMELINE_S {
        return Err(TimelineError::TooLong);
    }

    let pauses = pauses(timer);
    let mut seconds = vec![None; (last - first + 1) as usize];
    for pair in points.windows(2) {
        let [(ta, a), (tb, b)] = [pair[0], pair[1]];
        let hole = ta + 1..tb;
        let in_pause = pauses
            .iter()
            .any(|p| p.start < hole.end && hole.start < p.end);
        if in_pause || tb - ta - 1 > MAX_FILLED_HOLE_S {
            continue;
        }
        for t in hole {
            seconds[(t - first) as usize] = Some(fill(&a, &b, t - ta, tb - ta));
        }
    }
    for (t, sample) in points {
        seconds[(t - first) as usize] = Some(sample);
    }
    Ok(Timeline {
        start_utc: DateTime::from_timestamp(first, 0).expect("a timestamp chrono can hold"),
        seconds,
    })
}

/// Two records of the same second: the later one wins where it has a value.
fn merge(earlier: &Sample, later: &Sample) -> Sample {
    Sample {
        power_w: later.power_w.or(earlier.power_w),
        heart_rate_bpm: later.heart_rate_bpm.or(earlier.heart_rate_bpm),
        cadence_rpm: later.cadence_rpm.or(earlier.cadence_rpm),
        altitude_m: later.altitude_m.or(earlier.altitude_m),
        speed_m_s: later.speed_m_s.or(earlier.speed_m_s),
        distance_m: later.distance_m.or(earlier.distance_m),
    }
}

/// The seconds the timer was stopped, as `stop..start` ranges, from events in
/// file order. A stop never followed by a start lasts until the end.
fn pauses(timer: &[(DateTime<Utc>, Timer)]) -> Vec<Range<i64>> {
    let mut pauses = Vec::new();
    let mut stopped_at = None;
    for &(at, event) in timer {
        match (event, stopped_at) {
            (Timer::Stop, None) => stopped_at = Some(at.timestamp()),
            (Timer::Start, Some(stop)) => {
                pauses.push(stop..at.timestamp());
                stopped_at = None;
            }
            _ => {}
        }
    }
    if let Some(stop) = stopped_at {
        pauses.push(stop..i64::MAX);
    }
    pauses
}

/// The Sample `k` seconds into a hole of `span` seconds from `a` to `b`:
/// power, heart rate and cadence repeat `a`; altitude, speed and distance are
/// interpolated linearly, when both ends have a value.
fn fill(a: &Sample, b: &Sample, k: i64, span: i64) -> Sample {
    let lerp = |from: Option<f64>, to: Option<f64>| {
        let (from, to) = (from?, to?);
        Some(from + (to - from) * k as f64 / span as f64)
    };
    Sample {
        power_w: a.power_w,
        heart_rate_bpm: a.heart_rate_bpm,
        cadence_rpm: a.cadence_rpm,
        altitude_m: lerp(a.altitude_m, b.altitude_m),
        speed_m_s: lerp(a.speed_m_s, b.speed_m_s),
        distance_m: lerp(a.distance_m, b.distance_m),
    }
}
