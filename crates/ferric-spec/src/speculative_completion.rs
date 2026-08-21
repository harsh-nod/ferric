//! Exact publication join for one bounded greedy speculative round.

use crate::completion::CompletionEpoch;
use crate::{
    validate_compact_completion, verify_greedy_round, CompactCompletionError,
    CompactCompletionRecord, GreedyCommit, GreedyVerificationError, Identity, RequestId, TokenId,
};
use vstd::prelude::*;

verus! {

/// Fail-closed error while joining a device result to greedy semantics.
#[derive(Debug, PartialEq, Eq)]
pub enum SpeculativeCompletionError {
    /// The host attempted a round beyond the exact M1 `K <= 16` bound.
    DraftLengthOutOfRange,
    /// The untrusted record failed its request/epoch/plan/bounds contract.
    Completion(CompactCompletionError),
    /// The target choices did not contain exactly `K + 1` greedy choices.
    Greedy(GreedyVerificationError),
    /// The device claimed a different accepted draft prefix.
    AcceptedLengthMismatch,
    /// The compact device payload differs from the exact greedy publication.
    EmittedTokenMismatch,
}

pub open spec fn compact_record_matches_greedy_commit(
    record: CompactCompletionRecord,
    commit: &GreedyCommit,
) -> bool {
    record.accepted_draft_tokens as nat == commit.accepted_spec()
        && record.emitted_token_count as nat == commit.emitted_spec().len()
        && forall|index: int|
            0 <= index < record.emitted_token_count as int
                ==> record.emitted_spec()[index] == commit.emitted_spec()[index]
}

pub open spec fn compact_record_is_valid_for_round(
    record: CompactCompletionRecord,
    expected_request: RequestId,
    expected_epoch: CompletionEpoch,
    expected_plan_id: Identity,
    draft_token_count: u8,
) -> bool {
    (exists|index: int|
        0 <= index < expected_plan_id.bytes_spec().len()
            && expected_plan_id.bytes_spec()[index] != 0)
        && record.request.slot_spec() == expected_request.slot_spec()
        && record.request.generation_spec() == expected_request.generation_spec()
        && record.epoch.value == expected_epoch.value
        && record.plan_id.bytes_spec() == expected_plan_id.bytes_spec()
        && draft_token_count <= 16
        && record.accepted_draft_tokens <= draft_token_count
        && record.emitted_token_count as int == record.accepted_draft_tokens as int + 1
        && record.emitted_token_count as int <= crate::M1_MAX_COMPLETION_TOKENS as int
        && forall|index: int|
            0 <= index < record.emitted_token_count as int
                ==> record.emitted_spec()[index] < crate::QWEN3_VOCABULARY_SIZE
        && forall|index: int|
            record.emitted_token_count as int <= index < crate::M1_MAX_COMPLETION_TOKENS as int
                ==> record.emitted_spec()[index] == 0
}

pub open spec fn greedy_commit_is_valid_for_round(
    draft_tokens: Seq<TokenId>,
    target_choices: Seq<TokenId>,
    commit: &GreedyCommit,
) -> bool {
    commit.accepted_spec() <= draft_tokens.len()
        && commit.accepted_spec() < target_choices.len()
        && forall|index: int|
            0 <= index < commit.accepted_spec()
                ==> #[trigger] draft_tokens[index] == target_choices[index]
        && (commit.accepted_spec() == draft_tokens.len()
            || draft_tokens[commit.accepted_spec() as int]
                != target_choices[commit.accepted_spec() as int])
        && commit.emitted_spec()
            == draft_tokens.subrange(0, commit.accepted_spec() as int).push(
                target_choices[commit.accepted_spec() as int],
            )
        && commit.correction_or_bonus_spec()
            == target_choices[commit.accepted_spec() as int]
}

/// Complete publication relation for one target-verified speculative round.
pub open spec fn speculative_completion_matches(
    record: CompactCompletionRecord,
    expected_request: RequestId,
    expected_epoch: CompletionEpoch,
    expected_plan_id: Identity,
    draft_tokens: Seq<TokenId>,
    target_choices: Seq<TokenId>,
    commit: &GreedyCommit,
) -> bool {
    draft_tokens.len() <= 16
        && target_choices.len() == draft_tokens.len() + 1
        && compact_record_is_valid_for_round(
            record,
            expected_request,
            expected_epoch,
            expected_plan_id,
            draft_tokens.len() as u8,
        )
        && greedy_commit_is_valid_for_round(draft_tokens, target_choices, commit)
        && compact_record_matches_greedy_commit(record, commit)
}

/// Validates and joins an untrusted compact device result to exact greedy
/// target verification before any tokens or speculative state are published.
///
/// # Errors
///
/// Returns [`SpeculativeCompletionError`] when `K > 16`, the record has stale
/// request/epoch/plan authority, target verification is incomplete, or the
/// accepted prefix or emitted correction/bonus differs from greedy semantics.
pub fn verify_speculative_completion(
    record: &CompactCompletionRecord,
    expected_request: RequestId,
    expected_epoch: CompletionEpoch,
    expected_plan_id: &Identity,
    draft_tokens: &[TokenId],
    target_choices: &[TokenId],
) -> (result: Result<GreedyCommit, SpeculativeCompletionError>)
    ensures
        match result {
            Ok(commit) => speculative_completion_matches(
                *record,
                expected_request,
                expected_epoch,
                *expected_plan_id,
                draft_tokens@,
                target_choices@,
                &commit,
            ),
            Err(_) => true,
        },
{
    if draft_tokens.len() > 16 {
        return Err(SpeculativeCompletionError::DraftLengthOutOfRange);
    }
    let draft_token_count = match u8::try_from(draft_tokens.len()) {
        Ok(count) => count,
        Err(_) => return Err(SpeculativeCompletionError::DraftLengthOutOfRange),
    };
    let completion_result = validate_compact_completion(
        record,
        expected_request,
        expected_epoch,
        expected_plan_id,
        draft_token_count,
    );
    if let Err(error) = completion_result {
        return Err(SpeculativeCompletionError::Completion(error));
    }
    assert(compact_record_is_valid_for_round(
        *record,
        expected_request,
        expected_epoch,
        *expected_plan_id,
        draft_token_count,
    )) by {
        reveal(crate::m1_completion::compact_completion_matches);
        reveal(crate::m1_completion::compact_completion_header_matches);
        reveal(crate::m1_completion::compact_completion_live_tokens_match);
        reveal(crate::m1_completion::compact_completion_unused_tokens_match);
        reveal(crate::m1_completion::identity_present);
        reveal(compact_record_is_valid_for_round);
    }

    let greedy_result = verify_greedy_round(draft_tokens, target_choices);
    let commit = match greedy_result {
        Ok(commit) => commit,
        Err(error) => return Err(SpeculativeCompletionError::Greedy(error)),
    };
    assert(greedy_commit_is_valid_for_round(
        draft_tokens@,
        target_choices@,
        &commit,
    )) by {
        reveal(crate::speculation::greedy_commit_matches);
        reveal(crate::speculation::is_greedy_accepted_prefix);
        reveal(greedy_commit_is_valid_for_round);
    }
    if record.accepted_draft_tokens as usize != commit.accepted_draft_tokens() {
        return Err(SpeculativeCompletionError::AcceptedLengthMismatch);
    }
    let commit_tokens = commit.emitted_tokens();
    if record.emitted_token_count as usize != commit_tokens.len() {
        return Err(SpeculativeCompletionError::EmittedTokenMismatch);
    }
    proof {
        crate::m1_completion::compact_completion_emitted_view(record);
    }
    assert(record.emitted_spec().len() == crate::M1_MAX_COMPLETION_TOKENS);

    let mut index = 0usize;
    while index < record.emitted_token_count as usize
        invariant
            index <= record.emitted_token_count as int,
            record.emitted_token_count as int <= crate::M1_MAX_COMPLETION_TOKENS as int,
            record.emitted_spec().len() == crate::M1_MAX_COMPLETION_TOKENS,
            record.emitted_token_count as nat == commit.emitted_spec().len(),
            commit_tokens@ == commit.emitted_spec(),
            forall|prior: int|
                0 <= prior < index
                    ==> record.emitted_spec()[prior] == commit.emitted_spec()[prior],
        decreases record.emitted_token_count as int - index,
    {
        assert(index < record.emitted_spec().len());
        assert(index < commit.emitted_spec().len());
        if record.emitted_tokens[index] != commit_tokens[index] {
            return Err(SpeculativeCompletionError::EmittedTokenMismatch);
        }
        assert(record.emitted_spec()[index as int] == commit.emitted_spec()[index as int]) by {
            crate::m1_completion::compact_completion_emitted_view(record);
        }
        assert forall|prior: int|
            0 <= prior < index + 1
                implies record.emitted_spec()[prior] == commit.emitted_spec()[prior] by {
            if prior < index {
                assert(record.emitted_spec()[prior] == commit.emitted_spec()[prior]);
            } else {
                assert(prior == index);
            }
        }
        index += 1;
    }
    Ok(commit)
}

} // verus!

#[cfg(test)]
mod tests {
    use super::{verify_speculative_completion, SpeculativeCompletionError};
    use crate::completion::CompletionEpoch;
    use crate::{
        CompactCompletionError, CompactCompletionRecord, Identity, RequestId,
        M1_MAX_COMPLETION_TOKENS,
    };

    const fn identity(byte: u8) -> Identity {
        Identity::new([byte; 32])
    }

    fn record(accepted: u8, emitted: &[u32]) -> CompactCompletionRecord {
        let mut tokens = [0; M1_MAX_COMPLETION_TOKENS];
        tokens[..emitted.len()].copy_from_slice(emitted);
        CompactCompletionRecord {
            request: RequestId::new(3, 7),
            epoch: CompletionEpoch { value: 11 },
            plan_id: identity(5),
            accepted_draft_tokens: accepted,
            emitted_token_count: u8::try_from(emitted.len())
                .expect("completion fixture is bounded by the fixed record"),
            emitted_tokens: tokens,
        }
    }

    fn verify(
        record: &CompactCompletionRecord,
        draft: &[u32],
        target: &[u32],
    ) -> Result<crate::GreedyCommit, SpeculativeCompletionError> {
        verify_speculative_completion(
            record,
            RequestId::new(3, 7),
            CompletionEpoch { value: 11 },
            &identity(5),
            draft,
            target,
        )
    }

    #[test]
    fn zero_partial_and_full_acceptance_publish_exact_target_tokens() {
        let zero = verify(&record(0, &[9]), &[3, 4, 5], &[9, 4, 5, 6]).unwrap();
        assert_eq!(zero.accepted_draft_tokens(), 0);
        assert_eq!(zero.emitted_tokens(), &[9]);

        let partial = verify(&record(2, &[3, 4, 9]), &[3, 4, 5], &[3, 4, 9, 6]).unwrap();
        assert_eq!(partial.accepted_draft_tokens(), 2);
        assert_eq!(partial.emitted_tokens(), &[3, 4, 9]);

        let full = verify(&record(3, &[3, 4, 5, 6]), &[3, 4, 5], &[3, 4, 5, 6]).unwrap();
        assert_eq!(full.accepted_draft_tokens(), 3);
        assert_eq!(full.emitted_tokens(), &[3, 4, 5, 6]);
    }

    #[test]
    fn accepted_prefix_and_rejected_suffix_substitution_fail_closed() {
        assert_eq!(
            verify(&record(1, &[3, 9]), &[3, 4, 5], &[3, 4, 9, 6]),
            Err(SpeculativeCompletionError::AcceptedLengthMismatch)
        );
        assert_eq!(
            verify(&record(2, &[3, 4, 5]), &[3, 4, 5], &[3, 4, 9, 6]),
            Err(SpeculativeCompletionError::EmittedTokenMismatch)
        );
    }

    #[test]
    fn stale_authority_and_noncanonical_unused_tokens_fail_before_publish() {
        let mut stale = record(2, &[3, 4, 9]);
        stale.plan_id = identity(6);
        assert_eq!(
            verify(&stale, &[3, 4, 5], &[3, 4, 9, 6]),
            Err(SpeculativeCompletionError::Completion(
                CompactCompletionError::PlanIdentityMismatch
            ))
        );

        let mut unused = record(2, &[3, 4, 9]);
        unused.emitted_tokens[16] = 8;
        assert_eq!(
            verify(&unused, &[3, 4, 5], &[3, 4, 9, 6]),
            Err(SpeculativeCompletionError::Completion(
                CompactCompletionError::NonzeroUnusedToken
            ))
        );
    }

    #[test]
    fn round_bounds_and_incomplete_target_verification_fail_closed() {
        let draft = [1; 17];
        assert_eq!(
            verify(&record(0, &[2]), &draft, &[2]),
            Err(SpeculativeCompletionError::DraftLengthOutOfRange)
        );
        assert!(matches!(
            verify(&record(2, &[3, 4, 9]), &[3, 4, 5], &[3, 4, 9]),
            Err(SpeculativeCompletionError::Greedy(_))
        ));
    }
}
