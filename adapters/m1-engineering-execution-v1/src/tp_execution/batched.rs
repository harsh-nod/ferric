//! Resident, multi-row Qwen3 execution over a separately admitted paged image.
//!
//! Every layer executes one GPU batch across all ranks. Page metadata comes
//! from the transactional pool; intermediate prompt rows remain causal but do
//! not publish output tokens. Arithmetic and transport remain Contracted.

use super::numerical::{
    EngineeringTpNumericalCaptureV1, EngineeringTpNumericalProjectionV1 as NumericalRole,
};
use super::{
    EngineeringTpArgumentV1, EngineeringTpDispatchV1, EngineeringTpExecutionV1,
    EngineeringTpRankTransportV1, EngineeringTpReductionModeV3, Qwen3TensorKind,
    Qwen3TensorParallelCollectiveV1, Rank, Tensor, TpResult, allocate_tensor, dispatch, rmsnorm,
    rope_bytes,
};
use crate::tp_paged::{
    EngineeringTpBatchCompletionV1, EngineeringTpPagedPoolV1, EngineeringTpPoolScopeV1,
    EngineeringTpPreparedBatchV1,
};
use ferric_build::AuthenticatedModelWeightLayout;
use ferric_spec::{ModelConfig, Qwen3ModelRole};

const MAX_ROWS: usize = 16;
const PAGE_TOKENS: u32 = 16;
const EMBEDDING: &str = "ferric_qwen3_tp_batch_embedding_bf16_v2";
const GEMM: &str = "ferric_qwen3_tp_batch_gemm_bf16_f32_bf16_v2";
pub(super) const PARTIAL: &str = "ferric_qwen3_tp_batch_gemm_partial_bf16_f32_v2";
const SWIGLU: &str = "ferric_qwen3_tp_batch_swiglu_bf16_f32_v2";
const ROPE: &str = "ferric_qwen3_tp_batch_rope_v2";
const APPEND: &str = "ferric_qwen3_tp_batch_paged_kv_append_v2";
const ATTENTION: &str = "ferric_qwen3_tp_batch_paged_gqa_bf16_f32_v2";
const ARGMAX: &str = "ferric_qwen3_tp_batch_argmax_bf16_v2";

/// Successful all-layer/all-rank work, still awaiting pool and scheduler commit.
pub struct EngineeringTpBatchOutputV2 {
    /// Greedy choices in the order of the explicitly requested output rows.
    pub choices: Vec<u32>,
    /// Pool-specific completion token created only after successful rank completion.
    pub completion: EngineeringTpBatchCompletionV1,
}

/// One resident weight set and physical KV pool for a bounded request stream.
// Lifecycle state and independent, explicitly selected ablations are orthogonal.
#[allow(clippy::struct_excessive_bools)]
pub struct EngineeringTpBatchExecutionV2<R: EngineeringTpRankTransportV1> {
    inner: EngineeringTpExecutionV1<R>,
    row_capacity: usize,
    positions: Vec<Tensor>,
    page_tables: Vec<Tensor>,
    scope: EngineeringTpPoolScopeV1,
    pool_identity: u64,
    context_tokens: u32,
    physical_pages: u32,
    table_stride: u32,
    last_batch: u64,
    completed_batches: u64,
    poisoned: bool,
    prune_output_head: bool,
    projection: super::projection::ProjectionPolicy,
    projection_configured: bool,
    wave_attention: bool,
    numerical: Option<Box<EngineeringTpNumericalCaptureV1>>,
}

impl<R: EngineeringTpRankTransportV1> EngineeringTpBatchExecutionV2<R> {
    /// Uploads weights once and allocates shared physical pages and sixteen-row workspaces.
    ///
    /// The page pool must be fresh: old initialized metadata cannot be attached
    /// to newly allocated GPU memory. Every transport must admit the v2 image.
    ///
    /// # Errors
    /// Rejects unsupported model/storage, a nonempty pool, or any transport failure.
    pub fn new(
        transports: Vec<R>,
        model: ModelConfig,
        weights: &[u8],
        layout: &AuthenticatedModelWeightLayout,
        pool: &EngineeringTpPagedPoolV1,
    ) -> TpResult<Self> {
        Self::new_bounded(transports, model, weights, layout, pool, MAX_ROWS)
    }

    /// Allocates genuine 32-row workspaces for transports admitting the separate v5 image.
    /// # Errors
    /// Rejects incompatible model/pool geometry or any transport failure.
    pub fn new_wide32(
        transports: Vec<R>,
        model: ModelConfig,
        weights: &[u8],
        layout: &AuthenticatedModelWeightLayout,
        pool: &EngineeringTpPagedPoolV1,
    ) -> TpResult<Self> {
        Self::new_bounded(transports, model, weights, layout, pool, 32)
    }

    fn new_bounded(
        mut transports: Vec<R>,
        model: ModelConfig,
        weights: &[u8],
        layout: &AuthenticatedModelWeightLayout,
        pool: &EngineeringTpPagedPoolV1,
        row_capacity: usize,
    ) -> TpResult<Self> {
        if model.role != Qwen3ModelRole::Target8B
            || !pool.is_empty()
            || pool.row_capacity() != row_capacity
        {
            for transport in &mut transports {
                let _ = transport.close();
            }
            return Err("batched execution requires Qwen3-8B and a fresh page pool".into());
        }
        let limits = pool.limits();
        let physical_pages = limits.physical_page_count();
        let context_tokens = limits.context_tokens();
        let table_stride = context_tokens.div_ceil(PAGE_TOKENS);
        let mut inner = EngineeringTpExecutionV1::new_with_storage(
            transports,
            model,
            weights,
            layout,
            physical_pages * PAGE_TOKENS,
            u32::try_from(row_capacity).map_err(|_| "row limit conversion")?,
        )?;
        let metadata = (|| {
            let mut positions = Vec::new();
            let mut page_tables = Vec::new();
            for transport in &mut inner.transports {
                positions.push(allocate_tensor(transport, row_capacity, 4)?);
                page_tables.push(allocate_tensor(
                    transport,
                    row_capacity * table_stride as usize,
                    4,
                )?);
            }
            Ok::<_, String>((positions, page_tables))
        })();
        let (positions, page_tables) = match metadata {
            Ok(value) => value,
            Err(error) => {
                let close = inner.close();
                return Err(match close {
                    Ok(()) => error,
                    Err(close) => format!("{error}; close: {close}"),
                });
            }
        };
        Ok(Self {
            inner,
            row_capacity,
            positions,
            page_tables,
            scope: pool.scope(),
            pool_identity: pool.identity(),
            context_tokens,
            physical_pages,
            table_stride,
            last_batch: 0,
            completed_batches: 0,
            poisoned: false,
            prune_output_head: false,
            projection: super::projection::ProjectionPolicy::default(),
            projection_configured: false,
            wave_attention: false,
            numerical: None,
        })
    }

    /// Maximum physical rows supported by this allocation and kernel profile.
    #[must_use]
    pub const fn row_capacity(&self) -> usize {
        self.row_capacity
    }

    /// Enables bounded diagnostic readbacks on a fresh, nonsequenced TP1 stream.
    /// # Errors
    /// Rejects unsupported geometry, lifecycle, or repeated configuration.
    pub fn configure_numerical_capture(
        &mut self,
        capture: EngineeringTpNumericalCaptureV1,
    ) -> TpResult<()> {
        if self.last_batch != 0
            || self.poisoned
            || self.inner.closed
            || self.numerical.is_some()
            || self.row_capacity != 16
            || self.inner.ranks.len() != 1
            || self.inner.sequences.is_some()
            || self.inner.timing.is_enabled()
            || self.wave_attention
            || !capture.matches_profile(self.projection.mode.label(), self.prune_output_head)
        {
            return Err(
                "numerical capture requires a fresh TP1/16-row nonsequenced diagnostic stream"
                    .into(),
            );
        }
        self.numerical = Some(Box::new(capture));
        Ok(())
    }

    /// Copies scheduler identities only for the explicitly selected diagnostic batch.
    /// # Errors
    /// Rejects stale or repeated selected bindings.
    pub fn bind_numerical_rows(
        &mut self,
        scheduler_batch: u64,
        rows: &[crate::tp_scheduler::TpBatchRowV1],
    ) -> TpResult<()> {
        if let Some(capture) = &mut self.numerical {
            capture.bind_rows(
                self.completed_batches
                    .checked_add(1)
                    .ok_or("numerical ordinal overflow")?,
                scheduler_batch,
                rows,
            )?;
        }
        Ok(())
    }

    /// Finalizes complete diagnostics only after successful owned-worker close.
    /// # Errors
    /// Rejects failed, incomplete, unclosed, or repeated captures.
    pub fn finish_numerical_capture(&mut self) -> TpResult<Option<serde_json::Value>> {
        self.numerical
            .as_mut()
            .map(|capture| {
                if !self.inner.closed || self.poisoned {
                    return Err("numerical finish requires a clean closed driver".into());
                }
                capture.finish()
            })
            .transpose()
    }

    /// Attaches opt-in host diagnostics before execution, without altering GPU policy.
    /// # Errors
    /// Rejects a started, closed or failed stream.
    pub fn configure_host_timing(
        &mut self,
        timing: crate::host_timing::HostTiming,
    ) -> TpResult<()> {
        if self.last_batch != 0
            || self.poisoned
            || self.inner.closed
            || (self.numerical.is_some() && timing.is_enabled())
        {
            return Err("host timing requires a fresh batch stream".into());
        }
        self.inner.timing = timing;
        Ok(())
    }

    /// Selects an explicit reduction ablation before the first batch.
    /// `DeviceTp1V3` requires a transport admitting the separate v3 image.
    ///
    /// # Errors
    /// Rejects a started/poisoned stream, unsupported profile, or allocation failure.
    pub fn configure_reduction(&mut self, mode: EngineeringTpReductionModeV3) -> TpResult<()> {
        if self.poisoned
            || self.last_batch != 0
            || self.completed_batches != 0
            || self.numerical.is_some()
        {
            return Err("reduction mode requires a fresh batch stream".into());
        }
        self.inner.configure_reduction(mode)
    }

    /// Active reduction profile, for identity-bound measurement receipts.
    #[must_use]
    pub const fn reduction_mode(&self) -> EngineeringTpReductionModeV3 {
        self.inner.reduction.mode()
    }

    /// Executes a prepared batch once, without committing reusable page ownership.
    ///
    /// Call `pool.begin_submission` first. On any error after submission the
    /// caller must quarantine the pool and fail its scheduler, even if some
    /// ranks completed. On success commit the returned token before publication.
    ///
    /// # Errors
    /// Rejects stale/foreign metadata, exhausted packet budget, closed or failed state,
    /// invalid geometry, nonfinite reductions, and any incomplete GPU operation.
    pub fn execute(
        &mut self,
        batch: &EngineeringTpPreparedBatchV1,
    ) -> TpResult<EngineeringTpBatchOutputV2> {
        let output_rows = (0..batch.rows().len()).collect::<Vec<_>>();
        self.execute_selected(batch, &output_rows)
    }

    /// Enables output-head pruning before the first batch; baseline is unchanged by default.
    /// # Errors
    /// Rejects changes after submission, failure, or closure.
    pub fn configure_output_head_pruning(&mut self, enabled: bool) -> TpResult<()> {
        if self.last_batch != 0 || self.poisoned || self.inner.closed || self.numerical.is_some() {
            return Err("output-head policy must be configured before execution".into());
        }
        self.prune_output_head = enabled;
        Ok(())
    }

    /// Selects admitted projection roots and prepares transposed weights once.
    /// The caller must admit the exact corresponding v3 image before selection.
    /// # Errors
    /// Rejects changes after setup/execution, source drift, or allocation failure.
    pub fn configure_projection(
        &mut self,
        mode: super::EngineeringTpProjectionModeV3,
        weights: &[u8],
        layout: &AuthenticatedModelWeightLayout,
    ) -> TpResult<()> {
        if self.last_batch != 0
            || self.poisoned
            || self.inner.closed
            || self.projection_configured
            || self.numerical.is_some()
        {
            return Err("projection policy must be configured once before execution".into());
        }
        match super::projection::ProjectionPolicy::prepare(mode, &mut self.inner, weights, layout) {
            Ok(policy) => {
                self.projection = policy;
                self.projection_configured = true;
                Ok(())
            }
            Err(error) => {
                self.poisoned = true;
                Err(match self.inner.close() {
                    Ok(()) => error,
                    Err(close) => format!("{error}; projection setup close: {close}"),
                })
            }
        }
    }

    /// Extra resident bytes for the setup-only transposed layout.
    #[must_use]
    pub const fn transposed_weight_bytes(&self) -> u64 {
        self.projection.bytes
    }

    /// Resident base weight payload, excluding transposes, page rounding, and workspaces.
    /// # Errors
    /// Rejects inconsistent tensor extents or an overflowing byte count.
    pub fn resident_weight_bytes(&self) -> TpResult<u64> {
        let mut total = 0_u64;
        for rank in &self.inner.ranks {
            // Independent ranks may reuse the same transport-local buffer ID.
            let mut seen = std::collections::BTreeMap::new();
            for (_, tensor) in rank
                .globals
                .iter()
                .chain(rank.layers.iter().flat_map(|layer| &layer.weights))
            {
                let bytes = u64::try_from(tensor.elements)
                    .ok()
                    .and_then(|n| n.checked_mul(u64::from(tensor.element_bytes)))
                    .ok_or("base weight byte count overflow")?;
                if let Some(previous) = seen.insert(tensor.id, bytes) {
                    if previous != bytes {
                        return Err("inconsistent base weight buffer extent".into());
                    }
                    continue;
                }
                total = total
                    .checked_add(bytes)
                    .ok_or("base weight total overflow")?;
            }
        }
        Ok(total)
    }

    /// Selects the admitted wave-cooperative attention root before execution.
    /// # Errors
    /// Rejects a started, poisoned, or closed execution.
    pub fn configure_wave_attention(&mut self, enabled: bool) -> TpResult<()> {
        if self.last_batch != 0 || self.poisoned || self.inner.closed || self.numerical.is_some() {
            return Err("attention policy must be configured before execution".into());
        }
        self.wave_attention = enabled;
        Ok(())
    }

    /// Enables ordered IPC sequences between existing collective barriers.
    /// # Errors
    /// Rejects unsupported transports, execution already begun, or closed state.
    pub fn configure_dispatch_sequences(&mut self, enabled: bool) -> TpResult<()> {
        if self.last_batch != 0
            || (enabled && self.numerical.is_some())
            || self.poisoned
            || self.inner.closed
            || (enabled
                && self
                    .inner
                    .transports
                    .iter()
                    .any(|rank| !rank.supports_sequences()))
        {
            return Err("dispatch sequence policy requires fresh supported ranks".into());
        }
        self.inner.sequences = enabled.then(|| vec![Vec::new(); self.inner.ranks.len()]);
        Ok(())
    }

    /// Actual projected rows, distinct from all physical transformer/KV rows.
    #[must_use]
    pub const fn output_head_rows(&self, physical: usize, published: usize) -> usize {
        if self.prune_output_head {
            published
        } else {
            physical
        }
    }

    /// Expected per-rank dispatch increments under the frozen execution policy.
    #[must_use]
    pub fn expected_dispatch_counts(&self, published: usize) -> Vec<u64> {
        let head = if self.prune_output_head && published == 0 {
            0
        } else {
            3
        };
        let extra = u64::from(self.inner.plan.model().layers)
            * self.reduction_mode().extra_dispatches_per_layer();
        (0..self.inner.plan.world_size())
            .map(|rank| {
                540 + extra
                    + if rank == 0 { 1 + head } else { 0 }
                    + self.reduction_mode().extra_dispatches_per_forward(rank)
            })
            .collect()
    }

    /// Executes all physical rows but returns only the requested, ascending output rows.
    /// With pruning enabled these rows are stably moved to the front of the GPU
    /// batch, so existing row-prefix kernels need no gather or host hidden-state read.
    /// # Errors
    /// Rejects duplicate/out-of-range selections before any GPU submission.
    pub fn execute_selected(
        &mut self,
        batch: &EngineeringTpPreparedBatchV1,
        output_rows: &[usize],
    ) -> TpResult<EngineeringTpBatchOutputV2> {
        let _timing = self.inner.timing.batch(batch.id());
        self.validate(batch)?;
        if output_rows.iter().any(|&row| row >= batch.rows().len())
            || output_rows.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err("output rows must be unique, ascending, and within the batch".into());
        }
        let per_rank = u64::from(self.inner.plan.model().layers)
            * (15 + self.reduction_mode().extra_dispatches_per_layer())
            + 4;
        let next = self
            .completed_batches
            .checked_add(1)
            .ok_or("batch counter overflow")?;
        let rollover = self
            .inner
            .transports
            .iter()
            .all(EngineeringTpRankTransportV1::supports_queue_rollover);
        if !rollover
            && next.checked_mul(per_rank).is_none_or(|packets| {
                packets > fe2o3_kfd::engineering_wire::MAX_UNRETIRED_RING_PACKETS_V1
            })
        {
            return Err("batched stream exceeds the conservative no-ring-rollover budget".into());
        }
        for index in 0..self.inner.transports.len() {
            if let Err(error) = self.inner.transports[index].prepare_packets(per_rank) {
                self.poisoned = true;
                return Err(match self.inner.close() {
                    Ok(()) => error,
                    Err(close) => format!("{error}; queue preparation close: {close}"),
                });
            }
        }
        self.last_batch = batch.id();
        match self.forward(batch, output_rows) {
            Ok(choices) => {
                self.completed_batches = next;
                Ok(EngineeringTpBatchOutputV2 {
                    choices,
                    completion: EngineeringTpBatchCompletionV1::after_all_ranks(batch),
                })
            }
            Err(error) => {
                self.poisoned = true;
                Err(error)
            }
        }
    }

    /// Cumulative successfully completed kernel dispatches in logical rank order.
    #[must_use]
    pub fn dispatch_counts(&self) -> Vec<u64> {
        self.inner.dispatch_counts()
    }

    /// Number of complete GPU batches, not number of request tokens.
    #[must_use]
    pub const fn completed_batches(&self) -> u64 {
        self.completed_batches
    }

    /// Closes and reaps every rank, including after partial completion.
    /// # Errors
    /// Reports any teardown whose completion cannot be confirmed.
    pub fn close(&mut self) -> TpResult<()> {
        let result = self.inner.close();
        if result.is_err() {
            self.poisoned = true;
        }
        result
    }

    fn validate(&self, batch: &EngineeringTpPreparedBatchV1) -> TpResult<()> {
        if self.inner.closed || self.poisoned {
            return Err("batched execution is closed or poisoned".into());
        }
        if batch.pool_identity() != self.pool_identity
            || batch.scope() != self.scope
            || batch.context_tokens() != self.context_tokens
            || batch.physical_page_count() != self.physical_pages
            || batch.page_table_stride() != self.table_stride
            || batch.id() <= self.last_batch
            || batch.rows().is_empty()
            || batch.rows().len() > self.row_capacity
        {
            return Err("foreign, stale, or invalid prepared GPU batch".into());
        }
        let mut writes = std::collections::BTreeSet::new();
        for row in batch.rows() {
            let position = row.position();
            let pages = row.physical_pages();
            let logical = (position / PAGE_TOKENS) as usize;
            if position >= self.context_tokens
                || row.token() >= self.inner.plan.model().vocabulary_size
                || pages.len() > self.table_stride as usize
                || logical >= pages.len()
                || pages.iter().any(|&page| page >= self.physical_pages)
                || pages
                    .iter()
                    .copied()
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    != pages.len()
                || !writes.insert((pages[logical], position % PAGE_TOKENS))
                || pages[logical] != row.writable_physical_page()
                || position % PAGE_TOKENS != row.writable_token_offset()
            {
                return Err("invalid paged row geometry or aliased write slot".into());
            }
            for peer in batch.rows() {
                for (peer_logical, &page) in peer.physical_pages().iter().enumerate() {
                    if page == pages[logical]
                        && (peer.sequence() != row.sequence() || peer_logical != logical)
                    {
                        return Err(
                            "batch writes a page shared with another sequence or logical position"
                                .into(),
                        );
                    }
                }
            }
        }
        Ok(())
    }

    fn forward(
        &mut self,
        batch: &EngineeringTpPreparedBatchV1,
        output_rows: &[usize],
    ) -> TpResult<Vec<u32>> {
        use EngineeringTpArgumentV1::U32;
        let metadata_timing = self.inner.timing.scope("metadata");
        let model = self.inner.plan.model();
        let world = self.inner.plan.world_size();
        let projection = &self.projection;
        let attention = if self.wave_attention {
            "ferric_qwen3_tp_wave_paged_gqa_bf16_v3"
        } else {
            ATTENTION
        };
        let rows = u32::try_from(batch.rows().len()).map_err(|_| "batch row conversion")?;
        let head_rows = u32::try_from(self.output_head_rows(batch.rows().len(), output_rows.len()))
            .map_err(|_| "output row conversion")?;
        let mut execution_order = Vec::with_capacity(batch.rows().len());
        if self.prune_output_head {
            execution_order.extend_from_slice(output_rows);
        }
        execution_order.extend(
            (0..batch.rows().len())
                .filter(|row| !self.prune_output_head || output_rows.binary_search(row).is_err()),
        );
        let ordinal = self.completed_batches + 1;
        if let Some(capture) = &mut self.numerical {
            capture.begin_batch(ordinal, batch, &execution_order, output_rows)?;
        }
        let expected = self.inner.collective.expected();
        if expected.epoch != self.completed_batches
            || expected.layer != 0
            || expected.operation != Qwen3TensorParallelCollectiveV1::AttentionOutputSum
        {
            return Err("batch collective cursor drifted".into());
        }
        self.inner.hidden = vec![0; rows as usize * model.hidden_size as usize];
        let mut tokens = Vec::with_capacity(rows as usize * 4);
        let mut positions = Vec::with_capacity(rows as usize * 4);
        let mut table = vec![u32::MAX; rows as usize * self.table_stride as usize];
        let mut cos = Vec::with_capacity(rows as usize * 256);
        let mut sin = Vec::with_capacity(rows as usize * 256);
        let mut max_context = 0;
        for (index, &source_row) in execution_order.iter().enumerate() {
            let row = &batch.rows()[source_row];
            tokens.extend_from_slice(&row.token().to_le_bytes());
            positions.extend_from_slice(&row.position().to_le_bytes());
            let start = index * self.table_stride as usize;
            table[start..start + row.physical_pages().len()].copy_from_slice(row.physical_pages());
            let (row_cos, row_sin) = rope_bytes(row.position(), model.rope_theta);
            cos.extend_from_slice(&row_cos);
            sin.extend_from_slice(&row_sin);
            max_context = max_context.max(row.position() + 1);
        }
        let table = table
            .into_iter()
            .flat_map(u32::to_le_bytes)
            .collect::<Vec<_>>();
        for (index, (rank, transport)) in self
            .inner
            .ranks
            .iter()
            .zip(&mut self.inner.transports)
            .enumerate()
        {
            transport.write(self.positions[index].id, 0, &positions)?;
            transport.write(self.page_tables[index].id, 0, &table)?;
            transport.write(rank.cos.id, 0, &cos)?;
            transport.write(rank.sin.id, 0, &sin)?;
        }
        let zero = &self.inner.ranks[0];
        self.inner.transports[0].write(zero.token.id, 0, &tokens)?;
        drop(metadata_timing);
        let embedding_timing = self.inner.timing.scope("embedding");
        self.inner.dispatch_zero(&dispatch(
            EMBEDDING,
            rows * model.hidden_size / 64,
            vec![
                zero.token.read(),
                zero.global(Qwen3TensorKind::TokenEmbedding).read(),
                zero.hidden.write(),
                U32(rows),
            ],
        ))?;
        self.inner.initialize_hidden_from_embedding()?;
        drop(embedding_timing);

        for layer in 0..model.layers {
            let attention_timing = self.inner.timing.scope("attention");
            let li = layer as usize;
            self.inner.dispatch_each(|r| {
                norm(
                    r,
                    r.hidden,
                    r.layers[li].weight(Qwen3TensorKind::InputLayerNorm),
                    r.normalized,
                    rows,
                    model.hidden_size,
                )
            })?;
            for (kind, tag) in [
                (Qwen3TensorKind::QueryProjection, 1),
                (Qwen3TensorKind::KeyProjection, 2),
                (Qwen3TensorKind::ValueProjection, 3),
            ] {
                self.inner.dispatch_each(|r| {
                    let (output, n) = if tag == 1 {
                        (r.q, r.geometry.query_channels.count)
                    } else if tag == 2 {
                        (r.k, r.geometry.kv_channels.count)
                    } else {
                        (r.v, r.geometry.kv_channels.count)
                    };
                    projection.command(
                        r.geometry.rank as usize,
                        GEMM,
                        r.normalized,
                        r.layers[li].weight(kind),
                        output,
                        [rows, n, model.hidden_size, world, tag],
                    )
                })?;
                capture_projection(
                    &mut self.numerical,
                    &mut self.inner,
                    projection,
                    ordinal,
                    layer,
                    if tag == 1 {
                        NumericalRole::Query
                    } else if tag == 2 {
                        NumericalRole::Key
                    } else {
                        NumericalRole::Value
                    },
                    rows,
                )?;
            }
            self.inner.dispatch_each(|r| {
                norm(
                    r,
                    r.q,
                    r.layers[li].weight(Qwen3TensorKind::QueryNorm),
                    r.q_normalized,
                    rows * r.geometry.query_heads.count,
                    128,
                )
            })?;
            self.inner.dispatch_each(|r| {
                norm(
                    r,
                    r.k,
                    r.layers[li].weight(Qwen3TensorKind::KeyNorm),
                    r.k_normalized,
                    rows * r.geometry.kv_heads.count,
                    128,
                )
            })?;
            let positions = &self.positions;
            let tables = &self.page_tables;
            let stride = self.table_stride;
            let pages = self.physical_pages;
            self.inner.dispatch_each(|r| {
                dispatch(
                    ROPE,
                    rows,
                    vec![
                        r.q_normalized.read(),
                        r.k_normalized.read(),
                        r.cos.read(),
                        r.sin.read(),
                        positions[r.geometry.rank as usize].read(),
                        r.q_rotated.write(),
                        r.k_rotated.write(),
                        U32(rows),
                        U32(world),
                    ],
                )
            })?;
            self.inner.dispatch_each(|r| {
                dispatch(
                    APPEND,
                    1,
                    vec![
                        r.k_rotated.read(),
                        r.v.read(),
                        positions[r.geometry.rank as usize].read(),
                        tables[r.geometry.rank as usize].read(),
                        r.layers[li].k_cache.write(),
                        r.layers[li].v_cache.write(),
                        U32(rows),
                        U32(world),
                        U32(stride),
                        U32(pages),
                    ],
                )
            })?;
            self.inner.dispatch_each(|r| {
                dispatch(
                    attention,
                    rows * r.geometry.query_heads.count,
                    vec![
                        r.q_rotated.read(),
                        r.layers[li].k_cache.read(),
                        r.layers[li].v_cache.read(),
                        positions[r.geometry.rank as usize].read(),
                        tables[r.geometry.rank as usize].read(),
                        r.attention.write(),
                        U32(rows),
                        U32(world),
                        U32(stride),
                        U32(pages),
                        U32(max_context),
                    ],
                )
            })?;
            self.inner.dispatch_each(|r| {
                projection.command(
                    r.geometry.rank as usize,
                    PARTIAL,
                    r.attention,
                    r.layers[li].weight(Qwen3TensorKind::OutputProjection),
                    r.partial,
                    [
                        rows,
                        model.hidden_size,
                        r.geometry.query_channels.count,
                        world,
                        1,
                    ],
                )
            })?;
            drop(attention_timing);
            capture_projection(
                &mut self.numerical,
                &mut self.inner,
                projection,
                ordinal,
                layer,
                NumericalRole::AttentionOutput,
                rows,
            )?;
            self.inner
                .reduce(layer, Qwen3TensorParallelCollectiveV1::AttentionOutputSum)?;
            let feed_forward_timing = self.inner.timing.scope("feed_forward");
            self.inner.dispatch_each(|r| {
                norm(
                    r,
                    r.hidden,
                    r.layers[li].weight(Qwen3TensorKind::PostAttentionLayerNorm),
                    r.normalized,
                    rows,
                    model.hidden_size,
                )
            })?;
            for (kind, tag) in [
                (Qwen3TensorKind::GateProjection, 4),
                (Qwen3TensorKind::UpProjection, 5),
            ] {
                self.inner.dispatch_each(|r| {
                    projection.command(
                        r.geometry.rank as usize,
                        GEMM,
                        r.normalized,
                        r.layers[li].weight(kind),
                        if tag == 4 { r.gate } else { r.up },
                        [
                            rows,
                            r.geometry.intermediate.count,
                            model.hidden_size,
                            world,
                            tag,
                        ],
                    )
                })?;
                capture_projection(
                    &mut self.numerical,
                    &mut self.inner,
                    projection,
                    ordinal,
                    layer,
                    if tag == 4 {
                        NumericalRole::Gate
                    } else {
                        NumericalRole::Up
                    },
                    rows,
                )?;
            }
            self.inner.dispatch_each(|r| {
                dispatch(
                    SWIGLU,
                    rows * r.geometry.intermediate.count / 64,
                    vec![
                        r.gate.read(),
                        r.up.read(),
                        r.activation.write(),
                        U32(rows),
                        U32(world),
                    ],
                )
            })?;
            self.inner.dispatch_each(|r| {
                projection.command(
                    r.geometry.rank as usize,
                    PARTIAL,
                    r.activation,
                    r.layers[li].weight(Qwen3TensorKind::DownProjection),
                    r.partial,
                    [
                        rows,
                        model.hidden_size,
                        r.geometry.intermediate.count,
                        world,
                        2,
                    ],
                )
            })?;
            drop(feed_forward_timing);
            capture_projection(
                &mut self.numerical,
                &mut self.inner,
                projection,
                ordinal,
                layer,
                NumericalRole::Down,
                rows,
            )?;
            self.inner
                .reduce(layer, Qwen3TensorParallelCollectiveV1::FeedForwardDownSum)?;
        }
        let expected = self.inner.collective.expected();
        if expected.epoch != self.completed_batches + 1
            || expected.layer != 0
            || expected.operation != Qwen3TensorParallelCollectiveV1::AttentionOutputSum
        {
            return Err("batch completed before every layer collective".into());
        }
        if head_rows == 0 {
            return Ok(Vec::new());
        }
        let head_timing = self.inner.timing.scope("output_head");
        let r = &self.inner.ranks[0];
        self.inner.dispatch_zero(&norm(
            r,
            r.hidden,
            r.global(Qwen3TensorKind::FinalNorm),
            r.normalized,
            head_rows,
            model.hidden_size,
        ))?;
        let r = &self.inner.ranks[0];
        self.inner.dispatch_zero(&projection.command(
            0,
            GEMM,
            r.normalized,
            r.global(Qwen3TensorKind::LanguageModelHead),
            r.logits,
            [
                head_rows,
                model.vocabulary_size,
                model.hidden_size,
                world,
                6,
            ],
        ))?;
        let r = &self.inner.ranks[0];
        self.inner.dispatch_zero(&dispatch(
            ARGMAX,
            head_rows,
            vec![r.logits.read(), r.choice.write(), U32(head_rows)],
        ))?;
        drop(head_timing);
        let _readback_timing = self.inner.timing.scope("output_readback");
        let mut bytes = vec![0; head_rows as usize * 4];
        self.inner.transports[0].read(self.inner.ranks[0].choice.id, 0, &mut bytes)?;
        let choices = bytes
            .chunks_exact(4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect::<Vec<_>>();
        if choices.iter().any(|&token| token >= model.vocabulary_size) {
            return Err("batched GPU choice outside vocabulary".into());
        }
        if let Some(capture) = &mut self.numerical
            && capture.selects(ordinal)
        {
            let rank = &self.inner.ranks[0];
            let weight = rank.global(Qwen3TensorKind::LanguageModelHead);
            let command = projection.command(
                0,
                GEMM,
                rank.normalized,
                weight,
                rank.logits,
                [
                    head_rows,
                    model.vocabulary_size,
                    model.hidden_size,
                    world,
                    6,
                ],
            );
            capture.capture_head(
                ordinal,
                &command,
                weight.read(),
                &choices,
                &mut self.inner.transports[0],
            )?;
        }
        if self.prune_output_head {
            Ok(choices)
        } else {
            Ok(output_rows.iter().map(|&row| choices[row]).collect())
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn capture_projection<R: EngineeringTpRankTransportV1>(
    capture: &mut Option<Box<EngineeringTpNumericalCaptureV1>>,
    inner: &mut EngineeringTpExecutionV1<R>,
    policy: &super::projection::ProjectionPolicy,
    ordinal: u64,
    layer: u32,
    role: NumericalRole,
    rows: u32,
) -> TpResult<()> {
    let Some(capture) = capture
        .as_mut()
        .filter(|capture| capture.selects_projection(ordinal, layer, role))
    else {
        return Ok(());
    };
    let (command, original) =
        numerical_projection_command(&inner.ranks[0], policy, layer, role, rows);
    capture.capture_projection(
        ordinal,
        layer,
        role,
        &command,
        original,
        &mut inner.transports[0],
    )
}

fn numerical_projection_command(
    rank: &Rank,
    policy: &super::projection::ProjectionPolicy,
    layer: u32,
    role: NumericalRole,
    rows: u32,
) -> (EngineeringTpDispatchV1, EngineeringTpArgumentV1) {
    let (kind, input, output, n, k, tag, kernel) = match role {
        NumericalRole::Query => (
            Qwen3TensorKind::QueryProjection,
            rank.normalized,
            rank.q,
            4096,
            4096,
            1,
            GEMM,
        ),
        NumericalRole::Key => (
            Qwen3TensorKind::KeyProjection,
            rank.normalized,
            rank.k,
            1024,
            4096,
            2,
            GEMM,
        ),
        NumericalRole::Value => (
            Qwen3TensorKind::ValueProjection,
            rank.normalized,
            rank.v,
            1024,
            4096,
            3,
            GEMM,
        ),
        NumericalRole::AttentionOutput => (
            Qwen3TensorKind::OutputProjection,
            rank.attention,
            rank.partial,
            4096,
            4096,
            1,
            PARTIAL,
        ),
        NumericalRole::Gate => (
            Qwen3TensorKind::GateProjection,
            rank.normalized,
            rank.gate,
            12288,
            4096,
            4,
            GEMM,
        ),
        NumericalRole::Up => (
            Qwen3TensorKind::UpProjection,
            rank.normalized,
            rank.up,
            12288,
            4096,
            5,
            GEMM,
        ),
        NumericalRole::Down => (
            Qwen3TensorKind::DownProjection,
            rank.activation,
            rank.partial,
            4096,
            12288,
            2,
            PARTIAL,
        ),
    };
    let original = rank.layers[layer as usize].weight(kind);
    let command = policy.command(0, kernel, input, original, output, [rows, n, k, 1, tag]);
    (command, original.read())
}

fn norm(
    rank: &Rank,
    input: Tensor,
    weight: Tensor,
    output: Tensor,
    rows: u32,
    width: u32,
) -> EngineeringTpDispatchV1 {
    let elements = rows as usize * width as usize;
    rmsnorm(
        rank,
        Tensor { elements, ..input },
        weight,
        Tensor { elements, ..output },
        rows,
        width,
    )
}

pub(super) fn matrix(
    kernel: &'static str,
    input: Tensor,
    weight: Tensor,
    output: Tensor,
    [rows, n, k, world, tag]: [u32; 5],
) -> EngineeringTpDispatchV1 {
    use EngineeringTpArgumentV1::U32;
    dispatch(
        kernel,
        n / 16,
        vec![
            input.read(),
            weight.read(),
            output.write(),
            U32(rows),
            U32(n),
            U32(k),
            U32(world),
            U32(tag),
        ],
    )
}

#[cfg(test)]
mod tests;
