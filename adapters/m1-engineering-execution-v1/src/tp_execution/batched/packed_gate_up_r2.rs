//! Opt-in gate/up-only packed storage; original and MFMA layouts remain resident.

use super::{
    AuthenticatedModelWeightLayout, EngineeringTpArgumentV1, EngineeringTpBatchExecutionV2,
    EngineeringTpDispatchV1, EngineeringTpExecutionV1, EngineeringTpPagedPoolV1,
    EngineeringTpRankTransportV1, EngineeringTpReductionModeV3,
    EngineeringTpWaveTargetArtifactsV17, ModelConfig, Qwen3ModelRole, Qwen3TensorKind, Rank,
    Tensor, TpResult, allocate_tensor, dispatch,
};
use crate::tp_artifact::{
    ENGINEERING_TP_PACKED_BF16_EXPORTS_R2 as ROOTS, EngineeringTpArtifactV1,
    EngineeringTpPackedBf16ArtifactR2, PackedBf16BindingR2,
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const K: usize = 4096;
const N: usize = 12_288;
const LAYERS: usize = 36;
const PACK_WORDS: usize = K / 2;
const WEIGHT_WORDS: usize = N * PACK_WORDS;
const WEIGHT_BYTES: u64 = (LAYERS * 2 * N * K * 2) as u64;

#[derive(Clone, Copy)]
pub(super) struct WeightSource {
    pub(super) layer: u32,
    pub(super) kind: Qwen3TensorKind,
    pub(super) original: Tensor,
    pub(super) rows: (u32, u32),
    pub(super) columns: (u32, u32),
    pub(super) range: (u64, u64),
    pub(super) sha256: [u8; 32],
}

impl WeightSource {
    fn key(self) -> TpResult<(u32, u32)> {
        let tag = match self.kind {
            Qwen3TensorKind::GateProjection => 4,
            Qwen3TensorKind::UpProjection => 5,
            _ => return Err("packed gate/up source has an unsupported role".into()),
        };
        Ok((self.layer, tag))
    }
}

// A scoped borrow permits one full-size source matrix at a time. Authentication
// and fixed-shape packing stay in prepare_sources, not in the source adapter.
pub(super) trait WeightSourceBytes {
    fn with_source<T>(
        &mut self,
        source: &WeightSource,
        consume: impl FnOnce(&[u8]) -> TpResult<T>,
    ) -> TpResult<T>;
}

impl WeightSourceBytes for &[u8] {
    fn with_source<T>(
        &mut self,
        source: &WeightSource,
        consume: impl FnOnce(&[u8]) -> TpResult<T>,
    ) -> TpResult<T> {
        consume(super::super::section_bytes(self, source.range)?)
    }
}

fn source_roster(sources: &[WeightSource]) -> TpResult<BTreeSet<u64>> {
    let mut keys = BTreeSet::new();
    let mut originals = BTreeSet::new();
    let mut ranges = BTreeMap::new();
    for source in sources {
        let key = source.key()?;
        let end = source
            .range
            .0
            .checked_add(source.range.1)
            .ok_or("packed gate/up source range overflow")?;
        if source.layer >= 36
            || !keys.insert(key)
            || source.rows != (0, 12_288)
            || source.columns != (0, 4096)
            || source.original.elements != N * K
            || source.original.element_bytes != 2
            || !originals.insert(source.original.id)
            || source.range.1 != (N * K * 2) as u64
            || ranges.insert(source.range.0, end).is_some()
        {
            return Err("packed gate/up source is not a distinct complete NxK BF16 matrix".into());
        }
    }
    let mut previous_end = 0;
    for (&start, &end) in &ranges {
        if start < previous_end {
            return Err("packed gate/up authenticated source ranges overlap".into());
        }
        previous_end = end;
    }
    if keys.len() != LAYERS * 2
        || (0..36).any(|layer| [4, 5].iter().any(|&tag| !keys.contains(&(layer, tag))))
    {
        return Err("packed gate/up authenticated source roster is incomplete".into());
    }
    Ok(originals)
}

#[derive(Clone, Copy)]
pub(super) struct PackedWeight {
    pub(super) original: Tensor,
    pub(super) packed: Tensor,
    pub(super) source_sha256: [u8; 32],
}

pub(super) struct Workspace {
    pub(super) binding: PackedBf16BindingR2,
    pub(super) weights: BTreeMap<(u32, u32), PackedWeight>,
    pub(super) scratch: Tensor,
    pub(super) selected: Option<bool>,
    pub(super) bytes: u64,
}

pub(super) fn admit<R: EngineeringTpRankTransportV1>(
    transports: &mut [R],
    binding: PackedBf16BindingR2,
) -> TpResult<()> {
    if transports.len() != 1 || transports[0].peer_group_rank().is_some() {
        return Err("packed gate/up requires exactly one nonpeer TP1 transport".into());
    }
    if transports[0].supports_token_program() {
        return Err("packed gate/up excludes fixed 652-packet token programs".into());
    }
    transports[0].require_loaded_image(binding.hsaco(), &ROOTS)
}

pub(super) fn admit_or_close<R: EngineeringTpRankTransportV1>(
    transports: &mut [R],
    binding: PackedBf16BindingR2,
) -> TpResult<()> {
    if let Err(error) = admit(transports, binding) {
        let failures = transports
            .iter_mut()
            .enumerate()
            .filter_map(|(rank, transport)| {
                transport
                    .close()
                    .err()
                    .map(|close| format!("rank {rank}: {close}"))
            })
            .collect::<Vec<_>>();
        return Err(if failures.is_empty() {
            error
        } else {
            format!(
                "{error}; packed gate/up admission close: {}",
                failures.join("; ")
            )
        });
    }
    Ok(())
}

impl Workspace {
    fn prepare<R: EngineeringTpRankTransportV1>(
        inner: &mut EngineeringTpExecutionV1<R>,
        weights: &[u8],
        layout: &AuthenticatedModelWeightLayout,
        binding: PackedBf16BindingR2,
    ) -> TpResult<Self> {
        let model = inner.plan.model();
        if inner.plan.world_size() != 1
            || inner.ranks.len() != 1
            || inner.transports.len() != 1
            || model.role != Qwen3ModelRole::Target8B
            || model.layers != 36
            || model.hidden_size != 4096
            || inner.ranks[0].geometry.intermediate.count != 12_288
            || weights.len() as u64 != model.role.tensor_data_bytes()
        {
            return Err("packed gate/up setup requires the complete TP1 Qwen3-8B source".into());
        }
        let mut sources = Vec::with_capacity(LAYERS * 2);
        for ordinal in 0..layout.section_count(model.role) {
            let source_binding = layout
                .by_ordinal(model.role, ordinal)
                .map_err(|error| error.to_string())?;
            let metadata = source_binding.metadata();
            if !matches!(
                metadata.kind,
                Qwen3TensorKind::GateProjection | Qwen3TensorKind::UpProjection
            ) {
                continue;
            }
            if metadata.layer >= model.layers {
                return Err("packed gate/up source layer exceeds the model".into());
            }
            let original = inner.ranks[0].layers[metadata.layer as usize].weight(metadata.kind);
            let shard = inner
                .plan
                .tensor(metadata, 0)
                .map_err(|error| format!("packed gate/up shard: {error:?}"))?;
            sources.push(WeightSource {
                layer: metadata.layer,
                kind: metadata.kind,
                original,
                rows: (shard.rows().start, shard.rows().count),
                columns: (shard.columns().start, shard.columns().count),
                range: source_binding.destination_range(),
                sha256: source_binding.sha256(),
            });
        }
        let mut source_bytes = weights;
        Self::prepare_sources(
            &mut inner.transports[0],
            inner.ranks[0].normalized.id,
            binding,
            &sources,
            &mut source_bytes,
        )
    }

    pub(super) fn prepare_sources<R: EngineeringTpRankTransportV1>(
        transport: &mut R,
        normalized: u64,
        binding: PackedBf16BindingR2,
        sources: &[WeightSource],
        source_bytes: &mut impl WeightSourceBytes,
    ) -> TpResult<Self> {
        // Preflight all original identities before any packed allocation can
        // alias a later layer's original source.
        let originals = source_roster(sources)?;
        let mut packed = BTreeMap::new();
        let mut allocations = BTreeSet::new();
        let mut bytes = 0_u64;
        for source in sources {
            // Only one 96-MiB packed staging matrix is live; no model-sized duplicate.
            let storage = source_bytes.with_source(source, |bytes| {
                pack_authenticated_rows(bytes, source.sha256, N)
            })?;
            let tensor = allocate_tensor(transport, WEIGHT_WORDS, 4)?;
            if originals.contains(&tensor.id)
                || tensor.id == normalized
                || !allocations.insert(tensor.id)
            {
                return Err("packed gate/up allocator repeated a live identity".into());
            }
            for (chunk, payload) in storage.chunks(super::super::UPLOAD_CHUNK_BYTES).enumerate() {
                transport.write(tensor.id, chunk * super::super::UPLOAD_CHUNK_BYTES, payload)?;
            }
            bytes = bytes
                .checked_add(storage.len() as u64)
                .ok_or("packed weight byte overflow")?;
            packed.insert(
                source.key()?,
                PackedWeight {
                    original: source.original,
                    packed: tensor,
                    source_sha256: source.sha256,
                },
            );
        }
        if packed.len() != LAYERS * 2
            || bytes != WEIGHT_BYTES
            || (0..36).any(|layer| {
                [4, 5]
                    .iter()
                    .any(|&tag| !packed.contains_key(&(layer, tag)))
            })
            || originals.iter().any(|id| allocations.contains(id))
        {
            return Err(
                "packed gate/up authenticated source roster is incomplete or aliased".into(),
            );
        }
        let scratch = allocate_tensor(transport, PACK_WORDS, 4)?;
        if originals.contains(&scratch.id)
            || allocations.contains(&scratch.id)
            || scratch.id == normalized
        {
            return Err("packed activation scratch aliases retained model storage".into());
        }
        Ok(Self {
            binding,
            weights: packed,
            scratch,
            selected: None,
            bytes,
        })
    }

    pub(super) fn active(&self, rows: usize, published: usize) -> bool {
        self.selected == Some(true) && rows == 1 && published == 1
    }

    pub(super) fn commands(
        &self,
        rank: &Rank,
        layer: u32,
    ) -> TpResult<[EngineeringTpDispatchV1; 3]> {
        if self.selected != Some(true)
            || rank.geometry.rank != 0
            || layer >= 36
            || rank.normalized.element_bytes != 2
            || rank.normalized.elements < K
            || rank.gate.element_bytes != 2
            || rank.gate.elements < N
            || rank.up.element_bytes != 2
            || rank.up.elements < N
            || self.scratch.element_bytes != 4
            || self.scratch.elements != PACK_WORDS
        {
            return Err("packed gate/up active storage contract drifted".into());
        }
        let source = Tensor {
            elements: K,
            ..rank.normalized
        };
        let pack = dispatch(
            ROOTS[1],
            32,
            vec![
                source.read(),
                self.scratch.write(),
                EngineeringTpArgumentV1::U32(1),
                EngineeringTpArgumentV1::U32(4096),
            ],
        );
        let project = |kind, tag, output: Tensor| -> TpResult<EngineeringTpDispatchV1> {
            let weight = self
                .weights
                .get(&(layer, tag))
                .ok_or("packed gate/up role missing")?;
            let original = rank.layers[layer as usize].weight(kind);
            if (original.id, original.elements, original.element_bytes)
                != (weight.original.id, N * K, 2)
                || (weight.original.elements, weight.original.element_bytes) != (N * K, 2)
                || weight.packed.elements != WEIGHT_WORDS
                || weight.packed.element_bytes != 4
                || weight.packed.id == original.id
            {
                return Err("packed gate/up authenticated weight binding drifted".into());
            }
            let mut command = super::matrix(
                ROOTS[0],
                self.scratch,
                weight.packed,
                Tensor {
                    elements: N,
                    ..output
                },
                [1, 12_288, 4096, 1, tag],
            );
            command.grid_workgroups = 12_288;
            Ok(command)
        };
        let commands = [
            pack,
            project(Qwen3TensorKind::GateProjection, 4, rank.gate)?,
            project(Qwen3TensorKind::UpProjection, 5, rank.up)?,
        ];
        let ids = [
            rank.normalized.id,
            self.scratch.id,
            rank.gate.id,
            rank.up.id,
            self.weights[&(layer, 4)].packed.id,
            self.weights[&(layer, 5)].packed.id,
        ];
        if ids
            .iter()
            .enumerate()
            .any(|(index, id)| ids[..index].contains(id))
        {
            return Err("packed gate/up activation, weights or consumers alias each other".into());
        }
        // Validate every command before queuing any member of this layer's group.
        for command in &commands {
            super::super::row_profile::bind_mode(false, 32, false, command.clone())?;
        }
        Ok(commands)
    }
}

fn pack_authenticated_rows(source: &[u8], expected: [u8; 32], rows: usize) -> TpResult<Vec<u8>> {
    if !(1..=N).contains(&rows)
        || rows.checked_mul(K * 2) != Some(source.len())
        || Sha256::digest(source).as_slice() != expected
    {
        return Err("packed gate/up source digest or row extent drifted".into());
    }
    let mut output = vec![0; source.len()];
    for (input, packed) in source
        .chunks_exact(K * 2)
        .zip(output.chunks_exact_mut(K * 2))
    {
        pack_row(input, packed)?;
    }
    Ok(output)
}

pub(super) fn pack_row(source: &[u8], output: &mut [u8]) -> TpResult<()> {
    if source.len() != K * 2 || output.len() != K * 2 {
        return Err("packed lane-time row extent differs from K4096".into());
    }
    for group in 0..32 {
        for lane in 0..64 {
            let low = (group * 128 + lane) * 2;
            let word = (group * 64 + lane) * 4;
            output[word..word + 2].copy_from_slice(&source[low..low + 2]);
            output[word + 2..word + 4].copy_from_slice(&source[low + 128..low + 130]);
        }
    }
    Ok(())
}

impl<R: EngineeringTpRankTransportV1> EngineeringTpBatchExecutionV2<R> {
    /// Admits the tenth image before existing V19 composition allocation, then retains packed
    /// gate/up copies for an explicitly selected candidate or same-image control.
    /// # Errors
    /// Rejects unsupported images/storage and closes all owned transports on failure.
    #[allow(clippy::too_many_arguments)]
    pub fn new_wide32_with_ordered64_kv_packed_gate_up_r2(
        mut transports: Vec<R>,
        model: ModelConfig,
        weights: &[u8],
        layout: &AuthenticatedModelWeightLayout,
        pool: &EngineeringTpPagedPoolV1,
        artifacts: &EngineeringTpWaveTargetArtifactsV17<'_>,
        prefill: &EngineeringTpArtifactV1,
        split: &EngineeringTpArtifactV1,
        gemv: &EngineeringTpArtifactV1,
        copy: &EngineeringTpArtifactV1,
        packed: &EngineeringTpPackedBf16ArtifactR2,
    ) -> TpResult<Self> {
        let binding = packed.binding();
        admit_or_close(&mut transports, binding)?;
        let mut driver = Self::new_wide32_with_ordered64_kv_copy_v1(
            transports, model, weights, layout, pool, artifacts, prefill, split, gemv, copy,
        )?;
        let prepared = Workspace::prepare(&mut driver.inner, weights, layout, binding);
        driver.finish_packed_gate_up_setup(prepared)?;
        Ok(driver)
    }

    pub(super) fn finish_packed_gate_up_setup(
        &mut self,
        prepared: TpResult<Workspace>,
    ) -> TpResult<()> {
        match prepared {
            Ok(workspace) => self.packed_gate_up_r2 = Some(workspace),
            Err(error) => {
                self.poisoned = true;
                return Err(match self.close() {
                    Ok(()) => error,
                    Err(close) => format!("{error}; packed gate/up setup close: {close}"),
                });
            }
        }
        Ok(())
    }

    /// Selects only after the current V19/V27/split8 ordered64 composition is configured.
    /// `false` is the same-image/storage control; it submits no new roots.
    /// # Errors
    /// Rejects late/repeated selection, a foreign image or unsupported composition.
    pub fn configure_ordered64_kv_packed_gate_up_r2(
        &mut self,
        artifact: &EngineeringTpPackedBf16ArtifactR2,
        enabled: bool,
    ) -> TpResult<()> {
        self.configure_packed_gate_up_binding_r2(artifact.binding(), enabled)
    }

    pub(super) fn configure_packed_gate_up_binding_r2(
        &mut self,
        binding: PackedBf16BindingR2,
        enabled: bool,
    ) -> TpResult<()> {
        if !cfg!(all(
            feature = "c1-ordered64",
            not(feature = "model-timestamps")
        )) || self.last_batch != 0
            || self.completed_batches != 0
            || self.poisoned
            || self.inner.closed
            || self.row_capacity != 32
            || self.inner.plan.world_size() != 1
            || self.inner.ranks.len() != 1
            || self.inner.transports.len() != 1
            || self.inner.transports[0].peer_group_rank().is_some()
            || self.inner.transports[0].supports_token_program()
            || self.inner.sequences.is_some()
            || self.inner.large_kv
            || self.inner.draft_v10
            || self.numerical.is_some()
            || self.inner.residual_arithmetic.is_some()
            || self.inner.packed_c1.is_some()
            || !self.head_profile_configured
            || !self.prune_output_head
            || !self.projection_configured
            || !self.c1_wave_layers
            || !self.wave_attention
            || self.inner.ranks[0].dispatches != 0
            || self.fp32_logits.is_none()
            || self.fp32_argmax_v11.is_none()
            || self.fp32_argmax_v11 != self.admitted_argmax_v11
            || self.query_hoist_v14.is_none()
            || self.query_hoist_v14 != self.admitted_query_hoist_v14
            || self.wave_rmsnorm_v15.is_none()
            || self.wave_rmsnorm_v15 != self.admitted_wave_rmsnorm_v15
            || self.projection.mode != super::super::EngineeringTpProjectionModeV3::Mfma
            || self.reduction_mode() != EngineeringTpReductionModeV3::DeviceTp1V3
            || self.c1_packet_packing_v22 != Some(true)
            || self.c1_split_attention_v25 != Some(true)
            || self.prefill_kv_copy_v28 != Some(true)
            || self.partial_gemv_v28 != Some(false)
            || self.prefill32_pages_v1.is_some()
            || self.packed_down_r1.is_some()
            || self.c1_kv_copy_v19.is_none()
            || self.c1_kv_copy_v19 != self.admitted_c1_kv_copy_v19
            || self.admitted_prefill_kv_copy_v27.is_none()
            || self.admitted_partial_gemv_v20.is_none()
            || self.split_attention_workspace_v25.is_none()
            || !self.inner.ordered_batch_width.is_wide()
            || !self.inner.transports[0].supports_ordered_batches64()
            || self
                .inner
                .ordered_batches
                .as_ref()
                .is_none_or(|pending| !pending.is_empty())
            || self.packed_gate_up_r2.as_ref().is_none_or(|workspace| {
                workspace.binding != binding
                    || workspace.selected.is_some()
                    || workspace.bytes != WEIGHT_BYTES
                    || workspace.weights.len() != LAYERS * 2
                    || workspace.scratch.elements != PACK_WORDS
                    || workspace.scratch.element_bytes != 4
            })
        {
            return Err("packed gate/up requires its fresh admitted V19/V27/split8/baseline-GEMV ordered64 TP1 composition".into());
        }
        #[cfg(feature = "model-timestamps")]
        if self.inner.transports[0].model_timestamps_enabled() {
            return Err("packed gate/up is separate from the historical timestamp profile".into());
        }
        self.packed_gate_up_r2
            .as_mut()
            .expect("validated packed workspace")
            .selected = Some(enabled);
        Ok(())
    }

    pub(super) fn packed_gate_up_active(&self, rows: usize, published: usize) -> bool {
        self.packed_gate_up_r2
            .as_ref()
            .is_some_and(|workspace| workspace.active(rows, published))
    }

    /// Separate route identity; existing default engines continue to return baseline.
    #[must_use]
    pub fn packed_gate_up_mode(&self) -> &'static str {
        match self
            .packed_gate_up_r2
            .as_ref()
            .map(|workspace| workspace.selected)
        {
            Some(Some(true)) => "packed-gate-up-u32-r2",
            Some(None) => "unconfigured-packed-gate-up-r2",
            _ => "baseline",
        }
    }

    /// Additional immutable weight bytes, excluding the 8192-byte activation scratch.
    #[must_use]
    pub fn packed_gate_up_weight_bytes(&self) -> u64 {
        self.packed_gate_up_r2
            .as_ref()
            .map_or(0, |workspace| workspace.bytes)
    }

    /// Actual additional activation scratch extent; zero for ordinary constructors.
    #[must_use]
    pub fn packed_gate_up_activation_scratch_bytes(&self) -> u64 {
        self.packed_gate_up_r2.as_ref().map_or(0, |workspace| {
            (workspace.scratch.elements as u64) * u64::from(workspace.scratch.element_bytes)
        })
    }

    /// Original authenticated tensor digests in closed (layer, role) order.
    #[must_use]
    pub fn packed_gate_up_source_identities(&self) -> Vec<(u32, u32, [u8; 32])> {
        self.packed_gate_up_r2
            .as_ref()
            .map_or_else(Vec::new, |workspace| {
                workspace
                    .weights
                    .iter()
                    .map(|(&(layer, role), weight)| (layer, role, weight.source_sha256))
                    .collect()
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packed_gate_up_host_layout_preserves_all_bf16_bits_and_lane_time_order() {
        for first in (0..65_536_u32).step_by(K) {
            let source = (first..first + 4096)
                .flat_map(|bits| u16::try_from(bits).unwrap().to_le_bytes())
                .collect::<Vec<_>>();
            let mut packed = vec![0; K * 2];
            pack_row(&source, &mut packed).unwrap();
            for group in 0..32 {
                for lane in 0..64 {
                    let word = (group * 64 + lane) * 4;
                    let low = (group * 128 + lane) * 2;
                    assert_eq!(&packed[word..word + 2], &source[low..low + 2]);
                    assert_eq!(&packed[word + 2..word + 4], &source[low + 128..low + 130]);
                }
            }
            assert_ne!(source, packed);
        }
    }

    #[test]
    fn packed_gate_up_host_packing_authenticates_before_layout_conversion() {
        let source = (0..8192_u32)
            .flat_map(|bits| u16::try_from(bits).unwrap().to_le_bytes())
            .collect::<Vec<_>>();
        let digest: [u8; 32] = Sha256::digest(&source).into();
        let actual = pack_authenticated_rows(&source, digest, 2).unwrap();
        assert_eq!(actual.len(), source.len());
        assert_eq!(&actual[K * 2..K * 2 + 2], &source[K * 2..K * 2 + 2]);
        assert_eq!(&actual[actual.len() - 2..], &source[source.len() - 2..]);
        let mut wrong = digest;
        wrong[0] ^= 1;
        assert!(pack_authenticated_rows(&source, wrong, 2).is_err());
        for rows in [0, 1, N + 1] {
            assert!(pack_authenticated_rows(&source, digest, rows).is_err());
        }
        assert!(pack_row(&source[..K * 2 - 2], &mut vec![0; K * 2]).is_err());
        assert!(pack_row(&source[..K * 2], &mut vec![0; K * 2 - 2]).is_err());
    }
}
