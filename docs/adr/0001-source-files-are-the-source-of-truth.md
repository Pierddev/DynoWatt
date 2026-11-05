# Source Files are the source of truth; SQLite holds only metadata and derived metrics

On import, the original `.FIT` file is copied unmodified into the Library, and SQLite stores only per-Activity metadata and derived metrics (summary values, MMP), never the per-second Samples. A 5-hour ride is ~18,000 Samples, so storing them row-by-row would bloat the database into millions of rows, while keeping the original file means every derived metric can be recomputed when an algorithm improves and the Athlete always owns a portable, tool-agnostic copy of their data. Samples are re-decoded from the Source File when an Activity is opened.

## Considered Options

- **One SQLite row per Sample**: simplest queries, but the database grows by millions of rows and duplicates what the Source File already holds.
- **Samples as a compressed blob per Activity in SQLite**: fast to load, but still a second copy of the data. Kept as a possible later cache if re-decoding proves too slow, not as the source of truth.

## Consequences

- Losing the Library loses the data; the database alone cannot rebuild Activities. Backups must cover the Library.
- Opening an Activity costs a `.FIT` decode, so decoding must stay fast.
