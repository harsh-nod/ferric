//! Deterministic, host-staged FP32 summation with one residual addition and
//! one BF16 round-to-nearest-even. Floating-point behavior is Contracted,
//! not Verus-proved or a hardware numerical-qualification result.

use super::TpResult;

/// One completed rank's host-visible FP32 partial vector.
pub struct HostStagedPartialV1<'a> {
    /// Logical rank assigned by the caller's retained child transport.
    pub rank: u32,
    /// Completed row-parallel projection output, before residual addition.
    pub values: &'a [f32],
}

/// Reduces in ascending rank order, then adds the residual and rounds once.
///
/// Inputs may arrive in any order. A complete unique rank roster, matching
/// lengths, and finite inputs/intermediates/output are required. On error no
/// caller-owned input is changed and no partial result is published.
///
/// # Errors
/// Rejects rank/shape drift or nonfinite inputs, sums, or BF16 output.
pub fn reduce_residual_bf16_v1(
    world_size: u32,
    partials: &[HostStagedPartialV1<'_>],
    residual: &[u16],
) -> TpResult<Vec<u16>> {
    if !matches!(world_size, 1 | 2 | 8)
        || partials.len() != world_size as usize
        || residual.is_empty()
    {
        return Err("invalid host-staged collective geometry".into());
    }
    let mut ranks = [None; 8];
    for partial in partials {
        let index = partial.rank as usize;
        if partial.rank >= world_size
            || ranks[index].is_some()
            || partial.values.len() != residual.len()
        {
            return Err("incomplete, duplicate, or misshaped collective rank".into());
        }
        ranks[index] = Some(partial.values);
    }
    let mut output = Vec::with_capacity(residual.len());
    for (index, bits) in residual.iter().copied().enumerate() {
        let mut sum = 0.0_f32;
        for values in &ranks[..world_size as usize] {
            let values = values.ok_or("missing collective rank")?;
            let value = values[index];
            if !value.is_finite() {
                return Err("nonfinite row-parallel partial".into());
            }
            sum += value;
            if !sum.is_finite() {
                return Err("row-parallel reduction overflow".into());
            }
        }
        let residual = f32::from_bits(u32::from(bits) << 16);
        let value = sum + residual;
        if !residual.is_finite() || !value.is_finite() {
            return Err("nonfinite residual sum".into());
        }
        let bits = value.to_bits();
        let rounding = 0x7fff + ((bits >> 16) & 1);
        let rounded = ((bits.wrapping_add(rounding)) >> 16) as u16;
        if !f32::from_bits(u32::from(rounded) << 16).is_finite() {
            return Err("BF16 residual sum overflow".into());
        }
        output.push(rounded);
    }
    Ok(output)
}

#[cfg(test)]
mod tests;
