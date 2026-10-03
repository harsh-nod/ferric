//! Joint-drain receipts for the explicit TP2 graph, not serial collective receipts.

use super::{
    EngineeringTp2PreparedInputV1 as Input, EngineeringTp2PreparedProgramV1 as Program,
    EngineeringTp2PreparedStepV1 as Step, TpResult,
};
use ferric_engine::tensor_parallel::{
    Qwen3TensorParallelCollectiveKeyV1 as Key, Qwen3TensorParallelCollectiveV1 as Operation,
};
use ferric_spec::Qwen3ModelRole;

const PACKETS: [u64; 2] = [759, 757];
const KERNELS: [u32; 2] = [616, 613];
const BARRIERS: [u32; 2] = [143, 144];

/// Immutable graph geometry; legacy interpreter and serial profiles remain short.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EngineeringTp2GraphGeometryV1 {
    /// Original context64/pages4 wire and allocation contract.
    #[default]
    Short64,
    /// Explicit context2304/pages144 graph-only wire and allocation contract.
    Long2304,
}

impl EngineeringTp2GraphGeometryV1 {
    /// Closed geometry choices.
    pub const ALL: [Self; 2] = [Self::Short64, Self::Long2304];
    /// Exact launch and evidence discriminator.
    #[must_use]
    pub const fn argument(self) -> &'static str {
        match self {
            Self::Short64 => "short64",
            Self::Long2304 => "long2304",
        }
    }
    /// Logical context capacity.
    #[must_use]
    pub const fn context_tokens(self) -> u32 {
        match self {
            Self::Short64 => 64,
            Self::Long2304 => 2304,
        }
    }
    /// Physical pages and the single-row page-table stride, with sixteen tokens per page.
    #[must_use]
    pub const fn pages(self) -> u32 {
        match self {
            Self::Short64 => 4,
            Self::Long2304 => 144,
        }
    }
    /// Number of fixed 128-token split-attention partitions.
    #[must_use]
    pub const fn attention_splits(self) -> u32 {
        match self {
            Self::Short64 => 1,
            Self::Long2304 => 18,
        }
    }
    /// Rank-private FP32 elements, sixteen heads and 192 values per partition.
    #[must_use]
    pub const fn attention_scratch_values(self) -> u64 {
        match self {
            Self::Short64 => 3_072,
            Self::Long2304 => 55_296,
        }
    }
    /// Geometry and arithmetic profile bound into the registered graph bytes.
    #[must_use]
    pub fn program_profile(self, profile: EngineeringTp2GraphKernelProfileV1) -> String {
        let mut value = format!(
            "qwen3-8b-tp2-mfma-baseline-attention-bf16-row1-cap16-context{}-pages{}",
            self.context_tokens(),
            self.pages()
        );
        if profile != EngineeringTp2GraphKernelProfileV1::Baseline {
            value.push('+');
            value.push_str(profile.argument());
        }
        value
    }
}

/// Geometry-bound graph input, separate from the frozen four-page interpreter input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineeringTp2GraphInputV1 {
    /// Immutable registered geometry.
    pub geometry: EngineeringTp2GraphGeometryV1,
    /// Hash returned by registration of the geometry-bound program.
    pub plan_sha256: [u8; 32],
    /// One-based completed-forward generation.
    pub generation: u64,
    /// Zero-based source collective epoch.
    pub epoch: u64,
    /// Exact input token.
    pub token: u32,
    /// Position equal to the epoch in the single-request graph.
    pub position: u32,
    /// Exactly four or 144 entries, as selected by geometry.
    pub page_table: Vec<u32>,
    /// Exactly 256 cosine bytes followed by 256 sine bytes.
    pub cos_sin: Vec<u8>,
}

impl EngineeringTp2GraphInputV1 {
    /// Converts only the exact short geometry to its unchanged legacy input type.
    /// # Errors
    /// Rejects long geometry or a non-four-entry page table.
    pub fn short_input(&self) -> TpResult<Input> {
        if self.geometry != EngineeringTp2GraphGeometryV1::Short64 {
            return Err("long graph input cannot enter the short protocol".into());
        }
        Ok(Input {
            plan_sha256: self.plan_sha256,
            generation: self.generation,
            epoch: self.epoch,
            token: self.token,
            position: self.position,
            page_table: self
                .page_table
                .as_slice()
                .try_into()
                .map_err(|_| "short graph table extent")?,
            cos_sin: self.cos_sin.clone(),
        })
    }
}

impl From<&Input> for EngineeringTp2GraphInputV1 {
    fn from(input: &Input) -> Self {
        Self {
            geometry: EngineeringTp2GraphGeometryV1::Short64,
            plan_sha256: input.plan_sha256,
            generation: input.generation,
            epoch: input.epoch,
            token: input.token,
            position: input.position,
            page_table: input.page_table.to_vec(),
            cos_sin: input.cos_sin.clone(),
        }
    }
}

/// Immutable graph-only BF16 kernel selection with matched sidecar residency.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineeringTp2GraphKernelProfileV1 {
    /// Original main15/peer2 images and scalar argmax.
    Baseline,
    /// Original scalar argmax with the exact V22 image resident on rank zero.
    V22Scalar,
    /// The same resident images, with only the final argmax using V22.
    V22Wave,
    /// V22 argmax with V15 resident on both ranks; other roots remain unchanged.
    WaveStackControl,
    /// Selects V15 only for the 145 hidden-width normalization dispatches.
    WaveStackNorm,
    /// Also selects the 72 main-image Wave64 paged attention dispatches.
    WaveStackNormAttention,
    /// Also selects 144 K/V projections using authenticated original weights.
    WaveStackNormAttentionKv,
    /// Also selects Wave gate/up and FP32 down partials, retaining BF16 boundaries.
    WaveStackNormAttentionKvMlp,
    /// Replaces each full-wave attention with the exact partial and merge roots.
    WaveStackNormSplitAttentionKvMlp,
}

impl EngineeringTp2GraphKernelProfileV1 {
    /// Closed graph arithmetic profiles; observation policy is a separate axis.
    pub const ALL: [Self; 9] = [
        Self::Baseline,
        Self::V22Scalar,
        Self::V22Wave,
        Self::WaveStackControl,
        Self::WaveStackNorm,
        Self::WaveStackNormAttention,
        Self::WaveStackNormAttentionKv,
        Self::WaveStackNormAttentionKvMlp,
        Self::WaveStackNormSplitAttentionKvMlp,
    ];

    /// Stable explicit CLI and performance-metadata discriminator.
    #[must_use]
    pub const fn argument(self) -> &'static str {
        match self {
            Self::Baseline => "baseline",
            Self::V22Scalar => "bf16-argmax-v22-scalar",
            Self::V22Wave => "bf16-argmax-v22-wave",
            Self::WaveStackControl => "wave-stack-control",
            Self::WaveStackNorm => "wave-stack-norm",
            Self::WaveStackNormAttention => "wave-stack-norm-attention",
            Self::WaveStackNormAttentionKv => "wave-stack-norm-attention-kv",
            Self::WaveStackNormAttentionKvMlp => "wave-stack-norm-attention-kv-mlp",
            Self::WaveStackNormSplitAttentionKvMlp => "wave-stack-norm-split-attention-kv-mlp",
        }
    }

    /// Whether the matched rank-zero V22 sidecar is required.
    #[must_use]
    pub const fn has_v22(self) -> bool {
        !matches!(self, Self::Baseline)
    }

    /// Whether the final rank-zero dispatch uses the V22 root.
    #[must_use]
    pub const fn wave_argmax(self) -> bool {
        !matches!(self, Self::Baseline | Self::V22Scalar)
    }

    /// Whether the matched V15 sidecar must be loaded on both ranks.
    #[must_use]
    pub const fn has_v15(self) -> bool {
        matches!(
            self,
            Self::WaveStackControl
                | Self::WaveStackNorm
                | Self::WaveStackNormAttention
                | Self::WaveStackNormAttentionKv
                | Self::WaveStackNormAttentionKvMlp
                | Self::WaveStackNormSplitAttentionKvMlp
        )
    }

    /// Whether hidden-width norms use V15; Q/K head norms remain unchanged.
    #[must_use]
    pub const fn wave_hidden_norm(self) -> bool {
        self.has_v15() && !matches!(self, Self::WaveStackControl)
    }

    /// Whether paged attention uses the existing main-image Wave64 root.
    #[must_use]
    pub const fn wave_attention(self) -> bool {
        matches!(
            self,
            Self::WaveStackNormAttention
                | Self::WaveStackNormAttentionKv
                | Self::WaveStackNormAttentionKvMlp
        )
    }

    /// Whether K/V use the model's original `NxK` rather than retained `KxN` tensors.
    #[must_use]
    pub const fn wave_kv(self) -> bool {
        matches!(
            self,
            Self::WaveStackNormAttentionKv
                | Self::WaveStackNormAttentionKvMlp
                | Self::WaveStackNormSplitAttentionKvMlp
        )
    }

    /// Whether gate/up and down partials also use original-layout Wave kernels.
    #[must_use]
    pub const fn wave_mlp(self) -> bool {
        matches!(
            self,
            Self::WaveStackNormAttentionKvMlp | Self::WaveStackNormSplitAttentionKvMlp
        )
    }

    /// Whether the separate two-root split sidecar and private scratch are required.
    #[must_use]
    pub const fn split_attention(self) -> bool {
        matches!(self, Self::WaveStackNormSplitAttentionKvMlp)
    }

    /// Exact source-level graph cardinality selected by this immutable profile.
    #[must_use]
    pub const fn steps(self) -> usize {
        if self.split_attention() { 1_085 } else { 1_013 }
    }

    /// Ordinary rank dispatches, excluding collective kernels.
    #[must_use]
    pub const fn rank_dispatches(self) -> u32 {
        if self.split_attention() { 1_013 } else { 941 }
    }

    /// Exact per-rank kernel counts, not loaded descriptor counts.
    #[must_use]
    pub const fn kernel_counts(self) -> [u32; 2] {
        if self.split_attention() {
            [652, 649]
        } else {
            KERNELS
        }
    }

    /// Exact per-rank packet counts including unchanged dependency barriers.
    #[must_use]
    pub const fn graph_packet_counts(self) -> [u64; 2] {
        if self.split_attention() {
            [795, 793]
        } else {
            PACKETS
        }
    }

    /// Exact loaded roots on rank zero and rank one, not dispatch counts.
    #[must_use]
    pub const fn loaded_kernel_counts(self) -> [u32; 2] {
        if self.split_attention() {
            [21, 20]
        } else if self.has_v15() {
            [19, 18]
        } else if self.has_v22() {
            [18, 17]
        } else {
            [17, 17]
        }
    }
}

/// Immutable launch-time validation policy; no variant changes GPU arithmetic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineeringTp2GraphPolicyV1 {
    /// Existing queued API with its original per-packet observation boundaries.
    QueuedBaseline,
    /// Separate API with full entry/exit and bounded transaction observations.
    TransactionFences,
    /// Same transaction API with load-owned immutable descriptor admission reuse.
    TransactionFencesAdmissionCache,
    /// Explicit operation-scoped full observations with immutable admission reuse.
    /// Transient changes entirely inside a scope are not equivalence-qualified.
    TransactionFencesAdmissionCacheScopedObservations,
    /// One metadata/graph/readback token scope with actual full entry/exit checks.
    /// Transient changes inside the token are not equivalence-qualified.
    ClosedTokenAdmissionCache,
    /// One exact request with provisional tokens until native completion and close.
    FiniteRequestAdmissionCache,
}

impl EngineeringTp2GraphPolicyV1 {
    /// Original policies retained for their unchanged regression matrices.
    pub const PRE_FINITE: [Self; 5] = [
        Self::QueuedBaseline,
        Self::TransactionFences,
        Self::TransactionFencesAdmissionCache,
        Self::TransactionFencesAdmissionCacheScopedObservations,
        Self::ClosedTokenAdmissionCache,
    ];
    /// All closed observation/admission choices, independent of the kernel profile.
    pub const ALL: [Self; 6] = [
        Self::QueuedBaseline,
        Self::TransactionFences,
        Self::TransactionFencesAdmissionCache,
        Self::TransactionFencesAdmissionCacheScopedObservations,
        Self::ClosedTokenAdmissionCache,
        Self::FiniteRequestAdmissionCache,
    ];

    /// Whether construction must opt into immutable kernel admission reuse.
    #[must_use]
    pub const fn admission_cache(self) -> bool {
        matches!(
            self,
            Self::TransactionFencesAdmissionCache
                | Self::TransactionFencesAdmissionCacheScopedObservations
                | Self::ClosedTokenAdmissionCache
                | Self::FiniteRequestAdmissionCache
        )
    }

    /// Whether the separate native scoped-observation constructor is required.
    #[must_use]
    pub const fn scoped_operation_observations(self) -> bool {
        matches!(
            self,
            Self::TransactionFencesAdmissionCacheScopedObservations
        )
    }

    /// Whether metadata, graph execution and choice readback form one token scope.
    #[must_use]
    pub const fn closed_token(self) -> bool {
        matches!(self, Self::ClosedTokenAdmissionCache)
    }

    /// Whether completion is request-wide and interior token results are provisional.
    #[must_use]
    pub const fn finite_request(self) -> bool {
        matches!(self, Self::FiniteRequestAdmissionCache)
    }

    /// Whether the dedicated native decode-token API is required.
    #[must_use]
    pub const fn decode_token(self) -> bool {
        self.closed_token() || self.finite_request()
    }
}

/// Actual counters returned by the selected native API, never inferred timings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EngineeringTp2GraphPolicyEvidenceV1 {
    /// The transport also validates the complete request receipt before final commit.
    FiniteRequestAdmissionCache {
        /// Actual token full boundaries: zero interior, one on final completion.
        full_boundaries: u32,
        /// Actual named operational graph boundaries.
        graph_operational_boundaries: u32,
        /// Actual named token boundaries.
        token_boundaries: u32,
        /// Actual paired reset-only observations.
        reset_only_rounds: u32,
        /// Actual checkpoint count.
        loop_checks: u64,
        /// Exact monotone native token ordinal.
        token_ordinal: u64,
        /// Interior outputs must not be externally accepted.
        provisional: bool,
    },
    /// The baseline API does not return transaction counters.
    QueuedBaseline,
    /// Counts from the one successfully completed transaction scope.
    TransactionFences {
        /// Actual successful full entry/exit observations.
        full_boundaries: u32,
        /// Actual successful operational observations at named graph boundaries.
        operational_boundaries: u32,
        /// Actual paired reset-only observations.
        reset_only_rounds: u32,
        /// Actual per-slot and boundary checkpoint calls.
        loop_checks: u64,
    },
    /// Same actual transaction observations with an explicitly cached owner.
    TransactionFencesAdmissionCache {
        /// Actual successful full entry/exit observations.
        full_boundaries: u32,
        /// Actual successful operational observations at named graph boundaries.
        operational_boundaries: u32,
        /// Actual paired reset-only observations.
        reset_only_rounds: u32,
        /// Actual per-slot and boundary checkpoint calls.
        loop_checks: u64,
    },
    /// Actual transaction counters from the explicitly scoped native owner.
    TransactionFencesAdmissionCacheScopedObservations {
        /// Actual transaction full-boundary calls, not total topology scans.
        full_boundaries: u32,
        /// Actual operational graph boundaries.
        operational_boundaries: u32,
        /// Actual paired reset-only observations.
        reset_only_rounds: u32,
        /// Actual per-slot and boundary checkpoints.
        loop_checks: u64,
    },
    /// Actual native token counters, distinct from the separate graph API.
    ClosedTokenAdmissionCache {
        /// Actual paired full token entry/exit observations.
        full_boundaries: u32,
        /// Actual operational graph boundaries inside the token.
        graph_operational_boundaries: u32,
        /// Actual named token boundaries, not every nested checkpoint.
        token_boundaries: u32,
        /// Actual paired reset-only graph observations.
        reset_only_rounds: u32,
        /// Actual per-slot and graph boundary checkpoints.
        loop_checks: u64,
    },
}

impl EngineeringTp2GraphPolicyEvidenceV1 {
    fn validate_for_profile(
        &self,
        policy: EngineeringTp2GraphPolicyV1,
        profile: EngineeringTp2GraphKernelProfileV1,
    ) -> TpResult<()> {
        let slots = profile.graph_packet_counts().into_iter().sum::<u64>();
        let first_scan = 6 * slots + 15;
        let later_scan = slots + 4;
        match (policy, self) {
            (
                EngineeringTp2GraphPolicyV1::FiniteRequestAdmissionCache,
                Self::FiniteRequestAdmissionCache {
                    full_boundaries,
                    graph_operational_boundaries,
                    token_boundaries,
                    reset_only_rounds,
                    loop_checks,
                    token_ordinal,
                    provisional,
                },
            ) if *token_ordinal > 0
                && *full_boundaries == u32::from(!provisional)
                && *graph_operational_boundaries == 9
                && *token_boundaries == 4
                && *loop_checks >= first_scan
                && (*loop_checks - first_scan).is_multiple_of(later_scan)
                && u64::from(*reset_only_rounds) <= *loop_checks =>
            {
                Ok(())
            }
            (EngineeringTp2GraphPolicyV1::QueuedBaseline, Self::QueuedBaseline) => Ok(()),
            (
                EngineeringTp2GraphPolicyV1::ClosedTokenAdmissionCache,
                Self::ClosedTokenAdmissionCache {
                    full_boundaries,
                    graph_operational_boundaries,
                    token_boundaries,
                    reset_only_rounds,
                    loop_checks,
                },
            ) if *full_boundaries == 2
                && *graph_operational_boundaries == 9
                && *token_boundaries == 4
                && *loop_checks >= first_scan
                && (*loop_checks - first_scan).is_multiple_of(later_scan)
                && u64::from(*reset_only_rounds) <= *loop_checks =>
            {
                Ok(())
            }
            (
                EngineeringTp2GraphPolicyV1::TransactionFences,
                Self::TransactionFences {
                    full_boundaries,
                    operational_boundaries,
                    reset_only_rounds,
                    loop_checks,
                },
            )
            | (
                EngineeringTp2GraphPolicyV1::TransactionFencesAdmissionCache,
                Self::TransactionFencesAdmissionCache {
                    full_boundaries,
                    operational_boundaries,
                    reset_only_rounds,
                    loop_checks,
                },
            )
            | (
                EngineeringTp2GraphPolicyV1::TransactionFencesAdmissionCacheScopedObservations,
                Self::TransactionFencesAdmissionCacheScopedObservations {
                    full_boundaries,
                    operational_boundaries,
                    reset_only_rounds,
                    loop_checks,
                },
            ) if *full_boundaries == 2
                && *operational_boundaries == 9
                && *loop_checks >= first_scan
                && (*loop_checks - first_scan).is_multiple_of(later_scan)
                && u64::from(*reset_only_rounds) <= *loop_checks =>
            {
                Ok(())
            }
            _ => Err("prepared graph observation policy or actual counter mismatch".into()),
        }
    }
}

/// Final acquired observations for one rank's complete graph reservation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineeringTp2GraphRankV1 {
    /// Physical identity independently checked against the transport's ordered devices.
    pub unique_id: u64,
    /// This fixed-ring profile does not permit queue replacement or rollover.
    pub queue_epoch: u64,
    /// First reserved absolute packet ID.
    pub first_packet: u64,
    /// Exclusive end of the reservation.
    pub next_packet: u64,
    /// Final acquired AQL write index.
    pub final_write: u64,
    /// Final acquired AQL read index.
    pub final_read: u64,
    /// One actual acquired completion value for every reserved packet.
    pub completion_values: Vec<i64>,
}

/// Source-ordered packet bindings, not an independently drained collective receipt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineeringTp2GraphCollectiveBindingV1 {
    /// Exact target-model collective key.
    pub key: Key,
    /// Native request ID, equal to the monotonically increasing collective generation.
    pub request_id: u64,
    /// One-based generation across all completed forward graphs.
    pub generation: u64,
    /// Scratch-reuse dependency barriers; absent only for the first collective.
    pub before_producers: Option<[u64; 2]>,
    /// Actual producer packet IDs in rank order.
    pub producers: [u64; 2],
    /// Dependency barriers waiting for both producers.
    pub after_producers: [u64; 2],
    /// Actual consumer packet IDs in rank order.
    pub consumers: [u64; 2],
}

/// Native graph receipt copied without synthesizing intermediate observations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineeringTp2GraphCompletionV1 {
    /// One-based forward program ID.
    pub program_id: u64,
    /// Existing model collective group ID.
    pub group_id: u64,
    /// Zero-based forward epoch.
    pub epoch: u64,
    /// Ordered physical devices, bound by the transport before construction.
    pub unique_ids: [u64; 2],
    /// Exactly one completed native graph call.
    pub graph_api_calls: u32,
    /// Number of source-level rank/collective steps.
    pub logical_steps: u32,
    /// Number of ordinary rank dispatches, excluding collective kernels.
    pub rank_dispatches: u32,
    /// Kernel dispatch counts in rank order.
    pub kernel_counts: [u32; 2],
    /// Actual Barrier-AND packet counts in rank order.
    pub barrier_counts: [u32; 2],
    /// Total reserved/completed packets in rank order.
    pub packet_counts: [u32; 2],
    /// Actual final rank observations.
    pub ranks: [EngineeringTp2GraphRankV1; 2],
    /// Rank-zero embedding, rank-one dependency barrier, rank-one copy packet IDs.
    pub embedding_copy_packets: [u64; 3],
    /// All 72 actual source-ordered collective schedule bindings.
    pub collectives: Vec<EngineeringTp2GraphCollectiveBindingV1>,
}

/// Separate whole-token completion type for a jointly drained queued graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineeringTp2PreparedGraphReceiptV1 {
    /// Identity of the complete registered immutable program.
    pub plan_sha256: [u8; 32],
    /// One-based forward generation.
    pub generation: u64,
    /// Zero-based source epoch.
    pub epoch: u64,
    /// Input position, identical to the epoch in this bounded profile.
    pub position: u32,
    /// Echo of the input token.
    pub input_token: u32,
    /// Actual GPU argmax readback, not a predicted host result.
    pub output_token: u32,
    /// Actual native joint-drain receipt.
    pub completion: EngineeringTp2GraphCompletionV1,
    /// Actual evidence from the immutable native observation policy.
    pub policy: EngineeringTp2GraphPolicyEvidenceV1,
}

fn advance(frontier: &mut [u64; 2], rank: usize) -> TpResult<u64> {
    let cursor = frontier
        .get_mut(rank)
        .ok_or("prepared graph rank outside TP2")?;
    let previous = *cursor;
    *cursor = previous
        .checked_add(1)
        .ok_or("prepared graph cursor overflow")?;
    Ok(previous)
}

fn advance_pair(frontier: &mut [u64; 2]) -> TpResult<[u64; 2]> {
    Ok([advance(frontier, 0)?, advance(frontier, 1)?])
}

impl EngineeringTp2PreparedGraphReceiptV1 {
    /// Independently checks the complete source-ordered graph before host commit.
    /// Transport validation must first bind worker PID, ordered physical devices,
    /// wire request ID, immutable policy and native implementation identity.
    ///
    /// # Errors
    /// Rejects partial drains, stale identities, reordered bindings, altered
    /// counters or any mismatch with the fixed single-row target program.
    pub fn validate_for(
        &self,
        program: &Program,
        input: &Input,
        policy: EngineeringTp2GraphPolicyV1,
    ) -> TpResult<()> {
        self.validate_graph_input(program, &EngineeringTp2GraphInputV1::from(input), policy)
    }

    /// Validates a geometry-bound graph completion, including complete queue drain.
    /// # Errors
    /// Rejects wrong geometry extents or any incomplete, stale or reordered observation.
    pub fn validate_graph_input(
        &self,
        program: &Program,
        input: &EngineeringTp2GraphInputV1,
        policy: EngineeringTp2GraphPolicyV1,
    ) -> TpResult<()> {
        self.validate_graph_input_for_profile(
            program,
            input,
            policy,
            EngineeringTp2GraphKernelProfileV1::Baseline,
        )
    }

    /// Validates the exact owner-selected arithmetic profile before host commit.
    /// # Errors
    /// Rejects cross-profile counts, incomplete drains, or any stale identity.
    pub fn validate_graph_input_for_profile(
        &self,
        program: &Program,
        input: &EngineeringTp2GraphInputV1,
        policy: EngineeringTp2GraphPolicyV1,
        profile: EngineeringTp2GraphKernelProfileV1,
    ) -> TpResult<()> {
        let native = &self.completion;
        let packets = profile.graph_packet_counts();
        if input.epoch.checked_add(1) != Some(input.generation)
            || u64::from(input.position) != input.epoch
            || input.position >= input.geometry.context_tokens()
            || input.page_table.len()
                != usize::try_from(input.geometry.pages()).map_err(|_| "graph pages bound")?
            || input.token >= 151_936
            || input.cos_sin.len() != 512
            || self.plan_sha256 != input.plan_sha256
            || self.generation != input.generation
            || self.epoch != input.epoch
            || self.position != input.position
            || self.input_token != input.token
            || self.output_token >= 151_936
            || program.steps.len() != profile.steps()
            || native.program_id != input.generation
            || native.group_id != program.group_id
            || native.epoch != input.epoch
            || native.unique_ids.contains(&0)
            || native.unique_ids[0] == native.unique_ids[1]
            || native.graph_api_calls != 1
            || usize::try_from(native.logical_steps).ok() != Some(profile.steps())
            || native.rank_dispatches != profile.rank_dispatches()
            || native.kernel_counts != profile.kernel_counts()
            || native.barrier_counts != BARRIERS
            || native.packet_counts.map(u64::from) != packets
            || native.collectives.len() != 72
        {
            return Err("prepared graph identity or cardinality mismatch".into());
        }
        self.policy.validate_for_profile(policy, profile)?;
        let first = [
            input.epoch.checked_mul(packets[0]),
            input.epoch.checked_mul(packets[1]),
        ];
        let [Some(first0), Some(first1)] = first else {
            return Err("prepared graph first cursor overflow".into());
        };
        let mut frontier = [first0, first1];
        let expected_end = [
            input.generation.checked_mul(packets[0]),
            input.generation.checked_mul(packets[1]),
        ];
        for (rank, actual) in native.ranks.iter().enumerate() {
            if actual.unique_id != native.unique_ids[rank]
                || actual.queue_epoch != 0
                || Some(actual.first_packet) != first[rank]
                || Some(actual.next_packet) != expected_end[rank]
                || actual.final_write != actual.next_packet
                || actual.final_read != actual.next_packet
                || usize::try_from(packets[rank]).ok() != Some(actual.completion_values.len())
                || actual.completion_values.iter().any(|value| *value != 0)
            {
                return Err("prepared graph incomplete actual joint drain".into());
            }
        }
        let copy = first1
            .checked_add(1)
            .ok_or("prepared graph copy cursor overflow")?;
        if native.embedding_copy_packets != [first0, first1, copy] {
            return Err("prepared graph embedding dependency binding mismatch".into());
        }
        let mut collectives = 0usize;
        let mut ordinary = 0u32;
        let mut kernels = [0u32; 2];
        for (index, step) in program.steps.iter().enumerate() {
            match step {
                Step::Rank { rank, dispatch } => {
                    if index == 0
                        && (*rank != 0
                            || dispatch.kernel != "ferric_qwen3_tp_batch_embedding_bf16_v2")
                    {
                        return Err("prepared graph first embedding step mismatch".into());
                    }
                    if index == 1 {
                        if *rank != 1 || dispatch.kernel != "ferric_qwen3_tp_peer_copy_bf16_v4" {
                            return Err("prepared graph peer-copy step mismatch".into());
                        }
                        advance(&mut frontier, 1)?;
                    }
                    let rank =
                        usize::try_from(*rank).map_err(|_| "prepared graph rank conversion")?;
                    advance(&mut frontier, rank)?;
                    kernels[rank] += 1;
                    ordinary += 1;
                }
                Step::Collective(template) => {
                    if index < 2 || collectives >= 72 {
                        return Err("prepared graph collective outside source grammar".into());
                    }
                    template.validate()?;
                    let mut key = template.key;
                    key.epoch = input.epoch;
                    let operation = if collectives.is_multiple_of(2) {
                        Operation::AttentionOutputSum
                    } else {
                        Operation::FeedForwardDownSum
                    };
                    if key.group_id != program.group_id
                        || key.model_role != Qwen3ModelRole::Target8B
                        || template.key.epoch != 0
                        || u32::try_from(collectives / 2).ok() != Some(key.layer)
                        || key.operation != operation
                        || template.rows != 1
                    {
                        return Err("prepared graph collective key or source order mismatch".into());
                    }
                    let generation = input
                        .epoch
                        .checked_mul(72)
                        .and_then(|value| value.checked_add(collectives as u64 + 1))
                        .ok_or("prepared graph collective generation overflow")?;
                    let before = if collectives == 0 {
                        None
                    } else {
                        Some(advance_pair(&mut frontier)?)
                    };
                    let expected = EngineeringTp2GraphCollectiveBindingV1 {
                        key,
                        request_id: generation,
                        generation,
                        before_producers: before,
                        producers: advance_pair(&mut frontier)?,
                        after_producers: advance_pair(&mut frontier)?,
                        consumers: advance_pair(&mut frontier)?,
                    };
                    if native.collectives[collectives] != expected {
                        return Err(
                            "prepared graph actual collective schedule binding mismatch".into()
                        );
                    }
                    for count in &mut kernels {
                        *count += 2;
                    }
                    collectives += 1;
                }
            }
        }
        if ordinary != profile.rank_dispatches()
            || collectives != 72
            || kernels != profile.kernel_counts()
            || frontier.map(Some) != expected_end
        {
            return Err("prepared graph incomplete source schedule".into());
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "prepared_graph_tests.rs"]
mod tests;
