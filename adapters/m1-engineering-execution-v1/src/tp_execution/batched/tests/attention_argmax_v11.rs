//! Recording contracts for the separate wave-attention/v11 experiment, not GPU evidence.

use super::*;
use crate::tp_artifact::{ENGINEERING_TP_FP32_ARGMAX32_EXPORTS_V11, Fp32ArgmaxBindingV11};

const BASELINE_GQA: &str = "ferric_qwen3_tp_batch32_paged_gqa_bf16_f32_v5";
const WAVE_GQA: &str = "ferric_qwen3_tp_batch32_wave_paged_gqa_bf16_v5";

fn configured(
    pool: &EngineeringTpPagedPoolV1,
    wave: bool,
) -> EngineeringTpBatchExecutionV2<Recording> {
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
    driver.configure_wave_attention(wave).unwrap();
    driver.configure_head_precision_v8(true).unwrap();
    // The synthetic fixture bypasses constructors; admission itself is tested separately.
    driver.admitted_argmax_v11 = Some(Fp32ArgmaxBindingV11::recording());
    driver
}

fn select(driver: &mut EngineeringTpBatchExecutionV2<Recording>, wave: bool) -> TpResult<()> {
    if wave {
        driver.configure_wave_attention_fp32_argmax_binding_v11(Fp32ArgmaxBindingV11::recording())
    } else {
        driver.configure_fp32_argmax_binding_v11(Fp32ArgmaxBindingV11::recording())
    }
}

fn allocations(driver: &EngineeringTpBatchExecutionV2<Recording>) -> Vec<(u64, usize)> {
    driver.inner.transports[0]
        .buffers
        .iter()
        .map(|(&id, bytes)| (id, bytes.len()))
        .collect()
}

#[test]
fn attention_v11_changes_only_gqa_root_for_every_row_and_output_selection() {
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
            for wave in [false, true] {
                let mut pool = wide_pool();
                let mut driver = configured(&pool, wave);
                let initial_allocations = allocations(&driver);
                select(&mut driver, wave).unwrap();
                assert_eq!(allocations(&driver), initial_allocations);
                let batch = prepare(&mut pool, rows);
                pool.begin_submission(&batch).unwrap();
                let result = driver.execute_selected(&batch, &selected).unwrap();
                assert_eq!(result.choices.len(), selected.len());
                let packets = if selected.is_empty() { 613 } else { 616 };
                assert_eq!(driver.dispatch_counts(), [packets]);
                assert_eq!(driver.completed_batches(), 1);
                assert_eq!(allocations(&driver), initial_allocations);
                let transport = &driver.inner.transports[0];
                let mut commands = transport.commands.clone();
                let attention = if wave { WAVE_GQA } else { BASELINE_GQA };
                let mut count = 0;
                for command in &mut commands {
                    if command.kernel == attention {
                        count += 1;
                        assert_eq!(command.arguments.len(), 11);
                        assert_eq!(command.grid_workgroups, rows * 32);
                        assert_eq!(command.workgroup_size, 64);
                        assert_eq!(scalar(command, 6), rows);
                        assert_eq!(scalar(command, 7), 1);
                        command.kernel = BASELINE_GQA;
                    }
                }
                assert_eq!(count, 36);
                if !selected.is_empty() {
                    assert_eq!(
                        commands.last().unwrap().kernel,
                        ENGINEERING_TP_FP32_ARGMAX32_EXPORTS_V11[0]
                    );
                }
                let mut events = transport.events.borrow().clone();
                for event in &mut events {
                    if let Event::Submit(_, kernel) | Event::Wait(_, kernel) = event
                        && *kernel == WAVE_GQA
                    {
                        *kernel = BASELINE_GQA;
                    }
                }
                recordings.push((
                    commands,
                    events,
                    transport.reads.clone(),
                    transport.writes.clone(),
                    transport.write_payloads.clone(),
                    transport.packet_preparations.clone(),
                    initial_allocations,
                    result.choices.clone(),
                ));
                pool.commit_batch(&batch, result.completion).unwrap();
                pool.check_invariants().unwrap();
                driver.close().unwrap();
            }
            assert_eq!(recordings[0], recordings[1]);
        }
    }
}

#[test]
fn attention_v11_rejects_wrong_bindings_and_unsupported_profiles_without_io() {
    for mutation in 0..22 {
        let mut driver = configured(&wide_pool(), true);
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
                    super::super::super::EngineeringTpProjectionModeV3::Baseline
            }
            12 => driver.projection.mode = super::super::super::EngineeringTpProjectionModeV3::Wave,
            13 => driver.projection.mode = super::super::super::EngineeringTpProjectionModeV3::Auto,
            14 => driver.last_batch = 1,
            15 => driver.completed_batches = 1,
            16 => driver.poisoned = true,
            17 => driver.inner.closed = true,
            18 => driver.inner.transports[0].argmax_peer = Some((1, 0, 1)),
            19 => driver.inner.transports.clear(),
            20 => driver.inner.ranks.clear(),
            21 => {
                driver.inner.reduction =
                    super::super::super::reduction::ReductionWorkspace::Baseline
            }
            _ => unreachable!(),
        }
        let before = driver
            .inner
            .transports
            .iter()
            .map(|rank| {
                (
                    rank.buffers.len(),
                    rank.reads.len(),
                    rank.writes.len(),
                    rank.commands.len(),
                )
            })
            .collect::<Vec<_>>();
        assert!(select(&mut driver, true).is_err(), "mutation {mutation}");
        assert!(driver.fp32_argmax_v11.is_none());
        let after = driver
            .inner
            .transports
            .iter()
            .map(|rank| {
                (
                    rank.buffers.len(),
                    rank.reads.len(),
                    rank.writes.len(),
                    rank.commands.len(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(before, after);
    }
}

#[test]
fn attention_v11_keeps_legacy_rejection_and_freezes_every_policy() {
    let mut driver = configured(&wide_pool(), true);
    assert!(select(&mut driver, false).is_err());
    assert!(driver.fp32_argmax_v11.is_none());
    select(&mut driver, true).unwrap();
    assert!(select(&mut driver, true).is_err());
    assert!(select(&mut driver, false).is_err());
    assert!(driver.configure_wave_attention(true).is_err());
    assert!(driver.configure_wave_attention(false).is_err());
    assert!(driver.configure_output_head_pruning(false).is_err());
    assert!(
        driver
            .configure_reduction(EngineeringTpReductionModeV3::DeviceTp1V3)
            .is_err()
    );
    assert!(driver.configure_head_precision_v8(true).is_err());
    assert!(driver.configure_dispatch_sequences(true).is_err());
    assert!(driver.configure_ordered_batches(true).is_err());
    assert_eq!(driver.dispatch_counts(), [0]);

    let mut baseline = configured(&wide_pool(), false);
    assert!(select(&mut baseline, true).is_err());
    select(&mut baseline, false).unwrap();
    assert!(baseline.configure_wave_attention(true).is_err());
}

#[test]
fn attention_v11_submitted_failures_poison_without_successful_completion() {
    for wave in [false, true] {
        for failure in [
            Failure::AttentionSubmit,
            Failure::AttentionWait,
            Failure::ArgmaxSubmit,
            Failure::ArgmaxWait,
            Failure::BadChoice,
        ] {
            let mut pool = wide_pool();
            let mut driver = configured(&pool, wave);
            select(&mut driver, wave).unwrap();
            driver.inner.transports[0].failure = Some(failure);
            let batch = prepare(&mut pool, 1);
            pool.begin_submission(&batch).unwrap();
            assert!(driver.execute_selected(&batch, &[0]).is_err());
            assert!(driver.poisoned);
            assert_eq!(driver.completed_batches(), 0);
            pool.quarantine_batch(&batch).unwrap();
            assert_eq!(pool.stats().free_pages, 0);
            assert_eq!(pool.stats().quarantined_pages, 4);
            assert_eq!(pool.stats().cached_pages, 0);
            assert_eq!(
                pool.check_invariants(),
                Err(crate::tp_paged::EngineeringTpPagedErrorV1::Poisoned),
            );
            assert!(driver.execute_selected(&batch, &[0]).is_err());
            driver.close().unwrap();
            assert!(driver.inner.closed);
        }
    }
}

#[test]
fn attention_v11_host_spans_do_not_change_commands() {
    let mut commands = Vec::new();
    for enabled in [false, true] {
        let mut pool = wide_pool();
        let mut driver = configured(&pool, true);
        select(&mut driver, true).unwrap();
        let timing = if enabled {
            crate::host_timing::HostTiming::enabled()
        } else {
            crate::host_timing::HostTiming::default()
        };
        driver.configure_host_timing(timing.clone()).unwrap();
        let batch = prepare(&mut pool, 1);
        pool.begin_submission(&batch).unwrap();
        let result = driver.execute_selected(&batch, &[0]).unwrap();
        pool.commit_batch(&batch, result.completion).unwrap();
        commands.push(driver.inner.transports[0].commands.clone());
        if enabled {
            let snapshot = timing.snapshot();
            assert_eq!(snapshot["incomplete"], false);
            assert_eq!(snapshot["active_records"], 0);
            assert_attention_operation_spans(&snapshot, batch.id(), 36);
        }
        driver.close().unwrap();
    }
    assert_eq!(commands[0], commands[1]);
}
