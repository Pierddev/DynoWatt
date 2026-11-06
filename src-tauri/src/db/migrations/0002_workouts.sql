-- The Athlete authors Workouts in DynoWatt, so these rows are their only copy
-- (ADR 0005). The rules of a valid Workout are checked in Rust before saving.
CREATE TABLE workout (
  id          INTEGER PRIMARY KEY,
  athlete_id  INTEGER NOT NULL REFERENCES athlete(id),
  name        TEXT NOT NULL
);

-- A Workout's sequence, in riding order: a Step on its own when repeat_times
-- is NULL, otherwise a Repeat whose Steps are ridden repeat_times times.
CREATE TABLE workout_item (
  workout_id    INTEGER NOT NULL REFERENCES workout(id) ON DELETE CASCADE,
  position      INTEGER NOT NULL,         -- from 0
  repeat_times  INTEGER,
  PRIMARY KEY (workout_id, position)
);

-- The Steps of each item, in riding order: a single one for a Step on its own.
CREATE TABLE workout_step (
  workout_id       INTEGER NOT NULL,
  item_position    INTEGER NOT NULL,
  position         INTEGER NOT NULL,      -- from 0, within its item
  duration_s       INTEGER NOT NULL,
  kind             TEXT NOT NULL,         -- warm_up, work, recovery or cool_down
  target_low_pct   INTEGER,               -- % FTP, both NULL when ridden freely
  target_high_pct  INTEGER,
  PRIMARY KEY (workout_id, item_position, position),
  FOREIGN KEY (workout_id, item_position)
    REFERENCES workout_item(workout_id, position) ON DELETE CASCADE
);
