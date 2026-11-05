use std::path::Path;

use chrono::{DateTime, Duration, TimeZone, Utc};

use super::fixtures::{FileId, FitBuilder, Point, Sport, SubSport};
use super::*;
use crate::db::FitFileId;

/// `s` seconds after the synthetic rides' first record.
fn at(s: i64) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 6, 1, 7, 0, 0).unwrap() + Duration::seconds(s)
}

/// A road ride with a single record: the smallest file `decode` accepts.
fn ride() -> FitBuilder {
    FitBuilder::new()
        .session(Sport::CYCLING, SubSport::ROAD)
        .record(at(0), Point::default())
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
