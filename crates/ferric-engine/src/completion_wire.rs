//! Bounded K7 compact-completion wire decoding and custody joins.
//!
//! Raw bytes are decoded and checked against one immutable Ferric [`StepPlan`]
//! before they can be correlated with [`ExactCompletion`]. The byte checker is
//! deliberately inert: neither the bytes nor [`InertCheckedCompletionRecord`]
//! prove readback, queue publication, GPU completion, model content, kernel
//! execution, numerical refinement, inference, or hardware behavior. Production
//! code obtains bytes through
//! [`crate::M1PhysicalRecycledQueueSessionV1::read_and_check_completion`], which
//! binds the exact retained K7 range and scheduler roster before it creates one
//! [`ExactCompletion`]. Calling the inert checker alone grants no completion
//! authority.
//!
//! The wire format does not encode role, mode, or bucket. Those values remain
//! checked host context retained from `StepPlan`; the plan identity is the only
//! corresponding wire field. This module does not authenticate the catalog
//! relation between that identity and the retained selection.

use crate::ExactCompletion;
use core::fmt;
use ferric_qwen_kernels::logits::Qwen3LogitsCompactRecordLayoutV1 as Layout;
use ferric_spec::completion::CompletionEpoch;
use ferric_spec::{
    validate_compact_completion, verify_speculative_completion, CompactCompletionError,
    CompactCompletionRecord, GreedyCommit, Identity, Qwen3ExecutionMode, Qwen3ModelRole,
    Qwen3PlanBucket, Qwen3PlanError, Qwen3PlanSelection, RequestId, SpeculativeCompletionError,
    StepPlan, TokenId, M1_MAX_ACTIVE_SEQUENCES, M1_MAX_COMPLETION_TOKENS,
};

const _: () = {
    assert!(Layout::RECORD_BYTES == 120);
    assert!(Layout::RECORD_BYTES_USIZE == 120);
    assert!(Layout::TOKEN_SLOTS == M1_MAX_COMPLETION_TOKENS);
};

/// Exact semantic expectation supplied beside one immutable logical step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletionWireSemanticExpectation<'a> {
    /// Direct prefill/decode must publish the exact final active-row choice.
    DirectFinalRow {
        /// Expected final active-row argmax choice.
        choice: TokenId,
    },
    /// Speculation must publish the maximal accepted prefix and correction or bonus.
    Speculative {
        /// Exact draft proposal tokens for the selected bucket.
        draft_tokens: &'a [TokenId],
        /// Exact target choices, including the final correction-or-bonus row.
        target_choices: &'a [TokenId],
    },
}

/// Borrowed expected authority and semantics for one K7 record.
///
/// The plan and semantic arrays are host declarations. This value does not
/// authenticate their provenance or join them to loaded device allocations.
#[derive(Clone, Copy, Debug)]
pub struct CompletionWireExpectation<'a> {
    plan: &'a StepPlan,
    semantics: CompletionWireSemanticExpectation<'a>,
}

impl<'a> CompletionWireExpectation<'a> {
    /// Binds an immutable step plan to direct or speculative expectations.
    #[must_use]
    pub const fn new(plan: &'a StepPlan, semantics: CompletionWireSemanticExpectation<'a>) -> Self {
        Self { plan, semantics }
    }

    /// Immutable logical step authority.
    #[must_use]
    pub const fn plan(&self) -> &'a StepPlan {
        self.plan
    }

    /// Expected direct or speculative token semantics.
    #[must_use]
    pub const fn semantics(&self) -> CompletionWireSemanticExpectation<'a> {
        self.semantics
    }
}

/// Inert semantic observation retained after all byte and logical checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckedCompletionSemantics {
    /// Direct mode matched the declared final active-row choice.
    DirectFinalRow {
        /// Exact emitted token.
        token: TokenId,
    },
    /// Speculation matched the maximal prefix and correction-or-bonus relation.
    Speculative {
        /// Maximal accepted draft prefix length.
        accepted_draft_tokens: u8,
        /// Target correction at the first mismatch, or bonus after a full match.
        correction_or_bonus: TokenId,
    },
}

/// Fail-closed K7 wire or semantic validation error.
#[derive(Debug, PartialEq, Eq)]
pub enum CompletionWireError {
    /// The byte slice was truncated or had trailing bytes.
    RecordLength { expected: usize, actual: usize },
    /// One of the two reserved bytes was nonzero.
    ReservedNonzero,
    /// The encoded emitted count exceeded the fixed 17-slot payload.
    EmittedCountOutOfRange { actual: u8 },
    /// K7 only admits request slots below 32.
    RequestSlotOutOfRange { actual: u32 },
    /// K7 rejects generation zero.
    ZeroRequestGeneration,
    /// The plan's role, mode, and bucket were not structurally admitted.
    Selection(Qwen3PlanError),
    /// Compact completion publication is target-only.
    NonTargetRole,
    /// Direct/speculative expectations did not match the selected mode.
    ModeSemanticsMismatch,
    /// The semantic draft length differed from the selected speculative K.
    SpeculativeLengthMismatch { expected: usize, actual: usize },
    /// The record failed the shared logical compact-completion validator.
    Logical(CompactCompletionError),
    /// A direct record did not contain the declared final-row choice.
    DirectFinalRowMismatch { expected: TokenId, actual: TokenId },
    /// A speculative record did not match maximal-prefix greedy verification.
    Speculative(SpeculativeCompletionError),
}

impl fmt::Display for CompletionWireError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "K7 completion record rejected: {self:?}")
    }
}

impl std::error::Error for CompletionWireError {}

/// Inert checked record awaiting an independent completed-readback capability.
///
/// This value intentionally is not `Copy` or `Clone`, but its linear custody is
/// only host bookkeeping. It does not prove that a device wrote the bytes.
#[derive(Debug, PartialEq, Eq)]
pub struct InertCheckedCompletionRecord {
    record: CompactCompletionRecord,
    selection: Qwen3PlanSelection,
    semantics: CheckedCompletionSemantics,
}

impl InertCheckedCompletionRecord {
    /// Borrows the checked logical record.
    #[must_use]
    pub const fn record(&self) -> &CompactCompletionRecord {
        &self.record
    }

    /// Exact host-selected role, mode, and bucket.
    #[must_use]
    pub const fn selection(&self) -> Qwen3PlanSelection {
        self.selection
    }

    /// Inert direct or speculative semantic observation.
    #[must_use]
    pub const fn semantics(&self) -> CheckedCompletionSemantics {
        self.semantics
    }
}

/// Legacy exact-epoch correlation of inert checked bytes with quiescence.
///
/// This wrapper is linear because it contains [`ExactCompletion`]. It still
/// does not prove that the bytes came from the completed submission or a
/// particular host-download range, authenticate model content, or prove
/// inference correctness. Production completed readback uses the physical queue
/// lifecycle's range- and roster-bound join instead.
///
/// ```compile_fail
/// use ferric_engine::EpochJoinedCompletionRecord;
///
/// fn duplicate(joined: EpochJoinedCompletionRecord) {
///     let _first = joined.into_parts();
///     let _second = joined.into_parts();
/// }
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct EpochJoinedCompletionRecord {
    checked: InertCheckedCompletionRecord,
    completion: ExactCompletion,
}

impl EpochJoinedCompletionRecord {
    /// Borrows the checked record without exposing completion authority.
    #[must_use]
    pub const fn checked(&self) -> &InertCheckedCompletionRecord {
        &self.checked
    }

    /// Consumes the join into the logical record and exact completion.
    ///
    /// The returned completion remains the independently supplied token that
    /// may enter `Engine::complete_exact`. This join does not authorize the
    /// inert record to influence production state.
    #[must_use]
    pub fn into_parts(self) -> (CompactCompletionRecord, ExactCompletion) {
        (self.checked.record, self.completion)
    }
}

/// Ownership-preserving failure from the exact-epoch join.
///
/// ```compile_fail
/// use ferric_engine::CompletionEpochJoinFailure;
///
/// fn recover_twice(failure: CompletionEpochJoinFailure) {
///     let _first = failure.into_parts();
///     let _second = failure.into_parts();
/// }
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct CompletionEpochJoinFailure {
    checked: InertCheckedCompletionRecord,
    completion: ExactCompletion,
}

impl CompletionEpochJoinFailure {
    /// Recovers both unchanged inputs after an epoch mismatch.
    #[must_use]
    pub fn into_parts(self) -> (InertCheckedCompletionRecord, ExactCompletion) {
        (self.checked, self.completion)
    }
}

/// Decodes and checks one exact K7 record without creating completion authority.
///
/// All structural, identity, role/mode, bound, and token-semantic checks finish
/// before a value is returned. The function does not mutate engine state.
///
/// # Errors
///
/// Returns [`CompletionWireError`] for any byte-layout, selection, identity,
/// direct-final-row, or speculative maximal-prefix mismatch.
pub fn check_inert_completion_record(
    bytes: &[u8],
    expectation: CompletionWireExpectation<'_>,
) -> Result<InertCheckedCompletionRecord, CompletionWireError> {
    let record = decode_completion_record(bytes)?;
    let selection = expectation.plan.selection();
    selection
        .validate()
        .map_err(CompletionWireError::Selection)?;
    if selection.role != Qwen3ModelRole::Target8B {
        return Err(CompletionWireError::NonTargetRole);
    }

    let expected_request = expectation.plan.request();
    let expected_epoch = expectation.plan.completion_epoch();
    let expected_plan_id = expectation.plan.plan_id();
    let semantics = match (selection.mode, expectation.semantics) {
        (
            Qwen3ExecutionMode::Prefill | Qwen3ExecutionMode::Decode,
            CompletionWireSemanticExpectation::DirectFinalRow { choice },
        ) => {
            validate_compact_completion(
                &record,
                expected_request,
                expected_epoch,
                expected_plan_id,
                0,
            )
            .map_err(CompletionWireError::Logical)?;
            let actual = record.emitted_tokens[0];
            if actual != choice {
                return Err(CompletionWireError::DirectFinalRowMismatch {
                    expected: choice,
                    actual,
                });
            }
            CheckedCompletionSemantics::DirectFinalRow { token: actual }
        }
        (
            Qwen3ExecutionMode::Speculative,
            CompletionWireSemanticExpectation::Speculative {
                draft_tokens,
                target_choices,
            },
        ) => {
            let expected = speculative_k(selection.bucket)
                .ok_or(CompletionWireError::ModeSemanticsMismatch)?;
            if draft_tokens.len() != expected {
                return Err(CompletionWireError::SpeculativeLengthMismatch {
                    expected,
                    actual: draft_tokens.len(),
                });
            }
            let commit = verify_speculative_completion(
                &record,
                expected_request,
                expected_epoch,
                expected_plan_id,
                draft_tokens,
                target_choices,
            )
            .map_err(CompletionWireError::Speculative)?;
            checked_speculative_semantics(&commit)
        }
        _ => return Err(CompletionWireError::ModeSemanticsMismatch),
    };

    Ok(InertCheckedCompletionRecord {
        record,
        selection,
        semantics,
    })
}

/// Correlates an inert checked record with separately obtained completion custody.
///
/// This compatibility utility checks only that both inputs name the same epoch.
/// It does not establish completed-readback provenance. Production code should
/// retain the checked records and single [`ExactCompletion`] minted together by
/// [`crate::M1PhysicalRecycledQueueSessionV1::read_and_check_completion`].
///
/// # Errors
///
/// Returns both unchanged inputs when their epochs differ.
pub fn bind_inert_completion_epoch(
    checked: InertCheckedCompletionRecord,
    completion: ExactCompletion,
) -> Result<EpochJoinedCompletionRecord, Box<CompletionEpochJoinFailure>> {
    if checked.record.epoch != completion.epoch() {
        return Err(Box::new(CompletionEpochJoinFailure {
            checked,
            completion,
        }));
    }
    Ok(EpochJoinedCompletionRecord {
        checked,
        completion,
    })
}

fn checked_speculative_semantics(commit: &GreedyCommit) -> CheckedCompletionSemantics {
    let accepted_draft_tokens = u8::try_from(commit.accepted_draft_tokens())
        .expect("verified M1 speculative accepted length is at most 16");
    CheckedCompletionSemantics::Speculative {
        accepted_draft_tokens,
        correction_or_bonus: commit.target_correction_or_bonus(),
    }
}

fn speculative_k(bucket: Qwen3PlanBucket) -> Option<usize> {
    match bucket {
        Qwen3PlanBucket::SpeculativeS1K4C8192 | Qwen3PlanBucket::SpeculativeS8K4C8192 => Some(4),
        Qwen3PlanBucket::SpeculativeS1K8C8192 => Some(8),
        Qwen3PlanBucket::SpeculativeS1K16C8192 => Some(16),
        _ => None,
    }
}

fn decode_completion_record(bytes: &[u8]) -> Result<CompactCompletionRecord, CompletionWireError> {
    if bytes.len() != Layout::RECORD_BYTES_USIZE {
        return Err(CompletionWireError::RecordLength {
            expected: Layout::RECORD_BYTES_USIZE,
            actual: bytes.len(),
        });
    }
    if bytes[Layout::RESERVED_OFFSET..Layout::RESERVED_OFFSET + Layout::RESERVED_BYTES]
        .iter()
        .any(|byte| *byte != 0)
    {
        return Err(CompletionWireError::ReservedNonzero);
    }
    let emitted_token_count = bytes[Layout::EMITTED_TOKEN_COUNT_OFFSET];
    if usize::from(emitted_token_count) > Layout::TOKEN_SLOTS {
        return Err(CompletionWireError::EmittedCountOutOfRange {
            actual: emitted_token_count,
        });
    }

    let slot = read_u32(bytes, Layout::REQUEST_SLOT_OFFSET);
    if slot >= M1_MAX_ACTIVE_SEQUENCES {
        return Err(CompletionWireError::RequestSlotOutOfRange { actual: slot });
    }
    let generation = read_u32(bytes, Layout::REQUEST_GENERATION_OFFSET);
    if generation == 0 {
        return Err(CompletionWireError::ZeroRequestGeneration);
    }
    let epoch = read_u64(bytes, Layout::COMPLETION_EPOCH_OFFSET);
    let mut plan_id = [0; Layout::PLAN_IDENTITY_BYTES];
    plan_id.copy_from_slice(
        &bytes[Layout::PLAN_IDENTITY_OFFSET
            ..Layout::PLAN_IDENTITY_OFFSET + Layout::PLAN_IDENTITY_BYTES],
    );
    let mut emitted_tokens = [0; M1_MAX_COMPLETION_TOKENS];
    for (index, token) in emitted_tokens.iter_mut().enumerate() {
        let offset = Layout::token_offset(index).expect("fixed token array is layout-bounded");
        *token = read_u32(bytes, offset);
    }

    Ok(CompactCompletionRecord {
        request: RequestId::new(slot, generation),
        epoch: CompletionEpoch::new(epoch),
        plan_id: Identity::new(plan_id),
        accepted_draft_tokens: bytes[Layout::ACCEPTED_DRAFT_TOKENS_OFFSET],
        emitted_token_count,
        emitted_tokens,
    })
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(
        bytes[offset..offset + size_of::<u32>()]
            .try_into()
            .expect("the exact record length dominates every layout offset"),
    )
}

fn read_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(
        bytes[offset..offset + size_of::<u64>()]
            .try_into()
            .expect("the exact record length dominates every layout offset"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const REQUEST: RequestId = RequestId::new(3, 9);
    const EPOCH: CompletionEpoch = CompletionEpoch::new(12);
    const PLAN_ID: Identity = Identity::new([5; 32]);

    fn selection(mode: Qwen3ExecutionMode, bucket: Qwen3PlanBucket) -> Qwen3PlanSelection {
        Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode,
            bucket,
        }
    }

    fn plan(mode: Qwen3ExecutionMode, bucket: Qwen3PlanBucket) -> StepPlan {
        StepPlan::new(REQUEST, EPOCH, PLAN_ID, selection(mode, bucket))
    }

    fn encode(
        request: RequestId,
        epoch: CompletionEpoch,
        plan_id: &Identity,
        accepted: u8,
        emitted: &[TokenId],
    ) -> [u8; Layout::RECORD_BYTES_USIZE] {
        let mut bytes = [0; Layout::RECORD_BYTES_USIZE];
        bytes[Layout::REQUEST_SLOT_OFFSET..Layout::REQUEST_SLOT_OFFSET + 4]
            .copy_from_slice(&request.slot().to_le_bytes());
        bytes[Layout::REQUEST_GENERATION_OFFSET..Layout::REQUEST_GENERATION_OFFSET + 4]
            .copy_from_slice(&request.generation().to_le_bytes());
        bytes[Layout::COMPLETION_EPOCH_OFFSET..Layout::COMPLETION_EPOCH_OFFSET + 8]
            .copy_from_slice(&epoch.value().to_le_bytes());
        bytes[Layout::PLAN_IDENTITY_OFFSET
            ..Layout::PLAN_IDENTITY_OFFSET + Layout::PLAN_IDENTITY_BYTES]
            .copy_from_slice(plan_id.as_bytes());
        bytes[Layout::ACCEPTED_DRAFT_TOKENS_OFFSET] = accepted;
        bytes[Layout::EMITTED_TOKEN_COUNT_OFFSET] =
            u8::try_from(emitted.len()).expect("test payload fits");
        for (index, token) in emitted.iter().enumerate() {
            let offset = Layout::token_offset(index).expect("test payload is bounded");
            bytes[offset..offset + 4].copy_from_slice(&token.to_le_bytes());
        }
        bytes
    }

    fn direct_bytes() -> [u8; Layout::RECORD_BYTES_USIZE] {
        encode(REQUEST, EPOCH, &PLAN_ID, 0, &[17])
    }

    fn direct_checked(bytes: &[u8]) -> Result<InertCheckedCompletionRecord, CompletionWireError> {
        let plan = plan(Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS1C8192);
        check_inert_completion_record(
            bytes,
            CompletionWireExpectation::new(
                &plan,
                CompletionWireSemanticExpectation::DirectFinalRow { choice: 17 },
            ),
        )
    }

    #[test]
    fn exact_direct_final_row_decodes_and_checks() {
        let checked = direct_checked(&direct_bytes()).unwrap();
        assert_eq!(checked.record().request, REQUEST);
        assert_eq!(checked.record().epoch, EPOCH);
        assert_eq!(checked.record().plan_id, PLAN_ID);
        assert_eq!(
            checked.semantics(),
            CheckedCompletionSemantics::DirectFinalRow { token: 17 }
        );
    }

    #[test]
    fn exact_speculative_prefix_correction_and_bonus_check() {
        let plan = plan(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
        );
        for (draft, target, accepted, emitted, correction_or_bonus) in [
            (
                &[3, 4, 5, 6][..],
                &[3, 4, 9, 7, 8][..],
                2,
                &[3, 4, 9][..],
                9,
            ),
            (
                &[3, 4, 5, 6][..],
                &[3, 4, 5, 6, 8][..],
                4,
                &[3, 4, 5, 6, 8][..],
                8,
            ),
        ] {
            let bytes = encode(REQUEST, EPOCH, &PLAN_ID, accepted, emitted);
            let checked = check_inert_completion_record(
                &bytes,
                CompletionWireExpectation::new(
                    &plan,
                    CompletionWireSemanticExpectation::Speculative {
                        draft_tokens: draft,
                        target_choices: target,
                    },
                ),
            )
            .unwrap();
            assert_eq!(
                checked.semantics(),
                CheckedCompletionSemantics::Speculative {
                    accepted_draft_tokens: accepted,
                    correction_or_bonus,
                }
            );
        }
    }

    #[test]
    fn truncation_trailing_reserved_and_count_fail_closed() {
        let exact = direct_bytes();
        assert_eq!(
            direct_checked(&exact[..Layout::RECORD_BYTES_USIZE - 1]),
            Err(CompletionWireError::RecordLength {
                expected: Layout::RECORD_BYTES_USIZE,
                actual: Layout::RECORD_BYTES_USIZE - 1,
            })
        );
        let mut trailing = exact.to_vec();
        trailing.push(0);
        assert_eq!(
            direct_checked(&trailing),
            Err(CompletionWireError::RecordLength {
                expected: Layout::RECORD_BYTES_USIZE,
                actual: Layout::RECORD_BYTES_USIZE + 1,
            })
        );
        for offset in Layout::RESERVED_OFFSET..Layout::RESERVED_OFFSET + Layout::RESERVED_BYTES {
            let mut changed = exact;
            changed[offset] = 1;
            assert_eq!(
                direct_checked(&changed),
                Err(CompletionWireError::ReservedNonzero)
            );
        }
        let mut changed = exact;
        changed[Layout::EMITTED_TOKEN_COUNT_OFFSET] = 18;
        assert_eq!(
            direct_checked(&changed),
            Err(CompletionWireError::EmittedCountOutOfRange { actual: 18 })
        );
    }

    #[test]
    fn every_authority_field_and_little_endian_order_fail_closed() {
        let exact = direct_bytes();
        for (offset, error) in [
            (
                Layout::REQUEST_SLOT_OFFSET,
                CompletionWireError::Logical(CompactCompletionError::RequestMismatch),
            ),
            (
                Layout::REQUEST_GENERATION_OFFSET,
                CompletionWireError::Logical(CompactCompletionError::RequestMismatch),
            ),
            (
                Layout::COMPLETION_EPOCH_OFFSET,
                CompletionWireError::Logical(CompactCompletionError::EpochMismatch),
            ),
        ] {
            let mut changed = exact;
            changed[offset] ^= 1;
            assert_eq!(direct_checked(&changed), Err(error));
        }
        for index in 0..Layout::PLAN_IDENTITY_BYTES {
            let mut changed = exact;
            changed[Layout::PLAN_IDENTITY_OFFSET + index] ^= 1;
            assert_eq!(
                direct_checked(&changed),
                Err(CompletionWireError::Logical(
                    CompactCompletionError::PlanIdentityMismatch
                ))
            );
        }

        for (offset, width) in [
            (Layout::REQUEST_SLOT_OFFSET, 4),
            (Layout::REQUEST_GENERATION_OFFSET, 4),
            (Layout::COMPLETION_EPOCH_OFFSET, 8),
            (Layout::TOKENS_OFFSET, 4),
        ] {
            let mut changed = exact;
            changed[offset..offset + width].reverse();
            assert!(direct_checked(&changed).is_err());
        }
    }

    #[test]
    fn slot_generation_counts_and_all_token_slots_fail_closed() {
        let mut changed = direct_bytes();
        changed[Layout::REQUEST_SLOT_OFFSET..Layout::REQUEST_SLOT_OFFSET + 4]
            .copy_from_slice(&M1_MAX_ACTIVE_SEQUENCES.to_le_bytes());
        assert_eq!(
            direct_checked(&changed),
            Err(CompletionWireError::RequestSlotOutOfRange {
                actual: M1_MAX_ACTIVE_SEQUENCES
            })
        );
        changed = direct_bytes();
        changed[Layout::REQUEST_GENERATION_OFFSET..Layout::REQUEST_GENERATION_OFFSET + 4]
            .copy_from_slice(&0_u32.to_le_bytes());
        assert_eq!(
            direct_checked(&changed),
            Err(CompletionWireError::ZeroRequestGeneration)
        );
        changed = direct_bytes();
        changed[Layout::ACCEPTED_DRAFT_TOKENS_OFFSET] = 1;
        assert_eq!(
            direct_checked(&changed),
            Err(CompletionWireError::Logical(
                CompactCompletionError::AcceptedLengthOutOfRange
            ))
        );
        changed = direct_bytes();
        changed[Layout::EMITTED_TOKEN_COUNT_OFFSET] = 2;
        assert_eq!(
            direct_checked(&changed),
            Err(CompletionWireError::Logical(
                CompactCompletionError::EmittedLengthMismatch
            ))
        );
        for index in 0..Layout::TOKEN_SLOTS {
            changed = direct_bytes();
            let offset = Layout::token_offset(index).unwrap();
            let token: TokenId = if index == 0 { 18 } else { 1 };
            changed[offset..offset + 4].copy_from_slice(&token.to_le_bytes());
            assert!(direct_checked(&changed).is_err(), "token slot {index}");
        }
        changed = direct_bytes();
        changed[Layout::TOKENS_OFFSET..Layout::TOKENS_OFFSET + 4]
            .copy_from_slice(&ferric_spec::QWEN3_VOCABULARY_SIZE.to_le_bytes());
        assert_eq!(
            direct_checked(&changed),
            Err(CompletionWireError::Logical(
                CompactCompletionError::TokenOutOfRange
            ))
        );
    }

    #[test]
    fn role_mode_bucket_and_semantic_substitutions_fail_closed() {
        let bytes = direct_bytes();
        let draft = StepPlan::new(
            REQUEST,
            EPOCH,
            PLAN_ID,
            Qwen3PlanSelection {
                role: Qwen3ModelRole::Draft06B,
                mode: Qwen3ExecutionMode::Decode,
                bucket: Qwen3PlanBucket::DecodeS1C8192,
            },
        );
        assert_eq!(
            check_inert_completion_record(
                &bytes,
                CompletionWireExpectation::new(
                    &draft,
                    CompletionWireSemanticExpectation::DirectFinalRow { choice: 17 },
                ),
            ),
            Err(CompletionWireError::NonTargetRole)
        );

        let invalid_mode = StepPlan::new(
            REQUEST,
            EPOCH,
            PLAN_ID,
            selection(Qwen3ExecutionMode::Decode, Qwen3PlanBucket::PrefillS1T128),
        );
        assert!(matches!(
            check_inert_completion_record(
                &bytes,
                CompletionWireExpectation::new(
                    &invalid_mode,
                    CompletionWireSemanticExpectation::DirectFinalRow { choice: 17 },
                ),
            ),
            Err(CompletionWireError::Selection(
                Qwen3PlanError::ModeBucketMismatch
            ))
        ));

        let speculative = plan(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
        );
        assert_eq!(
            check_inert_completion_record(
                &bytes,
                CompletionWireExpectation::new(
                    &speculative,
                    CompletionWireSemanticExpectation::DirectFinalRow { choice: 17 },
                ),
            ),
            Err(CompletionWireError::ModeSemanticsMismatch)
        );
        assert_eq!(
            direct_checked(&encode(REQUEST, EPOCH, &PLAN_ID, 0, &[18])),
            Err(CompletionWireError::DirectFinalRowMismatch {
                expected: 17,
                actual: 18,
            })
        );
    }

    #[test]
    fn speculative_bucket_length_and_maximal_prefix_drift_fail_closed() {
        let plan = plan(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
        );
        let short_draft = [3, 4, 5];
        let short_target = [3, 4, 5, 6];
        let bytes = encode(REQUEST, EPOCH, &PLAN_ID, 3, &[3, 4, 5, 6]);
        assert_eq!(
            check_inert_completion_record(
                &bytes,
                CompletionWireExpectation::new(
                    &plan,
                    CompletionWireSemanticExpectation::Speculative {
                        draft_tokens: &short_draft,
                        target_choices: &short_target,
                    },
                ),
            ),
            Err(CompletionWireError::SpeculativeLengthMismatch {
                expected: 4,
                actual: 3,
            })
        );

        let draft = [3, 4, 5, 6];
        let target = [3, 4, 9, 7, 8];
        let nonmaximal = encode(REQUEST, EPOCH, &PLAN_ID, 1, &[3, 4]);
        assert!(matches!(
            check_inert_completion_record(
                &nonmaximal,
                CompletionWireExpectation::new(
                    &plan,
                    CompletionWireSemanticExpectation::Speculative {
                        draft_tokens: &draft,
                        target_choices: &target,
                    },
                ),
            ),
            Err(CompletionWireError::Speculative(
                SpeculativeCompletionError::AcceptedLengthMismatch
                    | SpeculativeCompletionError::EmittedTokenMismatch
            ))
        ));
        let wrong_correction = encode(REQUEST, EPOCH, &PLAN_ID, 2, &[3, 4, 10]);
        assert_eq!(
            check_inert_completion_record(
                &wrong_correction,
                CompletionWireExpectation::new(
                    &plan,
                    CompletionWireSemanticExpectation::Speculative {
                        draft_tokens: &draft,
                        target_choices: &target,
                    },
                ),
            ),
            Err(CompletionWireError::Speculative(
                SpeculativeCompletionError::EmittedTokenMismatch
            ))
        );
    }

    #[test]
    fn exact_completion_join_is_epoch_bound_and_ownership_preserving() {
        let checked = direct_checked(&direct_bytes()).unwrap();
        let wrong = ExactCompletion::from_contracted_hsa_quiescence(CompletionEpoch::new(13));
        let failure = bind_inert_completion_epoch(checked, wrong).unwrap_err();
        let (checked, completion) = failure.into_parts();
        assert_eq!(checked.record().epoch, EPOCH);
        assert_eq!(completion.epoch(), CompletionEpoch::new(13));

        let exact = ExactCompletion::from_contracted_hsa_quiescence(EPOCH);
        let joined = bind_inert_completion_epoch(checked, exact).unwrap();
        let (record, completion) = joined.into_parts();
        assert_eq!(record.epoch, EPOCH);
        assert_eq!(completion.epoch(), EPOCH);
    }
}
