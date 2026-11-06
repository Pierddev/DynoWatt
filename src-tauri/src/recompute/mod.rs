//! Brings stale Activity Summaries up to date: a summary computed by rules
//! older than [`SUMMARY_VERSION`] is computed again from its Source File in
//! the Library, the source of truth (ADR 0001). One bad file never aborts the
//! run, and the run holds the database for one Activity at a time, so it can
//! go on in the background while the commands use the database.

#[cfg(test)]
mod tests;

use std::path::Path;
use std::sync::Mutex;

use rusqlite::Connection;
use serde::Serialize;

use crate::db::{self, Activity, ActivityId};
use crate::decoder;
use crate::summary::{SUMMARY_VERSION, summarize};

/// What became of the stale summaries, oldest import first.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct RecomputeReport {
    /// The Activities whose summary is now up to date.
    pub recomputed: Vec<ActivityId>,
    /// The Activities left as they were, with their older figures.
    pub skipped: Vec<Skipped>,
}

/// An Activity whose summary could not be recomputed, and why: its Source
/// File is missing from the Library or cannot be decoded, or the database
/// refused the new figures.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Skipped {
    pub activity_id: ActivityId,
    pub reason: String,
}

/// Where the recompute stands, as the window follows it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Status {
    /// Going on: some summaries may still be stale.
    Running,
    /// Every stale summary was visited once.
    Done(RecomputeReport),
    /// Stopped: the database could not tell which summaries are stale.
    Failed { reason: String },
}

/// Recomputes the stale summaries and tells how the run ended.
pub fn run(database: &Mutex<Connection>, library: &Path) -> Status {
    match stale_summaries(database, library) {
        Ok(report) => Status::Done(report),
        Err(reason) => Status::Failed { reason },
    }
}

/// Recomputes every summary older than [`SUMMARY_VERSION`] from the Source
/// Files in `library`, each once. Fails only when the database is unavailable
/// or cannot list the stale summaries.
pub fn stale_summaries(
    database: &Mutex<Connection>,
    library: &Path,
) -> Result<RecomputeReport, String> {
    let mut report = RecomputeReport::default();
    let mut after = None;
    loop {
        // Held for this one Activity only: the commands get the database in
        // between, and an Activity they delete is never met.
        let conn = database
            .lock()
            .map_err(|_| "the database is unavailable".to_owned())?;
        let Some(activity) = db::next_stale_summary(&conn, SUMMARY_VERSION, after)
            .map_err(|e| format!("cannot list the stale summaries: {e}"))?
        else {
            return Ok(report);
        };
        after = Some(activity.id);
        match recompute(&conn, library, &activity) {
            Ok(()) => report.recomputed.push(activity.id),
            Err(reason) => report.skipped.push(Skipped {
                activity_id: activity.id,
                reason,
            }),
        }
    }
}

/// Computes the summary of `activity` again from its Source File and records
/// it with the current version.
fn recompute(conn: &Connection, library: &Path, activity: &Activity) -> Result<(), String> {
    let decoded = decoder::decode_file(&library.join(&activity.record.source_path))?;
    let summary = summarize(&decoded.timeline, &decoded.device_totals);
    db::update_summary(conn, activity.id, &summary)
        .map_err(|e| format!("cannot record the summary: {e}"))
}
