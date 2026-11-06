use rusqlite_migration::{M, Migrations};

/// Append-only: a released migration is never edited, a new one is added.
const MIGRATION_LIST: &[M<'_>] = &[
    M::up(include_str!("migrations/0001_initial.sql")),
    M::up(include_str!("migrations/0002_workouts.sql")),
];

pub(super) const MIGRATIONS: Migrations<'_> = Migrations::from_slice(MIGRATION_LIST);
