//! Brings stale Activity Summaries up to date: a summary computed by rules
//! older than [`SUMMARY_VERSION`] is computed again from its Source File in
//! the Library, the source of truth (ADR 0001). One bad file never aborts the
//! run.

#[cfg(test)]
mod tests;

use std::fs;
use std::path::Path;

use rusqlite::Connection;

use crate::db::{self, Activity, ActivityId};
use crate::decoder;
use crate::summary::{SUMMARY_VERSION, summarize};

/// What became of the stale summaries, oldest import first.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RecomputeReport {
    /// The Activities whose summary is now up to date.
    pub recomputed: Vec<ActivityId>,
    /// The Activities left as they were, with their older figures.
    pub skipped: Vec<Skipped>,
}

/// An Activity whose summary could not be recomputed, and why: its Source
/// File is missing from the Library or cannot be decoded, or the database
/// refused the new figures.
#[derive(Debug, Clone, PartialEq)]
pub struct Skipped {
    pub activity_id: ActivityId,
    pub reason: String,
}

/// Recomputes every summary older than [`SUMMARY_VERSION`] from the Source
/// Files in `library`. Fails only when the stale summaries cannot be listed.
pub fn stale_summaries(conn: &Connection, library: &Path) -> rusqlite::Result<RecomputeReport> {
    let mut report = RecomputeReport::default();
    for activity in db::list_stale_summaries(conn, SUMMARY_VERSION)? {
        match recompute(conn, library, &activity) {
            Ok(()) => report.recomputed.push(activity.id),
            Err(reason) => report.skipped.push(Skipped {
                activity_id: activity.id,
                reason,
            }),
        }
    }
    Ok(report)
}

/// Computes the summary of `activity` again from its Source File and records
/// it with the current version.
fn recompute(conn: &Connection, library: &Path, activity: &Activity) -> Result<(), String> {
    let file = library.join(&activity.record.source_path);
    let bytes = fs::read(&file).map_err(|e| format!("cannot read {}: {e}", file.display()))?;
    let decoded =
        decoder::decode(&bytes).map_err(|e| format!("cannot decode {}: {e}", file.display()))?;
    let summary = summarize(&decoded.timeline, &decoded.device_totals);
    db::update_summary(conn, activity.id, &summary)
        .map_err(|e| format!("cannot record the summary: {e}"))
}
