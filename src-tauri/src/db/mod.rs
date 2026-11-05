//! The only module that talks to SQLite (ADR 0003): every other module goes
//! through the typed functions below and never writes SQL.

mod migrations;

use std::path::Path;

use rusqlite::Connection;

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

#[cfg(test)]
mod tests;
