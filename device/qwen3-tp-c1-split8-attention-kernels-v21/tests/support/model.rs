//! CPU arithmetic model over precomputed scores, not gfx950 math emulation.

use std::ops::Range;

#[derive(Clone, Debug)]
pub struct State {
    pub maximum: f32,
    pub denominator: f32,
    pub numerators: [f32; 128],
}

pub fn ranges(active: usize) -> Result<[Range<usize>; 8], &'static str> {
    if !(128..=256).contains(&active) {
        return Err("active context");
    }
    let span = active.div_ceil(8);
    Ok(std::array::from_fn(|partition| {
        let begin = partition * span;
        begin..(begin + span).min(active)
    }))
}

fn valid(state: &State) -> bool {
    state.maximum.is_finite()
        && state.denominator.is_finite()
        && state.denominator > 0.0
        && state.numerators.iter().all(|value| value.is_finite())
}

pub fn partition(
    scores: &[f32],
    values: &[[f32; 128]],
    range: Range<usize>,
) -> Result<State, &'static str> {
    if scores.len() != values.len() || range.is_empty() || range.end > scores.len() {
        return Err("state input shape");
    }
    let mut state = State {
        maximum: scores[range.start],
        denominator: 1.0,
        numerators: values[range.start],
    };
    if !valid(&state) {
        return Err("first state is nonfinite");
    }
    for token in range.start + 1..range.end {
        let score = scores[token];
        if !score.is_finite() || !values[token].iter().all(|value| value.is_finite()) {
            return Err("nonfinite token");
        }
        let maximum = if score > state.maximum {
            score
        } else {
            state.maximum
        };
        let previous = (state.maximum - maximum).exp();
        let current = (score - maximum).exp();
        state.denominator = state.denominator * previous + current;
        for (sum, value) in state.numerators.iter_mut().zip(values[token]) {
            *sum = *sum * previous + value * current;
        }
        state.maximum = maximum;
        if !previous.is_finite()
            || previous < 0.0
            || !current.is_finite()
            || current < 0.0
            || !valid(&state)
        {
            return Err("nonfinite token accumulation");
        }
    }
    Ok(state)
}

pub fn merge(states: &[State]) -> Result<State, &'static str> {
    if states.len() != 8 || !states.iter().all(valid) {
        return Err("eight valid partition states required");
    }
    let mut merged = states[0].clone();
    for state in &states[1..] {
        let maximum = if state.maximum > merged.maximum {
            state.maximum
        } else {
            merged.maximum
        };
        let previous = (merged.maximum - maximum).exp();
        let current = (state.maximum - maximum).exp();
        merged.denominator = merged.denominator * previous + state.denominator * current;
        for (sum, value) in merged.numerators.iter_mut().zip(state.numerators) {
            *sum = *sum * previous + value * current;
        }
        merged.maximum = maximum;
        if !previous.is_finite()
            || previous < 0.0
            || !current.is_finite()
            || current < 0.0
            || !valid(&merged)
        {
            return Err("nonfinite merge");
        }
    }
    Ok(merged)
}

pub fn normalized(state: &State) -> Result<[f32; 128], &'static str> {
    if !valid(state) {
        return Err("invalid normalized state");
    }
    let values = state.numerators.map(|value| value / state.denominator);
    if values.iter().all(|value| value.is_finite()) {
        Ok(values)
    } else {
        Err("nonfinite normalized value")
    }
}

pub fn split(scores: &[f32], values: &[[f32; 128]]) -> Result<[f32; 128], &'static str> {
    let states: Result<Vec<_>, _> = ranges(scores.len())?
        .into_iter()
        .map(|range| partition(scores, values, range))
        .collect();
    normalized(&merge(&states?)?)
}

pub fn serial(scores: &[f32], values: &[[f32; 128]]) -> Result<[f32; 128], &'static str> {
    normalized(&partition(scores, values, 0..scores.len())?)
}

pub fn ideal(scores: &[f32], values: &[[f32; 128]]) -> [f64; 128] {
    let maximum = scores
        .iter()
        .map(|value| f64::from(*value))
        .fold(f64::NEG_INFINITY, f64::max);
    let weights: Vec<_> = scores
        .iter()
        .map(|value| (f64::from(*value) - maximum).exp())
        .collect();
    let denominator: f64 = weights.iter().sum();
    std::array::from_fn(|column| {
        weights
            .iter()
            .zip(values)
            .map(|(weight, row)| weight * f64::from(row[column]))
            .sum::<f64>()
            / denominator
    })
}
