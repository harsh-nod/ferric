//! Recording contracts only: synthetic outputs are not numerical qualification.

use super::*;
use crate::tp_artifact::Fp32ArgmaxBindingV11;

const MFMA: &str = "ferric_qwen3_tp_batch32_mfma_gemm_bf16_v5";
const MFMA_PARTIAL: &str = "ferric_qwen3_tp_batch32_mfma_gemm_partial_f32_v5";
const WAVE: &str = "ferric_qwen3_tp_batch32_wave_gemv_bf16_v5";
const WAVE_PARTIAL: &str = "ferric_qwen3_tp_batch32_wave_gemv_partial_f32_v5";
const HEAD: &str = "ferric_qwen3_tp_batch32_mfma_head_f32_v8";
const ARGMAX_WAVE: &str = "ferric_qwen3_tp_batch32_wave_argmax_f32_v11";

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
    driver.projection_configured = true;
    driver.configure_wave_attention(true).unwrap();
    driver.configure_head_precision_v8(true).unwrap();
    driver.admitted_argmax_v11 = Some(Fp32ArgmaxBindingV11::recording());
    driver
}

fn select(driver: &mut EngineeringTpBatchExecutionV2<Recording>) -> TpResult<()> {
    driver
        .configure_ordered_c1_wave_layers_fp32_argmax_binding_v11(Fp32ArgmaxBindingV11::recording())
}

fn allocations(driver: &EngineeringTpBatchExecutionV2<Recording>) -> Vec<(u64, usize)> {
    driver.inner.transports[0]
        .buffers
        .iter()
        .map(|(&id, bytes)| (id, bytes.len()))
        .collect()
}

fn ordered_shape(transport: &Recording, published: bool) {
    let events = transport.events.borrow();
    let groups = events
        .iter()
        .filter_map(|event| match event {
            Event::OrderedSubmit(_, count) => Some(*count),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(groups, [11, 6].repeat(36));
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, Event::OrderedWait(_, _)))
            .count(),
        72
    );
    let mut cursor = 1;
    for count in groups {
        let group = &transport.commands[cursor..cursor + count];
        assert_eq!(
            group.last().unwrap().kernel,
            "ferric_qwen3_tp_batch32_residual_bf16_v5"
        );
        assert!(
            group
                .iter()
                .all(|command| !matches!(command.kernel, HEAD | ARGMAX_WAVE))
        );
        cursor += count;
    }
    if published {
        assert_eq!(transport.commands[cursor].kernel, RMSNORM);
        assert_eq!(transport.commands[cursor + 1].kernel, HEAD);
        assert_eq!(transport.commands[cursor + 2].kernel, ARGMAX_WAVE);
        cursor += 3;
    }
    assert_eq!(cursor, transport.commands.len());
}

#[test]
fn layer_c1_wave_all_rows_change_only_seven_layer_projections_and_keep_head_exact() {
    for rows in 1..=32 {
        let mut recordings = Vec::new();
        for enabled in [false, true] {
            let mut pool = wide_pool();
            let mut driver = configured(&pool);
            let before = allocations(&driver);
            assert_eq!(driver.layer_projection_mode(), "mfma");
            if enabled {
                select(&mut driver).unwrap();
            } else {
                driver
                    .configure_ordered_wave_attention_fp32_argmax_binding_v11(
                        Fp32ArgmaxBindingV11::recording(),
                    )
                    .unwrap();
            }
            assert_eq!(
                driver.layer_projection_mode(),
                if enabled { "c1-wave" } else { "mfma" }
            );
            assert_eq!(
                driver.projection.mode,
                super::super::super::EngineeringTpProjectionModeV3::Mfma
            );
            assert_eq!(allocations(&driver), before);
            let original = driver.inner.ranks[0]
                .global(Qwen3TensorKind::LanguageModelHead)
                .id;
            let batch = prepare(&mut pool, rows);
            pool.begin_submission(&batch).unwrap();
            let output = driver
                .execute_selected(&batch, &[rows as usize - 1])
                .unwrap();
            assert_eq!(driver.dispatch_counts(), [616]);
            assert_eq!(driver.completed_batches(), 1);
            assert_eq!(allocations(&driver), before);
            let transport = &driver.inner.transports[0];
            ordered_shape(transport, true);
            assert!(transport.pending.is_none() && transport.pending_ordered.is_none());
            let mut commands = transport.commands.clone();
            let head = commands
                .iter()
                .find(|command| command.kernel == HEAD)
                .unwrap();
            let transposed = buffer(head, 1).0;
            assert_ne!(transposed, original);
            let mut layer_count = 0;
            let mut waves = 0;
            for command in &mut commands {
                if matches!(command.kernel, MFMA | MFMA_PARTIAL | WAVE | WAVE_PARTIAL) {
                    layer_count += 1;
                    assert_eq!(scalar(command, 3), rows);
                    let wave = matches!(command.kernel, WAVE | WAVE_PARTIAL);
                    assert_eq!(wave, enabled && rows == 1);
                    assert_eq!(
                        buffer(command, 1).0,
                        if wave { original } else { transposed }
                    );
                    assert_eq!(
                        buffer(command, 2).3,
                        if matches!(command.kernel, MFMA_PARTIAL | WAVE_PARTIAL) {
                            4
                        } else {
                            2
                        }
                    );
                    if wave {
                        waves += 1;
                        assert_eq!(command.grid_workgroups, rows * scalar(command, 4));
                        command.kernel = if command.kernel == WAVE {
                            MFMA
                        } else {
                            MFMA_PARTIAL
                        };
                        let EngineeringTpArgumentV1::Buffer { id, .. } = &mut command.arguments[1]
                        else {
                            panic!("weight")
                        };
                        *id = transposed;
                        command.grid_workgroups = rows.div_ceil(16) * (scalar(command, 4) / 16);
                    }
                }
            }
            assert_eq!(layer_count, 7 * 36);
            assert_eq!(waves, if enabled && rows == 1 { 7 * 36 } else { 0 });
            recordings.push((
                commands,
                transport.reads.clone(),
                transport.writes.clone(),
                transport.write_payloads.clone(),
                transport.packet_preparations.clone(),
                before,
                output.choices.clone(),
            ));
            pool.commit_batch(&batch, output.completion).unwrap();
            pool.check_invariants().unwrap();
            pool.retire_sequence(batch.rows()[0].sequence(), false, 1)
                .unwrap();
            assert_eq!(pool.stats().free_pages, 4);
            assert_eq!(pool.stats().quarantined_pages, 0);
            driver.close().unwrap();
        }
        assert_eq!(recordings[0], recordings[1], "rows {rows}");
    }
}

#[test]
fn layer_c1_wave_pruning_keeps_zero_or_multiple_selected_head_rows_unchanged() {
    for rows in [1, 17, 32] {
        for selected in [Vec::new(), (0..rows as usize).collect()] {
            let mut heads = Vec::new();
            for enabled in [false, true] {
                let mut pool = wide_pool();
                let mut driver = configured(&pool);
                if enabled {
                    select(&mut driver).unwrap();
                } else {
                    driver
                        .configure_ordered_wave_attention_fp32_argmax_binding_v11(
                            Fp32ArgmaxBindingV11::recording(),
                        )
                        .unwrap();
                }
                let batch = prepare(&mut pool, rows);
                pool.begin_submission(&batch).unwrap();
                let output = driver.execute_selected(&batch, &selected).unwrap();
                let transport = &driver.inner.transports[0];
                ordered_shape(transport, !selected.is_empty());
                heads.push(transport.commands[613..].to_vec());
                assert_eq!(
                    driver.dispatch_counts(),
                    [if selected.is_empty() { 613 } else { 616 }]
                );
                pool.commit_batch(&batch, output.completion).unwrap();
                driver.close().unwrap();
            }
            assert_eq!(heads[0], heads[1]);
        }
    }
}

#[test]
fn layer_c1_wave_rejection_is_atomic_for_every_existing_profile_boundary() {
    for mutation in 0..27 {
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
            22 => driver.inner.transports[0].ordered_supported = false,
            23 => driver
                .inner
                .ranks
                .push(fixture(1, &wide_pool()).inner.ranks.pop().unwrap()),
            24 => driver
                .inner
                .transports
                .push(fixture(1, &wide_pool()).inner.transports.pop().unwrap()),
            25 => driver.c1_wave_layers = true,
            26 => driver.fp32_argmax_v11 = Some(Fp32ArgmaxBindingV11::recording()),
            _ => unreachable!(),
        }
        let before = (
            driver.c1_wave_layers,
            driver.fp32_argmax_v11,
            driver.inner.ordered_batches.clone(),
        );
        let io = || {
            driver
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
                .collect::<Vec<_>>()
        };
        let io_before = io();
        assert!(select(&mut driver).is_err(), "mutation {mutation}");
        assert_eq!(
            before,
            (
                driver.c1_wave_layers,
                driver.fp32_argmax_v11,
                driver.inner.ordered_batches.clone()
            )
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
fn layer_c1_wave_is_opt_in_and_terminal_without_broadening_old_selectors() {
    let mut legacy = configured(&wide_pool());
    legacy
        .configure_ordered_wave_attention_fp32_argmax_binding_v11(Fp32ArgmaxBindingV11::recording())
        .unwrap();
    assert!(!legacy.c1_wave_layers);
    assert!(select(&mut legacy).is_err());
    for mode in [
        super::super::super::EngineeringTpProjectionModeV3::Wave,
        super::super::super::EngineeringTpProjectionModeV3::Auto,
    ] {
        let mut driver = configured(&wide_pool());
        driver.projection.mode = mode;
        assert!(
            driver
                .configure_ordered_wave_attention_fp32_argmax_binding_v11(
                    Fp32ArgmaxBindingV11::recording()
                )
                .is_err()
        );
        assert!(
            driver
                .configure_wave_attention_fp32_argmax_binding_v11(Fp32ArgmaxBindingV11::recording())
                .is_err()
        );
        assert!(select(&mut driver).is_err());
        assert!(!driver.c1_wave_layers);
    }
    let mut driver = configured(&wide_pool());
    select(&mut driver).unwrap();
    assert!(select(&mut driver).is_err());
    assert!(
        driver
            .configure_ordered_wave_attention_fp32_argmax_binding_v11(
                Fp32ArgmaxBindingV11::recording()
            )
            .is_err()
    );
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
        assert!(driver.configure_ordered_batches(enabled).is_err());
        assert!(driver.configure_dispatch_sequences(enabled).is_err());
    }
    assert!(
        driver
            .configure_reduction(EngineeringTpReductionModeV3::DeviceTp1V3)
            .is_err()
    );
    assert_eq!(driver.layer_projection_mode(), "c1-wave");
    assert_eq!(
        driver.projection.mode,
        super::super::super::EngineeringTpProjectionModeV3::Mfma
    );
}

#[test]
fn layer_c1_wave_invalid_selected_rows_have_no_transport_effects() {
    for (rows, selected) in [
        (1, vec![1]),
        (1, vec![0, 0]),
        (17, vec![17]),
        (17, vec![1, 0]),
    ] {
        let mut pool = wide_pool();
        let mut driver = configured(&pool);
        select(&mut driver).unwrap();
        let before = allocations(&driver);
        let batch = prepare(&mut pool, rows);
        assert!(driver.execute_selected(&batch, &selected).is_err());
        assert_eq!(driver.completed_batches(), 0);
        assert_eq!(driver.dispatch_counts(), [0]);
        assert!(!driver.poisoned);
        assert_eq!(allocations(&driver), before);
        assert!(driver.inner.transports[0].packet_preparations.is_empty());
        assert!(driver.inner.transports[0].commands.is_empty());
        assert!(driver.inner.transports[0].writes.is_empty());
        assert!(driver.inner.ordered_batches.as_ref().unwrap().is_empty());
        driver.close().unwrap();
    }
}

#[test]
fn layer_c1_wave_failures_poison_without_completion_and_quarantine_pool() {
    for failure in [
        Failure::WaveProjectionSubmit,
        Failure::WaveProjectionWait,
        Failure::OrderedSubmit,
        Failure::OrderedWait,
        Failure::AttentionSubmit,
        Failure::AttentionWait,
        Failure::ResidualSubmit,
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
        let batch = prepare(&mut pool, 1);
        pool.begin_submission(&batch).unwrap();
        let collective = driver.inner.collective.expected();
        assert!(
            driver.execute_selected(&batch, &[0]).is_err(),
            "{failure:?}"
        );
        assert!(driver.poisoned);
        assert_eq!(driver.completed_batches(), 0);
        if matches!(
            failure,
            Failure::WaveProjectionSubmit
                | Failure::WaveProjectionWait
                | Failure::OrderedSubmit
                | Failure::OrderedWait
        ) {
            assert_eq!(driver.dispatch_counts(), [1]);
            assert_eq!(driver.inner.collective.expected(), collective);
        }
        assert!(driver.inner.ordered_batches.as_ref().unwrap().is_empty());
        assert!(driver.inner.transports[0].pending.is_none());
        assert!(driver.inner.transports[0].pending_ordered.is_none());
        assert!(driver.execute_selected(&batch, &[0]).is_err());
        pool.quarantine_batch(&batch).unwrap();
        assert_eq!(pool.stats().quarantined_pages, 4);
        assert_eq!(pool.stats().free_pages, 0);
        assert_eq!(
            pool.check_invariants(),
            Err(crate::tp_paged::EngineeringTpPagedErrorV1::Poisoned)
        );
        driver.close().unwrap();
    }
}
