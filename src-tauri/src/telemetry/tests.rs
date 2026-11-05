use chrono::DateTime;

use super::*;

const GAP: Option<Sample> = None;

fn timeline(seconds: Vec<Option<Sample>>) -> Timeline {
    Timeline {
        start_utc: DateTime::parse_from_rfc3339("2026-03-14T07:00:00Z")
            .unwrap()
            .to_utc(),
        seconds,
    }
}

/// A second where every sensor recorded, each channel with its own value.
fn full(power_w: u16) -> Option<Sample> {
    Some(Sample {
        power_w: Some(power_w),
        heart_rate_bpm: Some(140),
        cadence_rpm: Some(90),
        altitude_m: Some(250.5),
        speed_m_s: Some(8.25),
        distance_m: Some(1000.0),
    })
}

/// A second where only the power meter recorded.
fn power_only(power_w: u16) -> Option<Sample> {
    Some(Sample {
        power_w: Some(power_w),
        ..Sample::default()
    })
}

#[test]
fn every_second_of_the_timeline_has_a_time_gaps_included() {
    let telemetry = columns(&timeline(vec![full(200), GAP, GAP, full(210)]));

    assert_eq!(telemetry.time_s, [0, 1, 2, 3]);
}

#[test]
fn each_channel_is_a_column_of_its_own_values() {
    let telemetry = columns(&timeline(vec![full(200), full(210)]));

    assert_eq!(
        telemetry,
        Telemetry {
            time_s: vec![0, 1],
            power_w: Some(vec![Some(200), Some(210)]),
            heart_rate_bpm: Some(vec![Some(140), Some(140)]),
            cadence_rpm: Some(vec![Some(90), Some(90)]),
            altitude_m: Some(vec![Some(250.5), Some(250.5)]),
            speed_m_s: Some(vec![Some(8.25), Some(8.25)]),
        }
    );
}

#[test]
fn a_gap_is_missing_in_every_channel_never_zero() {
    let telemetry = columns(&timeline(vec![full(200), GAP, full(210)]));

    assert_eq!(telemetry.power_w, Some(vec![Some(200), None, Some(210)]));
    assert_eq!(
        telemetry.heart_rate_bpm,
        Some(vec![Some(140), None, Some(140)])
    );
    assert_eq!(telemetry.cadence_rpm, Some(vec![Some(90), None, Some(90)]));
    assert_eq!(
        telemetry.altitude_m,
        Some(vec![Some(250.5), None, Some(250.5)])
    );
    assert_eq!(
        telemetry.speed_m_s,
        Some(vec![Some(8.25), None, Some(8.25)])
    );
}

#[test]
fn a_channel_no_sample_recorded_is_left_out() {
    let telemetry = columns(&timeline(vec![power_only(200), GAP, power_only(210)]));

    assert_eq!(telemetry.power_w, Some(vec![Some(200), None, Some(210)]));
    assert_eq!(
        (
            telemetry.heart_rate_bpm,
            telemetry.cadence_rpm,
            telemetry.altitude_m,
            telemetry.speed_m_s
        ),
        (None, None, None, None)
    );
}

#[test]
fn a_channel_recorded_for_a_single_second_is_kept() {
    let telemetry = columns(&timeline(vec![power_only(200), full(210), power_only(220)]));

    assert_eq!(telemetry.heart_rate_bpm, Some(vec![None, Some(140), None]));
}

#[test]
fn a_channel_recorded_as_zeros_is_kept() {
    let telemetry = columns(&timeline(vec![power_only(0), power_only(0)]));

    assert_eq!(telemetry.power_w, Some(vec![Some(0), Some(0)]));
}

#[test]
fn an_empty_timeline_has_no_channel() {
    assert_eq!(
        columns(&timeline(vec![])),
        Telemetry {
            time_s: vec![],
            power_w: None,
            heart_rate_bpm: None,
            cadence_rpm: None,
            altitude_m: None,
            speed_m_s: None,
        }
    );
}

#[test]
fn altitude_is_rounded_to_the_decimetre_and_speed_to_the_millimetre_per_second() {
    let second = |altitude_m: f64, speed_m_s: f64| {
        Some(Sample {
            altitude_m: Some(altitude_m),
            speed_m_s: Some(speed_m_s),
            ..Sample::default()
        })
    };

    let telemetry = columns(&timeline(vec![
        second(123.46, 8.1236),
        // What a rescaled FIT altitude looks like: 623.4 m − 500 m.
        second(123.399_999_999_999_98, 0.0),
        second(-12.34, 15.0),
    ]));

    assert_eq!(
        telemetry.altitude_m,
        Some(vec![Some(123.5), Some(123.4), Some(-12.3)])
    );
    assert_eq!(
        telemetry.speed_m_s,
        Some(vec![Some(8.124), Some(0.0), Some(15.0)])
    );
}

#[test]
fn telemetry_serialises_as_the_frontend_reads_it() {
    let telemetry = columns(&timeline(vec![
        power_only(200),
        GAP,
        Some(Sample {
            power_w: Some(210),
            altitude_m: Some(123.399_999_999_999_98),
            ..Sample::default()
        }),
    ]));

    assert_eq!(
        serde_json::to_string(&telemetry).unwrap(),
        r#"{"timeS":[0,1,2],"powerW":[200,null,210],"heartRateBpm":null,"cadenceRpm":null,"altitudeM":[null,null,123.4],"speedMS":null}"#
    );
}

#[test]
fn a_five_hour_ride_with_every_sensor_fits_in_a_megabyte() {
    let seconds = (0..5 * 3600)
        .map(|s: u32| {
            Some(Sample {
                power_w: Some(150 + (s % 300) as u16),
                heart_rate_bpm: Some(100 + (s % 90) as u8),
                cadence_rpm: Some(60 + (s % 50) as u8),
                // Interpolated values, with every digit a double can hold.
                altitude_m: Some(1000.0 + f64::from(s) / 7.0),
                speed_m_s: Some(5.0 + f64::from(s % 100) / 7.0),
                distance_m: Some(f64::from(s) * 8.0),
            })
        })
        .collect();

    let json = serde_json::to_string(&columns(&timeline(seconds))).unwrap();

    assert!(json.len() < 1_000_000, "{} bytes", json.len());
}
