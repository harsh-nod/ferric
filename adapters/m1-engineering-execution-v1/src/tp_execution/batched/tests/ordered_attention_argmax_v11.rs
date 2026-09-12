//! Ordered wave/v11 host contracts only; no kernel emulation or native evidence.

use super::*;
use crate::tp_artifact::{ENGINEERING_TP_FP32_ARGMAX32_EXPORTS_V11, Fp32ArgmaxBindingV11};

const WAVE_GQA: &str = "ferric_qwen3_tp_batch32_wave_paged_gqa_bf16_v5";
const RESIDUAL: &str = "ferric_qwen3_tp_batch32_residual_bf16_v5";
const HEAD: &str = "ferric_qwen3_tp_batch32_mfma_head_f32_v8";
const WAVE_ARGMAX: &str = ENGINEERING_TP_FP32_ARGMAX32_EXPORTS_V11[0];

fn configured(pool: &EngineeringTpPagedPoolV1) -> EngineeringTpBatchExecutionV2<Recording> {
    let mut driver = fixture(1, pool);
    driver
        .configure_reduction(EngineeringTpReductionModeV3::DeviceTp1V3)
        .unwrap();
    driver.configure_output_head_pruning(true).unwrap();
    let original = driver.inner.ranks[0].global(Qwen3TensorKind::LanguageModelHead);
    let transposed = allocate_tensor(&mut driver.inner.transports[0], 1, 2).unwrap();
    driver.projection =
        super::super::super::projection::ProjectionPolicy::synthetic_mfma_for_recording(
            original.id,
            transposed,
        );
    driver.configure_wave_attention(true).unwrap();
    driver.configure_head_precision_v8(true).unwrap();
    // Constructors and image admission have separate tests; this fixture represents that binding.
    driver.admitted_argmax_v11 = Some(Fp32ArgmaxBindingV11::recording());
    driver
}

fn select(driver: &mut EngineeringTpBatchExecutionV2<Recording>) -> TpResult<()> {
    driver
        .configure_ordered_wave_attention_fp32_argmax_binding_v11(Fp32ArgmaxBindingV11::recording())
}

fn allocations(driver: &EngineeringTpBatchExecutionV2<Recording>) -> Vec<(u64, usize)> {
    driver.inner.transports[0]
        .buffers
        .iter()
        .map(|(&id, bytes)| (id, bytes.len()))
        .collect()
}

fn event_pair(events: &[Event], cursor: &mut usize, command: &EngineeringTpDispatchV1) {
    assert_eq!(events[*cursor], Event::Submit(0, command.kernel));
    assert_eq!(events[*cursor + 1], Event::Wait(0, command.kernel));
    *cursor += 2;
}

fn ordered_barriers(transport: &Recording, published: bool) {
    let events = transport.events.borrow();
    let commands = &transport.commands;
    let mut cursor = 0;
    assert_eq!(
        commands[0].kernel,
        "ferric_qwen3_tp_batch32_embedding_bf16_v5"
    );
    event_pair(&events, &mut cursor, &commands[0]);
    let mut index = 1;
    for _ in 0..36 {
        for count in [10, 5] {
            assert_eq!(events[cursor], Event::OrderedSubmit(0, count));
            assert_eq!(events[cursor + 1], Event::OrderedWait(0, count));
            cursor += 2;
            let group = &commands[index..index + count];
            assert_eq!(
                group
                    .iter()
                    .filter(|command| command.kernel == WAVE_GQA)
                    .count(),
                usize::from(count == 10),
            );
            assert!(
                group
                    .iter()
                    .all(|command| !matches!(command.kernel, RESIDUAL | HEAD | WAVE_ARGMAX))
            );
            for command in group {
                event_pair(&events, &mut cursor, command);
            }
            index += count;
            assert_eq!(commands[index].kernel, RESIDUAL);
            event_pair(&events, &mut cursor, &commands[index]);
            index += 1;
        }
    }
    if published {
        for kernel in [RMSNORM, HEAD, WAVE_ARGMAX] {
            assert_eq!(commands[index].kernel, kernel);
            event_pair(&events, &mut cursor, &commands[index]);
            index += 1;
        }
    }
    assert_eq!(index, commands.len());
    assert_eq!(cursor, events.len());
}

#[test]
fn ordered_wave_v11_preserves_flattened_commands_io_and_head_barriers() {
    for rows in [1, 16, 17, 32] {
        for selected in [
            Vec::new(),
            vec![rows as usize - 1],
            if rows > 1 {
                vec![0, rows as usize - 1]
            } else {
                vec![0]
            },
            (0..rows as usize).collect(),
        ] {
            let mut recordings = Vec::new();
            for ordered in [false, true] {
                let mut pool = wide_pool();
                let mut driver = configured(&pool);
                let initial_allocations = allocations(&driver);
                if ordered {
                    select(&mut driver).unwrap();
                } else {
                    driver
                        .configure_wave_attention_fp32_argmax_binding_v11(
                            Fp32ArgmaxBindingV11::recording(),
                        )
                        .unwrap();
                }
                assert_eq!(allocations(&driver), initial_allocations);
                let batch = prepare(&mut pool, rows);
                pool.begin_submission(&batch).unwrap();
                let output = driver.execute_selected(&batch, &selected).unwrap();
                assert_eq!(output.choices.len(), selected.len());
                assert_eq!(driver.completed_batches(), 1);
                assert_eq!(
                    driver.dispatch_counts(),
                    [if selected.is_empty() { 613 } else { 616 }]
                );
                assert_eq!(allocations(&driver), initial_allocations);
                let transport = &driver.inner.transports[0];
                assert!(transport.pending.is_none());
                assert!(transport.pending_ordered.is_none());
                assert!(transport.pending_sequence.is_none());
                assert!(
                    driver
                        .inner
                        .ordered_batches
                        .as_ref()
                        .is_none_or(Vec::is_empty)
                );
                if ordered {
                    ordered_barriers(transport, !selected.is_empty());
                } else {
                    assert!(!transport.events.borrow().iter().any(|event| matches!(
                        event,
                        Event::OrderedSubmit(..) | Event::OrderedWait(..)
                    )));
                }
                let expected_reads = if selected.is_empty() {
                    Vec::new()
                } else {
                    vec![(driver.inner.ranks[0].choice.id, selected.len() * 4)]
                };
                assert_eq!(transport.reads, expected_reads);
                recordings.push((
                    transport.commands.clone(),
                    transport.reads.clone(),
                    transport.writes.clone(),
                    transport.write_payloads.clone(),
                    transport.packet_preparations.clone(),
                    initial_allocations,
                    output.choices.clone(),
                ));
                pool.commit_batch(&batch, output.completion).unwrap();
                pool.check_invariants().unwrap();
                let sequence = batch.rows()[0].sequence();
                pool.retire_sequence(sequence, false, 1).unwrap();
                pool.check_invariants().unwrap();
                assert_eq!(pool.stats().free_pages, 4);
                assert_eq!(pool.stats().quarantined_pages, 0);
                assert_eq!(pool.stats().cached_pages, 0);
                driver.close().unwrap();
            }
            assert_eq!(recordings[0], recordings[1]);
        }
    }
}

#[test]
fn ordered_wave_v11_rejection_is_atomic_for_binding_capability_and_profile() {
    for mutation in 0..25 {
        let mut driver = configured(&wide_pool());
        match mutation {
            0 => driver.admitted_argmax_v11 = None,
            1 => {
                let mut other = Fp32ArgmaxBindingV11::recording();
                other.hsaco[0] ^= 1;
                driver.admitted_argmax_v11 = Some(other);
            }
            2 => driver.row_capacity = 16,
            3 => driver.inner.draft_v10 = true,
            4 => driver.fp32_logits = None,
            5 => driver.head_profile_configured = false,
            6 => driver.inner.large_kv = true,
            7 => driver.inner.sequences = Some(vec![Vec::new()]),
            8 => driver.inner.ordered_batches = Some(Vec::new()),
            9 => driver.wave_attention = false,
            10 => driver.prune_output_head = false,
            11 => {
                driver.projection.mode =
                    super::super::super::EngineeringTpProjectionModeV3::Baseline;
            }
            12 => {
                driver.projection.mode = super::super::super::EngineeringTpProjectionModeV3::Wave;
            }
            13 => {
                driver.projection.mode = super::super::super::EngineeringTpProjectionModeV3::Auto;
            }
            14 => driver.last_batch = 1,
            15 => driver.completed_batches = 1,
            16 => driver.poisoned = true,
            17 => driver.inner.closed = true,
            18 => driver.inner.transports[0].argmax_peer = Some((1, 0, 1)),
            19 => driver.inner.transports.clear(),
            20 => driver.inner.ranks.clear(),
            21 => {
                driver.inner.reduction =
                    super::super::super::reduction::ReductionWorkspace::Baseline;
            }
            22 => driver.inner.transports[0].ordered_supported = false,
            23 => {
                driver
                    .inner
                    .ranks
                    .push(fixture(1, &wide_pool()).inner.ranks.pop().unwrap());
            }
            24 => {
                driver
                    .inner
                    .transports
                    .push(fixture(1, &wide_pool()).inner.transports.pop().unwrap());
            }
            _ => unreachable!(),
        }
        let before = (driver.fp32_argmax_v11, driver.inner.ordered_batches.clone());
        let io_before = driver
            .inner
            .transports
            .iter()
            .map(|transport| {
                (
                    transport.buffers.len(),
                    transport.commands.clone(),
                    transport.reads.clone(),
                    transport.writes.clone(),
                    transport.events.borrow().clone(),
                )
            })
            .collect::<Vec<_>>();
        assert!(select(&mut driver).is_err(), "mutation {mutation}");
        assert_eq!(
            (driver.fp32_argmax_v11, driver.inner.ordered_batches.clone()),
            before
        );
        let io_after = driver
            .inner
            .transports
            .iter()
            .map(|transport| {
                (
                    transport.buffers.len(),
                    transport.commands.clone(),
                    transport.reads.clone(),
                    transport.writes.clone(),
                    transport.events.borrow().clone(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(io_before, io_after);
    }
}

#[test]
fn ordered_wave_v11_keeps_existing_selectors_closed_and_freezes_policies() {
    let mut synchronous = configured(&wide_pool());
    synchronous
        .configure_wave_attention_fp32_argmax_binding_v11(Fp32ArgmaxBindingV11::recording())
        .unwrap();
    assert!(synchronous.configure_ordered_batches(true).is_err());
    assert!(select(&mut synchronous).is_err());
    assert!(synchronous.inner.ordered_batches.is_none());
    let mut legacy_ordered = configured(&wide_pool());
    legacy_ordered.configure_ordered_batches(true).unwrap();
    assert!(
        legacy_ordered
            .configure_wave_attention_fp32_argmax_binding_v11(Fp32ArgmaxBindingV11::recording())
            .is_err()
    );
    assert!(select(&mut legacy_ordered).is_err());
    assert!(legacy_ordered.fp32_argmax_v11.is_none());

    let mut driver = configured(&wide_pool());
    select(&mut driver).unwrap();
    assert_eq!(driver.fp32_argmax_mode(), "wave-v11");
    assert_eq!(driver.inner.ordered_batches.as_ref().unwrap().len(), 0);
    assert_eq!(driver.dispatch_counts(), [0]);
    assert!(select(&mut driver).is_err());
    assert!(
        driver
            .configure_fp32_argmax_binding_v11(Fp32ArgmaxBindingV11::recording())
            .is_err()
    );
    assert!(
        driver
            .configure_wave_attention_fp32_argmax_binding_v11(Fp32ArgmaxBindingV11::recording())
            .is_err()
    );
    for enabled in [false, true] {
        assert!(driver.configure_wave_attention(enabled).is_err());
        assert!(driver.configure_output_head_pruning(enabled).is_err());
        assert!(driver.configure_head_precision_v8(enabled).is_err());
        assert!(driver.configure_dispatch_sequences(enabled).is_err());
        assert!(driver.configure_ordered_batches(enabled).is_err());
    }
    assert!(
        driver
            .configure_reduction(EngineeringTpReductionModeV3::DeviceTp1V3)
            .is_err()
    );
    assert_eq!(
        driver.fp32_argmax_v11,
        Some(Fp32ArgmaxBindingV11::recording())
    );
    assert!(driver.inner.ordered_batches.as_ref().unwrap().is_empty());
}

#[test]
fn ordered_wave_v11_failures_never_complete_and_quarantine_the_submitted_pool() {
    for failure in [
        Failure::OrderedSubmit,
        Failure::OrderedWait,
        Failure::AttentionSubmit,
        Failure::AttentionWait,
        Failure::ResidualWait,
        Failure::ArgmaxSubmit,
        Failure::ArgmaxWait,
        Failure::BadChoice,
        Failure::PreparePackets,
    ] {
        let mut pool = wide_pool();
        let mut driver = configured(&pool);
        select(&mut driver).unwrap();
        driver.inner.transports[0].failure = Some(failure);
        let batch = prepare(&mut pool, 17);
        pool.begin_submission(&batch).unwrap();
        assert!(driver.execute_selected(&batch, &[16]).is_err());
        assert_eq!(driver.completed_batches(), 0);
        assert!(driver.poisoned);
        if matches!(
            failure,
            Failure::OrderedSubmit
                | Failure::OrderedWait
                | Failure::AttentionSubmit
                | Failure::AttentionWait
        ) {
            assert_eq!(driver.dispatch_counts(), [1]);
        }
        assert!(driver.inner.ordered_batches.as_ref().unwrap().is_empty());
        assert!(driver.inner.transports[0].pending_ordered.is_none());
        assert!(driver.inner.transports[0].pending.is_none());
        assert!(driver.execute_selected(&batch, &[16]).is_err());
        assert!(driver.runtime_diagnostic_snapshot().is_err());
        pool.quarantine_batch(&batch).unwrap();
        assert_eq!(pool.stats().free_pages, 0);
        assert_eq!(pool.stats().quarantined_pages, 4);
        assert_eq!(pool.stats().cached_pages, 0);
        assert_eq!(
            pool.check_invariants(),
            Err(crate::tp_paged::EngineeringTpPagedErrorV1::Poisoned)
        );
        assert!(
            pool.retire_sequence(batch.rows()[0].sequence(), false, 1)
                .is_err()
        );
        driver.close().unwrap();
        assert!(driver.inner.closed);
    }
}

#[test]
fn ordered_wave_v11_rejects_bad_rows_before_publication() {
    for selected in [vec![17], vec![0, 0], vec![1, 0]] {
        let mut pool = wide_pool();
        let mut driver = configured(&pool);
        select(&mut driver).unwrap();
        let batch = prepare(&mut pool, 17);
        assert!(driver.execute_selected(&batch, &selected).is_err());
        assert_eq!(driver.completed_batches(), 0);
        assert_eq!(driver.dispatch_counts(), [0]);
        assert!(!driver.poisoned);
        assert!(driver.inner.transports[0].packet_preparations.is_empty());
        assert!(driver.inner.transports[0].commands.is_empty());
        assert!(driver.inner.ordered_batches.as_ref().unwrap().is_empty());
        driver.close().unwrap();
    }
}

#[test]
fn ordered_wave_v11_host_spans_preserve_commands_and_name_the_flush_boundary() {
    let mut traces = Vec::new();
    for enabled in [false, true] {
        let mut pool = wide_pool();
        let mut driver = configured(&pool);
        select(&mut driver).unwrap();
        let timing = if enabled {
            crate::host_timing::HostTiming::enabled()
        } else {
            crate::host_timing::HostTiming::default()
        };
        driver.configure_host_timing(timing.clone()).unwrap();
        let batch = prepare(&mut pool, 1);
        pool.begin_submission(&batch).unwrap();
        let output = driver.execute_selected(&batch, &[0]).unwrap();
        pool.commit_batch(&batch, output.completion).unwrap();
        traces.push(driver.inner.transports[0].commands.clone());
        if enabled {
            let snapshot = timing.snapshot();
            assert_eq!(snapshot["incomplete"], false);
            assert_eq!(snapshot["active_records"], 0);
            assert_attention_operation_spans(&snapshot, batch.id(), 36);
            let flushes = snapshot["records"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| row["label"] == "flush_ordered_batches")
                .collect::<Vec<_>>();
            assert_eq!(flushes.len(), 2);
            for (phase, count) in [
                ("collective_attention", 36),
                ("collective_feed_forward", 36),
            ] {
                let row = flushes.iter().find(|row| row["phase"] == phase).unwrap();
                assert_eq!(row["category"], "span");
                assert_eq!(row["count"], count);
            }
        }
        driver.close().unwrap();
    }
    assert_eq!(traces[0], traces[1]);
}
