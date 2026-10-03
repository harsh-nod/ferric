//! Source-owned, inert contract for a two-forward finite-worker composition.
//! No native registration, allocation, artifact admission or launch occurs here.

use super::*;
use std::collections::BTreeSet;
use std::marker::PhantomData;

#[path = "finite_composition_wire_projection.rs"]
mod wire_projection;

const WEIGHTS: [(Qwen3TensorKind, usize); 11] = [
    (Qwen3TensorKind::InputLayerNorm, 4096),
    (Qwen3TensorKind::QueryProjection, 2048 * 4096),
    (Qwen3TensorKind::KeyProjection, 512 * 4096),
    (Qwen3TensorKind::ValueProjection, 512 * 4096),
    (Qwen3TensorKind::QueryNorm, 128),
    (Qwen3TensorKind::KeyNorm, 128),
    (Qwen3TensorKind::OutputProjection, 4096 * 2048),
    (Qwen3TensorKind::PostAttentionLayerNorm, 4096),
    (Qwen3TensorKind::GateProjection, 6144 * 4096),
    (Qwen3TensorKind::UpProjection, 6144 * 4096),
    (Qwen3TensorKind::DownProjection, 4096 * 6144),
];

/// An existing child-local allocation viewed at its exact finite-worker extent.
/// These descriptive numbers are not a native buffer handle or import authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EngineeringTp2FiniteBufferV1 {
    rank: u32,
    id: u64,
    elements: usize,
    element_bytes: u32,
}

impl EngineeringTp2FiniteBufferV1 {
    /// Owning logical rank.
    #[must_use]
    pub const fn rank(self) -> u32 {
        self.rank
    }
    /// Existing transport-local allocation identity.
    #[must_use]
    pub const fn id(self) -> u64 {
        self.id
    }
    /// Exact number of elements in the offset-zero finite view.
    #[must_use]
    pub const fn elements(self) -> usize {
        self.elements
    }
    /// Scalar width; the closed role distinguishes F32 from U32.
    #[must_use]
    pub const fn element_bytes(self) -> u32 {
        self.element_bytes
    }
}

/// Existing per-rank scratch roles in the closed layer dataflow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineeringTp2FiniteScratchRoleV1 {
    /// Embedding or previous final residual; destination of the final residual.
    Hidden,
    /// First residual destination and MLP input, never normalized input.
    PostAttentionResidual,
    /// Shared normalization scratch, consumed before its next overwrite.
    Normalized,
    /// Post-head-normalization and RoPE query.
    Query,
    /// Attention output feeding the output projection.
    Attention,
    /// Gate projection result.
    Gate,
    /// Up projection result.
    Up,
    /// SwiGLU result.
    Activation,
    /// FP32 O/down partial, each consumed before its next overwrite.
    Partial,
}

/// Existing ordinary source-grammar buffers, not additional native handles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineeringTp2FiniteAuxiliaryKindV1 {
    /// Unnormalized Q projection, BF16.
    QueryProjection,
    /// Unnormalized K projection, BF16.
    KeyProjection,
    /// V projection, BF16.
    ValueProjection,
    /// Head-normalized Q, BF16.
    QueryNormalized,
    /// Head-normalized K, BF16.
    KeyNormalized,
    /// Rotated K, BF16.
    KeyRotated,
    /// Cosine table, F32.
    Cosine,
    /// Sine table, F32.
    Sine,
    /// Logical zero-length BF16 placeholder; native allocation remains nonnull.
    Empty,
    /// Input token indices, U32.
    Token,
    /// BF16 output-head logits; rank one retains its unused one-element buffer.
    Logits,
    /// Greedy output indices, U32.
    Choice,
    /// Ordinary prepared positions, U32.
    Positions,
    /// Ordinary prepared page table, U32; distinct from finite CacheMetadata.
    PageTable,
}

/// Original, authenticated source weight shards and per-layer caches.
#[derive(Debug)]
pub struct EngineeringTp2FiniteLayerV1 {
    rank: u32,
    layer: u32,
    weights: Vec<(Qwen3TensorKind, EngineeringTp2FiniteBufferV1)>,
    caches: [EngineeringTp2FiniteBufferV1; 2],
}

impl EngineeringTp2FiniteLayerV1 {
    /// Owning rank.
    #[must_use]
    pub const fn rank(&self) -> u32 {
        self.rank
    }
    /// Model layer ordinal.
    #[must_use]
    pub const fn layer(&self) -> u32 {
        self.layer
    }
    /// Original NxK weights, not queued MFMA transposes. Q/K/V and Q/K norm
    /// still require a separately checked packing operation before registration.
    #[must_use]
    pub fn weights(&self) -> &[(Qwen3TensorKind, EngineeringTp2FiniteBufferV1)] {
        &self.weights
    }
    /// Distinct K and V caches, each 2304 * 512 BF16 elements.
    #[must_use]
    pub const fn caches(&self) -> &[EngineeringTp2FiniteBufferV1; 2] {
        &self.caches
    }
}

/// New non-atomic allocations required before a future native registration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineeringTp2FinitePendingBufferKindV1 {
    /// Per-layer concatenation of Q, K and V original NxK matrices.
    PackedQkvWeight,
    /// Per-layer Q-norm followed by K-norm BF16 vectors.
    PackedHeadNormWeight,
    /// Per-rank packed projection result.
    QkvOutput,
    /// Per-rank cosine followed by sine F32 values.
    Rotary,
    /// Per-rank position plus complete 144-page U32 permutation.
    CacheMetadata,
}

/// An allocation requirement only: deliberately has no buffer identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EngineeringTp2FinitePendingBufferV1 {
    rank: u32,
    layer: Option<u32>,
    kind: EngineeringTp2FinitePendingBufferKindV1,
    elements: usize,
    element_bytes: u32,
}

impl EngineeringTp2FinitePendingBufferV1 {
    /// Owning rank.
    #[must_use]
    pub const fn rank(self) -> u32 {
        self.rank
    }
    /// Layer for immutable packed weights, otherwise shared rank-local scratch.
    #[must_use]
    pub const fn layer(self) -> Option<u32> {
        self.layer
    }
    /// Required closed allocation role; this distinguishes F32 from U32.
    #[must_use]
    pub const fn kind(self) -> EngineeringTp2FinitePendingBufferKindV1 {
        self.kind
    }
    /// Exact element count.
    #[must_use]
    pub const fn elements(self) -> usize {
        self.elements
    }
    /// Exact scalar width.
    #[must_use]
    pub const fn element_bytes(self) -> u32 {
        self.element_bytes
    }
}

/// Distinct unchanged finite worker protocols.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EngineeringTp2FiniteStateKindV1 {
    /// V5 prefix, 16 tasks and 22 coherent AtomicU32 words.
    PrefixV5,
    /// V1 MLP, five tasks and 11 coherent AtomicU32 words.
    MlpV1,
}

/// A logical use site, not a minted native state reservation or device pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EngineeringTp2FiniteStateSlotV1 {
    forward: u32,
    layer: u32,
    rank: u32,
    kind: EngineeringTp2FiniteStateKindV1,
}

impl EngineeringTp2FiniteStateSlotV1 {
    /// Bounded forward ordinal, zero or one.
    #[must_use]
    pub const fn forward(self) -> u32 {
        self.forward
    }
    /// Layer ordinal, zero through 35.
    #[must_use]
    pub const fn layer(self) -> u32 {
        self.layer
    }
    /// Owning rank, zero or one.
    #[must_use]
    pub const fn rank(self) -> u32 {
        self.rank
    }
    /// Protocol-specific typed state still to be allocated by the native owner.
    #[must_use]
    pub const fn kind(self) -> EngineeringTp2FiniteStateKindV1 {
        self.kind
    }
    /// Exact coherent atomic word count; device epoch remains one for every slot.
    #[must_use]
    pub const fn atomic_words(self) -> usize {
        match self.kind {
            EngineeringTp2FiniteStateKindV1::PrefixV5 => 22,
            EngineeringTp2FiniteStateKindV1::MlpV1 => 11,
        }
    }
}

/// Immutable prepare-only profile, borrowed from one authenticated execution.
/// It is not accepted by an existing queued registration API. Native typed
/// allocation, packing validation, artifact joins and execution remain pending.
#[derive(Debug)]
pub struct EngineeringTp2FiniteCompositionV1<'owner> {
    bundle_id: [u8; 32],
    model_id: [u8; 32],
    session: [u8; 32],
    pool_identity: u64,
    group_id: u64,
    child_identity: u32,
    layers: Vec<EngineeringTp2FiniteLayerV1>,
    globals: Vec<(Qwen3TensorKind, EngineeringTp2FiniteBufferV1)>,
    auxiliary: Vec<(
        EngineeringTp2FiniteAuxiliaryKindV1,
        EngineeringTp2FiniteBufferV1,
    )>,
    scratch: Vec<(
        EngineeringTp2FiniteScratchRoleV1,
        EngineeringTp2FiniteBufferV1,
    )>,
    pending_buffers: Vec<EngineeringTp2FinitePendingBufferV1>,
    state_slots: Vec<EngineeringTp2FiniteStateSlotV1>,
    source_program: Program,
    owner: PhantomData<&'owner ()>,
}

impl EngineeringTp2FiniteCompositionV1<'_> {
    /// Distinct inert profile; no queued-profile enum is extended.
    pub const PROFILE: &'static str =
        "qwen3-8b-tp2-finite-prefix-v5-mlp-v1-two-forward-context2304-v1";
    /// Exactly two forwards; not the sustained 2048/256 performance workload.
    pub const FORWARDS: u32 = 2;
    /// Exact total typed-state requirements; 144 per owner rank.
    pub const STATE_SLOTS: usize = 288;
    /// Authenticated bundle identity retained after weight intake.
    #[must_use]
    pub const fn bundle_id(&self) -> &[u8; 32] {
        &self.bundle_id
    }
    /// Admitted target-model identity, distinct from its bundle identity.
    #[must_use]
    pub const fn model_id(&self) -> &[u8; 32] {
        &self.model_id
    }
    /// Existing pool session, not a newly minted native registration.
    #[must_use]
    pub const fn session(&self) -> &[u8; 32] {
        &self.session
    }
    /// Existing pool identity.
    #[must_use]
    pub const fn pool_identity(&self) -> u64 {
        self.pool_identity
    }
    /// Existing collective group identity; meaningful only with session/child.
    #[must_use]
    pub const fn group_id(&self) -> u64 {
        self.group_id
    }
    /// Existing common child identity; no claim of native registration.
    #[must_use]
    pub const fn child_identity(&self) -> u32 {
        self.child_identity
    }
    /// All 72 rank/layer records in rank-major order.
    #[must_use]
    pub fn layers(&self) -> &[EngineeringTp2FiniteLayerV1] {
        &self.layers
    }
    /// Rank-zero original embedding, final norm and language-model head weights.
    #[must_use]
    pub fn globals(&self) -> &[(Qwen3TensorKind, EngineeringTp2FiniteBufferV1)] {
        &self.globals
    }
    /// All 28 ordinary source-grammar buffers in rank-major closed-role order.
    #[must_use]
    pub fn auxiliary(
        &self,
    ) -> &[(
        EngineeringTp2FiniteAuxiliaryKindV1,
        EngineeringTp2FiniteBufferV1,
    )] {
        &self.auxiliary
    }
    /// Existing scratch with exact single-row views, grouped by rank.
    #[must_use]
    pub fn scratch(
        &self,
    ) -> &[(
        EngineeringTp2FiniteScratchRoleV1,
        EngineeringTp2FiniteBufferV1,
    )] {
        &self.scratch
    }
    /// No IDs are assigned to these 150 pending ordinary allocations.
    #[must_use]
    pub fn pending_buffers(&self) -> &[EngineeringTp2FinitePendingBufferV1] {
        &self.pending_buffers
    }
    /// Forward/layer/phase/rank ordered use sites. Native IDs must be freshly
    /// minted by the owning group, never imported from these descriptive slots.
    #[must_use]
    pub fn state_slots(&self) -> &[EngineeringTp2FiniteStateSlotV1] {
        &self.state_slots
    }
    /// Unchanged queued source grammar, including embedding and final head.
    /// This is input to future lowering, not a finite-native launch program.
    #[must_use]
    pub const fn source_program(&self) -> &Program {
        &self.source_program
    }
}

fn bind(
    seen: &mut BTreeSet<(u32, u64)>,
    rank: u32,
    tensor: Tensor,
    allocation_elements: usize,
    view_elements: usize,
    element_bytes: u32,
) -> TpResult<EngineeringTp2FiniteBufferV1> {
    if tensor.id == 0
        || tensor.elements != allocation_elements
        || tensor.element_bytes != element_bytes
        || view_elements > allocation_elements
        || !seen.insert((rank, tensor.id))
    {
        return Err(
            "finite composition has wrong scalar/extent or aliased source allocation".into(),
        );
    }
    Ok(EngineeringTp2FiniteBufferV1 {
        rank,
        id: tensor.id,
        elements: view_elements,
        element_bytes,
    })
}

impl<R: EngineeringTpRankTransportV1> EngineeringTpBatchExecutionV2<R> {
    /// Records an inert, source-owned finite profile without touching a transport.
    /// The legacy queued baseline supplies the whole-token source grammar only;
    /// its capability checks do not authorize the new finite workers.
    /// # Errors
    /// Rejects missing authentic target binding, stale owner/session state,
    /// unsupported geometry, incomplete weights, scalar/extent errors and aliases.
    pub fn prepare_finite_two_forward_composition_v1(
        &self,
    ) -> TpResult<EngineeringTp2FiniteCompositionV1<'_>> {
        self.inner.check_peer_dependency_source_profile()?;
        self.check_prepared_source_profile(
            Some(GraphPolicy::TransactionFences),
            GraphKernels::Baseline,
            Geometry::Long2304,
        )?;
        let (bundle_id, model) = self
            .inner
            .finite_model_binding
            .ok_or("finite composition requires authenticated target-model intake")?;
        if bundle_id == [0; 32]
            || model != self.inner.plan.model()
            || model.role != ferric_spec::Qwen3ModelRole::Target8B
            || model.query_heads != 32
            || model.kv_heads != 8
            || model.head_dim != 128
            || model.max_position_embeddings != 40_960
            || model.rope_theta != 1_000_000
            || model.tie_word_embeddings
            || self.scope.model != *model.model_id.as_bytes()
            || self.scope.model == [0; 32]
            || self.scope.session == [0; 32]
            || self.pool_identity == 0
            || self.prepared_peer.is_some()
            || self.completed_batches != 0
            || self.last_batch != 0
            || self.inner.position() != 0
            || self.inner.peer_dependency_pending.is_some()
            || self.inner.ranks.len() != 2
            || self.inner.transports.len() != 2
            || self.positions.len() != 2
            || self.page_tables.len() != 2
            || self.split_attention_scratch.is_some()
            || self
                .inner
                .ranks
                .iter()
                .any(|rank| rank.dispatches != 0 || rank.layers.len() != 36)
        {
            return Err(
                "finite composition requires fresh authentic Qwen3-8B TP2 owner state".into(),
            );
        }
        let Some((child_identity, 0, 2)) = self.inner.transports[0].peer_group_rank() else {
            return Err("finite composition requires an existing two-rank child identity".into());
        };
        if child_identity == 0
            || self.inner.transports[1].peer_group_rank() != Some((child_identity, 1, 2))
        {
            return Err("finite composition child/rank identity mismatch".into());
        }
        let ReductionWorkspace::DevicePeer(residuals, _) = &self.inner.reduction else {
            return Err("finite composition requires existing resident residual scratch".into());
        };
        if residuals.len() != 2 {
            return Err("finite residual cardinality".into());
        }
        let mut seen = BTreeSet::new();
        let mut layers = Vec::with_capacity(72);
        let mut globals = Vec::with_capacity(3);
        let mut auxiliary = Vec::with_capacity(28);
        let mut scratch = Vec::with_capacity(18);
        let mut pending_buffers = Vec::with_capacity(150);
        for (index, rank) in self.inner.ranks.iter().enumerate() {
            let owner = u32::try_from(index).map_err(|_| "finite rank ordinal")?;
            let expected = self
                .inner
                .plan
                .rank(owner)
                .map_err(|_| "finite rank plan")?;
            if rank.geometry != expected {
                return Err("finite rank geometry mismatch".into());
            }
            // The source constructor has uploaded original NxK shards. Do not
            // substitute the optional queued MFMA transposition cache here.
            for (ordinal, layer) in rank.layers.iter().enumerate() {
                let ordinal = u32::try_from(ordinal).map_err(|_| "finite layer ordinal")?;
                if layer.weights.len() != WEIGHTS.len() {
                    return Err("finite original weight roster cardinality".into());
                }
                let mut weights = Vec::with_capacity(WEIGHTS.len());
                for (kind, elements) in WEIGHTS {
                    let mut matches = layer.weights.iter().filter(|(actual, _)| *actual == kind);
                    let (_, tensor) = matches
                        .next()
                        .ok_or("finite original weight role missing")?;
                    if matches.next().is_some() {
                        return Err("finite duplicate weight role".into());
                    }
                    weights.push((
                        kind,
                        bind(&mut seen, owner, *tensor, elements, elements, 2)?,
                    ));
                }
                let caches = [
                    bind(&mut seen, owner, layer.k_cache, 2304 * 512, 2304 * 512, 2)?,
                    bind(&mut seen, owner, layer.v_cache, 2304 * 512, 2304 * 512, 2)?,
                ];
                layers.push(EngineeringTp2FiniteLayerV1 {
                    rank: owner,
                    layer: ordinal,
                    weights,
                    caches,
                });
                for (kind, elements) in [
                    (
                        EngineeringTp2FinitePendingBufferKindV1::PackedQkvWeight,
                        3072 * 4096,
                    ),
                    (
                        EngineeringTp2FinitePendingBufferKindV1::PackedHeadNormWeight,
                        256,
                    ),
                ] {
                    pending_buffers.push(EngineeringTp2FinitePendingBufferV1 {
                        rank: owner,
                        layer: Some(ordinal),
                        kind,
                        elements,
                        element_bytes: 2,
                    });
                }
            }
            use EngineeringTp2FiniteScratchRoleV1 as S;
            for (role, tensor, elements, width) in [
                (S::Hidden, rank.hidden, 4096, 2),
                (S::PostAttentionResidual, residuals[index], 4096, 2),
                (S::Normalized, rank.normalized, 4096, 2),
                (S::Query, rank.q_rotated, 2048, 2),
                (S::Attention, rank.attention, 2048, 2),
                (S::Gate, rank.gate, 6144, 2),
                (S::Up, rank.up, 6144, 2),
                (S::Activation, rank.activation, 6144, 2),
                (S::Partial, rank.partial, 4096, 4),
            ] {
                scratch.push((
                    role,
                    bind(&mut seen, owner, tensor, elements * 16, elements, width)?,
                ));
            }
            // These remain in the retained source grammar (including its head),
            // even though the future finite layer lowering will replace them.
            use EngineeringTp2FiniteAuxiliaryKindV1 as A;
            for (kind, tensor, elements, width) in [
                (A::QueryProjection, rank.q, 2048 * 16, 2),
                (A::KeyProjection, rank.k, 512 * 16, 2),
                (A::ValueProjection, rank.v, 512 * 16, 2),
                (A::QueryNormalized, rank.q_normalized, 2048 * 16, 2),
                (A::KeyNormalized, rank.k_normalized, 512 * 16, 2),
                (A::KeyRotated, rank.k_rotated, 512 * 16, 2),
                (A::Cosine, rank.cos, 64 * 16, 4),
                (A::Sine, rank.sin, 64 * 16, 4),
                (A::Empty, rank.empty, 0, 2),
                (A::Token, rank.token, 16, 4),
                (
                    A::Logits,
                    rank.logits,
                    if owner == 0 { 151_936 * 16 } else { 1 },
                    2,
                ),
                (A::Choice, rank.choice, 16, 4),
                (A::Positions, self.positions[index], 16, 4),
                (A::PageTable, self.page_tables[index], 16 * 144, 4),
            ] {
                auxiliary.push((
                    kind,
                    bind(&mut seen, owner, tensor, elements, elements, width)?,
                ));
            }
            for (kind, elements, element_bytes) in [
                (EngineeringTp2FinitePendingBufferKindV1::QkvOutput, 3072, 2),
                (EngineeringTp2FinitePendingBufferKindV1::Rotary, 128, 4),
                (
                    EngineeringTp2FinitePendingBufferKindV1::CacheMetadata,
                    145,
                    4,
                ),
            ] {
                pending_buffers.push(EngineeringTp2FinitePendingBufferV1 {
                    rank: owner,
                    layer: None,
                    kind,
                    elements,
                    element_bytes,
                });
            }
        }
        // Validate globals before the existing recorder's infallible lookup.
        let zero = &self.inner.ranks[0];
        if zero.globals.len() != 3 || !self.inner.ranks[1].globals.is_empty() {
            return Err("finite rank-zero global roster".into());
        }
        for (kind, elements) in [
            (Qwen3TensorKind::TokenEmbedding, 151_936 * 4096),
            (Qwen3TensorKind::FinalNorm, 4096),
            (Qwen3TensorKind::LanguageModelHead, 151_936 * 4096),
        ] {
            let mut matches = zero.globals.iter().filter(|(actual, _)| *actual == kind);
            let (_, tensor) = matches.next().ok_or("finite global weight missing")?;
            if matches.next().is_some() {
                return Err("finite duplicate global weight".into());
            }
            globals.push((kind, bind(&mut seen, 0, *tensor, elements, elements, 2)?));
        }
        let source_program = self.record_prepared_program(Geometry::Long2304)?;
        self.validate_finite_source_boundaries(&source_program)?;
        let mut state_slots = Vec::with_capacity(EngineeringTp2FiniteCompositionV1::STATE_SLOTS);
        for forward in 0..EngineeringTp2FiniteCompositionV1::FORWARDS {
            for layer in 0..36 {
                for kind in [
                    EngineeringTp2FiniteStateKindV1::PrefixV5,
                    EngineeringTp2FiniteStateKindV1::MlpV1,
                ] {
                    for rank in 0..2 {
                        state_slots.push(EngineeringTp2FiniteStateSlotV1 {
                            forward,
                            layer,
                            rank,
                            kind,
                        });
                    }
                }
            }
        }
        Ok(EngineeringTp2FiniteCompositionV1 {
            bundle_id,
            model_id: *model.model_id.as_bytes(),
            session: self.scope.session,
            pool_identity: self.pool_identity,
            group_id: source_program.group_id,
            child_identity,
            layers,
            globals,
            auxiliary,
            scratch,
            pending_buffers,
            state_slots,
            source_program,
            owner: PhantomData,
        })
    }

    // Check exact existing source grammar. Its head is still queued MFMA with
    // an authenticated transpose; the separately retained global is original
    // NxK. This check is not a finite-native head layout conversion.
    pub(crate) fn validate_finite_source_boundaries(&self, program: &Program) -> TpResult<()> {
        let zero = &self.inner.ranks[0];
        let one = &self.inner.ranks[1];
        if program.steps.len() != 1013
            || program.group_id != self.inner.collective.expected().group_id
            || program.token_buffer != zero.token.id
            || program.result_buffer != zero.choice.id
        {
            return Err("finite source boundary header or cardinality".into());
        }
        for rank in 0..2 {
            if program.metadata[rank]
                != (Metadata {
                    positions: self.positions[rank].id,
                    page_table: self.page_tables[rank].id,
                    cos: self.inner.ranks[rank].cos.id,
                    sin: self.inner.ranks[rank].sin.id,
                })
            {
                return Err("finite source ordinary metadata binding".into());
            }
        }
        let expected = [
            (
                0,
                0,
                dispatch(
                    EMBEDDING,
                    64,
                    vec![
                        zero.token.read(),
                        zero.global(Qwen3TensorKind::TokenEmbedding).read(),
                        zero.hidden.write(),
                        U32(1),
                    ],
                ),
            ),
            (
                1,
                1,
                dispatch(
                    "ferric_qwen3_tp_peer_copy_bf16_v4",
                    64,
                    vec![
                        Tensor {
                            elements: 4096,
                            ..zero.hidden
                        }
                        .read(),
                        Tensor {
                            elements: 4096,
                            ..one.hidden
                        }
                        .write(),
                        U32(1),
                    ],
                ),
            ),
            (
                1010,
                0,
                norm(
                    zero,
                    zero.hidden,
                    zero.global(Qwen3TensorKind::FinalNorm),
                    zero.normalized,
                    1,
                    4096,
                ),
            ),
            (
                1011,
                0,
                self.projection.command(
                    0,
                    GEMM,
                    zero.normalized,
                    zero.global(Qwen3TensorKind::LanguageModelHead),
                    zero.logits,
                    [1, 151_936, 4096, 2, 6],
                ),
            ),
            (
                1012,
                0,
                dispatch(
                    ARGMAX,
                    1,
                    vec![zero.logits.read(), zero.choice.write(), U32(1)],
                ),
            ),
        ];
        for (index, rank, dispatch) in expected {
            if program.steps[index] != (Step::Rank { rank, dispatch }) {
                return Err("finite embedding/copy/final-norm/head/argmax source binding".into());
            }
        }
        Ok(())
    }
}
