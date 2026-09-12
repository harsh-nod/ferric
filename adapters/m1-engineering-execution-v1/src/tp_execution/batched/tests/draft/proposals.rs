//! Real host forwards with injected compact choices, not GPU numerical evidence.

use super::{
    EngineeringTpBatchExecutionV2, EngineeringTpDraftBatchExecutionV10, EngineeringTpPageRowV1,
    EngineeringTpPagedLimitsV1, EngineeringTpPagedPoolV1, EngineeringTpReductionModeV3,
    EngineeringTpSpeculativeKvV1, EngineeringTpSpeculativePhaseV1, Failure, Qwen3ModelRole,
    Qwen3TensorKind, Recording, allocate_tensor, draft_pool, driver, fixture, index, paired,
    wide_pool,
};
use crate::tp_paged::EngineeringTpSequenceIdV1;

fn resize_recording_to_160(driver: &mut EngineeringTpBatchExecutionV2<Recording>) {
    // The shared Recording fixture is fixed at 64 tokens; native constructors
    // already allocate from pool limits. Keep this larger synthetic case local.
    driver.context_tokens = 160;
    driver.physical_pages = 10;
    driver.table_stride = 10;
    driver.inner.capacity = 160;
    driver.inner.sequence =
        super::TensorParallelSequenceV1::new(160, driver.inner.plan.model().vocabulary_size)
            .unwrap();
    let transport = &mut driver.inner.transports[0];
    let rank = &mut driver.inner.ranks[0];
    let elements = usize::try_from(rank.geometry.kv_channels.count).unwrap() * 160;
    for layer in &mut rank.layers {
        transport.buffers.remove(&layer.k_cache.id).unwrap();
        transport.buffers.remove(&layer.v_cache.id).unwrap();
        layer.k_cache = allocate_tensor(transport, elements, 2).unwrap();
        layer.v_cache = allocate_tensor(transport, elements, 2).unwrap();
    }
    transport.buffers.remove(&driver.page_tables[0].id).unwrap();
    driver.page_tables[0] = allocate_tensor(transport, 32 * 10, 4).unwrap();
}

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
    let epoch = owner.next_epoch();
    let pool_identity = owner.pool(Qwen3ModelRole::Draft06B).identity();
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
        assert_eq!(result.ordinal(), ordinal);
        assert_eq!(result.batch_id(), previous_id);
        assert_eq!(result.pool_identity(), pool_identity);
        assert_eq!(result.completion_epoch(), epoch);
        assert_eq!(result.request(), ferric_spec::RequestId::new(2, 3));
        assert_eq!(result.input().token, expected_token);
        assert_eq!(result.input().position, cursor + u32::from(ordinal));
        assert_eq!(result.choice(), base + u32::from(ordinal));
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

fn paired_127() -> (
    EngineeringTpBatchExecutionV2<Recording>,
    EngineeringTpDraftBatchExecutionV10<Recording>,
    EngineeringTpSpeculativeKvV1,
    EngineeringTpSequenceIdV1,
    EngineeringTpSequenceIdV1,
) {
    let limits = EngineeringTpPagedLimitsV1::new(160, 1, 10, 100).unwrap();
    let mut target_pool =
        EngineeringTpPagedPoolV1::new_wide32(wide_pool().scope(), limits).unwrap();
    let mut draft_pool =
        EngineeringTpPagedPoolV1::new_wide32(draft_pool().scope(), limits).unwrap();
    let mut target = fixture(1, &target_pool);
    resize_recording_to_160(&mut target);
    target.configure_output_head_pruning(true).unwrap();
    target
        .configure_reduction(EngineeringTpReductionModeV3::DeviceTp1V3)
        .unwrap();
    let original = target.inner.ranks[0].global(Qwen3TensorKind::LanguageModelHead);
    let transposed = allocate_tensor(&mut target.inner.transports[0], 1, 2).unwrap();
    target.projection =
        crate::tp_execution::projection::ProjectionPolicy::synthetic_mfma_for_recording(
            original.id,
            transposed,
        );
    target.configure_head_precision_v8(true).unwrap();
    target.inner.transports[0].rollover_supported = true;
    let mut draft = driver(&draft_pool, false);
    resize_recording_to_160(draft.recording_inner());
    draft.recording_inner().inner.transports[0].rollover_supported = true;
    let mut prompt = vec![7; 128];
    prompt[127] = 323;
    let target_sequence = target_pool
        .open_sequence(target_pool.scope(), &prompt, 0)
        .unwrap()
        .sequence();
    let draft_sequence = draft_pool
        .open_sequence(draft_pool.scope(), &prompt, 0)
        .unwrap()
        .sequence();
    for (pool, sequence, is_draft) in [
        (&mut target_pool, target_sequence, false),
        (&mut draft_pool, draft_sequence, true),
    ] {
        for start in (0..127).step_by(16) {
            let rows = (start..(start + 16).min(127))
                .map(|position| EngineeringTpPageRowV1 {
                    sequence,
                    position,
                    token: 7,
                })
                .collect::<Vec<_>>();
            let batch = pool.reserve_batch(&rows).unwrap();
            pool.begin_submission(&batch).unwrap();
            let output = if is_draft {
                draft.execute_selected(&batch, &[])
            } else {
                target.execute_selected(&batch, &[])
            }
            .unwrap();
            assert!(output.choices.is_empty());
            pool.commit_batch(&batch, output.completion).unwrap();
        }
        assert_eq!(pool.committed_position(sequence).unwrap(), 127);
    }
    assert_eq!(target.dispatch_counts(), [8 * 613]);
    assert_eq!(draft.dispatch_counts(), [8 * 477]);
    let mut bootstrap = index(127, 1, 4, 323);
    bootstrap.draft_tokens = [0; 16];
    let owner = EngineeringTpSpeculativeKvV1::new(
        target_pool,
        target_sequence,
        draft_pool,
        draft_sequence,
        &bootstrap,
    )
    .unwrap();
    (target, draft, owner, target_sequence, draft_sequence)
}

#[test]
fn paired_k4_two_rounds_from_127_cross_pages_and_finish_final_catchup() {
    for acceptances in [[0_u32, 4], [4, 0], [2, 4], [4, 2], [4, 4]] {
        let (mut target, mut draft, mut owner, ts, ds) = paired_127();
        let mut cursor = 127;
        let mut catch_ups = 0;
        let mut last_target_batch = 8;
        let mut last_draft_batch = 8;
        for (round, accepted) in acceptances.into_iter().enumerate() {
            let epoch = owner.next_epoch();
            let anchor = owner.next_anchor();
            let base = 100 + 100 * u32::try_from(round).unwrap();
            let proposal_last = generate(&mut owner, &mut draft, cursor, 4, base);
            assert!(proposal_last > last_draft_batch);
            last_draft_batch = proposal_last;
            let mut choices = (base..base + 4).collect::<Vec<_>>();
            choices.push(999);
            choices[usize::try_from(accepted).unwrap()] = 999;
            target.inner.transports[0].choice_override = Some(choices.clone());
            let result = target
                .execute_speculative_target(owner.target_work().unwrap())
                .unwrap();
            assert_eq!(result.completion_epoch(), epoch);
            assert_eq!(result.request(), ferric_spec::RequestId::new(2, 3));
            assert_eq!(
                result.pool_identity(),
                owner.pool(Qwen3ModelRole::Target8B).identity()
            );
            assert!(result.batch_id() > last_target_batch);
            last_target_batch = result.batch_id();
            assert_eq!(result.output_rows(), [0, 1, 2, 3, 4]);
            assert_eq!(result.choices(), choices);
            assert_eq!(
                result
                    .inputs()
                    .iter()
                    .map(|row| row.token)
                    .collect::<Vec<_>>(),
                [anchor, base, base + 1, base + 2, base + 3]
            );
            assert_eq!(
                result
                    .inputs()
                    .iter()
                    .map(|row| row.position)
                    .collect::<Vec<_>>(),
                (cursor..cursor + 5).collect::<Vec<_>>()
            );
            owner.record_target(result).unwrap();
            let settled = owner.settle().unwrap();
            assert_eq!(
                settled.greedy_commit().accepted_draft_tokens(),
                usize::try_from(accepted).unwrap()
            );
            assert_eq!(
                settled.greedy_commit().emitted_tokens(),
                &choices[..=usize::try_from(accepted).unwrap()]
            );
            assert_eq!(settled.target_cursor(), cursor + accepted + 1);
            assert_eq!(settled.draft_cursor(), cursor + (accepted + 1).min(4));
            assert_eq!(settled.draft_catch_up_required(), accepted == 4);
            if accepted == 4 {
                assert_eq!(
                    owner.phase(),
                    EngineeringTpSpeculativePhaseV1::DraftCatchUpRequired
                );
                assert!(owner.reserve_proposals().is_err());
                owner.reserve_draft_catch_up().unwrap();
                let result = draft
                    .execute_speculative_draft(owner.draft_work().unwrap())
                    .unwrap();
                assert!(result.is_catch_up());
                assert_eq!(result.completion_epoch(), owner.next_epoch());
                assert_eq!(
                    result.pool_identity(),
                    owner.pool(Qwen3ModelRole::Draft06B).identity()
                );
                assert!(result.batch_id() > last_draft_batch);
                last_draft_batch = result.batch_id();
                assert_eq!(result.inputs().len(), 1);
                assert_eq!(
                    (result.inputs()[0].token, result.inputs()[0].position),
                    (base + 3, cursor + 4)
                );
                owner.record_draft(result).unwrap();
                catch_ups += 1;
            }
            cursor += accepted + 1;
            assert_eq!(owner.phase(), EngineeringTpSpeculativePhaseV1::Ready);
            assert_eq!(owner.next_anchor(), 999);
            assert_eq!(
                owner.next_epoch().value(),
                u64::try_from(round).unwrap() + 2
            );
            for (role, sequence) in [
                (Qwen3ModelRole::Target8B, ts),
                (Qwen3ModelRole::Draft06B, ds),
            ] {
                let pool = owner.pool(role);
                pool.check_invariants().unwrap();
                assert_eq!(pool.committed_position(sequence).unwrap(), cursor);
                assert_eq!(
                    (pool.stats().cached_pages, pool.stats().quarantined_pages),
                    (0, 0)
                );
            }
        }
        assert_eq!(target.completed_batches(), 10);
        assert_eq!(target.dispatch_counts(), [6136]);
        assert_eq!(draft.completed_batches(), 16 + catch_ups);
        assert_eq!(
            draft.dispatch_counts(),
            [8 * 477 + 8 * 480 + catch_ups * 477]
        );
        target.close().unwrap();
        draft.close().unwrap();
    }
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
