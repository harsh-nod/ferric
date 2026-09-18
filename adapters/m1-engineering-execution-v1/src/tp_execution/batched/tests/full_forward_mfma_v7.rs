//! Exact command/state fixtures; not GPU numerical or performance evidence.

use super::super::super::{EngineeringTpProjectionModeV3, ReductionWorkspace};
use super::*;

fn configured(pool: &EngineeringTpPagedPoolV1) -> EngineeringTpBatchExecutionV2<Recording> {
    let mut driver = fixture(1, pool);
    driver
        .configure_reduction(EngineeringTpReductionModeV3::DeviceTp1V3)
        .unwrap();
    // The one-element transposed placeholder is only a recording-fixture alias.
    // Production setup authenticates, transposes and retains each complete tensor.
    let original = driver.inner.ranks[0].global(Qwen3TensorKind::LanguageModelHead);
    let transposed = allocate_tensor(&mut driver.inner.transports[0], 1, 2).unwrap();
    driver.projection =
        super::super::super::projection::ProjectionPolicy::synthetic_mfma_for_recording(
            original.id,
            transposed,
        );
    driver.projection_configured = true;
    driver.configure_head_precision_v7(true).unwrap();
    driver
}

#[test]
fn mfma_full_forward_matches_serial_commands_io_and_fp32_head_bindings() {
    for selected in [Vec::new(), vec![0]] {
        let mut recordings = Vec::new();
        for full in [false, true] {
            let mut pool = pool();
            let mut driver = configured(&pool);
            if full {
                driver.configure_mfma_v7_full_forward().unwrap();
            }
            assert_eq!(driver.fp32_head_workspace_bytes(), 9_723_904);
            let mut choices = Vec::new();
            for epoch in 1..=2 {
                let batch = prepare(&mut pool, 1);
                pool.begin_submission(&batch).unwrap();
                let result = driver.execute_selected(&batch, &selected).unwrap();
                choices.push(result.choices.clone());
                assert_eq!(driver.dispatch_counts(), vec![616 * epoch]);
                assert_eq!(driver.inner.collective.expected().epoch, epoch);
                assert_eq!(driver.inner.collective.expected().layer, 0);
                assert!(driver.inner.full_forward.is_none());
                pool.commit_batch(&batch, result.completion).unwrap();
            }
            let transport = &driver.inner.transports[0];
            assert_eq!(transport.packet_preparations, vec![616, 616]);
            assert_eq!(transport.commands.len(), 1232);
            assert_eq!(transport.reads.len(), 2);
            for commands in transport.commands.chunks_exact(616) {
                assert_eq!(commands[0].kernel, EMBEDDING);
                assert_eq!(commands[613].kernel, RMSNORM);
                assert_eq!(commands[614].kernel, FP32_MFMA_HEAD);
                assert_eq!(commands[615].kernel, FP32_ARGMAX);
                assert_eq!(buffer(&commands[614], 0).3, 2);
                assert_eq!(buffer(&commands[614], 1).3, 2);
                assert_eq!(buffer(&commands[614], 2), buffer(&commands[615], 0));
                assert_eq!(buffer(&commands[614], 2).2, 16 * 151_936);
                assert_eq!(buffer(&commands[614], 2).3, 4);
                assert_eq!(buffer(&commands[615], 1).3, 4);
                assert_eq!(
                    commands
                        .iter()
                        .filter(|c| c.kernel == "ferric_qwen3_tp_mfma_gemm_bf16_v3")
                        .count(),
                    180
                );
                assert_eq!(
                    commands
                        .iter()
                        .filter(|c| c.kernel == "ferric_qwen3_tp_mfma_gemm_partial_f32_v3")
                        .count(),
                    72
                );
                for layer in 0..36 {
                    assert_eq!(commands[9 + layer * 17].kernel, ATTENTION);
                    for residual in [11 + layer * 17, 17 + layer * 17] {
                        assert_eq!(
                            commands[residual].kernel,
                            "ferric_qwen3_tp_batch_residual_bf16_v3"
                        );
                        assert_eq!(
                            buffer(&commands[residual], 2).0,
                            buffer(&commands[residual + 1], 0).0
                        );
                    }
                }
            }
            let events = transport.events.borrow();
            assert_eq!(
                events
                    .iter()
                    .filter(|event| matches!(event, Event::FullForwardSubmit(0, 616)))
                    .count(),
                if full { 2 } else { 0 }
            );
            assert_eq!(
                events
                    .iter()
                    .filter(|event| matches!(event, Event::FullForwardWait(0, 616)))
                    .count(),
                if full { 2 } else { 0 }
            );
            assert!(!events.iter().any(|event| matches!(
                event,
                Event::OrderedSubmit(..) | Event::SequenceSubmit(..)
            )));
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
fn mfma_full_forward_failures_leave_live_state_uncommitted() {
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
        driver.configure_mfma_v7_full_forward().unwrap();
        let hidden = driver.inner.ranks[0].hidden;
        let ReductionWorkspace::DeviceTp1(scratch) = driver.inner.reduction else {
            panic!("device residual required");
        };
        let cursor = driver.inner.collective;
        let host = driver.inner.hidden.clone();
        let logits = driver.fp32_logits;
        driver.inner.transports[0].failure = Some(failure);
        let batch = prepare(&mut pool, 1);
        pool.begin_submission(&batch).unwrap();
        assert!(driver.execute(&batch).is_err(), "{failure:?}");
        assert_eq!(driver.inner.ranks[0].hidden, hidden);
        let ReductionWorkspace::DeviceTp1(after_scratch) = driver.inner.reduction else {
            panic!("device residual required");
        };
        assert_eq!(after_scratch, scratch);
        assert_eq!(driver.inner.collective, cursor);
        assert_eq!(driver.inner.hidden, host);
        assert_eq!(driver.fp32_logits, logits);
        assert_eq!(driver.dispatch_counts(), vec![0]);
        assert_eq!((driver.last_batch, driver.completed_batches), (0, 0));
        assert!(driver.poisoned);
        assert!(driver.execute(&batch).is_err());
        assert!(driver.runtime_diagnostic_snapshot().is_err());
        assert!(driver.configure_mfma_v7_full_forward().is_err());
        driver.close().unwrap();
    }
}

#[test]
fn mfma_full_forward_requires_exact_preconfigured_arithmetic_and_common_geometry() {
    for mutation in 0..28 {
        let mut driver = configured(&pool());
        match mutation {
            0 => driver.projection_configured = false,
            1 => driver.projection.mode = EngineeringTpProjectionModeV3::Baseline,
            2 => driver.projection.mode = EngineeringTpProjectionModeV3::Wave,
            3 => driver.projection.mode = EngineeringTpProjectionModeV3::Auto,
            4 => driver.wave_attention = true,
            5 => driver.prune_output_head = true,
            6 => driver.head_profile_configured = false,
            7 => driver.fp32_logits = None,
            8 => driver.fp32_logits.as_mut().unwrap().elements -= 1,
            9 => driver.fp32_logits.as_mut().unwrap().element_bytes = 2,
            10 => driver.inner.reduction = ReductionWorkspace::default(),
            11 => driver.inner.sequences = Some(vec![Vec::new()]),
            12 => driver.inner.ordered_batches = Some(Vec::new()),
            13 => driver.inner.transports[0].full_forward_supported = false,
            14 => driver.inner.transports[0].rollover_supported = true,
            15 => driver.inner.transports[0].argmax_peer = Some((0, 1, 0)),
            16 => driver.inner.draft_v10 = true,
            17 => driver.inner.large_kv = true,
            18 => driver.last_batch = 1,
            19 => driver.completed_batches = 1,
            20 => driver.poisoned = true,
            21 => driver.inner.closed = true,
            22 => driver.context_tokens = 63,
            23 => driver.physical_pages = 5,
            24 => driver.table_stride = 5,
            25 => driver.c1_wave_layers = true,
            26 => driver.row_capacity = 32,
            27 => driver.inner.row_capacity = 32,
            _ => unreachable!(),
        }
        assert!(
            driver.configure_mfma_v7_full_forward().is_err(),
            "mutation {mutation}"
        );
        assert!(!driver.inner.full_forward_enabled);
    }
}

#[test]
fn scalar_and_mfma_full_forward_profiles_remain_distinct_and_terminal() {
    let mut scalar = fixture(1, &pool());
    scalar
        .configure_reduction(EngineeringTpReductionModeV3::DeviceTp1V3)
        .unwrap();
    scalar.projection_configured = true;
    assert!(scalar.configure_mfma_v7_full_forward().is_err());
    scalar.configure_scalar_v3_full_forward().unwrap();
    assert!(scalar.configure_mfma_v7_full_forward().is_err());

    let mut mfma = configured(&pool());
    assert!(mfma.configure_scalar_v3_full_forward().is_err());
    mfma.configure_mfma_v7_full_forward().unwrap();
    assert!(mfma.configure_scalar_v3_full_forward().is_err());
    assert!(mfma.configure_mfma_v7_full_forward().is_err());
    assert!(mfma.configure_scalar_v3_ordered_batches().is_err());
    for enabled in [false, true] {
        assert!(mfma.configure_head_precision_v7(enabled).is_err());
        assert!(mfma.configure_head_precision_v8(enabled).is_err());
        assert!(mfma.configure_wave_attention(enabled).is_err());
        assert!(mfma.configure_output_head_pruning(enabled).is_err());
        assert!(mfma.configure_dispatch_sequences(enabled).is_err());
        assert!(mfma.configure_ordered_batches(enabled).is_err());
    }
    assert!(
        mfma.configure_reduction(EngineeringTpReductionModeV3::DeviceTp1V3)
            .is_err()
    );
    assert!(
        mfma.configure_host_timing(crate::host_timing::HostTiming::default())
            .is_err()
    );
}

#[test]
fn mfma_full_forward_preserves_single_row_and_no_rollover_budget() {
    for multiple in [false, true] {
        let mut pool = pool();
        let mut driver = configured(&pool);
        driver.configure_mfma_v7_full_forward().unwrap();
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
