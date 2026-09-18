//! Command/ownership tests, not GPU emulation or numerical equivalence evidence.

use super::*;

const WAVE: &str = "ferric_qwen3_tp_wave_paged_gqa_bf16_v3";

fn configured(pool: &EngineeringTpPagedPoolV1) -> EngineeringTpBatchExecutionV2<Recording> {
    let mut driver = fixture(1, pool);
    driver
        .configure_reduction(EngineeringTpReductionModeV3::DeviceTp1V3)
        .unwrap();
    // Every synthetic weight aliases this one placeholder. Real weight intake
    // and authenticated transposition are covered by their existing tests.
    let original = driver.inner.ranks[0].global(Qwen3TensorKind::LanguageModelHead);
    let transposed = allocate_tensor(&mut driver.inner.transports[0], 1, 2).unwrap();
    driver.projection =
        super::super::super::projection::ProjectionPolicy::synthetic_mfma_for_recording(
            original.id,
            transposed,
        );
    driver.projection_configured = true;
    driver
}

#[test]
fn v7_wave_preserves_every_operand_grid_host_byte_and_non_attention_command() {
    for rows in [1, 3, 16] {
        for prune in [false, true] {
            for selected in [
                Vec::new(),
                vec![rows - 1],
                (0..rows).collect(),
                (0..rows).step_by(2).collect(),
            ] {
                let mut recordings = Vec::new();
                for wave in [false, true] {
                    let mut pool = pool();
                    let mut driver = configured(&pool);
                    driver.configure_output_head_pruning(prune).unwrap();
                    driver.configure_wave_attention(wave).unwrap();
                    driver.configure_head_precision_v7(true).unwrap();
                    assert_eq!(driver.fp32_head_workspace_bytes(), 9_723_904);
                    let batch = prepare(&mut pool, rows);
                    pool.begin_submission(&batch).unwrap();
                    let selected = selected.iter().map(|&row| row as usize).collect::<Vec<_>>();
                    let result = driver.execute_selected(&batch, &selected).unwrap();
                    let head_rows = if prune { selected.len() } else { rows as usize };
                    let packets = if head_rows == 0 { 613 } else { 616 };
                    assert_eq!(driver.dispatch_counts(), vec![packets]);
                    assert_eq!(
                        driver.expected_dispatch_counts(selected.len()),
                        vec![packets]
                    );
                    let transport = &driver.inner.transports[0];
                    // Queue capacity reserves the unchanged conservative bound;
                    // pruned no-head execution still emits only 613 packets.
                    assert_eq!(transport.packet_preparations, vec![616]);
                    let mut commands = transport.commands.clone();
                    assert_eq!(commands.len(), packets as usize);
                    let mut attention_count = 0;
                    for (index, command) in commands.iter_mut().enumerate() {
                        if command.kernel == if wave { WAVE } else { ATTENTION } {
                            assert_eq!(index, 9 + attention_count * 17);
                            assert_eq!(command.grid_workgroups, rows * 32);
                            assert_eq!(command.workgroup_size, 64);
                            assert_eq!(command.arguments.len(), 11);
                            for operand in 0..6 {
                                assert_eq!(
                                    matches!(
                                        command.arguments[operand],
                                        EngineeringTpArgumentV1::Buffer {
                                            access: EngineeringTpBufferAccessV1::Read,
                                            ..
                                        }
                                    ),
                                    operand < 5
                                );
                            }
                            assert_eq!(scalar(command, 6), rows);
                            assert_eq!(scalar(command, 7), 1);
                            assert_eq!(scalar(command, 8), 4);
                            assert_eq!(scalar(command, 9), 4);
                            assert_eq!(scalar(command, 10), rows);
                            assert_eq!(buffer(command, 0).3, 2);
                            assert_eq!(buffer(command, 5).3, 2);
                            command.kernel = ATTENTION;
                            attention_count += 1;
                        }
                    }
                    assert_eq!(attention_count, 36);
                    if head_rows != 0 {
                        assert_eq!(commands[614].kernel, FP32_MFMA_HEAD);
                        assert_eq!(commands[615].kernel, FP32_ARGMAX);
                        assert_eq!(commands[615].grid_workgroups, head_rows as u32);
                        assert_eq!(buffer(&commands[614], 2), buffer(&commands[615], 0));
                        assert_eq!(buffer(&commands[614], 2).2, 16 * 151_936);
                        assert_eq!(buffer(&commands[614], 2).3, 4);
                    }
                    let events = transport
                        .events
                        .borrow()
                        .iter()
                        .map(|event| match event {
                            Event::Submit(rank, name) if *name == WAVE => {
                                Event::Submit(*rank, ATTENTION)
                            }
                            Event::Wait(rank, name) if *name == WAVE => {
                                Event::Wait(*rank, ATTENTION)
                            }
                            other => other.clone(),
                        })
                        .collect::<Vec<_>>();
                    recordings.push((
                        commands,
                        transport.reads.clone(),
                        transport.writes.clone(),
                        transport.write_payloads.clone(),
                        transport.buffers.clone(),
                        events,
                        result.choices.clone(),
                    ));
                    pool.commit_batch(&batch, result.completion).unwrap();
                    driver.close().unwrap();
                }
                assert_eq!(recordings[0], recordings[1]);
            }
        }
    }
}

#[test]
fn v7_wave_requires_exact_positive_combination_and_preserves_other_guards() {
    for mutation in 0..14 {
        let mut driver = configured(&pool());
        driver.configure_wave_attention(true).unwrap();
        match mutation {
            0 => {
                driver.projection.mode =
                    super::super::super::EngineeringTpProjectionModeV3::Baseline
            }
            1 => driver.projection.mode = super::super::super::EngineeringTpProjectionModeV3::Wave,
            2 => driver.projection.mode = super::super::super::EngineeringTpProjectionModeV3::Auto,
            3 => driver.inner.reduction = super::super::super::ReductionWorkspace::default(),
            4 => driver.inner.sequences = Some(vec![vec![]]),
            5 => driver.inner.ordered_batches = Some(Vec::new()),
            6 => driver.inner.large_kv = true,
            7 => driver.inner.draft_v10 = true,
            8 => driver.inner.transports[0].argmax_peer = Some((0, 1, 0)),
            9 => driver.row_capacity = 32,
            10 => driver.last_batch = 1,
            11 => driver.poisoned = true,
            12 => driver.inner.closed = true,
            13 => driver.inner.plan = Qwen3TensorParallelPlanV1::new(target(), 2).unwrap(),
            _ => unreachable!(),
        }
        let allocations = driver.inner.transports[0].buffers.len();
        assert!(
            driver.configure_head_precision_v7(true).is_err(),
            "mutation {mutation}"
        );
        assert!(!driver.head_profile_configured && driver.fp32_logits.is_none());
        assert_eq!(driver.inner.transports[0].buffers.len(), allocations);
    }
    let mut driver = configured(&pool());
    driver.configure_wave_attention(true).unwrap();
    assert!(driver.configure_head_precision_v7(false).is_err());
    assert!(driver.configure_head_precision_v8(true).is_err());
    for world in [2, 8] {
        let mut driver = fixture(world, &pool());
        driver.configure_wave_attention(true).unwrap();
        assert!(driver.configure_head_precision_v7(true).is_err());
    }
}

#[test]
fn v7_wave_head_freezes_attention_and_rejects_ordered_profiles() {
    let mut driver = configured(&pool());
    driver.configure_wave_attention(true).unwrap();
    driver.configure_head_precision_v7(true).unwrap();
    for enabled in [false, true] {
        assert!(driver.configure_wave_attention(enabled).is_err());
        assert!(driver.configure_head_precision_v7(enabled).is_err());
        assert!(driver.configure_head_precision_v8(enabled).is_err());
    }
    assert!(driver.configure_dispatch_sequences(true).is_err());
    for mode in [
        EngineeringTpReductionModeV3::HostStagedV1,
        EngineeringTpReductionModeV3::HostStagedReuseV3,
        EngineeringTpReductionModeV3::DeviceTp1V3,
    ] {
        assert!(driver.configure_reduction(mode).is_err());
    }
    assert!(driver.configure_ordered_batches(true).is_err());
    assert!(driver.configure_scalar_v3_ordered_batches().is_err());
    driver.close().unwrap();
    assert!(driver.configure_head_precision_v7(true).is_err());
}

#[test]
fn v7_wave_attention_failure_keeps_residual_cursor_and_head_unpublished() {
    for failure in [Failure::AttentionSubmit, Failure::AttentionWait] {
        let mut pool = pool();
        let mut driver = configured(&pool);
        driver.configure_wave_attention(true).unwrap();
        driver.configure_head_precision_v7(true).unwrap();
        let hidden = driver.inner.ranks[0].hidden.id;
        let logits = driver.fp32_logits.unwrap().id;
        driver.inner.transports[0].failure = Some(failure);
        let batch = prepare(&mut pool, 3);
        pool.begin_submission(&batch).unwrap();
        assert!(driver.execute(&batch).is_err());
        assert_eq!(driver.completed_batches, 0);
        assert_eq!(driver.dispatch_counts(), vec![9]);
        assert_eq!(driver.inner.ranks[0].hidden.id, hidden);
        let cursor = driver.inner.collective.expected();
        assert_eq!(cursor.epoch, 0);
        assert_eq!(cursor.layer, 0);
        assert_eq!(
            cursor.operation,
            Qwen3TensorParallelCollectiveV1::AttentionOutputSum
        );
        assert!(
            driver.inner.transports[0].buffers[&logits]
                .iter()
                .all(|&byte| byte == 0xa5)
        );
        assert!(driver.inner.transports[0].reads.is_empty());
        assert!(driver.poisoned);
        assert!(driver.execute(&batch).is_err());
        assert!(driver.configure_head_precision_v7(true).is_err());
        assert!(driver.runtime_diagnostic_snapshot().is_err());
        driver.close().unwrap();
        assert!(driver.inner.closed && driver.inner.transports[0].pending.is_none());
    }
}
