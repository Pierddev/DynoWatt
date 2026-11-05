use chrono::{DateTime, Utc};
use serde_json::json;

use super::*;
use crate::db::{ActivityId, ActivityRecord, DEFAULT_ATHLETE, FitFileId, insert_activity};

fn at(text: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(text).unwrap().to_utc()
}

/// A complete record starting at `start_utc`, `utc_offset_s` away from UTC
/// where it was ridden; `n` makes its unique columns unique.
fn record(n: u32, start_utc: &str, utc_offset_s: i32) -> ActivityRecord {
    ActivityRecord {
        athlete_id: DEFAULT_ATHLETE,
        source_path: format!("2026/ride-{n}.fit"),
        content_hash: format!("hash-{n}"),
        fit_file_id: FitFileId {
            manufacturer: Some(1),
            serial_number: Some(1000 + n),
            time_created: Some(at(start_utc)),
        },
        start_utc: at(start_utc),
        utc_offset_s,
        is_indoor: false,
        elapsed_time_s: 3700,
        moving_time_s: 3600,
        distance_m: Some(30_500.5),
        elevation_gain_m: Some(420.0),
        average_power_w: Some(210.5),
        average_hr_bpm: Some(142.25),
        summary_version: 1,
        imported_at: at("2026-11-05T09:30:00Z"),
    }
}

fn listed_one(record: &ActivityRecord) -> ListedActivity {
    let conn = db::open_in_memory().unwrap();
    insert_activity(&conn, record).unwrap();
    let mut listed = list(&conn, DEFAULT_ATHLETE).unwrap();
    assert_eq!(listed.len(), 1);
    listed.remove(0)
}

fn start_local(start_utc: &str, utc_offset_s: i32) -> String {
    listed_one(&record(1, start_utc, utc_offset_s)).start_local
}

#[test]
fn a_ride_started_at_08_00_in_another_time_zone_starts_at_08_00() {
    assert_eq!(
        start_local("2026-03-14T07:00:00Z", 3600),
        "2026-03-14T08:00:00"
    );
    assert_eq!(
        start_local("2026-03-14T13:00:00Z", -5 * 3600),
        "2026-03-14T08:00:00"
    );
    assert_eq!(
        start_local("2026-03-14T08:00:00Z", 0),
        "2026-03-14T08:00:00"
    );
}

#[test]
fn the_local_start_keeps_its_seconds_and_crosses_midnight() {
    assert_eq!(
        start_local("2026-03-14T23:30:15Z", 2 * 3600),
        "2026-03-15T01:30:15"
    );
    assert_eq!(
        start_local("2026-03-15T02:00:59Z", -3 * 3600),
        "2026-03-14T23:00:59"
    );
}

#[test]
fn a_listed_activity_carries_its_activity_summary() {
    let mut indoor = record(1, "2026-03-14T07:00:00Z", 3600);
    indoor.is_indoor = true;
    indoor.elapsed_time_s = 5400;
    indoor.moving_time_s = 5025;
    indoor.distance_m = Some(42_195.0);
    indoor.elevation_gain_m = Some(512.4);
    indoor.average_power_w = Some(215.6);
    indoor.average_hr_bpm = Some(141.5);

    assert_eq!(
        listed_one(&indoor),
        ListedActivity {
            id: 1,
            start_local: "2026-03-14T08:00:00".to_owned(),
            is_indoor: true,
            elapsed_time_s: 5400,
            moving_time_s: 5025,
            distance_m: Some(42_195.0),
            elevation_gain_m: Some(512.4),
            average_power_w: Some(215.6),
            average_hr_bpm: Some(141.5),
        }
    );
}

#[test]
fn missing_metrics_stay_missing() {
    let mut bare = record(1, "2026-03-14T07:00:00Z", 3600);
    bare.distance_m = None;
    bare.elevation_gain_m = None;
    bare.average_power_w = None;
    bare.average_hr_bpm = None;

    let listed = listed_one(&bare);

    assert_eq!(
        (
            listed.distance_m,
            listed.elevation_gain_m,
            listed.average_power_w,
            listed.average_hr_bpm
        ),
        (None, None, None, None)
    );
}

#[test]
fn activities_are_listed_newest_first() {
    let conn = db::open_in_memory().unwrap();
    let older = insert_activity(&conn, &record(1, "2026-03-14T07:00:00Z", 0)).unwrap();
    let newer = insert_activity(&conn, &record(2, "2026-03-15T07:00:00Z", 0)).unwrap();

    let ids: Vec<ActivityId> = list(&conn, DEFAULT_ATHLETE)
        .unwrap()
        .iter()
        .map(|activity| ActivityId(activity.id))
        .collect();

    assert_eq!(ids, [newer, older]);
}

#[test]
fn the_list_has_no_activity_before_the_first_import() {
    let conn = db::open_in_memory().unwrap();

    assert_eq!(list(&conn, DEFAULT_ATHLETE).unwrap(), []);
}

#[test]
fn a_listed_activity_serialises_as_the_frontend_reads_it() {
    let mut bare = record(1, "2026-03-14T07:00:00Z", 3600);
    bare.average_power_w = None;

    assert_eq!(
        serde_json::to_value(listed_one(&bare)).unwrap(),
        json!({
            "id": 1,
            "startLocal": "2026-03-14T08:00:00",
            "isIndoor": false,
            "elapsedTimeS": 3700,
            "movingTimeS": 3600,
            "distanceM": 30_500.5,
            "elevationGainM": 420.0,
            "averagePowerW": null,
            "averageHrBpm": 142.25,
        })
    );
}
