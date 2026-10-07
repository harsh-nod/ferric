//! Arithmetic-order and storage models only; these do not execute GPU kernels.

use alloc::vec::Vec;
use fe2o3_device::Bf16;

pub const K: usize = 4096;
pub const N: usize = 12288;
pub const LANES: usize = 64;
pub const GROUPS: usize = 32;
pub const WORDS: usize = K / 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Length,
    Coordinate,
    Allocation,
    NonFinite,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Geometry {
    pub activation_words: usize,
    pub gate_weight_words: usize,
    pub up_weight_words: usize,
    pub gate_output_elements: usize,
    pub up_output_elements: usize,
    pub launch_threads: usize,
}

impl Geometry {
    pub fn validate(self) -> Result<(), Error> {
        if self.activation_words != WORDS
            || self.gate_weight_words != N * WORDS
            || self.up_weight_words != N * WORDS
            || self.gate_output_elements != N
            || self.up_output_elements != N
            || self.launch_threads != N * LANES
        {
            return Err(Error::Length);
        }
        Ok(())
    }
}

pub fn source_pair(group: usize, lane: usize) -> Result<[usize; 2], Error> {
    if group >= GROUPS || lane >= LANES {
        return Err(Error::Coordinate);
    }
    let low = group * 2 * LANES + lane;
    Ok([low, low + LANES])
}

pub fn packed_weight_index(column: usize, group: usize, lane: usize) -> Result<usize, Error> {
    source_pair(group, lane)?;
    if column >= N {
        return Err(Error::Coordinate);
    }
    Ok(column * WORDS + group * LANES + lane)
}

pub fn pack_row(source: &[u16]) -> Result<Vec<u32>, Error> {
    if source.len() != K {
        return Err(Error::Length);
    }
    let mut output = Vec::new();
    output
        .try_reserve_exact(WORDS)
        .map_err(|_| Error::Allocation)?;
    for group in 0..GROUPS {
        for lane in 0..LANES {
            let [low, high] = source_pair(group, lane)?;
            output.push(u32::from(source[low]) | (u32::from(source[high]) << 16));
        }
    }
    Ok(output)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Checkpoint {
    pub inner: usize,
    pub activation: u16,
    pub weight: u16,
    pub product: u32,
    pub partial: u32,
    pub finite: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LaneTrace {
    pub steps: Vec<Checkpoint>,
    pub partial: u32,
    pub finite: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PairTrace {
    pub gate: LaneTrace,
    pub up: LaneTrace,
}

// Nonfinite inputs are rejected. NaN payloads are normalized only for comparison.
fn bits(value: f32) -> u32 {
    if value.is_nan() {
        f32::NAN.to_bits()
    } else {
        value.to_bits()
    }
}

fn checkpoints() -> Result<Vec<Checkpoint>, Error> {
    let mut output = Vec::new();
    output
        .try_reserve_exact(K / LANES)
        .map_err(|_| Error::Allocation)?;
    Ok(output)
}

pub fn baseline_lane(a: &[u16], weights: &[u16], lane: usize) -> Result<LaneTrace, Error> {
    if lane >= LANES {
        return Err(Error::Coordinate);
    }
    if a.len() != K || weights.len() != K {
        return Err(Error::Length);
    }
    let mut steps = checkpoints()?;
    let mut partial = 0.0_f32;
    let mut finite = true;
    for step in 0..K / LANES {
        let inner = step * LANES + lane;
        let left = f32::from_bits(u32::from(a[inner]) << 16);
        let right = f32::from_bits(u32::from(weights[inner]) << 16);
        let product = left * right;
        partial += product;
        finite &= product.is_finite() & partial.is_finite();
        steps.push(Checkpoint {
            inner,
            activation: a[inner],
            weight: weights[inner],
            product: bits(product),
            partial: bits(partial),
            finite,
        });
    }
    Ok(LaneTrace {
        steps,
        partial: bits(partial),
        finite,
    })
}

pub fn fused_lane(a: &[u32], gate: &[u32], up: &[u32], lane: usize) -> Result<PairTrace, Error> {
    if lane >= LANES {
        return Err(Error::Coordinate);
    }
    if a.len() != WORDS || gate.len() != WORDS || up.len() != WORDS {
        return Err(Error::Length);
    }
    let mut gate_steps = checkpoints()?;
    let mut up_steps = checkpoints()?;
    let mut gate_partial = 0.0_f32;
    let mut up_partial = 0.0_f32;
    let mut gate_finite = true;
    let mut up_finite = true;
    for group in 0..GROUPS {
        let word = group * LANES + lane;
        let left_bits = a[word];
        let gate_bits = gate[word];
        let up_bits = up[word];
        for half in 0..2 {
            let inner = (group * 2 + half) * LANES + lane;
            let activation = (left_bits >> (half * 16)) as u16;
            let gate_weight = (gate_bits >> (half * 16)) as u16;
            let up_weight = (up_bits >> (half * 16)) as u16;
            let left = Bf16::from_bits(activation).to_f32();
            let gate_product = left * Bf16::from_bits(gate_weight).to_f32();
            gate_partial += gate_product;
            gate_finite &= gate_product.is_finite() & gate_partial.is_finite();
            let up_product = left * Bf16::from_bits(up_weight).to_f32();
            up_partial += up_product;
            up_finite &= up_product.is_finite() & up_partial.is_finite();
            gate_steps.push(Checkpoint {
                inner,
                activation,
                weight: gate_weight,
                product: bits(gate_product),
                partial: bits(gate_partial),
                finite: gate_finite,
            });
            up_steps.push(Checkpoint {
                inner,
                activation,
                weight: up_weight,
                product: bits(up_product),
                partial: bits(up_partial),
                finite: up_finite,
            });
        }
    }
    Ok(PairTrace {
        gate: LaneTrace {
            steps: gate_steps,
            partial: bits(gate_partial),
            finite: gate_finite,
        },
        up: LaneTrace {
            steps: up_steps,
            partial: bits(up_partial),
            finite: up_finite,
        },
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RowResult {
    pub sums: [u32; LANES],
    pub narrowed: [u16; LANES],
    pub accepted: bool,
}

pub fn finish_row(lanes: &[LaneTrace]) -> Result<RowResult, Error> {
    if lanes.len() != LANES {
        return Err(Error::Length);
    }
    let mut values =
        core::array::from_fn::<_, LANES, _>(|lane| f32::from_bits(lanes[lane].partial));
    for offset in [1, 2, 4, 8, 16, 32] {
        let before = values;
        for (lane, value) in values.iter_mut().enumerate() {
            *value = before[lane] + before[lane ^ offset];
        }
    }
    let narrowed = values.map(|value| Bf16::from_f32(value).to_bits());
    Ok(RowResult {
        sums: values.map(bits),
        narrowed,
        accepted: lanes.iter().enumerate().all(|(lane, state)| {
            state.finite && values[lane].is_finite() && Bf16::from_bits(narrowed[lane]).is_finite()
        }),
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PairResult {
    pub gate: RowResult,
    pub up: RowResult,
}

impl PairResult {
    pub fn accepted(&self) -> bool {
        self.gate.accepted && self.up.accepted
    }

    pub fn store(&self, gate: &mut [u16], up: &mut [u16], column: usize) -> Result<(), Error> {
        if gate.len() != N || up.len() != N {
            return Err(Error::Length);
        }
        if column >= N {
            return Err(Error::Coordinate);
        }
        if !self.accepted() {
            return Err(Error::NonFinite);
        }
        gate[column] = self.gate.narrowed[0];
        up[column] = self.up.narrowed[0];
        Ok(())
    }
}

pub fn finish_pair(lanes: &[PairTrace]) -> Result<PairResult, Error> {
    if lanes.len() != LANES {
        return Err(Error::Length);
    }
    let gate = lanes
        .iter()
        .map(|lane| lane.gate.clone())
        .collect::<Vec<_>>();
    let up = lanes.iter().map(|lane| lane.up.clone()).collect::<Vec<_>>();
    Ok(PairResult {
        gate: finish_row(&gate)?,
        up: finish_row(&up)?,
    })
}
