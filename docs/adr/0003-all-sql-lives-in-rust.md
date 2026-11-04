# All SQL lives in Rust; the frontend only calls Tauri commands

The database is accessed exclusively from Rust through `rusqlite` (bundled SQLite) with `rusqlite_migration`, and the Vue frontend talks only to typed Tauri commands. We rejected `tauri-plugin-sql`, the official and most obvious option, because it exposes raw SQL to the frontend, which would scatter domain logic between TypeScript and Rust when the Rust core is meant to own it.

## Considered Options

- **`tauri-plugin-sql`**: official, quick to start, but frontend-facing SQL and a sqlx dependency that can collide with `rusqlite` on `libsqlite3-sys`.
- **`sqlx`**: async, but compile-time checked queries need a live database or a committed `.sqlx/` cache at build time: overhead with no payoff for a single-user embedded database.
- **An ORM (Diesel, SeaORM)**: typed queries, but migrations stay hand-written SQL anyway, and the app has two tables and a handful of queries while its heavy lifting (MMP, W'bal) happens in memory on Samples, where no ORM helps. Diesel's CLI, generated `schema.rs` and learning curve buy too little; SeaORM also brings `sqlx` back.

## Consequences

- Every SQL statement lives in a single `db` module exposing typed Rust functions (`insert_activity`, `find_duplicate`, `list_activities`, …); no other module writes SQL. Should queries multiply, moving to Diesel would touch only that module.
- `rusqlite_migration` 2.6 requires `rusqlite` 0.40, so `rusqlite` is pinned to 0.40.x until the migration crate catches up.
- `.FIT` decoding uses `fitparser` behind an internal module that yields our own Samples, so swapping decoder (e.g. to `rustyfit`) touches only that module. That choice is reversible and deliberately not recorded as an ADR.
