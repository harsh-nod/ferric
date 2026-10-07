use super::*;
use crate::tp_paged::speculative::tests::{index, record_draft_for_test};
use crate::tp_paged::speculative::{EngineeringTpSpeculativeKvV1, EngineeringTpSpeculativePhaseV1};

fn setup(
    k: u8,
) -> (
    EngineeringTpBatchExecutionV2<Recording>,
    EngineeringTpSpeculativeKvV1,
    ferric_spec::SpeculativeKvRoundIndex,
) {
    let mut target_pool = wide_pool();
    let mut engine = fixture(1, &target_pool);
    engine.configure_output_head_pruning(true).unwrap();
    let target_sequence = target_pool
        .open_sequence(target_pool.scope(), &[77], 0)
        .unwrap()
        .sequence();
    let mut draft_pool = EngineeringTpPagedPoolV1::new_wide32(
        EngineeringTpPoolScopeV1 {
            model: [3; 32],
            session: [4; 32],
        },
        EngineeringTpPagedLimitsV1::new(64, 32, 4, 100).unwrap(),
    )
    .unwrap();
    let draft_sequence = draft_pool
        .open_sequence(draft_pool.scope(), &[77], 0)
        .unwrap()
        .sequence();
    let mut index = index(0, 1, k, 77);
    for (ordinal, token) in index
        .draft_tokens
        .iter_mut()
        .take(usize::from(k))
        .enumerate()
    {
        *token = 42 + u32::try_from(ordinal).unwrap();
    }
    let owner = EngineeringTpSpeculativeKvV1::new(
        target_pool,
        target_sequence,
        draft_pool,
        draft_sequence,
        &index,
    )
    .unwrap();
    (engine, owner, index)
}

#[test]
fn actual_driver_seals_every_target_choice_and_row_before_paired_settlement() {
    for k in [4, 8, 16] {
        let (mut engine, mut owner, index) = setup(k);
        owner.reserve_round(&index).unwrap();
        let result = engine
            .execute_speculative_target(owner.target_work().unwrap())
            .unwrap();
        assert_eq!(engine.completed_batches(), 1);
        assert_eq!(engine.dispatch_counts(), vec![544]);
        owner.record_target(result).unwrap();
        record_draft_for_test(&mut owner);
        let receipt = owner.settle().unwrap();
        assert_eq!(
            receipt.greedy_commit().accepted_draft_tokens(),
            usize::from(k)
        );
        assert_eq!(
            receipt.greedy_commit().emitted_tokens(),
            &(42..=42 + u32::from(k)).collect::<Vec<_>>()
        );
        assert_eq!(receipt.target_cursor(), u32::from(k) + 1);
        assert_eq!(receipt.draft_cursor(), u32::from(k));
        assert_eq!(
            owner.phase(),
            EngineeringTpSpeculativePhaseV1::DraftCatchUpRequired
        );
        engine.close().unwrap();
    }
}

#[test]
fn actual_driver_failure_has_no_opaque_success_and_pool_remains_uncommitted() {
    for failure in [Failure::Write, Failure::Wait, Failure::BadChoice] {
        let (mut engine, mut owner, index) = setup(4);
        engine.inner.transports[0].failure = Some(failure);
        owner.reserve_round(&index).unwrap();
        assert!(
            engine
                .execute_speculative_target(owner.target_work().unwrap())
                .is_err()
        );
        assert_eq!(engine.completed_batches(), 0);
        owner.fail_submitted().unwrap();
        for role in [Qwen3ModelRole::Target8B, Qwen3ModelRole::Draft06B] {
            assert_eq!(owner.pool(role).stats().quarantined_pages, 4);
        }
        engine.close().unwrap();
    }
}

#[test]
fn foreign_target_ticket_rejects_before_dispatch() {
    let (mut engine, _, _) = setup(4);
    let (_, mut owner, index) = setup(4);
    owner.reserve_round(&index).unwrap();
    assert!(
        engine
            .execute_speculative_target(owner.target_work().unwrap())
            .is_err()
    );
    assert_eq!(engine.dispatch_counts(), vec![0]);
    owner.fail_submitted().unwrap();
    engine.close().unwrap();
}
