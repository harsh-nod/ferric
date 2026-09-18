//! Exact command/state fixtures; not GPU numerical or performance evidence.

use super::super::super::{EngineeringTpProjectionModeV3, ReductionWorkspace};
use super::*;
use crate::tp_artifact::{Fp32ArgmaxBindingV11, QueryHoistBindingV14, WaveRmsNormBindingV15};

const WAVE: &str = "ferric_qwen3_tp_wave_paged_gqa_bf16_v3";

fn configured(
    pool: &EngineeringTpPagedPoolV1,
    wave: bool,
) -> EngineeringTpBatchExecutionV2<Recording> {
    let mut driver = fixture(1, pool);
    driver
        .configure_reduction(EngineeringTpReductionModeV3::DeviceTp1V3)
        .unwrap();
    // Synthetic aliases exercise command ownership, not authenticated weight intake.
    let original = driver.inner.ranks[0].global(Qwen3TensorKind::LanguageModelHead);
    let transposed = allocate_tensor(&mut driver.inner.transports[0], 1, 2).unwrap();
    driver.projection =
        super::super::super::projection::ProjectionPolicy::synthetic_mfma_for_recording(
            original.id,
            transposed,
        );
    driver.projection_configured = true;
    driver.configure_wave_attention(wave).unwrap();
    driver.configure_head_precision_v7(true).unwrap();
    driver
}

#[test]
fn mfma_wave_full_forward_preserves_two_forwards_and_only_substitutes_attention() {
    for selected in [Vec::new(), vec![0]] {
        let mut recordings = Vec::new();
        let mut original_commands = Vec::new();
        for (wave, full) in [(false, true), (true, false), (true, true)] {
            let mut pool = pool();
            let mut driver = configured(&pool, wave);
            if full {
                if wave {
                    driver.configure_mfma_v7_wave_full_forward().unwrap();
                } else {
                    driver.configure_mfma_v7_full_forward().unwrap();
                }
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
                assert_eq!(driver.inner.hidden.len(), 4096);
                assert!(driver.inner.full_forward.is_none());
                pool.commit_batch(&batch, result.completion).unwrap();
            }
            let transport = &driver.inner.transports[0];
            assert_eq!(transport.packet_preparations, vec![616, 616]);
            assert_eq!(transport.commands.len(), 1232);
            assert_eq!(transport.reads.len(), 2);
            original_commands.push(transport.commands.clone());
            let mut commands = transport.commands.clone();
            for forward in commands.chunks_exact_mut(616) {
                assert_eq!(forward[0].kernel, EMBEDDING);
                assert_eq!(forward[613].kernel, RMSNORM);
                assert_eq!(forward[614].kernel, FP32_MFMA_HEAD);
                assert_eq!(forward[615].kernel, FP32_ARGMAX);
                assert_eq!(buffer(&forward[614], 0).3, 2);
                assert_eq!(buffer(&forward[614], 1).3, 2);
                assert_eq!(buffer(&forward[614], 2), buffer(&forward[615], 0));
                assert_eq!(buffer(&forward[614], 2).2, 16 * 151_936);
                assert_eq!(buffer(&forward[614], 2).3, 4);
                assert_eq!(buffer(&forward[615], 1).3, 4);
                assert_eq!(
                    forward
                        .iter()
                        .filter(|command| command.kernel == WAVE)
                        .count(),
                    if wave { 36 } else { 0 }
                );
                assert_eq!(
                    forward
                        .iter()
                        .filter(|command| command.kernel == ATTENTION)
                        .count(),
                    if wave { 0 } else { 36 }
                );
                for layer in 0..36 {
                    let attention = &mut forward[9 + layer * 17];
                    assert_eq!(attention.kernel, if wave { WAVE } else { ATTENTION });
                    assert_eq!(attention.grid_workgroups, 32);
                    assert_eq!(attention.workgroup_size, 64);
                    assert_eq!(attention.arguments.len(), 11);
                    attention.kernel = ATTENTION;
                    for residual in [11 + layer * 17, 17 + layer * 17] {
                        assert_eq!(
                            forward[residual].kernel,
                            "ferric_qwen3_tp_batch_residual_bf16_v3"
                        );
                        assert_eq!(
                            buffer(&forward[residual], 2).0,
                            buffer(&forward[residual + 1], 0).0
                        );
                    }
                }
            }
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
            let serial_events = events
                .iter()
                .filter_map(|event| match event {
                    Event::FullForwardSubmit(..) | Event::FullForwardWait(..) => None,
                    Event::Submit(rank, name) if *name == WAVE => {
                        Some(Event::Submit(*rank, ATTENTION))
                    }
                    Event::Wait(rank, name) if *name == WAVE => Some(Event::Wait(*rank, ATTENTION)),
                    other => Some(other.clone()),
                })
                .collect::<Vec<_>>();
            let ReductionWorkspace::DeviceTp1(scratch) = driver.inner.reduction else {
                panic!("device residual required");
            };
            recordings.push((
                commands,
                transport.reads.clone(),
                transport.writes.clone(),
                transport.write_payloads.clone(),
                transport.buffers.clone(),
                serial_events,
                choices,
                (
                    driver.inner.hidden.clone(),
                    driver.inner.ranks[0].hidden,
                    scratch,
                    driver.inner.collective,
                    driver.last_batch,
                    driver.completed_batches,
                    driver.fp32_logits,
                ),
            ));
            drop(events);
            driver.close().unwrap();
        }
        assert_eq!(original_commands[1], original_commands[2]);
        assert_eq!(recordings[0], recordings[1]);
        assert_eq!(recordings[1], recordings[2]);
    }
}

#[test]
fn mfma_wave_full_forward_failures_leave_live_state_uncommitted() {
    for failure in [
        Failure::FullForwardSubmit,
        Failure::FullForwardWait,
        Failure::AttentionSubmit,
        Failure::AttentionWait,
        Failure::ResidualSubmit,
        Failure::ResidualWait,
        Failure::Read,
        Failure::BadChoice,
        Failure::Write,
        Failure::PreparePackets,
    ] {
        let mut pool = pool();
        let mut driver = configured(&pool, true);
        driver.configure_mfma_v7_wave_full_forward().unwrap();
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
        assert!(driver.configure_mfma_v7_wave_full_forward().is_err());
        driver.close().unwrap();
        assert!(driver.inner.closed);
    }
}

#[test]
fn mfma_wave_full_forward_requires_exact_preconfigured_profile_and_geometry() {
    for mutation in 0..37 {
        let mut driver = configured(&pool(), true);
        match mutation {
            0 => driver.projection_configured = false,
            1 => driver.projection.mode = EngineeringTpProjectionModeV3::Baseline,
            2 => driver.projection.mode = EngineeringTpProjectionModeV3::Wave,
            3 => driver.projection.mode = EngineeringTpProjectionModeV3::Auto,
            4 => driver.wave_attention = false,
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
            28 => driver.inner.plan = Qwen3TensorParallelPlanV1::new(target(), 2).unwrap(),
            29 => driver.inner.timing = crate::host_timing::HostTiming::enabled(),
            30 => driver.fp32_argmax_v11 = Some(Fp32ArgmaxBindingV11::recording()),
            31 => driver.admitted_argmax_v11 = Some(Fp32ArgmaxBindingV11::recording()),
            32 => driver.query_hoist_v14 = Some(QueryHoistBindingV14::recording()),
            33 => driver.admitted_query_hoist_v14 = Some(QueryHoistBindingV14::recording()),
            34 => driver.wave_rmsnorm_v15 = Some(WaveRmsNormBindingV15::recording()),
            35 => driver.admitted_wave_rmsnorm_v15 = Some(WaveRmsNormBindingV15::recording()),
            36 => driver.inner.ranks.clear(),
            _ => unreachable!(),
        }
        assert!(
            driver.configure_mfma_v7_wave_full_forward().is_err(),
            "mutation {mutation}"
        );
        assert!(!driver.inner.full_forward_enabled);
        assert!(driver.inner.transports[0].packet_preparations.is_empty());
        assert!(driver.inner.transports[0].commands.is_empty());
    }
}

#[test]
fn mfma_wave_full_forward_selector_is_distinct_and_terminal() {
    let mut scalar = fixture(1, &pool());
    scalar
        .configure_reduction(EngineeringTpReductionModeV3::DeviceTp1V3)
        .unwrap();
    scalar.projection_configured = true;
    assert!(scalar.configure_mfma_v7_wave_full_forward().is_err());
    scalar.configure_scalar_v3_full_forward().unwrap();
    assert!(scalar.configure_mfma_v7_wave_full_forward().is_err());

    let mut baseline = configured(&pool(), false);
    assert!(baseline.configure_mfma_v7_wave_full_forward().is_err());
    baseline.configure_mfma_v7_full_forward().unwrap();
    assert!(baseline.configure_mfma_v7_wave_full_forward().is_err());

    let mut wave = configured(&pool(), true);
    assert!(wave.configure_scalar_v3_full_forward().is_err());
    assert!(wave.configure_mfma_v7_full_forward().is_err());
    wave.configure_mfma_v7_wave_full_forward().unwrap();
    assert!(wave.configure_scalar_v3_full_forward().is_err());
    assert!(wave.configure_mfma_v7_full_forward().is_err());
    assert!(wave.configure_mfma_v7_wave_full_forward().is_err());
    assert!(wave.configure_scalar_v3_ordered_batches().is_err());
    for enabled in [false, true] {
        assert!(wave.configure_head_precision_v7(enabled).is_err());
        assert!(wave.configure_head_precision_v8(enabled).is_err());
        assert!(wave.configure_wave_attention(enabled).is_err());
        assert!(wave.configure_output_head_pruning(enabled).is_err());
        assert!(wave.configure_dispatch_sequences(enabled).is_err());
        assert!(wave.configure_ordered_batches(enabled).is_err());
    }
    assert!(
        wave.configure_reduction(EngineeringTpReductionModeV3::DeviceTp1V3)
            .is_err()
    );
    assert!(
        wave.configure_host_timing(crate::host_timing::HostTiming::default())
            .is_err()
    );
}

#[test]
fn mfma_wave_full_forward_preserves_single_row_and_no_rollover_budget() {
    for multiple in [false, true] {
        let mut pool = pool();
        let mut driver = configured(&pool, true);
        driver.configure_mfma_v7_wave_full_forward().unwrap();
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
fn mfma_wave_full_forward_recording_error_never_submits_a_partial_forward() {
    let mut pool = pool();
    let mut driver = configured(&pool, true);
    driver.configure_mfma_v7_wave_full_forward().unwrap();
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
