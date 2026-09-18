//! Exact host command/IO equivalence, not GPU arithmetic or performance evidence.

use super::*;

fn configured(pool: &EngineeringTpPagedPoolV1) -> EngineeringTpBatchExecutionV2<Recording> {
    let mut driver = fixture(1, pool);
    driver
        .configure_reduction(EngineeringTpReductionModeV3::DeviceTp1V3)
        .unwrap();
    // The synthetic fixture has no authenticated dense weights to intake.
    // Its unchanged default projection is the explicitly selected baseline.
    driver.projection_configured = true;
    driver
}

#[test]
fn scalar_ordered_preserves_every_command_operand_and_host_byte() {
    for rows in [1, 3, 16] {
        for selected in [Vec::new(), vec![rows - 1], (0..rows).collect()] {
            let mut recordings = Vec::new();
            for ordered in [false, true] {
                let mut pool = pool();
                let mut driver = configured(&pool);
                if ordered {
                    driver.configure_scalar_v3_ordered_batches().unwrap();
                }
                let batch = prepare(&mut pool, rows);
                pool.begin_submission(&batch).unwrap();
                let selected = selected.iter().map(|&row| row as usize).collect::<Vec<_>>();
                let result = driver.execute_selected(&batch, &selected).unwrap();
                // No pruning: even a prefill batch with no published row runs its head.
                assert_eq!(driver.expected_dispatch_counts(selected.len()), vec![616]);
                assert_eq!(driver.dispatch_counts(), vec![616]);
                let transport = &driver.inner.transports[0];
                assert_eq!(transport.packet_preparations, vec![616]);
                let events = transport.events.borrow();
                let groups = events
                    .iter()
                    .filter_map(|event| match event {
                        Event::OrderedSubmit(0, count) => Some(*count),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                assert_eq!(
                    groups,
                    if ordered {
                        [11, 6].repeat(36)
                    } else {
                        Vec::new()
                    }
                );
                assert_eq!(transport.commands.len(), 616);
                assert_eq!(transport.commands[0].kernel, EMBEDDING);
                assert_eq!(transport.commands[613].kernel, RMSNORM);
                assert_eq!(transport.commands[614].kernel, GEMM);
                assert_eq!(transport.commands[615].kernel, ARGMAX);
                for layer in 0..36 {
                    let first = 1 + layer * 17;
                    assert_eq!(transport.commands[first + 8].kernel, ATTENTION);
                    for residual in [first + 10, first + 16] {
                        let command = &transport.commands[residual];
                        assert_eq!(command.kernel, "ferric_qwen3_tp_batch_residual_bf16_v3");
                        assert_eq!(scalar(command, 3), rows);
                        // The next norm consumes the destination only after group completion.
                        assert_eq!(
                            buffer(command, 2).0,
                            buffer(&transport.commands[residual + 1], 0).0
                        );
                    }
                }
                assert!(transport.pending_ordered.is_none());
                assert!(
                    driver
                        .inner
                        .ordered_batches
                        .as_ref()
                        .is_none_or(Vec::is_empty)
                );
                let serial_events = events
                    .iter()
                    .filter(|event| {
                        !matches!(event, Event::OrderedSubmit(..) | Event::OrderedWait(..))
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                recordings.push((
                    transport.commands.clone(),
                    transport.reads.clone(),
                    transport.writes.clone(),
                    transport.write_payloads.clone(),
                    transport.buffers.clone(),
                    serial_events,
                    result.choices.clone(),
                ));
                drop(events);
                pool.commit_batch(&batch, result.completion).unwrap();
                assert!(driver.configure_scalar_v3_ordered_batches().is_err());
                driver.close().unwrap();
            }
            assert_eq!(recordings[0], recordings[1]);
        }
    }
}

#[test]
fn scalar_ordered_failure_never_swaps_or_advances_the_failed_group() {
    for ordinal in [0, 1, 36, 71] {
        for failure in [
            Failure::OrderedSubmitAt(ordinal),
            Failure::OrderedWaitAt(ordinal),
        ] {
            let mut pool = pool();
            let mut driver = configured(&pool);
            driver.configure_scalar_v3_ordered_batches().unwrap();
            let initial_hidden = driver.inner.ranks[0].hidden.id;
            let super::super::super::ReductionWorkspace::DeviceTp1(initial_scratch) =
                driver.inner.reduction
            else {
                panic!("device residual workspace required");
            };
            driver.inner.transports[0].failure = Some(failure);
            let batch = prepare(&mut pool, 3);
            pool.begin_submission(&batch).unwrap();
            assert!(driver.execute(&batch).is_err());
            assert_eq!(driver.completed_batches, 0);
            assert_eq!(
                driver.dispatch_counts(),
                vec![1 + [11, 6].into_iter().cycle().take(ordinal).sum::<u64>()]
            );
            assert_eq!(
                driver.inner.ranks[0].hidden.id,
                if ordinal % 2 == 0 {
                    initial_hidden
                } else {
                    initial_scratch.id
                }
            );
            let cursor = driver.inner.collective.expected();
            assert_eq!(cursor.epoch, 0);
            assert_eq!(cursor.layer as usize, ordinal / 2);
            assert_eq!(
                cursor.operation,
                if ordinal % 2 == 0 {
                    Qwen3TensorParallelCollectiveV1::AttentionOutputSum
                } else {
                    Qwen3TensorParallelCollectiveV1::FeedForwardDownSum
                }
            );
            assert!(driver.poisoned);
            assert!(driver.execute(&batch).is_err());
            assert!(driver.runtime_diagnostic_snapshot().is_err());
            assert!(driver.configure_scalar_v3_ordered_batches().is_err());
            assert!(driver.inner.ordered_batches.as_ref().unwrap().is_empty());
            assert!(driver.inner.transports[0].pending_ordered.is_none());
            driver.close().unwrap();
            assert!(driver.inner.closed);
        }
    }
}

#[test]
fn scalar_ordered_requires_its_exact_fresh_profile() {
    for mutation in 0..16 {
        let pool = pool();
        let mut driver = configured(&pool);
        match mutation {
            0 => driver.projection_configured = false,
            1 => driver.projection.mode = super::super::super::EngineeringTpProjectionModeV3::Wave,
            2 => driver.wave_attention = true,
            3 => driver.prune_output_head = true,
            4 => driver.configure_head_precision_v7(false).unwrap(),
            5 => driver.configure_dispatch_sequences(true).unwrap(),
            6 => driver.inner.transports[0].ordered_supported = false,
            7 => driver.inner.transports[0].argmax_peer = Some((0, 1, 0)),
            8 => driver.inner.large_kv = true,
            9 => driver.inner.draft_v10 = true,
            10 => driver.inner.reduction = super::super::super::ReductionWorkspace::default(),
            11 => driver.last_batch = 1,
            12 => driver.completed_batches = 1,
            13 => driver.poisoned = true,
            14 => driver.inner.closed = true,
            15 => driver.c1_wave_layers = true,
            _ => unreachable!(),
        }
        assert!(
            driver.configure_scalar_v3_ordered_batches().is_err(),
            "mutation {mutation}"
        );
        assert!(driver.inner.ordered_batches.is_none());
    }
    let mut wide = configured(&wide_pool());
    assert!(wide.configure_scalar_v3_ordered_batches().is_err());
    let mut peers = fixture(2, &pool());
    assert!(peers.configure_scalar_v3_ordered_batches().is_err());
}

#[test]
fn scalar_ordered_configuration_is_terminal_and_does_not_widen_legacy_admission() {
    let pool = pool();
    let mut driver = configured(&pool);
    assert!(driver.configure_ordered_batches(true).is_err());
    driver.configure_scalar_v3_ordered_batches().unwrap();
    assert!(driver.configure_scalar_v3_ordered_batches().is_err());
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
    driver.close().unwrap();
    assert!(driver.configure_scalar_v3_ordered_batches().is_err());
}
