use chrono::{Duration, TimeZone, Utc};

use super::*;
use crate::decoder::fixtures::{EventType, FitBuilder, Point, Sport, SubSport, Totals};
use crate::decoder::{Sample, decode};

/// A second of a Gap.
const GAP: Option<Sample> = None;

/// A timeline of `seconds`, starting at an arbitrary instant.
fn timeline(seconds: Vec<Option<Sample>>) -> Timeline {
    Timeline {
        start_utc: Utc.with_ymd_and_hms(2026, 6, 1, 7, 0, 0).unwrap(),
        seconds,
    }
}

/// The summary of `seconds`, from a device that wrote no totals.
fn summary(seconds: Vec<Option<Sample>>) -> ActivitySummary {
    summarize(&timeline(seconds), &DeviceTotals::default())
}

fn power(watts: u16) -> Option<Sample> {
    Some(Sample {
        power_w: Some(watts),
        ..Default::default()
    })
}

fn heart_rate(bpm: u8) -> Option<Sample> {
    Some(Sample {
        heart_rate_bpm: Some(bpm),
        ..Default::default()
    })
}

#[test]
fn elapsed_time_runs_from_the_first_sample_to_the_last_gaps_included() {
    assert_eq!(
        summary(vec![power(100), GAP, GAP, power(100)]).elapsed_time_s,
        3
    );
}

#[test]
fn moving_time_is_elapsed_time_minus_the_gaps() {
    // The Gap runs from the Sample before it, at 2 s, to the one after it, at
    // 5 s.
    let seconds = vec![
        power(100),
        power(100),
        power(100),
        GAP,
        GAP,
        power(100),
        power(100),
    ];
    let summary = summary(seconds);
    assert_eq!((summary.elapsed_time_s, summary.moving_time_s), (6, 3));
}

#[test]
fn a_single_sample_lasts_no_time() {
    let summary = summary(vec![Some(Sample::default())]);
    assert_eq!((summary.elapsed_time_s, summary.moving_time_s), (0, 0));
}

#[test]
fn coasting_zeros_count_in_the_average_power() {
    let summary = summary(vec![power(300), power(300), power(0), power(0)]);
    assert_eq!(summary.average_power_w, Some(150.0));
}

#[test]
fn gaps_do_not_count_in_the_average_power() {
    let summary = summary(vec![power(200), GAP, GAP, GAP, power(100)]);
    assert_eq!(summary.average_power_w, Some(150.0));
}

#[test]
fn seconds_without_power_are_left_out_of_the_average() {
    let summary = summary(vec![power(300), heart_rate(150), power(100)]);
    assert_eq!(summary.average_power_w, Some(200.0));
}

#[test]
fn a_ride_without_power_meter_has_no_average_power() {
    let summary = summary(vec![heart_rate(120), heart_rate(130)]);
    assert_eq!(summary.average_power_w, None);
}

#[test]
fn average_heart_rate_skips_gaps_and_seconds_without_heart_rate() {
    let summary = summary(vec![heart_rate(120), GAP, power(0), heart_rate(150)]);
    assert_eq!(summary.average_hr_bpm, Some(135.0));
}

#[test]
fn a_ride_without_heart_rate_strap_has_no_average_heart_rate() {
    assert_eq!(summary(vec![power(200), power(0)]).average_hr_bpm, None);
}

#[test]
fn the_device_totals_are_kept() {
    let device = DeviceTotals {
        distance_m: Some(42_195.0),
        elevation_gain_m: Some(512.0),
    };
    let summary = summarize(&timeline(vec![power(0)]), &device);
    assert_eq!(
        (summary.distance_m, summary.elevation_gain_m),
        (Some(42_195.0), Some(512.0))
    );
}

fn distance(metres: f64) -> Option<Sample> {
    Some(Sample {
        distance_m: Some(metres),
        ..Default::default()
    })
}

fn altitude(metres: f64) -> Option<Sample> {
    Some(Sample {
        altitude_m: Some(metres),
        ..Default::default()
    })
}

/// The summary of `seconds`, from a device that wrote `device`.
fn summary_with(device: DeviceTotals, seconds: Vec<Option<Sample>>) -> ActivitySummary {
    summarize(&timeline(seconds), &device)
}

fn device_distance(metres: f64) -> DeviceTotals {
    DeviceTotals {
        distance_m: Some(metres),
        ..Default::default()
    }
}

fn device_elevation_gain(metres: f64) -> DeviceTotals {
    DeviceTotals {
        elevation_gain_m: Some(metres),
        ..Default::default()
    }
}

#[test]
fn the_device_distance_wins_over_the_samples() {
    let summary = summary_with(
        device_distance(1000.0),
        vec![distance(0.0), distance(900.0)],
    );
    assert_eq!(summary.distance_m, Some(1000.0));
}

#[test]
fn without_a_device_distance_the_last_sample_distance_is_used() {
    let seconds = vec![distance(0.0), distance(10.0), GAP, distance(30.0), power(0)];
    assert_eq!(summary(seconds).distance_m, Some(30.0));
}

#[test]
fn a_ride_without_distance_has_none() {
    assert_eq!(summary(vec![power(200), power(0)]).distance_m, None);
}

#[test]
fn a_device_zero_distance_without_distance_in_the_samples_is_none() {
    let summary = summary_with(device_distance(0.0), vec![power(200), power(0)]);
    assert_eq!(summary.distance_m, None);
}

#[test]
fn a_device_zero_distance_gives_way_to_the_samples() {
    let summary = summary_with(device_distance(0.0), vec![distance(0.0), distance(8.0)]);
    assert_eq!(summary.distance_m, Some(8.0));
}

#[test]
fn the_device_elevation_gain_wins_over_the_samples() {
    let summary = summary_with(
        device_elevation_gain(512.0),
        vec![altitude(100.0), altitude(110.0)],
    );
    assert_eq!(summary.elevation_gain_m, Some(512.0));
}

#[test]
fn without_a_device_elevation_gain_the_climbs_of_the_samples_add_up() {
    let altitudes = [100.0, 105.0, 110.0, 104.0, 102.0, 108.0];
    let summary = summary(altitudes.into_iter().map(altitude).collect());
    assert_eq!(summary.elevation_gain_m, Some(16.0));
}

#[test]
fn altitude_noise_is_not_a_climb() {
    let altitudes = [100.0, 101.0, 100.0, 101.5, 100.0, 103.0, 101.0];
    let summary = summary(altitudes.into_iter().map(altitude).collect());
    assert_eq!(summary.elevation_gain_m, Some(0.0));
}

#[test]
fn a_dip_no_deeper_than_the_hysteresis_does_not_end_a_climb() {
    let altitudes = [100.0, 104.0, 110.0, 107.0, 111.0];
    let summary = summary(altitudes.into_iter().map(altitude).collect());
    assert_eq!(summary.elevation_gain_m, Some(11.0));
}

#[test]
fn a_new_climb_starts_from_the_drop_that_ended_the_last_one() {
    let altitudes = [100.0, 110.0, 106.0, 109.5];
    let summary = summary(altitudes.into_iter().map(altitude).collect());
    assert_eq!(summary.elevation_gain_m, Some(13.5));
}

#[test]
fn a_climb_across_a_gap_counts() {
    let summary = summary(vec![altitude(100.0), GAP, GAP, altitude(110.0)]);
    assert_eq!(summary.elevation_gain_m, Some(10.0));
}

#[test]
fn a_ride_without_altitude_has_no_elevation_gain() {
    assert_eq!(summary(vec![power(200), power(0)]).elevation_gain_m, None);
}

#[test]
fn a_device_zero_elevation_gain_without_altitude_in_the_samples_is_none() {
    let summary = summary_with(device_elevation_gain(0.0), vec![power(200), power(0)]);
    assert_eq!(summary.elevation_gain_m, None);
}

#[test]
fn a_device_zero_elevation_gain_gives_way_to_the_samples() {
    let summary = summary_with(
        device_elevation_gain(0.0),
        vec![altitude(100.0), altitude(110.0)],
    );
    assert_eq!(summary.elevation_gain_m, Some(10.0));
}

/// The summary of a Source File, decoded.
fn summary_of(bytes: &[u8]) -> ActivitySummary {
    let decoded = decode(bytes).expect("a decodable file");
    summarize(&decoded.timeline, &decoded.device_totals)
}

/// Two seconds of riding: 8 m and 4 m of climbing.
fn two_records(fit: FitBuilder) -> FitBuilder {
    let start = Utc.with_ymd_and_hms(2026, 6, 1, 7, 0, 0).unwrap();
    let point = |distance_m, altitude_m| Point {
        distance_m: Some(distance_m),
        altitude_m: Some(altitude_m),
        ..Default::default()
    };
    fit.record(start, point(0.0, 100.0))
        .record(start + Duration::seconds(1), point(8.0, 104.0))
}

#[test]
fn a_decoded_session_gives_distance_and_elevation_gain() {
    let totals = Totals {
        distance_m: Some(10.0),
        ascent_m: Some(5),
    };
    let summary = summary_of(
        &two_records(FitBuilder::new().session_with(Sport::CYCLING, SubSport::ROAD, totals))
            .build(),
    );
    assert_eq!(
        (summary.distance_m, summary.elevation_gain_m),
        (Some(10.0), Some(5.0))
    );
}

#[test]
fn without_a_session_message_the_samples_give_distance_and_elevation_gain() {
    let summary =
        summary_of(&two_records(FitBuilder::new().sport(Sport::CYCLING, SubSport::ROAD)).build());
    assert_eq!(
        (summary.distance_m, summary.elevation_gain_m),
        (Some(8.0), Some(4.0))
    );
}

#[test]
fn moving_time_is_the_timer_time_when_each_timer_stop_and_start_has_a_record() {
    let start = Utc.with_ymd_and_hms(2026, 6, 1, 7, 0, 0).unwrap();
    let at = |s| start + Duration::seconds(s);
    // Running from 0 to 10 s and from 15 to 40 s, with a record at each end,
    // as Garmin devices write them: 35 s of timer time. The pause is shorter
    // than a hole the decoder fills, so only the timer makes it a Gap.
    let mut fit = FitBuilder::new().session(Sport::CYCLING, SubSport::ROAD);
    for running in [0..=10, 15..=40] {
        fit = fit.timer(at(*running.start()), EventType::START);
        for s in running.clone() {
            let point = Point {
                power_w: Some(200),
                ..Default::default()
            };
            fit = fit.record(at(s), point);
        }
        fit = fit.timer(at(*running.end()), EventType::STOP_ALL);
    }

    let summary = summary_of(&fit.build());

    assert_eq!((summary.elapsed_time_s, summary.moving_time_s), (40, 35));
}

#[test]
fn summaries_of_version_1_are_stale() {
    // Version 1 counted each record as a second, so each running stretch
    // lasted one second too long.
    const { assert!(SUMMARY_VERSION > 1) };
}

#[test]
fn the_summary_carries_the_version_of_its_rules() {
    assert_eq!(summary(vec![power(0)]).version, SUMMARY_VERSION);
}
