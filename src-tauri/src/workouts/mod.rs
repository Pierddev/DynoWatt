//! Workouts: the sessions the Athlete plans in DynoWatt and rides later from
//! their Bike Computer. The database holds their only copy (ADR 0005).

#[cfg(test)]
mod tests;

use std::ops::RangeInclusive;

use serde::{Deserialize, Serialize};

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
