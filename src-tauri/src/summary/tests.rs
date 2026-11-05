use chrono::{TimeZone, Utc};

use super::*;
use crate::decoder::Sample;

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
fn elapsed_time_counts_every_second_gaps_included() {
    assert_eq!(
        summary(vec![power(100), GAP, GAP, power(100)]).elapsed_time_s,
        4
    );
}

#[test]
fn moving_time_is_elapsed_time_minus_the_gaps() {
    assert_eq!(
        summary(vec![power(100), GAP, GAP, power(100), power(100)]).moving_time_s,
        3
    );
}

#[test]
fn a_single_sample_lasts_one_second() {
    let summary = summary(vec![Some(Sample::default())]);
    assert_eq!((summary.elapsed_time_s, summary.moving_time_s), (1, 1));
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

#[test]
fn the_summary_carries_the_version_of_its_rules() {
    assert_eq!(summary(vec![power(0)]).version, SUMMARY_VERSION);
}
