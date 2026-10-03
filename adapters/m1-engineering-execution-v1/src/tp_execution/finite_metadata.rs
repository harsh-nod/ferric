//! Prepare-only metadata for the fixed Qwen3-8B TP2 finite-worker geometry.
//!
//! This conversion does not register a profile, authenticate sequence history,
//! own device memory, or authorize execution. Legacy graph validation is unchanged.

use super::{EngineeringTp2GraphGeometryV1, EngineeringTp2GraphInputV1, TpResult};

const PAGE_COUNT: usize = 144;
const CONTEXT_TOKENS: u32 = 2304;
const TOKENS_PER_PAGE: u32 = 16;
const ROTARY_ELEMENTS: usize = 128;

/// Owned scalar metadata matching the existing Output V5 storage roots.
///
/// The completed inactive page-table tail is only a deterministic permutation
/// completion. It does not commit those logical pages or admit their KV contents.
#[derive(Clone, Debug, PartialEq)]
pub struct EngineeringTp2Finite2304MetadataV1 {
    cache_metadata: [u32; PAGE_COUNT + 1],
    rotary: [f32; ROTARY_ELEMENTS],
}

impl EngineeringTp2Finite2304MetadataV1 {
    /// Convert the exact long-geometry legacy scalar representation.
    ///
    /// Every active mapping is retained. Missing physical page IDs fill the
    /// inactive tail in ascending order. Rotary values are decoded, not computed:
    /// 64 little-endian F32 cosine words followed by 64 sine words.
    ///
    /// The caller still owns plan/model/rank identity, generation and cross-token
    /// committed-history validation; none is established by this scalar helper.
    ///
    /// # Errors
    /// Rejects any other geometry, out-of-range position, malformed MAX-tail page
    /// table, incorrect byte extent, or nonfinite rotary value.
    pub fn prepare(input: &EngineeringTp2GraphInputV1) -> TpResult<Self> {
        if input.geometry != EngineeringTp2GraphGeometryV1::Long2304
            || input.position >= CONTEXT_TOKENS
        {
            return Err("finite2304 metadata geometry or position".into());
        }
        let pages: &[u32; PAGE_COUNT] = input
            .page_table
            .as_slice()
            .try_into()
            .map_err(|_| "finite2304 page table extent")?;
        if input.cos_sin.len() != ROTARY_ELEMENTS * 4 {
            return Err("finite2304 rotary byte extent".into());
        }
        let active = usize::try_from(input.position / TOKENS_PER_PAGE + 1)
            .map_err(|_| "finite2304 active page count")?;
        let mut cache_metadata = [0_u32; PAGE_COUNT + 1];
        cache_metadata[0] = input.position;
        let mut seen = [false; PAGE_COUNT];
        for (logical, &physical) in pages[..active].iter().enumerate() {
            let physical_index = usize::try_from(physical)
                .ok()
                .filter(|index| *index < PAGE_COUNT)
                .ok_or("finite2304 active physical page bounds")?;
            if seen[physical_index] {
                return Err("finite2304 active physical page alias".into());
            }
            seen[physical_index] = true;
            cache_metadata[logical + 1] = physical;
        }
        if pages[active..].iter().any(|page| *page != u32::MAX) {
            return Err("finite2304 legacy inactive tail must be MAX".into());
        }

        // Complete only uncommitted entries; supplied active mappings never move.
        let mut logical = active + 1;
        for (physical, occupied) in seen.iter().enumerate() {
            if !occupied {
                cache_metadata[logical] = u32::try_from(physical)
                    .map_err(|_| "finite2304 physical page representation")?;
                logical += 1;
            }
        }
        let mut rotary = [0.0_f32; ROTARY_ELEMENTS];
        for (value, bytes) in rotary.iter_mut().zip(input.cos_sin.chunks_exact(4)) {
            *value = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            if !value.is_finite() {
                return Err("finite2304 nonfinite rotary value".into());
            }
        }
        Ok(Self {
            cache_metadata,
            rotary,
        })
    }

    /// Exact ordinary U32 root: position followed by 144 physical page IDs.
    #[must_use]
    pub const fn cache_metadata(&self) -> &[u32; 145] {
        &self.cache_metadata
    }

    /// Exact F32 root: cosine lanes 0..64 followed by sine lanes 0..64.
    #[must_use]
    pub const fn rotary(&self) -> &[f32; 128] {
        &self.rotary
    }
}

#[cfg(test)]
#[path = "finite_metadata_tests.rs"]
mod tests;
