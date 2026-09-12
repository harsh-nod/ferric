//! Isolated, non-authoritative, token-at-a-time Qwen tensor-parallel execution.
//!
//! Each transport is one independent GPU child process. Rank-local projections,
//! normalization, rotary embedding, KV append, attention and logits execute on
//! those devices. Baseline FP32 row reductions, residual rounding, rotary-table
//! construction and byte transport run on the host; batched execution also
//! offers explicit reduction ablations. These paths are Contracted, not
//! protected M1 execution or a source-to-device correctness proof.

mod collective;
mod peer_reduction;
mod performance;
mod reduction;
mod row_profile;

#[cfg(feature = "tp-batch-engineering")]
pub mod batched;
pub mod numerical;
#[cfg(feature = "tp-batch-engineering")]
mod projection;
#[cfg(feature = "tp-batch-engineering")]
pub use projection::EngineeringTpProjectionModeV3;

pub use collective::{HostStagedPartialV1, reduce_residual_bf16_v1};
pub use reduction::EngineeringTpReductionModeV3;
use reduction::ReductionWorkspace;

use ferric_build::AuthenticatedModelWeightLayout;
use ferric_engine::tensor_parallel::{
    Qwen3TensorParallelCollectiveStateV1, Qwen3TensorParallelCollectiveV1,
    Qwen3TensorParallelPlanV1, Qwen3TensorParallelRankV1,
};
use ferric_engine::tensor_parallel_execution::TensorParallelSequenceV1;
use ferric_spec::{ModelConfig, QWEN3_NO_LAYER, Qwen3ModelRole, Qwen3TensorKind};
use sha2::{Digest, Sha256};

/// Engineering errors carry context without conferring runtime authority.
pub type TpResult<T> = Result<T, String>;

/// Exact compiler buffer access expected by a dispatch argument.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineeringTpBufferAccessV1 {
    /// Read-only input.
    Read,
    /// Write-only output.
    Write,
    /// Read/write output of an unchanged legacy GEMM entry.
    ReadWrite,
}

/// One ordered explicit compiler argument, with no process-local pointer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EngineeringTpArgumentV1 {
    /// A slice ABI record: an owned-buffer pointer followed by u64 element count.
    Buffer {
        /// Opaque child-local buffer identity.
        id: u64,
        /// Byte offset within that allocation.
        offset: usize,
        /// Exact slice element count; zero is an empty nonnull slice.
        elements: usize,
        /// Size of each element in bytes.
        element_bytes: u32,
        /// Compiler-declared access, checked by the transport.
        access: EngineeringTpBufferAccessV1,
    },
    /// One four-byte integer scalar.
    U32(u32),
    /// One four-byte floating-point scalar.
    F32(f32),
}

/// A one-dimensional dispatch against a retained engineering artifact.
#[derive(Clone, Debug, PartialEq)]
pub struct EngineeringTpDispatchV1 {
    /// Exact entry-point symbol; transports must reject missing or wrong ABIs.
    pub kernel: &'static str,
    /// Workgroups, NOT AQL total workitems.
    pub grid_workgroups: u32,
    /// Workitems per workgroup.
    pub workgroup_size: u16,
    /// Ordered explicit arguments, excluding the zero-filled COV6 tail.
    pub arguments: Vec<EngineeringTpArgumentV1>,
}

/// Safe owned-byte transport to one explicitly opted-in engineering GPU child.
///
/// Exactly one request may be pending after `submit`. `wait` succeeds only on
/// observed completion of that request; all reads/writes are synchronous.
/// A transport implementation must bind its own distinct physical device,
/// exact artifact, scalar ABI, allocation extents and pointer-fixup ownership.
/// These are external Contracted prerequisites, not proved by this driver.
pub trait EngineeringTpRankTransportV1 {
    /// Checks retained successful load receipts before any allocation or dispatch.
    /// # Errors
    /// Rejects unsupported transport, non-fresh state, or any image/root mismatch.
    fn require_loaded_image(&mut self, _image: [u8; 32], _kernels: &[&str]) -> TpResult<()> {
        Err("transport cannot bind a fresh additional kernel image".into())
    }
    /// Explicit diagnostic-only cumulative worker host-wall counters, never GPU timestamps.
    /// # Errors
    /// Rejects disabled profiling, pending work, closed/poisoned state or malformed receipts.
    fn runtime_diagnostic_snapshot(&mut self) -> TpResult<serde_json::Value> {
        Err("transport does not support runtime diagnostic snapshots".into())
    }
    /// Explicit checked mixed-rank publication support; never inferred from PID.
    fn supports_concurrent_rounds(&self) -> bool {
        false
    }
    /// Explicit shared-process peer identity: child PID, logical rank and world.
    /// Independent rank workers return `None`; no capability is inferred.
    fn peer_group_rank(&self) -> Option<(u32, u32, u32)> {
        None
    }
    /// Allocates owner-local memory mapped for read-only argument binding on peers.
    /// # Errors
    /// Rejects unsupported mappings or any ambiguous group transition.
    fn allocate_peer_readable(&mut self, _byte_len: usize) -> TpResult<u64> {
        Err("transport does not support owned peer-readable allocations".into())
    }
    /// Allocates one child-owned, bounded device buffer.
    /// # Errors
    /// Rejects allocation limits, closed state, or child failure.
    fn allocate(&mut self, byte_len: usize) -> TpResult<u64>;
    /// Completes a bounded host-to-device byte copy.
    /// # Errors
    /// Rejects invalid ownership/extents, pending work, or child failure.
    fn write(&mut self, buffer: u64, offset: usize, bytes: &[u8]) -> TpResult<()>;
    /// Completes a bounded device-to-host byte copy.
    /// # Errors
    /// Rejects invalid ownership/extents, pending work, or child failure.
    fn read(&mut self, buffer: u64, offset: usize, bytes: &mut [u8]) -> TpResult<()>;
    /// Sends a dispatch without waiting, enabling overlap across children.
    /// # Errors
    /// Rejects invalid metadata, arguments, ownership, pending work or child failure.
    fn submit(&mut self, dispatch: &EngineeringTpDispatchV1) -> TpResult<()>;
    /// Observes completion of the sole pending dispatch.
    /// # Errors
    /// Rejects absent pending work, timeout, or failed completion.
    fn wait(&mut self) -> TpResult<()>;
    /// Whether bounded ordered command sequences are explicitly enabled.
    fn supports_sequences(&self) -> bool {
        false
    }
    /// Submits one bounded ordered sequence without waiting for completion.
    /// # Errors
    /// Rejects unsupported operation or invalid sequence/ownership.
    fn submit_sequence(&mut self, _dispatches: &[EngineeringTpDispatchV1]) -> TpResult<()> {
        Err("transport does not support dispatch sequences".into())
    }
    /// Confirms exact completion of every command in the pending sequence.
    /// # Errors
    /// Rejects missing/partial/failed completion.
    fn wait_sequence(&mut self, _count: usize) -> TpResult<()> {
        Err("transport does not support dispatch sequences".into())
    }
    /// Explicit dependent packet-batch support, distinct from synchronous sequences.
    fn supports_ordered_batches(&self) -> bool {
        false
    }
    /// Publishes one bounded dependent batch with a single aggregate deadline.
    /// # Errors
    /// Rejects unsupported operation, invalid bindings, or pending work.
    fn submit_ordered_batch(&mut self, _dispatches: &[EngineeringTpDispatchV1]) -> TpResult<()> {
        Err("transport does not support ordered dispatch batches".into())
    }
    /// Confirms the exact completed dispatch count, not per-kernel timing.
    /// # Errors
    /// Rejects missing, partial, failed, or wrong-kind completion.
    fn wait_ordered_batch(&mut self, _count: usize) -> TpResult<()> {
        Err("transport does not support ordered dispatch batches".into())
    }
    /// Whether checked queue destruction and recreation are explicitly enabled.
    fn supports_queue_rollover(&self) -> bool {
        false
    }
    /// Ensures capacity for the next complete batch, retaining user allocations.
    /// # Errors
    /// Rejects exhaustion or any ambiguous rollover transition.
    fn prepare_packets(&mut self, _count: u64) -> TpResult<()> {
        Ok(())
    }
    /// Deterministically tears down the child, including any failed request.
    /// # Errors
    /// Reports any teardown whose completion could not be confirmed.
    fn close(&mut self) -> TpResult<()>;
}

const GEMV: &str = "ferric_qwen3_tp_gemv_bf16_f32_bf16_v1";
const PARTIAL: &str = "ferric_qwen3_tp_gemv_partial_bf16_f32_v1";
const SWIGLU: &str = "ferric_qwen3_tp_swiglu_bf16_f32_v1";
const ROPE: &str = "ferric_qwen3_tp_rope_v1";
const KV_APPEND: &str = "ferric_qwen3_tp_kv_append_v1";
const ATTENTION: &str = "ferric_qwen3_tp_gqa_decode_bf16_f32_v1";
const RMSNORM: &str = "qwen3_rmsnorm_v1";
const EMBEDDING: &str = "ferric_qwen3_token_embedding_bf16_copy_v1";
const LM_HEAD: &str = "ferric_qwen3_gemm_reference_bf16_f32_bf16_v1";
const ARGMAX: &str = "ferric_qwen3_lowest_id_argmax_bf16_v1";
const UPLOAD_CHUNK_BYTES: usize = 1 << 20;

#[derive(Clone, Copy, Debug)]
struct Tensor {
    id: u64,
    elements: usize,
    element_bytes: u32,
}

impl Tensor {
    fn argument(self, access: EngineeringTpBufferAccessV1) -> EngineeringTpArgumentV1 {
        EngineeringTpArgumentV1::Buffer {
            id: self.id,
            offset: 0,
            elements: self.elements,
            element_bytes: self.element_bytes,
            access,
        }
    }

    fn read(self) -> EngineeringTpArgumentV1 {
        self.argument(EngineeringTpBufferAccessV1::Read)
    }

    fn write(self) -> EngineeringTpArgumentV1 {
        self.argument(EngineeringTpBufferAccessV1::Write)
    }
}

struct Layer {
    weights: Vec<(Qwen3TensorKind, Tensor)>,
    k_cache: Tensor,
    v_cache: Tensor,
}

impl Layer {
    fn weight(&self, kind: Qwen3TensorKind) -> Tensor {
        self.weights
            .iter()
            .find(|(candidate, _)| *candidate == kind)
            .expect("complete authenticated layer roster")
            .1
    }
}

struct Rank {
    geometry: Qwen3TensorParallelRankV1,
    layers: Vec<Layer>,
    globals: Vec<(Qwen3TensorKind, Tensor)>,
    hidden: Tensor,
    normalized: Tensor,
    q: Tensor,
    k: Tensor,
    v: Tensor,
    q_normalized: Tensor,
    k_normalized: Tensor,
    q_rotated: Tensor,
    k_rotated: Tensor,
    attention: Tensor,
    gate: Tensor,
    up: Tensor,
    activation: Tensor,
    partial: Tensor,
    cos: Tensor,
    sin: Tensor,
    empty: Tensor,
    token: Tensor,
    logits: Tensor,
    choice: Tensor,
    dispatches: u64,
}

impl Rank {
    fn global(&self, kind: Qwen3TensorKind) -> Tensor {
        self.globals
            .iter()
            .find(|(candidate, _)| *candidate == kind)
            .expect("complete authenticated rank-zero global roster")
            .1
    }
}

/// Persistent weights, contexts and rank-local KV for one engineering sequence.
///
/// A failed step permanently poisons this instance. Successful sequence reset
/// retains allocations but starts KV at position zero; no stale suffix is read.
pub struct EngineeringTpExecutionV1<R: EngineeringTpRankTransportV1> {
    transports: Vec<R>,
    ranks: Vec<Rank>,
    plan: Qwen3TensorParallelPlanV1,
    sequence: TensorParallelSequenceV1,
    collective: Qwen3TensorParallelCollectiveStateV1,
    capacity: u32,
    row_capacity: u32,
    large_kv: bool,
    draft_v10: bool,
    hidden: Vec<u16>,
    reduction: ReductionWorkspace,
    sequences: Option<Vec<Vec<EngineeringTpDispatchV1>>>,
    ordered_batches: Option<Vec<EngineeringTpDispatchV1>>,
    timing: crate::host_timing::HostTiming,
    closed: bool,
}

#[derive(Clone, Copy)]
struct StorageGeometry {
    logical_tokens: u32,
    physical_tokens: u32,
    rows: u32,
    large_kv: bool,
}

impl<R: EngineeringTpRankTransportV1> EngineeringTpExecutionV1<R> {
    /// Verifies actual weight-section bytes, shards them, and uploads each rank.
    ///
    /// Device/artifact admission and distinct-device binding belong to the
    /// transports. No protected authority is created by this constructor.
    ///
    /// # Errors
    /// Rejects geometry, weight identity/range drift, or any transport failure.
    pub fn new(
        transports: Vec<R>,
        model: ModelConfig,
        weights: &[u8],
        layout: &AuthenticatedModelWeightLayout,
        capacity: u32,
    ) -> TpResult<Self> {
        Self::new_with_storage(
            transports,
            model,
            weights,
            layout,
            StorageGeometry {
                logical_tokens: capacity,
                physical_tokens: capacity,
                rows: 1,
                large_kv: false,
            },
        )
    }

    fn new_with_storage(
        mut transports: Vec<R>,
        model: ModelConfig,
        weights: &[u8],
        layout: &AuthenticatedModelWeightLayout,
        storage: StorageGeometry,
    ) -> TpResult<Self> {
        let StorageGeometry {
            logical_tokens: capacity,
            physical_tokens,
            rows,
            large_kv,
        } = storage;
        let prepared = (|| {
            if !(1..=32).contains(&rows)
                || physical_tokens == 0
                || physical_tokens > if large_kv { 262_144 } else { 8192 }
                || (large_kv && (rows != 32 || transports.len() != 1))
            {
                return Err("TP storage row bound exceeded".into());
            }
            let world = u32::try_from(transports.len()).map_err(|_| "too many TP ranks")?;
            let plan = Qwen3TensorParallelPlanV1::new(model, world)
                .map_err(|error| format!("TP geometry: {error:?}"))?;
            let sequence = TensorParallelSequenceV1::new(capacity, model.vocabulary_size)
                .map_err(|error| format!("TP sequence: {error:?}"))?;
            if weights.len() as u64 != model.role.tensor_data_bytes() {
                return Err("prepacked model byte length drifted".into());
            }
            for ordinal in 0..layout.section_count(model.role) {
                let binding = layout
                    .by_ordinal(model.role, ordinal)
                    .map_err(|e| e.to_string())?;
                let source = section_bytes(weights, binding.destination_range())?;
                if Sha256::digest(source).as_slice() != binding.sha256() {
                    return Err(format!("prepacked model tensor {ordinal} digest drifted"));
                }
                for rank in 0..world {
                    plan.tensor(binding.metadata(), rank)
                        .map_err(|error| format!("tensor shard: {error:?}"))?;
                }
            }
            let mut ranks = Vec::with_capacity(transports.len());
            for (index, transport) in transports.iter_mut().enumerate() {
                let index = u32::try_from(index).map_err(|_| "rank index overflow")?;
                ranks.push(allocate_rank_storage(
                    transport,
                    &plan,
                    index,
                    physical_tokens,
                    rows,
                )?);
            }
            for ordinal in 0..layout.section_count(model.role) {
                let binding = layout
                    .by_ordinal(model.role, ordinal)
                    .map_err(|e| e.to_string())?;
                let metadata = binding.metadata();
                let source = section_bytes(weights, binding.destination_range())?;
                for (index, (rank, transport)) in ranks.iter_mut().zip(&mut transports).enumerate()
                {
                    if metadata.layer == QWEN3_NO_LAYER && index != 0 {
                        continue;
                    }
                    let shard = plan
                        .tensor(
                            metadata,
                            u32::try_from(index).map_err(|_| "rank index overflow")?,
                        )
                        .map_err(|e| format!("tensor shard: {e:?}"))?;
                    let row_bytes = shard.columns().count as usize * 2;
                    let rows = shard.rows().count as usize;
                    let tensor = allocate_tensor(
                        transport,
                        rows.checked_mul(row_bytes / 2)
                            .ok_or("tensor size overflow")?,
                        2,
                    )?;
                    let chunk_rows = (UPLOAD_CHUNK_BYTES / row_bytes).max(1);
                    let mut chunk = vec![0; chunk_rows.min(rows) * row_bytes];
                    for row_start in (0..rows).step_by(chunk_rows) {
                        let count = chunk_rows.min(rows - row_start);
                        for local in 0..count {
                            shard
                                .copy_bf16_row_into(
                                    source,
                                    u32::try_from(row_start + local)
                                        .map_err(|_| "shard row overflow")?,
                                    &mut chunk[local * row_bytes..(local + 1) * row_bytes],
                                )
                                .map_err(|e| format!("BF16 shard copy: {e:?}"))?;
                        }
                        transport.write(
                            tensor.id,
                            row_start * row_bytes,
                            &chunk[..count * row_bytes],
                        )?;
                    }
                    if metadata.layer == QWEN3_NO_LAYER {
                        rank.globals.push((metadata.kind, tensor));
                    } else {
                        rank.layers[metadata.layer as usize]
                            .weights
                            .push((metadata.kind, tensor));
                    }
                }
            }
            Ok((plan, sequence, ranks))
        })();
        let (plan, sequence, ranks) = match prepared {
            Ok(value) => value,
            Err(error) => {
                for transport in &mut transports {
                    let _ = transport.close();
                }
                return Err(error);
            }
        };
        let collective = Qwen3TensorParallelCollectiveStateV1::new(&plan, 0, 0);
        Ok(Self {
            transports,
            ranks,
            plan,
            sequence,
            collective,
            capacity,
            row_capacity: rows,
            large_kv,
            draft_v10: false,
            hidden: vec![0; model.hidden_size as usize * rows as usize],
            reduction: ReductionWorkspace::default(),
            sequences: None,
            ordered_batches: None,
            timing: crate::host_timing::HostTiming::default(),
            closed: false,
        })
    }

    /// Returns successfully completed dispatch counts for each logical rank.
    #[must_use]
    pub fn dispatch_counts(&self) -> Vec<u64> {
        self.ranks.iter().map(|rank| rank.dispatches).collect()
    }

    /// Returns the next logical KV position.
    #[must_use]
    pub fn position(&self) -> u32 {
        self.sequence.position()
    }

    /// Starts a new sequence without reallocating weights or reading old KV.
    /// # Errors
    /// Rejects closed, poisoned, in-flight, or epoch-exhausted state.
    pub fn reset_sequence(&mut self) -> TpResult<()> {
        if self.closed {
            return Err("TP execution is closed".into());
        }
        self.sequence
            .reset()
            .map_err(|e| format!("TP reset: {e:?}"))?;
        self.collective =
            Qwen3TensorParallelCollectiveStateV1::new(&self.plan, self.sequence.epoch(), 0);
        self.hidden.fill(0);
        Ok(())
    }

    /// Executes one token on the GPUs and returns rank zero's greedy choice.
    ///
    /// Prompt priming is repeated m=1 decode, not optimized batched prefill.
    ///
    /// # Errors
    /// Rejects invalid tokens, exhausted context, bad state, nonfinite reduction,
    /// or transport failure. Failures after begin permanently poison the group.
    pub fn step(&mut self, token: u32) -> TpResult<u32> {
        if self.closed {
            return Err("TP execution is closed".into());
        }
        let position = self
            .sequence
            .begin(token)
            .map_err(|e| format!("TP begin: {e:?}"))?;
        match self.forward(token, position) {
            Ok(choice) => {
                self.sequence
                    .complete()
                    .map_err(|e| format!("TP completion: {e:?}"))?;
                Ok(choice)
            }
            Err(error) => {
                self.sequence.poison();
                Err(error)
            }
        }
    }

    /// Closes every transport, including after a failed step.
    /// # Errors
    /// Reports all child teardown failures after attempting every close.
    pub fn close(&mut self) -> TpResult<()> {
        self.closed = true;
        let errors = self
            .transports
            .iter_mut()
            .enumerate()
            .filter_map(|(rank, transport)| {
                transport
                    .close()
                    .err()
                    .map(|error| format!("rank {rank}: {error}"))
            })
            .collect::<Vec<_>>();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }

    fn forward(&mut self, token: u32, position: u32) -> TpResult<u32> {
        use EngineeringTpArgumentV1::U32;
        let model = self.plan.model();
        let role = role_tag(model.role);
        let world = self.plan.world_size();
        let key = self.collective.expected();
        if key.layer != 0
            || key.epoch != u64::from(position)
            || key.group_id != self.sequence.epoch()
            || key.operation != Qwen3TensorParallelCollectiveV1::AttentionOutputSum
        {
            return Err("TP collective cursor drifted before token".into());
        }
        let rank = &self.ranks[0];
        self.transports[0].write(rank.token.id, 0, &token.to_le_bytes())?;
        self.dispatch_zero(&dispatch(
            EMBEDDING,
            model.hidden_size / 64,
            vec![
                rank.token.read(),
                rank.global(Qwen3TensorKind::TokenEmbedding).read(),
                rank.hidden.write(),
                U32(1),
                U32(model.hidden_size),
                U32(model.vocabulary_size),
            ],
        ))?;
        self.initialize_hidden_from_embedding()?;
        let (cos, sin) = rope_bytes(position, model.rope_theta);
        for (rank, transport) in self.ranks.iter().zip(&mut self.transports) {
            transport.write(rank.cos.id, 0, &cos)?;
            transport.write(rank.sin.id, 0, &sin)?;
        }
        for layer in 0..model.layers {
            let layer_index = layer as usize;
            self.dispatch_each(|r| {
                rmsnorm(
                    r,
                    r.hidden,
                    r.layers[layer_index].weight(Qwen3TensorKind::InputLayerNorm),
                    r.normalized,
                    1,
                    model.hidden_size,
                )
            })?;
            for (kind, operation) in [
                (Qwen3TensorKind::QueryProjection, 1),
                (Qwen3TensorKind::KeyProjection, 2),
                (Qwen3TensorKind::ValueProjection, 3),
            ] {
                self.dispatch_each(|r| {
                    let output = match operation {
                        1 => r.q,
                        2 => r.k,
                        _ => r.v,
                    };
                    let columns = if operation == 1 {
                        r.geometry.query_channels.count
                    } else {
                        r.geometry.kv_channels.count
                    };
                    dispatch(
                        GEMV,
                        columns / 64,
                        vec![
                            r.normalized.read(),
                            r.layers[layer_index].weight(kind).read(),
                            output.write(),
                            U32(columns),
                            U32(model.hidden_size),
                            U32(role),
                            U32(world),
                            U32(operation),
                        ],
                    )
                })?;
            }
            self.dispatch_each(|r| {
                rmsnorm(
                    r,
                    r.q,
                    r.layers[layer_index].weight(Qwen3TensorKind::QueryNorm),
                    r.q_normalized,
                    r.geometry.query_heads.count,
                    128,
                )
            })?;
            self.dispatch_each(|r| {
                rmsnorm(
                    r,
                    r.k,
                    r.layers[layer_index].weight(Qwen3TensorKind::KeyNorm),
                    r.k_normalized,
                    r.geometry.kv_heads.count,
                    128,
                )
            })?;
            self.dispatch_each(|r| {
                dispatch(
                    ROPE,
                    1,
                    vec![
                        r.q_normalized.read(),
                        r.k_normalized.read(),
                        r.cos.read(),
                        r.sin.read(),
                        r.q_rotated.write(),
                        r.k_rotated.write(),
                        U32(position),
                        U32(role),
                        U32(world),
                    ],
                )
            })?;
            let capacity = self.capacity;
            self.dispatch_each(|r| {
                dispatch(
                    KV_APPEND,
                    1,
                    vec![
                        r.k_rotated.read(),
                        r.v.read(),
                        r.layers[layer_index].k_cache.write(),
                        r.layers[layer_index].v_cache.write(),
                        U32(position),
                        U32(capacity),
                        U32(role),
                        U32(world),
                    ],
                )
            })?;
            self.dispatch_each(|r| {
                dispatch(
                    ATTENTION,
                    r.geometry.query_heads.count,
                    vec![
                        r.q_rotated.read(),
                        r.layers[layer_index].k_cache.read(),
                        r.layers[layer_index].v_cache.read(),
                        r.attention.write(),
                        U32(position + 1),
                        U32(capacity),
                        U32(role),
                        U32(world),
                    ],
                )
            })?;
            self.dispatch_each(|r| {
                dispatch(
                    PARTIAL,
                    model.hidden_size / 64,
                    vec![
                        r.attention.read(),
                        r.layers[layer_index]
                            .weight(Qwen3TensorKind::OutputProjection)
                            .read(),
                        r.partial.write(),
                        U32(model.hidden_size),
                        U32(r.geometry.query_channels.count),
                        U32(role),
                        U32(world),
                        U32(1),
                    ],
                )
            })?;
            self.reduce(layer, Qwen3TensorParallelCollectiveV1::AttentionOutputSum)?;
            self.dispatch_each(|r| {
                rmsnorm(
                    r,
                    r.hidden,
                    r.layers[layer_index].weight(Qwen3TensorKind::PostAttentionLayerNorm),
                    r.normalized,
                    1,
                    model.hidden_size,
                )
            })?;
            for (kind, operation) in [
                (Qwen3TensorKind::GateProjection, 4),
                (Qwen3TensorKind::UpProjection, 5),
            ] {
                self.dispatch_each(|r| {
                    let output = if operation == 4 { r.gate } else { r.up };
                    dispatch(
                        GEMV,
                        r.geometry.intermediate.count / 64,
                        vec![
                            r.normalized.read(),
                            r.layers[layer_index].weight(kind).read(),
                            output.write(),
                            U32(r.geometry.intermediate.count),
                            U32(model.hidden_size),
                            U32(role),
                            U32(world),
                            U32(operation),
                        ],
                    )
                })?;
            }
            self.dispatch_each(|r| {
                dispatch(
                    SWIGLU,
                    r.geometry.intermediate.count / 64,
                    vec![
                        r.gate.read(),
                        r.up.read(),
                        r.activation.write(),
                        U32(role),
                        U32(world),
                    ],
                )
            })?;
            self.dispatch_each(|r| {
                dispatch(
                    PARTIAL,
                    model.hidden_size / 64,
                    vec![
                        r.activation.read(),
                        r.layers[layer_index]
                            .weight(Qwen3TensorKind::DownProjection)
                            .read(),
                        r.partial.write(),
                        U32(model.hidden_size),
                        U32(r.geometry.intermediate.count),
                        U32(role),
                        U32(world),
                        U32(2),
                    ],
                )
            })?;
            self.reduce(layer, Qwen3TensorParallelCollectiveV1::FeedForwardDownSum)?;
        }
        let completed = self.collective.expected();
        if completed.epoch != u64::from(position) + 1
            || completed.layer != 0
            || completed.operation != Qwen3TensorParallelCollectiveV1::AttentionOutputSum
        {
            return Err("token completed before all layer collectives".into());
        }
        let rank = &self.ranks[0];
        self.dispatch_zero(&rmsnorm(
            rank,
            rank.hidden,
            rank.global(Qwen3TensorKind::FinalNorm),
            rank.normalized,
            1,
            model.hidden_size,
        ))?;
        let rank = &self.ranks[0];
        self.dispatch_zero(&dispatch(
            LM_HEAD,
            model.vocabulary_size.div_ceil(16),
            vec![
                rank.normalized.read(),
                rank.global(Qwen3TensorKind::LanguageModelHead).read(),
                rank.logits.argument(EngineeringTpBufferAccessV1::ReadWrite),
                U32(1),
                U32(model.vocabulary_size),
                U32(model.hidden_size),
                U32(0),
            ],
        ))?;
        let rank = &self.ranks[0];
        self.dispatch_zero(&dispatch(
            ARGMAX,
            1,
            vec![
                rank.logits.read(),
                rank.choice.write(),
                U32(1),
                U32(model.vocabulary_size),
            ],
        ))?;
        let mut choice = [0; 4];
        self.transports[0].read(self.ranks[0].choice.id, 0, &mut choice)?;
        let choice = u32::from_le_bytes(choice);
        if choice >= model.vocabulary_size {
            return Err("GPU choice outside vocabulary".into());
        }
        Ok(choice)
    }

    fn dispatch_zero(&mut self, command: &EngineeringTpDispatchV1) -> TpResult<()> {
        let _timing = self.timing.span("dispatch_zero", None);
        let bound = if self.row_capacity == 32 {
            Some(row_profile::bind_mode(
                self.draft_v10,
                32,
                self.large_kv,
                command.clone(),
            )?)
        } else {
            None
        };
        let command = bound.as_ref().unwrap_or(command);
        self.flush_dispatch_groups()?;
        if self.ranks[0].dispatches == u64::MAX {
            return Err("dispatch counter overflow".into());
        }
        self.transports[0].submit(command)?;
        self.transports[0].wait()?;
        self.ranks[0].dispatches += 1;
        Ok(())
    }

    fn dispatch_each(
        &mut self,
        command: impl Fn(&Rank) -> EngineeringTpDispatchV1,
    ) -> TpResult<()> {
        let _timing = self.timing.span("dispatch_each", None);
        if let Some(pending) = &mut self.ordered_batches {
            if self.ranks.len() != 1 || self.sequences.is_some() || pending.len() >= 16 {
                return Err("pending ordered dispatch batch bound drifted".into());
            }
            pending.push(row_profile::bind_mode(
                self.draft_v10,
                self.row_capacity,
                self.large_kv,
                command(&self.ranks[0]),
            )?);
            return Ok(());
        }
        if let Some(pending) = &mut self.sequences {
            if pending.len() != self.ranks.len() || pending.iter().any(|rank| rank.len() >= 16) {
                return Err("pending rank sequence bound drifted".into());
            }
            for (rank, commands) in self.ranks.iter().zip(pending) {
                commands.push(row_profile::bind_mode(
                    self.draft_v10,
                    self.row_capacity,
                    self.large_kv,
                    command(rank),
                )?);
            }
            return Ok(());
        }
        if self.ranks.iter().any(|rank| rank.dispatches == u64::MAX) {
            return Err("dispatch counter overflow".into());
        }
        // A wide-profile routing failure must precede publication on every rank.
        let bound = if self.row_capacity == 32 {
            Some(
                self.ranks
                    .iter()
                    .map(|rank| {
                        row_profile::bind_mode(self.draft_v10, 32, self.large_kv, command(rank))
                    })
                    .collect::<TpResult<Vec<_>>>()?,
            )
        } else {
            None
        };
        let mut submitted = 0;
        let mut error = None;
        for (index, (rank, transport)) in self.ranks.iter().zip(&mut self.transports).enumerate() {
            let owned;
            let next = if let Some(commands) = &bound {
                &commands[index]
            } else {
                owned = command(rank);
                &owned
            };
            match transport.submit(next) {
                Ok(()) => submitted += 1,
                Err(message) => {
                    error = Some(message);
                    break;
                }
            }
        }
        for index in 0..submitted {
            match self.transports[index].wait() {
                Ok(()) => self.ranks[index].dispatches += 1,
                Err(message) => {
                    if error.is_none() {
                        error = Some(message);
                    }
                }
            }
        }
        error.map_or(Ok(()), Err)
    }

    fn reduce(&mut self, layer: u32, operation: Qwen3TensorParallelCollectiveV1) -> TpResult<()> {
        let _timing = self.timing.scope(match operation {
            Qwen3TensorParallelCollectiveV1::AttentionOutputSum => "collective_attention",
            Qwen3TensorParallelCollectiveV1::FeedForwardDownSum => "collective_feed_forward",
        });
        if self.ordered_batches.is_none()
            || self.draft_v10
            || self.reduction.mode() != EngineeringTpReductionModeV3::DeviceTp1V3
        {
            self.flush_dispatch_groups()?;
        }
        match self.reduction.mode() {
            EngineeringTpReductionModeV3::HostStagedReuseV3 => {
                return self.reduce_host_reused(layer, operation);
            }
            EngineeringTpReductionModeV3::DeviceTp1V3 => {
                return self.reduce_device_tp1(layer, operation);
            }
            EngineeringTpReductionModeV3::DevicePeerV4
            | EngineeringTpReductionModeV3::DevicePeerConcurrentV1 => {
                return self.reduce_device_peer(layer, operation);
            }
            EngineeringTpReductionModeV3::HostStagedV1 => {}
        }
        let key = self.collective.expected();
        if key.layer != layer || key.operation != operation {
            return Err("collective operation reordered".into());
        }
        let mut partials = Vec::with_capacity(self.ranks.len());
        for (rank, transport) in self.ranks.iter().zip(&mut self.transports) {
            let mut bytes = vec![0; self.hidden.len() * 4];
            transport.read(rank.partial.id, 0, &mut bytes)?;
            let values = bytes
                .chunks_exact(4)
                .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                .collect::<Vec<_>>();
            partials.push(values);
            self.collective
                .arrive(rank.geometry.rank, key)
                .map_err(|e| format!("collective arrival: {e:?}"))?;
        }
        let borrowed = partials
            .iter()
            .zip(&self.ranks)
            .map(|(values, rank)| HostStagedPartialV1 {
                rank: rank.geometry.rank,
                values,
            })
            .collect::<Vec<_>>();
        let result = reduce_residual_bf16_v1(self.plan.world_size(), &borrowed, &self.hidden)?;
        let bytes = result
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect::<Vec<_>>();
        self.broadcast_hidden(&bytes)?;
        self.collective
            .advance()
            .map_err(|e| format!("collective advance: {e:?}"))?;
        self.hidden = result;
        Ok(())
    }

    fn broadcast_hidden(&mut self, bytes: &[u8]) -> TpResult<()> {
        let _timing = self.timing.span("broadcast_hidden", None);
        if bytes.len() != self.hidden.len() * 2 {
            return Err("hidden broadcast length drifted".into());
        }
        for (rank, transport) in self.ranks.iter().zip(&mut self.transports) {
            transport.write(rank.hidden.id, 0, bytes)?;
        }
        Ok(())
    }
}

fn dispatch(
    kernel: &'static str,
    grid_workgroups: u32,
    arguments: Vec<EngineeringTpArgumentV1>,
) -> EngineeringTpDispatchV1 {
    EngineeringTpDispatchV1 {
        kernel,
        grid_workgroups,
        workgroup_size: 64,
        arguments,
    }
}

fn rmsnorm(
    rank: &Rank,
    input: Tensor,
    weight: Tensor,
    output: Tensor,
    rows: u32,
    width: u32,
) -> EngineeringTpDispatchV1 {
    use EngineeringTpArgumentV1::{F32, U32};
    dispatch(
        RMSNORM,
        rows,
        vec![
            input.read(),
            rank.empty.read(),
            weight.read(),
            rank.empty.write(),
            output.write(),
            U32(rows),
            U32(width),
            F32(1e-6),
            U32(0),
        ],
    )
}

fn role_tag(role: Qwen3ModelRole) -> u32 {
    match role {
        Qwen3ModelRole::Target8B => 1,
        Qwen3ModelRole::Draft06B => 2,
    }
}

fn section_bytes(bytes: &[u8], range: (u64, u64)) -> TpResult<&[u8]> {
    let start = usize::try_from(range.0).map_err(|_| "weight offset overflow")?;
    let len = usize::try_from(range.1).map_err(|_| "weight length overflow")?;
    let end = start.checked_add(len).ok_or("weight range overflow")?;
    bytes
        .get(start..end)
        .ok_or_else(|| "weight section outside prepacked bytes".into())
}

fn allocate_tensor<R: EngineeringTpRankTransportV1>(
    transport: &mut R,
    elements: usize,
    element_bytes: u32,
) -> TpResult<Tensor> {
    let byte_len = elements
        .checked_mul(element_bytes as usize)
        .ok_or("device tensor size overflow")?;
    if byte_len == 0 {
        return Err("empty allocation is not allowed".into());
    }
    Ok(Tensor {
        id: transport.allocate(byte_len)?,
        elements,
        element_bytes,
    })
}

#[cfg(test)]
fn allocate_rank<R: EngineeringTpRankTransportV1>(
    transport: &mut R,
    plan: &Qwen3TensorParallelPlanV1,
    index: u32,
    capacity: u32,
) -> TpResult<Rank> {
    allocate_rank_storage(transport, plan, index, capacity, 1)
}

fn allocate_rank_storage<R: EngineeringTpRankTransportV1>(
    transport: &mut R,
    plan: &Qwen3TensorParallelPlanV1,
    index: u32,
    capacity: u32,
    rows: u32,
) -> TpResult<Rank> {
    if !(1..=32).contains(&rows) {
        return Err("TP storage row bound exceeded".into());
    }
    let geometry = plan
        .rank(index)
        .map_err(|e| format!("rank geometry: {e:?}"))?;
    let model = plan.model();
    let rows = rows as usize;
    let hidden = model.hidden_size as usize * rows;
    let q = geometry.query_channels.count as usize * rows;
    let kv = geometry.kv_channels.count as usize * rows;
    let intermediate = geometry.intermediate.count as usize * rows;
    let cache = (geometry.kv_channels.count as usize)
        .checked_mul(capacity as usize)
        .ok_or("KV cache size overflow")?;
    let mut layers = Vec::new();
    for _ in 0..model.layers {
        layers.push(Layer {
            weights: Vec::new(),
            k_cache: allocate_tensor(transport, cache, 2)?,
            v_cache: allocate_tensor(transport, cache, 2)?,
        });
    }
    let mut empty = allocate_tensor(transport, 1, 2)?;
    empty.elements = 0;
    Ok(Rank {
        geometry,
        layers,
        globals: Vec::new(),
        hidden: allocate_tensor(transport, hidden, 2)?,
        normalized: allocate_tensor(transport, hidden, 2)?,
        q: allocate_tensor(transport, q, 2)?,
        k: allocate_tensor(transport, kv, 2)?,
        v: allocate_tensor(transport, kv, 2)?,
        q_normalized: allocate_tensor(transport, q, 2)?,
        k_normalized: allocate_tensor(transport, kv, 2)?,
        q_rotated: allocate_tensor(transport, q, 2)?,
        k_rotated: allocate_tensor(transport, kv, 2)?,
        attention: allocate_tensor(transport, q, 2)?,
        gate: allocate_tensor(transport, intermediate, 2)?,
        up: allocate_tensor(transport, intermediate, 2)?,
        activation: allocate_tensor(transport, intermediate, 2)?,
        partial: allocate_tensor(transport, hidden, 4)?,
        cos: allocate_tensor(transport, 64 * rows, 4)?,
        sin: allocate_tensor(transport, 64 * rows, 4)?,
        empty,
        token: allocate_tensor(transport, rows, 4)?,
        logits: allocate_tensor(
            transport,
            if index == 0 {
                model.vocabulary_size as usize * rows
            } else {
                1
            },
            2,
        )?,
        choice: allocate_tensor(transport, rows, 4)?,
        dispatches: 0,
    })
}

fn decode_bf16(bytes: &[u8]) -> TpResult<Vec<u16>> {
    if !bytes.len().is_multiple_of(2) {
        return Err("odd BF16 payload length".into());
    }
    let values = bytes
        .chunks_exact(2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .collect::<Vec<_>>();
    if values
        .iter()
        .any(|bits| !f32::from_bits(u32::from(*bits) << 16).is_finite())
    {
        return Err("nonfinite embedding output".into());
    }
    Ok(values)
}

#[allow(clippy::cast_possible_truncation)]
fn rope_bytes(position: u32, theta: u32) -> (Vec<u8>, Vec<u8>) {
    let mut cos = Vec::with_capacity(256);
    let mut sin = Vec::with_capacity(256);
    for pair in 0..64 {
        let inverse_frequency = f64::from(theta).powf(-f64::from(pair) / 64.0);
        let angle = f64::from(position) * inverse_frequency;
        cos.extend_from_slice(&(angle.cos() as f32).to_le_bytes());
        sin.extend_from_slice(&(angle.sin() as f32).to_le_bytes());
    }
    (cos, sin)
}

#[cfg(test)]
mod tests;
