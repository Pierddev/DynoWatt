//! The activity list: each Activity with its Activity Summary, as the list
//! view shows it.

#[cfg(test)]
mod tests;

use chrono::TimeDelta;
use rusqlite::Connection;
use serde::Serialize;

use crate::db::{self, Activity, AthleteId};

/// One Activity of the list, with its Activity Summary.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListedActivity {
    pub id: i64,
    /// The wall-clock time where the ride started, `YYYY-MM-DDTHH:MM:SS` with
    /// no offset: a ride started at 08:00 in another time zone reads 08:00
    /// wherever the list is shown.
    pub start_local: String,
    pub is_indoor: bool,
    pub moving_time_s: u32,
    pub distance_m: Option<f64>,
    pub elevation_gain_m: Option<f64>,
    /// `None` when the ride had no power meter, never `0`.
    pub average_power_w: Option<f64>,
    pub average_hr_bpm: Option<f64>,
}

/// The Athlete's Activities, newest first.
pub fn list(conn: &Connection, athlete_id: AthleteId) -> rusqlite::Result<Vec<ListedActivity>> {
    Ok(db::list_activities(conn, athlete_id)?
        .into_iter()
        .map(listed)
        .collect())
}

fn listed(activity: Activity) -> ListedActivity {
    let record = activity.record;
    let start_local = record.start_utc.naive_utc() + TimeDelta::seconds(record.utc_offset_s.into());
    ListedActivity {
        id: activity.id.0,
        start_local: start_local.format("%Y-%m-%dT%H:%M:%S").to_string(),
        is_indoor: record.is_indoor,
        moving_time_s: record.moving_time_s,
        distance_m: record.distance_m,
        elevation_gain_m: record.elevation_gain_m,
        average_power_w: record.average_power_w,
        average_hr_bpm: record.average_hr_bpm,
    }
}
