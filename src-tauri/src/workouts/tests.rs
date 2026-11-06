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

fn store() -> Connection {
    db::open_in_memory().unwrap()
}

fn named(name: &str) -> Workout {
    Workout {
        name: name.to_owned(),
        ..sweet_spot()
    }
}

fn invalid() -> Workout {
    Workout {
        items: vec![],
        ..sweet_spot()
    }
}

fn listed(conn: &Connection) -> Vec<(i64, String)> {
    list(conn, db::DEFAULT_ATHLETE)
        .unwrap()
        .into_iter()
        .map(|listed| (listed.id, listed.name))
        .collect()
}

#[test]
fn the_list_gives_each_workout_its_name_and_unrolled_duration() {
    let mut conn = store();
    let id = create(&mut conn, db::DEFAULT_ATHLETE, sweet_spot()).unwrap();

    assert_eq!(
        list(&conn, db::DEFAULT_ATHLETE).unwrap(),
        [ListedWorkout {
            id: id.0,
            name: "Sweet spot".to_owned(),
            duration_s: 2700,
        }]
    );
}

#[test]
fn a_listed_workout_travels_to_the_window_in_camel_case() {
    let listed = ListedWorkout {
        id: 4,
        name: "Sweet spot".to_owned(),
        duration_s: 2700,
    };

    assert_eq!(
        serde_json::to_value(listed).unwrap(),
        json!({"id": 4, "name": "Sweet spot", "durationS": 2700})
    );
}

#[test]
fn workouts_are_listed_in_creation_order() {
    let mut conn = store();
    let b = create(&mut conn, db::DEFAULT_ATHLETE, named("B")).unwrap();
    let a = create(&mut conn, db::DEFAULT_ATHLETE, named("A")).unwrap();

    assert_eq!(
        listed(&conn),
        [(b.0, "B".to_owned()), (a.0, "A".to_owned())]
    );
}

#[test]
fn a_created_workout_is_stored_with_its_name_trimmed() {
    let mut conn = store();

    let id = create(&mut conn, db::DEFAULT_ATHLETE, named("  Sweet spot \t")).unwrap();

    assert_eq!(get(&conn, id).unwrap(), sweet_spot());
}

#[test]
fn an_invalid_workout_is_not_created() {
    let mut conn = store();

    assert_eq!(
        create(&mut conn, db::DEFAULT_ATHLETE, invalid()),
        Err("the Workout is not valid".to_owned())
    );
    assert_eq!(
        create(&mut conn, db::DEFAULT_ATHLETE, named(" ")),
        Err("the Workout is not valid".to_owned())
    );
    assert_eq!(listed(&conn), []);
}

#[test]
fn an_unknown_workout_cannot_be_opened() {
    assert_eq!(
        get(&store(), WorkoutId(1)),
        Err("no Workout has the id 1".to_owned())
    );
}

#[test]
fn an_update_replaces_the_workout_with_its_name_trimmed() {
    let mut conn = store();
    let id = create(&mut conn, db::DEFAULT_ATHLETE, named("Old")).unwrap();
    let new = Workout {
        name: " New ".to_owned(),
        items: vec![Item::Step(step(60, StepKind::Work, None))],
    };

    update(&mut conn, id, new).unwrap();

    assert_eq!(
        get(&conn, id).unwrap(),
        Workout {
            name: "New".to_owned(),
            items: vec![Item::Step(step(60, StepKind::Work, None))],
        }
    );
}

#[test]
fn an_invalid_update_leaves_the_workout_as_it_was() {
    let mut conn = store();
    let id = create(&mut conn, db::DEFAULT_ATHLETE, sweet_spot()).unwrap();

    assert_eq!(
        update(&mut conn, id, invalid()),
        Err("the Workout is not valid".to_owned())
    );
    assert_eq!(get(&conn, id).unwrap(), sweet_spot());
}

#[test]
fn an_unknown_workout_cannot_be_updated() {
    assert_eq!(
        update(&mut store(), WorkoutId(1), sweet_spot()),
        Err("no Workout has the id 1".to_owned())
    );
}

#[test]
fn a_duplicate_is_a_new_workout_with_the_same_sequence_and_the_given_name() {
    let mut conn = store();
    let original = create(&mut conn, db::DEFAULT_ATHLETE, sweet_spot()).unwrap();

    let copy = duplicate(
        &mut conn,
        db::DEFAULT_ATHLETE,
        original,
        "Sweet spot (copy)".to_owned(),
    )
    .unwrap();

    assert_ne!(copy, original);
    assert_eq!(get(&conn, copy).unwrap(), named("Sweet spot (copy)"));
    assert_eq!(get(&conn, original).unwrap(), sweet_spot());
}

#[test]
fn a_duplicate_needs_a_name() {
    let mut conn = store();
    let original = create(&mut conn, db::DEFAULT_ATHLETE, sweet_spot()).unwrap();

    assert_eq!(
        duplicate(&mut conn, db::DEFAULT_ATHLETE, original, "  ".to_owned()),
        Err("the Workout is not valid".to_owned())
    );
    assert_eq!(listed(&conn), [(original.0, "Sweet spot".to_owned())]);
}

#[test]
fn an_unknown_workout_cannot_be_duplicated() {
    assert_eq!(
        duplicate(
            &mut store(),
            db::DEFAULT_ATHLETE,
            WorkoutId(1),
            "Copy".to_owned()
        ),
        Err("no Workout has the id 1".to_owned())
    );
}

#[test]
fn a_deleted_workout_leaves_the_list() {
    let mut conn = store();
    let deleted = create(&mut conn, db::DEFAULT_ATHLETE, named("Gone")).unwrap();
    let kept = create(&mut conn, db::DEFAULT_ATHLETE, named("Kept")).unwrap();

    delete(&conn, deleted).unwrap();

    assert_eq!(listed(&conn), [(kept.0, "Kept".to_owned())]);
}

#[test]
fn an_unknown_workout_cannot_be_deleted() {
    assert_eq!(
        delete(&store(), WorkoutId(1)),
        Err("no Workout has the id 1".to_owned())
    );
}

#[test]
fn workouts_and_their_copies_belong_to_the_given_athlete() {
    let mut conn = store();
    conn.execute(
        "INSERT INTO athlete (id, name, created_at) VALUES (2, 'Other', '2026-01-01T00:00:00Z')",
        [],
    )
    .unwrap();
    let other = db::AthleteId(2);
    let original = create(&mut conn, other, named("Theirs")).unwrap();
    let copy = duplicate(&mut conn, other, original, "Copy".to_owned()).unwrap();

    assert_eq!(listed(&conn), []);
    assert_eq!(
        list(&conn, other)
            .unwrap()
            .into_iter()
            .map(|listed| listed.id)
            .collect::<Vec<_>>(),
        [original.0, copy.0]
    );
}
