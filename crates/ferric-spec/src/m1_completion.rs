//! Deterministic M1 argmax and compact completion-record semantics.

use crate::completion::CompletionEpoch;
use crate::{Identity, RequestId, TokenId, QWEN3_VOCABULARY_SIZE};
use vstd::prelude::*;

verus! {

/// Maximum tokens published by one `K <= 16` greedy speculative round.
pub const M1_MAX_COMPLETION_TOKENS: usize = 17;

/// Untrusted compact result written by the final logits/completion device graph.
///
/// The record is accepted only after [`validate_compact_completion`] binds it
/// to the exact request generation, completion epoch, plan identity, and draft
/// length. Slots beyond `emitted_token_count` must be zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompactCompletionRecord {
    pub request: RequestId,
    pub epoch: CompletionEpoch,
    pub plan_id: Identity,
    pub accepted_draft_tokens: u8,
    pub emitted_token_count: u8,
    pub emitted_tokens: [TokenId; M1_MAX_COMPLETION_TOKENS],
}

impl CompactCompletionRecord {
    pub closed spec fn emitted_spec(&self) -> Seq<TokenId> {
        self.emitted_tokens@
    }
}

pub proof fn compact_completion_emitted_view(record: &CompactCompletionRecord)
    ensures record.emitted_spec() == record.emitted_tokens@,
{
}

/// Fail-closed rejection for deterministic M1 result publication.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompactCompletionError {
    ExpectedPlanIdentityAbsent,
    RequestMismatch,
    EpochMismatch,
    PlanIdentityMismatch,
    DraftLengthOutOfRange,
    AcceptedLengthOutOfRange,
    EmittedLengthMismatch,
    TokenOutOfRange,
    NonzeroUnusedToken,
    ScoreCountMismatch,
}

pub open spec fn identity_present(identity: Identity) -> bool {
    exists|index: int|
        0 <= index < identity.bytes_spec().len()
            && identity.bytes_spec()[index] != 0
}

/// Scalar identity and length header of one compact completion record.
pub open spec fn compact_completion_header_matches(
    record: CompactCompletionRecord,
    expected_request: RequestId,
    expected_epoch: CompletionEpoch,
    expected_plan_id: Identity,
    draft_token_count: u8,
) -> bool {
    identity_present(expected_plan_id)
        && record.request.slot_spec() == expected_request.slot_spec()
        && record.request.generation_spec() == expected_request.generation_spec()
        && record.epoch.value == expected_epoch.value
        && record.plan_id.bytes_spec() == expected_plan_id.bytes_spec()
        && draft_token_count <= 16
        && record.accepted_draft_tokens <= draft_token_count
        && record.emitted_token_count as int == record.accepted_draft_tokens as int + 1
        && record.emitted_token_count as int <= M1_MAX_COMPLETION_TOKENS as int
}

pub open spec fn compact_completion_live_tokens_match(
    record: CompactCompletionRecord,
) -> bool {
    forall|index: int|
        0 <= index < record.emitted_token_count as int
            ==> record.emitted_spec()[index] < QWEN3_VOCABULARY_SIZE
}

pub open spec fn compact_completion_unused_tokens_match(
    record: CompactCompletionRecord,
) -> bool {
    forall|index: int|
        record.emitted_token_count as int <= index < M1_MAX_COMPLETION_TOKENS as int
            ==> record.emitted_spec()[index] == 0
}

/// Mathematical acceptance relation for one compact completion record.
pub open spec fn compact_completion_matches(
    record: CompactCompletionRecord,
    expected_request: RequestId,
    expected_epoch: CompletionEpoch,
    expected_plan_id: Identity,
    draft_token_count: u8,
) -> bool {
    compact_completion_header_matches(
        record,
        expected_request,
        expected_epoch,
        expected_plan_id,
        draft_token_count,
    ) && compact_completion_live_tokens_match(record)
        && compact_completion_unused_tokens_match(record)
}

/// Validates an untrusted device completion before any token or state is
/// published to the logical engine.
///
/// # Errors
///
/// Returns [`CompactCompletionError`] unless every identity, bound, live
/// token, and canonical unused slot matches the exact expected round.
pub fn validate_compact_completion(
    record: &CompactCompletionRecord,
    expected_request: RequestId,
    expected_epoch: CompletionEpoch,
    expected_plan_id: &Identity,
    draft_token_count: u8,
) -> (result: Result<(), CompactCompletionError>)
    ensures
        result.is_ok() == compact_completion_matches(
            *record,
            expected_request,
            expected_epoch,
            *expected_plan_id,
            draft_token_count,
        ),
{
    if !expected_plan_id.is_present() {
        return Err(CompactCompletionError::ExpectedPlanIdentityAbsent);
    }
    if record.request.slot() != expected_request.slot()
        || record.request.generation() != expected_request.generation()
    {
        return Err(CompactCompletionError::RequestMismatch);
    }
    if record.epoch.value != expected_epoch.value {
        return Err(CompactCompletionError::EpochMismatch);
    }
    if !record.plan_id.equals(expected_plan_id) {
        return Err(CompactCompletionError::PlanIdentityMismatch);
    }
    if draft_token_count > 16 {
        return Err(CompactCompletionError::DraftLengthOutOfRange);
    }
    if record.accepted_draft_tokens > draft_token_count {
        return Err(CompactCompletionError::AcceptedLengthOutOfRange);
    }
    let expected_emitted = record.accepted_draft_tokens + 1;
    if record.emitted_token_count != expected_emitted
        || record.emitted_token_count as usize > M1_MAX_COMPLETION_TOKENS
    {
        return Err(CompactCompletionError::EmittedLengthMismatch);
    }
    assert(compact_completion_header_matches(
        *record,
        expected_request,
        expected_epoch,
        *expected_plan_id,
        draft_token_count,
    ));

    let mut index = 0;
    while index < record.emitted_token_count as usize
        invariant
            compact_completion_header_matches(
                *record,
                expected_request,
                expected_epoch,
                *expected_plan_id,
                draft_token_count,
            ),
            index <= record.emitted_token_count as int,
            record.emitted_token_count as int <= M1_MAX_COMPLETION_TOKENS as int,
            forall|prior: int|
                0 <= prior < index
                    ==> record.emitted_spec()[prior] < QWEN3_VOCABULARY_SIZE,
        decreases record.emitted_token_count as int - index,
    {
        if record.emitted_tokens[index] >= QWEN3_VOCABULARY_SIZE {
            assert(record.emitted_spec()[index as int] == record.emitted_tokens[index as int]);
            assert(!compact_completion_matches(
                *record,
                expected_request,
                expected_epoch,
                *expected_plan_id,
                draft_token_count,
            )) by {
                assert(!(forall|position: int|
                    0 <= position < record.emitted_token_count as int
                        ==> record.emitted_spec()[position] < QWEN3_VOCABULARY_SIZE));
            }
            return Err(CompactCompletionError::TokenOutOfRange);
        }
        index += 1;
    }
    while index < M1_MAX_COMPLETION_TOKENS
        invariant
            compact_completion_header_matches(
                *record,
                expected_request,
                expected_epoch,
                *expected_plan_id,
                draft_token_count,
            ),
            record.emitted_token_count as int <= index <= M1_MAX_COMPLETION_TOKENS as int,
            forall|prior: int|
                0 <= prior < record.emitted_token_count as int
                    ==> record.emitted_spec()[prior] < QWEN3_VOCABULARY_SIZE,
            forall|prior: int|
                record.emitted_token_count as int <= prior < index
                    ==> record.emitted_spec()[prior] == 0,
        decreases M1_MAX_COMPLETION_TOKENS - index,
    {
        if record.emitted_tokens[index] != 0 {
            assert(record.emitted_spec()[index as int] == record.emitted_tokens[index as int]);
            assert(record.emitted_token_count as int <= index as int);
            assert((index as int) < M1_MAX_COMPLETION_TOKENS as int);
            assert(record.emitted_spec()[index as int] != 0);
            assert(!compact_completion_unused_tokens_match(*record)) by {
                reveal(compact_completion_unused_tokens_match);
                assert(!(forall|position: int|
                    record.emitted_token_count as int
                            <= position < M1_MAX_COMPLETION_TOKENS as int
                        ==> record.emitted_spec()[position] == 0)) by {
                    if forall|position: int|
                        record.emitted_token_count as int
                                <= position < M1_MAX_COMPLETION_TOKENS as int
                            ==> record.emitted_spec()[position] == 0
                    {
                        assert(record.emitted_spec()[index as int] == 0);
                        assert(false);
                    }
                }
            }
            assert(!compact_completion_matches(
                *record,
                expected_request,
                expected_epoch,
                *expected_plan_id,
                draft_token_count,
            )) by {
                reveal(compact_completion_matches);
            }
            return Err(CompactCompletionError::NonzeroUnusedToken);
        }
        index += 1;
    }
    Ok(())
}

/// Mathematical lowest-token tie-breaking argmax relation.
pub open spec fn is_lowest_argmax(scores: Seq<i64>, token: TokenId) -> bool {
    scores.len() == QWEN3_VOCABULARY_SIZE
        && token < QWEN3_VOCABULARY_SIZE
        && forall|index: int|
            0 <= index < scores.len() ==> scores[token as int] >= scores[index]
        && forall|index: int|
            0 <= index < token as int ==> scores[index] < scores[token as int]
}

/// Selects the maximum ordered logit, retaining the lowest token ID on ties.
///
/// The input is an integer total-order abstraction. Mapping BF16/FP32 machine
/// values, NaNs, and signed zero into this order remains a separate numerical
/// contract.
///
/// # Errors
///
/// Returns [`CompactCompletionError::ScoreCountMismatch`] unless there is
/// exactly one score for every pinned Qwen3 vocabulary entry.
pub fn select_lowest_argmax(
    scores: &[i64],
) -> (result: Result<TokenId, CompactCompletionError>)
    ensures
        match result {
            Ok(token) => is_lowest_argmax(scores@, token),
            Err(CompactCompletionError::ScoreCountMismatch) => {
                scores@.len() != QWEN3_VOCABULARY_SIZE
            },
            Err(_) => false,
        },
{
    if scores.len() != QWEN3_VOCABULARY_SIZE as usize {
        return Err(CompactCompletionError::ScoreCountMismatch);
    }
    let mut best = 0u32;
    let mut index = 1u32;
    while (index as usize) < scores.len()
        invariant
            scores@.len() == QWEN3_VOCABULARY_SIZE,
            0 <= best < index <= scores@.len(),
            forall|prior: int|
                0 <= prior < index ==> scores@[best as int] >= scores@[prior],
            forall|prior: int|
                0 <= prior < best ==> scores@[prior] < scores@[best as int],
        decreases scores@.len() - index as int,
    {
        if scores[index as usize] > scores[best as usize] {
            best = index;
        }
        index += 1;
    }
    assert(best < QWEN3_VOCABULARY_SIZE);
    Ok(best)
}

} // verus!

#[cfg(test)]
mod tests {
    use super::{
        select_lowest_argmax, validate_compact_completion, CompactCompletionError,
        CompactCompletionRecord, M1_MAX_COMPLETION_TOKENS,
    };
    use crate::completion::CompletionEpoch;
    use crate::{Identity, RequestId, QWEN3_VOCABULARY_SIZE};

    fn record() -> CompactCompletionRecord {
        let mut tokens = [0; M1_MAX_COMPLETION_TOKENS];
        tokens[0] = 7;
        tokens[1] = 11;
        tokens[2] = 13;
        CompactCompletionRecord {
            request: RequestId::new(3, 9),
            epoch: CompletionEpoch::new(12),
            plan_id: Identity::new([5; 32]),
            accepted_draft_tokens: 2,
            emitted_token_count: 3,
            emitted_tokens: tokens,
        }
    }

    #[test]
    fn exact_completion_record_is_accepted() {
        assert_eq!(
            validate_compact_completion(
                &record(),
                RequestId::new(3, 9),
                CompletionEpoch::new(12),
                &Identity::new([5; 32]),
                4,
            ),
            Ok(())
        );
    }

    #[test]
    fn stale_identity_bounds_and_unused_slots_fail_closed() {
        let expected = Identity::new([5; 32]);
        let mut changed = record();
        changed.request = RequestId::new(3, 10);
        assert_eq!(
            validate_compact_completion(
                &changed,
                RequestId::new(3, 9),
                CompletionEpoch::new(12),
                &expected,
                4,
            ),
            Err(CompactCompletionError::RequestMismatch)
        );

        changed = record();
        changed.accepted_draft_tokens = 5;
        assert_eq!(
            validate_compact_completion(
                &changed,
                RequestId::new(3, 9),
                CompletionEpoch::new(12),
                &expected,
                4,
            ),
            Err(CompactCompletionError::AcceptedLengthOutOfRange)
        );

        changed = record();
        changed.emitted_tokens[16] = 1;
        assert_eq!(
            validate_compact_completion(
                &changed,
                RequestId::new(3, 9),
                CompletionEpoch::new(12),
                &expected,
                4,
            ),
            Err(CompactCompletionError::NonzeroUnusedToken)
        );
    }

    #[test]
    fn every_completion_header_and_live_token_drift_fails_closed() {
        let expected = Identity::new([5; 32]);
        let expected_request = RequestId::new(3, 9);
        let expected_epoch = CompletionEpoch::new(12);

        assert_eq!(
            validate_compact_completion(
                &record(),
                expected_request,
                expected_epoch,
                &Identity::new([0; 32]),
                4,
            ),
            Err(CompactCompletionError::ExpectedPlanIdentityAbsent)
        );

        let mut changed = record();
        changed.epoch = CompletionEpoch::new(13);
        assert_eq!(
            validate_compact_completion(&changed, expected_request, expected_epoch, &expected, 4),
            Err(CompactCompletionError::EpochMismatch)
        );

        assert_eq!(
            validate_compact_completion(
                &record(),
                expected_request,
                expected_epoch,
                &Identity::new([6; 32]),
                4,
            ),
            Err(CompactCompletionError::PlanIdentityMismatch)
        );

        assert_eq!(
            validate_compact_completion(&record(), expected_request, expected_epoch, &expected, 17),
            Err(CompactCompletionError::DraftLengthOutOfRange)
        );

        changed = record();
        changed.emitted_token_count = 2;
        assert_eq!(
            validate_compact_completion(&changed, expected_request, expected_epoch, &expected, 4),
            Err(CompactCompletionError::EmittedLengthMismatch)
        );

        changed = record();
        changed.emitted_tokens[0] = QWEN3_VOCABULARY_SIZE;
        assert_eq!(
            validate_compact_completion(&changed, expected_request, expected_epoch, &expected, 4),
            Err(CompactCompletionError::TokenOutOfRange)
        );
    }

    #[test]
    fn lowest_token_id_wins_argmax_ties() {
        let mut scores = vec![0; QWEN3_VOCABULARY_SIZE as usize];
        scores[17] = 99;
        scores[23] = 99;
        assert_eq!(select_lowest_argmax(&scores), Ok(17));
        assert_eq!(
            select_lowest_argmax(&scores[..scores.len() - 1]),
            Err(CompactCompletionError::ScoreCountMismatch)
        );
    }
}
