//! The Library kept in step with the database (ADR 0001): an Activity and
//! its Source File go together. A file left without its row is an orphan
//! nothing shows; a row left without its file is an Activity that cannot be
//! opened.

#[cfg(test)]
mod tests;

use std::fs;
use std::io;
use std::path::Path;

use rusqlite::Connection;

use crate::db::{self, ActivityId};
use crate::import::write_new;

/// Deletes the Activity `id` and its Source File from `library`: both, or
/// neither when one of them fails. A Source File already missing from the
/// Library does not stop the deletion. The error says why nothing was
/// deleted, for the Athlete to read.
pub fn delete_activity(
    conn: &mut Connection,
    library: &Path,
    id: ActivityId,
) -> Result<(), String> {
    let activity = db::find_activity(conn, id)
        .map_err(|e| format!("cannot find the Activity: {e}"))?
        .ok_or_else(|| format!("no Activity has the id {}", id.0))?;
    let source_file = library.join(&activity.record.source_path);
    let not_deleted = |e: rusqlite::Error| format!("cannot delete the Activity: {e}");
    // The row goes first, inside a transaction: when the file cannot be
    // removed, dropping the transaction brings the row back.
    let transaction = conn.transaction().map_err(not_deleted)?;
    db::delete_activity(&transaction, id).map_err(not_deleted)?;
    let bytes = take(&source_file)?;
    transaction.commit().map_err(|e| {
        if let Some(bytes) = bytes {
            // Best effort: the row is back, so its file must be too.
            let _ = write_new(&source_file, &bytes);
        }
        not_deleted(e)
    })
}

/// Removes the file at `path` and returns its bytes, to put it back if need
/// be, or `None` when there is no file to remove.
fn take(path: &Path) -> Result<Option<Vec<u8>>, String> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("cannot read {}: {e}", path.display())),
    };
    fs::remove_file(path).map_err(|e| format!("cannot remove {}: {e}", path.display()))?;
    Ok(Some(bytes))
}
