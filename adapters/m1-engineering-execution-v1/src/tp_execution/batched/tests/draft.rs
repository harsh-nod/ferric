//! Actual forward/settlement orchestration with synthetic byte outputs, not GPU math.

use super::*;
use crate::tp_artifact::{DraftBindingV10, ENGINEERING_DRAFT_BATCH32_EXPORTS_V10};
use crate::tp_paged::speculative::tests::index;
use crate::tp_paged::speculative::{EngineeringTpSpeculativeKvV1, EngineeringTpSpeculativePhaseV1};

fn model() -> ModelConfig {
    ModelConfig {
        role: Qwen3ModelRole::Draft06B,
        model_id: ferric_spec::Identity::new([3; 32]),
        layers: 28,
        hidden_size: 1024,
        intermediate_size: 3072,
        query_heads: 16,
        tie_word_embeddings: true,
        ..target()
    }
}

fn draft_pool() -> EngineeringTpPagedPoolV1 {
    EngineeringTpPagedPoolV1::new_wide32(
        EngineeringTpPoolScopeV1 {
            model: [3; 32],
            session: [4; 32],
        },
        EngineeringTpPagedLimitsV1::new(64, 32, 4, 100).unwrap(),
    )
    .unwrap()
}

fn driver(
    pool: &EngineeringTpPagedPoolV1,
    mfma: bool,
) -> EngineeringTpDraftBatchExecutionV10<Recording> {
    let mut inner = fixture_for_model(1, pool, model());
    inner.inner.draft_v10 = true;
    inner.inner.transports[0].sequences_supported = false;
    inner.inner.transports[0].ordered_supported = false;
    inner.configure_output_head_pruning(true).unwrap();
    let scratch = allocate_tensor(&mut inner.inner.transports[0], 32 * 1024, 2).unwrap();
    inner.inner.reduction = super::super::super::ReductionWorkspace::DeviceTp1(scratch);
    inner.fp32_logits =
        Some(allocate_tensor(&mut inner.inner.transports[0], 32 * 151_936, 4).unwrap());
    inner.head_profile_configured = true;
    if mfma {
        let original = inner.inner.ranks[0].globals[0].1.id;
        let transposed = allocate_tensor(&mut inner.inner.transports[0], 1, 2).unwrap();
        inner.projection =
            super::super::super::projection::ProjectionPolicy::synthetic_mfma_for_recording(
                original, transposed,
            );
    }
    EngineeringTpDraftBatchExecutionV10::recording(inner)
}

#[derive(Default)]
struct Admission {
    image: Option<[u8; 32]>,
    checks: usize,
    allocations: usize,
    sequences: bool,
    ordered: bool,
    peer: bool,
}
impl EngineeringTpRankTransportV1 for Admission {
    fn require_loaded_image(&mut self, image: [u8; 32], roots: &[&str]) -> TpResult<()> {
        self.checks += 1;
        if self.image != Some(image)
            || roots != ENGINEERING_DRAFT_BATCH32_EXPORTS_V10
            || self.allocations != 0
        {
            return Err("wrong complete draft image".into());
        }
        Ok(())
    }
    fn supports_sequences(&self) -> bool {
        self.sequences
    }
    fn supports_ordered_batches(&self) -> bool {
        self.ordered
    }
    fn peer_group_rank(&self) -> Option<(u32, u32, u32)> {
        self.peer.then_some((1, 0, 1))
    }
    fn allocate(&mut self, _: usize) -> TpResult<u64> {
        self.allocations += 1;
        Err("no allocations in admission test".into())
    }
    fn write(&mut self, _: u64, _: usize, _: &[u8]) -> TpResult<()> {
        Err("no payload".into())
    }
    fn read(&mut self, _: u64, _: usize, _: &mut [u8]) -> TpResult<()> {
        Err("no payload".into())
    }
    fn submit(&mut self, _: &EngineeringTpDispatchV1) -> TpResult<()> {
        Err("no dispatch".into())
    }
    fn wait(&mut self) -> TpResult<()> {
        Err("no dispatch".into())
    }
    fn close(&mut self) -> TpResult<()> {
        Ok(())
    }
}

#[test]
fn exact_draft_model_scope_roster_and_transport_are_required_before_allocation() {
    use super::super::draft::validate_binding;
    let binding = DraftBindingV10::recording();
    let pool = draft_pool();
    let mut transport = [Admission::default()];
    assert!(validate_binding(&mut transport, model(), &pool, binding).is_err());
    assert_eq!(transport[0].checks, 1);
    transport[0].image = Some(binding.hsaco);
    validate_binding(&mut transport, model(), &pool, binding).unwrap();
    for case in 0..7 {
        let mut transport = [Admission {
            image: Some(binding.hsaco),
            ..Admission::default()
        }];
        let mut wrong = model();
        match case {
            0 => wrong = target(),
            1 => wrong.model_id = ferric_spec::Identity::new([8; 32]),
            2 => wrong.hidden_size = 4096,
            3 => transport[0].sequences = true,
            4 => transport[0].ordered = true,
            5 => transport[0].peer = true,
            _ => wrong.tie_word_embeddings = false,
        }
        assert!(validate_binding(&mut transport, wrong, &pool, binding).is_err());
        assert_eq!((transport[0].checks, transport[0].allocations), (0, 0));
    }
    let mut used = draft_pool();
    used.open_sequence(used.scope(), &[7], 0).unwrap();
    assert!(validate_binding(&mut transport, model(), &used, binding).is_err());
    assert!(validate_binding(&mut transport, model(), &super::pool(), binding).is_err());
    assert!(validate_binding::<Admission>(&mut [], model(), &pool, binding).is_err());
    assert!(
        validate_binding(
            &mut [Admission::default(), Admission::default()],
            model(),
            &pool,
            binding
        )
        .is_err()
    );
    assert_eq!(transport[0].allocations, 0);
}

#[test]
fn actual_draft_forward_uses_only_v10_with_exact_active_extents_and_head_selection() {
    for mfma in [false, true] {
        for rows in [1_usize, 16, 17, 32] {
            let mut pool = draft_pool();
            let mut driver = driver(&pool, mfma);
            let batch = prepare(&mut pool, u32::try_from(rows).unwrap());
            pool.begin_submission(&batch).unwrap();
            let selected = if rows == 1 {
                vec![0]
            } else {
                vec![0, rows - 1]
            };
            let output = driver.execute_selected(&batch, &selected).unwrap();
            assert_eq!(
                output.choices,
                (42..42 + u32::try_from(selected.len()).unwrap()).collect::<Vec<_>>()
            );
            assert_eq!(driver.dispatch_counts(), [480]);
            assert_eq!(driver.expected_dispatch_counts(selected.len()), [480]);
            assert_eq!(driver.expected_dispatch_counts(0), [477]);
            assert_eq!(driver.row_capacity(), 32);
            assert_eq!(driver.image_id(), [110; 32]);
            let commands = &driver.recording_inner().inner.transports[0].commands;
            assert!(
                commands
                    .iter()
                    .all(|c| ENGINEERING_DRAFT_BATCH32_EXPORTS_V10.contains(&c.kernel))
            );
            assert_eq!(
                commands
                    .iter()
                    .filter(|c| c.kernel == ENGINEERING_DRAFT_BATCH32_EXPORTS_V10[8])
                    .count(),
                28
            );
            assert_eq!(
                commands
                    .iter()
                    .filter(|c| c.kernel == ENGINEERING_DRAFT_BATCH32_EXPORTS_V10[10])
                    .count(),
                56
            );
            let projection = &commands[2];
            assert_eq!(
                projection.kernel,
                ENGINEERING_DRAFT_BATCH32_EXPORTS_V10[if mfma { 3 } else { 2 }]
            );
            assert_eq!(
                projection.grid_workgroups,
                u32::try_from(rows.div_ceil(16) * 128).unwrap()
            );
            assert_eq!(buffer(&commands[0], 2).2, 32 * 1024);
            assert_eq!(scalar(&commands[0], 3), u32::try_from(rows).unwrap());
            let head = &commands[commands.len() - 2];
            assert_eq!(
                head.kernel,
                ENGINEERING_DRAFT_BATCH32_EXPORTS_V10[if mfma { 12 } else { 11 }]
            );
            assert_eq!(scalar(head, 3), u32::try_from(selected.len()).unwrap());
            assert_eq!(buffer(head, 2).3, 4);
            let expected_weight = buffer(projection, 1).0;
            assert_eq!(buffer(head, 1).0, expected_weight);
            pool.commit_batch(&batch, output.completion).unwrap();
            assert!(driver.execute_selected(&batch, &selected).is_err());
            driver.close().unwrap();
        }
    }
}

fn paired(
    cursor: usize,
    k: u8,
    accepted: u8,
) -> (
    EngineeringTpBatchExecutionV2<Recording>,
    EngineeringTpDraftBatchExecutionV10<Recording>,
    EngineeringTpSpeculativeKvV1,
    ferric_spec::SpeculativeKvRoundIndex,
) {
    let mut target_pool = wide_pool();
    let mut draft_pool = draft_pool();
    let mut target = fixture(1, &target_pool);
    target.configure_output_head_pruning(true).unwrap();
    let mut draft = driver(&draft_pool, true);
    let ts = target_pool
        .open_sequence(target_pool.scope(), &[7], 0)
        .unwrap()
        .sequence();
    let ds = draft_pool
        .open_sequence(draft_pool.scope(), &[7], 0)
        .unwrap()
        .sequence();
    if cursor != 0 {
        for (pool, sequence, role) in [(&mut target_pool, ts, false), (&mut draft_pool, ds, true)] {
            let rows = (0..cursor)
                .map(|i| EngineeringTpPageRowV1 {
                    sequence,
                    position: u32::try_from(i).unwrap(),
                    token: 7,
                })
                .collect::<Vec<_>>();
            let batch = pool.reserve_batch(&rows).unwrap();
            pool.begin_submission(&batch).unwrap();
            let output = if role {
                draft.execute_selected(&batch, &[])
            } else {
                target.execute_selected(&batch, &[])
            }
            .unwrap();
            pool.commit_batch(&batch, output.completion).unwrap();
        }
    }
    let mut index = index(u32::try_from(cursor).unwrap(), 1, k, 77);
    for (i, token) in index.draft_tokens[..usize::from(k)].iter_mut().enumerate() {
        *token = if i == usize::from(accepted) {
            999
        } else {
            42 + u32::try_from(i).unwrap()
        };
    }
    let owner = EngineeringTpSpeculativeKvV1::new(target_pool, ts, draft_pool, ds, &index).unwrap();
    (target, draft, owner, index)
}

#[test]
fn actual_paired_drivers_settle_zero_partial_and_full_across_page_boundary() {
    for k in [4, 8, 16] {
        for accepted in [0, k / 2, k] {
            let (mut target, mut draft, mut owner, index) = paired(15, k, accepted);
            owner.reserve_round(&index).unwrap();
            let target_result = target
                .execute_speculative_target(owner.target_work().unwrap())
                .unwrap();
            owner.record_target(target_result).unwrap();
            let before = draft.dispatch_counts()[0];
            let work = owner.draft_work().unwrap();
            assert!(!work.is_catch_up());
            assert_eq!(work.batch().rows().len(), usize::from(k));
            let result = draft.execute_speculative_draft(work).unwrap();
            assert_eq!(draft.dispatch_counts()[0] - before, 477);
            owner.record_draft(result).unwrap();
            let receipt = owner.settle().unwrap();
            assert_eq!(
                receipt.greedy_commit().accepted_draft_tokens(),
                usize::from(accepted)
            );
            assert_eq!(receipt.target_cursor(), 16 + u32::from(accepted));
            assert_eq!(
                receipt.draft_cursor(),
                15 + u32::from((accepted + 1).min(k))
            );
            assert_eq!(owner.next_anchor(), 42 + u32::from(accepted));
            for role in [Qwen3ModelRole::Target8B, Qwen3ModelRole::Draft06B] {
                owner.pool(role).check_invariants().unwrap();
                assert_eq!(owner.pool(role).stats().cached_pages, 0);
            }
            target.close().unwrap();
            draft.close().unwrap();
        }
    }
}

#[test]
fn actual_full_acceptance_catch_up_consumes_last_proposal_not_bonus_then_next_round() {
    let (mut target, mut draft, mut owner, first) = paired(15, 4, 4);
    owner.reserve_round(&first).unwrap();
    let result = target
        .execute_speculative_target(owner.target_work().unwrap())
        .unwrap();
    owner.record_target(result).unwrap();
    let result = draft
        .execute_speculative_draft(owner.draft_work().unwrap())
        .unwrap();
    owner.record_draft(result).unwrap();
    let settled = owner.settle().unwrap();
    assert_eq!((settled.target_cursor(), settled.draft_cursor()), (20, 19));
    let mut next = index(20, 2, 4, 46);
    next.draft_tokens[..4].copy_from_slice(&[42, 43, 999, 45]);
    assert!(owner.reserve_round(&next).is_err());
    owner.reserve_draft_catch_up().unwrap();
    let work = owner.draft_work().unwrap();
    assert!(work.is_catch_up());
    assert_eq!(
        (
            work.batch().rows()[0].position(),
            work.batch().rows()[0].token()
        ),
        (19, 45)
    );
    let result = draft.execute_speculative_draft(work).unwrap();
    let inner = draft.recording_inner();
    let token_id = inner.inner.ranks[0].token.id;
    assert_eq!(inner.inner.transports[0].u32_values(token_id, 1), [45]);
    assert_eq!(
        inner.inner.transports[0].u32_values(inner.positions[0].id, 1),
        [19]
    );
    assert!(owner.reserve_round(&next).is_err());
    owner.record_draft(result).unwrap();
    assert_eq!(owner.phase(), EngineeringTpSpeculativePhaseV1::Ready);
    owner.reserve_round(&next).unwrap();
    let result = target
        .execute_speculative_target(owner.target_work().unwrap())
        .unwrap();
    owner.record_target(result).unwrap();
    let result = draft
        .execute_speculative_draft(owner.draft_work().unwrap())
        .unwrap();
    owner.record_draft(result).unwrap();
    let settled = owner.settle().unwrap();
    assert_eq!((settled.target_cursor(), settled.draft_cursor()), (23, 23));
    assert_eq!(settled.greedy_commit().emitted_tokens(), &[42, 43, 44]);
    assert_eq!(draft.completed_batches(), 4);
    assert_eq!(draft.dispatch_counts(), [4 * 477]);
    target.close().unwrap();
    draft.close().unwrap();
}

#[test]
fn draft_failure_and_foreign_ticket_never_mint_success_or_publish_either_pool() {
    for failure in [Failure::Write, Failure::Wait, Failure::ResidualWait] {
        let (mut target, mut draft, mut owner, index) = paired(0, 4, 2);
        owner.reserve_round(&index).unwrap();
        let result = target
            .execute_speculative_target(owner.target_work().unwrap())
            .unwrap();
        owner.record_target(result).unwrap();
        draft.recording_inner().inner.transports[0].failure = Some(failure);
        assert!(
            draft
                .execute_speculative_draft(owner.draft_work().unwrap())
                .is_err()
        );
        assert_eq!(draft.completed_batches(), 0);
        owner.fail_submitted().unwrap();
        assert_eq!(owner.phase(), EngineeringTpSpeculativePhaseV1::Terminal);
        for role in [Qwen3ModelRole::Target8B, Qwen3ModelRole::Draft06B] {
            assert_eq!(owner.pool(role).stats().quarantined_pages, 4);
        }
        target.close().unwrap();
        draft.close().unwrap();
    }
    let (_, mut draft, _, _) = paired(0, 4, 2);
    let (_, _, mut foreign, index) = paired(0, 4, 2);
    foreign.reserve_round(&index).unwrap();
    assert!(
        draft
            .execute_speculative_draft(foreign.draft_work().unwrap())
            .is_err()
    );
    assert_eq!(draft.dispatch_counts(), [0]);
    foreign.fail_submitted().unwrap();
    draft.close().unwrap();
}

#[test]
fn failed_actual_catch_up_keeps_next_round_blocked_and_quarantines_both_roles() {
    let (mut target, mut draft, mut owner, first) = paired(0, 4, 4);
    owner.reserve_round(&first).unwrap();
    let result = target
        .execute_speculative_target(owner.target_work().unwrap())
        .unwrap();
    owner.record_target(result).unwrap();
    let result = draft
        .execute_speculative_draft(owner.draft_work().unwrap())
        .unwrap();
    owner.record_draft(result).unwrap();
    owner.settle().unwrap();
    owner.reserve_draft_catch_up().unwrap();
    draft.recording_inner().inner.transports[0].failure = Some(Failure::Wait);
    assert!(
        draft
            .execute_speculative_draft(owner.draft_work().unwrap())
            .is_err()
    );
    assert_eq!(draft.completed_batches(), 1);
    owner.fail_submitted().unwrap();
    assert!(owner.reserve_round(&index(5, 2, 4, 46)).is_err());
    for role in [Qwen3ModelRole::Target8B, Qwen3ModelRole::Draft06B] {
        assert_eq!(owner.pool(role).stats().quarantined_pages, 4);
    }
    target.close().unwrap();
    draft.close().unwrap();
}
