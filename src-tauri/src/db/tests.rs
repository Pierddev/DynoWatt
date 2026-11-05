use chrono::{NaiveDateTime, Utc};

use super::*;

#[test]
fn migrations_are_valid() {
    migrations::MIGRATIONS.validate().unwrap();
}

#[test]
fn a_fresh_database_holds_the_single_default_athlete() {
    let conn = open_in_memory().unwrap();

    let athletes: Vec<(i64, String, String)> = conn
        .prepare("SELECT id, name, created_at FROM athlete")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();

    assert_eq!(athletes.len(), 1);
    let (id, name, created_at) = &athletes[0];
    assert_eq!((*id, name.as_str()), (1, "Athlete"));
    let created_at = NaiveDateTime::parse_from_str(created_at, "%Y-%m-%dT%H:%M:%SZ")
        .unwrap()
        .and_utc();
    assert!((Utc::now() - created_at).num_seconds().abs() < 60);
}

#[test]
fn reopening_a_database_keeps_its_data_and_runs_no_migration_twice() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("dynowatt.db");
    let conn = open(&path).unwrap();
    conn.execute("UPDATE athlete SET name = 'Renamed'", [])
        .unwrap();
    drop(conn);

    let conn = open(&path).unwrap();

    assert_eq!(migrations::MIGRATIONS.pending_migrations(&conn).unwrap(), 0);
    let names: Vec<String> = conn
        .prepare("SELECT name FROM athlete")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(names, ["Renamed"]);
}

#[test]
fn an_activity_must_belong_to_an_existing_athlete() {
    let conn = open_in_memory().unwrap();

    let inserted = conn.execute(
        "INSERT INTO activity (athlete_id, source_path, content_hash, start_utc,
           utc_offset_s, is_indoor, elapsed_time_s, moving_time_s, summary_version,
           imported_at)
         VALUES (2, 'a.fit', 'hash', '2026-05-01T07:00:00Z', 0, 0, 60, 60, 1,
           '2026-05-01T09:00:00Z')",
        [],
    );

    assert!(inserted.is_err());
}
