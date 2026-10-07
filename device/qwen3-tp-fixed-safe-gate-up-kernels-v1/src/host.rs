//! Checked packing and a host arithmetic-order model, not a GPU validation.

use alloc::vec::Vec;
use crate::reference_bf16::Bf16;

pub const K: usize = 4096;
pub const LANES: usize = 64;
pub const GROUPS: usize = K / (2 * LANES);
pub const WORDS: usize = K / 2;
pub const MAX_ROWS: usize = 12288;
pub const MAX_ACTIVATION_ROWS: usize = 32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackingError {
    Shape,
    Length,
    Coordinate,
    Overflow,
    Allocation,
}

fn shape(rows: usize, k: usize) -> Result<(usize, usize), PackingError> {
    if k != K || rows == 0 || rows > MAX_ROWS {
        return Err(PackingError::Shape);
    }
    Ok((
        rows.checked_mul(K).ok_or(PackingError::Overflow)?,
        rows.checked_mul(WORDS).ok_or(PackingError::Overflow)?,
    ))
}

pub fn source_pair_indices(
    rows: usize,
    k: usize,
    row: usize,
    group: usize,
    lane: usize,
) -> Result<[usize; 2], PackingError> {
    shape(rows, k)?;
    if row >= rows || group >= GROUPS || lane >= LANES {
        return Err(PackingError::Coordinate);
    }
    let low = row
        .checked_mul(K)
        .and_then(|base| base.checked_add(group * 2 * LANES))
        .and_then(|base| base.checked_add(lane))
        .ok_or(PackingError::Overflow)?;
    Ok([low, low.checked_add(LANES).ok_or(PackingError::Overflow)?])
}

pub fn word_index(
    rows: usize,
    k: usize,
    row: usize,
    group: usize,
    lane: usize,
) -> Result<usize, PackingError> {
    source_pair_indices(rows, k, row, group, lane)?;
    row.checked_mul(WORDS)
        .and_then(|base| base.checked_add(group * LANES))
        .and_then(|base| base.checked_add(lane))
        .ok_or(PackingError::Overflow)
}

pub const fn unpack_word(word: u32) -> [u16; 2] {
    [word as u16, (word >> 16) as u16]
}

pub fn pack_rows(source: &[u16], rows: usize, k: usize) -> Result<Vec<u32>, PackingError> {
    let (elements, words) = shape(rows, k)?;
    if source.len() != elements {
        return Err(PackingError::Length);
    }
    let mut packed = Vec::new();
    packed.try_reserve_exact(words).map_err(|_| PackingError::Allocation)?;
    for row in 0..rows {
        for group in 0..GROUPS {
            for lane in 0..LANES {
                let [low, high] = source_pair_indices(rows, k, row, group, lane)?;
                packed.push(u32::from(source[low]) | (u32::from(source[high]) << 16));
            }
        }
    }
    Ok(packed)
}

pub fn unpack_rows(packed: &[u32], rows: usize, k: usize) -> Result<Vec<u16>, PackingError> {
    let (elements, words) = shape(rows, k)?;
    if packed.len() != words {
        return Err(PackingError::Length);
    }
    let mut source = Vec::new();
    source.try_reserve_exact(elements).map_err(|_| PackingError::Allocation)?;
    source.resize(elements, 0);
    for row in 0..rows {
        for group in 0..GROUPS {
            for lane in 0..LANES {
                let [low, high] = source_pair_indices(rows, k, row, group, lane)?;
                let [left, right] = unpack_word(packed[word_index(rows, k, row, group, lane)?]);
                source[low] = left;
                source[high] = right;
            }
        }
    }
    Ok(source)
}

pub fn packed_le_bytes(packed: &[u32], rows: usize, k: usize) -> Result<Vec<u8>, PackingError> {
    let (_, words) = shape(rows, k)?;
    if packed.len() != words {
        return Err(PackingError::Length);
    }
    let bytes = words.checked_mul(4).ok_or(PackingError::Overflow)?;
    let mut output = Vec::new();
    output.try_reserve_exact(bytes).map_err(|_| PackingError::Allocation)?;
    for word in packed {
        output.extend_from_slice(&word.to_le_bytes());
    }
    Ok(output)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KernelGeometry {
    pub rows: usize,
    pub n: usize,
    pub k: usize,
    pub world_size: usize,
    pub projection: usize,
    pub activation_words: usize,
    pub weight_words: usize,
    pub output_elements: usize,
    pub launch_threads: usize,
}

impl KernelGeometry {
    pub fn validate(self) -> Result<(), PackingError> {
        if self.rows != 1 || self.k != K || self.world_size != 1
            || !matches!((self.projection, self.n),
                (1, 4096) | (2 | 3, 1024) | (4 | 5, 12288))
        {
            return Err(PackingError::Shape);
        }
        if !(WORDS..=MAX_ACTIVATION_ROWS * WORDS).contains(&self.activation_words)
            || self.weight_words != self.n * WORDS
            || !(self.n..=MAX_ACTIVATION_ROWS * self.n).contains(&self.output_elements)
            || self.launch_threads != self.n * LANES
        {
            return Err(PackingError::Length);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Checkpoint {
    pub inner: usize,
    pub left_bits: u16,
    pub right_bits: u16,
    pub product_bits: u32,
    pub partial_bits: u32,
    pub finite: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LaneTrace {
    pub steps: Vec<Checkpoint>,
    pub partial_bits: u32,
    pub finite: bool,
}

// NaN payloads are not an acceptance contract. Finite values and signed zeros
// retain every bit; rejected NaNs are normalized only in this host model.
fn recorded_bits(value: f32) -> u32 {
    if value.is_nan() { f32::NAN.to_bits() } else { value.to_bits() }
}

fn trace_storage() -> Result<Vec<Checkpoint>, PackingError> {
    let mut result = Vec::new();
    result.try_reserve_exact(K / LANES).map_err(|_| PackingError::Allocation)?;
    Ok(result)
}

pub fn baseline_lane(a: &[u16], weights: &[u16], lane: usize) -> Result<LaneTrace, PackingError> {
    if lane >= LANES {
        return Err(PackingError::Coordinate);
    }
    if !(K..=MAX_ACTIVATION_ROWS * K).contains(&a.len()) || weights.len() != K {
        return Err(PackingError::Length);
    }
    let mut steps = trace_storage()?;
    let mut partial = 0.0_f32;
    let mut finite = true;
    for step in 0..K / LANES {
        let inner = step * LANES + lane;
        let left = Bf16::from_bits(a[inner]).to_f32();
        let right = Bf16::from_bits(weights[inner]).to_f32();
        let product = left * right;
        partial += product;
        finite &= product.is_finite() & partial.is_finite();
        steps.push(Checkpoint {
            inner, left_bits: a[inner], right_bits: weights[inner],
            product_bits: recorded_bits(product), partial_bits: recorded_bits(partial), finite,
        });
    }
    Ok(LaneTrace { steps, partial_bits: recorded_bits(partial), finite })
}

pub fn packed_lane(a: &[u32], weights: &[u32], lane: usize) -> Result<LaneTrace, PackingError> {
    if lane >= LANES {
        return Err(PackingError::Coordinate);
    }
    if !(WORDS..=MAX_ACTIVATION_ROWS * WORDS).contains(&a.len()) || weights.len() != WORDS {
        return Err(PackingError::Length);
    }
    let mut steps = trace_storage()?;
    let mut partial = 0.0_f32;
    let mut finite = true;
    for group in 0..GROUPS {
        let index = group * LANES + lane;
        let left_pair = unpack_word(a[index]);
        let right_pair = unpack_word(weights[index]);
        for half in 0..2 {
            let inner = (group * 2 + half) * LANES + lane;
            let left = Bf16::from_bits(left_pair[half]).to_f32();
            let right = Bf16::from_bits(right_pair[half]).to_f32();
            let product = left * right;
            partial += product;
            finite &= product.is_finite() & partial.is_finite();
            steps.push(Checkpoint {
                inner, left_bits: left_pair[half], right_bits: right_pair[half],
                product_bits: recorded_bits(product), partial_bits: recorded_bits(partial), finite,
            });
        }
    }
    Ok(LaneTrace { steps, partial_bits: recorded_bits(partial), finite })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RowResult {
    pub sums: [u32; LANES],
    pub narrowed: [u16; LANES],
    pub accepted: bool,
}

pub fn finish_row(lanes: &[LaneTrace]) -> Result<RowResult, PackingError> {
    if lanes.len() != LANES {
        return Err(PackingError::Length);
    }
    let mut values = core::array::from_fn::<_, LANES, _>(|lane| f32::from_bits(lanes[lane].partial_bits));
    for offset in [1, 2, 4, 8, 16, 32] {
        let before = values;
        for (lane, value) in values.iter_mut().enumerate() {
            *value = before[lane] + before[lane ^ offset];
        }
    }
    let sums = values.map(recorded_bits);
    let narrowed = values.map(|value| Bf16::from_f32(value).to_bits());
    let accepted = lanes.iter().enumerate().all(|(lane, state)| {
        state.finite && values[lane].is_finite() && Bf16::from_bits(narrowed[lane]).is_finite()
    });
    Ok(RowResult { sums, narrowed, accepted })
}
