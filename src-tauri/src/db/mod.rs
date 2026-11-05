//! The only module that talks to SQLite (ADR 0003): every other module goes
//! through the typed functions below and never writes SQL.

mod migrations;

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::{DateTime, NaiveDateTime, Utc};
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSqlOutput, ValueRef};
use rusqlite::{Connection, OptionalExtension, Row, ToSql, named_params};

use crate::summary::ActivitySummary;

/// Row id of an Athlete.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AthleteId(pub i64);

/// The single Athlete the first migration creates.
pub const DEFAULT_ATHLETE: AthleteId = AthleteId(1);

/// Row id of an Activity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivityId(pub i64);

/// The FIT `file_id` triple, the first key of Duplicate detection. FIT makes
/// each part optional.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FitFileId {
    pub manufacturer: Option<u16>,
    pub serial_number: Option<u32>,
    pub time_created: Option<DateTime<Utc>>,
}

/// An Activity as stored, without its row id. Timestamps are kept to the
/// second.
#[derive(Debug, Clone, PartialEq)]
pub struct ActivityRecord {
    pub athlete_id: AthleteId,
    /// Path of the Source File relative to the Library, `/`-separated.
    pub source_path: String,
    pub content_hash: String,
    pub fit_file_id: FitFileId,
    pub start_utc: DateTime<Utc>,
    /// Offset from UTC, in seconds, of the local time where the ride happened.
    pub utc_offset_s: i32,
    pub is_indoor: bool,
    pub elapsed_time_s: u32,
    pub moving_time_s: u32,
    pub distance_m: Option<f64>,
    pub elevation_gain_m: Option<f64>,
    /// `None` when the ride had no power meter, never `0`.
    pub average_power_w: Option<f64>,
    pub average_hr_bpm: Option<f64>,
    pub summary_version: u32,
    pub imported_at: DateTime<Utc>,
}

/// A stored Activity.
#[derive(Debug, Clone, PartialEq)]
pub struct Activity {
    pub id: ActivityId,
    pub record: ActivityRecord,
}

/// The connection Tauri commands share, as managed state.
pub struct Database(pub Mutex<Connection>);

/// Where the database lives in the app data directory: next to the Library.
pub fn path_in(data_dir: &Path) -> PathBuf {
    data_dir.join("dynowatt.db")
}

/// Opens the database at `path`, creating it if needed, and brings its schema
/// up to date.
pub fn open(path: &Path) -> Result<Connection, rusqlite_migration::Error> {
    prepare(Connection::open(path)?)
}

/// Opens a private in-memory database with the latest schema, for tests.
#[cfg(test)]
pub(crate) fn open_in_memory() -> Result<Connection, rusqlite_migration::Error> {
    prepare(Connection::open_in_memory()?)
}

fn prepare(mut conn: Connection) -> Result<Connection, rusqlite_migration::Error> {
    migrations::MIGRATIONS.to_latest(&mut conn)?;
    // A per-connection setting, off by default in SQLite. Turned on after the
    // migrations, which may have to rebuild tables.
    conn.pragma_update(None, "foreign_keys", true)?;
    Ok(conn)
}

pub fn insert_activity(conn: &Connection, record: &ActivityRecord) -> rusqlite::Result<ActivityId> {
    conn.execute(
        "INSERT INTO activity (
           athlete_id, source_path, content_hash,
           fit_manufacturer, fit_serial_number, fit_time_created,
           start_utc, utc_offset_s, is_indoor, elapsed_time_s, moving_time_s,
           distance_m, elevation_gain_m, average_power_w, average_hr_bpm,
           summary_version, imported_at
         ) VALUES (
           :athlete_id, :source_path, :content_hash,
           :fit_manufacturer, :fit_serial_number, :fit_time_created,
           :start_utc, :utc_offset_s, :is_indoor, :elapsed_time_s, :moving_time_s,
           :distance_m, :elevation_gain_m, :average_power_w, :average_hr_bpm,
           :summary_version, :imported_at
         )",
        named_params! {
            ":athlete_id": record.athlete_id.0,
            ":source_path": record.source_path,
            ":content_hash": record.content_hash,
            ":fit_manufacturer": record.fit_file_id.manufacturer,
            ":fit_serial_number": record.fit_file_id.serial_number,
            ":fit_time_created": record.fit_file_id.time_created.map(SqlTime),
            ":start_utc": SqlTime(record.start_utc),
            ":utc_offset_s": record.utc_offset_s,
            ":is_indoor": record.is_indoor,
            ":elapsed_time_s": record.elapsed_time_s,
            ":moving_time_s": record.moving_time_s,
            ":distance_m": record.distance_m,
            ":elevation_gain_m": record.elevation_gain_m,
            ":average_power_w": record.average_power_w,
            ":average_hr_bpm": record.average_hr_bpm,
            ":summary_version": record.summary_version,
            ":imported_at": SqlTime(record.imported_at),
        },
    )?;
    Ok(ActivityId(conn.last_insert_rowid()))
}

/// Finds the Activity a Source File duplicates: by FIT `file_id` triple, which
/// survives a re-export, then by content hash. An incomplete triple matches
/// nothing, since SQL never finds NULL equal to NULL.
pub fn find_duplicate(
    conn: &Connection,
    fit_file_id: &FitFileId,
    content_hash: &str,
) -> rusqlite::Result<Option<ActivityId>> {
    let by_file_id = conn
        .query_row(
            "SELECT id FROM activity
             WHERE fit_manufacturer = ?1 AND fit_serial_number = ?2 AND fit_time_created = ?3",
            (
                fit_file_id.manufacturer,
                fit_file_id.serial_number,
                fit_file_id.time_created.map(SqlTime),
            ),
            |row| row.get(0).map(ActivityId),
        )
        .optional()?;
    if by_file_id.is_some() {
        return Ok(by_file_id);
    }
    conn.query_row(
        "SELECT id FROM activity WHERE content_hash = ?1",
        [content_hash],
        |row| row.get(0).map(ActivityId),
    )
    .optional()
}

/// The Athlete's Activities whose time range, from their start through their
/// Elapsed Time, overlaps `start..end`, oldest first: the same ride recorded
/// on two devices. Ranges are half-open, so a ride starting the second
/// another ends does not overlap it.
pub fn find_overlapping(
    conn: &Connection,
    athlete_id: AthleteId,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> rusqlite::Result<Vec<Activity>> {
    conn.prepare(
        "SELECT * FROM activity
         WHERE athlete_id = :athlete_id
           AND start_utc < :end
           AND unixepoch(start_utc) + elapsed_time_s > unixepoch(:start)
         ORDER BY start_utc, id",
    )?
    .query_map(
        named_params! {
            ":athlete_id": athlete_id.0,
            ":start": SqlTime(start),
            ":end": SqlTime(end),
        },
        read_activity,
    )?
    .collect()
}

/// The Athlete's Activities, newest first.
pub fn list_activities(
    conn: &Connection,
    athlete_id: AthleteId,
) -> rusqlite::Result<Vec<Activity>> {
    conn.prepare("SELECT * FROM activity WHERE athlete_id = ?1 ORDER BY start_utc DESC, id DESC")?
        .query_map([athlete_id.0], read_activity)?
        .collect()
}

/// The Activity `id`, or `None` when no Activity has this id.
pub fn find_activity(conn: &Connection, id: ActivityId) -> rusqlite::Result<Option<Activity>> {
    conn.query_row(
        "SELECT * FROM activity WHERE id = ?1",
        [id.0],
        read_activity,
    )
    .optional()
}

/// Returns `false` when no Activity has this id.
pub fn delete_activity(conn: &Connection, id: ActivityId) -> rusqlite::Result<bool> {
    Ok(conn.execute("DELETE FROM activity WHERE id = ?1", [id.0])? > 0)
}

/// The Activities whose summary was computed by rules older than
/// `current_version`, oldest import first.
pub fn list_stale_summaries(
    conn: &Connection,
    current_version: u32,
) -> rusqlite::Result<Vec<Activity>> {
    conn.prepare("SELECT * FROM activity WHERE summary_version < ?1 ORDER BY id")?
        .query_map([current_version], read_activity)?
        .collect()
}

/// Replaces the Activity Summary of the Activity `id`, and the version that
/// computed it, leaving its other columns as they are. Does nothing when no
/// Activity has this id.
pub fn update_summary(
    conn: &Connection,
    id: ActivityId,
    summary: &ActivitySummary,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE activity SET
           elapsed_time_s = :elapsed_time_s,
           moving_time_s = :moving_time_s,
           distance_m = :distance_m,
           elevation_gain_m = :elevation_gain_m,
           average_power_w = :average_power_w,
           average_hr_bpm = :average_hr_bpm,
           summary_version = :summary_version
         WHERE id = :id",
        named_params! {
            ":id": id.0,
            ":elapsed_time_s": summary.elapsed_time_s,
            ":moving_time_s": summary.moving_time_s,
            ":distance_m": summary.distance_m,
            ":elevation_gain_m": summary.elevation_gain_m,
            ":average_power_w": summary.average_power_w,
            ":average_hr_bpm": summary.average_hr_bpm,
            ":summary_version": summary.version,
        },
    )?;
    Ok(())
}

fn read_activity(row: &Row<'_>) -> rusqlite::Result<Activity> {
    Ok(Activity {
        id: ActivityId(row.get("id")?),
        record: ActivityRecord {
            athlete_id: AthleteId(row.get("athlete_id")?),
            source_path: row.get("source_path")?,
            content_hash: row.get("content_hash")?,
            fit_file_id: FitFileId {
                manufacturer: row.get("fit_manufacturer")?,
                serial_number: row.get("fit_serial_number")?,
                time_created: row
                    .get::<_, Option<SqlTime>>("fit_time_created")?
                    .map(|t| t.0),
            },
            start_utc: row.get::<_, SqlTime>("start_utc")?.0,
            utc_offset_s: row.get("utc_offset_s")?,
            is_indoor: row.get("is_indoor")?,
            elapsed_time_s: row.get("elapsed_time_s")?,
            moving_time_s: row.get("moving_time_s")?,
            distance_m: row.get("distance_m")?,
            elevation_gain_m: row.get("elevation_gain_m")?,
            average_power_w: row.get("average_power_w")?,
            average_hr_bpm: row.get("average_hr_bpm")?,
            summary_version: row.get("summary_version")?,
            imported_at: row.get::<_, SqlTime>("imported_at")?.0,
        },
    })
}

/// UTC ISO-8601 to the second, as in migration 1. Fixed width, so text order
/// is time order: `ORDER BY start_utc` relies on it.
const TIME_FORMAT: &str = "%Y-%m-%dT%H:%M:%SZ";

/// A UTC instant in its stored text form.
struct SqlTime(DateTime<Utc>);

impl ToSql for SqlTime {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(self.0.format(TIME_FORMAT).to_string().into())
    }
}

impl FromSql for SqlTime {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        NaiveDateTime::parse_from_str(value.as_str()?, TIME_FORMAT)
            .map(|time| SqlTime(time.and_utc()))
            .map_err(|err| FromSqlError::Other(Box::new(err)))
    }
}

#[cfg(test)]
mod tests;
