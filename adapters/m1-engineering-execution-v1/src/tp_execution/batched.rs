//! Resident, multi-row Qwen3 execution over a separately admitted paged image.
//!
//! Every layer executes one GPU batch across all ranks. Page metadata comes
//! from the transactional pool; intermediate prompt rows remain causal but do
//! not publish output tokens. Arithmetic and transport remain Contracted.

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
const PARTIAL: &str = "ferric_qwen3_tp_batch_gemm_partial_bf16_f32_v2";
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
pub struct EngineeringTpBatchExecutionV2<R: EngineeringTpRankTransportV1> {
    inner: EngineeringTpExecutionV1<R>,
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
        mut transports: Vec<R>,
        model: ModelConfig,
        weights: &[u8],
        layout: &AuthenticatedModelWeightLayout,
        pool: &EngineeringTpPagedPoolV1,
    ) -> TpResult<Self> {
        if model.role != Qwen3ModelRole::Target8B || !pool.is_empty() {
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
            u32::try_from(MAX_ROWS).map_err(|_| "row limit conversion")?,
        )?;
        let metadata = (|| {
            let mut positions = Vec::new();
            let mut page_tables = Vec::new();
            for transport in &mut inner.transports {
                positions.push(allocate_tensor(transport, MAX_ROWS, 4)?);
                page_tables.push(allocate_tensor(
                    transport,
                    MAX_ROWS * table_stride as usize,
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
        })
    }

    /// Selects an explicit reduction ablation before the first batch.
    /// `DeviceTp1V3` requires a transport admitting the separate v3 image.
    ///
    /// # Errors
    /// Rejects a started/poisoned stream, unsupported profile, or allocation failure.
    pub fn configure_reduction(&mut self, mode: EngineeringTpReductionModeV3) -> TpResult<()> {
        if self.poisoned || self.last_batch != 0 || self.completed_batches != 0 {
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
        if self.last_batch != 0 || self.poisoned || self.inner.closed {
            return Err("output-head policy must be configured before execution".into());
        }
        self.prune_output_head = enabled;
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
            .map(|rank| 540 + extra + if rank == 0 { 1 + head } else { 0 })
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
        if next.checked_mul(per_rank).is_none_or(|packets| {
            packets > fe2o3_kfd::engineering_wire::MAX_UNRETIRED_RING_PACKETS_V1
        }) {
            return Err("batched stream exceeds the conservative no-ring-rollover budget".into());
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
        self.inner.close()
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
            || batch.rows().len() > MAX_ROWS
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
        let model = self.inner.plan.model();
        let world = self.inner.plan.world_size();
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

        for layer in 0..model.layers {
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
                    matrix(
                        GEMM,
                        r.normalized,
                        r.layers[li].weight(kind),
                        output,
                        [rows, n, model.hidden_size, world, tag],
                    )
                })?;
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
                    ATTENTION,
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
                matrix(
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
            self.inner
                .reduce(layer, Qwen3TensorParallelCollectiveV1::AttentionOutputSum)?;
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
                    matrix(
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
                matrix(
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
        self.inner.dispatch_zero(&matrix(
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
        let mut bytes = vec![0; head_rows as usize * 4];
        self.inner.transports[0].read(self.inner.ranks[0].choice.id, 0, &mut bytes)?;
        let choices = bytes
            .chunks_exact(4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect::<Vec<_>>();
        if choices.iter().any(|&token| token >= model.vocabulary_size) {
            return Err("batched GPU choice outside vocabulary".into());
        }
        if self.prune_output_head {
            Ok(choices)
        } else {
            Ok(output_rows.iter().map(|&row| choices[row]).collect())
        }
    }
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

fn matrix(
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
