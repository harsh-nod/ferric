//! Deterministic repeated speculative-generation coordination for M1.
//!
//! The physical runtime already executes one K4, K8, or K16 draft/verify graph,
//! checks its compact K7 records, settles target and draft KV writes, and rearms
//! a long-lived queue. This module supplies the request-local loop transition
//! between those physical rounds. It binds an exact round, epoch, selection,
//! and active roster, then turns checked completion semantics into publication,
//! target/draft commit and rollback coordinates, terminal decisions, and the
//! correction-or-bonus anchor used as the next draft-model input.
//!
//! No queue, allocation, KFD, packet, or device authority is represented here.

use core::fmt;

use ferric_spec::completion::CompletionEpoch;
use ferric_spec::{
    Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket, Qwen3PlanSelection, RequestId, TokenId,
    M1_MAX_ACTIVE_SEQUENCES, M1_MAX_COMPLETION_TOKENS, M1_MAX_CONTEXT_TOKENS,
    QWEN3_END_OF_TEXT_TOKEN, QWEN3_IM_END_TOKEN, QWEN3_VOCABULARY_SIZE,
};

use crate::{
    CheckedCompletionSemantics, M1CheckedCompletionOutputV1, M1DeviceKvCompletionDispositionV1,
};

const M1_SPECULATIVE_STOP_TOKEN_CAPACITY_V1: usize = 2;

/// One fixed physical speculative graph shape admitted by M1.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1SpeculativePhysicalShapeV1 {
    selection: Qwen3PlanSelection,
    sequences: u8,
    draft_tokens: u8,
}

impl M1SpeculativePhysicalShapeV1 {
    /// Validates a target-side speculative selection.
    ///
    /// # Errors
    ///
    /// Returns [`M1SpeculativeGenerationLoopErrorV1::UnsupportedSelection`] for
    /// any selection outside K4/S1, K4/S8, K8/S1, or K16/S1.
    pub fn from_selection(
        selection: Qwen3PlanSelection,
    ) -> Result<Self, M1SpeculativeGenerationLoopErrorV1> {
        if selection.role != Qwen3ModelRole::Target8B
            || selection.mode != Qwen3ExecutionMode::Speculative
        {
            return Err(M1SpeculativeGenerationLoopErrorV1::UnsupportedSelection {
                actual: selection,
            });
        }
        let (sequences, draft_tokens) = match selection.bucket {
            Qwen3PlanBucket::SpeculativeS1K4C8192 => (1, 4),
            Qwen3PlanBucket::SpeculativeS8K4C8192 => (8, 4),
            Qwen3PlanBucket::SpeculativeS1K8C8192 => (1, 8),
            Qwen3PlanBucket::SpeculativeS1K16C8192 => (1, 16),
            _ => {
                return Err(M1SpeculativeGenerationLoopErrorV1::UnsupportedSelection {
                    actual: selection,
                });
            }
        };
        Ok(Self {
            selection,
            sequences,
            draft_tokens,
        })
    }

    /// Exact target selection consumed by the physical step builder.
    #[must_use]
    pub const fn selection(self) -> Qwen3PlanSelection {
        self.selection
    }

    /// Maximum live members carried by the selected fixed output shape.
    #[must_use]
    pub const fn sequences(self) -> u8 {
        self.sequences
    }

    /// Exact autoregressive draft width, one of 4, 8, or 16.
    #[must_use]
    pub const fn draft_tokens(self) -> u8 {
        self.draft_tokens
    }
}

/// Bounded per-request publication and stop policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1SpeculativeGenerationPolicyV1 {
    max_output_tokens: u32,
    stop_tokens: [TokenId; M1_SPECULATIVE_STOP_TOKEN_CAPACITY_V1],
    stop_token_count: u8,
}

impl M1SpeculativeGenerationPolicyV1 {
    /// Creates a bounded policy with at most two distinct stop tokens.
    ///
    /// # Errors
    ///
    /// Rejects zero or over-context output limits, too many stop tokens,
    /// duplicate stop tokens, and tokens outside the Qwen3 vocabulary.
    pub fn new(
        max_output_tokens: u32,
        stop_tokens: &[TokenId],
    ) -> Result<Self, M1SpeculativeGenerationLoopErrorV1> {
        if max_output_tokens == 0 || max_output_tokens > M1_MAX_CONTEXT_TOKENS {
            return Err(M1SpeculativeGenerationLoopErrorV1::InvalidOutputLimit {
                actual: max_output_tokens,
            });
        }
        if stop_tokens.len() > M1_SPECULATIVE_STOP_TOKEN_CAPACITY_V1 {
            return Err(M1SpeculativeGenerationLoopErrorV1::TooManyStopTokens {
                maximum: M1_SPECULATIVE_STOP_TOKEN_CAPACITY_V1,
                actual: stop_tokens.len(),
            });
        }
        let mut bounded = [0; M1_SPECULATIVE_STOP_TOKEN_CAPACITY_V1];
        for (index, token) in stop_tokens.iter().copied().enumerate() {
            if token >= QWEN3_VOCABULARY_SIZE {
                return Err(M1SpeculativeGenerationLoopErrorV1::TokenOutOfRange { token });
            }
            if bounded[..index].contains(&token) {
                return Err(M1SpeculativeGenerationLoopErrorV1::DuplicateStopToken { token });
            }
            bounded[index] = token;
        }
        let stop_token_count = u8::try_from(stop_tokens.len()).map_err(|_| {
            M1SpeculativeGenerationLoopErrorV1::TooManyStopTokens {
                maximum: M1_SPECULATIVE_STOP_TOKEN_CAPACITY_V1,
                actual: stop_tokens.len(),
            }
        })?;
        Ok(Self {
            max_output_tokens,
            stop_tokens: bounded,
            stop_token_count,
        })
    }

    /// Standard Qwen3 terminal tokens for one bounded request.
    ///
    /// # Errors
    ///
    /// Rejects an invalid output limit.
    pub fn qwen3(max_output_tokens: u32) -> Result<Self, M1SpeculativeGenerationLoopErrorV1> {
        Self::new(
            max_output_tokens,
            &[QWEN3_END_OF_TEXT_TOKEN, QWEN3_IM_END_TOKEN],
        )
    }

    /// Maximum number of tokens visible to the caller.
    #[must_use]
    pub const fn max_output_tokens(self) -> u32 {
        self.max_output_tokens
    }

    /// Exact configured stop-token prefix.
    #[must_use]
    pub fn stop_tokens(&self) -> &[TokenId] {
        &self.stop_tokens[..usize::from(self.stop_token_count)]
    }

    fn is_stop(self, token: TokenId) -> bool {
        self.stop_tokens().contains(&token)
    }
}

/// Initial request-local state after paired prefill or an equivalent join.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1SpeculativeMemberSeedV1 {
    request: RequestId,
    round_anchor: TokenId,
    target_committed_tokens: u32,
    draft_committed_tokens: u32,
    policy: M1SpeculativeGenerationPolicyV1,
}

impl M1SpeculativeMemberSeedV1 {
    /// Declares one active request and the exact pre-round role cursors.
    #[must_use]
    pub const fn new(
        request: RequestId,
        round_anchor: TokenId,
        target_committed_tokens: u32,
        draft_committed_tokens: u32,
        policy: M1SpeculativeGenerationPolicyV1,
    ) -> Self {
        Self {
            request,
            round_anchor,
            target_committed_tokens,
            draft_committed_tokens,
            policy,
        }
    }
}

/// Stable terminal cause selected after checked target verification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1SpeculativeTerminalReasonV1 {
    /// A published token matched the request's configured stop set.
    StopToken { token: TokenId },
    /// Publication reached the request's exact output limit.
    OutputLimit,
}

/// Stable caller-supplied cancellation class.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1SpeculativeCancellationReasonV1 {
    Client,
    Deadline,
    ServerShutdown,
}

/// Current request-local loop lifecycle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1SpeculativeMemberStatusV1 {
    Active,
    Completed(M1SpeculativeTerminalReasonV1),
    Cancelled(M1SpeculativeCancellationReasonV1),
}

/// Copy-only request state visible to serving integration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1SpeculativeMemberSnapshotV1 {
    request: RequestId,
    status: M1SpeculativeMemberStatusV1,
    next_anchor: TokenId,
    target_committed_tokens: u32,
    draft_committed_tokens: u32,
    generated_tokens: u32,
}

impl M1SpeculativeMemberSnapshotV1 {
    #[must_use]
    pub const fn request(self) -> RequestId {
        self.request
    }

    #[must_use]
    pub const fn status(self) -> M1SpeculativeMemberStatusV1 {
        self.status
    }

    /// Correction or bonus token to consume at ordinal zero of the next round.
    #[must_use]
    pub const fn next_anchor(self) -> TokenId {
        self.next_anchor
    }

    #[must_use]
    pub const fn target_committed_tokens(self) -> u32 {
        self.target_committed_tokens
    }

    #[must_use]
    pub const fn draft_committed_tokens(self) -> u32 {
        self.draft_committed_tokens
    }

    #[must_use]
    pub const fn generated_tokens(self) -> u32 {
        self.generated_tokens
    }
}

#[derive(Debug)]
struct M1SpeculativeMemberStateV1 {
    request: RequestId,
    policy: M1SpeculativeGenerationPolicyV1,
    status: M1SpeculativeMemberStatusV1,
    next_anchor: TokenId,
    target_committed_tokens: u32,
    draft_committed_tokens: u32,
    generated_tokens: u32,
}

impl M1SpeculativeMemberStateV1 {
    const fn snapshot(&self) -> M1SpeculativeMemberSnapshotV1 {
        M1SpeculativeMemberSnapshotV1 {
            request: self.request,
            status: self.status,
            next_anchor: self.next_anchor,
            target_committed_tokens: self.target_committed_tokens,
            draft_committed_tokens: self.draft_committed_tokens,
            generated_tokens: self.generated_tokens,
        }
    }
}

/// Exact input coordinates for one active member in a bound physical round.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1SpeculativeRoundMemberInputV1 {
    request: RequestId,
    round_anchor: TokenId,
    target_pre_committed: u32,
    draft_pre_committed: u32,
}

impl M1SpeculativeRoundMemberInputV1 {
    #[must_use]
    pub const fn request(self) -> RequestId {
        self.request
    }

    #[must_use]
    pub const fn round_anchor(self) -> TokenId {
        self.round_anchor
    }

    #[must_use]
    pub const fn target_pre_committed(self) -> u32 {
        self.target_pre_committed
    }

    #[must_use]
    pub const fn draft_pre_committed(self) -> u32 {
        self.draft_pre_committed
    }
}

/// Inert exact binding handed to physical scheduling for one loop round.
#[must_use = "a bound round must be completed or retained unchanged"]
#[derive(Debug)]
pub struct M1SpeculativeRoundBindingV1 {
    shape: M1SpeculativePhysicalShapeV1,
    round: u64,
    epoch: CompletionEpoch,
    members: Box<[M1SpeculativeRoundMemberInputV1]>,
}

impl M1SpeculativeRoundBindingV1 {
    #[must_use]
    pub const fn shape(&self) -> M1SpeculativePhysicalShapeV1 {
        self.shape
    }

    #[must_use]
    pub const fn round(&self) -> u64 {
        self.round
    }

    #[must_use]
    pub const fn epoch(&self) -> CompletionEpoch {
        self.epoch
    }

    #[must_use]
    pub fn members(&self) -> &[M1SpeculativeRoundMemberInputV1] {
        &self.members
    }
}

/// Caller decision applied after the already-dispatched round completes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1SpeculativeMemberControlActionV1 {
    Continue,
    Cancel(M1SpeculativeCancellationReasonV1),
}

/// Request-bound control in exact scheduler-roster order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1SpeculativeMemberControlV1 {
    request: RequestId,
    action: M1SpeculativeMemberControlActionV1,
}

impl M1SpeculativeMemberControlV1 {
    #[must_use]
    pub const fn continuing(request: RequestId) -> Self {
        Self {
            request,
            action: M1SpeculativeMemberControlActionV1::Continue,
        }
    }

    #[must_use]
    pub const fn cancelling(request: RequestId, reason: M1SpeculativeCancellationReasonV1) -> Self {
        Self {
            request,
            action: M1SpeculativeMemberControlActionV1::Cancel(reason),
        }
    }
}

/// Bounded token prefix from one checked K7 member record.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1SpeculativeTokenBlockV1 {
    tokens: [TokenId; M1_MAX_COMPLETION_TOKENS],
    count: u8,
}

impl M1SpeculativeTokenBlockV1 {
    const fn empty() -> Self {
        Self {
            tokens: [0; M1_MAX_COMPLETION_TOKENS],
            count: 0,
        }
    }

    fn from_slice(tokens: &[TokenId]) -> Result<Self, M1SpeculativeGenerationLoopErrorV1> {
        if tokens.len() > M1_MAX_COMPLETION_TOKENS {
            return Err(M1SpeculativeGenerationLoopErrorV1::EmittedCount {
                lane: 0,
                expected: M1_MAX_COMPLETION_TOKENS,
                actual: tokens.len(),
            });
        }
        let mut block = Self::empty();
        block.tokens[..tokens.len()].copy_from_slice(tokens);
        block.count = u8::try_from(tokens.len()).map_err(|_| {
            M1SpeculativeGenerationLoopErrorV1::EmittedCount {
                lane: 0,
                expected: M1_MAX_COMPLETION_TOKENS,
                actual: tokens.len(),
            }
        })?;
        Ok(block)
    }

    #[must_use]
    pub fn tokens(&self) -> &[TokenId] {
        &self.tokens[..usize::from(self.count)]
    }

    #[must_use]
    pub const fn len(self) -> u8 {
        self.count
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.count == 0
    }
}

/// Whether target verification produced a mismatch correction or full-match bonus.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1SpeculativeVerificationChoiceV1 {
    Correction { token: TokenId },
    Bonus { token: TokenId },
}

impl M1SpeculativeVerificationChoiceV1 {
    #[must_use]
    pub const fn token(self) -> TokenId {
        match self {
            Self::Correction { token } | Self::Bonus { token } => token,
        }
    }
}

/// One role's exact tentative interval settlement.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1SpeculativeKvRoleSettlementV1 {
    role: Qwen3ModelRole,
    pre_committed: u32,
    tentative_end: u32,
    commit_end: u32,
    rollback_tokens: u8,
}

impl M1SpeculativeKvRoleSettlementV1 {
    #[must_use]
    pub const fn role(self) -> Qwen3ModelRole {
        self.role
    }

    #[must_use]
    pub const fn pre_committed(self) -> u32 {
        self.pre_committed
    }

    #[must_use]
    pub const fn tentative_end(self) -> u32 {
        self.tentative_end
    }

    #[must_use]
    pub const fn commit_end(self) -> u32 {
        self.commit_end
    }

    #[must_use]
    pub const fn rollback_tokens(self) -> u8 {
        self.rollback_tokens
    }
}

/// Complete deterministic result for one member in one physical round.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1SpeculativeMemberRoundOutcomeV1 {
    request: RequestId,
    accepted_draft_tokens: u8,
    raw_emitted: M1SpeculativeTokenBlockV1,
    published: M1SpeculativeTokenBlockV1,
    verification_choice: M1SpeculativeVerificationChoiceV1,
    target_settlement: M1SpeculativeKvRoleSettlementV1,
    draft_settlement: M1SpeculativeKvRoleSettlementV1,
    status: M1SpeculativeMemberStatusV1,
    physical_disposition: M1DeviceKvCompletionDispositionV1,
    next_draft_anchor: Option<TokenId>,
}

impl M1SpeculativeMemberRoundOutcomeV1 {
    #[must_use]
    pub const fn request(self) -> RequestId {
        self.request
    }

    #[must_use]
    pub const fn accepted_draft_tokens(self) -> u8 {
        self.accepted_draft_tokens
    }

    /// Checked accepted-prefix tokens, excluding the correction or bonus.
    #[must_use]
    pub fn accepted_prefix_tokens(&self) -> &[TokenId] {
        &self.raw_emitted.tokens()[..usize::from(self.accepted_draft_tokens)]
    }

    /// Full checked K7 output before cancellation, EOS, or output-limit policy.
    #[must_use]
    pub const fn raw_emitted(self) -> M1SpeculativeTokenBlockV1 {
        self.raw_emitted
    }

    /// Exact caller-visible prefix after terminal and cancellation policy.
    #[must_use]
    pub const fn published(self) -> M1SpeculativeTokenBlockV1 {
        self.published
    }

    #[must_use]
    pub const fn verification_choice(self) -> M1SpeculativeVerificationChoiceV1 {
        self.verification_choice
    }

    #[must_use]
    pub const fn target_settlement(self) -> M1SpeculativeKvRoleSettlementV1 {
        self.target_settlement
    }

    #[must_use]
    pub const fn draft_settlement(self) -> M1SpeculativeKvRoleSettlementV1 {
        self.draft_settlement
    }

    #[must_use]
    pub const fn status(self) -> M1SpeculativeMemberStatusV1 {
        self.status
    }

    /// Existing physical completion disposition for dynamic rearm integration.
    #[must_use]
    pub const fn physical_disposition(self) -> M1DeviceKvCompletionDispositionV1 {
        self.physical_disposition
    }

    /// Correction or bonus to feed both model roles at the next round anchor.
    #[must_use]
    pub const fn next_draft_anchor(self) -> Option<TokenId> {
        self.next_draft_anchor
    }
}

/// Whole-roster result after atomic coordinator state mutation.
#[must_use = "round publication and next-roster state must be consumed"]
#[derive(Debug)]
pub struct M1SpeculativeRoundOutcomeV1 {
    selection: Qwen3PlanSelection,
    completed_round: u64,
    completed_epoch: CompletionEpoch,
    members: Box<[M1SpeculativeMemberRoundOutcomeV1]>,
    next_active_roster: Box<[RequestId]>,
}

impl M1SpeculativeRoundOutcomeV1 {
    #[must_use]
    pub const fn selection(&self) -> Qwen3PlanSelection {
        self.selection
    }

    #[must_use]
    pub const fn completed_round(&self) -> u64 {
        self.completed_round
    }

    #[must_use]
    pub const fn completed_epoch(&self) -> CompletionEpoch {
        self.completed_epoch
    }

    #[must_use]
    pub fn members(&self) -> &[M1SpeculativeMemberRoundOutcomeV1] {
        &self.members
    }

    #[must_use]
    pub fn next_active_roster(&self) -> &[RequestId] {
        &self.next_active_roster
    }
}

#[derive(Clone, Copy, Debug)]
struct M1SpeculativePreparedMemberUpdateV1 {
    member_index: usize,
    generated_tokens: u32,
}

/// Checked whole-roster transition awaiting physical completion.
///
/// Serving integration may borrow the derived member outcomes to select exact
/// physical Continue/Retire dispositions. It should commit this permit only
/// after the physical completion and target/draft KV settlement succeed.
#[must_use = "a checked speculative transition must be committed or retained for retry"]
#[derive(Debug)]
pub struct M1SpeculativePreflightedRoundV1 {
    binding: M1SpeculativeRoundBindingV1,
    updates: Box<[M1SpeculativePreparedMemberUpdateV1]>,
    outcomes: Box<[M1SpeculativeMemberRoundOutcomeV1]>,
    next_active_roster: Box<[RequestId]>,
}

impl M1SpeculativePreflightedRoundV1 {
    #[must_use]
    pub const fn selection(&self) -> Qwen3PlanSelection {
        self.binding.shape.selection()
    }

    #[must_use]
    pub const fn round(&self) -> u64 {
        self.binding.round
    }

    #[must_use]
    pub const fn epoch(&self) -> CompletionEpoch {
        self.binding.epoch
    }

    /// Checked per-member publication, KV settlement, and disposition results.
    #[must_use]
    pub fn members(&self) -> &[M1SpeculativeMemberRoundOutcomeV1] {
        &self.outcomes
    }

    /// Roster that becomes active if this permit commits.
    #[must_use]
    pub fn next_active_roster(&self) -> &[RequestId] {
        &self.next_active_roster
    }
}

/// Retry-safe rejection retaining an unchanged checked round permit.
#[must_use = "a rejected checked transition remains available for retry"]
#[derive(Debug)]
pub struct M1SpeculativePreparedRoundCommitFailureV1 {
    error: M1SpeculativeGenerationLoopErrorV1,
    preflighted: M1SpeculativePreflightedRoundV1,
}

impl M1SpeculativePreparedRoundCommitFailureV1 {
    #[must_use]
    pub const fn error(&self) -> &M1SpeculativeGenerationLoopErrorV1 {
        &self.error
    }

    /// Recovers both the diagnostic and unchanged checked transition.
    pub fn into_parts(
        self,
    ) -> (
        M1SpeculativeGenerationLoopErrorV1,
        M1SpeculativePreflightedRoundV1,
    ) {
        (self.error, self.preflighted)
    }
}

/// Stable fail-closed coordinator diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum M1SpeculativeGenerationLoopErrorV1 {
    UnsupportedSelection {
        actual: Qwen3PlanSelection,
    },
    InvalidOutputLimit {
        actual: u32,
    },
    TooManyStopTokens {
        maximum: usize,
        actual: usize,
    },
    DuplicateStopToken {
        token: TokenId,
    },
    TokenOutOfRange {
        token: TokenId,
    },
    EmptyRoster,
    RosterCapacity {
        maximum: usize,
        actual: usize,
    },
    InvalidRequest {
        lane: usize,
        request: RequestId,
    },
    DuplicateRequest {
        first_lane: usize,
        lane: usize,
    },
    ContextExceeded {
        lane: usize,
        role: Qwen3ModelRole,
    },
    NoActiveMembers,
    RoundDrift {
        expected: u64,
        actual: u64,
    },
    RoundExhausted,
    ZeroEpoch,
    EpochSequence {
        expected: u64,
        actual: u64,
    },
    RosterCount {
        expected: usize,
        actual: usize,
    },
    RosterOrder {
        lane: usize,
        expected: RequestId,
        actual: RequestId,
    },
    BindingMemberState {
        lane: usize,
        request: RequestId,
    },
    SelectionDrift {
        expected: Qwen3PlanSelection,
        actual: Qwen3PlanSelection,
    },
    EpochDrift {
        expected: CompletionEpoch,
        actual: CompletionEpoch,
    },
    CompletionCount {
        expected: usize,
        actual: usize,
    },
    CompletionRequest {
        lane: usize,
        expected: RequestId,
        actual: RequestId,
    },
    CompletionSemantics {
        lane: usize,
    },
    AcceptedCount {
        lane: usize,
        maximum: u8,
        actual: u8,
    },
    EmittedCount {
        lane: usize,
        expected: usize,
        actual: usize,
    },
    CorrectionOrBonus {
        lane: usize,
        expected: TokenId,
        actual: TokenId,
    },
    ControlCount {
        expected: usize,
        actual: usize,
    },
    ControlRequest {
        lane: usize,
        expected: RequestId,
        actual: RequestId,
    },
    HostAllocation,
}

impl fmt::Display for M1SpeculativeGenerationLoopErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "M1 speculative generation loop rejected: {self:?}"
        )
    }
}

impl std::error::Error for M1SpeculativeGenerationLoopErrorV1 {}

/// Host-only repeated-round coordinator over already-checked physical outputs.
#[derive(Debug)]
pub struct M1SpeculativeGenerationLoopV1 {
    shape: M1SpeculativePhysicalShapeV1,
    members: Vec<M1SpeculativeMemberStateV1>,
    next_round: u64,
    last_epoch: Option<CompletionEpoch>,
}

impl M1SpeculativeGenerationLoopV1 {
    /// Creates one deterministic loop in caller-supplied scheduler order.
    ///
    /// # Errors
    ///
    /// Rejects unsupported shapes, empty/oversized/duplicate rosters, invalid
    /// request generations, tokens, or role cursors that cannot fit one full
    /// selected speculative graph.
    pub fn new(
        selection: Qwen3PlanSelection,
        seeds: &[M1SpeculativeMemberSeedV1],
    ) -> Result<Self, M1SpeculativeGenerationLoopErrorV1> {
        let shape = M1SpeculativePhysicalShapeV1::from_selection(selection)?;
        if seeds.is_empty() {
            return Err(M1SpeculativeGenerationLoopErrorV1::EmptyRoster);
        }
        let capacity = usize::from(shape.sequences());
        if seeds.len() > capacity {
            return Err(M1SpeculativeGenerationLoopErrorV1::RosterCapacity {
                maximum: capacity,
                actual: seeds.len(),
            });
        }
        let mut members = Vec::new();
        members
            .try_reserve_exact(seeds.len())
            .map_err(|_| M1SpeculativeGenerationLoopErrorV1::HostAllocation)?;
        for (lane, seed) in seeds.iter().copied().enumerate() {
            validate_seed(shape, seeds, lane, seed)?;
            members.push(M1SpeculativeMemberStateV1 {
                request: seed.request,
                policy: seed.policy,
                status: M1SpeculativeMemberStatusV1::Active,
                next_anchor: seed.round_anchor,
                target_committed_tokens: seed.target_committed_tokens,
                draft_committed_tokens: seed.draft_committed_tokens,
                generated_tokens: 0,
            });
        }
        Ok(Self {
            shape,
            members,
            next_round: 0,
            last_epoch: None,
        })
    }

    #[must_use]
    pub const fn shape(&self) -> M1SpeculativePhysicalShapeV1 {
        self.shape
    }

    #[must_use]
    pub const fn next_round(&self) -> u64 {
        self.next_round
    }

    #[must_use]
    pub const fn last_epoch(&self) -> Option<CompletionEpoch> {
        self.last_epoch
    }

    /// Returns one member snapshot without exposing mutable coordinator state.
    #[must_use]
    pub fn member(&self, request: RequestId) -> Option<M1SpeculativeMemberSnapshotV1> {
        self.members
            .iter()
            .find(|member| member.request == request)
            .map(M1SpeculativeMemberStateV1::snapshot)
    }

    /// Current active roster in original scheduler order.
    #[must_use]
    pub fn active_roster(&self) -> Vec<RequestId> {
        self.members
            .iter()
            .filter(|member| member.status == M1SpeculativeMemberStatusV1::Active)
            .map(|member| member.request)
            .collect()
    }

    /// Binds the exact next physical round without mutating coordinator state.
    ///
    /// # Errors
    ///
    /// Rejects stale/skipped rounds or epochs, empty or reordered rosters, and
    /// any active role cursor that cannot fit the selected full graph width.
    pub fn bind_round(
        &self,
        round: u64,
        epoch: CompletionEpoch,
        roster: &[RequestId],
    ) -> Result<M1SpeculativeRoundBindingV1, M1SpeculativeGenerationLoopErrorV1> {
        if round != self.next_round {
            return Err(M1SpeculativeGenerationLoopErrorV1::RoundDrift {
                expected: self.next_round,
                actual: round,
            });
        }
        if self.next_round == u64::MAX {
            return Err(M1SpeculativeGenerationLoopErrorV1::RoundExhausted);
        }
        if epoch.value() == 0 {
            return Err(M1SpeculativeGenerationLoopErrorV1::ZeroEpoch);
        }
        if let Some(last) = self.last_epoch {
            let expected = last
                .value()
                .checked_add(1)
                .ok_or(M1SpeculativeGenerationLoopErrorV1::RoundExhausted)?;
            if epoch.value() != expected {
                return Err(M1SpeculativeGenerationLoopErrorV1::EpochSequence {
                    expected,
                    actual: epoch.value(),
                });
            }
        }
        let active = self
            .members
            .iter()
            .filter(|member| member.status == M1SpeculativeMemberStatusV1::Active);
        let active_count = active.clone().count();
        if active_count == 0 {
            return Err(M1SpeculativeGenerationLoopErrorV1::NoActiveMembers);
        }
        if roster.len() != active_count {
            return Err(M1SpeculativeGenerationLoopErrorV1::RosterCount {
                expected: active_count,
                actual: roster.len(),
            });
        }
        let mut inputs = Vec::new();
        inputs
            .try_reserve_exact(active_count)
            .map_err(|_| M1SpeculativeGenerationLoopErrorV1::HostAllocation)?;
        for (lane, (member, actual)) in active.zip(roster.iter().copied()).enumerate() {
            if member.request != actual {
                return Err(M1SpeculativeGenerationLoopErrorV1::RosterOrder {
                    lane,
                    expected: member.request,
                    actual,
                });
            }
            validate_context(self.shape, lane, member)?;
            inputs.push(M1SpeculativeRoundMemberInputV1 {
                request: member.request,
                round_anchor: member.next_anchor,
                target_pre_committed: member.target_committed_tokens,
                draft_pre_committed: member.draft_committed_tokens,
            });
        }
        Ok(M1SpeculativeRoundBindingV1 {
            shape: self.shape,
            round,
            epoch,
            members: inputs.into_boxed_slice(),
        })
    }

    /// Preflights one checked physical completion without changing loop state.
    ///
    /// All selection, epoch, roster, semantics, token-shape, and control checks
    /// finish for the complete batch. The returned permit exposes exact
    /// per-member physical dispositions and remains inert until committed.
    ///
    /// # Errors
    ///
    /// Returns a stable drift or semantic diagnostic without mutation.
    pub fn preflight_checked_round(
        &self,
        binding: M1SpeculativeRoundBindingV1,
        checked: &M1CheckedCompletionOutputV1,
        controls: &[M1SpeculativeMemberControlV1],
    ) -> Result<M1SpeculativePreflightedRoundV1, M1SpeculativeGenerationLoopErrorV1> {
        let mut observations = Vec::new();
        observations
            .try_reserve_exact(checked.records().len())
            .map_err(|_| M1SpeculativeGenerationLoopErrorV1::HostAllocation)?;
        for (lane, checked_record) in checked.records().iter().enumerate() {
            let record = checked_record.record();
            let count = usize::from(record.emitted_token_count);
            let tokens = record.emitted_tokens.get(..count).ok_or(
                M1SpeculativeGenerationLoopErrorV1::EmittedCount {
                    lane,
                    expected: M1_MAX_COMPLETION_TOKENS,
                    actual: count,
                },
            )?;
            let emitted = M1SpeculativeTokenBlockV1::from_slice(tokens)
                .map_err(|error| with_emitted_lane(error, lane))?;
            observations.push(CheckedMemberObservationV1 {
                request: record.request,
                semantics: checked_record.semantics(),
                emitted,
            });
        }
        self.preflight_observed_round(
            binding,
            checked.selection(),
            checked.epoch(),
            &observations,
            controls,
        )
    }

    /// Applies one semantically checked completion immediately.
    ///
    /// Physical integration should instead use [`Self::preflight_checked_round`],
    /// complete the physical KV transition from the permit's dispositions, and
    /// then call [`Self::commit_preflighted_round`]. This convenience method is
    /// intended for already-settled or host-only composition.
    ///
    /// # Errors
    ///
    /// Returns a stable drift or semantic diagnostic without partial mutation.
    pub fn complete_checked_round(
        &mut self,
        binding: M1SpeculativeRoundBindingV1,
        checked: &M1CheckedCompletionOutputV1,
        controls: &[M1SpeculativeMemberControlV1],
    ) -> Result<M1SpeculativeRoundOutcomeV1, M1SpeculativeGenerationLoopErrorV1> {
        let preflighted = self.preflight_checked_round(binding, checked, controls)?;
        self.commit_preflighted_round(preflighted)
            .map_err(|failure| failure.into_parts().0)
    }

    /// Commits a checked round after its physical KV transition succeeds.
    ///
    /// This stage performs no allocation. It validates that the coordinator
    /// still names the permit's exact active state before applying every member
    /// update and advancing the round/epoch once.
    ///
    /// # Errors
    ///
    /// Returns the unchanged permit when coordinator state drifted after
    /// preflight.
    pub fn commit_preflighted_round(
        &mut self,
        preflighted: M1SpeculativePreflightedRoundV1,
    ) -> Result<M1SpeculativeRoundOutcomeV1, Box<M1SpeculativePreparedRoundCommitFailureV1>> {
        if let Err(error) = validate_binding(self, &preflighted.binding) {
            return Err(Box::new(M1SpeculativePreparedRoundCommitFailureV1 {
                error,
                preflighted,
            }));
        }
        let M1SpeculativePreflightedRoundV1 {
            binding,
            updates,
            outcomes,
            next_active_roster,
        } = preflighted;
        for (update, outcome) in updates.iter().zip(outcomes.iter()) {
            let member = &mut self.members[update.member_index];
            member.status = outcome.status;
            member.next_anchor = outcome.verification_choice.token();
            member.target_committed_tokens = outcome.target_settlement.commit_end;
            member.draft_committed_tokens = outcome.draft_settlement.commit_end;
            member.generated_tokens = update.generated_tokens;
        }
        let completed_round = self.next_round;
        self.next_round += 1;
        self.last_epoch = Some(binding.epoch);
        Ok(M1SpeculativeRoundOutcomeV1 {
            selection: self.shape.selection(),
            completed_round,
            completed_epoch: binding.epoch,
            members: outcomes,
            next_active_roster,
        })
    }

    fn preflight_observed_round(
        &self,
        binding: M1SpeculativeRoundBindingV1,
        selection: Qwen3PlanSelection,
        epoch: CompletionEpoch,
        observations: &[CheckedMemberObservationV1],
        controls: &[M1SpeculativeMemberControlV1],
    ) -> Result<M1SpeculativePreflightedRoundV1, M1SpeculativeGenerationLoopErrorV1> {
        validate_binding(self, &binding)?;
        if selection != self.shape.selection() {
            return Err(M1SpeculativeGenerationLoopErrorV1::SelectionDrift {
                expected: self.shape.selection(),
                actual: selection,
            });
        }
        if epoch != binding.epoch {
            return Err(M1SpeculativeGenerationLoopErrorV1::EpochDrift {
                expected: binding.epoch,
                actual: epoch,
            });
        }
        if observations.len() != binding.members.len() {
            return Err(M1SpeculativeGenerationLoopErrorV1::CompletionCount {
                expected: binding.members.len(),
                actual: observations.len(),
            });
        }
        if controls.len() != binding.members.len() {
            return Err(M1SpeculativeGenerationLoopErrorV1::ControlCount {
                expected: binding.members.len(),
                actual: controls.len(),
            });
        }

        let mut prepared = Vec::new();
        prepared
            .try_reserve_exact(binding.members.len())
            .map_err(|_| M1SpeculativeGenerationLoopErrorV1::HostAllocation)?;
        for lane in 0..binding.members.len() {
            let input = binding.members[lane];
            let observation = observations[lane];
            let control = controls[lane];
            if observation.request != input.request {
                return Err(M1SpeculativeGenerationLoopErrorV1::CompletionRequest {
                    lane,
                    expected: input.request,
                    actual: observation.request,
                });
            }
            if control.request != input.request {
                return Err(M1SpeculativeGenerationLoopErrorV1::ControlRequest {
                    lane,
                    expected: input.request,
                    actual: control.request,
                });
            }
            let Some(member_index) = self
                .members
                .iter()
                .position(|member| member.request == input.request)
            else {
                return Err(M1SpeculativeGenerationLoopErrorV1::BindingMemberState {
                    lane,
                    request: input.request,
                });
            };
            let member = &self.members[member_index];
            prepared.push(prepare_member(
                self.shape,
                lane,
                member_index,
                member,
                &observation,
                control.action,
            )?);
        }

        let mut updates = Vec::new();
        updates
            .try_reserve_exact(prepared.len())
            .map_err(|_| M1SpeculativeGenerationLoopErrorV1::HostAllocation)?;
        let mut outcomes = Vec::new();
        outcomes
            .try_reserve_exact(prepared.len())
            .map_err(|_| M1SpeculativeGenerationLoopErrorV1::HostAllocation)?;
        for prepared_member in prepared {
            updates.push(M1SpeculativePreparedMemberUpdateV1 {
                member_index: prepared_member.member_index,
                generated_tokens: prepared_member.generated_tokens,
            });
            outcomes.push(prepared_member.outcome);
        }
        let mut next_active_roster = Vec::new();
        next_active_roster
            .try_reserve_exact(self.members.len())
            .map_err(|_| M1SpeculativeGenerationLoopErrorV1::HostAllocation)?;
        for outcome in &outcomes {
            if outcome.status == M1SpeculativeMemberStatusV1::Active {
                next_active_roster.push(outcome.request);
            }
        }
        Ok(M1SpeculativePreflightedRoundV1 {
            binding,
            updates: updates.into_boxed_slice(),
            outcomes: outcomes.into_boxed_slice(),
            next_active_roster: next_active_roster.into_boxed_slice(),
        })
    }
}

#[derive(Clone, Copy, Debug)]
struct CheckedMemberObservationV1 {
    request: RequestId,
    semantics: CheckedCompletionSemantics,
    emitted: M1SpeculativeTokenBlockV1,
}

#[derive(Debug)]
struct PreparedMemberV1 {
    member_index: usize,
    generated_tokens: u32,
    outcome: M1SpeculativeMemberRoundOutcomeV1,
}

fn validate_seed(
    shape: M1SpeculativePhysicalShapeV1,
    seeds: &[M1SpeculativeMemberSeedV1],
    lane: usize,
    seed: M1SpeculativeMemberSeedV1,
) -> Result<(), M1SpeculativeGenerationLoopErrorV1> {
    if seed.request.generation() == 0 || seed.request.slot() >= M1_MAX_ACTIVE_SEQUENCES {
        return Err(M1SpeculativeGenerationLoopErrorV1::InvalidRequest {
            lane,
            request: seed.request,
        });
    }
    if let Some(first_lane) = seeds[..lane]
        .iter()
        .position(|prior| prior.request == seed.request)
    {
        return Err(M1SpeculativeGenerationLoopErrorV1::DuplicateRequest { first_lane, lane });
    }
    if seed.round_anchor >= QWEN3_VOCABULARY_SIZE {
        return Err(M1SpeculativeGenerationLoopErrorV1::TokenOutOfRange {
            token: seed.round_anchor,
        });
    }
    let state = M1SpeculativeMemberStateV1 {
        request: seed.request,
        policy: seed.policy,
        status: M1SpeculativeMemberStatusV1::Active,
        next_anchor: seed.round_anchor,
        target_committed_tokens: seed.target_committed_tokens,
        draft_committed_tokens: seed.draft_committed_tokens,
        generated_tokens: 0,
    };
    validate_context(shape, lane, &state)
}

fn validate_context(
    shape: M1SpeculativePhysicalShapeV1,
    lane: usize,
    member: &M1SpeculativeMemberStateV1,
) -> Result<(), M1SpeculativeGenerationLoopErrorV1> {
    let draft_width = u32::from(shape.draft_tokens());
    let target_width = draft_width + 1;
    if member.target_committed_tokens > M1_MAX_CONTEXT_TOKENS - target_width {
        return Err(M1SpeculativeGenerationLoopErrorV1::ContextExceeded {
            lane,
            role: Qwen3ModelRole::Target8B,
        });
    }
    if member.draft_committed_tokens > M1_MAX_CONTEXT_TOKENS - draft_width {
        return Err(M1SpeculativeGenerationLoopErrorV1::ContextExceeded {
            lane,
            role: Qwen3ModelRole::Draft06B,
        });
    }
    Ok(())
}

fn validate_binding(
    coordinator: &M1SpeculativeGenerationLoopV1,
    binding: &M1SpeculativeRoundBindingV1,
) -> Result<(), M1SpeculativeGenerationLoopErrorV1> {
    if binding.shape != coordinator.shape {
        return Err(M1SpeculativeGenerationLoopErrorV1::SelectionDrift {
            expected: coordinator.shape.selection(),
            actual: binding.shape.selection(),
        });
    }
    if binding.round != coordinator.next_round {
        return Err(M1SpeculativeGenerationLoopErrorV1::RoundDrift {
            expected: coordinator.next_round,
            actual: binding.round,
        });
    }
    let active = coordinator
        .members
        .iter()
        .filter(|member| member.status == M1SpeculativeMemberStatusV1::Active);
    let active_count = active.clone().count();
    if binding.members.len() != active_count {
        return Err(M1SpeculativeGenerationLoopErrorV1::RosterCount {
            expected: active_count,
            actual: binding.members.len(),
        });
    }
    for (lane, (member, input)) in active.zip(binding.members.iter()).enumerate() {
        if member.request != input.request {
            return Err(M1SpeculativeGenerationLoopErrorV1::RosterOrder {
                lane,
                expected: member.request,
                actual: input.request,
            });
        }
        if member.next_anchor != input.round_anchor
            || member.target_committed_tokens != input.target_pre_committed
            || member.draft_committed_tokens != input.draft_pre_committed
        {
            return Err(M1SpeculativeGenerationLoopErrorV1::BindingMemberState {
                lane,
                request: member.request,
            });
        }
    }
    Ok(())
}

fn prepare_member(
    shape: M1SpeculativePhysicalShapeV1,
    lane: usize,
    member_index: usize,
    member: &M1SpeculativeMemberStateV1,
    observation: &CheckedMemberObservationV1,
    control: M1SpeculativeMemberControlActionV1,
) -> Result<PreparedMemberV1, M1SpeculativeGenerationLoopErrorV1> {
    let CheckedCompletionSemantics::Speculative {
        accepted_draft_tokens,
        correction_or_bonus,
    } = observation.semantics
    else {
        return Err(M1SpeculativeGenerationLoopErrorV1::CompletionSemantics { lane });
    };
    let width = shape.draft_tokens();
    if accepted_draft_tokens > width {
        return Err(M1SpeculativeGenerationLoopErrorV1::AcceptedCount {
            lane,
            maximum: width,
            actual: accepted_draft_tokens,
        });
    }
    let expected_emitted = usize::from(accepted_draft_tokens) + 1;
    if observation.emitted.tokens().len() != expected_emitted {
        return Err(M1SpeculativeGenerationLoopErrorV1::EmittedCount {
            lane,
            expected: expected_emitted,
            actual: observation.emitted.tokens().len(),
        });
    }
    let actual_correction = observation.emitted.tokens()[usize::from(accepted_draft_tokens)];
    if actual_correction != correction_or_bonus {
        return Err(M1SpeculativeGenerationLoopErrorV1::CorrectionOrBonus {
            lane,
            expected: correction_or_bonus,
            actual: actual_correction,
        });
    }

    let target_accepted = u32::from(accepted_draft_tokens) + 1;
    let draft_accepted = if accepted_draft_tokens < width {
        u32::from(accepted_draft_tokens) + 1
    } else {
        u32::from(width)
    };
    let target_tentative_end = member.target_committed_tokens + u32::from(width) + 1;
    let draft_tentative_end = member.draft_committed_tokens + u32::from(width);
    let target_commit_end = member.target_committed_tokens + target_accepted;
    let draft_commit_end = member.draft_committed_tokens + draft_accepted;
    let target_rollback = width - accepted_draft_tokens;
    let draft_rollback = if accepted_draft_tokens < width {
        width - accepted_draft_tokens - 1
    } else {
        0
    };
    let target_settlement = M1SpeculativeKvRoleSettlementV1 {
        role: Qwen3ModelRole::Target8B,
        pre_committed: member.target_committed_tokens,
        tentative_end: target_tentative_end,
        commit_end: target_commit_end,
        rollback_tokens: target_rollback,
    };
    let draft_settlement = M1SpeculativeKvRoleSettlementV1 {
        role: Qwen3ModelRole::Draft06B,
        pre_committed: member.draft_committed_tokens,
        tentative_end: draft_tentative_end,
        commit_end: draft_commit_end,
        rollback_tokens: draft_rollback,
    };
    let verification_choice = if accepted_draft_tokens == width {
        M1SpeculativeVerificationChoiceV1::Bonus {
            token: correction_or_bonus,
        }
    } else {
        M1SpeculativeVerificationChoiceV1::Correction {
            token: correction_or_bonus,
        }
    };

    let (published, status, generated_tokens) =
        apply_member_policy(member, observation.emitted, control);
    let active = status == M1SpeculativeMemberStatusV1::Active;
    let outcome = M1SpeculativeMemberRoundOutcomeV1 {
        request: member.request,
        accepted_draft_tokens,
        raw_emitted: observation.emitted,
        published,
        verification_choice,
        target_settlement,
        draft_settlement,
        status,
        physical_disposition: if active {
            M1DeviceKvCompletionDispositionV1::Continue
        } else {
            M1DeviceKvCompletionDispositionV1::Retire
        },
        next_draft_anchor: active.then_some(correction_or_bonus),
    };
    Ok(PreparedMemberV1 {
        member_index,
        generated_tokens,
        outcome,
    })
}

fn apply_member_policy(
    member: &M1SpeculativeMemberStateV1,
    raw: M1SpeculativeTokenBlockV1,
    control: M1SpeculativeMemberControlActionV1,
) -> (M1SpeculativeTokenBlockV1, M1SpeculativeMemberStatusV1, u32) {
    if let M1SpeculativeMemberControlActionV1::Cancel(reason) = control {
        return (
            M1SpeculativeTokenBlockV1::empty(),
            M1SpeculativeMemberStatusV1::Cancelled(reason),
            member.generated_tokens,
        );
    }
    let mut published = M1SpeculativeTokenBlockV1::empty();
    let mut generated_tokens = member.generated_tokens;
    let mut status = M1SpeculativeMemberStatusV1::Active;
    for token in raw.tokens().iter().copied() {
        if generated_tokens == member.policy.max_output_tokens() {
            status =
                M1SpeculativeMemberStatusV1::Completed(M1SpeculativeTerminalReasonV1::OutputLimit);
            break;
        }
        let index = usize::from(published.count);
        published.tokens[index] = token;
        published.count += 1;
        generated_tokens += 1;
        if member.policy.is_stop(token) {
            status =
                M1SpeculativeMemberStatusV1::Completed(M1SpeculativeTerminalReasonV1::StopToken {
                    token,
                });
            break;
        }
        if generated_tokens == member.policy.max_output_tokens() {
            status =
                M1SpeculativeMemberStatusV1::Completed(M1SpeculativeTerminalReasonV1::OutputLimit);
            break;
        }
    }
    (published, status, generated_tokens)
}

fn with_emitted_lane(
    error: M1SpeculativeGenerationLoopErrorV1,
    lane: usize,
) -> M1SpeculativeGenerationLoopErrorV1 {
    match error {
        M1SpeculativeGenerationLoopErrorV1::EmittedCount {
            expected, actual, ..
        } => M1SpeculativeGenerationLoopErrorV1::EmittedCount {
            lane,
            expected,
            actual,
        },
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selection(bucket: Qwen3PlanBucket) -> Qwen3PlanSelection {
        Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Speculative,
            bucket,
        }
    }

    fn request(lane: u32) -> RequestId {
        RequestId::new(lane, 1)
    }

    fn seed(lane: u32, max_output_tokens: u32) -> M1SpeculativeMemberSeedV1 {
        M1SpeculativeMemberSeedV1::new(
            request(lane),
            70 + lane,
            10,
            10,
            M1SpeculativeGenerationPolicyV1::new(max_output_tokens, &[999]).unwrap(),
        )
    }

    fn observation(
        request: RequestId,
        accepted: u8,
        tokens: &[TokenId],
    ) -> CheckedMemberObservationV1 {
        CheckedMemberObservationV1 {
            request,
            semantics: CheckedCompletionSemantics::Speculative {
                accepted_draft_tokens: accepted,
                correction_or_bonus: *tokens.last().unwrap(),
            },
            emitted: M1SpeculativeTokenBlockV1::from_slice(tokens).unwrap(),
        }
    }

    fn complete_test_round(
        coordinator: &mut M1SpeculativeGenerationLoopV1,
        binding: M1SpeculativeRoundBindingV1,
        observations: &[CheckedMemberObservationV1],
        controls: &[M1SpeculativeMemberControlV1],
    ) -> Result<M1SpeculativeRoundOutcomeV1, M1SpeculativeGenerationLoopErrorV1> {
        let selection = binding.shape.selection();
        let epoch = binding.epoch;
        let preflighted = coordinator.preflight_observed_round(
            binding,
            selection,
            epoch,
            observations,
            controls,
        )?;
        coordinator
            .commit_preflighted_round(preflighted)
            .map_err(|failure| failure.into_parts().0)
    }

    #[test]
    fn every_admitted_physical_shape_binds_its_exact_width_and_capacity() {
        let cases = [
            (Qwen3PlanBucket::SpeculativeS1K4C8192, 1_u8, 4),
            (Qwen3PlanBucket::SpeculativeS8K4C8192, 8_u8, 4),
            (Qwen3PlanBucket::SpeculativeS1K8C8192, 1_u8, 8),
            (Qwen3PlanBucket::SpeculativeS1K16C8192, 1_u8, 16),
        ];
        for (bucket, sequences, width) in cases {
            let seeds: Vec<_> = (0..u32::from(sequences))
                .map(|lane| seed(lane, 64))
                .collect();
            let coordinator =
                M1SpeculativeGenerationLoopV1::new(selection(bucket), &seeds).unwrap();
            assert_eq!(coordinator.shape().sequences(), sequences);
            assert_eq!(coordinator.shape().draft_tokens(), width);
            let roster = coordinator.active_roster();
            let binding = coordinator
                .bind_round(0, CompletionEpoch::new(3), &roster)
                .unwrap();
            assert_eq!(binding.members().len(), usize::from(sequences));
        }
    }

    #[test]
    fn partial_then_full_match_repeats_with_exact_feedback_and_role_settlement() {
        let mut coordinator = M1SpeculativeGenerationLoopV1::new(
            selection(Qwen3PlanBucket::SpeculativeS1K4C8192),
            &[seed(0, 32)],
        )
        .unwrap();
        let first = coordinator
            .bind_round(0, CompletionEpoch::new(7), &[request(0)])
            .unwrap();
        let first_outcome = complete_test_round(
            &mut coordinator,
            first,
            &[observation(request(0), 2, &[100, 101, 900])],
            &[M1SpeculativeMemberControlV1::continuing(request(0))],
        )
        .unwrap();
        let member = first_outcome.members()[0];
        assert_eq!(member.accepted_prefix_tokens(), &[100, 101]);
        assert_eq!(member.published().tokens(), &[100, 101, 900]);
        assert_eq!(
            member.verification_choice(),
            M1SpeculativeVerificationChoiceV1::Correction { token: 900 }
        );
        assert_eq!(member.next_draft_anchor(), Some(900));
        assert_eq!(
            member.target_settlement(),
            M1SpeculativeKvRoleSettlementV1 {
                role: Qwen3ModelRole::Target8B,
                pre_committed: 10,
                tentative_end: 15,
                commit_end: 13,
                rollback_tokens: 2,
            }
        );
        assert_eq!(
            member.draft_settlement(),
            M1SpeculativeKvRoleSettlementV1 {
                role: Qwen3ModelRole::Draft06B,
                pre_committed: 10,
                tentative_end: 14,
                commit_end: 13,
                rollback_tokens: 1,
            }
        );

        let second = coordinator
            .bind_round(1, CompletionEpoch::new(8), &[request(0)])
            .unwrap();
        assert_eq!(second.members()[0].round_anchor(), 900);
        assert_eq!(second.members()[0].target_pre_committed(), 13);
        assert_eq!(second.members()[0].draft_pre_committed(), 13);
        let second_outcome = complete_test_round(
            &mut coordinator,
            second,
            &[observation(request(0), 4, &[200, 201, 202, 203, 901])],
            &[M1SpeculativeMemberControlV1::continuing(request(0))],
        )
        .unwrap();
        let member = second_outcome.members()[0];
        assert_eq!(
            member.verification_choice(),
            M1SpeculativeVerificationChoiceV1::Bonus { token: 901 }
        );
        assert_eq!(member.target_settlement().commit_end(), 18);
        assert_eq!(member.target_settlement().rollback_tokens(), 0);
        assert_eq!(member.draft_settlement().commit_end(), 17);
        assert_eq!(member.draft_settlement().rollback_tokens(), 0);
        assert_eq!(
            coordinator.member(request(0)).unwrap().generated_tokens(),
            8
        );
    }

    #[test]
    fn two_phase_preflight_exposes_dispositions_without_advancing_and_is_retry_safe() {
        let mut coordinator = M1SpeculativeGenerationLoopV1::new(
            selection(Qwen3PlanBucket::SpeculativeS1K4C8192),
            &[seed(0, 32)],
        )
        .unwrap();
        let first_binding = coordinator
            .bind_round(0, CompletionEpoch::new(12), &[request(0)])
            .unwrap();
        let stale_binding = coordinator
            .bind_round(0, CompletionEpoch::new(12), &[request(0)])
            .unwrap();
        let observations = [observation(request(0), 0, &[500])];
        let controls = [M1SpeculativeMemberControlV1::continuing(request(0))];
        let first = coordinator
            .preflight_observed_round(
                first_binding,
                coordinator.shape().selection(),
                CompletionEpoch::new(12),
                &observations,
                &controls,
            )
            .unwrap();
        let stale = coordinator
            .preflight_observed_round(
                stale_binding,
                coordinator.shape().selection(),
                CompletionEpoch::new(12),
                &observations,
                &controls,
            )
            .unwrap();
        assert_eq!(coordinator.next_round(), 0);
        assert_eq!(coordinator.last_epoch(), None);
        assert_eq!(
            first.members()[0].physical_disposition(),
            M1DeviceKvCompletionDispositionV1::Continue
        );
        let outcome = coordinator.commit_preflighted_round(first).unwrap();
        assert_eq!(outcome.completed_round(), 0);
        assert_eq!(coordinator.next_round(), 1);

        let failure = coordinator.commit_preflighted_round(stale).unwrap_err();
        assert_eq!(
            failure.error(),
            &M1SpeculativeGenerationLoopErrorV1::RoundDrift {
                expected: 1,
                actual: 0,
            }
        );
        let (_, recovered) = failure.into_parts();
        assert_eq!(recovered.round(), 0);
        assert_eq!(recovered.epoch(), CompletionEpoch::new(12));
        assert_eq!(coordinator.next_round(), 1);
    }

    #[test]
    fn k8_and_k16_first_mismatch_roll_back_the_complete_rejected_suffix() {
        for (bucket, width) in [
            (Qwen3PlanBucket::SpeculativeS1K8C8192, 8),
            (Qwen3PlanBucket::SpeculativeS1K16C8192, 16),
        ] {
            let mut coordinator =
                M1SpeculativeGenerationLoopV1::new(selection(bucket), &[seed(0, 32)]).unwrap();
            let binding = coordinator
                .bind_round(0, CompletionEpoch::new(1), &[request(0)])
                .unwrap();
            let outcome = complete_test_round(
                &mut coordinator,
                binding,
                &[observation(request(0), 0, &[555])],
                &[M1SpeculativeMemberControlV1::continuing(request(0))],
            )
            .unwrap();
            assert_eq!(
                outcome.members()[0].target_settlement().rollback_tokens(),
                width
            );
            assert_eq!(
                outcome.members()[0].draft_settlement().rollback_tokens(),
                width - 1
            );
        }
    }

    #[test]
    fn s8_mixed_continue_stop_limit_and_cancel_produces_exact_next_roster() {
        let seeds: Vec<_> = (0..8)
            .map(|lane| seed(lane, if lane == 2 { 1 } else { 32 }))
            .collect();
        let mut coordinator = M1SpeculativeGenerationLoopV1::new(
            selection(Qwen3PlanBucket::SpeculativeS8K4C8192),
            &seeds,
        )
        .unwrap();
        let roster = coordinator.active_roster();
        let binding = coordinator
            .bind_round(0, CompletionEpoch::new(20), &roster)
            .unwrap();
        let observations: Vec<_> = (0..8)
            .map(|lane| {
                let token = if lane == 1 { 999 } else { 100 + lane };
                observation(request(lane), 0, &[token])
            })
            .collect();
        let controls: Vec<_> = (0..8)
            .map(|lane| {
                if lane == 3 {
                    M1SpeculativeMemberControlV1::cancelling(
                        request(lane),
                        M1SpeculativeCancellationReasonV1::Client,
                    )
                } else {
                    M1SpeculativeMemberControlV1::continuing(request(lane))
                }
            })
            .collect();
        let outcome =
            complete_test_round(&mut coordinator, binding, &observations, &controls).unwrap();
        assert_eq!(
            outcome.members()[1].status(),
            M1SpeculativeMemberStatusV1::Completed(M1SpeculativeTerminalReasonV1::StopToken {
                token: 999
            })
        );
        assert_eq!(
            outcome.members()[2].status(),
            M1SpeculativeMemberStatusV1::Completed(M1SpeculativeTerminalReasonV1::OutputLimit)
        );
        assert_eq!(
            outcome.members()[3].status(),
            M1SpeculativeMemberStatusV1::Cancelled(M1SpeculativeCancellationReasonV1::Client)
        );
        assert!(outcome.members()[3].published().is_empty());
        assert_eq!(
            outcome.members()[3].physical_disposition(),
            M1DeviceKvCompletionDispositionV1::Retire
        );
        assert_eq!(
            outcome.next_active_roster(),
            &[request(0), request(4), request(5), request(6), request(7)]
        );
        let next = coordinator
            .bind_round(1, CompletionEpoch::new(21), outcome.next_active_roster())
            .unwrap();
        assert_eq!(next.members().len(), 5);
    }

    #[test]
    fn stop_and_output_limit_truncate_publication_without_changing_kv_settlement() {
        let stop_seed = seed(0, 32);
        let limit_seed = seed(1, 2);
        let mut coordinator = M1SpeculativeGenerationLoopV1::new(
            selection(Qwen3PlanBucket::SpeculativeS8K4C8192),
            &[stop_seed, limit_seed],
        )
        .unwrap();
        let binding = coordinator
            .bind_round(0, CompletionEpoch::new(30), &[request(0), request(1)])
            .unwrap();
        let outcome = complete_test_round(
            &mut coordinator,
            binding,
            &[
                observation(request(0), 2, &[100, 999, 700]),
                observation(request(1), 3, &[200, 201, 202, 701]),
            ],
            &[
                M1SpeculativeMemberControlV1::continuing(request(0)),
                M1SpeculativeMemberControlV1::continuing(request(1)),
            ],
        )
        .unwrap();
        assert_eq!(
            outcome.members()[0].raw_emitted().tokens(),
            &[100, 999, 700]
        );
        assert_eq!(outcome.members()[0].published().tokens(), &[100, 999]);
        assert_eq!(outcome.members()[0].target_settlement().commit_end(), 13);
        assert_eq!(outcome.members()[0].draft_settlement().commit_end(), 13);
        assert_eq!(
            outcome.members()[0].status(),
            M1SpeculativeMemberStatusV1::Completed(M1SpeculativeTerminalReasonV1::StopToken {
                token: 999
            })
        );
        assert_eq!(outcome.members()[1].published().tokens(), &[200, 201]);
        assert_eq!(outcome.members()[1].target_settlement().commit_end(), 14);
        assert_eq!(outcome.members()[1].draft_settlement().commit_end(), 14);
        assert_eq!(
            outcome.members()[1].status(),
            M1SpeculativeMemberStatusV1::Completed(M1SpeculativeTerminalReasonV1::OutputLimit)
        );
        assert!(outcome.next_active_roster().is_empty());
    }

    #[test]
    fn round_epoch_roster_and_completion_drift_leave_state_unchanged() {
        let coordinator = M1SpeculativeGenerationLoopV1::new(
            selection(Qwen3PlanBucket::SpeculativeS1K4C8192),
            &[seed(0, 32)],
        )
        .unwrap();
        assert!(matches!(
            coordinator.bind_round(1, CompletionEpoch::new(1), &[request(0)]),
            Err(M1SpeculativeGenerationLoopErrorV1::RoundDrift {
                expected: 0,
                actual: 1,
            })
        ));
        assert!(matches!(
            coordinator.bind_round(0, CompletionEpoch::new(1), &[request(1)]),
            Err(M1SpeculativeGenerationLoopErrorV1::RosterOrder { .. })
        ));
        let before = coordinator.member(request(0)).unwrap();
        let binding = coordinator
            .bind_round(0, CompletionEpoch::new(1), &[request(0)])
            .unwrap();
        let error = coordinator
            .preflight_observed_round(
                binding,
                coordinator.shape().selection(),
                CompletionEpoch::new(2),
                &[observation(request(0), 0, &[42])],
                &[M1SpeculativeMemberControlV1::continuing(request(0))],
            )
            .unwrap_err();
        assert!(matches!(
            error,
            M1SpeculativeGenerationLoopErrorV1::EpochDrift { .. }
        ));
        assert_eq!(coordinator.member(request(0)).unwrap(), before);
        assert_eq!(coordinator.next_round(), 0);
    }

    #[test]
    fn stale_binding_and_skipped_epoch_are_rejected_after_success() {
        let mut coordinator = M1SpeculativeGenerationLoopV1::new(
            selection(Qwen3PlanBucket::SpeculativeS1K4C8192),
            &[seed(0, 32)],
        )
        .unwrap();
        let stale = coordinator
            .bind_round(0, CompletionEpoch::new(7), &[request(0)])
            .unwrap();
        let current = coordinator
            .bind_round(0, CompletionEpoch::new(7), &[request(0)])
            .unwrap();
        let _ = complete_test_round(
            &mut coordinator,
            current,
            &[observation(request(0), 0, &[80])],
            &[M1SpeculativeMemberControlV1::continuing(request(0))],
        )
        .unwrap();
        assert_eq!(
            complete_test_round(
                &mut coordinator,
                stale,
                &[observation(request(0), 0, &[81])],
                &[M1SpeculativeMemberControlV1::continuing(request(0))],
            )
            .unwrap_err(),
            M1SpeculativeGenerationLoopErrorV1::RoundDrift {
                expected: 1,
                actual: 0,
            }
        );
        assert!(matches!(
            coordinator.bind_round(1, CompletionEpoch::new(9), &[request(0)]),
            Err(M1SpeculativeGenerationLoopErrorV1::EpochSequence {
                expected: 8,
                actual: 9,
            })
        ));
    }

    #[test]
    fn whole_roster_semantic_failure_is_atomic() {
        let mut coordinator = M1SpeculativeGenerationLoopV1::new(
            selection(Qwen3PlanBucket::SpeculativeS8K4C8192),
            &[seed(0, 32), seed(1, 32)],
        )
        .unwrap();
        let binding = coordinator
            .bind_round(0, CompletionEpoch::new(1), &[request(0), request(1)])
            .unwrap();
        let first_before = coordinator.member(request(0)).unwrap();
        let second_before = coordinator.member(request(1)).unwrap();
        let malformed = CheckedMemberObservationV1 {
            request: request(1),
            semantics: CheckedCompletionSemantics::Speculative {
                accepted_draft_tokens: 2,
                correction_or_bonus: 700,
            },
            emitted: M1SpeculativeTokenBlockV1::from_slice(&[1, 700]).unwrap(),
        };
        assert_eq!(
            complete_test_round(
                &mut coordinator,
                binding,
                &[observation(request(0), 0, &[50]), malformed],
                &[
                    M1SpeculativeMemberControlV1::continuing(request(0)),
                    M1SpeculativeMemberControlV1::continuing(request(1)),
                ],
            )
            .unwrap_err(),
            M1SpeculativeGenerationLoopErrorV1::EmittedCount {
                lane: 1,
                expected: 3,
                actual: 2,
            }
        );
        assert_eq!(coordinator.member(request(0)).unwrap(), first_before);
        assert_eq!(coordinator.member(request(1)).unwrap(), second_before);
    }

    #[test]
    fn context_and_policy_boundaries_fail_closed() {
        assert_eq!(
            M1SpeculativeGenerationPolicyV1::new(0, &[]),
            Err(M1SpeculativeGenerationLoopErrorV1::InvalidOutputLimit { actual: 0 })
        );
        let near_end = M1SpeculativeMemberSeedV1::new(
            request(0),
            1,
            M1_MAX_CONTEXT_TOKENS - 4,
            0,
            M1SpeculativeGenerationPolicyV1::new(1, &[]).unwrap(),
        );
        assert_eq!(
            M1SpeculativeGenerationLoopV1::new(
                selection(Qwen3PlanBucket::SpeculativeS1K4C8192),
                &[near_end],
            )
            .unwrap_err(),
            M1SpeculativeGenerationLoopErrorV1::ContextExceeded {
                lane: 0,
                role: Qwen3ModelRole::Target8B,
            }
        );
    }
}
