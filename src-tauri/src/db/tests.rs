use chrono::{Duration, TimeDelta};

use super::*;

fn at(text: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(text).unwrap().to_utc()
}

/// A complete, valid record; `n` makes its unique columns unique.
fn record(n: u32) -> ActivityRecord {
    let start_utc = at("2026-05-01T07:00:00Z") + Duration::days(n.into());
    ActivityRecord {
        athlete_id: DEFAULT_ATHLETE,
        source_path: format!("2026/ride-{n}.fit"),
        content_hash: format!("hash-{n}"),
        fit_file_id: FitFileId {
            manufacturer: Some(1),
            serial_number: Some(1000 + n),
            time_created: Some(start_utc - Duration::minutes(5)),
        },
        start_utc,
        utc_offset_s: 7200,
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

fn ids(activities: &[Activity]) -> Vec<ActivityId> {
    activities.iter().map(|activity| activity.id).collect()
}

#[test]
fn the_database_sits_in_the_app_data_directory() {
    assert_eq!(
        path_in(Path::new("/data/dynowatt")),
        Path::new("/data/dynowatt/dynowatt.db")
    );
}

#[test]
fn migrations_are_valid() {
    migrations::MIGRATIONS.validate().unwrap();
}

#[test]
fn a_fresh_database_holds_the_single_default_athlete() {
    let conn = open_in_memory().unwrap();

    let athletes: Vec<(i64, String, SqlTime)> = conn
        .prepare("SELECT id, name, created_at FROM athlete")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();

    assert_eq!(athletes.len(), 1);
    let (id, name, created_at) = &athletes[0];
    assert_eq!(
        (AthleteId(*id), name.as_str()),
        (DEFAULT_ATHLETE, "Athlete")
    );
    assert!((Utc::now() - created_at.0).abs() < TimeDelta::minutes(1));
}

#[test]
fn reopening_a_database_keeps_its_data_and_runs_no_migration_twice() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("dynowatt.db");
    let conn = open(&path).unwrap();
    let id = insert_activity(&conn, &record(1)).unwrap();
    drop(conn);

    let conn = open(&path).unwrap();

    assert_eq!(migrations::MIGRATIONS.pending_migrations(&conn).unwrap(), 0);
    let athletes: i64 = conn
        .query_row("SELECT count(*) FROM athlete", [], |row| row.get(0))
        .unwrap();
    assert_eq!(athletes, 1);
    assert_eq!(ids(&list_activities(&conn, DEFAULT_ATHLETE).unwrap()), [id]);
}

#[test]
fn an_inserted_activity_reads_back_unchanged() {
    let conn = open_in_memory().unwrap();
    let record = ActivityRecord {
        is_indoor: true,
        utc_offset_s: -18_000,
        summary_version: 3,
        ..record(1)
    };

    let id = insert_activity(&conn, &record).unwrap();

    assert_eq!(
        list_activities(&conn, DEFAULT_ATHLETE).unwrap(),
        [Activity { id, record }]
    );
}

#[test]
fn missing_metrics_round_trip_as_none_never_as_zero() {
    let conn = open_in_memory().unwrap();
    let record = ActivityRecord {
        fit_file_id: FitFileId::default(),
        distance_m: None,
        elevation_gain_m: None,
        average_power_w: None,
        average_hr_bpm: None,
        ..record(1)
    };

    let id = insert_activity(&conn, &record).unwrap();

    assert_eq!(
        list_activities(&conn, DEFAULT_ATHLETE).unwrap(),
        [Activity { id, record }]
    );
    let stored_types: String = conn
        .query_row(
            "SELECT typeof(fit_manufacturer) || typeof(fit_serial_number)
               || typeof(fit_time_created) || typeof(distance_m)
               || typeof(elevation_gain_m) || typeof(average_power_w)
               || typeof(average_hr_bpm)
             FROM activity",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stored_types, "null".repeat(7));
}

#[test]
fn timestamps_are_stored_as_utc_iso_8601_to_the_second() {
    let conn = open_in_memory().unwrap();
    let mut record = record(1);
    record.start_utc = at("2026-05-02T09:00:00.750+02:00");
    record.fit_file_id.time_created = Some(at("2026-05-02T06:55:00.250Z"));
    record.imported_at = at("2026-11-05T09:30:00.999Z");

    insert_activity(&conn, &record).unwrap();

    let stored: (String, String, String) = conn
        .query_row(
            "SELECT start_utc, fit_time_created, imported_at FROM activity",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(
        stored,
        (
            "2026-05-02T07:00:00Z".to_owned(),
            "2026-05-02T06:55:00Z".to_owned(),
            "2026-11-05T09:30:00Z".to_owned()
        )
    );
    let read = &list_activities(&conn, DEFAULT_ATHLETE).unwrap()[0].record;
    assert_eq!(read.start_utc, at("2026-05-02T07:00:00Z"));
    assert_eq!(
        read.fit_file_id.time_created,
        Some(at("2026-05-02T06:55:00Z"))
    );
    assert_eq!(read.imported_at, at("2026-11-05T09:30:00Z"));
}

#[test]
fn an_unreadable_stored_timestamp_is_an_error() {
    let conn = open_in_memory().unwrap();
    insert_activity(&conn, &record(1)).unwrap();
    conn.execute("UPDATE activity SET start_utc = '2026-05-02 07:00:00'", [])
        .unwrap();

    assert!(list_activities(&conn, DEFAULT_ATHLETE).is_err());
}

#[test]
fn an_activity_must_belong_to_an_existing_athlete() {
    let conn = open_in_memory().unwrap();
    let record = ActivityRecord {
        athlete_id: AthleteId(2),
        ..record(1)
    };

    assert!(insert_activity(&conn, &record).is_err());
}

#[test]
fn a_source_path_content_hash_or_complete_file_id_is_stored_once() {
    let conn = open_in_memory().unwrap();
    let first = record(1);
    insert_activity(&conn, &first).unwrap();

    let same_path = ActivityRecord {
        source_path: first.source_path.clone(),
        ..record(2)
    };
    let same_hash = ActivityRecord {
        content_hash: first.content_hash.clone(),
        ..record(3)
    };
    let same_file_id = ActivityRecord {
        fit_file_id: first.fit_file_id,
        ..record(4)
    };
    assert!(insert_activity(&conn, &same_path).is_err());
    assert!(insert_activity(&conn, &same_hash).is_err());
    assert!(insert_activity(&conn, &same_file_id).is_err());

    // Activities without a complete file_id may share its known parts.
    let partial = FitFileId {
        serial_number: None,
        ..first.fit_file_id
    };
    for n in [5, 6] {
        let record = ActivityRecord {
            fit_file_id: partial,
            ..record(n)
        };
        insert_activity(&conn, &record).unwrap();
    }
}

#[test]
fn a_re_exported_source_file_is_a_duplicate_by_file_id() {
    let conn = open_in_memory().unwrap();
    let id = insert_activity(&conn, &record(1)).unwrap();

    let found = find_duplicate(&conn, &record(1).fit_file_id, "re-exported-bytes").unwrap();

    assert_eq!(found, Some(id));
}

#[test]
fn the_file_id_match_wins_over_the_content_hash_match() {
    let conn = open_in_memory().unwrap();
    let by_file_id = insert_activity(&conn, &record(1)).unwrap();
    insert_activity(&conn, &record(2)).unwrap();

    let found = find_duplicate(&conn, &record(1).fit_file_id, &record(2).content_hash).unwrap();

    assert_eq!(found, Some(by_file_id));
}

#[test]
fn identical_bytes_are_a_duplicate_by_content_hash() {
    let conn = open_in_memory().unwrap();
    let record = ActivityRecord {
        fit_file_id: FitFileId::default(),
        ..record(1)
    };
    let id = insert_activity(&conn, &record).unwrap();

    let found = find_duplicate(&conn, &record.fit_file_id, &record.content_hash).unwrap();

    assert_eq!(found, Some(id));
}

#[test]
fn a_file_id_differing_in_any_part_is_not_a_duplicate() {
    let conn = open_in_memory().unwrap();
    let stored = record(1).fit_file_id;
    insert_activity(&conn, &record(1)).unwrap();

    let candidates = [
        FitFileId {
            manufacturer: Some(2),
            ..stored
        },
        FitFileId {
            serial_number: Some(9999),
            ..stored
        },
        FitFileId {
            time_created: Some(at("2026-01-01T00:00:00Z")),
            ..stored
        },
    ];

    for candidate in candidates {
        assert_eq!(
            find_duplicate(&conn, &candidate, "new-bytes").unwrap(),
            None
        );
    }
}

#[test]
fn an_incomplete_file_id_never_matches() {
    let conn = open_in_memory().unwrap();
    let partial = FitFileId {
        serial_number: None,
        ..record(1).fit_file_id
    };
    let record = ActivityRecord {
        fit_file_id: partial,
        ..record(1)
    };
    insert_activity(&conn, &record).unwrap();

    assert_eq!(find_duplicate(&conn, &partial, "new-bytes").unwrap(), None);
}

/// The Activities [`find_overlapping`] finds between two RFC 3339 instants.
fn overlapping(conn: &Connection, start: &str, end: &str) -> Vec<ActivityId> {
    ids(&find_overlapping(conn, DEFAULT_ATHLETE, at(start), at(end)).unwrap())
}

#[test]
fn an_activity_overlapping_the_range_is_found_whole() {
    let conn = open_in_memory().unwrap();
    // Rides from 07:00:00 for 3700 s, through 08:01:39.
    let id = insert_activity(&conn, &record(1)).unwrap();

    let found = find_overlapping(
        &conn,
        DEFAULT_ATHLETE,
        at("2026-05-02T07:30:00Z"),
        at("2026-05-02T09:00:00Z"),
    )
    .unwrap();

    assert_eq!(
        found,
        [Activity {
            id,
            record: record(1)
        }]
    );
}

#[test]
fn ranges_that_only_touch_do_not_overlap() {
    let conn = open_in_memory().unwrap();
    // Rides from 07:00:00 for 3700 s, through 08:01:39.
    let id = insert_activity(&conn, &record(1)).unwrap();

    assert_eq!(
        overlapping(&conn, "2026-05-02T08:01:40Z", "2026-05-02T09:00:00Z"),
        []
    );
    assert_eq!(
        overlapping(&conn, "2026-05-02T08:01:39Z", "2026-05-02T09:00:00Z"),
        [id]
    );
    assert_eq!(
        overlapping(&conn, "2026-05-02T06:00:00Z", "2026-05-02T07:00:00Z"),
        []
    );
    assert_eq!(
        overlapping(&conn, "2026-05-02T06:00:00Z", "2026-05-02T07:00:01Z"),
        [id]
    );
}

#[test]
fn a_range_within_an_activity_overlaps_it() {
    let conn = open_in_memory().unwrap();
    let id = insert_activity(&conn, &record(1)).unwrap();

    assert_eq!(
        overlapping(&conn, "2026-05-02T07:10:00Z", "2026-05-02T07:20:00Z"),
        [id]
    );
}

#[test]
fn overlapping_activities_are_listed_oldest_first() {
    let conn = open_in_memory().unwrap();
    let later = insert_activity(
        &conn,
        &ActivityRecord {
            start_utc: at("2026-05-02T07:30:00Z"),
            ..record(2)
        },
    )
    .unwrap();
    let earlier = insert_activity(&conn, &record(1)).unwrap();
    // Two days later: out of the range.
    insert_activity(&conn, &record(3)).unwrap();

    assert_eq!(
        overlapping(&conn, "2026-05-02T07:00:00Z", "2026-05-02T09:00:00Z"),
        [earlier, later]
    );
}

#[test]
fn only_the_given_athletes_activities_overlap() {
    let conn = open_in_memory().unwrap();
    conn.execute(
        "INSERT INTO athlete (id, name, created_at) VALUES (2, 'Other', '2026-01-01T00:00:00Z')",
        [],
    )
    .unwrap();
    insert_activity(
        &conn,
        &ActivityRecord {
            athlete_id: AthleteId(2),
            ..record(1)
        },
    )
    .unwrap();

    assert_eq!(
        overlapping(&conn, "2026-05-02T07:00:00Z", "2026-05-02T09:00:00Z"),
        []
    );
}

#[test]
fn activities_are_listed_newest_first() {
    let conn = open_in_memory().unwrap();
    let first = insert_activity(&conn, &record(1)).unwrap();
    let third = insert_activity(&conn, &record(3)).unwrap();
    let second = insert_activity(&conn, &record(2)).unwrap();

    let listed = list_activities(&conn, DEFAULT_ATHLETE).unwrap();

    assert_eq!(ids(&listed), [third, second, first]);
}

#[test]
fn activities_starting_together_list_the_latest_import_first() {
    let conn = open_in_memory().unwrap();
    let start_utc = record(1).start_utc;
    let earlier = insert_activity(&conn, &record(1)).unwrap();
    let later = insert_activity(
        &conn,
        &ActivityRecord {
            start_utc,
            ..record(2)
        },
    )
    .unwrap();

    let listed = list_activities(&conn, DEFAULT_ATHLETE).unwrap();

    assert_eq!(ids(&listed), [later, earlier]);
}

#[test]
fn only_the_given_athletes_activities_are_listed() {
    let conn = open_in_memory().unwrap();
    conn.execute(
        "INSERT INTO athlete (id, name, created_at) VALUES (2, 'Other', '2026-01-01T00:00:00Z')",
        [],
    )
    .unwrap();
    let mine = insert_activity(&conn, &record(1)).unwrap();
    let theirs = insert_activity(
        &conn,
        &ActivityRecord {
            athlete_id: AthleteId(2),
            ..record(2)
        },
    )
    .unwrap();

    assert_eq!(
        ids(&list_activities(&conn, DEFAULT_ATHLETE).unwrap()),
        [mine]
    );
    assert_eq!(
        ids(&list_activities(&conn, AthleteId(2)).unwrap()),
        [theirs]
    );
}

#[test]
fn an_activity_is_found_by_its_id() {
    let conn = open_in_memory().unwrap();
    insert_activity(&conn, &record(1)).unwrap();
    let id = insert_activity(&conn, &record(2)).unwrap();
    insert_activity(&conn, &record(3)).unwrap();

    assert_eq!(
        find_activity(&conn, id).unwrap(),
        Some(Activity {
            id,
            record: record(2)
        })
    );
}

#[test]
fn an_unknown_activity_is_not_found() {
    let conn = open_in_memory().unwrap();
    let id = insert_activity(&conn, &record(1)).unwrap();

    assert_eq!(find_activity(&conn, ActivityId(id.0 + 1)).unwrap(), None);
}

#[test]
fn deleting_an_activity_removes_only_its_row() {
    let conn = open_in_memory().unwrap();
    let deleted = insert_activity(&conn, &record(1)).unwrap();
    let kept = insert_activity(&conn, &record(2)).unwrap();

    assert!(delete_activity(&conn, deleted).unwrap());

    assert_eq!(
        ids(&list_activities(&conn, DEFAULT_ATHLETE).unwrap()),
        [kept]
    );
}

#[test]
fn deleting_an_unknown_activity_reports_it() {
    let conn = open_in_memory().unwrap();
    insert_activity(&conn, &record(1)).unwrap();

    assert!(!delete_activity(&conn, ActivityId(99)).unwrap());
}

#[test]
fn stale_summaries_are_those_older_than_the_current_version() {
    let conn = open_in_memory().unwrap();
    let mut stale = Vec::new();
    for (n, summary_version) in [(1, 1), (2, 3), (3, 0), (4, 2)] {
        let record = ActivityRecord {
            summary_version,
            ..record(n)
        };
        let id = insert_activity(&conn, &record).unwrap();
        if summary_version < 2 {
            stale.push(Activity { id, record });
        }
    }

    assert_eq!(list_stale_summaries(&conn, 2).unwrap(), stale);
}

#[test]
fn updating_a_summary_rewrites_only_its_figures_and_version() {
    let conn = open_in_memory().unwrap();
    let updated = insert_activity(&conn, &record(1)).unwrap();
    let untouched = insert_activity(&conn, &record(2)).unwrap();
    let summary = ActivitySummary {
        version: 7,
        elapsed_time_s: 11,
        moving_time_s: 12,
        distance_m: Some(13.5),
        elevation_gain_m: Some(14.5),
        average_power_w: Some(15.5),
        average_hr_bpm: Some(16.5),
    };

    update_summary(&conn, updated, &summary).unwrap();

    assert_eq!(
        list_activities(&conn, DEFAULT_ATHLETE).unwrap(),
        [
            Activity {
                id: untouched,
                record: record(2),
            },
            Activity {
                id: updated,
                record: ActivityRecord {
                    elapsed_time_s: 11,
                    moving_time_s: 12,
                    distance_m: Some(13.5),
                    elevation_gain_m: Some(14.5),
                    average_power_w: Some(15.5),
                    average_hr_bpm: Some(16.5),
                    summary_version: 7,
                    ..record(1)
                },
            },
        ]
    );
}
