//! Explicit projection selection and setup-only transposed weight storage.

use super::{
    AuthenticatedModelWeightLayout, EngineeringTpDispatchV1, EngineeringTpExecutionV1,
    EngineeringTpRankTransportV1, QWEN3_NO_LAYER, Qwen3TensorKind, Tensor, TpResult,
    UPLOAD_CHUNK_BYTES, allocate_tensor, section_bytes,
};
use ferric_engine::tensor_parallel::Qwen3TensorParallelTensorV1;
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
    #[cfg(test)]
    pub(super) fn synthetic_mfma_for_recording(original: u64, transposed: Tensor) -> Self {
        Self {
            mode: EngineeringTpProjectionModeV3::Mfma,
            transposed: vec![BTreeMap::from([(original, transposed)])],
            bytes: 0,
        }
    }

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
                transpose_shard(source, &shard, &mut transposed)?;
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
        self.command_with_mode(self.mode, rank, baseline, input, weight, output, shape)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn layer_command(
        &self,
        c1_wave: bool,
        rank: usize,
        baseline: &'static str,
        input: Tensor,
        weight: Tensor,
        output: Tensor,
        shape: [u32; 5],
    ) -> EngineeringTpDispatchV1 {
        let mode = if c1_wave && shape[0] == 1 {
            EngineeringTpProjectionModeV3::Wave
        } else {
            self.mode
        };
        self.command_with_mode(mode, rank, baseline, input, weight, output, shape)
    }

    #[allow(clippy::too_many_arguments)]
    fn command_with_mode(
        &self,
        mode: EngineeringTpProjectionModeV3,
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
        let mfma = mode == Mfma || (mode == Auto && rows > 1);
        let kernel = if mode == Baseline {
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
        if mode == Wave || (mode == Auto && !mfma) {
            command.grid_workgroups = rows
                .checked_mul(n)
                .expect("admitted bounded projection geometry");
        }
        command
    }
}

const TRANSPOSE_TILE: usize = 32;

fn transpose_shard(
    source: &[u8],
    shard: &Qwen3TensorParallelTensorV1,
    output: &mut [u8],
) -> TpResult<()> {
    let rows = shard.rows().count as usize;
    for first in (0..rows).step_by(TRANSPOSE_TILE) {
        let count = (rows - first).min(TRANSPOSE_TILE);
        let mut source_rows: [&[u8]; TRANSPOSE_TILE] = [&[]; TRANSPOSE_TILE];
        for (offset, row) in source_rows[..count].iter_mut().enumerate() {
            let (start, length) = shard
                .source_row_bytes(
                    u32::try_from(first + offset).map_err(|_| "transposed row overflow")?,
                    u64::try_from(source.len()).map_err(|_| "transposed source overflow")?,
                )
                .map_err(|error| format!("transposed BF16 row: {error:?}"))?;
            let start = usize::try_from(start).map_err(|_| "transposed offset overflow")?;
            let length = usize::try_from(length).map_err(|_| "transposed row length overflow")?;
            let end = start
                .checked_add(length)
                .ok_or("transposed row end overflow")?;
            *row = source
                .get(start..end)
                .ok_or("transposed source row extent")?;
        }
        transpose_tile(&source_rows[..count], rows, first, output)?;
    }
    Ok(())
}

fn transpose_tile(source: &[&[u8]], rows: usize, first: usize, output: &mut [u8]) -> TpResult<()> {
    let row_bytes = source.first().map_or(0, |row| row.len());
    if row_bytes == 0
        || !row_bytes.is_multiple_of(2)
        || source.len() > TRANSPOSE_TILE
        || first.checked_add(source.len()).is_none_or(|end| end > rows)
        || rows.checked_mul(row_bytes) != Some(output.len())
        || source.iter().any(|row| row.len() != row_bytes)
    {
        return Err("transposed tile extent mismatch".into());
    }
    let columns = row_bytes / 2;
    let mut tile = [0_u8; TRANSPOSE_TILE * TRANSPOSE_TILE * 2];
    // Stage compact rows so both the source fetches and destination stores use
    // contiguous cache lines even when either matrix stride aliases cache sets.
    for column_start in (0..columns).step_by(TRANSPOSE_TILE) {
        let column_end = column_start + (columns - column_start).min(TRANSPOSE_TILE);
        let tile_columns = column_end - column_start;
        for (offset, row) in source.iter().enumerate() {
            let start = offset * TRANSPOSE_TILE * 2;
            tile[start..start + tile_columns * 2]
                .copy_from_slice(&row[column_start * 2..column_end * 2]);
        }
        for column in column_start..column_end {
            let destination = (column * rows + first) * 2;
            let tile_column = (column - column_start) * 2;
            for (value, row) in output[destination..destination + source.len() * 2]
                .chunks_exact_mut(2)
                .zip(tile.chunks_exact(TRANSPOSE_TILE * 2))
            {
                value.copy_from_slice(&row[tile_column..tile_column + 2]);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
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

    fn patterned_bytes(elements: usize) -> Vec<u8> {
        (0..elements)
            .flat_map(|index| {
                let mixed = index.wrapping_mul(17) ^ (index >> 12) ^ (index >> 24);
                u16::try_from(mixed & 0xffff).unwrap().to_le_bytes()
            })
            .collect()
    }

    #[test]
    fn tiled_transpose_preserves_every_bf16_pattern_and_tile_tails() {
        for (rows, columns) in [
            (1, 1),
            (1, 65),
            (65, 1),
            (31, 33),
            (32, 32),
            (33, 63),
            (65, 65),
            (256, 256),
        ] {
            let source = if rows == 256 {
                (0..=u16::MAX)
                    .flat_map(u16::to_le_bytes)
                    .collect::<Vec<_>>()
            } else {
                patterned_bytes(rows * columns)
            };
            let mut expected = vec![0; source.len()];
            let mut actual = vec![0xa5; source.len()];
            for (index, row) in source.chunks_exact(columns * 2).enumerate() {
                transpose_row(row, rows, index, &mut expected).unwrap();
            }
            for first in (0..rows).step_by(TRANSPOSE_TILE) {
                let count = (rows - first).min(TRANSPOSE_TILE);
                let tile = (first..first + count)
                    .map(|row| &source[row * columns * 2..(row + 1) * columns * 2])
                    .collect::<Vec<_>>();
                transpose_tile(&tile, rows, first, &mut actual).unwrap();
            }
            assert_eq!(actual, expected, "{rows}x{columns}");
        }
        let mut output = [0xa5; 24];
        for (tile, rows, first) in [
            (vec![], 3, 0),
            (vec![&[0_u8; 8][..]], 3, 3),
            (vec![&[0_u8; 7][..]], 3, 0),
            (vec![&[0_u8; 6][..]], 3, 0),
            (vec![&[0_u8; 8][..], &[0_u8; 6][..]], 3, 0),
            (vec![&[0_u8; 8][..]], usize::MAX, 0),
            (vec![&[0_u8; 8][..]], 3, usize::MAX),
        ] {
            assert!(transpose_tile(&tile, rows, first, &mut output).is_err());
            assert_eq!(output, [0xa5; 24]);
        }
    }

    #[test]
    #[ignore = "explicit host-only full production arrays and setup-helper microbenchmark"]
    fn production_shard_transpose_exactness_and_host_microbenchmark() {
        use ferric_engine::tensor_parallel::Qwen3TensorParallelPlanV1;
        use ferric_spec::{
            Identity, ModelConfig, Qwen3ModelRole, Qwen3TensorMetadata, TensorDType,
        };
        use std::time::Instant;
        let model = ModelConfig {
            role: Qwen3ModelRole::Target8B,
            model_id: Identity::new([1; 32]),
            config_id: Identity::new([2; 32]),
            vocabulary_size: 151_936,
            layers: 36,
            hidden_size: 4096,
            intermediate_size: 12_288,
            query_heads: 32,
            kv_heads: 8,
            head_dim: 128,
            max_position_embeddings: 40_960,
            rope_theta: 1_000_000,
            tie_word_embeddings: false,
        };
        for (kind, source_rows, source_columns) in [
            (Qwen3TensorKind::QueryProjection, 4096, 4096),
            (Qwen3TensorKind::KeyProjection, 1024, 4096),
            (Qwen3TensorKind::ValueProjection, 1024, 4096),
            (Qwen3TensorKind::OutputProjection, 4096, 4096),
            (Qwen3TensorKind::GateProjection, 12_288, 4096),
            (Qwen3TensorKind::UpProjection, 12_288, 4096),
            (Qwen3TensorKind::DownProjection, 4096, 12_288),
            (Qwen3TensorKind::LanguageModelHead, 151_936, 4096),
        ] {
            let source = patterned_bytes(source_rows as usize * source_columns as usize);
            let metadata = Qwen3TensorMetadata {
                role: model.role,
                kind,
                layer: if kind == Qwen3TensorKind::LanguageModelHead {
                    QWEN3_NO_LAYER
                } else {
                    0
                },
                dtype: TensorDType::Bf16,
                rank: 2,
                dimension_0: source_rows,
                dimension_1: source_columns,
            };
            for world in [1, 2, 8] {
                let plan = Qwen3TensorParallelPlanV1::new(model, world).unwrap();
                for rank in 0..world {
                    if kind == Qwen3TensorKind::LanguageModelHead && rank != 0 {
                        continue;
                    }
                    let shard = plan.tensor(metadata, rank).unwrap();
                    let n = shard.rows().count as usize;
                    let k = shard.columns().count as usize;
                    let mut expected = vec![0xa5; n * k * 2];
                    let mut actual = vec![0xa5; n * k * 2];
                    let mut row = vec![0; k * 2];
                    for repeat in 0..3 {
                        let mut baseline = || {
                            let start = Instant::now();
                            for index in 0..n {
                                shard
                                    .copy_bf16_row_into(
                                        &source,
                                        u32::try_from(index).unwrap(),
                                        &mut row,
                                    )
                                    .unwrap();
                                transpose_row(&row, n, index, &mut expected).unwrap();
                            }
                            start.elapsed().as_nanos()
                        };
                        let mut tiled = || {
                            let start = Instant::now();
                            transpose_shard(&source, &shard, &mut actual).unwrap();
                            start.elapsed().as_nanos()
                        };
                        let (baseline_ns, tiled_ns) = if repeat % 2 == 0 {
                            (baseline(), tiled())
                        } else {
                            let tiled_ns = tiled();
                            (baseline(), tiled_ns)
                        };
                        assert_eq!(actual, expected, "{kind:?} TP{world} rank{rank}");
                        println!(
                            "{{\"schema\":\"FerricHostTransposeMicrobenchmarkV1\",\"kind\":\"{kind:?}\",\"tp\":{world},\"rank\":{rank},\"n\":{n},\"k\":{k},\"repeat\":{repeat},\"bytes\":{},\"baseline_ns\":{baseline_ns},\"tiled_ns\":{tiled_ns},\"full_array_exact\":true,\"model_timing\":false}}",
                            actual.len()
                        );
                    }
                    let before = actual.clone();
                    assert!(
                        transpose_shard(&source[..source.len() - 1], &shard, &mut actual).is_err()
                    );
                    assert_eq!(actual, before);
                    assert!(
                        transpose_shard(&source, &shard, &mut actual[..before.len() - 1]).is_err()
                    );
                    assert_eq!(actual, before);
                }
            }
        }
    }

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
    fn layer_c1_wave_reuses_existing_commands_for_all_rows_and_seven_layer_shapes() {
        use EngineeringTpProjectionModeV3::{Auto, Baseline, Mfma, Wave};
        let input = Tensor { id: 1, elements: 32 * 12_288, element_bytes: 2 };
        let weight = Tensor { id: 2, elements: 4096 * 12_288, element_bytes: 2 };
        let output = Tensor { id: 3, elements: 32 * 12_288, element_bytes: 2 };
        for mode in [Baseline, Wave, Mfma, Auto] {
            let policy = ProjectionPolicy {
                mode,
                transposed: vec![BTreeMap::from([(2, Tensor { id: 4, ..weight })])],
                bytes: 7,
            };
            for rows in 1..=32 {
                for (partial, n, k, tag) in [
                    (false, 4096, 4096, 1),
                    (false, 1024, 4096, 2),
                    (false, 1024, 4096, 3),
                    (true, 4096, 4096, 1),
                    (false, 12_288, 4096, 4),
                    (false, 12_288, 4096, 5),
                    (true, 4096, 12_288, 2),
                ] {
                    let root = if partial { super::super::batched::PARTIAL } else { super::super::batched::GEMM };
                    let output = Tensor { element_bytes: if partial { 4 } else { 2 }, ..output };
                    let shape = [rows, n, k, 1, tag];
                    let legacy = policy.command(0, root, input, weight, output, shape);
                    assert_eq!(policy.layer_command(false, 0, root, input, weight, output, shape), legacy);
                    if mode == Mfma {
                        let expected = if rows == 1 {
                            ProjectionPolicy { mode: Wave, ..ProjectionPolicy::default() }.command(0, root, input, weight, output, shape)
                        } else { legacy };
                        assert_eq!(policy.layer_command(true, 0, root, input, weight, output, shape), expected);
                        assert_eq!(policy.mode, Mfma);
                        assert_eq!(policy.bytes, 7);
                    }
                }
            }
        }
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
