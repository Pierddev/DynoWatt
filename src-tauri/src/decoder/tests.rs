use std::path::Path;

use chrono::{DateTime, Duration, TimeZone, Utc};

use super::fixtures::{Event, EventType, FileId, FitBuilder, Point, Sport, SubSport};
use super::*;
use crate::db::FitFileId;

/// `s` seconds after the synthetic rides' first record.
fn at(s: i64) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 6, 1, 7, 0, 0).unwrap() + Duration::seconds(s)
}

/// A road ride, no record yet.
fn cycling() -> FitBuilder {
    FitBuilder::new().session(Sport::CYCLING, SubSport::ROAD)
}

/// A road ride with a single record: the smallest file `decode` accepts.
fn ride() -> FitBuilder {
    cycling().record(at(0), Point::default())
}

fn decoded(bytes: &[u8]) -> DecodedActivity {
    decode(bytes).expect("a decodable file")
}

fn is_unreadable(result: Result<DecodedActivity, DecodeError>) -> bool {
    matches!(result, Err(DecodeError::Unreadable(_)))
}

#[test]
fn the_file_id_triple_is_decoded() {
    let id = FileId {
        manufacturer: 1,
        serial_number: 3_999_999_999,
        time_created: at(-60),
    };
    assert_eq!(
        decoded(&ride().file_id(id).build()).fit_file_id,
        FitFileId {
            manufacturer: Some(1),
            serial_number: Some(3_999_999_999),
            time_created: Some(at(-60)),
        }
    );
}

#[test]
fn a_missing_file_id_triple_decodes_as_unknown() {
    assert_eq!(decoded(&ride().build()).fit_file_id, FitFileId::default());
}

#[test]
fn the_content_hash_is_the_sha256_of_the_whole_file() {
    assert_eq!(
        content_hash(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    let bytes = ride().build();
    assert_eq!(decoded(&bytes).content_hash, content_hash(&bytes));
}

#[test]
fn a_corrupt_file_is_unreadable() {
    let mut bytes = ride().build();
    let middle = bytes.len() / 2;
    bytes[middle] ^= 0xFF;
    assert!(is_unreadable(decode(&bytes)));
}

#[test]
fn a_truncated_file_is_unreadable() {
    let bytes = ride().build();
    assert!(is_unreadable(decode(&bytes[..bytes.len() - 10])));
}

#[test]
fn bytes_that_are_not_fit_are_unreadable() {
    assert!(is_unreadable(decode(b"not a FIT file")));
}

fn is_not_cycling(result: Result<DecodedActivity, DecodeError>) -> bool {
    matches!(result, Err(DecodeError::NotCycling))
}

#[test]
fn a_road_ride_is_an_outdoor_activity() {
    assert!(!decoded(&ride().build()).is_indoor);
}

#[test]
fn a_trail_ride_is_cycling_too() {
    let bytes = FitBuilder::new()
        .session(Sport::CYCLING, SubSport::MOUNTAIN)
        .record(at(0), Point::default())
        .build();
    assert!(!decoded(&bytes).is_indoor);
}

#[test]
fn home_trainer_sub_sports_make_an_indoor_activity() {
    for sub_sport in [
        SubSport::INDOOR_CYCLING,
        SubSport::VIRTUAL_ACTIVITY,
        SubSport::SPIN,
    ] {
        let bytes = FitBuilder::new()
            .session(Sport::CYCLING, sub_sport)
            .record(at(0), Point::default())
            .build();
        assert!(decoded(&bytes).is_indoor, "{sub_sport:?}");
    }
}

#[test]
fn a_ride_without_sub_sport_is_outdoor() {
    // 0xFF is FIT's invalid enum, which decoders treat as an absent field.
    let bytes = FitBuilder::new()
        .session(Sport::CYCLING, SubSport(0xFF))
        .record(at(0), Point::default())
        .build();
    assert!(!decoded(&bytes).is_indoor);
}

#[test]
fn a_non_cycling_file_is_rejected() {
    for sport in [Sport::RUNNING, Sport::E_BIKING] {
        let bytes = FitBuilder::new()
            .session(sport, SubSport::GENERIC)
            .record(at(0), Point::default())
            .build();
        assert!(is_not_cycling(decode(&bytes)), "{sport:?}");
    }
}

#[test]
fn a_file_recording_no_sport_is_rejected() {
    let bytes = FitBuilder::new().record(at(0), Point::default()).build();
    assert!(is_not_cycling(decode(&bytes)));
}

#[test]
fn the_sport_message_stands_in_for_a_missing_session() {
    let bytes = FitBuilder::new()
        .sport(Sport::CYCLING, SubSport::INDOOR_CYCLING)
        .record(at(0), Point::default())
        .build();
    assert!(decoded(&bytes).is_indoor);
}

#[test]
fn the_session_sport_wins_over_the_sport_message() {
    let bytes = FitBuilder::new()
        .sport(Sport::CYCLING, SubSport::ROAD)
        .session(Sport::RUNNING, SubSport::GENERIC)
        .record(at(0), Point::default())
        .build();
    assert!(is_not_cycling(decode(&bytes)));
}

fn power(watts: u16) -> Point {
    Point {
        power_w: Some(watts),
        ..Default::default()
    }
}

/// Which seconds of the timeline belong to a Gap.
fn gaps(bytes: &[u8]) -> Vec<bool> {
    let seconds = decoded(bytes).timeline.seconds;
    seconds.iter().map(Option::is_none).collect()
}

fn close(value: Option<f64>, expected: f64) -> bool {
    value.is_some_and(|v| (v - expected).abs() < 1e-9)
}

#[test]
fn steady_one_hertz_recording_gives_one_sample_per_record() {
    let sample = |s: u16| Sample {
        power_w: Some(200 + s),
        heart_rate_bpm: Some(140),
        cadence_rpm: Some(90),
        altitude_m: Some(120.0),
        speed_m_s: Some(8.5),
        distance_m: Some(10.25 * f64::from(s)),
    };
    let mut file = cycling().timer(at(0), EventType::START);
    for s in 0..5 {
        let Sample {
            power_w,
            heart_rate_bpm,
            cadence_rpm,
            altitude_m,
            speed_m_s,
            distance_m,
        } = sample(s);
        let point = Point {
            power_w,
            heart_rate_bpm,
            cadence_rpm,
            altitude_m,
            speed_m_s,
            distance_m,
        };
        file = file.record(at(i64::from(s)), point);
    }
    let timeline = decoded(&file.timer(at(4), EventType::STOP_ALL).build()).timeline;
    assert_eq!(timeline.start_utc, at(0));
    assert_eq!(
        timeline.seconds,
        (0..5).map(|s| Some(sample(s))).collect::<Vec<_>>()
    );
}

#[test]
fn smart_recording_holes_are_filled() {
    let bytes = cycling()
        .timer(at(0), EventType::START)
        .record(
            at(0),
            Point {
                power_w: Some(100),
                heart_rate_bpm: Some(120),
                cadence_rpm: Some(80),
                altitude_m: Some(100.0),
                speed_m_s: Some(5.0),
                distance_m: Some(0.0),
            },
        )
        .record(
            at(4),
            Point {
                power_w: Some(200),
                heart_rate_bpm: Some(130),
                cadence_rpm: Some(90),
                altitude_m: Some(104.0),
                speed_m_s: Some(9.0),
                distance_m: Some(20.0),
            },
        )
        .build();
    let seconds = decoded(&bytes).timeline.seconds;
    assert_eq!(seconds.len(), 5);
    for (k, second) in seconds.iter().enumerate().take(4).skip(1) {
        let s = second.expect("a filled second");
        let k = k as f64;
        // Power, heart rate and cadence repeat the last value…
        assert_eq!(
            (s.power_w, s.heart_rate_bpm, s.cadence_rpm),
            (Some(100), Some(120), Some(80))
        );
        // …altitude, speed and distance are interpolated.
        assert!(close(s.altitude_m, 100.0 + k), "{s:?}");
        assert!(close(s.speed_m_s, 5.0 + k), "{s:?}");
        assert!(close(s.distance_m, 5.0 * k), "{s:?}");
    }
    assert_eq!(seconds[4].unwrap().power_w, Some(200));
}

#[test]
fn interpolation_needs_a_value_at_both_ends() {
    let bytes = cycling()
        .record(
            at(0),
            Point {
                altitude_m: Some(100.0),
                ..Default::default()
            },
        )
        .record(
            at(2),
            Point {
                speed_m_s: Some(5.0),
                ..Default::default()
            },
        )
        .build();
    let filled = decoded(&bytes).timeline.seconds[1].expect("a filled second");
    assert_eq!(filled, Sample::default());
}

#[test]
fn a_hole_inside_an_auto_pause_is_a_gap() {
    let stops = [
        EventType::STOP,
        EventType::STOP_ALL,
        EventType::STOP_DISABLE,
        EventType::STOP_DISABLE_ALL,
    ];
    for stop in stops {
        let bytes = cycling()
            .timer(at(0), EventType::START)
            .record(at(0), power(200))
            .record(at(1), power(200))
            .timer(at(1), stop)
            .timer(at(5), EventType::START)
            .record(at(5), power(180))
            .record(at(6), power(180))
            .build();
        let expected = [false, false, true, true, true, false, false];
        assert_eq!(gaps(&bytes), expected, "{stop:?}");
    }
}

#[test]
fn a_hole_touching_a_pause_is_a_gap_as_a_whole() {
    let bytes = cycling()
        .timer(at(0), EventType::START)
        .record(at(0), power(200))
        .timer(at(5), EventType::STOP_ALL)
        .timer(at(7), EventType::START)
        .record(at(8), power(200))
        .build();
    let mut expected = vec![true; 9];
    (expected[0], expected[8]) = (false, false);
    assert_eq!(gaps(&bytes), expected);
}

#[test]
fn a_hole_ending_when_the_timer_stops_is_filled() {
    let bytes = cycling()
        .timer(at(0), EventType::START)
        .record(at(0), power(200))
        .record(at(3), power(200))
        .timer(at(3), EventType::STOP_ALL)
        .timer(at(6), EventType::START)
        .record(at(6), power(200))
        .build();
    assert_eq!(
        gaps(&bytes),
        [false, false, false, false, true, true, false]
    );
}

#[test]
fn a_hole_starting_when_the_timer_restarts_is_filled() {
    let bytes = cycling()
        .timer(at(0), EventType::START)
        .record(at(0), power(200))
        .record(at(1), power(200))
        .timer(at(1), EventType::STOP_ALL)
        .timer(at(2), EventType::START)
        .record(at(5), power(200))
        .build();
    assert_eq!(gaps(&bytes), [false; 6]);
}

#[test]
fn the_ride_between_two_pauses_keeps_running() {
    let bytes = cycling()
        .timer(at(0), EventType::START)
        .record(at(0), power(200))
        .timer(at(0), EventType::STOP_ALL)
        .timer(at(3), EventType::START)
        .record(at(3), power(200))
        .record(at(6), power(200))
        .timer(at(6), EventType::STOP_ALL)
        .timer(at(9), EventType::START)
        .record(at(9), power(200))
        .build();
    let expected = [
        false, true, true, false, false, false, false, true, true, false,
    ];
    assert_eq!(gaps(&bytes), expected);
}

#[test]
fn a_second_stop_does_not_move_the_start_of_a_pause() {
    // Some devices write `stop` then `stop_all` for the same pause.
    let bytes = cycling()
        .timer(at(0), EventType::START)
        .record(at(0), power(200))
        .timer(at(0), EventType::STOP)
        .record(at(2), power(0))
        .timer(at(2), EventType::STOP_ALL)
        .timer(at(4), EventType::START)
        .record(at(4), power(200))
        .build();
    assert_eq!(gaps(&bytes), [false, true, false, true, false]);
}

#[test]
fn a_timer_stopped_for_good_makes_every_later_hole_a_gap() {
    let bytes = cycling()
        .timer(at(0), EventType::START)
        .record(at(0), power(200))
        .timer(at(0), EventType::STOP_ALL)
        .record(at(3), power(0))
        .build();
    assert_eq!(gaps(&bytes), [false, true, true, false]);
}

#[test]
fn a_lap_event_does_not_pause_the_timer() {
    let bytes = cycling()
        .timer(at(0), EventType::START)
        .record(at(0), power(200))
        .event(at(0), Event::LAP, EventType::STOP)
        .record(at(3), power(200))
        .build();
    assert_eq!(gaps(&bytes), [false; 4]);
}

#[test]
fn a_timer_marker_does_not_pause_the_timer() {
    let bytes = cycling()
        .timer(at(0), EventType::START)
        .record(at(0), power(200))
        .timer(at(0), EventType::MARKER)
        .record(at(3), power(200))
        .build();
    assert_eq!(gaps(&bytes), [false; 4]);
}

#[test]
fn without_timer_events_a_ten_second_hole_is_filled() {
    let bytes = cycling()
        .record(at(0), power(200))
        .record(at(11), power(200))
        .build();
    assert_eq!(gaps(&bytes), [false; 12]);
}

#[test]
fn without_timer_events_a_longer_hole_is_a_gap() {
    let bytes = cycling()
        .record(at(0), power(200))
        .record(at(12), power(200))
        .build();
    let mut expected = vec![true; 13];
    (expected[0], expected[12]) = (false, false);
    assert_eq!(gaps(&bytes), expected);
}

#[test]
fn a_long_dropout_while_the_timer_runs_is_a_gap() {
    let bytes = cycling()
        .timer(at(0), EventType::START)
        .record(at(0), power(200))
        .record(at(30), power(200))
        .build();
    assert_eq!(gaps(&bytes).iter().filter(|g| **g).count(), 29);
}

#[test]
fn records_sharing_a_second_are_merged_the_later_winning() {
    let a = Point {
        power_w: Some(100),
        heart_rate_bpm: Some(120),
        cadence_rpm: Some(80),
        altitude_m: Some(100.0),
        speed_m_s: Some(5.0),
        distance_m: Some(10.0),
    };
    let b = Point {
        power_w: Some(200),
        heart_rate_bpm: Some(130),
        cadence_rpm: Some(90),
        altitude_m: Some(110.0),
        speed_m_s: Some(6.0),
        distance_m: Some(20.0),
    };
    let bytes = cycling()
        .record(at(0), a)
        .record(at(0), b)
        .record(at(1), a)
        .record(at(1), Point::default())
        .build();
    let seconds = decoded(&bytes).timeline.seconds;
    let sample = |p: Point| {
        Some(Sample {
            power_w: p.power_w,
            heart_rate_bpm: p.heart_rate_bpm,
            cadence_rpm: p.cadence_rpm,
            altitude_m: p.altitude_m,
            speed_m_s: p.speed_m_s,
            distance_m: p.distance_m,
        })
    };
    assert_eq!(seconds, [sample(b), sample(a)]);
}

#[test]
fn records_out_of_order_are_put_back_in_time_order() {
    let bytes = cycling()
        .record(at(1), power(2))
        .record(at(0), power(1))
        .build();
    let timeline = decoded(&bytes).timeline;
    assert_eq!(timeline.start_utc, at(0));
    let powers: Vec<_> = timeline
        .seconds
        .iter()
        .map(|s| s.unwrap().power_w)
        .collect();
    assert_eq!(powers, [Some(1), Some(2)]);
}

#[test]
fn older_devices_16_bit_altitude_and_speed_are_read() {
    let bytes = cycling().legacy_record(at(0), 100.0, 5.0).build();
    let sample = decoded(&bytes).timeline.seconds[0].unwrap();
    assert!(close(sample.altitude_m, 100.0), "{sample:?}");
    assert!(close(sample.speed_m_s, 5.0), "{sample:?}");
}

#[test]
fn an_empty_file_is_unreadable() {
    assert!(is_unreadable(decode(b"")));
}

#[test]
fn a_file_without_records_is_unreadable() {
    assert!(is_unreadable(decode(&cycling().build())));
}

#[test]
fn records_spread_over_a_week_are_accepted() {
    let bytes = cycling()
        .record(at(0), power(200))
        .record(at(timeline::MAX_TIMELINE_S), power(200))
        .build();
    assert_eq!(decoded(&bytes).timeline.seconds.len(), 7 * 24 * 3600 + 1);
}

#[test]
fn records_spread_over_more_than_a_week_are_unreadable() {
    let bytes = cycling()
        .record(at(0), power(200))
        .record(at(timeline::MAX_TIMELINE_S + 1), power(200))
        .build();
    assert!(is_unreadable(decode(&bytes)));
}

/// ADR 0003: swapping decoder must touch only this module.
#[test]
fn fitparser_is_named_only_inside_the_decoder_module() {
    fn rust_files(dir: &Path, found: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                rust_files(&path, found);
            } else if path.extension().is_some_and(|e| e == "rs") {
                found.push(path);
            }
        }
    }
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rust_files(&src, &mut files);
    let decoder = src.join("decoder");
    let offenders: Vec<_> = files
        .iter()
        .filter(|f| !f.starts_with(&decoder))
        .filter(|f| std::fs::read_to_string(f).unwrap().contains("fitparser"))
        .collect();
    assert!(files.len() > 3, "the walk found the sources");
    assert!(offenders.is_empty(), "fitparser used in {offenders:?}");
}
