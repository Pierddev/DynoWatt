//! Imports Source Files into the Library (ADR 0001): each file is decoded,
//! checked for a Duplicate, summarised, copied unmodified into the Library,
//! then recorded in the database. One bad file never aborts the run.

#[cfg(test)]
mod tests;

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use chrono::{DateTime, TimeDelta, Utc};
use rusqlite::Connection;
use serde::Serialize;

use crate::db::{self, ActivityId, ActivityRecord, DEFAULT_ATHLETE};
use crate::decoder::{self, DecodeError};
use crate::summary::summarize;

/// The Import Report: one line per file, in the order the run met them.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct ImportReport {
    pub files: Vec<FileReport>,
}

/// What became of one file of the run.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FileReport {
    /// The file as the Athlete picked or dropped it, or found in a folder.
    pub path: String,
    pub outcome: Outcome,
}

/// The one outcome of a file. Only an imported file reaches the Library.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Outcome {
    /// Copied into the Library and recorded as a new Activity. `overlaps`
    /// holds the Library paths of the Activities whose time range it
    /// overlaps, most likely the same ride recorded on another device.
    Imported {
        activity_id: i64,
        overlaps: Vec<String>,
    },
    /// Already in the Library as `activity_id`: skipped.
    Duplicate { activity_id: i64 },
    /// Rejected: a readable file recording something other than cycling.
    NotCycling,
    /// Rejected: missing, unreadable, or not a FIT file.
    Unreadable { reason: String },
    /// The Library copy or the database failed: the machine's fault, not the
    /// file's. Nothing was kept.
    Failed { reason: String },
}

/// Where the Library lives in the app data directory.
pub fn library_in(data_dir: &Path) -> PathBuf {
    data_dir.join("library")
}

/// Imports the Source Files at `paths` into `library` and the database: a
/// file is imported as given, a folder is scanned recursively for `.fit`
/// files. `now` is the import time recorded with each Activity.
pub fn run(
    conn: &Connection,
    library: &Path,
    paths: &[PathBuf],
    now: DateTime<Utc>,
) -> ImportReport {
    let files = source_files(paths)
        .into_iter()
        .map(|found| match found {
            Ok(file) => FileReport {
                path: file.display().to_string(),
                outcome: import_file(conn, library, &file, now).unwrap_or_else(|stop| stop),
            },
            Err(Unlisted { folder, reason }) => FileReport {
                path: folder.display().to_string(),
                outcome: Outcome::Unreadable { reason },
            },
        })
        .collect();
    ImportReport { files }
}

/// Takes one file through the import. `Err` is an outcome that stops it
/// before the Library: a Duplicate, a rejection or a failure.
fn import_file(
    conn: &Connection,
    library: &Path,
    file: &Path,
    now: DateTime<Utc>,
) -> Result<Outcome, Outcome> {
    let bytes = fs::read(file).map_err(|e| Outcome::Unreadable {
        reason: e.to_string(),
    })?;
    let decoded = decoder::decode(&bytes).map_err(|e| match e {
        DecodeError::NotCycling => Outcome::NotCycling,
        DecodeError::Unreadable(reason) => Outcome::Unreadable { reason },
    })?;
    if let Some(ActivityId(activity_id)) =
        db::find_duplicate(conn, &decoded.fit_file_id, &decoded.content_hash).map_err(failed)?
    {
        return Err(Outcome::Duplicate { activity_id });
    }
    let summary = summarize(&decoded.timeline, &decoded.device_totals);
    let start_utc = decoded.timeline.start_utc;
    let end_utc = start_utc + TimeDelta::seconds(summary.elapsed_time_s.into());
    let overlaps = db::find_overlapping(conn, DEFAULT_ATHLETE, start_utc, end_utc)
        .map_err(failed)?
        .into_iter()
        .map(|activity| activity.record.source_path)
        .collect();
    let record = ActivityRecord {
        athlete_id: DEFAULT_ATHLETE,
        source_path: library_path(start_utc, decoded.utc_offset_s, &decoded.content_hash),
        content_hash: decoded.content_hash,
        fit_file_id: decoded.fit_file_id,
        start_utc,
        utc_offset_s: decoded.utc_offset_s,
        is_indoor: decoded.is_indoor,
        elapsed_time_s: summary.elapsed_time_s,
        moving_time_s: summary.moving_time_s,
        distance_m: summary.distance_m,
        elevation_gain_m: summary.elevation_gain_m,
        average_power_w: summary.average_power_w,
        average_hr_bpm: summary.average_hr_bpm,
        summary_version: summary.version,
        imported_at: now,
    };
    let ActivityId(activity_id) = store(conn, library, &bytes, &record).map_err(failed)?;
    Ok(Outcome::Imported {
        activity_id,
        overlaps,
    })
}

fn failed(error: impl ToString) -> Outcome {
    Outcome::Failed {
        reason: error.to_string(),
    }
}

/// Copies the Source File into the Library, then records its Activity. When
/// either fails, nothing is left behind: no file without its row, no row
/// without its file.
fn store(
    conn: &Connection,
    library: &Path,
    bytes: &[u8],
    record: &ActivityRecord,
) -> Result<ActivityId, String> {
    let target = library.join(&record.source_path);
    write_new(&target, bytes).map_err(|e| format!("cannot copy into the Library: {e}"))?;
    db::insert_activity(conn, record).map_err(|e| {
        // Best effort: the row is what makes the file an Activity.
        let _ = fs::remove_file(&target);
        format!("cannot record the Activity: {e}")
    })
}

/// Writes `bytes`, the very bytes that were hashed, to a new file at
/// `target`, creating its folder. Never overwrites a file; removes a partly
/// written one.
fn write_new(target: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(folder) = target.parent() {
        fs::create_dir_all(folder)?;
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .inspect_err(|_| {
            let _ = fs::remove_file(target);
        })
}

/// Hex digits of the content hash that tell apart two rides started in the
/// same minute.
const SHORT_HASH_LEN: usize = 8;

/// Where a Source File goes in the Library, relative to it and `/`-separated:
/// `YYYY/YYYY-MM-DD_HHMM_<short hash>.fit`, from the ride's local start time,
/// so the names stay meaningful without DynoWatt.
fn library_path(start_utc: DateTime<Utc>, utc_offset_s: i32, content_hash: &str) -> String {
    let local = start_utc.naive_utc() + TimeDelta::seconds(utc_offset_s.into());
    let short_hash = content_hash.get(..SHORT_HASH_LEN).unwrap_or(content_hash);
    format!("{}_{short_hash}.fit", local.format("%Y/%Y-%m-%d_%H%M"))
}

/// A folder the scan could not list, and why.
#[derive(Debug, PartialEq)]
struct Unlisted {
    folder: PathBuf,
    reason: String,
}

/// The files a run covers, in order: each file as given, whatever its
/// extension, and for each folder the `.fit` files below it.
fn source_files(paths: &[PathBuf]) -> Vec<Result<PathBuf, Unlisted>> {
    let mut files = Vec::new();
    for path in paths {
        if path.is_dir() {
            collect_fit_files(path, &mut files);
        } else {
            files.push(Ok(path.clone()));
        }
    }
    files
}

/// Adds the `.fit` files below `folder`, any case, sorted by path. Linked
/// folders are not followed, so a link loop cannot trap the scan.
fn collect_fit_files(folder: &Path, files: &mut Vec<Result<PathBuf, Unlisted>>) {
    let entries = match fs::read_dir(folder).and_then(Iterator::collect::<io::Result<Vec<_>>>) {
        Ok(entries) => entries,
        Err(e) => {
            files.push(Err(Unlisted {
                folder: folder.to_owned(),
                reason: e.to_string(),
            }));
            return;
        }
    };
    let mut entries: Vec<_> = entries
        .into_iter()
        .filter_map(|entry| Some((entry.path(), entry.file_type().ok()?)))
        .collect();
    entries.sort_by(|(a, _), (b, _)| a.cmp(b));
    for (path, kind) in entries {
        if kind.is_dir() {
            collect_fit_files(&path, files);
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("fit"))
        {
            files.push(Ok(path));
        }
    }
}
