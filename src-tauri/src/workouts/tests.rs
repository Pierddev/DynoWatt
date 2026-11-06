use serde_json::json;

use super::*;

fn step(duration_s: u32, kind: StepKind, target: Option<(u16, u16)>) -> Step {
    Step {
        duration_s,
        kind,
        target: target.map(|(low_pct, high_pct)| PowerTarget { low_pct, high_pct }),
    }
}

/// A valid Workout: Warm-up, 3 × (Work, Recovery), Cool-down.
fn sweet_spot() -> Workout {
    Workout {
        name: "Sweet spot".to_owned(),
        items: vec![
            Item::Step(step(600, StepKind::WarmUp, Some((50, 75)))),
            Item::Repeat(Repeat {
                times: 3,
                steps: vec![
                    step(480, StepKind::Work, Some((88, 94))),
                    step(120, StepKind::Recovery, None),
                ],
            }),
            Item::Step(step(300, StepKind::CoolDown, None)),
        ],
    }
}

fn with_target(low_pct: u16, high_pct: u16) -> Workout {
    Workout {
        items: vec![Item::Step(step(
            60,
            StepKind::Work,
            Some((low_pct, high_pct)),
        ))],
        ..sweet_spot()
    }
}

fn rules(workout: &Workout) -> Vec<Rule> {
    validate(workout)
        .into_iter()
        .map(|problem| problem.rule)
        .collect()
}

#[test]
fn a_workout_travels_to_the_window_as_tagged_json() {
    let json = json!({
        "name": "Sweet spot",
        "items": [
            {"type": "step", "durationS": 600, "kind": "warmUp", "target": {"lowPct": 50, "highPct": 75}},
            {"type": "repeat", "times": 3, "steps": [
                {"durationS": 480, "kind": "work", "target": {"lowPct": 88, "highPct": 94}},
                {"durationS": 120, "kind": "recovery", "target": null},
            ]},
            {"type": "step", "durationS": 300, "kind": "coolDown", "target": null},
        ],
    });

    assert_eq!(serde_json::to_value(sweet_spot()).unwrap(), json);
    assert_eq!(
        serde_json::from_value::<Workout>(json).unwrap(),
        sweet_spot()
    );
}

#[test]
fn the_duration_unrolls_repeats() {
    assert_eq!(sweet_spot().duration_s(), 600 + 3 * (480 + 120) + 300);
}

#[test]
fn a_workout_without_items_lasts_nothing() {
    let workout = Workout {
        items: vec![],
        ..sweet_spot()
    };

    assert_eq!(workout.duration_s(), 0);
}

#[test]
fn the_duration_does_not_overflow_on_long_repeats() {
    let workout = Workout {
        items: vec![Item::Repeat(Repeat {
            times: u32::MAX,
            steps: vec![step(u32::MAX, StepKind::Work, None)],
        })],
        ..sweet_spot()
    };

    assert_eq!(
        workout.duration_s(),
        u64::from(u32::MAX) * u64::from(u32::MAX)
    );
}

#[test]
fn a_well_formed_workout_has_no_problem() {
    assert_eq!(validate(&sweet_spot()), []);
}

#[test]
fn a_problem_travels_to_the_window_with_its_place() {
    let problem = Problem {
        item: Some(1),
        step: None,
        rule: Rule::TooFewTimes,
    };

    assert_eq!(
        serde_json::to_value(problem).unwrap(),
        json!({"item": 1, "step": null, "rule": "tooFewTimes"})
    );
}

#[test]
fn a_blank_name_is_missing() {
    for name in ["", "  \t "] {
        let workout = Workout {
            name: name.to_owned(),
            ..sweet_spot()
        };

        assert_eq!(
            validate(&workout),
            [Problem {
                item: None,
                step: None,
                rule: Rule::NameMissing,
            }]
        );
    }
}

#[test]
fn a_workout_needs_at_least_one_step() {
    let empty = Workout {
        items: vec![],
        ..sweet_spot()
    };

    assert_eq!(
        validate(&empty),
        [Problem {
            item: None,
            step: None,
            rule: Rule::NoStep,
        }]
    );
}

#[test]
fn an_empty_repeat_brings_no_step() {
    let workout = Workout {
        items: vec![Item::Repeat(Repeat {
            times: 2,
            steps: vec![],
        })],
        ..sweet_spot()
    };

    assert_eq!(rules(&workout), [Rule::NoStep, Rule::EmptyRepeat]);
}

#[test]
fn a_single_step_in_a_repeat_is_enough() {
    let workout = Workout {
        items: vec![Item::Repeat(Repeat {
            times: 2,
            steps: vec![step(60, StepKind::Work, None)],
        })],
        ..sweet_spot()
    };

    assert_eq!(validate(&workout), []);
}

#[test]
fn a_step_lasts_at_least_one_second() {
    let mut workout = sweet_spot();
    workout.items[0] = Item::Step(step(0, StepKind::WarmUp, None));
    assert_eq!(
        validate(&workout),
        [Problem {
            item: Some(0),
            step: None,
            rule: Rule::DurationTooShort,
        }]
    );

    workout.items[0] = Item::Step(step(1, StepKind::WarmUp, None));
    assert_eq!(validate(&workout), []);
}

#[test]
fn a_fault_inside_a_repeat_points_at_its_step() {
    let mut workout = sweet_spot();
    let Item::Repeat(repeat) = &mut workout.items[1] else {
        unreachable!()
    };
    repeat.steps[1].duration_s = 0;

    assert_eq!(
        validate(&workout),
        [Problem {
            item: Some(1),
            step: Some(1),
            rule: Rule::DurationTooShort,
        }]
    );
}

#[test]
fn power_targets_range_from_1_to_1000_percent_ftp() {
    assert_eq!(rules(&with_target(1, 1000)), []);
    assert_eq!(rules(&with_target(0, 50)), [Rule::TargetOutOfRange]);
    assert_eq!(rules(&with_target(50, 1001)), [Rule::TargetOutOfRange]);
}

#[test]
fn a_power_target_low_bound_is_strictly_below_its_high_bound() {
    assert_eq!(rules(&with_target(90, 91)), []);
    assert_eq!(rules(&with_target(90, 90)), [Rule::TargetNotRising]);
    assert_eq!(rules(&with_target(95, 90)), [Rule::TargetNotRising]);
}

#[test]
fn a_target_can_break_both_of_its_rules() {
    assert_eq!(
        rules(&with_target(1001, 0)),
        [Rule::TargetOutOfRange, Rule::TargetNotRising]
    );
}

#[test]
fn a_repeat_is_ridden_at_least_twice() {
    let mut workout = sweet_spot();
    let Item::Repeat(repeat) = &mut workout.items[1] else {
        unreachable!()
    };
    repeat.times = 2;
    assert_eq!(validate(&workout), []);

    let Item::Repeat(repeat) = &mut workout.items[1] else {
        unreachable!()
    };
    repeat.times = 1;
    assert_eq!(
        validate(&workout),
        [Problem {
            item: Some(1),
            step: None,
            rule: Rule::TooFewTimes,
        }]
    );
}

#[test]
fn every_problem_is_reported_in_order() {
    let workout = Workout {
        name: String::new(),
        items: vec![
            Item::Step(step(0, StepKind::WarmUp, Some((80, 70)))),
            Item::Repeat(Repeat {
                times: 0,
                steps: vec![
                    step(60, StepKind::Work, None),
                    step(0, StepKind::Recovery, Some((0, 50))),
                ],
            }),
        ],
    };

    assert_eq!(
        validate(&workout),
        [
            Problem {
                item: None,
                step: None,
                rule: Rule::NameMissing,
            },
            Problem {
                item: Some(0),
                step: None,
                rule: Rule::DurationTooShort,
            },
            Problem {
                item: Some(0),
                step: None,
                rule: Rule::TargetNotRising,
            },
            Problem {
                item: Some(1),
                step: None,
                rule: Rule::TooFewTimes,
            },
            Problem {
                item: Some(1),
                step: Some(1),
                rule: Rule::DurationTooShort,
            },
            Problem {
                item: Some(1),
                step: Some(1),
                rule: Rule::TargetOutOfRange,
            },
        ]
    );
}
