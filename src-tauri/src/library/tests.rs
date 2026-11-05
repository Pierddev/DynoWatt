use std::fs;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use tempfile::TempDir;

use super::*;
use crate::db::{
    ActivityRecord, DEFAULT_ATHLETE, FitFileId, find_activity, insert_activity, open_in_memory,
};

fn start() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-06-01T07:00:00Z")
        .unwrap()
        .to_utc()
}

fn record(source_path: &str) -> ActivityRecord {
    ActivityRecord {
        athlete_id: DEFAULT_ATHLETE,
        source_path: source_path.to_owned(),
        content_hash: format!("hash-{source_path}"),
        fit_file_id: FitFileId::default(),
        start_utc: start(),
        utc_offset_s: 7200,
        is_indoor: false,
        elapsed_time_s: 3600,
        moving_time_s: 3600,
        distance_m: None,
        elevation_gain_m: None,
        average_power_w: Some(200.0),
        average_hr_bpm: None,
        summary_version: 2,
        imported_at: start(),
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

    fn file(&self, source_path: &str) -> PathBuf {
        self.library().join(source_path)
    }

    /// Records an Activity at `source_path` and, unless `bytes` is `None`,
    /// puts its Source File in the Library.
    fn activity(&self, source_path: &str, bytes: Option<&[u8]>) -> ActivityId {
        if let Some(bytes) = bytes {
            let file = self.file(source_path);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, bytes).unwrap();
        }
        insert_activity(&self.conn, &record(source_path)).unwrap()
    }

    fn delete(&mut self, id: ActivityId) -> Result<(), String> {
        let library = self.library();
        delete_activity(&mut self.conn, &library, id)
    }

    fn exists(&self, id: ActivityId) -> bool {
        find_activity(&self.conn, id).unwrap().is_some()
    }
}

#[test]
fn deleting_an_activity_removes_its_row_and_its_source_file() {
    let mut setup = Setup::new();
    let kept = setup.activity("2026/kept.fit", Some(b"kept ride"));
    let deleted = setup.activity("2026/deleted.fit", Some(b"deleted ride"));

    assert_eq!(setup.delete(deleted), Ok(()));

    assert!(!setup.exists(deleted));
    assert!(!setup.file("2026/deleted.fit").exists());
    assert!(setup.exists(kept));
    assert_eq!(fs::read(setup.file("2026/kept.fit")).unwrap(), b"kept ride");
}

#[test]
fn an_activity_whose_source_file_is_already_missing_is_deleted() {
    let mut setup = Setup::new();
    let id = setup.activity("2026/ride.fit", None);

    assert_eq!(setup.delete(id), Ok(()));

    assert!(!setup.exists(id));
}

#[test]
fn an_unknown_activity_is_reported_and_nothing_is_deleted() {
    let mut setup = Setup::new();
    let id = setup.activity("2026/ride.fit", Some(b"ride"));

    assert_eq!(
        setup.delete(ActivityId(id.0 + 1)),
        Err(format!("no Activity has the id {}", id.0 + 1))
    );

    assert!(setup.exists(id));
    assert!(setup.file("2026/ride.fit").exists());
}

#[test]
fn a_database_failure_is_reported() {
    let mut setup = Setup::new();
    let id = setup.activity("2026/ride.fit", Some(b"ride"));
    setup.conn.execute("DROP TABLE activity", []).unwrap();

    let reason = setup.delete(id).unwrap_err();

    assert!(reason.starts_with("cannot find the Activity: "), "{reason}");
    assert!(setup.file("2026/ride.fit").exists());
}

#[test]
fn a_row_the_database_refuses_to_delete_keeps_its_source_file() {
    let mut setup = Setup::new();
    let id = setup.activity("2026/ride.fit", Some(b"ride"));
    setup
        .conn
        .execute_batch(
            "CREATE TEMP TRIGGER refuse BEFORE DELETE ON activity
             BEGIN SELECT RAISE(ABORT, 'delete refused'); END",
        )
        .unwrap();

    let reason = setup.delete(id).unwrap_err();

    assert_eq!(reason, "cannot delete the Activity: delete refused");
    assert!(setup.exists(id));
    assert_eq!(fs::read(setup.file("2026/ride.fit")).unwrap(), b"ride");
}

#[test]
fn a_deletion_the_database_refuses_to_commit_puts_the_source_file_back() {
    let mut setup = Setup::new();
    let id = setup.activity("2026/ride.fit", Some(b"ride"));
    // A reference checked only at commit: the DELETE passes, the COMMIT fails.
    setup
        .conn
        .execute_batch(&format!(
            "CREATE TABLE pin (activity_id INTEGER REFERENCES activity(id) DEFERRABLE INITIALLY DEFERRED);
             INSERT INTO pin VALUES ({})",
            id.0
        ))
        .unwrap();

    let reason = setup.delete(id).unwrap_err();

    assert!(
        reason.starts_with("cannot delete the Activity: "),
        "{reason}"
    );
    assert!(setup.exists(id));
    assert_eq!(fs::read(setup.file("2026/ride.fit")).unwrap(), b"ride");
}

#[cfg(unix)]
mod locked {
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    #[test]
    fn a_source_file_that_cannot_be_removed_keeps_its_row() {
        let mut setup = Setup::new();
        let id = setup.activity("2026/ride.fit", Some(b"ride"));
        let folder = setup.file("2026");
        fs::set_permissions(&folder, fs::Permissions::from_mode(0o555)).unwrap();
        // Root writes into any folder: nothing to test then.
        let bound = fs::write(folder.join("probe"), b"").is_err();

        let result = setup.delete(id);

        fs::set_permissions(&folder, fs::Permissions::from_mode(0o755)).unwrap();
        if !bound {
            return;
        }
        let reason = result.unwrap_err();
        assert!(reason.starts_with("cannot remove "), "{reason}");
        assert!(reason.contains("ride.fit"), "{reason}");
        assert!(setup.exists(id));
        assert_eq!(fs::read(setup.file("2026/ride.fit")).unwrap(), b"ride");
    }

    #[test]
    fn a_source_file_that_cannot_be_read_keeps_its_row() {
        let mut setup = Setup::new();
        let id = setup.activity("2026/ride.fit", Some(b"ride"));
        let file = setup.file("2026/ride.fit");
        fs::set_permissions(&file, fs::Permissions::from_mode(0o000)).unwrap();
        // Root reads any file: nothing to test then.
        let bound = fs::read(&file).is_err();

        let result = setup.delete(id);

        fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).unwrap();
        if !bound {
            return;
        }
        let reason = result.unwrap_err();
        assert!(reason.starts_with("cannot read "), "{reason}");
        assert!(reason.contains("ride.fit"), "{reason}");
        assert!(setup.exists(id));
        assert!(file.exists());
    }
}
