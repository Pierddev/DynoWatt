//! Workouts: the sessions the Athlete plans in DynoWatt and rides later from
//! their Bike Computer. The database holds their only copy (ADR 0005).

#[cfg(test)]
mod tests;

use std::ops::RangeInclusive;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::db::{self, AthleteId, WorkoutId};

/// A named, planned session: an ordered sequence of Steps and Repeats.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workout {
    pub name: String,
    pub items: Vec<Item>,
}

/// One entry of a Workout's sequence: a Step on its own, or a Repeat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Item {
    Step(Step),
    Repeat(Repeat),
}

/// One part of a Workout, ridden for a fixed time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    pub duration_s: u32,
    pub kind: StepKind,
    /// `None` when the Step is ridden freely.
    pub target: Option<PowerTarget>,
}

/// The role a Step plays in a Workout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StepKind {
    WarmUp,
    Work,
    Recovery,
    CoolDown,
}

/// The power range to hold during a Step, in % of the FTP set on the Bike
/// Computer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PowerTarget {
    pub low_pct: u16,
    pub high_pct: u16,
}

/// Steps ridden `times` times in a row. Holds Steps only, never a Repeat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Repeat {
    pub times: u32,
    pub steps: Vec<Step>,
}

impl Item {
    /// The Steps of this Item, ridden once each per round.
    pub fn steps(&self) -> &[Step] {
        match self {
            Item::Step(step) => std::slice::from_ref(step),
            Item::Repeat(repeat) => &repeat.steps,
        }
    }

    /// How many rounds of its Steps this Item rides.
    pub fn times(&self) -> u32 {
        match self {
            Item::Step(_) => 1,
            Item::Repeat(repeat) => repeat.times,
        }
    }
}

impl Workout {
    /// How long the whole Workout lasts, Repeats unrolled.
    pub fn duration_s(&self) -> u64 {
        self.items
            .iter()
            .map(|item| {
                let round: u64 = item
                    .steps()
                    .iter()
                    .map(|step| u64::from(step.duration_s))
                    .sum();
                u64::from(item.times()) * round
            })
            .sum()
    }
}

/// A rule of a valid Workout, and where the Workout breaks it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Problem {
    /// The Item at fault, by its index in the Workout; `None` when the fault is
    /// the Workout's as a whole.
    pub item: Option<usize>,
    /// The Step at fault, by its index inside the Repeat `item`; `None` when
    /// the Item itself is at fault.
    pub step: Option<usize>,
    pub rule: Rule,
}

/// The rules a Workout must follow to be saved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Rule {
    /// The name is empty or only blanks.
    NameMissing,
    /// The Workout holds no Step at all.
    NoStep,
    /// A Step lasts less than one second.
    DurationTooShort,
    /// A Power Target bound lies outside 1–1000 % FTP.
    TargetOutOfRange,
    /// A Power Target's low bound is not strictly below its high bound.
    TargetNotRising,
    /// A Repeat is ridden fewer than two times.
    TooFewTimes,
    /// A Repeat holds no Step.
    EmptyRepeat,
}

/// The shortest a Step may last.
pub const MIN_DURATION_S: u32 = 1;

/// The bounds a Power Target may take, in % FTP.
pub const TARGET_PCT: RangeInclusive<u16> = 1..=1000;

/// The fewest times a Repeat may be ridden: once would be plain Steps.
pub const MIN_TIMES: u32 = 2;

/// Every rule `workout` breaks, Workout-wide ones first, then Item by Item.
/// Empty when the Workout can be saved.
pub fn validate(workout: &Workout) -> Vec<Problem> {
    let mut problems = Vec::new();
    let mut fault = |item, step, rule| problems.push(Problem { item, step, rule });
    if workout.name.trim().is_empty() {
        fault(None, None, Rule::NameMissing);
    }
    if workout.items.iter().all(|item| item.steps().is_empty()) {
        fault(None, None, Rule::NoStep);
    }
    for (i, item) in workout.items.iter().enumerate() {
        match item {
            Item::Step(step) => {
                for rule in step_faults(step) {
                    fault(Some(i), None, rule);
                }
            }
            Item::Repeat(repeat) => {
                if repeat.times < MIN_TIMES {
                    fault(Some(i), None, Rule::TooFewTimes);
                }
                if repeat.steps.is_empty() {
                    fault(Some(i), None, Rule::EmptyRepeat);
                }
                for (j, step) in repeat.steps.iter().enumerate() {
                    for rule in step_faults(step) {
                        fault(Some(i), Some(j), rule);
                    }
                }
            }
        }
    }
    problems
}

fn step_faults(step: &Step) -> Vec<Rule> {
    let mut rules = Vec::new();
    if step.duration_s < MIN_DURATION_S {
        rules.push(Rule::DurationTooShort);
    }
    if let Some(target) = step.target {
        if !TARGET_PCT.contains(&target.low_pct) || !TARGET_PCT.contains(&target.high_pct) {
            rules.push(Rule::TargetOutOfRange);
        }
        if target.low_pct >= target.high_pct {
            rules.push(Rule::TargetNotRising);
        }
    }
    rules
}

/// One Workout of the list, with its total duration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListedWorkout {
    pub id: i64,
    pub name: String,
    /// Repeats unrolled.
    pub duration_s: u64,
}

/// The Athlete's Workouts, in the order they were created: the window sorts
/// them by name, as the Athlete's language does.
pub fn list(conn: &Connection, athlete_id: AthleteId) -> rusqlite::Result<Vec<ListedWorkout>> {
    Ok(db::list_workouts(conn, athlete_id)?
        .into_iter()
        .map(|(id, workout)| ListedWorkout {
            id: id.0,
            duration_s: workout.duration_s(),
            name: workout.name,
        })
        .collect())
}

/// The Workout `id`.
pub fn get(conn: &Connection, id: WorkoutId) -> Result<Workout, String> {
    db::find_workout(conn, id)
        .map_err(|err| err.to_string())?
        .ok_or_else(|| unknown(id))
}

/// Saves `workout`, its name trimmed, as a new Workout of the Athlete, unless
/// it breaks a rule.
pub fn create(
    conn: &mut Connection,
    athlete_id: AthleteId,
    workout: Workout,
) -> Result<WorkoutId, String> {
    let workout = checked(workout)?;
    db::insert_workout(conn, athlete_id, &workout).map_err(|err| err.to_string())
}

/// Replaces the Workout `id` with `workout`, its name trimmed, unless it
/// breaks a rule.
pub fn update(conn: &mut Connection, id: WorkoutId, workout: Workout) -> Result<(), String> {
    let workout = checked(workout)?;
    if db::update_workout(conn, id, &workout).map_err(|err| err.to_string())? {
        Ok(())
    } else {
        Err(unknown(id))
    }
}

/// Saves a copy of the Workout `id` named `name` for the Athlete. The window
/// picks the name, so it can follow the Athlete's language.
pub fn duplicate(
    conn: &mut Connection,
    athlete_id: AthleteId,
    id: WorkoutId,
    name: String,
) -> Result<WorkoutId, String> {
    let original = get(conn, id)?;
    create(
        conn,
        athlete_id,
        Workout {
            name,
            items: original.items,
        },
    )
}

pub fn delete(conn: &Connection, id: WorkoutId) -> Result<(), String> {
    if db::delete_workout(conn, id).map_err(|err| err.to_string())? {
        Ok(())
    } else {
        Err(unknown(id))
    }
}

/// `workout` with its name trimmed, when it follows every rule.
fn checked(mut workout: Workout) -> Result<Workout, String> {
    workout.name = workout.name.trim().to_owned();
    if validate(&workout).is_empty() {
        Ok(workout)
    } else {
        Err("the Workout is not valid".to_owned())
    }
}

fn unknown(id: WorkoutId) -> String {
    format!("no Workout has the id {}", id.0)
}
