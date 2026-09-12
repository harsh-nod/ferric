use super::{EngineeringTpDraftProposalResultV1, EngineeringTpSpeculativeKvV1, Error, Phase};
use crate::tp_paged::speculative::EngineeringTpSpeculativePhaseV1;
use crate::tp_paged::speculative::tests::{index, owner, role_pool, target_result};
use crate::tp_paged::{EngineeringTpBatchCompletionV1, EngineeringTpPagedErrorV1};
use ferric_spec::{QWEN3_VOCABULARY_SIZE, Qwen3ModelRole};

fn step(
    owner: &mut EngineeringTpSpeculativeKvV1,
    token: u32,
) -> EngineeringTpDraftProposalResultV1 {
    let work = owner.proposal_work().unwrap();
    let completion = EngineeringTpBatchCompletionV1::after_all_ranks(work.batch());
    work.seal(completion, vec![token], &[0]).unwrap()
}

fn generated(owner: &mut EngineeringTpSpeculativeKvV1, k: u8) {
    owner.reserve_proposals().unwrap();
    for ordinal in 0..k {
        let result = step(owner, 100 + u32::from(ordinal));
        owner.record_proposal(result).unwrap();
    }
}

fn committed(owner: &EngineeringTpSpeculativeKvV1) -> (Vec<u32>, Vec<u32>) {
    (
        owner.target.state.sequences[&owner.target_sequence.serial]
            .tokens
            .clone(),
        owner.draft.state.sequences[&owner.draft_sequence.serial]
            .tokens
            .clone(),
    )
}

#[test]
fn every_acceptance_uses_private_chained_rows_then_existing_atomic_prefix_settlement() {
    for k in [4, 8, 16] {
        for accepted in 0..=k {
            for cursor in [0, 15, 16, 31] {
                let mut owner = owner(cursor, k);
                let before = committed(&owner);
                owner.reserve_proposals().unwrap();
                let Phase::Proposing(round) = &owner.phase else {
                    panic!()
                };
                assert_eq!(round.target.rows.len(), usize::from(k) + 1);
                assert_eq!(round.draft.rows.len(), usize::from(k));
                assert_eq!(
                    owner.phase(),
                    EngineeringTpSpeculativePhaseV1::DraftProposing
                );
                assert!(owner.target_work().is_err());
                assert!(owner.draft_work().is_err());
                let mut last_id = 0;
                for ordinal in 0..k {
                    let work = owner.proposal_work().unwrap();
                    assert_eq!(work.ordinal(), ordinal);
                    let row = &work.batch().rows()[0];
                    assert_eq!(row.position(), cursor + u32::from(ordinal));
                    assert_eq!(
                        row.token(),
                        if ordinal == 0 {
                            77
                        } else {
                            99 + u32::from(ordinal)
                        }
                    );
                    assert!(work.batch().id() > last_id);
                    last_id = work.batch().id();
                    let completion = EngineeringTpBatchCompletionV1::after_all_ranks(work.batch());
                    let result = work
                        .seal(completion, vec![100 + u32::from(ordinal)], &[0])
                        .unwrap();
                    assert!(owner.proposal_work().is_err());
                    assert!(owner.abort_unsubmitted().is_err());
                    assert!(owner.settle().is_err());
                    owner.record_proposal(result).unwrap();
                    assert_eq!(committed(&owner), before);
                    assert!(owner.draft.pending.as_ref().unwrap().submitted);
                    for role in [Qwen3ModelRole::Target8B, Qwen3ModelRole::Draft06B] {
                        owner.pool(role).check_invariants().unwrap();
                    }
                }
                assert!(owner.proposal_work().is_err());
                assert!(owner.draft_work().is_err());
                let Phase::Round(round) = &owner.phase else {
                    panic!()
                };
                assert_eq!(
                    round.index.draft_tokens[..usize::from(k)],
                    (100..100 + u32::from(k)).collect::<Vec<_>>()
                );
                let mut choices = round.index.draft_tokens[..usize::from(k)].to_vec();
                choices.push(999);
                choices[usize::from(accepted)] = 999;
                let result = target_result(&mut owner, choices);
                owner.record_target(result).unwrap();
                let settled = owner.settle().unwrap();
                assert_eq!(
                    settled.greedy_commit().accepted_draft_tokens(),
                    usize::from(accepted)
                );
                let mut expected = vec![7; cursor as usize];
                expected.push(77);
                expected.extend(100..100 + u32::from(accepted));
                let (target, draft) = committed(&owner);
                assert_eq!(target, expected);
                assert_eq!(
                    draft,
                    target[..cursor as usize + usize::from((accepted + 1).min(k))]
                );
                assert_eq!(owner.next_anchor(), 999);
                for (pool, length) in [(&owner.target, target.len()), (&owner.draft, draft.len())] {
                    pool.check_invariants().unwrap();
                    assert_eq!(
                        pool.stats().retained_pages,
                        u32::try_from(length.div_ceil(16)).unwrap()
                    );
                }
                if accepted == k {
                    assert!(owner.reserve_proposals().is_err());
                    owner.reserve_draft_catch_up().unwrap();
                    let work = owner.draft_work().unwrap();
                    assert_eq!(work.batch().rows()[0].token(), 99 + u32::from(k));
                    assert!(work.batch().id() > last_id);
                    let result = work.complete_for_test();
                    owner.record_draft(result).unwrap();
                    assert_eq!(committed(&owner).0, committed(&owner).1);
                }
                owner.reserve_proposals().unwrap();
                owner.abort_unsubmitted().unwrap();
            }
        }
    }
}

#[test]
fn reservations_abort_before_submission_and_never_reuse_ids() {
    let mut owner = owner(15, 16);
    let before = committed(&owner);
    owner.reserve_proposals().unwrap();
    let last_reserved = owner.draft.next_batch - 1;
    let target_id = owner.target.pending.as_ref().unwrap().id;
    owner.abort_unsubmitted().unwrap();
    assert_eq!(committed(&owner), before);
    assert_eq!(owner.next_epoch().value, 1);
    owner.reserve_proposals().unwrap();
    assert!(owner.target.pending.as_ref().unwrap().id > target_id);
    {
        let work = owner.proposal_work().unwrap();
        assert!(work.batch().id() > last_reserved);
    }
    assert!(owner.abort_unsubmitted().is_err());
    assert!(owner.proposal_work().is_err());
    owner.fail_submitted().unwrap();
    assert_eq!(owner.phase(), EngineeringTpSpeculativePhaseV1::Terminal);
    assert_eq!(committed(&owner), before);
    assert_eq!(owner.target.stats().free_pages, 0);
    assert_eq!(owner.draft.stats().free_pages, 0);
}

#[test]
fn capacity_and_counter_failures_happen_before_any_proposal_ticket() {
    for target_oom in [true, false] {
        let (target, ts) = role_pool(1, 15, if target_oom { 1 } else { 4 });
        let (draft, ds) = role_pool(2, 15, if target_oom { 4 } else { 1 });
        let mut owner =
            EngineeringTpSpeculativeKvV1::new(target, ts, draft, ds, &index(15, 1, 4, 77)).unwrap();
        let before = committed(&owner);
        assert_eq!(
            owner.reserve_proposals(),
            Err(Error::Pool(EngineeringTpPagedErrorV1::OutOfPages))
        );
        assert_eq!(committed(&owner), before);
        assert!(owner.target.pending.is_none() && owner.draft.pending.is_none());
        assert!(owner.proposal_work().is_err());
    }
    for field in 0..3 {
        let mut owner = owner(0, 16);
        match field {
            0 => owner.draft.next_batch = u64::MAX - 16,
            1 => owner.target.next_batch = u64::MAX,
            _ => owner.epoch.value = u64::MAX,
        }
        assert!(owner.reserve_proposals().is_err());
        assert!(owner.target.pending.is_none() && owner.draft.pending.is_none());
        assert_eq!(owner.phase(), EngineeringTpSpeculativePhaseV1::Ready);
    }
    let mut owner = owner(127, 4);
    assert!(owner.reserve_proposals().is_err());
    assert!(owner.target.pending.is_none() && owner.draft.pending.is_none());
}

#[test]
fn substituted_row_results_terminalize_both_without_partial_commit() {
    for field in 0..11 {
        let mut owner = owner(15, 4);
        owner.reserve_proposals().unwrap();
        let before = committed(&owner);
        let mut result = step(&mut owner, 100);
        match field {
            0 => result.identity.round.request = ferric_spec::RequestId::new(9, 9),
            1 => result.identity.round.epoch.value += 1,
            2 => result.identity.round.plan = ferric_spec::Identity::new([8; 32]),
            3 => result.identity.round.role = Qwen3ModelRole::Target8B,
            4 => result.identity.round.batch += 1,
            5 => result.identity.ordinal += 1,
            6 => result.identity.batch += 1,
            7 => result.input.token += 1,
            8 => result.input.position += 1,
            9 => result.completion.batch += 1,
            _ => result.choice = QWEN3_VOCABULARY_SIZE,
        }
        assert!(owner.record_proposal(result).is_err());
        assert_eq!(owner.phase(), EngineeringTpSpeculativePhaseV1::Terminal);
        assert_eq!(committed(&owner), before);
        assert_eq!(owner.target.stats().free_pages, 0);
        assert_eq!(owner.draft.stats().free_pages, 0);
    }
}

#[test]
fn duplicate_and_foreign_results_do_not_advance_the_next_private_row() {
    let mut first = owner(0, 4);
    let mut second = owner(0, 4);
    first.reserve_proposals().unwrap();
    second.reserve_proposals().unwrap();
    let foreign = step(&mut first, 100);
    {
        let _work = second.proposal_work().unwrap();
    }
    assert!(second.record_proposal(foreign).is_err());
    assert_eq!(second.phase(), EngineeringTpSpeculativePhaseV1::Terminal);
    first.fail_submitted().unwrap();

    let mut owner = owner(0, 4);
    owner.reserve_proposals().unwrap();
    let result = step(&mut owner, 100);
    let replay = EngineeringTpDraftProposalResultV1 {
        identity: result.identity,
        input: result.input,
        choice: result.choice,
        completion: EngineeringTpBatchCompletionV1 {
            pool: result.completion.pool,
            batch: result.completion.batch,
        },
    };
    owner.record_proposal(result).unwrap();
    assert!(owner.record_proposal(replay).is_err());
    assert_eq!(owner.phase(), EngineeringTpSpeculativePhaseV1::Terminal);
}

#[test]
fn seal_rejects_wrong_completion_selection_choice_count_and_vocabulary() {
    for field in 0..6 {
        let mut owner = owner(0, 4);
        owner.reserve_proposals().unwrap();
        let work = owner.proposal_work().unwrap();
        let mut completion = EngineeringTpBatchCompletionV1::after_all_ranks(work.batch());
        let mut choices = vec![100];
        let mut rows = vec![0];
        match field {
            0 => completion.pool += 1,
            1 => completion.batch += 1,
            2 => choices.clear(),
            3 => choices.push(101),
            4 => choices[0] = QWEN3_VOCABULARY_SIZE,
            _ => rows[0] = 1,
        }
        assert!(work.seal(completion, choices, &rows).is_err());
        owner.fail_submitted().unwrap();
    }
}

#[test]
fn retained_row_evidence_is_rechecked_before_either_prefix_is_published() {
    let mut owner = owner(15, 4);
    generated(&mut owner, 4);
    let target = target_result(&mut owner, vec![100, 101, 102, 103, 999]);
    owner.record_target(target).unwrap();
    let before = committed(&owner);
    let Phase::Round(round) = &mut owner.phase else {
        panic!()
    };
    round.proposal_results.as_mut().unwrap().results[1]
        .input
        .token += 1;
    assert!(owner.settle().is_err());
    assert_eq!(committed(&owner), before);
    assert_eq!(owner.phase(), EngineeringTpSpeculativePhaseV1::Terminal);
}

#[test]
fn generated_suffix_never_changes_shared_cached_prefix_or_other_sequences() {
    let mut roles = Vec::new();
    for model in [1, 2] {
        let (mut pool, sequence) = role_pool(model, 16, 8);
        pool.retire_sequence(sequence, true, 1).unwrap();
        let scope = pool.scope();
        let active = pool.open_sequence(scope, &[7; 17], 2).unwrap().sequence();
        let other = pool.open_sequence(scope, &[7; 17], 2).unwrap().sequence();
        let before = pool.state.sequences[&other.serial].clone();
        roles.push((pool, active, other, before));
    }
    let (draft, ds, other_draft, before_draft) = roles.pop().unwrap();
    let (target, ts, other_target, before_target) = roles.pop().unwrap();
    let mut owner =
        EngineeringTpSpeculativeKvV1::new(target, ts, draft, ds, &index(16, 1, 16, 77)).unwrap();
    generated(&mut owner, 16);
    let result = target_result(&mut owner, vec![999; 17]);
    owner.record_target(result).unwrap();
    owner.settle().unwrap();
    assert_eq!(
        owner.target.state.sequences[&other_target.serial],
        before_target
    );
    assert_eq!(
        owner.draft.state.sequences[&other_draft.serial],
        before_draft
    );
    for pool in [&owner.target, &owner.draft] {
        pool.check_invariants().unwrap();
        assert_eq!(pool.stats().cached_pages, 1);
        assert_eq!(pool.state.pages[0].refs, 3);
    }
}
