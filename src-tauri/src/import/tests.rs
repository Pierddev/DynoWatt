use chrono::Duration;
use tempfile::TempDir;

use super::*;
use crate::db::{Activity, FitFileId, list_activities, open_in_memory};
use crate::decoder::fixtures::{FileId, FitBuilder, Point, Sport, SubSport};
use crate::summary::SUMMARY_VERSION;

fn at(text: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(text).unwrap().to_utc()
}

const NOW: &str = "2026-11-05T09:30:00Z";

/// A ride recorded by the device with serial number `serial`, from `start`
/// for `seconds` seconds at `power_w`, two hours ahead of UTC.
fn ride_at(serial: u32, start: &str, seconds: i64, power_w: u16) -> Vec<u8> {
    fit_file(serial, start, seconds, Sport::CYCLING, power_w)
}

fn fit_file(serial: u32, start: &str, seconds: i64, sport: Sport, power_w: u16) -> Vec<u8> {
    let start = at(start);
    let mut fit = FitBuilder::new()
        .file_id(FileId {
            manufacturer: 1,
            serial_number: serial,
            time_created: start,
        })
        .session(sport, SubSport::GENERIC);
    for s in 0..seconds {
        fit = fit.record(
            start + Duration::seconds(s),
            Point {
                power_w: Some(power_w),
                ..Point::default()
            },
        );
    }
    fit.activity(start + Duration::seconds(seconds), 7200)
        .build()
}

/// A one-minute ride, its start told apart by `n`.
fn ride(n: u32) -> Vec<u8> {
    ride_at(n, &format!("2026-06-{n:02}T07:00:00Z"), 60, 200)
}

/// A folder of Source Files, an empty Library next to it, and a database.
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

    fn sources(&self) -> PathBuf {
        self.dir.path().join("sources")
    }

    /// Writes a Source File at `name` in the sources folder.
    fn source(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.sources().join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, bytes).unwrap();
        path
    }

    fn import(&self, paths: &[PathBuf]) -> ImportReport {
        run(&self.conn, &self.library(), paths, at(NOW))
    }

    fn outcomes(&self, paths: &[PathBuf]) -> Vec<Outcome> {
        self.import(paths)
            .files
            .into_iter()
            .map(|file| file.outcome)
            .collect()
    }

    /// The files in the Library, relative to it, sorted.
    fn library_files(&self) -> Vec<String> {
        fn walk(folder: &Path, prefix: &str, files: &mut Vec<String>) {
            let Ok(entries) = fs::read_dir(folder) else {
                return;
            };
            for entry in entries {
                let entry = entry.unwrap();
                let name = format!("{prefix}{}", entry.file_name().to_string_lossy());
                if entry.file_type().unwrap().is_dir() {
                    walk(&entry.path(), &format!("{name}/"), files);
                } else {
                    files.push(name);
                }
            }
        }
        let mut files = Vec::new();
        walk(&self.library(), "", &mut files);
        files.sort();
        files
    }

    fn activities(&self) -> Vec<Activity> {
        list_activities(&self.conn, DEFAULT_ATHLETE).unwrap()
    }
}

fn hash_of(bytes: &[u8]) -> String {
    decoder::decode(bytes).unwrap().content_hash
}

fn imported(activity_id: i64) -> Outcome {
    Outcome::Imported {
        activity_id,
        overlaps: vec![],
    }
}

fn reason(outcome: &Outcome) -> &str {
    match outcome {
        Outcome::Unreadable { reason } | Outcome::Failed { reason } => reason,
        other => panic!("no reason in {other:?}"),
    }
}

#[test]
fn the_library_sits_in_the_app_data_directory() {
    assert_eq!(
        library_in(Path::new("/data/dynowatt")),
        Path::new("/data/dynowatt/library")
    );
}

#[test]
fn a_library_name_tells_the_local_start_time_and_the_short_hash() {
    let hash = "0123456789abcdef";

    assert_eq!(
        library_path(at("2026-06-01T07:05:59Z"), 0, hash),
        "2026/2026-06-01_0705_01234567.fit"
    );
    assert_eq!(
        library_path(at("2026-12-31T23:30:00Z"), 3600, hash),
        "2027/2027-01-01_0030_01234567.fit"
    );
    assert_eq!(
        library_path(at("2026-01-01T02:05:00Z"), -18_000, hash),
        "2025/2025-12-31_2105_01234567.fit"
    );
}

#[test]
fn an_imported_file_is_copied_unmodified_into_the_library_and_recorded() {
    let setup = Setup::new();
    let bytes = ride_at(7, "2026-12-31T23:30:00Z", 60, 200);
    let hash = hash_of(&bytes);
    let path = setup.source("ride.fit", &bytes);

    let report = setup.import(std::slice::from_ref(&path));

    let activities = setup.activities();
    assert_eq!(activities.len(), 1);
    let Activity { id, record } = &activities[0];
    assert_eq!(
        report,
        ImportReport {
            files: vec![FileReport {
                path: path.display().to_string(),
                outcome: imported(id.0),
            }]
        }
    );
    let source_path = format!("2027/2027-01-01_0130_{}.fit", &hash[..8]);
    assert_eq!(
        *record,
        ActivityRecord {
            athlete_id: DEFAULT_ATHLETE,
            source_path: source_path.clone(),
            content_hash: hash,
            fit_file_id: FitFileId {
                manufacturer: Some(1),
                serial_number: Some(7),
                time_created: Some(at("2026-12-31T23:30:00Z")),
            },
            start_utc: at("2026-12-31T23:30:00Z"),
            utc_offset_s: 7200,
            is_indoor: false,
            elapsed_time_s: 60,
            moving_time_s: 60,
            distance_m: None,
            elevation_gain_m: None,
            average_power_w: Some(200.0),
            average_hr_bpm: None,
            summary_version: SUMMARY_VERSION,
            imported_at: at(NOW),
        }
    );
    assert_eq!(setup.library_files(), [source_path.as_str()]);
    assert_eq!(fs::read(setup.library().join(source_path)).unwrap(), bytes);
}

#[test]
fn reimporting_the_same_folder_yields_only_duplicates() {
    let setup = Setup::new();
    setup.source("a.fit", &ride(1));
    setup.source("2026/b.fit", &ride(2));
    let folder = [setup.sources()];

    let first = setup.outcomes(&folder);
    let second = setup.outcomes(&folder);

    let [
        Outcome::Imported { activity_id: b, .. },
        Outcome::Imported { activity_id: a, .. },
    ] = first[..]
    else {
        panic!("both imported the first time: {first:?}");
    };
    assert_eq!(
        second,
        [
            Outcome::Duplicate { activity_id: b },
            Outcome::Duplicate { activity_id: a },
        ]
    );
    assert_eq!(setup.activities().len(), 2);
    assert_eq!(setup.library_files().len(), 2);
}

#[test]
fn a_file_re_exported_with_other_bytes_is_a_duplicate() {
    let setup = Setup::new();
    let original = ride_at(1, "2026-06-01T07:00:00Z", 60, 200);
    let re_exported = ride_at(1, "2026-06-01T07:00:00Z", 60, 180);
    assert_ne!(original, re_exported);
    let first = setup.outcomes(&[setup.source("original.fit", &original)]);
    let [Outcome::Imported { activity_id, .. }] = first[..] else {
        panic!("imported: {first:?}");
    };

    let second = setup.outcomes(&[setup.source("re-exported.fit", &re_exported)]);

    assert_eq!(second, [Outcome::Duplicate { activity_id }]);
    assert_eq!(setup.library_files().len(), 1);
}

#[test]
fn a_file_not_cycling_is_rejected_and_never_copied() {
    let setup = Setup::new();
    let run = fit_file(1, "2026-06-01T07:00:00Z", 60, Sport::RUNNING, 200);

    let outcomes = setup.outcomes(&[setup.source("run.fit", &run)]);

    assert_eq!(outcomes, [Outcome::NotCycling]);
    assert_eq!(setup.library_files(), Vec::<String>::new());
    assert_eq!(setup.activities(), []);
}

#[test]
fn a_file_that_is_not_fit_is_rejected_as_unreadable_and_never_copied() {
    let setup = Setup::new();

    let outcomes = setup.outcomes(&[setup.source("notes.fit", b"not a FIT file")]);

    assert!(matches!(outcomes[..], [Outcome::Unreadable { .. }]));
    assert_eq!(setup.library_files(), Vec::<String>::new());
    assert_eq!(setup.activities(), []);
}

#[test]
fn a_missing_file_is_rejected_as_unreadable() {
    let setup = Setup::new();
    let missing = setup.sources().join("missing.fit");

    let report = setup.import(std::slice::from_ref(&missing));

    assert_eq!(report.files.len(), 1);
    assert_eq!(report.files[0].path, missing.display().to_string());
    assert!(matches!(
        report.files[0].outcome,
        Outcome::Unreadable { .. }
    ));
}

#[test]
fn a_bad_file_never_aborts_the_run() {
    let setup = Setup::new();
    setup.source("1-garbled.fit", b"garbled");
    setup.source(
        "2-run.fit",
        &fit_file(1, "2026-06-01T07:00:00Z", 60, Sport::RUNNING, 200),
    );
    setup.source("3-ride.fit", &ride(2));

    let outcomes = setup.outcomes(&[setup.sources()]);

    assert!(matches!(
        outcomes[..],
        [
            Outcome::Unreadable { .. },
            Outcome::NotCycling,
            Outcome::Imported { .. }
        ]
    ));
    assert_eq!(setup.activities().len(), 1);
}

#[test]
fn a_ride_overlapping_an_activity_is_imported_with_a_warning() {
    let setup = Setup::new();
    let first = setup
        .outcomes(&[setup.source("garmin.fit", &ride_at(1, "2026-06-01T07:00:00Z", 600, 200))]);
    assert!(matches!(first[..], [Outcome::Imported { ref overlaps, .. }] if overlaps.is_empty()));
    let garmin = setup.activities()[0].record.source_path.clone();

    // Started earlier on the other device, so it overlaps only through its
    // Elapsed Time.
    let second =
        setup.outcomes(&[setup.source("wahoo.fit", &ride_at(2, "2026-06-01T06:55:00Z", 600, 210))]);

    let [Outcome::Imported { ref overlaps, .. }] = second[..] else {
        panic!("imported: {second:?}");
    };
    assert_eq!(*overlaps, [garmin]);
    assert_eq!(setup.activities().len(), 2);
}

#[test]
fn back_to_back_rides_do_not_overlap() {
    let setup = Setup::new();
    // One minute each, from 07:00:00, 07:01:00 and 07:02:00. The middle one
    // comes first, so the others end right where it starts, or start right
    // where it ends.
    let middle = setup.source("middle.fit", &ride_at(1, "2026-06-01T07:01:00Z", 60, 200));
    let before = setup.source("before.fit", &ride_at(2, "2026-06-01T07:00:00Z", 60, 200));
    let after = setup.source("after.fit", &ride_at(3, "2026-06-01T07:02:00Z", 60, 200));

    let outcomes = setup.outcomes(&[middle, before, after]);

    // Newest first: after, middle, before.
    let ids: Vec<i64> = setup.activities().iter().map(|a| a.id.0).collect();
    assert_eq!(
        outcomes,
        [imported(ids[1]), imported(ids[2]), imported(ids[0])]
    );
}

#[test]
fn rides_sharing_their_last_second_overlap() {
    let setup = Setup::new();
    let later = setup.source("later.fit", &ride_at(1, "2026-06-01T07:00:59Z", 60, 200));
    // 07:00:00 through 07:00:59.
    let earlier = setup.source("earlier.fit", &ride_at(2, "2026-06-01T07:00:00Z", 60, 200));
    setup.import(&[later]);
    let later_path = setup.activities()[0].record.source_path.clone();

    let outcomes = setup.outcomes(&[earlier]);

    let [Outcome::Imported { ref overlaps, .. }] = outcomes[..] else {
        panic!("imported: {outcomes:?}");
    };
    assert_eq!(*overlaps, [later_path]);
}

#[test]
fn a_failed_library_copy_leaves_no_row() {
    let setup = Setup::new();
    // A file where the year's folder should be.
    fs::create_dir_all(setup.library()).unwrap();
    fs::write(setup.library().join("2026"), b"in the way").unwrap();

    let outcomes = setup.outcomes(&[setup.source("ride.fit", &ride(1))]);

    assert!(reason(&outcomes[0]).starts_with("cannot copy into the Library"));
    assert!(matches!(outcomes[..], [Outcome::Failed { .. }]));
    assert_eq!(setup.activities(), []);
    assert_eq!(setup.library_files(), ["2026"]);
}

#[test]
fn an_existing_library_file_is_never_overwritten() {
    let setup = Setup::new();
    let bytes = ride(1);
    let target = setup.library().join(format!(
        "2026/2026-06-01_0900_{}.fit",
        &hash_of(&bytes)[..8]
    ));
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::write(&target, b"someone else's").unwrap();

    let outcomes = setup.outcomes(&[setup.source("ride.fit", &bytes)]);

    assert!(matches!(outcomes[..], [Outcome::Failed { .. }]));
    assert_eq!(fs::read(&target).unwrap(), b"someone else's");
    assert_eq!(setup.activities(), []);
}

#[test]
fn a_failed_database_insert_leaves_no_library_file() {
    let setup = Setup::new();
    let bytes = ride(1);
    let source_path = format!("2026/2026-06-01_0900_{}.fit", &hash_of(&bytes)[..8]);
    // A row already claims that Library path, though its file is gone.
    let blocker = ActivityRecord {
        athlete_id: DEFAULT_ATHLETE,
        source_path: source_path.clone(),
        content_hash: "another hash".to_owned(),
        fit_file_id: FitFileId::default(),
        start_utc: at("2026-01-01T07:00:00Z"),
        utc_offset_s: 0,
        is_indoor: false,
        elapsed_time_s: 60,
        moving_time_s: 60,
        distance_m: None,
        elevation_gain_m: None,
        average_power_w: None,
        average_hr_bpm: None,
        summary_version: SUMMARY_VERSION,
        imported_at: at(NOW),
    };
    db::insert_activity(&setup.conn, &blocker).unwrap();

    let outcomes = setup.outcomes(&[setup.source("ride.fit", &bytes)]);

    assert!(reason(&outcomes[0]).starts_with("cannot record the Activity"));
    assert!(matches!(outcomes[..], [Outcome::Failed { .. }]));
    assert_eq!(setup.library_files(), Vec::<String>::new());
    assert_eq!(setup.activities().len(), 1);
}

#[test]
fn a_database_error_fails_each_file_and_the_run_goes_on() {
    let setup = Setup::new();
    setup.conn.execute("DROP TABLE activity", []).unwrap();
    let files = [
        setup.source("1.fit", &ride(1)),
        setup.source("2.fit", &ride(2)),
    ];

    let outcomes = setup.outcomes(&files);

    assert!(matches!(
        outcomes[..],
        [Outcome::Failed { .. }, Outcome::Failed { .. }]
    ));
    assert_eq!(setup.library_files(), Vec::<String>::new());
}

#[test]
fn a_folder_yields_its_fit_files_at_any_depth_sorted_by_path() {
    let setup = Setup::new();
    for name in [
        "b.fit",
        "a/c.FIT",
        "a/notes.txt",
        "a/deep/d.Fit",
        "e.gpx",
        "f.fit",
    ] {
        setup.source(name, b"");
    }
    let s = setup.sources();

    assert_eq!(
        source_files(std::slice::from_ref(&s)),
        [
            Ok(s.join("a/c.FIT")),
            Ok(s.join("a/deep/d.Fit")),
            Ok(s.join("b.fit")),
            Ok(s.join("f.fit")),
        ]
    );
}

#[test]
fn files_given_one_by_one_are_kept_in_order_whatever_their_extension() {
    let paths = [PathBuf::from("/rides/b.gpx"), PathBuf::from("/rides/a.fit")];

    assert_eq!(
        source_files(&paths),
        [Ok(paths[0].clone()), Ok(paths[1].clone())]
    );
}

#[cfg(unix)]
#[test]
fn a_linked_folder_is_not_followed() {
    let setup = Setup::new();
    let ride = setup.source("a.fit", b"");
    std::os::unix::fs::symlink(setup.sources(), setup.sources().join("loop")).unwrap();

    assert_eq!(source_files(&[setup.sources()]), [Ok(ride)]);
}

#[cfg(unix)]
#[test]
fn a_folder_that_cannot_be_listed_is_reported_unreadable() {
    use std::os::unix::fs::PermissionsExt;

    let setup = Setup::new();
    setup.source("locked/a.fit", &ride(1));
    let locked = setup.sources().join("locked");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    // Root reads any folder: nothing to test then.
    let listable = fs::read_dir(&locked).is_ok();

    let report = setup.import(&[setup.sources()]);

    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
    if listable {
        return;
    }
    assert_eq!(report.files.len(), 1);
    assert_eq!(report.files[0].path, locked.display().to_string());
    assert!(matches!(
        report.files[0].outcome,
        Outcome::Unreadable { .. }
    ));
}

#[test]
fn the_report_reaches_the_frontend_as_camel_case_json() {
    let report = ImportReport {
        files: vec![
            FileReport {
                path: "/rides/a.fit".to_owned(),
                outcome: Outcome::Imported {
                    activity_id: 3,
                    overlaps: vec!["2026/2026-06-01_0900_01234567.fit".to_owned()],
                },
            },
            FileReport {
                path: "/rides/b.fit".to_owned(),
                outcome: Outcome::Duplicate { activity_id: 1 },
            },
            FileReport {
                path: "/rides/c.fit".to_owned(),
                outcome: Outcome::NotCycling,
            },
            FileReport {
                path: "/rides/d.fit".to_owned(),
                outcome: Outcome::Unreadable {
                    reason: "garbled".to_owned(),
                },
            },
            FileReport {
                path: "/rides/e.fit".to_owned(),
                outcome: Outcome::Failed {
                    reason: "disk full".to_owned(),
                },
            },
        ],
    };

    assert_eq!(
        serde_json::to_value(&report).unwrap(),
        serde_json::json!({
            "files": [
                {
                    "path": "/rides/a.fit",
                    "outcome": {
                        "kind": "imported",
                        "activityId": 3,
                        "overlaps": ["2026/2026-06-01_0900_01234567.fit"]
                    }
                },
                {"path": "/rides/b.fit", "outcome": {"kind": "duplicate", "activityId": 1}},
                {"path": "/rides/c.fit", "outcome": {"kind": "notCycling"}},
                {"path": "/rides/d.fit", "outcome": {"kind": "unreadable", "reason": "garbled"}},
                {"path": "/rides/e.fit", "outcome": {"kind": "failed", "reason": "disk full"}}
            ]
        })
    );
}
