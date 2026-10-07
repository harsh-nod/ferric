use super::*;
use crate::tp_paged::{EngineeringTpPagedLimitsV1, EngineeringTpPoolScopeV1};
use ferric_spec::{
    CorrectionBonusKvDisposition, Qwen3ExecutionMode, Qwen3PlanBucket, SpeculativeKvInterval,
};

pub(crate) fn index(cursor: u32, epoch: u64, k: u8, anchor: u32) -> SpeculativeKvRoundIndex {
    let bucket = match k {
        4 => Qwen3PlanBucket::SpeculativeS1K4C8192,
        8 => Qwen3PlanBucket::SpeculativeS1K8C8192,
        16 => Qwen3PlanBucket::SpeculativeS1K16C8192,
        _ => panic!("finite width required"),
    };
    let mut target_commit_ends = [0; 17];
    let mut draft_commit_ends = [0; 17];
    let mut draft_tokens = [0; 16];
    for a in 0..=k {
        target_commit_ends[usize::from(a)] = cursor + u32::from(a) + 1;
        draft_commit_ends[usize::from(a)] = cursor + u32::from((a + 1).min(k));
        if a < k {
            draft_tokens[usize::from(a)] = 100 + u32::from(a);
        }
    }
    SpeculativeKvRoundIndex {
        request: RequestId::new(2, 3),
        completion_epoch: CompletionEpoch::new(epoch),
        plan_id: Identity::new([9; 32]),
        target_selection: Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Speculative,
            bucket,
        },
        draft_selection: Qwen3PlanSelection {
            role: Qwen3ModelRole::Draft06B,
            mode: Qwen3ExecutionMode::Speculative,
            bucket,
        },
        draft_token_count: k,
        round_anchor: anchor,
        draft_tokens,
        target_pre_committed: cursor,
        draft_pre_committed: cursor,
        target_tentative: SpeculativeKvInterval {
            start: cursor,
            end: cursor + u32::from(k) + 1,
        },
        draft_tentative: SpeculativeKvInterval {
            start: cursor,
            end: cursor + u32::from(k),
        },
        target_commit_ends,
        draft_commit_ends,
        correction_bonus: CorrectionBonusKvDisposition::DeferredUntilNextStep,
    }
}

fn append(pool: &mut EngineeringTpPagedPoolV1, seq: EngineeringTpSequenceIdV1, tokens: &[u32]) {
    for chunk in tokens.chunks(32) {
        let start = pool.committed_position(seq).unwrap();
        let rows = chunk
            .iter()
            .enumerate()
            .map(|(i, token)| EngineeringTpPageRowV1 {
                sequence: seq,
                position: start + u32::try_from(i).unwrap(),
                token: *token,
            })
            .collect::<Vec<_>>();
        let batch = pool.reserve_batch(&rows).unwrap();
        pool.begin_submission(&batch).unwrap();
        pool.commit_batch(
            &batch,
            EngineeringTpBatchCompletionV1::after_all_ranks(&batch),
        )
        .unwrap();
    }
}

pub(super) fn role_pool(
    model: u8,
    cursor: u32,
    pages: u32,
) -> (EngineeringTpPagedPoolV1, EngineeringTpSequenceIdV1) {
    let scope = EngineeringTpPoolScopeV1 {
        model: [model; 32],
        session: [model + 10; 32],
    };
    let mut pool = EngineeringTpPagedPoolV1::new_wide32(
        scope,
        EngineeringTpPagedLimitsV1::new(128, 4, pages, 100).unwrap(),
    )
    .unwrap();
    let seq = pool.open_sequence(scope, &[7], 0).unwrap().sequence();
    append(&mut pool, seq, &vec![7; cursor as usize]);
    (pool, seq)
}

pub(super) fn owner(cursor: u32, k: u8) -> EngineeringTpSpeculativeKvV1 {
    let (target, target_seq) = role_pool(1, cursor, 16);
    let (draft, draft_seq) = role_pool(2, cursor, 16);
    EngineeringTpSpeculativeKvV1::new(
        target,
        target_seq,
        draft,
        draft_seq,
        &index(cursor, 1, k, 77),
    )
    .unwrap()
}

fn choices(index: &SpeculativeKvRoundIndex, a: u8) -> Vec<u32> {
    let mut result = index.draft_tokens[..usize::from(index.draft_token_count)].to_vec();
    result.push(999);
    if a < index.draft_token_count {
        result[usize::from(a)] = 999;
    }
    result
}

pub(super) fn target_result(
    owner: &mut EngineeringTpSpeculativeKvV1,
    values: Vec<u32>,
) -> EngineeringTpSpeculativeTargetResultV1 {
    let work = owner.target_work().unwrap();
    let completion = EngineeringTpBatchCompletionV1::after_all_ranks(work.batch);
    let selected = (0..work.rows).collect();
    work.seal(completion, values, selected).unwrap()
}

pub(crate) fn record_draft_for_test(owner: &mut EngineeringTpSpeculativeKvV1) {
    let result = owner.draft_work().unwrap().complete_for_test();
    owner.record_draft(result).unwrap();
}

fn completed(owner: &mut EngineeringTpSpeculativeKvV1, index: &SpeculativeKvRoundIndex, a: u8) {
    owner.reserve_round(index).unwrap();
    let target = target_result(owner, choices(index, a));
    owner.record_target(target).unwrap();
    record_draft_for_test(owner);
}

#[test]
fn every_finite_acceptance_retains_only_consumed_prefix_and_exact_pages() {
    for k in [4, 8, 16] {
        for a in 0..=k {
            for cursor in [0, 15, 16, 31] {
                let mut owner = owner(cursor, k);
                let index = index(cursor, 1, k, 77);
                completed(&mut owner, &index, a);
                let receipt = owner.settle().unwrap();
                let mut target = vec![7; cursor as usize];
                target.push(77);
                target.extend_from_slice(&index.draft_tokens[..usize::from(a)]);
                let draft = &target[..cursor as usize + usize::from((a + 1).min(k))];
                assert_eq!(
                    owner.target.state.sequences[&owner.target_sequence.serial].tokens,
                    target
                );
                assert_eq!(
                    owner.draft.state.sequences[&owner.draft_sequence.serial].tokens,
                    draft
                );
                assert_eq!(
                    receipt.greedy_commit().accepted_draft_tokens(),
                    usize::from(a)
                );
                assert_eq!(receipt.greedy_commit().target_correction_or_bonus(), 999);
                assert_eq!(receipt.target_cursor(), cursor + u32::from(a) + 1);
                assert_eq!(receipt.draft_cursor(), cursor + u32::from((a + 1).min(k)));
                assert_eq!(receipt.draft_catch_up_required(), a == k);
                assert_eq!(owner.next_anchor(), 999);
                for (pool, length) in [(&owner.target, target.len()), (&owner.draft, draft.len())] {
                    pool.check_invariants().unwrap();
                    assert_eq!(
                        pool.stats().retained_pages,
                        u32::try_from(length.div_ceil(16)).unwrap()
                    );
                    assert_eq!(pool.stats().cached_pages, 0);
                }
                assert_eq!(owner.settle().unwrap_err(), Error::Phase);
            }
        }
    }
}

#[test]
fn zero_acceptance_crossing_15_to_16_releases_only_new_suffix_pages() {
    let mut owner = owner(15, 4);
    let index = index(15, 1, 4, 77);
    let old_target_pages = owner.target.state.sequences[&owner.target_sequence.serial]
        .pages
        .clone();
    completed(&mut owner, &index, 0);
    assert_eq!(owner.target.stats().retained_pages, 2);
    owner.settle().unwrap();
    assert_eq!(
        owner.target.state.sequences[&owner.target_sequence.serial].pages,
        old_target_pages
    );
    assert_eq!(owner.target.stats().retained_pages, 1);
    let next = self::index(16, 2, 4, 999);
    owner.reserve_round(&next).unwrap();
    let Phase::Round(round) = &owner.phase else {
        panic!()
    };
    assert_eq!(round.target.rows()[0].position(), 16);
    assert_eq!(round.target.rows()[0].token(), 999);
    assert_ne!(
        round.target.rows()[0].writable_physical_page(),
        old_target_pages[0]
    );
}

#[test]
fn stale_request_epoch_role_plan_cursor_and_anchor_reject_before_reservation() {
    for mutation in 0..8 {
        let mut owner = owner(15, 4);
        let before = (owner.target.state.clone(), owner.draft.state.clone());
        let mut bad = index(15, 1, 4, 77);
        match mutation {
            0 => bad.request = RequestId::new(2, 4),
            1 => bad.request = RequestId::new(3, 3),
            2 => bad.completion_epoch = CompletionEpoch::new(2),
            3 => bad.target_selection.role = Qwen3ModelRole::Draft06B,
            4 => bad.plan_id = Identity::new([8; 32]),
            5 => bad.target_pre_committed = 14,
            6 => bad.round_anchor = 78,
            _ => bad.target_commit_ends[0] = 14,
        }
        assert!(owner.reserve_round(&bad).is_err());
        assert_eq!(
            (owner.target.state.clone(), owner.draft.state.clone()),
            before
        );
        assert!(owner.target.pending.is_none() && owner.draft.pending.is_none());
    }
}

#[test]
fn opaque_result_mutations_cannot_partially_commit_or_publish() {
    for mutation in 0..12 {
        let mut owner = owner(15, 4);
        let before = (owner.target.state.clone(), owner.draft.state.clone());
        let index = index(15, 1, 4, 77);
        owner.reserve_round(&index).unwrap();
        let mut result = target_result(&mut owner, choices(&index, 2));
        match mutation {
            0 => result.identity.role = Qwen3ModelRole::Draft06B,
            1 => result.identity.epoch = CompletionEpoch::new(2),
            2 => result.identity.request = RequestId::new(2, 4),
            3 => result.identity.plan = Identity::new([8; 32]),
            4 => result.identity.pool += 1,
            5 => result.identity.batch += 1,
            6 => result.rows[0].token += 1,
            7 => result.rows[1].position += 1,
            8 => result.output_rows.swap(0, 1),
            9 => {
                result.choices.pop();
            }
            10 => result.choices[0] = QWEN3_VOCABULARY_SIZE,
            _ => result.completion.batch += 1,
        }
        assert!(owner.record_target(result).is_err());
        assert_eq!(
            (owner.target.state.clone(), owner.draft.state.clone()),
            before
        );
        assert_eq!(owner.phase(), EngineeringTpSpeculativePhaseV1::Terminal);
        for pool in [&owner.target, &owner.draft] {
            assert_eq!(pool.stats().quarantined_pages, 16);
            assert_eq!(pool.stats().free_pages, 0);
        }
    }
}

#[test]
fn stale_aborted_certificate_and_duplicate_completion_fail_closed() {
    let mut owner = owner(0, 4);
    let index = index(0, 1, 4, 77);
    owner.reserve_round(&index).unwrap();
    let Phase::Round(round) = &owner.phase else {
        panic!()
    };
    let old_batch = round.target.id;
    owner.abort_unsubmitted().unwrap();
    owner.reserve_round(&index).unwrap();
    let mut result = target_result(&mut owner, choices(&index, 0));
    assert_ne!(result.identity.batch, old_batch);
    result.identity.batch = old_batch;
    result.completion.batch = old_batch;
    assert_eq!(owner.record_target(result), Err(Error::Completion));
    assert_eq!(owner.phase(), EngineeringTpSpeculativePhaseV1::Terminal);

    let mut owner = self::owner(0, 4);
    owner.reserve_round(&index).unwrap();
    let result = target_result(&mut owner, choices(&index, 0));
    let duplicate = EngineeringTpSpeculativeTargetResultV1 {
        identity: result.identity,
        rows: result.rows.clone(),
        output_rows: result.output_rows.clone(),
        choices: result.choices.clone(),
        completion: EngineeringTpBatchCompletionV1 {
            pool: result.completion.pool,
            batch: result.completion.batch,
        },
    };
    owner.record_target(result).unwrap();
    assert_eq!(owner.record_target(duplicate), Err(Error::Completion));
}

#[test]
fn both_complete_before_settlement_and_submitted_failure_quarantines_both() {
    let mut owner = owner(15, 4);
    let index = index(15, 1, 4, 77);
    owner.reserve_round(&index).unwrap();
    let result = target_result(&mut owner, choices(&index, 1));
    owner.record_target(result).unwrap();
    assert_eq!(owner.settle().unwrap_err(), Error::Phase);
    assert_eq!(owner.abort_unsubmitted(), Err(Error::Phase));
    owner.fail_submitted().unwrap();
    assert_eq!(owner.phase(), EngineeringTpSpeculativePhaseV1::Terminal);
    assert_eq!(owner.target.stats().quarantined_pages, 16);
    assert_eq!(owner.draft.stats().quarantined_pages, 16);
    assert!(owner.reserve_round(&index).is_err());
}

#[test]
fn invalid_draft_prefix_preflight_preserves_both_committed_states() {
    let mut owner = owner(15, 4);
    let before = (owner.target.state.clone(), owner.draft.state.clone());
    completed(&mut owner, &index(15, 1, 4, 77), 0);
    let pending = owner.draft.pending.as_mut().unwrap();
    let rejected = pending.proposed.sequences[&owner.draft_sequence.serial].pages[1];
    pending.proposed.pages[rejected as usize].cached = true;
    assert!(owner.settle().is_err());
    assert_eq!(
        (owner.target.state.clone(), owner.draft.state.clone()),
        before
    );
    assert_eq!(owner.phase(), EngineeringTpSpeculativePhaseV1::Terminal);
}

#[test]
fn two_rounds_full_acceptance_requires_completion_bound_draft_catch_up() {
    let mut owner = owner(15, 4);
    completed(&mut owner, &index(15, 1, 4, 77), 4);
    let first = owner.settle().unwrap();
    assert_eq!((first.target_cursor(), first.draft_cursor()), (20, 19));
    let next = index(20, 2, 4, 999);
    assert_eq!(owner.reserve_round(&next), Err(Error::Phase));
    owner.reserve_draft_catch_up().unwrap();
    assert_eq!(owner.reserve_round(&next), Err(Error::Phase));
    let work = owner.draft_work().unwrap();
    assert!(work.is_catch_up());
    assert_eq!(work.batch().rows().len(), 1);
    assert_eq!(work.batch().rows()[0].position(), 19);
    assert_eq!(work.batch().rows()[0].token(), 103);
    let result = work.complete_for_test();
    owner.record_draft(result).unwrap();
    assert_eq!(owner.phase(), EngineeringTpSpeculativePhaseV1::Ready);
    assert_eq!(
        owner.target.state.sequences[&owner.target_sequence.serial].tokens,
        owner.draft.state.sequences[&owner.draft_sequence.serial].tokens
    );
    completed(&mut owner, &next, 2);
    let second = owner.settle().unwrap();
    assert_eq!((second.target_cursor(), second.draft_cursor()), (23, 23));
    assert_eq!(second.greedy_commit().emitted_tokens(), &[100, 101, 999]);
    assert_eq!(owner.next_epoch(), CompletionEpoch::new(3));
}

#[test]
fn stale_catch_up_purpose_or_epoch_quarantines_instead_of_advancing() {
    for mutate_epoch in [false, true] {
        let mut owner = owner(15, 4);
        completed(&mut owner, &index(15, 1, 4, 77), 4);
        owner.settle().unwrap();
        let before = (owner.target.state.clone(), owner.draft.state.clone());
        owner.reserve_draft_catch_up().unwrap();
        let mut result = owner.draft_work().unwrap().complete_for_test();
        if mutate_epoch {
            result.identity.epoch = CompletionEpoch::new(1);
        } else {
            result.identity.catch_up = false;
        }
        assert!(owner.record_draft(result).is_err());
        assert_eq!(
            (owner.target.state.clone(), owner.draft.state.clone()),
            before
        );
        assert_eq!(owner.phase(), EngineeringTpSpeculativePhaseV1::Terminal);
    }
}

#[test]
fn draft_oom_aborts_target_and_epoch_exhaustion_never_reserves() {
    let (target, target_seq) = role_pool(1, 15, 4);
    let (draft, draft_seq) = role_pool(2, 15, 1);
    let index = index(15, 1, 4, 77);
    let mut owner =
        EngineeringTpSpeculativeKvV1::new(target, target_seq, draft, draft_seq, &index).unwrap();
    let before = (owner.target.state.clone(), owner.draft.state.clone());
    assert_eq!(
        owner.reserve_round(&index),
        Err(Error::Pool(EngineeringTpPagedErrorV1::OutOfPages))
    );
    assert!(owner.target.pending.is_none() && owner.draft.pending.is_none());
    assert_eq!(
        (owner.target.state.clone(), owner.draft.state.clone()),
        before
    );
    owner.epoch = CompletionEpoch::new(u64::MAX);
    let mut exhausted = index;
    exhausted.completion_epoch = owner.epoch;
    assert_eq!(owner.reserve_round(&exhausted), Err(Error::Exhausted));
    assert_eq!(owner.phase(), EngineeringTpSpeculativePhaseV1::Ready);
}

#[test]
fn target_seal_rejects_incomplete_choices_selection_and_foreign_completion() {
    for mutation in 0..3 {
        let mut owner = owner(0, 4);
        let index = index(0, 1, 4, 77);
        owner.reserve_round(&index).unwrap();
        let work = owner.target_work().unwrap();
        let mut complete = EngineeringTpBatchCompletionV1::after_all_ranks(work.batch);
        let mut outputs = choices(&index, 0);
        let mut selected = (0..5).collect::<Vec<_>>();
        match mutation {
            0 => {
                outputs.pop();
            }
            1 => selected.swap(0, 1),
            _ => complete.pool += 1,
        }
        assert!(work.seal(complete, outputs, selected).is_err());
    }
}

#[test]
fn draft_seal_and_record_reject_foreign_completion_and_stale_bound_inputs() {
    for mutation in 0..8 {
        let mut owner = owner(15, 4);
        let before = (owner.target.state.clone(), owner.draft.state.clone());
        owner.reserve_round(&index(15, 1, 4, 77)).unwrap();
        let work = owner.draft_work().unwrap();
        let completion = EngineeringTpBatchCompletionV1::after_all_ranks(work.batch());
        let mut result = work.seal(completion).unwrap();
        match mutation {
            0 => result.identity.role = Qwen3ModelRole::Target8B,
            1 => result.identity.request = RequestId::new(2, 4),
            2 => result.identity.epoch = CompletionEpoch::new(2),
            3 => result.identity.batch += 1,
            4 => result.identity.catch_up = true,
            5 => result.rows[0].token += 1,
            6 => result.rows[0].position += 1,
            _ => result.completion.pool += 1,
        }
        assert!(owner.record_draft(result).is_err());
        assert_eq!(owner.phase(), EngineeringTpSpeculativePhaseV1::Terminal);
        assert_eq!(
            (owner.target.state.clone(), owner.draft.state.clone()),
            before
        );
    }
    let mut owner = owner(0, 4);
    owner.reserve_round(&index(0, 1, 4, 77)).unwrap();
    let work = owner.draft_work().unwrap();
    let mut completion = EngineeringTpBatchCompletionV1::after_all_ranks(work.batch());
    completion.batch += 1;
    assert!(work.seal(completion).is_err());
}

#[test]
fn successful_round_replay_cannot_rewrite_committed_tokens() {
    let mut owner = owner(15, 4);
    let index = index(15, 1, 4, 77);
    completed(&mut owner, &index, 2);
    owner.settle().unwrap();
    let before = (owner.target.state.clone(), owner.draft.state.clone());
    assert_eq!(owner.reserve_round(&index), Err(Error::Index));
    assert_eq!(
        (owner.target.state.clone(), owner.draft.state.clone()),
        before
    );
    assert!(owner.target.pending.is_none() && owner.draft.pending.is_none());
}

#[test]
fn aborting_unsubmitted_catch_up_never_skips_missing_draft_input() {
    let mut owner = owner(15, 4);
    completed(&mut owner, &index(15, 1, 4, 77), 4);
    owner.settle().unwrap();
    let before = (owner.target.state.clone(), owner.draft.state.clone());
    owner.reserve_draft_catch_up().unwrap();
    let Phase::CatchUp { batch, .. } = &owner.phase else {
        panic!()
    };
    let old_batch = batch.id();
    owner.abort_unsubmitted().unwrap();
    assert_eq!(
        owner.phase(),
        EngineeringTpSpeculativePhaseV1::DraftCatchUpRequired
    );
    assert_eq!(
        (owner.target.state.clone(), owner.draft.state.clone()),
        before
    );
    owner.reserve_draft_catch_up().unwrap();
    let work = owner.draft_work().unwrap();
    assert_ne!(work.batch().id(), old_batch);
    assert_eq!(work.batch().rows()[0].token(), 103);
    let result = work.complete_for_test();
    owner.record_draft(result).unwrap();
    assert_eq!(owner.phase(), EngineeringTpSpeculativePhaseV1::Ready);
}

#[test]
fn committed_cached_shared_pages_are_retained_and_other_sequences_framed() {
    let mut roles = Vec::new();
    for model in [1, 2] {
        let (mut pool, seq) = role_pool(model, 16, 8);
        pool.retire_sequence(seq, true, 1).unwrap();
        let scope = pool.scope();
        let first = pool.open_sequence(scope, &[7; 17], 2).unwrap().sequence();
        let other = pool.open_sequence(scope, &[7; 17], 2).unwrap().sequence();
        let other_state = pool.state.sequences[&other.serial].clone();
        roles.push((pool, first, other, other_state));
    }
    let (draft, ds, do_, db) = roles.pop().unwrap();
    let (target, ts, to, tb) = roles.pop().unwrap();
    let index = index(16, 1, 4, 77);
    let mut owner = EngineeringTpSpeculativeKvV1::new(target, ts, draft, ds, &index).unwrap();
    completed(&mut owner, &index, 0);
    owner.settle().unwrap();
    assert_eq!(owner.target.state.sequences[&to.serial], tb);
    assert_eq!(owner.draft.state.sequences[&do_.serial], db);
    for pool in [&owner.target, &owner.draft] {
        pool.check_invariants().unwrap();
        assert_eq!(pool.stats().cached_pages, 1);
        assert_eq!(pool.state.pages[0].refs, 3);
    }
}
