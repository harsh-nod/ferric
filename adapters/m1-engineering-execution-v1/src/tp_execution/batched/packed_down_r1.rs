//! Default-off, decode-only packed down projection with authenticated resident weights.

use super::{
    AuthenticatedModelWeightLayout, EngineeringTpArgumentV1, EngineeringTpBatchExecutionV2,
    EngineeringTpDispatchV1, EngineeringTpExecutionV1, EngineeringTpPagedPoolV1,
    EngineeringTpRankTransportV1, EngineeringTpReductionModeV3,
    EngineeringTpWaveTargetArtifactsV17, ModelConfig, Qwen3ModelRole, Qwen3TensorKind, Rank,
    Tensor, TpResult, allocate_tensor, dispatch,
};
use crate::tp_artifact::{
    ENGINEERING_TP_PACKED_DOWN_EXPORTS_R1 as ROOTS, EngineeringTpArtifactV1,
    EngineeringTpPackedDownArtifactR1, PackedDownBindingR1,
};
use crate::tp_paged::EngineeringTpPreparedBatchV1;
use crate::tp_scheduler::{TpBatchRowKindV1, TpBatchRowV1};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const K: usize = 12_288;
const N: usize = 4096;
const LAYERS: usize = 36;
const PACK_WORDS: usize = K / 2;
const WEIGHT_WORDS: usize = N * PACK_WORDS;
const WEIGHT_BYTES: u64 = (LAYERS * N * K * 2) as u64;

#[derive(Clone, Copy)]
pub(super) struct WeightSource {
    pub(super) layer: u32,
    pub(super) original: Tensor,
    pub(super) rows: (u32, u32),
    pub(super) columns: (u32, u32),
    pub(super) range: (u64, u64),
    pub(super) sha256: [u8; 32],
}

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
    let mut layers = BTreeSet::new();
    let mut originals = BTreeSet::new();
    let mut ranges = BTreeMap::new();
    for source in sources {
        let end = source
            .range
            .0
            .checked_add(source.range.1)
            .ok_or("packed down source overflow")?;
        if source.layer >= 36
            || !layers.insert(source.layer)
            || source.rows != (0, 4096)
            || source.columns != (0, 12_288)
            || source.original.elements != N * K
            || source.original.element_bytes != 2
            || !originals.insert(source.original.id)
            || source.range.1 != (N * K * 2) as u64
            || ranges.insert(source.range.0, end).is_some()
        {
            return Err(
                "packed down requires 36 distinct complete NxK BF16 source matrices".into(),
            );
        }
    }
    let mut previous_end = 0;
    for (&start, &end) in &ranges {
        if start < previous_end {
            return Err("packed down authenticated source ranges overlap".into());
        }
        previous_end = end;
    }
    if layers.len() != LAYERS || (0..36).any(|layer| !layers.contains(&layer)) {
        return Err("packed down authenticated source roster is incomplete".into());
    }
    Ok(originals)
}

#[derive(Clone, Copy)]
pub(super) struct PackedWeight {
    pub(super) original: Tensor,
    pub(super) packed: Tensor,
    pub(super) source_sha256: [u8; 32],
}

pub(super) struct BatchBinding {
    pool: u64,
    batch: u64,
    rows: Vec<(u32, u32, TpBatchRowKindV1)>,
    published: usize,
}

impl BatchBinding {
    fn validate(&self, batch: &EngineeringTpPreparedBatchV1, published: usize) -> TpResult<bool> {
        if self.pool != batch.pool_identity()
            || self.batch != batch.id()
            || self.rows.len() != batch.rows().len()
            || self.published != published
            || self
                .rows
                .iter()
                .zip(batch.rows())
                .any(|(&(token, position, _), row)| {
                    token != row.token() || position != row.position()
                })
        {
            return Err("packed down scheduler/prepared batch identity drifted".into());
        }
        Ok(self.rows.len() == 1 && published == 1 && self.rows[0].2 == TpBatchRowKindV1::Decode)
    }
}

pub(super) struct Workspace {
    pub(super) binding: PackedDownBindingR1,
    pub(super) weights: BTreeMap<u32, PackedWeight>,
    pub(super) scratch: Tensor,
    pub(super) selected: Option<bool>,
    pub(super) bytes: u64,
    pub(super) batch: Option<BatchBinding>,
}

pub(super) fn admit<R: EngineeringTpRankTransportV1>(
    transports: &mut [R],
    binding: PackedDownBindingR1,
) -> TpResult<()> {
    if !cfg!(all(
        feature = "c1-ordered64",
        not(feature = "model-timestamps"),
        not(feature = "c1-token-program")
    )) || transports.len() != 1
        || transports[0].peer_group_rank().is_some()
        || transports[0].supports_token_program()
        || !transports[0].supports_ordered_batches64()
    {
        return Err("packed down requires an ordinary nonpeer TP1 ordered64 transport".into());
    }
    transports[0].require_loaded_image(binding.hsaco(), &ROOTS)
}

fn close_admission<R: EngineeringTpRankTransportV1>(transports: &mut [R], error: String) -> String {
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
    if failures.is_empty() {
        error
    } else {
        format!(
            "{error}; packed down admission close: {}",
            failures.join("; ")
        )
    }
}

impl Workspace {
    fn prepare<R: EngineeringTpRankTransportV1>(
        inner: &mut EngineeringTpExecutionV1<R>,
        weights: &[u8],
        layout: &AuthenticatedModelWeightLayout,
        binding: PackedDownBindingR1,
        extra_retained: &[u64],
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
            return Err("packed down setup requires complete TP1 Qwen3-8B source".into());
        }
        let mut sources = Vec::with_capacity(LAYERS);
        for ordinal in 0..layout.section_count(model.role) {
            let source = layout
                .by_ordinal(model.role, ordinal)
                .map_err(|error| error.to_string())?;
            let metadata = source.metadata();
            if metadata.kind != Qwen3TensorKind::DownProjection {
                continue;
            }
            if metadata.layer >= model.layers {
                return Err("packed down source layer exceeds model".into());
            }
            let original = inner.ranks[0].layers[metadata.layer as usize].weight(metadata.kind);
            let shard = inner
                .plan
                .tensor(metadata, 0)
                .map_err(|error| format!("packed down shard: {error:?}"))?;
            sources.push(WeightSource {
                layer: metadata.layer,
                original,
                rows: (shard.rows().start, shard.rows().count),
                columns: (shard.columns().start, shard.columns().count),
                range: source.destination_range(),
                sha256: source.sha256(),
            });
        }
        let rank = &inner.ranks[0];
        let mut retained = [
            rank.hidden,
            rank.normalized,
            rank.q,
            rank.k,
            rank.v,
            rank.q_normalized,
            rank.k_normalized,
            rank.q_rotated,
            rank.k_rotated,
            rank.attention,
            rank.gate,
            rank.up,
            rank.activation,
            rank.partial,
            rank.cos,
            rank.sin,
            rank.empty,
            rank.token,
            rank.logits,
            rank.choice,
        ]
        .map(|tensor| tensor.id)
        .into_iter()
        .collect::<BTreeSet<_>>();
        retained.extend(rank.globals.iter().map(|(_, tensor)| tensor.id));
        retained.extend(extra_retained.iter().copied());
        for layer in &rank.layers {
            retained.extend(layer.weights.iter().map(|(_, tensor)| tensor.id));
            retained.extend([layer.k_cache.id, layer.v_cache.id]);
        }
        let mut source_bytes = weights;
        Self::prepare_sources(
            &mut inner.transports[0],
            &retained,
            binding,
            &sources,
            &mut source_bytes,
        )
    }

    pub(super) fn prepare_sources<R: EngineeringTpRankTransportV1>(
        transport: &mut R,
        retained: &BTreeSet<u64>,
        binding: PackedDownBindingR1,
        sources: &[WeightSource],
        source_bytes: &mut impl WeightSourceBytes,
    ) -> TpResult<Self> {
        let originals = source_roster(sources)?;
        let mut packed = BTreeMap::new();
        let mut allocations = BTreeSet::new();
        let mut bytes = 0_u64;
        for source in sources {
            // Authenticate before allocating; only one 96-MiB staging matrix is live.
            let storage = source_bytes.with_source(source, |bytes| {
                pack_authenticated_rows(bytes, source.sha256, N)
            })?;
            let tensor = allocate_tensor(transport, WEIGHT_WORDS, 4)?;
            if originals.contains(&tensor.id)
                || retained.contains(&tensor.id)
                || !allocations.insert(tensor.id)
            {
                return Err("packed down allocator repeated a live identity".into());
            }
            for (chunk, payload) in storage.chunks(super::super::UPLOAD_CHUNK_BYTES).enumerate() {
                transport.write(tensor.id, chunk * super::super::UPLOAD_CHUNK_BYTES, payload)?;
            }
            bytes = bytes
                .checked_add(storage.len() as u64)
                .ok_or("packed down weight byte overflow")?;
            packed.insert(
                source.layer,
                PackedWeight {
                    original: source.original,
                    packed: tensor,
                    source_sha256: source.sha256,
                },
            );
        }
        if packed.len() != LAYERS || bytes != WEIGHT_BYTES {
            return Err("packed down packed roster is incomplete".into());
        }
        let scratch = allocate_tensor(transport, PACK_WORDS, 4)?;
        if originals.contains(&scratch.id)
            || retained.contains(&scratch.id)
            || allocations.contains(&scratch.id)
        {
            return Err("packed down activation scratch aliases retained storage".into());
        }
        Ok(Self {
            binding,
            weights: packed,
            scratch,
            selected: None,
            bytes,
            batch: None,
        })
    }

    pub(super) fn commands(
        &self,
        rank: &Rank,
        layer: u32,
    ) -> TpResult<[EngineeringTpDispatchV1; 2]> {
        if self.selected != Some(true)
            || rank.geometry.rank != 0
            || layer >= 36
            || rank.activation.element_bytes != 2
            || rank.activation.elements < K
            || rank.partial.element_bytes != 4
            || rank.partial.elements < N
            || self.scratch.element_bytes != 4
            || self.scratch.elements != PACK_WORDS
        {
            return Err("packed down active storage contract drifted".into());
        }
        let weight = self
            .weights
            .get(&layer)
            .ok_or("packed down layer missing")?;
        let original = rank.layers[layer as usize].weight(Qwen3TensorKind::DownProjection);
        if (original.id, original.elements, original.element_bytes)
            != (weight.original.id, N * K, 2)
            || (weight.original.elements, weight.original.element_bytes) != (N * K, 2)
            || weight.packed.elements != WEIGHT_WORDS
            || weight.packed.element_bytes != 4
        {
            return Err("packed down authenticated weight binding drifted".into());
        }
        let ids = [
            rank.activation.id,
            rank.partial.id,
            self.scratch.id,
            original.id,
            weight.packed.id,
        ];
        if ids
            .iter()
            .enumerate()
            .any(|(index, id)| ids[..index].contains(id))
        {
            return Err("packed down activation, weights or output alias".into());
        }
        let pack = dispatch(
            ROOTS[1],
            96,
            vec![
                Tensor {
                    elements: K,
                    ..rank.activation
                }
                .read(),
                self.scratch.write(),
                EngineeringTpArgumentV1::U32(1),
                EngineeringTpArgumentV1::U32(12_288),
            ],
        );
        let mut project = super::matrix(
            ROOTS[0],
            self.scratch,
            weight.packed,
            Tensor {
                elements: N,
                ..rank.partial
            },
            [1, 4096, 12_288, 1, 2],
        );
        project.grid_workgroups = 4096;
        let commands = [pack, project];
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
        return Err("packed down source digest or extent drifted".into());
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
        return Err("packed down row is not K12288".into());
    }
    for group in 0..96 {
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
    /// Admits the ninth image and packed storage in both arms, before explicit selection.
    /// # Errors
    /// Rejects unsupported images/storage and closes owned transports on setup failure.
    #[allow(clippy::too_many_arguments)]
    pub fn new_wide32_with_ordered64_packed_down_r1(
        mut transports: Vec<R>,
        model: ModelConfig,
        weights: &[u8],
        layout: &AuthenticatedModelWeightLayout,
        pool: &EngineeringTpPagedPoolV1,
        artifacts: &EngineeringTpWaveTargetArtifactsV17<'_>,
        prefill: &EngineeringTpArtifactV1,
        split: &EngineeringTpArtifactV1,
        gemv: &EngineeringTpArtifactV1,
        packed: &EngineeringTpPackedDownArtifactR1,
    ) -> TpResult<Self> {
        let binding = packed.binding();
        if let Err(error) = admit(&mut transports, binding) {
            return Err(close_admission(&mut transports, error));
        }
        let mut driver = Self::new_wide32_with_prefill_decode_gemv_v28(
            transports, model, weights, layout, pool, artifacts, prefill, split, gemv,
        )?;
        let mut extra_retained = driver
            .positions
            .iter()
            .chain(&driver.page_tables)
            .map(|tensor| tensor.id)
            .collect::<Vec<_>>();
        extra_retained.extend(driver.fp32_logits.map(|tensor| tensor.id));
        if let Some(split) = driver.split_attention_workspace_v25 {
            extra_retained.extend([split.stats.id, split.numerators.id]);
        }
        let prepared =
            Workspace::prepare(&mut driver.inner, weights, layout, binding, &extra_retained);
        driver.finish_packed_down_setup(prepared)?;
        Ok(driver)
    }

    pub(super) fn finish_packed_down_setup(
        &mut self,
        prepared: TpResult<Workspace>,
    ) -> TpResult<()> {
        match prepared {
            Ok(workspace) => self.packed_down_r1 = Some(workspace),
            Err(error) => {
                self.poisoned = true;
                return Err(match self.close() {
                    Ok(()) => error,
                    Err(close) => format!("{error}; packed down setup close: {close}"),
                });
            }
        }
        Ok(())
    }

    /// Selects only the fresh ordinary V27/split8/baseline-GEMV ordered64 composition.
    /// # Errors
    /// Rejects repeated/late selection, unsupported profiles or foreign image identities.
    pub fn configure_ordered64_packed_down_r1(
        &mut self,
        artifact: &EngineeringTpPackedDownArtifactR1,
        enabled: bool,
    ) -> TpResult<()> {
        self.configure_packed_down_binding_r1(artifact.binding(), enabled)
    }

    pub(super) fn configure_packed_down_binding_r1(
        &mut self,
        binding: PackedDownBindingR1,
        enabled: bool,
    ) -> TpResult<()> {
        if !cfg!(all(
            feature = "c1-ordered64",
            not(feature = "model-timestamps"),
            not(feature = "c1-token-program")
        )) || self.last_batch != 0
            || self.completed_batches != 0
            || self.poisoned
            || self.inner.closed
            || self.row_capacity != 32
            || self.inner.row_capacity != 32
            || self.inner.plan.world_size() != 1
            || self.inner.plan.model().role != Qwen3ModelRole::Target8B
            || self.inner.plan.model().layers != 36
            || self.inner.plan.model().hidden_size != 4096
            || self.inner.ranks.len() != 1
            || self.inner.transports.len() != 1
            || self.inner.transports[0].peer_group_rank().is_some()
            || self.inner.transports[0].supports_token_program()
            || self.inner.sequences.is_some()
            || self.inner.large_kv
            || self.inner.draft_v10
            || self.numerical.is_some()
            || self.inner.timing.is_enabled()
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
            || self.prefill16_ordered_v1.is_some()
            || self.packed_gate_up_r2.is_some()
            || self.c1_kv_copy_v19.is_some()
            || self.admitted_c1_kv_copy_v19.is_some()
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
            || self.packed_down_r1.as_ref().is_none_or(|workspace| {
                workspace.binding != binding
                    || workspace.selected.is_some()
                    || workspace.batch.is_some()
                    || workspace.bytes != WEIGHT_BYTES
                    || workspace.weights.len() != LAYERS
                    || workspace.scratch.elements != PACK_WORDS
                    || workspace.scratch.element_bytes != 4
            })
        {
            return Err("packed down requires its fresh ordinary V27/split8/baseline-GEMV ordered64 TP1 composition".into());
        }
        self.packed_down_r1
            .as_mut()
            .expect("validated packed down workspace")
            .selected = Some(enabled);
        Ok(())
    }

    /// Binds scheduler phase to the exact immutable pool batch before packet preflight.
    /// Ordinary constructors do nothing; both experimental arms require this binding.
    /// # Errors
    /// Rejects stale/duplicate bindings, mixed requests or mismatched prepared rows.
    /// # Panics
    /// Panics if the previously checked packed workspace is absent after validation.
    pub fn bind_dispatch_rows(
        &mut self,
        batch: &EngineeringTpPreparedBatchV1,
        rows: &[TpBatchRowV1],
    ) -> TpResult<()> {
        self.bind_splitk_rows(batch, rows)?;
        if self.packed_down_r1.is_none() {
            return Ok(());
        }
        if self
            .packed_down_r1
            .as_mut()
            .and_then(|workspace| workspace.batch.take())
            .is_some()
        {
            return Err("packed down duplicate binding abandoned its prior selection".into());
        }
        self.validate(batch)?;
        let workspace = self.packed_down_r1.as_mut().expect("checked workspace");
        if workspace.selected.is_none()
            || rows.is_empty()
            || rows.len() != batch.rows().len()
            || rows.iter().any(|row| row.request != rows[0].request)
            || batch
                .rows()
                .iter()
                .any(|row| row.sequence() != batch.rows()[0].sequence())
            || rows.iter().zip(batch.rows()).any(|(scheduled, prepared)| {
                scheduled.token_id != prepared.token()
                    || scheduled.absolute_position != prepared.position()
            })
            || (rows.iter().any(|row| row.kind == TpBatchRowKindV1::Decode) && rows.len() != 1)
        {
            return Err("packed down requires one exact scheduler request/phase binding".into());
        }
        workspace.batch = Some(BatchBinding {
            pool: batch.pool_identity(),
            batch: batch.id(),
            rows: rows
                .iter()
                .map(|row| (row.token_id, row.absolute_position, row.kind))
                .collect(),
            published: rows
                .iter()
                .filter(|row| row.kind != TpBatchRowKindV1::PrefillIntermediate)
                .count(),
        });
        Ok(())
    }

    /// Drops only the unsubmitted experimental binding after scheduler preflight rollback.
    pub fn abandon_dispatch_rows(&mut self) {
        self.abandon_splitk_rows();
        if let Some(workspace) = &mut self.packed_down_r1 {
            workspace.batch = None;
        }
    }

    pub(super) fn packed_down_active_for_batch(
        &self,
        batch: &EngineeringTpPreparedBatchV1,
        published: usize,
    ) -> TpResult<bool> {
        let Some(workspace) = &self.packed_down_r1 else {
            return Ok(false);
        };
        let active = workspace
            .batch
            .as_ref()
            .ok_or("packed down requires a scheduler binding")?
            .validate(batch, published)?;
        Ok(workspace.selected == Some(true) && active)
    }

    pub(super) fn take_packed_down_selection(
        &mut self,
        batch: &EngineeringTpPreparedBatchV1,
        outputs: &[usize],
    ) -> TpResult<bool> {
        let Some(workspace) = &mut self.packed_down_r1 else {
            return Ok(false);
        };
        let binding = workspace
            .batch
            .take()
            .ok_or("packed down requires a fresh scheduler binding")?;
        let active = binding.validate(batch, outputs.len())?;
        let expected = binding
            .rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| {
                (row.2 != TpBatchRowKindV1::PrefillIntermediate).then_some(index)
            })
            .collect::<Vec<_>>();
        if outputs != expected {
            return Err("packed down output selection differs from scheduler phase".into());
        }
        Ok(workspace.selected == Some(true) && active)
    }

    /// Actual explicit route; ordinary engines stay baseline.
    #[must_use]
    pub fn packed_down_mode(&self) -> &'static str {
        match self
            .packed_down_r1
            .as_ref()
            .map(|workspace| workspace.selected)
        {
            Some(Some(true)) => "packed-down-u32-r1",
            Some(None) => "unconfigured-packed-down-r1",
            _ => "baseline",
        }
    }
    /// Additional immutable packed weight bytes in either experimental arm.
    #[must_use]
    pub fn packed_down_weight_bytes(&self) -> u64 {
        self.packed_down_r1
            .as_ref()
            .map_or(0, |workspace| workspace.bytes)
    }
    /// Additional activation scratch bytes, excluding weights.
    #[must_use]
    pub fn packed_down_activation_scratch_bytes(&self) -> u64 {
        self.packed_down_r1.as_ref().map_or(0, |workspace| {
            (workspace.scratch.elements as u64) * u64::from(workspace.scratch.element_bytes)
        })
    }
    /// Original authenticated tensor digests in layer/role order; role 2 is down.
    #[must_use]
    pub fn packed_down_source_identities(&self) -> Vec<(u32, u32, [u8; 32])> {
        self.packed_down_r1
            .as_ref()
            .map_or_else(Vec::new, |workspace| {
                workspace
                    .weights
                    .iter()
                    .map(|(&layer, weight)| (layer, 2, weight.source_sha256))
                    .collect()
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sources() -> Vec<WeightSource> {
        (0..36)
            .map(|layer| WeightSource {
                layer,
                original: Tensor {
                    id: u64::from(layer) + 100,
                    elements: N * K,
                    element_bytes: 2,
                },
                rows: (0, 4096),
                columns: (0, 12_288),
                range: (u64::from(layer) * (N * K * 2) as u64, (N * K * 2) as u64),
                sha256: [1; 32],
            })
            .collect()
    }

    #[test]
    fn packed_down_roster_preflight_requires_36_distinct_full_shape_nonoverlapping_sources() {
        let good = sources();
        assert_eq!(source_roster(&good).unwrap().len(), 36);
        for mutation in 0..12 {
            let mut bad = good.clone();
            match mutation {
                0 => {
                    bad.pop();
                }
                1 => bad.push(good[0]),
                2 => bad[1].layer = 0,
                3 => bad[1].original.id = bad[0].original.id,
                4 => bad[0].rows = (1, 4096),
                5 => bad[0].columns = (0, 4096),
                6 => bad[0].original.element_bytes = 4,
                7 => bad[0].original.elements -= 1,
                8 => bad[0].range.1 -= 2,
                9 => bad[1].range.0 -= 2,
                10 => bad[0].range.0 = u64::MAX,
                _ => bad[0].layer = 36,
            }
            assert!(source_roster(&bad).is_err(), "mutation {mutation}");
        }
    }

    #[test]
    fn packed_down_host_layout_preserves_every_bf16_payload_and_tail() {
        let source = (0..K)
            .flat_map(|index| u16::try_from(index).unwrap().to_le_bytes())
            .collect::<Vec<_>>();
        let mut packed = vec![0; K * 2];
        pack_row(&source, &mut packed).unwrap();
        for group in 0..96 {
            for lane in 0..64 {
                let word = (group * 64 + lane) * 4;
                let low = (group * 128 + lane) * 2;
                assert_eq!(&packed[word..word + 2], &source[low..low + 2]);
                assert_eq!(&packed[word + 2..word + 4], &source[low + 128..low + 130]);
            }
        }
        for first in (0..65_536_u32).step_by(K) {
            let source = (first..first + 12_288)
                .flat_map(|value| u16::try_from(value & 0xffff).unwrap().to_le_bytes())
                .collect::<Vec<_>>();
            pack_row(&source, &mut packed).unwrap();
            for group in 0..96 {
                for lane in 0..64 {
                    let low = (group * 128 + lane) * 2;
                    let word = (group * 64 + lane) * 4;
                    assert_eq!(&packed[word..word + 2], &source[low..low + 2]);
                    assert_eq!(&packed[word + 2..word + 4], &source[low + 128..low + 130]);
                }
            }
        }
    }

    #[test]
    fn packed_down_host_authenticates_before_conversion() {
        let source = vec![0x3f; K * 4];
        let digest: [u8; 32] = Sha256::digest(&source).into();
        assert_eq!(
            pack_authenticated_rows(&source, digest, 2).unwrap().len(),
            source.len()
        );
        let mut wrong = digest;
        wrong[0] ^= 1;
        assert!(pack_authenticated_rows(&source, wrong, 2).is_err());
        for rows in [0, 1, N + 1] {
            assert!(pack_authenticated_rows(&source, digest, rows).is_err());
        }
        assert!(pack_row(&source[..K * 2 - 2], &mut vec![0; K * 2]).is_err());
    }
}
