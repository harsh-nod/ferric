//! A distinct, bit-preserving KxN head recipe. Original global uploads stay NxK.

use super::{EngineeringTp2FiniteWeightSourceV1, MAX_CHUNK};
use crate::tp_execution::TpResult;
use ferric_spec::{QWEN3_NO_LAYER, Qwen3TensorKind};
use sha2::{Digest, Sha256};

const N: usize = 151_936;
const K: usize = 4096;
const BYTES: usize = N * K * 2;

/// An authenticated setup recipe, not a native allocation or image admission.
pub struct EngineeringTp2FiniteHeadTransposeV1<'a> {
    source: &'a [u8],
    source_id: u64,
    source_sha256: [u8; 32],
    output_sha256: [u8; 32],
}

impl EngineeringTp2FiniteHeadTransposeV1<'_> {
    /// Original rank-zero LanguageModelHead source ID, never the transpose token.
    #[must_use]
    pub const fn source_id(&self) -> u64 {
        self.source_id
    }
    /// Digest of the original authenticated [151936,4096] BF16 section.
    #[must_use]
    pub const fn source_sha256(&self) -> [u8; 32] {
        self.source_sha256
    }
    /// Digest of the exact contiguous [4096,151936] BF16 upload.
    #[must_use]
    pub const fn sha256(&self) -> [u8; 32] {
        self.output_sha256
    }
    /// Exact distinct allocation byte count.
    #[must_use]
    pub const fn bytes(&self) -> usize {
        BYTES
    }
    /// Destination dimensions, in row-major order.
    #[must_use]
    pub const fn dimensions(&self) -> [u32; 2] {
        [K as u32, N as u32]
    }

    /// Stream exact BF16 bits in destination order with at most 4 MiB scratch.
    /// A sink error stops immediately; a partially uploaded native owner must
    /// become terminal. No floating-point conversion or GPU work occurs.
    /// # Errors
    /// Rejects an odd/out-of-range cap or a cap smaller than one output row.
    pub fn visit_chunks(
        &self,
        chunk_bytes: usize,
        mut sink: impl FnMut(usize, &[u8]) -> TpResult<()>,
    ) -> TpResult<()> {
        stream_transpose(self.source, N, K, chunk_bytes, &mut sink)
    }
}

impl<'a> EngineeringTp2FiniteWeightSourceV1<'a> {
    /// Derive an additional immutable transpose from the same authenticated
    /// original model section. This does not replace `global_uploads()`.
    /// # Errors
    /// Rejects any missing/mixed global source, extent or original tensor shape.
    pub fn head_transpose_upload(&self) -> TpResult<EngineeringTp2FiniteHeadTransposeV1<'a>> {
        let binding = self
            .composition
            .globals()
            .iter()
            .find(|(kind, _)| *kind == Qwen3TensorKind::LanguageModelHead)
            .ok_or("finite transpose original head binding missing")?
            .1;
        let part = self.part(0, QWEN3_NO_LAYER, Qwen3TensorKind::LanguageModelHead)?;
        if binding.rank() != 0
            || binding.id() == 0
            || binding.element_bytes() != 2
            || binding.elements() != N * K
            || part.source.len() != BYTES
            || part.shard.source_rows() != N as u32
            || part.shard.source_columns() != K as u32
            || part.shard.rows().start != 0
            || part.shard.rows().count != N as u32
            || part.shard.columns().start != 0
            || part.shard.columns().count != K as u32
        {
            return Err("finite transpose requires original complete NxK head".into());
        }
        let mut digest = Sha256::new();
        stream_transpose(part.source, N, K, MAX_CHUNK, &mut |_, bytes| {
            digest.update(bytes);
            Ok(())
        })?;
        Ok(EngineeringTp2FiniteHeadTransposeV1 {
            source: part.source,
            source_id: binding.id(),
            source_sha256: part.digest,
            output_sha256: digest.finalize().into(),
        })
    }
}

fn stream_transpose(
    source: &[u8],
    rows: usize,
    columns: usize,
    cap: usize,
    sink: &mut impl FnMut(usize, &[u8]) -> TpResult<()>,
) -> TpResult<()> {
    let bytes = rows
        .checked_mul(columns)
        .and_then(|n| n.checked_mul(2))
        .ok_or("finite transpose extent overflow")?;
    let output_row_bytes = rows.checked_mul(2).ok_or("finite transpose row overflow")?;
    if rows == 0
        || columns == 0
        || source.len() != bytes
        || !(2..=MAX_CHUNK).contains(&cap)
        || cap % 2 != 0
        || output_row_bytes > cap
    {
        return Err("finite transpose source or chunk bound".into());
    }
    // Batch complete destination rows. Every source bit appears exactly once;
    // the maximum temporary allocation is the caller's checked chunk cap.
    let batch_rows = (cap / output_row_bytes).min(columns);
    let mut chunk = vec![0; batch_rows * output_row_bytes];
    for first_column in (0..columns).step_by(batch_rows) {
        let count = batch_rows.min(columns - first_column);
        for source_row in 0..rows {
            let source_base = (source_row * columns + first_column) * 2;
            for local_column in 0..count {
                let from = source_base + local_column * 2;
                let to = (local_column * rows + source_row) * 2;
                chunk[to..to + 2].copy_from_slice(&source[from..from + 2]);
            }
        }
        sink(
            first_column * output_row_bytes,
            &chunk[..count * output_row_bytes],
        )?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "finite_head_transpose_tests.rs"]
mod tests;
