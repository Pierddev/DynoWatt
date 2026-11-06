# Workouts live in the database; Workout Files are generated on export

ADR 0001 makes the Source File the source of truth for an Activity because the Activity comes from outside. A Workout is the opposite: the Athlete authors it in DynoWatt, so its structured form (Steps, Repeats, Power Targets) in SQLite is the source of truth, and the `.FIT` Workout File is written from it on every export and never kept. ADR 0001's "SQLite holds only metadata and derived metrics" therefore applies to Activities only.

## Considered Options

- **Workouts as `.FIT` files next to the Library**: mirrors ADR 0001 and leaves a portable file, but every edit would round-trip through FIT encoding, whose workout format flattens a Repeat into a step that points back to earlier steps by index. The editor would have to rebuild Repeats from that on every load, and our model would be bound to the FIT profile's encoding quirks.

## Consequences

- The database is the only copy of the Athlete's Workouts: a backup must cover it, not just the Library.
- A Workout File is a one-off output. Re-exporting an edited Workout writes a new file; DynoWatt neither tracks nor replaces what is already on the Bike Computer.
