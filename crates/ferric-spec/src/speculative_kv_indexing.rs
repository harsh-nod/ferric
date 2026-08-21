//! Exact logical token-to-KV indexing for one speculative Qwen3 round.
//!
//! The graph's target width is `K + 1`: ordinal zero consumes the round
//! anchor and ordinal `i + 1` consumes draft candidate `i`. The draft width is
//! `K`: ordinal zero consumes the same anchor and ordinal `i + 1 < K`
//! consumes candidate `i`. Target choice `i` is the logit after target input
//! ordinal `i`; it is not a KV write. The selected correction or bonus is
//! therefore deferred until the next generated step.
//!
//! The pre-round cursor names the first tentative position, not the last
//! resident token. For accepted count `A`, the target commits the anchor plus
//! `A` candidates, while the draft commits the anchor plus the candidates it
//! consumed: `min(A + 1, K)`. This module defines only finite source-level
//! indexing. It does not compose publication or physical KV transitions and
//! makes no runner, queue, device, address, machine, or performance claim.

use crate::completion::CompletionEpoch;
use crate::{
    Identity, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket, Qwen3PlanError,
    Qwen3PlanSelection, RequestId, TokenId, M1_CONTINUOUS_BATCH_CAPACITY, M1_MAX_CONTEXT_TOKENS,
    QWEN3_VOCABULARY_SIZE,
};
use vstd::prelude::*;

verus! {

/// Exact finite `K` bound inherited from M1 greedy completion.
pub const M1_MAX_SPECULATIVE_KV_DRAFT_TOKENS: usize = 16;
/// The target consumes the anchor plus at most sixteen draft candidates.
pub const M1_MAX_SPECULATIVE_KV_TARGET_INPUTS: usize = 17;

/// Logical source of one model input whose K/V state is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpeculativeKvInputSource {
    RoundAnchor { token: TokenId },
    DraftCandidate { index: u8, token: TokenId },
}

/// Exact per-role graph ordinal and its logical KV position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpeculativeKvInputBinding {
    pub role: Qwen3ModelRole,
    pub graph_ordinal: u8,
    pub logical_position: u32,
    pub source: SpeculativeKvInputSource,
}

/// Target choices are logits only; they never stand for an implicit KV write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetChoiceUse {
    LogitOnly,
}

/// Exact target choice to target-input ordinal relation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TargetChoiceBinding {
    pub choice_index: u8,
    pub after_target_ordinal: u8,
    pub use_kind: TargetChoiceUse,
}

/// Correction/bonus placement is explicit rather than inferred from emission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CorrectionBonusKvDisposition {
    DeferredUntilNextStep,
    TargetResident,
    DraftResident,
}

/// Half-open logical interval `[start, end)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpeculativeKvInterval {
    pub start: u32,
    pub end: u32,
}

/// Canonical finite indexing input for one request-local speculative round.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpeculativeKvRoundIndex {
    pub request: RequestId,
    pub completion_epoch: CompletionEpoch,
    pub plan_id: Identity,
    pub target_selection: Qwen3PlanSelection,
    pub draft_selection: Qwen3PlanSelection,
    pub draft_token_count: u8,
    pub round_anchor: TokenId,
    pub draft_tokens: [TokenId; M1_MAX_SPECULATIVE_KV_DRAFT_TOKENS],
    pub target_pre_committed: u32,
    pub draft_pre_committed: u32,
    pub target_tentative: SpeculativeKvInterval,
    pub draft_tentative: SpeculativeKvInterval,
    /// Entry `A` is the target commit end for accepted count `A`.
    pub target_commit_ends: [u32; M1_MAX_SPECULATIVE_KV_TARGET_INPUTS],
    /// Entry `A` is the draft commit end for accepted count `A`.
    pub draft_commit_ends: [u32; M1_MAX_SPECULATIVE_KV_TARGET_INPUTS],
    pub correction_bonus: CorrectionBonusKvDisposition,
}

/// Fail-closed structural or authority rejection.
#[derive(Debug, PartialEq, Eq)]
pub enum SpeculativeKvIndexError {
    Request,
    CompletionEpoch,
    PlanIdentity,
    Selection(Qwen3PlanError),
    RoleOrMode,
    BucketMismatch,
    DraftLength,
    GraphDimensions,
    CursorMismatch,
    ContextExceeded,
    TentativeInterval,
    TokenOutOfRange,
    NoncanonicalUnusedToken,
    CommitEnd,
    CorrectionBonusNotDeferred,
    AuthorityMismatch,
}

pub closed spec fn correction_is_deferred(
    disposition: CorrectionBonusKvDisposition,
) -> bool {
    match disposition {
        CorrectionBonusKvDisposition::DeferredUntilNextStep => true,
        CorrectionBonusKvDisposition::TargetResident
        | CorrectionBonusKvDisposition::DraftResident => false,
    }
}

pub closed spec fn speculative_bucket_k(bucket: Qwen3PlanBucket) -> Option<u8> {
    match bucket {
        Qwen3PlanBucket::SpeculativeS1K4C8192
        | Qwen3PlanBucket::SpeculativeS8K4C8192 => Some(4),
        Qwen3PlanBucket::SpeculativeS1K8C8192 => Some(8),
        Qwen3PlanBucket::SpeculativeS1K16C8192 => Some(16),
        Qwen3PlanBucket::PrefillS1T128
        | Qwen3PlanBucket::PrefillS8T128
        | Qwen3PlanBucket::PrefillS1T512
        | Qwen3PlanBucket::PrefillS1T2048
        | Qwen3PlanBucket::DecodeS1C8192
        | Qwen3PlanBucket::DecodeS8C8192
        | Qwen3PlanBucket::DecodeS32C8192 => None,
    }
}

fn bucket_k(bucket: Qwen3PlanBucket) -> (result: Option<u8>)
    ensures result == speculative_bucket_k(bucket),
{
    match bucket {
        Qwen3PlanBucket::SpeculativeS1K4C8192
        | Qwen3PlanBucket::SpeculativeS8K4C8192 => Some(4),
        Qwen3PlanBucket::SpeculativeS1K8C8192 => Some(8),
        Qwen3PlanBucket::SpeculativeS1K16C8192 => Some(16),
        Qwen3PlanBucket::PrefillS1T128
        | Qwen3PlanBucket::PrefillS8T128
        | Qwen3PlanBucket::PrefillS1T512
        | Qwen3PlanBucket::PrefillS1T2048
        | Qwen3PlanBucket::DecodeS1C8192
        | Qwen3PlanBucket::DecodeS8C8192
        | Qwen3PlanBucket::DecodeS32C8192 => None,
    }
}

pub closed spec fn same_speculative_bucket(
    left: Qwen3PlanBucket,
    right: Qwen3PlanBucket,
) -> bool {
    match (left, right) {
        (Qwen3PlanBucket::SpeculativeS1K4C8192, Qwen3PlanBucket::SpeculativeS1K4C8192)
        | (Qwen3PlanBucket::SpeculativeS8K4C8192, Qwen3PlanBucket::SpeculativeS8K4C8192)
        | (Qwen3PlanBucket::SpeculativeS1K8C8192, Qwen3PlanBucket::SpeculativeS1K8C8192)
        | (Qwen3PlanBucket::SpeculativeS1K16C8192, Qwen3PlanBucket::SpeculativeS1K16C8192) => true,
        _ => false,
    }
}

fn buckets_match(
    left: Qwen3PlanBucket,
    right: Qwen3PlanBucket,
) -> (matches: bool)
    ensures matches == same_speculative_bucket(left, right),
{
    matches!(
        (left, right),
        (Qwen3PlanBucket::SpeculativeS1K4C8192, Qwen3PlanBucket::SpeculativeS1K4C8192)
            | (Qwen3PlanBucket::SpeculativeS8K4C8192, Qwen3PlanBucket::SpeculativeS8K4C8192)
            | (Qwen3PlanBucket::SpeculativeS1K8C8192, Qwen3PlanBucket::SpeculativeS1K8C8192)
            | (
                Qwen3PlanBucket::SpeculativeS1K16C8192,
                Qwen3PlanBucket::SpeculativeS1K16C8192,
            )
    )
}

pub closed spec fn input_source_matches(
    actual: SpeculativeKvInputSource,
    expected: SpeculativeKvInputSource,
) -> bool {
    match (actual, expected) {
        (
            SpeculativeKvInputSource::RoundAnchor { token: left },
            SpeculativeKvInputSource::RoundAnchor { token: right },
        ) => left == right,
        (
            SpeculativeKvInputSource::DraftCandidate { index: li, token: lt },
            SpeculativeKvInputSource::DraftCandidate { index: ri, token: rt },
        ) => li == ri && lt == rt,
        _ => false,
    }
}

pub closed spec fn input_binding_matches(
    actual: SpeculativeKvInputBinding,
    expected: SpeculativeKvInputBinding,
) -> bool {
    actual.role == expected.role
        && actual.graph_ordinal == expected.graph_ordinal
        && actual.logical_position == expected.logical_position
        && input_source_matches(actual.source, expected.source)
}

pub closed spec fn choice_binding_matches(
    actual: TargetChoiceBinding,
    expected: TargetChoiceBinding,
) -> bool {
    actual.choice_index == expected.choice_index
        && actual.after_target_ordinal == expected.after_target_ordinal
        && match (actual.use_kind, expected.use_kind) {
            (TargetChoiceUse::LogitOnly, TargetChoiceUse::LogitOnly) => true,
        }
}

impl SpeculativeKvRoundIndex {
    pub closed spec fn target_input_spec(
        &self,
        ordinal: u8,
    ) -> Option<SpeculativeKvInputBinding> {
        if ordinal <= self.draft_token_count
            && ordinal as int <= M1_MAX_SPECULATIVE_KV_DRAFT_TOKENS
            && self.target_pre_committed as int + ordinal as int <= u32::MAX
        {
            Some(SpeculativeKvInputBinding {
                role: Qwen3ModelRole::Target8B,
                graph_ordinal: ordinal,
                logical_position: (self.target_pre_committed as int + ordinal as int) as u32,
                source: if ordinal == 0 {
                    SpeculativeKvInputSource::RoundAnchor { token: self.round_anchor }
                } else {
                    SpeculativeKvInputSource::DraftCandidate {
                        index: (ordinal - 1) as u8,
                        token: self.draft_tokens[(ordinal - 1) as int],
                    }
                },
            })
        } else {
            None
        }
    }

    pub closed spec fn draft_input_spec(
        &self,
        ordinal: u8,
    ) -> Option<SpeculativeKvInputBinding> {
        if ordinal < self.draft_token_count
            && ordinal as int <= M1_MAX_SPECULATIVE_KV_DRAFT_TOKENS
            && self.draft_pre_committed as int + ordinal as int <= u32::MAX
        {
            Some(SpeculativeKvInputBinding {
                role: Qwen3ModelRole::Draft06B,
                graph_ordinal: ordinal,
                logical_position: (self.draft_pre_committed as int + ordinal as int) as u32,
                source: if ordinal == 0 {
                    SpeculativeKvInputSource::RoundAnchor { token: self.round_anchor }
                } else {
                    SpeculativeKvInputSource::DraftCandidate {
                        index: (ordinal - 1) as u8,
                        token: self.draft_tokens[(ordinal - 1) as int],
                    }
                },
            })
        } else {
            None
        }
    }

    pub closed spec fn target_choice_spec(
        &self,
        choice: u8,
    ) -> Option<TargetChoiceBinding> {
        if choice <= self.draft_token_count {
            Some(TargetChoiceBinding {
                choice_index: choice,
                after_target_ordinal: choice,
                use_kind: TargetChoiceUse::LogitOnly,
            })
        } else {
            None
        }
    }

    pub closed spec fn target_commit_end_spec(&self, accepted: u8) -> Option<u32> {
        if accepted <= self.draft_token_count
            && self.target_pre_committed as int + accepted as int + 1 <= u32::MAX
        {
            Some((self.target_pre_committed as int + accepted as int + 1) as u32)
        } else {
            None
        }
    }

    pub closed spec fn draft_commit_end_spec(&self, accepted: u8) -> Option<u32> {
        let consumed = if accepted < self.draft_token_count {
            accepted as int + 1
        } else {
            self.draft_token_count as int
        };
        if accepted <= self.draft_token_count
            && self.draft_pre_committed as int + consumed <= u32::MAX
        {
            Some((self.draft_pre_committed as int + consumed) as u32)
        } else {
            None
        }
    }

    /// Every rejected target/draft suffix fits within one 16-token KV page.
    pub proof fn rejected_tail_bounds(&self, accepted: u8)
        requires
            self.valid(),
            accepted <= self.draft_token_count,
        ensures
            self.target_commit_end_spec(accepted).is_some(),
            self.draft_commit_end_spec(accepted).is_some(),
            self.target_tentative.end as int
                - self.target_commit_end_spec(accepted).unwrap() as int
                == self.draft_token_count as int - accepted as int,
            self.draft_tentative.end as int
                - self.draft_commit_end_spec(accepted).unwrap() as int
                == if accepted < self.draft_token_count {
                    self.draft_token_count as int - accepted as int - 1
                } else {
                    0
                },
            0 <= self.target_tentative.end as int
                - self.target_commit_end_spec(accepted).unwrap() as int
                <= M1_MAX_SPECULATIVE_KV_DRAFT_TOKENS,
            0 <= self.draft_tentative.end as int
                - self.draft_commit_end_spec(accepted).unwrap() as int
                <= M1_MAX_SPECULATIVE_KV_DRAFT_TOKENS,
    {
        reveal(SpeculativeKvRoundIndex::valid);
        reveal(SpeculativeKvRoundIndex::target_commit_end_spec);
        reveal(SpeculativeKvRoundIndex::draft_commit_end_spec);
    }

    pub closed spec fn token_fields_valid(&self) -> bool {
        &&& self.round_anchor < QWEN3_VOCABULARY_SIZE
        &&& forall|index: int| 0 <= index < self.draft_token_count as int ==>
            #[trigger] self.draft_tokens@[index] < QWEN3_VOCABULARY_SIZE
        &&& forall|index: int|
            self.draft_token_count as int <= index < M1_MAX_SPECULATIVE_KV_DRAFT_TOKENS ==>
                #[trigger] self.draft_tokens@[index] == 0
    }

    pub closed spec fn commit_entry_valid(&self, accepted: int) -> bool {
        if accepted <= self.draft_token_count as int {
            &&& self.target_commit_ends@[accepted]
                == self.target_pre_committed as int + accepted + 1
            &&& self.draft_commit_ends@[accepted]
                == self.draft_pre_committed as int
                    + if accepted < self.draft_token_count as int {
                        accepted + 1
                    } else {
                        self.draft_token_count as int
                    }
        } else {
            self.target_commit_ends@[accepted] == 0
                && self.draft_commit_ends@[accepted] == 0
        }
    }

    pub closed spec fn commit_tables_valid(&self) -> bool {
        forall|accepted: int|
            0 <= accepted < M1_MAX_SPECULATIVE_KV_TARGET_INPUTS ==>
                #[trigger] self.commit_entry_valid(accepted)
    }

    pub closed spec fn valid(&self) -> bool {
        &&& self.request.slot_spec() < M1_CONTINUOUS_BATCH_CAPACITY
        &&& self.request.generation_spec() > 0
        &&& self.completion_epoch.value > 0
        &&& crate::m1_completion::identity_present(self.plan_id)
        &&& self.target_selection.valid()
        &&& self.draft_selection.valid()
        &&& self.target_selection.role == Qwen3ModelRole::Target8B
        &&& self.draft_selection.role == Qwen3ModelRole::Draft06B
        &&& self.target_selection.mode == Qwen3ExecutionMode::Speculative
        &&& self.draft_selection.mode == Qwen3ExecutionMode::Speculative
        &&& same_speculative_bucket(
            self.target_selection.bucket,
            self.draft_selection.bucket,
        )
        &&& speculative_bucket_k(self.target_selection.bucket) == Some(self.draft_token_count)
        &&& 0 < self.draft_token_count <= M1_MAX_SPECULATIVE_KV_DRAFT_TOKENS
        &&& self.target_selection.bucket.dimensions_spec(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Speculative,
        ).is_some()
        &&& self.draft_selection.bucket.dimensions_spec(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Speculative,
        ).is_some()
        &&& self.target_selection.bucket.dimensions_spec(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Speculative,
        ).unwrap().active_tokens as int == self.draft_token_count as int + 1
        &&& self.draft_selection.bucket.dimensions_spec(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Speculative,
        ).unwrap().active_tokens == self.draft_token_count
        &&& self.target_pre_committed == self.draft_pre_committed
        &&& self.target_pre_committed as int + self.draft_token_count as int + 1
            <= M1_MAX_CONTEXT_TOKENS
        &&& self.draft_pre_committed as int + self.draft_token_count as int
            <= M1_MAX_CONTEXT_TOKENS
        &&& self.target_tentative.start == self.target_pre_committed
        &&& self.target_tentative.end as int
            == self.target_pre_committed as int + self.draft_token_count as int + 1
        &&& self.draft_tentative.start == self.draft_pre_committed
        &&& self.draft_tentative.end as int
            == self.draft_pre_committed as int + self.draft_token_count as int
        &&& self.token_fields_valid()
        &&& self.commit_tables_valid()
        &&& correction_is_deferred(self.correction_bonus)
    }

    pub closed spec fn valid_for(
        &self,
        expected_request: RequestId,
        expected_epoch: CompletionEpoch,
        expected_plan_id: Identity,
        expected_target: Qwen3PlanSelection,
        expected_draft: Qwen3PlanSelection,
    ) -> bool {
        self.valid()
            && self.request.slot_spec() == expected_request.slot_spec()
            && self.request.generation_spec() == expected_request.generation_spec()
            && self.completion_epoch.value == expected_epoch.value
            && self.plan_id.bytes_spec() == expected_plan_id.bytes_spec()
            && self.target_selection == expected_target
            && self.draft_selection == expected_draft
    }

    pub proof fn valid_for_implies_valid(
        &self,
        expected_request: RequestId,
        expected_epoch: CompletionEpoch,
        expected_plan_id: Identity,
        expected_target: Qwen3PlanSelection,
        expected_draft: Qwen3PlanSelection,
    )
        requires self.valid_for(
            expected_request,
            expected_epoch,
            expected_plan_id,
            expected_target,
            expected_draft,
        ),
        ensures self.valid(),
    {
        reveal(SpeculativeKvRoundIndex::valid_for);
    }

    #[must_use]
    pub fn target_input(
        &self,
        ordinal: u8,
    ) -> (result: Option<SpeculativeKvInputBinding>)
        ensures result == self.target_input_spec(ordinal),
    {
        if ordinal > self.draft_token_count
            || ordinal as usize > M1_MAX_SPECULATIVE_KV_DRAFT_TOKENS
        {
            return None;
        }
        let logical_position = self.target_pre_committed.checked_add(u32::from(ordinal))?;
        let source = if ordinal == 0 {
            SpeculativeKvInputSource::RoundAnchor { token: self.round_anchor }
        } else {
            SpeculativeKvInputSource::DraftCandidate {
                index: ordinal - 1,
                token: self.draft_tokens[(ordinal - 1) as usize],
            }
        };
        Some(SpeculativeKvInputBinding {
            role: Qwen3ModelRole::Target8B,
            graph_ordinal: ordinal,
            logical_position,
            source,
        })
    }

    #[must_use]
    pub fn draft_input(
        &self,
        ordinal: u8,
    ) -> (result: Option<SpeculativeKvInputBinding>)
        ensures result == self.draft_input_spec(ordinal),
    {
        if ordinal >= self.draft_token_count
            || ordinal as usize > M1_MAX_SPECULATIVE_KV_DRAFT_TOKENS
        {
            return None;
        }
        let logical_position = self.draft_pre_committed.checked_add(u32::from(ordinal))?;
        let source = if ordinal == 0 {
            SpeculativeKvInputSource::RoundAnchor { token: self.round_anchor }
        } else {
            SpeculativeKvInputSource::DraftCandidate {
                index: ordinal - 1,
                token: self.draft_tokens[(ordinal - 1) as usize],
            }
        };
        Some(SpeculativeKvInputBinding {
            role: Qwen3ModelRole::Draft06B,
            graph_ordinal: ordinal,
            logical_position,
            source,
        })
    }

    #[must_use]
    pub fn target_choice(&self, choice: u8) -> (result: Option<TargetChoiceBinding>)
        ensures result == self.target_choice_spec(choice),
    {
        if choice > self.draft_token_count {
            None
        } else {
            Some(TargetChoiceBinding {
                choice_index: choice,
                after_target_ordinal: choice,
                use_kind: TargetChoiceUse::LogitOnly,
            })
        }
    }

    #[must_use]
    pub fn target_commit_end(&self, accepted: u8) -> (result: Option<u32>)
        ensures result == self.target_commit_end_spec(accepted),
    {
        if accepted > self.draft_token_count {
            return None;
        }
        self.target_pre_committed
            .checked_add(u32::from(accepted))?
            .checked_add(1)
    }

    #[must_use]
    pub fn draft_commit_end(&self, accepted: u8) -> (result: Option<u32>)
        ensures result == self.draft_commit_end_spec(accepted),
    {
        if accepted > self.draft_token_count {
            return None;
        }
        let consumed = if accepted < self.draft_token_count {
            u32::from(accepted) + 1
        } else {
            u32::from(self.draft_token_count)
        };
        self.draft_pre_committed.checked_add(consumed)
    }

    /// Validates the complete finite index, including every `A <= K` endpoint.
    ///
    /// # Errors
    ///
    /// Returns the first structural error and never weakens the contract.
    pub fn validate(&self) -> (result: Result<(), SpeculativeKvIndexError>)
        ensures result.is_ok() == self.valid(),
    {
        proof {
            reveal(SpeculativeKvRoundIndex::valid);
            reveal(SpeculativeKvRoundIndex::token_fields_valid);
            reveal(SpeculativeKvRoundIndex::commit_tables_valid);
            reveal(correction_is_deferred);
        }
        if self.request.slot() as usize >= M1_CONTINUOUS_BATCH_CAPACITY
            || self.request.generation() == 0
        {
            return Err(SpeculativeKvIndexError::Request);
        }
        if self.completion_epoch.value == 0 {
            return Err(SpeculativeKvIndexError::CompletionEpoch);
        }
        if !self.plan_id.is_present() {
            return Err(SpeculativeKvIndexError::PlanIdentity);
        }
        if let Err(error) = self.target_selection.validate() {
            return Err(SpeculativeKvIndexError::Selection(error));
        }
        if let Err(error) = self.draft_selection.validate() {
            return Err(SpeculativeKvIndexError::Selection(error));
        }
        if !matches!(self.target_selection.role, Qwen3ModelRole::Target8B)
            || !matches!(self.draft_selection.role, Qwen3ModelRole::Draft06B)
            || !matches!(self.target_selection.mode, Qwen3ExecutionMode::Speculative)
            || !matches!(self.draft_selection.mode, Qwen3ExecutionMode::Speculative)
        {
            return Err(SpeculativeKvIndexError::RoleOrMode);
        }
        if !buckets_match(
            self.target_selection.bucket,
            self.draft_selection.bucket,
        ) {
            return Err(SpeculativeKvIndexError::BucketMismatch);
        }
        let Some(k) = bucket_k(self.target_selection.bucket) else {
            return Err(SpeculativeKvIndexError::DraftLength);
        };
        if k != self.draft_token_count
            || self.draft_token_count == 0
            || self.draft_token_count as usize > M1_MAX_SPECULATIVE_KV_DRAFT_TOKENS
        {
            return Err(SpeculativeKvIndexError::DraftLength);
        }
        let Some(target_dimensions) = self
            .target_selection
            .bucket
            .dimensions(Qwen3ModelRole::Target8B, Qwen3ExecutionMode::Speculative)
        else {
            return Err(SpeculativeKvIndexError::GraphDimensions);
        };
        let Some(draft_dimensions) = self
            .draft_selection
            .bucket
            .dimensions(Qwen3ModelRole::Draft06B, Qwen3ExecutionMode::Speculative)
        else {
            return Err(SpeculativeKvIndexError::GraphDimensions);
        };
        if target_dimensions.active_tokens != u32::from(self.draft_token_count) + 1
            || draft_dimensions.active_tokens != u32::from(self.draft_token_count)
        {
            return Err(SpeculativeKvIndexError::GraphDimensions);
        }
        if self.target_pre_committed != self.draft_pre_committed {
            return Err(SpeculativeKvIndexError::CursorMismatch);
        }
        let target_width = u32::from(self.draft_token_count) + 1;
        let draft_width = u32::from(self.draft_token_count);
        if self.target_pre_committed > M1_MAX_CONTEXT_TOKENS - target_width
            || self.draft_pre_committed > M1_MAX_CONTEXT_TOKENS - draft_width
        {
            return Err(SpeculativeKvIndexError::ContextExceeded);
        }
        let target_end = self.target_pre_committed + target_width;
        let draft_end = self.draft_pre_committed + draft_width;
        if self.target_tentative.start != self.target_pre_committed
            || self.target_tentative.end != target_end
            || self.draft_tentative.start != self.draft_pre_committed
            || self.draft_tentative.end != draft_end
        {
            return Err(SpeculativeKvIndexError::TentativeInterval);
        }
        if self.round_anchor >= QWEN3_VOCABULARY_SIZE {
            return Err(SpeculativeKvIndexError::TokenOutOfRange);
        }
        let mut token_index = 0usize;
        while token_index < M1_MAX_SPECULATIVE_KV_DRAFT_TOKENS
            invariant
                token_index <= M1_MAX_SPECULATIVE_KV_DRAFT_TOKENS,
                0 < self.draft_token_count as int
                    <= M1_MAX_SPECULATIVE_KV_DRAFT_TOKENS,
                forall|index: int| 0 <= index < token_index ==>
                    if index < self.draft_token_count as int {
                        #[trigger] self.draft_tokens@[index] < QWEN3_VOCABULARY_SIZE
                    } else {
                        #[trigger] self.draft_tokens@[index] == 0
                    },
            decreases M1_MAX_SPECULATIVE_KV_DRAFT_TOKENS - token_index,
        {
            if token_index < self.draft_token_count as usize {
                if self.draft_tokens[token_index] >= QWEN3_VOCABULARY_SIZE {
                    return Err(SpeculativeKvIndexError::TokenOutOfRange);
                }
            } else if self.draft_tokens[token_index] != 0 {
                return Err(SpeculativeKvIndexError::NoncanonicalUnusedToken);
            }
            token_index += 1;
        }
        assert(self.token_fields_valid());
        let mut accepted = 0usize;
        while accepted < M1_MAX_SPECULATIVE_KV_TARGET_INPUTS
            invariant
                accepted <= M1_MAX_SPECULATIVE_KV_TARGET_INPUTS,
                0 < self.draft_token_count as int
                    <= M1_MAX_SPECULATIVE_KV_DRAFT_TOKENS,
                self.target_pre_committed as int + self.draft_token_count as int + 1
                    <= M1_MAX_CONTEXT_TOKENS,
                self.draft_pre_committed as int + self.draft_token_count as int
                    <= M1_MAX_CONTEXT_TOKENS,
                forall|index: int| 0 <= index < accepted ==>
                    #[trigger] self.commit_entry_valid(index),
            decreases M1_MAX_SPECULATIVE_KV_TARGET_INPUTS - accepted,
        {
            if accepted <= self.draft_token_count as usize {
                let accepted_u32 = match u32::try_from(accepted) {
                    Ok(value) => value,
                    Err(_) => return Err(SpeculativeKvIndexError::CommitEnd),
                };
                let expected_target = self.target_pre_committed + accepted_u32 + 1;
                let expected_draft = self.draft_pre_committed
                    + if accepted < self.draft_token_count as usize {
                        accepted_u32 + 1
                    } else {
                        u32::from(self.draft_token_count)
                    };
                assert(expected_target as int
                    == self.target_pre_committed as int + accepted as int + 1);
                assert(expected_draft as int
                    == self.draft_pre_committed as int
                        + if (accepted as int) < (self.draft_token_count as int) {
                            accepted as int + 1
                        } else {
                            self.draft_token_count as int
                        });
                if self.target_commit_ends[accepted] != expected_target
                    || self.draft_commit_ends[accepted] != expected_draft
                {
                    assert(!self.commit_entry_valid(accepted as int)) by {
                        reveal(SpeculativeKvRoundIndex::commit_entry_valid);
                    }
                    assert(!self.commit_tables_valid()) by {
                        reveal(SpeculativeKvRoundIndex::commit_tables_valid);
                    }
                    return Err(SpeculativeKvIndexError::CommitEnd);
                }
                assert(self.target_commit_ends@[accepted as int]
                    == self.target_pre_committed as int + accepted as int + 1);
                assert(self.draft_commit_ends@[accepted as int]
                    == self.draft_pre_committed as int
                        + if (accepted as int) < (self.draft_token_count as int) {
                            accepted as int + 1
                        } else {
                            self.draft_token_count as int
                        });
            } else if self.target_commit_ends[accepted] != 0
                || self.draft_commit_ends[accepted] != 0
            {
                assert(!self.commit_entry_valid(accepted as int)) by {
                    reveal(SpeculativeKvRoundIndex::commit_entry_valid);
                }
                assert(!self.commit_tables_valid()) by {
                    reveal(SpeculativeKvRoundIndex::commit_tables_valid);
                }
                return Err(SpeculativeKvIndexError::CommitEnd);
            } else {
                assert(self.target_commit_ends@[accepted as int] == 0);
                assert(self.draft_commit_ends@[accepted as int] == 0);
            }
            assert(self.commit_entry_valid(accepted as int)) by {
                reveal(SpeculativeKvRoundIndex::commit_entry_valid);
            }
            assert forall|index: int| 0 <= index < accepted + 1 implies
                #[trigger] self.commit_entry_valid(index) by {
                if index < accepted {
                    assert(self.commit_entry_valid(index));
                } else {
                    assert(index == accepted);
                    assert(self.commit_entry_valid(accepted as int));
                }
            }
            accepted += 1;
        }
        assert(self.commit_tables_valid()) by {
            reveal(SpeculativeKvRoundIndex::commit_tables_valid);
        }
        if !matches!(
            self.correction_bonus,
            CorrectionBonusKvDisposition::DeferredUntilNextStep
        ) {
            return Err(SpeculativeKvIndexError::CorrectionBonusNotDeferred);
        }
        Ok(())
    }

    /// Validates structure and exact external authority in one fail-closed call.
    ///
    /// # Errors
    ///
    /// Returns [`SpeculativeKvIndexError::AuthorityMismatch`] for any stale or
    /// substituted request, epoch, plan identity, or selection.
    pub fn validate_for(
        &self,
        expected_request: RequestId,
        expected_epoch: CompletionEpoch,
        expected_plan_id: &Identity,
        expected_target: Qwen3PlanSelection,
        expected_draft: Qwen3PlanSelection,
    ) -> (result: Result<(), SpeculativeKvIndexError>)
        ensures result.is_ok() == self.valid_for(
            expected_request,
            expected_epoch,
            *expected_plan_id,
            expected_target,
            expected_draft,
        ),
    {
        proof { reveal(SpeculativeKvRoundIndex::valid_for); }
        self.validate()?;
        if self.request.slot() != expected_request.slot()
            || self.request.generation() != expected_request.generation()
            || self.completion_epoch.value != expected_epoch.value
            || !self.plan_id.equals(expected_plan_id)
            || !self.target_selection.matches(expected_target)
            || !self.draft_selection.matches(expected_draft)
        {
            return Err(SpeculativeKvIndexError::AuthorityMismatch);
        }
        Ok(())
    }
}

} // verus!

#[cfg(test)]
mod tests {
    use super::*;

    fn selection(role: Qwen3ModelRole, bucket: Qwen3PlanBucket) -> Qwen3PlanSelection {
        Qwen3PlanSelection {
            role,
            mode: Qwen3ExecutionMode::Speculative,
            bucket,
        }
    }

    fn exact_index(bucket: Qwen3PlanBucket, k: u8) -> SpeculativeKvRoundIndex {
        let base = 41u32;
        let mut draft_tokens = [0; M1_MAX_SPECULATIVE_KV_DRAFT_TOKENS];
        for (index, token) in draft_tokens.iter_mut().enumerate().take(usize::from(k)) {
            *token = 100 + u32::try_from(index).unwrap();
        }
        let mut target_commit_ends = [0; M1_MAX_SPECULATIVE_KV_TARGET_INPUTS];
        let mut draft_commit_ends = [0; M1_MAX_SPECULATIVE_KV_TARGET_INPUTS];
        for accepted in 0..=usize::from(k) {
            let accepted_u32 = u32::try_from(accepted).unwrap();
            target_commit_ends[accepted] = base + accepted_u32 + 1;
            draft_commit_ends[accepted] = base
                + if accepted < usize::from(k) {
                    accepted_u32 + 1
                } else {
                    u32::from(k)
                };
        }
        SpeculativeKvRoundIndex {
            request: RequestId::new(3, 7),
            completion_epoch: CompletionEpoch::new(19),
            plan_id: Identity::new([9; 32]),
            target_selection: selection(Qwen3ModelRole::Target8B, bucket),
            draft_selection: selection(Qwen3ModelRole::Draft06B, bucket),
            draft_token_count: k,
            round_anchor: 77,
            draft_tokens,
            target_pre_committed: base,
            draft_pre_committed: base,
            target_tentative: SpeculativeKvInterval {
                start: base,
                end: base + u32::from(k) + 1,
            },
            draft_tentative: SpeculativeKvInterval {
                start: base,
                end: base + u32::from(k),
            },
            target_commit_ends,
            draft_commit_ends,
            correction_bonus: CorrectionBonusKvDisposition::DeferredUntilNextStep,
        }
    }

    #[test]
    fn every_finite_bucket_has_exact_graph_width_and_commit_table() {
        for (bucket, k) in [
            (Qwen3PlanBucket::SpeculativeS1K4C8192, 4),
            (Qwen3PlanBucket::SpeculativeS8K4C8192, 4),
            (Qwen3PlanBucket::SpeculativeS1K8C8192, 8),
            (Qwen3PlanBucket::SpeculativeS1K16C8192, 16),
        ] {
            let index = exact_index(bucket, k);
            assert_eq!(index.validate(), Ok(()));
            for accepted in 0..=k {
                assert_eq!(
                    index.target_commit_end(accepted),
                    Some(index.target_commit_ends[accepted as usize])
                );
                assert_eq!(
                    index.draft_commit_end(accepted),
                    Some(index.draft_commit_ends[accepted as usize])
                );
            }
            assert_eq!(index.target_commit_end(k + 1), None);
            assert_eq!(index.draft_commit_end(k + 1), None);
        }
    }

    #[test]
    fn target_and_draft_ordinals_bind_exact_input_sources() {
        let index = exact_index(Qwen3PlanBucket::SpeculativeS1K4C8192, 4);
        assert_eq!(
            index.target_input(0),
            Some(SpeculativeKvInputBinding {
                role: Qwen3ModelRole::Target8B,
                graph_ordinal: 0,
                logical_position: 41,
                source: SpeculativeKvInputSource::RoundAnchor { token: 77 },
            })
        );
        assert_eq!(
            index.target_input(4),
            Some(SpeculativeKvInputBinding {
                role: Qwen3ModelRole::Target8B,
                graph_ordinal: 4,
                logical_position: 45,
                source: SpeculativeKvInputSource::DraftCandidate {
                    index: 3,
                    token: 103,
                },
            })
        );
        assert_eq!(index.target_input(5), None);
        assert_eq!(
            index.draft_input(3),
            Some(SpeculativeKvInputBinding {
                role: Qwen3ModelRole::Draft06B,
                graph_ordinal: 3,
                logical_position: 44,
                source: SpeculativeKvInputSource::DraftCandidate {
                    index: 2,
                    token: 102,
                },
            })
        );
        assert_eq!(index.draft_input(4), None);
        assert_eq!(
            index.target_choice(4),
            Some(TargetChoiceBinding {
                choice_index: 4,
                after_target_ordinal: 4,
                use_kind: TargetChoiceUse::LogitOnly,
            })
        );
        assert_eq!(index.target_choice(5), None);
    }

    #[test]
    fn request_epoch_plan_and_selection_substitution_fail_closed() {
        let exact = exact_index(Qwen3PlanBucket::SpeculativeS1K4C8192, 4);
        assert_eq!(
            exact.validate_for(
                exact.request,
                exact.completion_epoch,
                &exact.plan_id,
                exact.target_selection,
                exact.draft_selection,
            ),
            Ok(())
        );
        let mut changed = exact;
        changed.request = RequestId::new(3, 8);
        assert_eq!(changed.validate(), Ok(()));
        assert_eq!(
            changed.validate_for(
                exact.request,
                exact.completion_epoch,
                &exact.plan_id,
                exact.target_selection,
                exact.draft_selection,
            ),
            Err(SpeculativeKvIndexError::AuthorityMismatch)
        );
        changed = exact;
        changed.completion_epoch = CompletionEpoch::new(20);
        assert_eq!(
            changed.validate_for(
                exact.request,
                exact.completion_epoch,
                &exact.plan_id,
                exact.target_selection,
                exact.draft_selection,
            ),
            Err(SpeculativeKvIndexError::AuthorityMismatch)
        );
        changed = exact;
        changed.plan_id = Identity::new([8; 32]);
        assert_eq!(
            changed.validate_for(
                exact.request,
                exact.completion_epoch,
                &exact.plan_id,
                exact.target_selection,
                exact.draft_selection,
            ),
            Err(SpeculativeKvIndexError::AuthorityMismatch)
        );
        changed = exact;
        changed.target_selection.bucket = Qwen3PlanBucket::SpeculativeS1K8C8192;
        assert!(changed.validate().is_err());
        changed = exact;
        changed.draft_selection.role = Qwen3ModelRole::Target8B;
        assert_eq!(changed.validate(), Err(SpeculativeKvIndexError::RoleOrMode));
    }

    #[test]
    fn every_scalar_interval_and_token_field_is_fail_closed() {
        let exact = exact_index(Qwen3PlanBucket::SpeculativeS1K4C8192, 4);
        let mut changed = exact;
        changed.request = RequestId::new(32, 7);
        assert_eq!(changed.validate(), Err(SpeculativeKvIndexError::Request));
        changed = exact;
        changed.completion_epoch = CompletionEpoch::new(0);
        assert_eq!(
            changed.validate(),
            Err(SpeculativeKvIndexError::CompletionEpoch)
        );
        changed = exact;
        changed.plan_id = Identity::new([0; 32]);
        assert_eq!(
            changed.validate(),
            Err(SpeculativeKvIndexError::PlanIdentity)
        );
        changed = exact;
        changed.draft_token_count = 3;
        assert_eq!(
            changed.validate(),
            Err(SpeculativeKvIndexError::DraftLength)
        );
        changed = exact;
        changed.draft_pre_committed += 1;
        assert_eq!(
            changed.validate(),
            Err(SpeculativeKvIndexError::CursorMismatch)
        );
        changed = exact;
        changed.target_tentative.start += 1;
        assert_eq!(
            changed.validate(),
            Err(SpeculativeKvIndexError::TentativeInterval)
        );
        changed = exact;
        changed.draft_tentative.end += 1;
        assert_eq!(
            changed.validate(),
            Err(SpeculativeKvIndexError::TentativeInterval)
        );
        changed = exact;
        changed.round_anchor = QWEN3_VOCABULARY_SIZE;
        assert_eq!(
            changed.validate(),
            Err(SpeculativeKvIndexError::TokenOutOfRange)
        );
        changed = exact;
        changed.draft_tokens[2] = QWEN3_VOCABULARY_SIZE;
        assert_eq!(
            changed.validate(),
            Err(SpeculativeKvIndexError::TokenOutOfRange)
        );
        changed = exact;
        changed.draft_tokens[4] = 1;
        assert_eq!(
            changed.validate(),
            Err(SpeculativeKvIndexError::NoncanonicalUnusedToken)
        );
        changed = exact;
        changed.target_pre_committed = M1_MAX_CONTEXT_TOKENS;
        changed.draft_pre_committed = M1_MAX_CONTEXT_TOKENS;
        assert_eq!(
            changed.validate(),
            Err(SpeculativeKvIndexError::ContextExceeded)
        );
    }

    #[test]
    fn every_live_and_unused_commit_endpoint_is_checked() {
        let exact = exact_index(Qwen3PlanBucket::SpeculativeS1K4C8192, 4);
        for accepted in 0..=4usize {
            let mut target_changed = exact;
            target_changed.target_commit_ends[accepted] += 1;
            assert_eq!(
                target_changed.validate(),
                Err(SpeculativeKvIndexError::CommitEnd)
            );
            let mut draft_changed = exact;
            draft_changed.draft_commit_ends[accepted] += 1;
            assert_eq!(
                draft_changed.validate(),
                Err(SpeculativeKvIndexError::CommitEnd)
            );
        }
        let mut unused = exact;
        unused.target_commit_ends[16] = 1;
        assert_eq!(unused.validate(), Err(SpeculativeKvIndexError::CommitEnd));
        unused = exact;
        unused.draft_commit_ends[16] = 1;
        assert_eq!(unused.validate(), Err(SpeculativeKvIndexError::CommitEnd));
    }

    #[test]
    fn correction_or_bonus_can_only_be_deferred() {
        let exact = exact_index(Qwen3PlanBucket::SpeculativeS1K4C8192, 4);
        let mut changed = exact;
        changed.correction_bonus = CorrectionBonusKvDisposition::TargetResident;
        assert_eq!(
            changed.validate(),
            Err(SpeculativeKvIndexError::CorrectionBonusNotDeferred)
        );
        changed.correction_bonus = CorrectionBonusKvDisposition::DraftResident;
        assert_eq!(
            changed.validate(),
            Err(SpeculativeKvIndexError::CorrectionBonusNotDeferred)
        );
    }
}
