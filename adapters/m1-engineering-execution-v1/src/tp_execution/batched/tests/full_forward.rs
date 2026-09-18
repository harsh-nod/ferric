//! Recording/transaction fixtures, not GPU arithmetic or performance evidence.

use super::super::super::{EngineeringTpProjectionModeV3, ReductionWorkspace};
use super::*;

fn configured(pool: &EngineeringTpPagedPoolV1) -> EngineeringTpBatchExecutionV2<Recording> {
    let mut driver = fixture(1, pool);
    driver
        .configure_reduction(EngineeringTpReductionModeV3::DeviceTp1V3)
        .unwrap();
    // Synthetic weights are not authenticated dense model intake.
    driver.projection_configured = true;
    driver
}

#[test]
fn full_forward_preserves_all_commands_io_and_committed_state() {
    for selected in [Vec::new(), vec![0]] {
        let mut recordings = Vec::new();
        for full in [false, true] {
            let mut pool = pool();
            let mut driver = configured(&pool);
            if full {
                driver.configure_scalar_v3_full_forward().unwrap();
            }
            let mut choices = Vec::new();
            for epoch in 1..=2 {
                let batch = prepare(&mut pool, 1);
                pool.begin_submission(&batch).unwrap();
                let result = driver.execute_selected(&batch, &selected).unwrap();
                choices.push(result.choices.clone());
                assert_eq!(driver.dispatch_counts(), vec![616 * epoch]);
                assert_eq!(driver.inner.collective.expected().epoch, epoch);
                assert_eq!(driver.inner.collective.expected().layer, 0);
                assert_eq!(driver.inner.hidden.len(), 4096);
                assert!(driver.inner.full_forward.is_none());
                pool.commit_batch(&batch, result.completion).unwrap();
            }
            let transport = &driver.inner.transports[0];
            assert_eq!(transport.packet_preparations, vec![616, 616]);
            assert_eq!(transport.commands.len(), 1232);
            assert_eq!(transport.reads.len(), 2);
            let events = transport.events.borrow();
            let full_events = events
                .iter()
                .filter(|event| {
                    matches!(
                        event,
                        Event::FullForwardSubmit(..) | Event::FullForwardWait(..)
                    )
                })
                .cloned()
                .collect::<Vec<_>>();
            assert_eq!(
                full_events,
                if full {
                    (0..2)
                        .flat_map(|_| {
                            [
                                Event::FullForwardSubmit(0, 616),
                                Event::FullForwardWait(0, 616),
                            ]
                        })
                        .collect::<Vec<_>>()
                } else {
                    Vec::new()
                }
            );
            assert!(!events.iter().any(|event| matches!(
                event,
                Event::OrderedSubmit(..) | Event::SequenceSubmit(..)
            )));
            for layer in 0..36 {
                for residual in [1 + layer * 17 + 10, 1 + layer * 17 + 16] {
                    let command = &transport.commands[residual];
                    assert_eq!(command.kernel, "ferric_qwen3_tp_batch_residual_bf16_v3");
                    assert_eq!(scalar(command, 3), 1);
                    assert_eq!(
                        buffer(command, 2).0,
                        buffer(&transport.commands[residual + 1], 0).0
                    );
                }
            }
            let serial_events = events
                .iter()
                .filter(|event| {
                    !matches!(
                        event,
                        Event::FullForwardSubmit(..) | Event::FullForwardWait(..)
                    )
                })
                .cloned()
                .collect::<Vec<_>>();
            recordings.push((
                transport.commands.clone(),
                transport.reads.clone(),
                transport.write_payloads.clone(),
                transport.buffers.clone(),
                serial_events,
                choices,
                driver.inner.hidden.clone(),
                driver.inner.ranks[0].hidden,
                driver.inner.collective,
                driver.last_batch,
                driver.completed_batches,
            ));
            drop(events);
            driver.close().unwrap();
        }
        assert_eq!(recordings[0], recordings[1]);
    }
}

#[test]
fn full_forward_failures_do_not_commit_live_state_or_allow_retry() {
    for failure in [
        Failure::FullForwardSubmit,
        Failure::FullForwardWait,
        Failure::Read,
        Failure::BadChoice,
        Failure::Write,
        Failure::PreparePackets,
    ] {
        let mut pool = pool();
        let mut driver = configured(&pool);
        driver.configure_scalar_v3_full_forward().unwrap();
        let initial_hidden = driver.inner.ranks[0].hidden;
        let ReductionWorkspace::DeviceTp1(initial_scratch) = driver.inner.reduction else {
            panic!("device residual required");
        };
        let initial_cursor = driver.inner.collective;
        let initial_host = driver.inner.hidden.clone();
        driver.inner.transports[0].failure = Some(failure);
        let batch = prepare(&mut pool, 1);
        pool.begin_submission(&batch).unwrap();
        assert!(driver.execute(&batch).is_err(), "{failure:?}");
        assert_eq!(driver.inner.ranks[0].hidden, initial_hidden);
        let ReductionWorkspace::DeviceTp1(scratch) = driver.inner.reduction else {
            panic!("device residual required");
        };
        assert_eq!(scratch, initial_scratch);
        assert_eq!(driver.inner.collective, initial_cursor);
        assert_eq!(driver.inner.hidden, initial_host);
        assert_eq!(driver.dispatch_counts(), vec![0]);
        assert_eq!((driver.last_batch, driver.completed_batches), (0, 0));
        assert!(driver.poisoned);
        assert!(driver.execute(&batch).is_err());
        assert!(driver.runtime_diagnostic_snapshot().is_err());
        assert!(driver.configure_scalar_v3_full_forward().is_err());
        // Late errors can have executed kernels; unchanged host metadata is not rollback.
        if matches!(
            failure,
            Failure::FullForwardWait | Failure::Read | Failure::BadChoice
        ) {
            assert!(driver.inner.transports[0].commands.len() >= 615);
            assert!(driver.inner.full_forward.is_some());
        }
        driver.close().unwrap();
        assert!(driver.inner.closed);
    }
}

#[test]
fn full_forward_recording_error_never_submits_a_partial_forward() {
    let mut pool = pool();
    let mut driver = configured(&pool);
    driver.configure_scalar_v3_full_forward().unwrap();
    driver.inner.ranks[0].partial.elements = 0;
    let cursor = driver.inner.collective;
    let hidden = driver.inner.hidden.clone();
    let batch = prepare(&mut pool, 1);
    pool.begin_submission(&batch).unwrap();
    assert!(driver.execute(&batch).is_err());
    assert!(driver.poisoned);
    assert!(driver.inner.transports[0].commands.is_empty());
    assert!(driver.inner.transports[0].events.borrow().is_empty());
    assert_eq!(driver.inner.collective, cursor);
    assert_eq!(driver.inner.hidden, hidden);
    assert_eq!(driver.dispatch_counts(), vec![0]);
    assert_eq!((driver.last_batch, driver.completed_batches), (0, 0));
    driver.close().unwrap();
}

#[test]
fn full_forward_recording_is_bounded_and_incomplete_plan_cannot_submit() {
    let mut driver = configured(&pool());
    driver.configure_scalar_v3_full_forward().unwrap();
    driver.inner.begin_full_forward(1).unwrap();
    let command = super::super::super::dispatch(ARGMAX, 1, Vec::new());
    for _ in 0..616 {
        driver
            .inner
            .record_full_forward_command(command.clone())
            .unwrap();
    }
    assert!(driver.inner.record_full_forward_command(command).is_err());
    // Packet count alone is insufficient: all 72 shadow reductions must be complete.
    assert!(driver.inner.finish_full_forward().is_err());
    assert!(driver.inner.transports[0].events.borrow().is_empty());
    assert_eq!(driver.dispatch_counts(), vec![0]);
    assert_eq!(driver.inner.collective.expected().epoch, 0);
    driver.close().unwrap();
}

#[test]
fn full_forward_requires_exact_fresh_profile_and_terminal_configuration() {
    for mutation in 0..22 {
        let pool = pool();
        let mut driver = configured(&pool);
        match mutation {
            0 => driver.projection_configured = false,
            1 => driver.projection.mode = EngineeringTpProjectionModeV3::Wave,
            2 => driver.wave_attention = true,
            3 => driver.prune_output_head = true,
            4 => driver.configure_head_precision_v7(false).unwrap(),
            5 => driver.configure_dispatch_sequences(true).unwrap(),
            6 => driver.inner.transports[0].full_forward_supported = false,
            7 => driver.inner.transports[0].argmax_peer = Some((0, 1, 0)),
            8 => driver.inner.large_kv = true,
            9 => driver.inner.draft_v10 = true,
            10 => driver.inner.reduction = ReductionWorkspace::default(),
            11 => driver.last_batch = 1,
            12 => driver.completed_batches = 1,
            13 => driver.poisoned = true,
            14 => driver.inner.closed = true,
            15 => driver.c1_wave_layers = true,
            16 => driver.context_tokens = 63,
            17 => driver.physical_pages = 5,
            18 => driver.table_stride = 5,
            19 => driver.inner.transports[0].rollover_supported = true,
            20 => driver.configure_scalar_v3_ordered_batches().unwrap(),
            21 => driver.inner.row_capacity = 32,
            _ => unreachable!(),
        }
        assert!(
            driver.configure_scalar_v3_full_forward().is_err(),
            "mutation {mutation}"
        );
        assert!(!driver.inner.full_forward_enabled);
    }
    assert!(
        configured(&wide_pool())
            .configure_scalar_v3_full_forward()
            .is_err()
    );
    assert!(
        fixture(2, &pool())
            .configure_scalar_v3_full_forward()
            .is_err()
    );
    let mut driver = configured(&pool());
    driver.configure_scalar_v3_full_forward().unwrap();
    assert!(driver.configure_scalar_v3_full_forward().is_err());
    assert!(driver.configure_scalar_v3_ordered_batches().is_err());
    assert!(
        driver
            .configure_host_timing(crate::host_timing::HostTiming::default())
            .is_err()
    );
    for enabled in [false, true] {
        assert!(driver.configure_ordered_batches(enabled).is_err());
        assert!(driver.configure_dispatch_sequences(enabled).is_err());
        assert!(driver.configure_wave_attention(enabled).is_err());
        assert!(driver.configure_output_head_pruning(enabled).is_err());
        assert!(driver.configure_head_precision_v7(enabled).is_err());
        assert!(driver.configure_head_precision_v8(enabled).is_err());
    }
    assert!(
        driver
            .configure_reduction(EngineeringTpReductionModeV3::DeviceTp1V3)
            .is_err()
    );
}

#[test]
fn full_forward_rejects_multiple_rows_and_exhausted_budget_before_submission() {
    for multiple in [false, true] {
        let mut pool = pool();
        let mut driver = configured(&pool);
        driver.configure_scalar_v3_full_forward().unwrap();
        if !multiple {
            driver.completed_batches = 212;
        }
        let batch = prepare(&mut pool, if multiple { 2 } else { 1 });
        pool.begin_submission(&batch).unwrap();
        assert!(driver.execute(&batch).is_err());
        assert!(driver.inner.transports[0].packet_preparations.is_empty());
        assert!(driver.inner.transports[0].commands.is_empty());
        assert!(driver.inner.full_forward.is_none());
        assert!(!driver.poisoned);
        driver.close().unwrap();
    }
}

#[test]
fn full_forward_rejects_speculative_entry_before_any_transport_operation() {
    use crate::tp_paged::speculative::EngineeringTpSpeculativeKvV1;
    use crate::tp_paged::speculative::tests::index;

    let mut target_pool = pool();
    let mut driver = configured(&target_pool);
    driver.configure_scalar_v3_full_forward().unwrap();
    let target_sequence = target_pool
        .open_sequence(target_pool.scope(), &[77], 0)
        .unwrap()
        .sequence();
    let mut draft_pool = EngineeringTpPagedPoolV1::new(
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
    let round = index(0, 1, 4, 77);
    let mut owner = EngineeringTpSpeculativeKvV1::new(
        target_pool,
        target_sequence,
        draft_pool,
        draft_sequence,
        &round,
    )
    .unwrap();
    owner.reserve_round(&round).unwrap();
    let result = driver.execute_speculative_target(owner.target_work().unwrap());
    assert_eq!(
        result.err().unwrap(),
        "full-forward execution is target-only, not speculative verification"
    );
    assert!(driver.inner.transports[0].packet_preparations.is_empty());
    assert!(driver.inner.transports[0].commands.is_empty());
    assert!(driver.inner.transports[0].events.borrow().is_empty());
    assert_eq!((driver.last_batch, driver.completed_batches), (0, 0));
    owner.fail_submitted().unwrap();
    driver.close().unwrap();
}
