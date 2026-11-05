use std::path::PathBuf;

use chrono::{DateTime, Duration, Utc};
use tempfile::TempDir;

use super::*;
use crate::db::{
    ActivityRecord, DEFAULT_ATHLETE, FitFileId, insert_activity, list_activities, open_in_memory,
};
use crate::decoder::fixtures::{FileId, FitBuilder, Point, Sport, SubSport, Totals};

fn at(text: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(text).unwrap().to_utc()
}

/// The start of ride `n`: one day apart.
fn start_of(n: u32) -> DateTime<Utc> {
    at("2026-06-01T07:00:00Z") + Duration::days(n.into())
}

/// Ride `n`: ten seconds at 200 W and 140 bpm, a record at each end, with
/// the device's distance and ascent.
fn ride(n: u32) -> Vec<u8> {
    let start = start_of(n);
    let mut fit = FitBuilder::new()
        .file_id(FileId {
            manufacturer: 1,
            serial_number: n,
            time_created: start,
        })
        .session_with(
            Sport::CYCLING,
            SubSport::GENERIC,
            Totals {
                distance_m: Some(85.0),
                ascent_m: Some(25),
            },
        );
    for s in 0..=10 {
        fit = fit.record(
            start + Duration::seconds(s),
            Point {
                power_w: Some(200),
                heart_rate_bpm: Some(140),
                ..Point::default()
            },
        );
    }
    fit.activity(start + Duration::seconds(10), 7200).build()
}

/// Ride `n` as version 1 recorded it: one second too many on Elapsed and
/// Moving Time, counted per Sample rather than between Samples.
fn version_1_record(n: u32) -> ActivityRecord {
    ActivityRecord {
        athlete_id: DEFAULT_ATHLETE,
        source_path: format!("2026/ride-{n}.fit"),
        content_hash: format!("hash-{n}"),
        fit_file_id: FitFileId {
            manufacturer: Some(1),
            serial_number: Some(n),
            time_created: Some(start_of(n)),
        },
        start_utc: start_of(n),
        utc_offset_s: 7200,
        is_indoor: false,
        elapsed_time_s: 11,
        moving_time_s: 11,
        distance_m: Some(85.0),
        elevation_gain_m: Some(25.0),
        average_power_w: Some(200.0),
        average_hr_bpm: Some(140.0),
        summary_version: 1,
        imported_at: at("2026-11-05T09:30:00Z"),
    }
}

/// Ride `n` as the current rules record it.
fn current_record(n: u32) -> ActivityRecord {
    ActivityRecord {
        elapsed_time_s: 10,
        moving_time_s: 10,
        summary_version: SUMMARY_VERSION,
        ..version_1_record(n)
    }
}

/// An empty Library and a database.
struct Setup {
    dir: TempDir,
    conn: Connection,
}

impl Setup {
    fn new() -> Self {
        Setup {
            dir: TempDir::new().unwrap(),
            conn: open_in_memory().unwrap(),
        }
    }

    fn library(&self) -> PathBuf {
        self.dir.path().join("library")
    }

    /// Records `record` and, unless `bytes` is `None`, puts its Source File
    /// in the Library.
    fn activity(&self, record: &ActivityRecord, bytes: Option<&[u8]>) -> ActivityId {
        if let Some(bytes) = bytes {
            let file = self.library().join(&record.source_path);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, bytes).unwrap();
        }
        insert_activity(&self.conn, record).unwrap()
    }

    fn recompute(&self) -> RecomputeReport {
        stale_summaries(&self.conn, &self.library()).unwrap()
    }

    fn record(&self, id: ActivityId) -> ActivityRecord {
        list_activities(&self.conn, DEFAULT_ATHLETE)
            .unwrap()
            .into_iter()
            .find(|activity| activity.id == id)
            .unwrap()
            .record
    }
}

#[test]
fn a_version_1_summary_is_recomputed_from_its_source_file() {
    let setup = Setup::new();
    let id = setup.activity(&version_1_record(1), Some(&ride(1)));

    let report = setup.recompute();

    assert_eq!(
        report,
        RecomputeReport {
            recomputed: vec![id],
            skipped: vec![],
        }
    );
    assert_eq!(setup.record(id), current_record(1));
}

#[test]
fn an_up_to_date_summary_is_left_untouched() {
    let setup = Setup::new();
    // Figures the Source File would not give: a recompute would show.
    let current = ActivityRecord {
        elapsed_time_s: 999,
        moving_time_s: 998,
        ..current_record(1)
    };
    let id = setup.activity(&current, Some(&ride(1)));

    let report = setup.recompute();

    assert_eq!(report, RecomputeReport::default());
    assert_eq!(setup.record(id), current);
}

#[test]
fn an_activity_whose_source_file_is_missing_is_skipped() {
    let setup = Setup::new();
    let missing = setup.activity(&version_1_record(1), None);
    let present = setup.activity(&version_1_record(2), Some(&ride(2)));

    let report = setup.recompute();

    assert_eq!(report.recomputed, [present]);
    let [
        Skipped {
            activity_id,
            reason,
        },
    ] = &report.skipped[..]
    else {
        panic!("one Activity skipped, not {:?}", report.skipped);
    };
    assert_eq!(*activity_id, missing);
    assert!(reason.starts_with("cannot read "), "{reason}");
    assert!(reason.contains("ride-1.fit"), "{reason}");
    assert_eq!(setup.record(missing), version_1_record(1));
    assert_eq!(setup.record(present), current_record(2));
}

#[test]
fn an_activity_whose_source_file_cannot_be_decoded_is_skipped() {
    let setup = Setup::new();
    let garbled = setup.activity(&version_1_record(1), Some(b"not a FIT file"));
    let present = setup.activity(&version_1_record(2), Some(&ride(2)));

    let report = setup.recompute();

    assert_eq!(report.recomputed, [present]);
    assert_eq!(
        report
            .skipped
            .iter()
            .map(|skipped| skipped.activity_id)
            .collect::<Vec<_>>(),
        [garbled]
    );
    assert!(
        report.skipped[0].reason.starts_with("cannot decode "),
        "{}",
        report.skipped[0].reason
    );
    assert_eq!(setup.record(garbled), version_1_record(1));
}

#[test]
fn an_activity_the_database_refuses_to_update_is_skipped() {
    let setup = Setup::new();
    let refused = setup.activity(&version_1_record(1), Some(&ride(1)));
    let accepted = setup.activity(&version_1_record(2), Some(&ride(2)));
    setup
        .conn
        .execute(
            &format!(
                "CREATE TEMP TRIGGER refuse BEFORE UPDATE ON activity WHEN OLD.id = {}
                 BEGIN SELECT RAISE(ABORT, 'update refused'); END",
                refused.0
            ),
            [],
        )
        .unwrap();

    let report = setup.recompute();

    assert_eq!(
        report,
        RecomputeReport {
            recomputed: vec![accepted],
            skipped: vec![Skipped {
                activity_id: refused,
                reason: "cannot record the summary: update refused".to_owned(),
            }],
        }
    );
    assert_eq!(setup.record(refused), version_1_record(1));
}

#[test]
fn the_run_fails_when_the_stale_summaries_cannot_be_listed() {
    let setup = Setup::new();
    setup.conn.execute("DROP TABLE activity", []).unwrap();

    assert!(stale_summaries(&setup.conn, &setup.library()).is_err());
}
