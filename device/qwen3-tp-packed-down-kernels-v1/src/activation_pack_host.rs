//! Checked CPU schedule model only. Production packing must remain on the GPU.

use crate::host::{K, LANES, MAX_ACTIVATION_ROWS, PackingError, WORDS};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActivationPackGeometry {
    pub rows: usize,
    pub k: usize,
    pub source_elements: usize,
    pub output_words: usize,
    pub launch_threads: usize,
}

impl ActivationPackGeometry {
    pub fn validate(self) -> Result<(), PackingError> {
        if self.rows != 1 || self.k != K {
            return Err(PackingError::Shape);
        }
        if !(K..=MAX_ACTIVATION_ROWS * K).contains(&self.source_elements)
            || !(WORDS..=MAX_ACTIVATION_ROWS * WORDS).contains(&self.output_words)
            || self.launch_threads != WORDS
        {
            return Err(PackingError::Length);
        }
        Ok(())
    }
}

pub fn activation_source_pair(word: usize) -> Result<[usize; 2], PackingError> {
    if word >= WORDS {
        return Err(PackingError::Coordinate);
    }
    let low = (word / LANES)
        .checked_mul(2 * LANES)
        .and_then(|base| base.checked_add(word % LANES))
        .ok_or(PackingError::Overflow)?;
    Ok([low, low.checked_add(LANES).ok_or(PackingError::Overflow)?])
}

pub fn model_activation_pack_into(
    source: &[u16],
    output: &mut [u32],
    rows: usize,
    k: usize,
    launch_threads: usize,
) -> Result<(), PackingError> {
    ActivationPackGeometry {
        rows,
        k,
        source_elements: source.len(),
        output_words: output.len(),
        launch_threads,
    }
    .validate()?;
    for (word, slot) in output.iter_mut().take(WORDS).enumerate() {
        let [low, high] = activation_source_pair(word)?;
        let low_bits = source.get(low).copied().ok_or(PackingError::Length)?;
        let high_bits = source.get(high).copied().ok_or(PackingError::Length)?;
        *slot = u32::from(low_bits) | (u32::from(high_bits) << 16);
    }
    Ok(())
}
