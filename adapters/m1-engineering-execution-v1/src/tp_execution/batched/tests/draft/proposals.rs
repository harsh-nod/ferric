//! Real host forwards with injected compact choices, not GPU numerical evidence.

use super::{
    EngineeringTpDraftBatchExecutionV10, EngineeringTpSpeculativeKvV1,
    EngineeringTpSpeculativePhaseV1, Failure, Qwen3ModelRole, Recording, paired,
};

fn generate(
    owner: &mut EngineeringTpSpeculativeKvV1,
    draft: &mut EngineeringTpDraftBatchExecutionV10<Recording>,
    cursor: u32,
    k: u8,
    base: u32,
) -> u64 {
    owner.reserve_proposals().unwrap();
    assert!(owner.target_work().is_err());
    let mut previous_id = 0;
    let anchor = owner.next_anchor();
    for ordinal in 0..k {
        draft.recording_inner().inner.transports[0].choice_override =
            Some(vec![base + u32::from(ordinal)]);
        let before = draft.dispatch_counts()[0];
        let work = owner.proposal_work().unwrap();
        assert_eq!(work.ordinal(), ordinal);
        assert_eq!(work.batch().rows().len(), 1);
        assert!(work.batch().id() > previous_id);
        previous_id = work.batch().id();
        let expected_token = if ordinal == 0 {
            anchor
        } else {
            base + u32::from(ordinal) - 1
        };
        assert_eq!(work.batch().rows()[0].token(), expected_token);
        let result = draft.execute_speculative_proposal(work).unwrap();
        assert_eq!(draft.dispatch_counts()[0] - before, 480);
        let inner = draft.recording_inner();
        assert_eq!(
            inner.inner.transports[0].u32_values(inner.inner.ranks[0].token.id, 1),
            [expected_token]
        );
        assert_eq!(
            inner.inner.transports[0].u32_values(inner.positions[0].id, 1),
            [cursor + u32::from(ordinal)]
        );
        assert!(owner.settle().is_err());
        assert!(owner.proposal_work().is_err());
        assert!(owner.abort_unsubmitted().is_err());
        owner.record_proposal(result).unwrap();
        if ordinal + 1 < k {
            assert!(owner.target_work().is_err());
        }
    }
    assert!(owner.proposal_work().is_err());
    assert!(owner.draft_work().is_err());
    previous_id
}

#[test]
fn actual_k4_k8_k16_generate_private_rows_and_settle_zero_partial_full() {
    for k in [4, 8, 16] {
        for accepted in [0, k / 2, k] {
            let (mut target, mut draft, mut owner, _) = paired(15, k, accepted);
            draft.recording_inner().inner.transports[0].rollover_supported = true;
            let initial = draft.dispatch_counts()[0];
            let last_id = generate(&mut owner, &mut draft, 15, k, 100);
            assert_eq!(draft.dispatch_counts()[0] - initial, u64::from(k) * 480);
            let mut choices = (100..100 + u32::from(k)).collect::<Vec<_>>();
            choices.push(999);
            choices[usize::from(accepted)] = 999;
            target.inner.transports[0].choice_override = Some(choices);
            let work = owner.target_work().unwrap();
            assert_eq!(work.batch().rows()[0].token(), 77);
            assert_eq!(
                work.batch().rows()[usize::from(k)].token(),
                99 + u32::from(k)
            );
            let result = target.execute_speculative_target(work).unwrap();
            owner.record_target(result).unwrap();
            let settled = owner.settle().unwrap();
            assert_eq!(
                settled.greedy_commit().accepted_draft_tokens(),
                usize::from(accepted)
            );
            assert_eq!(settled.target_cursor(), 16 + u32::from(accepted));
            assert_eq!(
                settled.draft_cursor(),
                15 + u32::from((accepted + 1).min(k))
            );
            assert_eq!(owner.next_anchor(), 999);
            if accepted == k {
                assert!(owner.reserve_proposals().is_err());
                owner.reserve_draft_catch_up().unwrap();
                let work = owner.draft_work().unwrap();
                assert!(work.batch().id() > last_id);
                assert_eq!(work.batch().rows()[0].token(), 99 + u32::from(k));
                let before = draft.dispatch_counts()[0];
                let result = draft.execute_speculative_draft(work).unwrap();
                assert_eq!(draft.dispatch_counts()[0] - before, 477);
                owner.record_draft(result).unwrap();
            }
            assert_eq!(owner.phase(), EngineeringTpSpeculativePhaseV1::Ready);
            for role in [Qwen3ModelRole::Target8B, Qwen3ModelRole::Draft06B] {
                owner.pool(role).check_invariants().unwrap();
            }
            target.close().unwrap();
            draft.close().unwrap();
        }
    }
}

#[test]
fn actual_full_acceptance_catchup_and_partial_next_round_use_fresh_ids_and_correct_anchor() {
    let (mut target, mut draft, mut owner, _) = paired(15, 4, 4);
    draft.recording_inner().inner.transports[0].rollover_supported = true;
    let last_first = generate(&mut owner, &mut draft, 15, 4, 100);
    target.inner.transports[0].choice_override = Some(vec![100, 101, 102, 103, 999]);
    let result = target
        .execute_speculative_target(owner.target_work().unwrap())
        .unwrap();
    owner.record_target(result).unwrap();
    let first = owner.settle().unwrap();
    assert_eq!((first.target_cursor(), first.draft_cursor()), (20, 19));
    owner.reserve_draft_catch_up().unwrap();
    let work = owner.draft_work().unwrap();
    let catch_up_id = work.batch().id();
    assert!(catch_up_id > last_first);
    assert_eq!(work.batch().rows()[0].token(), 103);
    let result = draft.execute_speculative_draft(work).unwrap();
    owner.record_draft(result).unwrap();
    let last_second = generate(&mut owner, &mut draft, 20, 4, 200);
    assert!(last_second > catch_up_id);
    target.inner.transports[0].choice_override = Some(vec![200, 201, 888, 203, 777]);
    let result = target
        .execute_speculative_target(owner.target_work().unwrap())
        .unwrap();
    owner.record_target(result).unwrap();
    let second = owner.settle().unwrap();
    assert_eq!((second.target_cursor(), second.draft_cursor()), (23, 23));
    assert_eq!(second.greedy_commit().emitted_tokens(), [200, 201, 888]);
    assert_eq!(draft.completed_batches(), 10);
    assert_eq!(draft.dispatch_counts(), [2 * 477 + 8 * 480]);
    owner.reserve_proposals().unwrap();
    {
        let work = owner.proposal_work().unwrap();
        assert_eq!(
            (
                work.batch().rows()[0].position(),
                work.batch().rows()[0].token()
            ),
            (23, 888)
        );
    }
    owner.fail_submitted().unwrap();
    target.close().unwrap();
    draft.close().unwrap();
}

#[test]
fn actual_draft_failure_after_a_sealed_row_never_mints_or_commits_the_next_choice() {
    for failure in [
        Failure::Write,
        Failure::Submit,
        Failure::Wait,
        Failure::BadChoice,
        Failure::PreparePackets,
    ] {
        let (mut target, mut draft, mut owner, _) = paired(15, 4, 0);
        owner.reserve_proposals().unwrap();
        let result = draft
            .execute_speculative_proposal(owner.proposal_work().unwrap())
            .unwrap();
        owner.record_proposal(result).unwrap();
        draft.recording_inner().inner.transports[0].failure = Some(failure);
        assert!(
            draft
                .execute_speculative_proposal(owner.proposal_work().unwrap())
                .is_err()
        );
        assert_eq!(draft.completed_batches(), 2);
        assert!(owner.target_work().is_err());
        assert!(owner.settle().is_err());
        owner.fail_submitted().unwrap();
        for role in [Qwen3ModelRole::Target8B, Qwen3ModelRole::Draft06B] {
            assert_eq!(owner.pool(role).stats().quarantined_pages, 4);
        }
        target.close().unwrap();
        draft.close().unwrap();
    }
}

#[test]
fn actual_target_failure_after_generated_proposals_quarantines_both_roles() {
    let (mut target, mut draft, mut owner, _) = paired(15, 4, 0);
    generate(&mut owner, &mut draft, 15, 4, 100);
    target.inner.transports[0].failure = Some(Failure::Wait);
    assert!(
        target
            .execute_speculative_target(owner.target_work().unwrap())
            .is_err()
    );
    owner.fail_submitted().unwrap();
    assert_eq!(owner.phase(), EngineeringTpSpeculativePhaseV1::Terminal);
    assert_eq!(owner.pool(Qwen3ModelRole::Target8B).stats().free_pages, 0);
    assert_eq!(owner.pool(Qwen3ModelRole::Draft06B).stats().free_pages, 0);
    target.close().unwrap();
    draft.close().unwrap();
}

#[test]
fn actual_k16_uses_existing_queue_rollover_between_completed_one_row_forwards() {
    let (mut target, mut draft, mut owner, _) = paired(15, 16, 16);
    let limit = fe2o3_kfd::engineering_wire::MAX_UNRETIRED_RING_PACKETS_V1;
    let transport = &mut draft.recording_inner().inner.transports[0];
    transport.rollover_supported = true;
    transport.queue_packets = limit - 479;
    generate(&mut owner, &mut draft, 15, 16, 100);
    let inner = draft.recording_inner();
    assert!(inner.inner.transports[0].queue_epochs >= 1);
    assert!(
        inner.inner.transports[0]
            .packet_preparations
            .iter()
            .all(|count| *count == 480)
    );
    assert_eq!(draft.completed_batches(), 17);
    target.inner.transports[0].choice_override = Some((100..117).collect());
    let result = target
        .execute_speculative_target(owner.target_work().unwrap())
        .unwrap();
    owner.record_target(result).unwrap();
    assert_eq!(
        owner
            .settle()
            .unwrap()
            .greedy_commit()
            .accepted_draft_tokens(),
        16
    );
    target.close().unwrap();
    draft.close().unwrap();
}

#[test]
fn absent_rollover_rejects_before_next_forward_and_foreign_driver_cannot_seal_a_ticket() {
    let (mut target, mut draft, mut owner, _) = paired(0, 16, 0);
    owner.reserve_proposals().unwrap();
    draft.recording_inner().completed_batches =
        fe2o3_kfd::engineering_wire::MAX_UNRETIRED_RING_PACKETS_V1 / 480;
    assert!(
        draft
            .execute_speculative_proposal(owner.proposal_work().unwrap())
            .is_err()
    );
    assert_eq!(draft.dispatch_counts(), [0]);
    owner.fail_submitted().unwrap();
    target.close().unwrap();
    draft.close().unwrap();

    let (mut target, mut draft, _, _) = paired(0, 4, 0);
    let (mut other_target, mut other_draft, mut owner, _) = paired(0, 4, 0);
    owner.reserve_proposals().unwrap();
    assert!(
        draft
            .execute_speculative_proposal(owner.proposal_work().unwrap())
            .is_err()
    );
    assert_eq!(draft.dispatch_counts(), [0]);
    owner.fail_submitted().unwrap();
    target.close().unwrap();
    draft.close().unwrap();
    other_target.close().unwrap();
    other_draft.close().unwrap();
}
