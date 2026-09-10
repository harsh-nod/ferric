//! Explicit projection selection and setup-only transposed weight storage.

use super::{
    AuthenticatedModelWeightLayout, EngineeringTpDispatchV1, EngineeringTpExecutionV1,
    EngineeringTpRankTransportV1, QWEN3_NO_LAYER, Qwen3TensorKind, Tensor, TpResult,
    UPLOAD_CHUNK_BYTES, allocate_tensor, section_bytes,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Independently selected arithmetic profile; no unsupported-mode fallback.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum EngineeringTpProjectionModeV3 {
    /// Original scalar batched projection.
    #[default]
    Baseline,
    /// One cooperative wave per output element, with original `NxK` weights.
    Wave,
    /// BF16 MFMA projection with separately resident `KxN` weights.
    Mfma,
    /// Wave for one active row, MFMA for multiple active rows.
    Auto,
}

impl EngineeringTpProjectionModeV3 {
    /// Stable measurement label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Baseline => "baseline",
            Self::Wave => "wave",
            Self::Mfma => "mfma",
            Self::Auto => "auto",
        }
    }

    /// Whether setup must retain transposed weights and admit MFMA roots.
    #[must_use]
    pub const fn requires_mfma(self) -> bool {
        matches!(self, Self::Mfma | Self::Auto)
    }
}

#[derive(Default)]
pub(super) struct ProjectionPolicy {
    pub(super) mode: EngineeringTpProjectionModeV3,
    transposed: Vec<BTreeMap<u64, Tensor>>,
    pub(super) bytes: u64,
}

impl ProjectionPolicy {
    pub(super) fn prepare<R: EngineeringTpRankTransportV1>(
        mode: EngineeringTpProjectionModeV3,
        inner: &mut EngineeringTpExecutionV1<R>,
        weights: &[u8],
        layout: &AuthenticatedModelWeightLayout,
    ) -> TpResult<Self> {
        let mut policy = Self {
            mode,
            transposed: vec![BTreeMap::new(); inner.ranks.len()],
            bytes: 0,
        };
        if !mode.requires_mfma() {
            return Ok(policy);
        }
        let model = inner.plan.model();
        if weights.len() as u64 != model.role.tensor_data_bytes() {
            return Err("transposed source model byte length drifted".into());
        }
        for ordinal in 0..layout.section_count(model.role) {
            let binding = layout
                .by_ordinal(model.role, ordinal)
                .map_err(|error| error.to_string())?;
            let metadata = binding.metadata();
            if !matches!(
                metadata.kind,
                Qwen3TensorKind::QueryProjection
                    | Qwen3TensorKind::KeyProjection
                    | Qwen3TensorKind::ValueProjection
                    | Qwen3TensorKind::OutputProjection
                    | Qwen3TensorKind::GateProjection
                    | Qwen3TensorKind::UpProjection
                    | Qwen3TensorKind::DownProjection
                    | Qwen3TensorKind::LanguageModelHead
            ) {
                continue;
            }
            let source = section_bytes(weights, binding.destination_range())?;
            if Sha256::digest(source).as_slice() != binding.sha256() {
                return Err("transposed source tensor digest drifted".into());
            }
            for (index, (rank, transport)) in
                inner.ranks.iter().zip(&mut inner.transports).enumerate()
            {
                if metadata.layer == QWEN3_NO_LAYER && index != 0 {
                    continue;
                }
                let original = if metadata.layer == QWEN3_NO_LAYER {
                    rank.global(metadata.kind)
                } else {
                    rank.layers[metadata.layer as usize].weight(metadata.kind)
                };
                let shard = inner
                    .plan
                    .tensor(metadata, rank.geometry.rank)
                    .map_err(|error| format!("transposed shard: {error:?}"))?;
                let n = shard.rows().count as usize;
                let k = shard.columns().count as usize;
                let elements = n.checked_mul(k).ok_or("transposed tensor size overflow")?;
                if elements != original.elements || original.element_bytes != 2 {
                    return Err("transposed source shape differs from resident tensor".into());
                }
                let mut transposed =
                    vec![0; elements.checked_mul(2).ok_or("transposed byte overflow")?];
                let mut row = vec![0; k * 2];
                for output in 0..n {
                    shard
                        .copy_bf16_row_into(
                            source,
                            u32::try_from(output).map_err(|_| "transposed row overflow")?,
                            &mut row,
                        )
                        .map_err(|error| format!("transposed BF16 row: {error:?}"))?;
                    transpose_row(&row, n, output, &mut transposed)?;
                }
                let tensor = allocate_tensor(transport, elements, 2)?;
                for (chunk, bytes) in transposed.chunks(UPLOAD_CHUNK_BYTES).enumerate() {
                    transport.write(tensor.id, chunk * UPLOAD_CHUNK_BYTES, bytes)?;
                }
                policy.bytes = policy
                    .bytes
                    .checked_add(transposed.len() as u64)
                    .ok_or("transposed allocation counter overflow")?;
                if policy.transposed[index]
                    .insert(original.id, tensor)
                    .is_some()
                {
                    return Err("duplicate transposed tensor identity".into());
                }
            }
        }
        Ok(policy)
    }

    pub(super) fn command(
        &self,
        rank: usize,
        baseline: &'static str,
        input: Tensor,
        weight: Tensor,
        output: Tensor,
        shape: [u32; 5],
    ) -> EngineeringTpDispatchV1 {
        use EngineeringTpProjectionModeV3::{Auto, Baseline, Mfma, Wave};
        let [rows, n, ..] = shape;
        let partial = baseline == super::batched::PARTIAL;
        let mfma = self.mode == Mfma || (self.mode == Auto && rows > 1);
        let kernel = if self.mode == Baseline {
            baseline
        } else if mfma && partial {
            "ferric_qwen3_tp_mfma_gemm_partial_f32_v3"
        } else if mfma {
            "ferric_qwen3_tp_mfma_gemm_bf16_v3"
        } else if partial {
            "ferric_qwen3_tp_wave_gemv_partial_f32_v3"
        } else {
            "ferric_qwen3_tp_wave_gemv_bf16_v3"
        };
        let weight = if mfma {
            *self.transposed[rank]
                .get(&weight.id)
                .expect("complete authenticated transposed projection roster")
        } else {
            weight
        };
        let mut command = super::batched::matrix(kernel, input, weight, output, shape);
        if self.mode == Wave || (self.mode == Auto && !mfma) {
            command.grid_workgroups = rows
                .checked_mul(n)
                .expect("admitted bounded projection geometry");
        }
        command
    }
}

fn transpose_row(row: &[u8], rows: usize, index: usize, output: &mut [u8]) -> TpResult<()> {
    if row.is_empty()
        || !row.len().is_multiple_of(2)
        || index >= rows
        || rows.checked_mul(row.len()) != Some(output.len())
    {
        return Err("transposed row extent mismatch".into());
    }
    for (column, value) in row.chunks_exact(2).enumerate() {
        let destination = (column * rows + index) * 2;
        output[destination..destination + 2].copy_from_slice(value);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tp_execution::EngineeringTpArgumentV1;

    #[test]
    fn transposed_rows_preserve_bf16_bits_and_use_k_by_n_order() {
        let mut output = vec![0xa5; 30];
        for row in 0..3 {
            let bytes = (0..5)
                .flat_map(|column| u16::try_from(row * 5 + column + 1).unwrap().to_le_bytes())
                .collect::<Vec<_>>();
            transpose_row(&bytes, 3, row, &mut output).unwrap();
        }
        let bits = output
            .chunks_exact(2)
            .map(|bytes| u16::from_le_bytes(bytes.try_into().unwrap()))
            .collect::<Vec<_>>();
        assert_eq!(bits, [1, 6, 11, 2, 7, 12, 3, 8, 13, 4, 9, 14, 5, 10, 15]);
        let unchanged = output.clone();
        assert!(transpose_row(&[0; 10], 3, 3, &mut output).is_err());
        assert!(transpose_row(&[0; 9], 3, 1, &mut output).is_err());
        assert!(transpose_row(&[0; 8], 3, 1, &mut output).is_err());
        assert_eq!(output, unchanged);
    }

    #[test]
    fn projection_selection_binds_grid_layout_and_active_row_threshold() {
        use EngineeringTpProjectionModeV3::{Auto, Baseline, Mfma, Wave};
        let input = Tensor {
            id: 1,
            elements: 16 * 4096,
            element_bytes: 2,
        };
        let weight = Tensor {
            id: 2,
            elements: 512 * 4096,
            element_bytes: 2,
        };
        let output = Tensor {
            id: 3,
            elements: 16 * 512,
            element_bytes: 2,
        };
        for mode in [Baseline, Wave, Mfma, Auto] {
            let policy = ProjectionPolicy {
                mode,
                transposed: vec![BTreeMap::from([(2, Tensor { id: 4, ..weight })])],
                bytes: 0,
            };
            for rows in [1, 3, 16] {
                let command = policy.command(
                    0,
                    "ferric_qwen3_tp_batch_gemm_bf16_f32_bf16_v2",
                    input,
                    weight,
                    output,
                    [rows, 512, 4096, 8, 1],
                );
                let mfma = mode == Mfma || (mode == Auto && rows > 1);
                let wave = mode == Wave || (mode == Auto && !mfma);
                assert_eq!(command.grid_workgroups, if wave { rows * 512 } else { 32 });
                assert_eq!(command.kernel.contains("_mfma_"), mfma);
                assert_eq!(command.kernel.contains("_wave_"), wave);
                let EngineeringTpArgumentV1::Buffer { id, .. } = command.arguments[1] else {
                    panic!("weight binding");
                };
                assert_eq!(id, if mfma { 4 } else { 2 });
                assert_eq!(command.arguments[3], EngineeringTpArgumentV1::U32(rows));
            }
        }
    }
}
