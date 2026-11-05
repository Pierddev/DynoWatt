CREATE TABLE athlete (
  id          INTEGER PRIMARY KEY,
  name        TEXT NOT NULL,
  created_at  TEXT NOT NULL                -- UTC ISO-8601
);

CREATE TABLE activity (
  id                 INTEGER PRIMARY KEY,
  athlete_id         INTEGER NOT NULL REFERENCES athlete(id),
  source_path        TEXT NOT NULL UNIQUE, -- relative to the Library
  content_hash       TEXT NOT NULL UNIQUE,
  fit_manufacturer   INTEGER,
  fit_serial_number  INTEGER,
  fit_time_created   TEXT,
  start_utc          TEXT NOT NULL,
  utc_offset_s       INTEGER NOT NULL,     -- local time where the ride happened
  is_indoor          INTEGER NOT NULL,
  elapsed_time_s     INTEGER NOT NULL,
  moving_time_s      INTEGER NOT NULL,
  distance_m         REAL,
  elevation_gain_m   REAL,
  average_power_w    REAL,                 -- NULL when no power meter
  average_hr_bpm     REAL,
  summary_version    INTEGER NOT NULL,
  imported_at        TEXT NOT NULL,
  UNIQUE (fit_manufacturer, fit_serial_number, fit_time_created)
);
CREATE INDEX activity_start ON activity(athlete_id, start_utc);

-- One installation tracks a single Athlete, so the app opens straight onto an
-- empty activity list with no onboarding.
INSERT INTO athlete (id, name, created_at)
VALUES (1, 'Athlete', strftime('%Y-%m-%dT%H:%M:%SZ', 'now'));
