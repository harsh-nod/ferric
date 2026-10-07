//! Scheduling and failure tests with synthetic bytes, not split-attention arithmetic emulation.

use super::super::c1_split_attention_v25::{Workspace, admit};
use super::*;
use crate::tp_artifact::{
    ENGINEERING_TP_SPLIT_ATTENTION_EXPORTS_V21, Fp32ArgmaxBindingV11, QueryHoistBindingV14,
    SplitAttentionBindingV21, WaveRmsNormBindingV15,
};

fn wide_context_pool() -> EngineeringTpPagedPoolV1 {
    EngineeringTpPagedPoolV1::new_wide32(
        pool().scope(),
        EngineeringTpPagedLimitsV1::new(512, 32, 32, 100).unwrap(),
    )
    .unwrap()
}

fn configured(pool: &EngineeringTpPagedPoolV1) -> EngineeringTpBatchExecutionV2<Recording> {
    let mut driver = fixture_for_model_capacity(1, pool, target(), 512, 32);
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
    driver.admitted_query_hoist_v14 = Some(QueryHoistBindingV14::recording());
    driver.admitted_wave_rmsnorm_v15 = Some(WaveRmsNormBindingV15::recording());
    let norm_weight = allocate_tensor(&mut driver.inner.transports[0], 4096, 2).unwrap();
    for layer in &mut driver.inner.ranks[0].layers {
        for (kind, weight) in &mut layer.weights {
            if matches!(
                kind,
                Qwen3TensorKind::InputLayerNorm | Qwen3TensorKind::PostAttentionLayerNorm
            ) {
                *weight = norm_weight;
            }
        }
    }
    for (kind, weight) in &mut driver.inner.ranks[0].globals {
        if *kind == Qwen3TensorKind::FinalNorm {
            *weight = norm_weight;
        }
    }
    driver.split_attention_workspace_v25 = Some(
        Workspace::allocate(
            &mut driver.inner.transports[0],
            SplitAttentionBindingV21::recording(),
        )
        .unwrap(),
    );
    driver
}

fn select(driver: &mut EngineeringTpBatchExecutionV2<Recording>, enabled: bool) -> TpResult<()> {
    driver.configure_split_bindings_v25(
        Fp32ArgmaxBindingV11::recording(),
        QueryHoistBindingV14::recording(),
        WaveRmsNormBindingV15::recording(),
        SplitAttentionBindingV21::recording(),
        enabled,
    )
}

fn prepare_at(
    pool: &mut EngineeringTpPagedPoolV1,
    context: u32,
    rows: u32,
) -> EngineeringTpPreparedBatchV1 {
    assert!(rows > 0 && rows <= context && rows <= 32);
    let prompt = vec![1; context as usize];
    let sequence = pool
        .open_sequence(pool.scope(), &prompt, 0)
        .unwrap()
        .sequence();
    // Publish only synthetic host metadata to reach the desired suffix. No earlier GPU work or numerical claim.
    let prefix = context - rows;
    for start in (0..prefix).step_by(32) {
        let selected = (start..(start + 32).min(prefix))
            .map(|position| EngineeringTpPageRowV1 {
                sequence,
                token: 1,
                position,
            })
            .collect::<Vec<_>>();
        let batch = pool.reserve_batch(&selected).unwrap();
        pool.begin_submission(&batch).unwrap();
        pool.commit_batch(
            &batch,
            crate::tp_paged::EngineeringTpBatchCompletionV1::after_all_ranks(&batch),
        )
        .unwrap();
    }
    let selected = (prefix..context)
        .map(|position| EngineeringTpPageRowV1 {
            sequence,
            token: 1,
            position,
        })
        .collect::<Vec<_>>();
    pool.reserve_batch(&selected).unwrap()
}

fn groups(driver: &EngineeringTpBatchExecutionV2<Recording>) -> Vec<usize> {
    driver.inner.transports[0]
        .events
        .borrow()
        .iter()
        .filter_map(|event| {
            if let Event::OrderedSubmit(0, count) = event {
                Some(*count)
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn c1_split_attention_v25_prefill_composition_preserves_fallback_and_scratch_across_rollover() {
    use super::prefill_kv_copy_v28::{composition_configured, select_composition};
    let mut pool = wide_context_pool();
    let mut driver = composition_configured(&pool);
    select_composition(&mut driver, true, true, true).unwrap();
    driver.inner.transports[0].rollover_supported = true;
    let scratch = driver.split_attention_workspace_v25.unwrap();
    for (ordinal, (context, rows, packets, copies, splits)) in [
        (127, 1, 616, 0, 0), (128, 1, 652, 0, 36), (192, 16, 616, 36, 0),
        (257, 1, 616, 0, 0), (192, 32, 616, 0, 0),
    ].into_iter().enumerate() {
        driver.inner.transports[0].queue_packets = fe2o3_kfd::engineering_wire::MAX_UNRETIRED_RING_PACKETS_V1 - 600;
        let batch = prepare_at(&mut pool, context, rows);
        let before = driver.inner.transports[0].commands.len();
        pool.begin_submission(&batch).unwrap();
        let output = driver.execute_selected(&batch, &[rows as usize - 1]).unwrap();
        pool.commit_batch(&batch, output.completion).unwrap();
        pool.retire_sequence(batch.rows()[0].sequence(), false, 0).unwrap();
        let commands = &driver.inner.transports[0].commands[before..];
        assert_eq!(commands.len(), packets);
        assert_eq!(commands.iter().filter(|command| command.kernel == crate::tp_artifact::ENGINEERING_TP_PREFILL_KV_COPY_EXPORTS_V27[0]).count(), copies);
        assert_eq!(commands.iter().filter(|command| command.kernel == ENGINEERING_TP_SPLIT_ATTENTION_EXPORTS_V21[0]).count(), splits);
        let current = driver.split_attention_workspace_v25.unwrap();
        assert_eq!((current.stats.id, current.numerators.id), (scratch.stats.id, scratch.numerators.id));
        assert!(driver.inner.packed_c1.is_none());
        assert!(driver.inner.ordered_batches.as_ref().unwrap().is_empty());
        assert_eq!(driver.inner.collective.expected().epoch, ordinal as u64 + 1);
        pool.check_invariants().unwrap();
    }
    assert_eq!(driver.inner.transports[0].queue_epochs, 5);
    driver.close().unwrap();
}

#[test]
fn c1_split_attention_v25_prefill_composition_failures_poison_without_committing() {
    use super::prefill_kv_copy_v28::{composition_configured, select_composition};
    for (rows, failure) in [
        (16, Failure::KvCopySubmit), (16, Failure::KvCopyWait),
        (1, Failure::SplitSubmit(0)), (1, Failure::SplitWait(0)),
        (1, Failure::SplitSubmit(1)), (1, Failure::SplitWait(1)),
        (1, Failure::OrderedSubmitAt(2)), (1, Failure::OrderedWaitAt(2)),
        (16, Failure::PreparePackets), (1, Failure::BadChoice),
    ] {
        let mut pool = wide_context_pool();
        let mut driver = composition_configured(&pool);
        select_composition(&mut driver, true, true, true).unwrap();
        let batch = prepare_at(&mut pool, 192, rows);
        pool.begin_submission(&batch).unwrap();
        driver.inner.transports[0].failure = Some(failure);
        assert!(driver.execute_selected(&batch, &[rows as usize - 1]).is_err(), "{failure:?}");
        assert!(driver.poisoned);
        assert_eq!(driver.completed_batches, 0);
        let count = driver.dispatch_counts();
        assert!(driver.execute_selected(&batch, &[rows as usize - 1]).is_err());
        assert_eq!(driver.dispatch_counts(), count);
        pool.quarantine_batch(&batch).unwrap();
        assert!(pool.check_invariants().is_err());
        driver.close().unwrap();
    }
}

#[test]
fn c1_split_attention_v25_exact_context_shape_fallback_and_packet_accounting() {
    for (context, rows) in [
        (127, 1),
        (128, 1),
        (129, 1),
        (191, 1),
        (192, 1),
        (193, 1),
        (255, 1),
        (256, 1),
        (257, 1),
        (128, 16),
        (192, 32),
    ] {
        for enabled in [false, true] {
            let mut pool = wide_context_pool();
            let mut driver = configured(&pool);
            select(&mut driver, enabled).unwrap();
            let workspace = driver.split_attention_workspace_v25.unwrap();
            let allocations = driver.inner.transports[0]
                .buffers
                .iter()
                .map(|(&id, bytes)| (id, bytes.len()))
                .collect::<Vec<_>>();
            let batch = prepare_at(&mut pool, context, rows);
            let split = enabled && rows == 1 && (128..=256).contains(&context);
            let expected = if split { 652 } else { 616 };
            assert_eq!(
                driver
                    .expected_dispatch_counts_for_batch(&batch, 1)
                    .unwrap(),
                [expected]
            );
            assert_eq!(
                driver
                    .expected_dispatch_counts_for_batch(&batch, 0)
                    .unwrap(),
                [expected - 3]
            );
            assert!(
                driver
                    .expected_dispatch_counts_for_batch(&batch, rows as usize + 1)
                    .is_err()
            );
            pool.begin_submission(&batch).unwrap();
            let output = driver
                .execute_selected(&batch, &[rows as usize - 1])
                .unwrap();
            assert_eq!(output.choices, [42]);
            assert_eq!(driver.dispatch_counts(), [expected]);
            assert_eq!(groups(&driver), [if split { 12 } else { 11 }, 6].repeat(36));
            assert_eq!(driver.inner.transports[0].packet_preparations, [expected]);
            let commands = &driver.inner.transports[0].commands;
            let partials = commands
                .iter()
                .filter(|command| command.kernel == ENGINEERING_TP_SPLIT_ATTENTION_EXPORTS_V21[0])
                .collect::<Vec<_>>();
            let merges = commands
                .iter()
                .filter(|command| command.kernel == ENGINEERING_TP_SPLIT_ATTENTION_EXPORTS_V21[1])
                .collect::<Vec<_>>();
            assert_eq!(
                (partials.len(), merges.len()),
                if split { (36, 36) } else { (0, 0) }
            );
            assert_eq!(
                commands
                    .iter()
                    .filter(|command| command.kernel
                        == crate::tp_artifact::ENGINEERING_TP_QUERY_HOIST_EXPORTS_V14[0])
                    .count(),
                if split { 0 } else { 36 }
            );
            for (partial, merge) in partials.into_iter().zip(merges) {
                assert_eq!((partial.grid_workgroups, merge.grid_workgroups), (256, 32));
                assert_eq!(scalar(partial, 11), context);
                assert_eq!(
                    (buffer(partial, 5).0, buffer(merge, 0).0),
                    (workspace.stats.id, workspace.stats.id)
                );
                assert_eq!(
                    (buffer(partial, 6).0, buffer(merge, 1).0),
                    (workspace.numerators.id, workspace.numerators.id)
                );
            }
            assert_eq!(driver.split_attention_workspace_bytes(), 133_120);
            assert_eq!(
                driver.inner.transports[0]
                    .buffers
                    .iter()
                    .map(|(&id, bytes)| (id, bytes.len()))
                    .collect::<Vec<_>>(),
                allocations
            );
            assert!(driver.inner.ordered_batches.as_ref().unwrap().is_empty());
            assert!(driver.inner.packed_c1.is_none());
            pool.commit_batch(&batch, output.completion).unwrap();
            pool.check_invariants().unwrap();
            driver.close().unwrap();
        }
    }
}

#[test]
fn c1_split_attention_v25_preserves_every_other_command_and_original_v17_control() {
    let mut baseline = None;
    for enabled in [None, Some(false), Some(true)] {
        let mut pool = wide_context_pool();
        let mut driver = configured(&pool);
        if let Some(enabled) = enabled {
            select(&mut driver, enabled).unwrap();
        } else {
            driver
                .configure_ordered_c1_wave_target_bindings_v17(
                    Fp32ArgmaxBindingV11::recording(),
                    QueryHoistBindingV14::recording(),
                    WaveRmsNormBindingV15::recording(),
                    EngineeringTpWaveTargetModeV17::Combined,
                )
                .unwrap();
        }
        let batch = prepare_at(&mut pool, 192, 1);
        pool.begin_submission(&batch).unwrap();
        let output = driver.execute_selected(&batch, &[0]).unwrap();
        let transport = &driver.inner.transports[0];
        let commands = transport
            .commands
            .iter()
            .filter(|command| {
                !ENGINEERING_TP_SPLIT_ATTENTION_EXPORTS_V21.contains(&command.kernel)
                    && command.kernel
                        != crate::tp_artifact::ENGINEERING_TP_QUERY_HOIST_EXPORTS_V14[0]
            })
            .cloned()
            .collect::<Vec<_>>();
        let recorded = (
            commands,
            transport.reads.clone(),
            transport.write_payloads.clone(),
            output.choices.clone(),
            driver.inner.collective,
            driver.inner.ranks[0].hidden.id,
        );
        if let Some(expected) = &baseline {
            assert_eq!(&recorded, expected);
        } else {
            baseline = Some(recorded);
        }
        pool.commit_batch(&batch, output.completion).unwrap();
        driver.close().unwrap();
    }
}

#[test]
fn c1_split_attention_v25_failed_partial_merge_and_frontiers_never_publish_uncompleted_state() {
    for failure in [
        Failure::SplitSubmit(0),
        Failure::SplitWait(0),
        Failure::SplitSubmit(1),
        Failure::SplitWait(1),
        Failure::OrderedSubmitAt(1),
        Failure::OrderedWaitAt(1),
        Failure::OrderedSubmitAt(2),
        Failure::OrderedWaitAt(2),
        Failure::OrderedSubmitAt(71),
        Failure::OrderedWaitAt(72),
    ] {
        let mut pool = wide_context_pool();
        let mut driver = configured(&pool);
        select(&mut driver, true).unwrap();
        let completed_groups = match failure {
            Failure::OrderedSubmitAt(ordinal) | Failure::OrderedWaitAt(ordinal) => ordinal - 1,
            _ => 0,
        };
        let mut expected = driver.inner.collective;
        for _ in 0..completed_groups {
            let key = expected.expected();
            expected.arrive(0, key).unwrap();
            expected.advance().unwrap();
        }
        let original_hidden = driver.inner.ranks[0].hidden.id;
        let super::super::super::reduction::ReductionWorkspace::DeviceTp1(scratch) =
            driver.inner.reduction
        else {
            panic!("TP1 scratch")
        };
        driver.inner.transports[0].failure = Some(failure);
        let batch = prepare_at(&mut pool, 192, 1);
        pool.begin_submission(&batch).unwrap();
        assert!(
            driver.execute_selected(&batch, &[0]).is_err(),
            "{failure:?}"
        );
        assert_eq!(driver.inner.collective, expected, "{failure:?}");
        assert_eq!(
            driver.inner.ranks[0].hidden.id,
            if completed_groups.is_multiple_of(2) {
                original_hidden
            } else {
                scratch.id
            }
        );
        assert!(driver.inner.transports[0].reads.is_empty());
        assert!(driver.poisoned && driver.completed_batches() == 0);
        assert!(driver.inner.ordered_batches.as_ref().unwrap().is_empty());
        assert!(driver.execute_selected(&batch, &[0]).is_err());
        pool.quarantine_batch(&batch).unwrap();
        driver.close().unwrap();
    }
}

#[test]
fn c1_split_attention_v25_completed_device_work_is_not_rolled_back_after_bad_head_output() {
    let mut pool = wide_context_pool();
    let mut driver = configured(&pool);
    select(&mut driver, true).unwrap();
    driver.inner.transports[0].failure = Some(Failure::BadChoice);
    let batch = prepare_at(&mut pool, 192, 1);
    pool.begin_submission(&batch).unwrap();
    assert!(driver.execute_selected(&batch, &[0]).is_err());
    assert_eq!(driver.dispatch_counts(), [652]);
    assert_eq!(driver.inner.collective.expected().epoch, 1);
    assert!(driver.poisoned && driver.completed_batches() == 0);
    assert_eq!(driver.inner.transports[0].reads.len(), 1);
    pool.quarantine_batch(&batch).unwrap();
    driver.close().unwrap();
}

#[test]
fn c1_split_attention_v25_no_rollover_guard_keeps_the_full_mixed_schedule_bound() {
    for enabled in [false, true] {
        let mut pool = wide_context_pool();
        let mut driver = configured(&pool);
        select(&mut driver, enabled).unwrap();
        driver.completed_batches = fe2o3_kfd::engineering_wire::MAX_UNRETIRED_RING_PACKETS_V1 / 652;
        let batch = prepare_at(&mut pool, 192, 16);
        assert!(driver.execute_selected(&batch, &[15]).is_err());
        assert!(driver.inner.transports[0].commands.is_empty());
        assert!(driver.inner.transports[0].packet_preparations.is_empty());
        pool.abort_batch(&batch).unwrap();
        driver.close().unwrap();
    }
}

#[test]
fn c1_split_attention_v25_selection_is_atomic_sealed_and_excludes_other_candidates() {
    for enabled in [false, true] {
        let mut driver = configured(&wide_context_pool());
        select(&mut driver, enabled).unwrap();
        assert_eq!(
            driver.split_attention_mode(),
            if enabled { "split8-v21" } else { "baseline" }
        );
        assert!(select(&mut driver, enabled).is_err());
        assert!(select(&mut driver, !enabled).is_err());
        assert!(driver.configure_c1_packet_packing_v22(false).is_err());
        driver.close().unwrap();
    }
    for mutation in 0..15 {
        let mut driver = configured(&wide_context_pool());
        match mutation {
            0 => driver.split_attention_workspace_v25 = None,
            1 => {
                driver
                    .split_attention_workspace_v25
                    .as_mut()
                    .unwrap()
                    .image
                    .hsaco[0] ^= 1;
            }
            2 => {
                let workspace = driver.split_attention_workspace_v25.as_mut().unwrap();
                workspace.numerators.id = workspace.stats.id;
            }
            3 => {
                driver
                    .split_attention_workspace_v25
                    .as_mut()
                    .unwrap()
                    .stats
                    .elements = 511;
            }
            4 => {
                driver
                    .split_attention_workspace_v25
                    .as_mut()
                    .unwrap()
                    .numerators
                    .elements = 32_767;
            }
            5 => {
                driver
                    .split_attention_workspace_v25
                    .as_mut()
                    .unwrap()
                    .stats
                    .element_bytes = 2;
            }
            6 => driver.c1_packet_packing_v22 = Some(false),
            7 => {
                driver.admitted_c1_kv_copy_v19 =
                    Some(crate::tp_artifact::C1KvCopyBindingV19::recording());
            }
            8 => driver.inner.large_kv = true,
            9 => driver.inner.draft_v10 = true,
            10 => driver.last_batch = 1,
            11 => driver.completed_batches = 1,
            12 => driver.poisoned = true,
            13 => driver.admitted_query_hoist_v14 = None,
            14 => driver.inner.transports[0].ordered_supported = false,
            _ => unreachable!(),
        }
        let committed = driver.inner.collective;
        assert!(select(&mut driver, true).is_err(), "mutation {mutation}");
        assert!(driver.c1_split_attention_v25.is_none());
        assert_eq!(driver.inner.collective, committed);
        assert!(driver.inner.transports[0].commands.is_empty());
    }
}

#[test]
fn c1_split_attention_v25_image_admission_is_exact_and_preallocation() {
    let image = SplitAttentionBindingV21::recording();
    for mutation in 0..6 {
        let mut driver = fixture(1, &wide_pool());
        let transport = &mut driver.inner.transports[0];
        transport.buffers.clear();
        transport.split_attention_v21_loaded = Some(image.hsaco);
        match mutation {
            0 => {}
            1 => transport.split_attention_v21_loaded = None,
            2 => transport.split_attention_v21_loaded.as_mut().unwrap()[0] ^= 1,
            3 => transport.argmax_peer = Some((1, 0, 1)),
            4 => transport.ordered_supported = false,
            5 => {
                transport.buffers.insert(999, vec![0]);
            }
            _ => unreachable!(),
        }
        let before = transport.buffers.clone();
        assert_eq!(
            admit(&mut driver.inner.transports, image).is_ok(),
            mutation == 0
        );
        assert_eq!(driver.inner.transports[0].buffers, before);
        assert!(driver.inner.transports[0].commands.is_empty());
    }
}

#[test]
fn c1_split_attention_v25_either_scratch_allocation_failure_closes_the_owned_transport() {
    for ordinal in 0..2 {
        let mut driver = fixture_for_model_capacity(1, &wide_context_pool(), target(), 512, 32);
        let next = driver.inner.transports[0].next;
        driver.inner.transports[0].failure = Some(Failure::AllocateAt(next + ordinal));
        assert!(
            driver
                .allocate_split_workspace_v25(SplitAttentionBindingV21::recording())
                .is_err()
        );
        assert!(driver.poisoned && driver.inner.closed);
        assert!(driver.split_attention_workspace_v25.is_none());
        assert!(driver.inner.transports[0].commands.is_empty());
        assert_eq!(
            *driver.inner.transports[0].events.borrow(),
            [Event::Close(0)]
        );
    }
}

#[test]
fn c1_split_attention_v25_headless_scratch_reuse_and_fixed_request_counts() {
    let mut pool = wide_context_pool();
    let mut driver = configured(&pool);
    select(&mut driver, true).unwrap();
    driver.inner.transports[0].rollover_supported = true;
    let workspace = driver.split_attention_workspace_v25.unwrap();
    for (ordinal, selected) in [Vec::new(), vec![0]].into_iter().enumerate() {
        let batch = prepare_at(&mut pool, 192, 1);
        assert_eq!(
            driver
                .expected_dispatch_counts_for_batch(&batch, selected.len())
                .unwrap(),
            [if selected.is_empty() { 649 } else { 652 }]
        );
        pool.begin_submission(&batch).unwrap();
        let output = driver.execute_selected(&batch, &selected).unwrap();
        pool.commit_batch(&batch, output.completion).unwrap();
        pool.retire_sequence(batch.rows()[0].sequence(), false, 0)
            .unwrap();
        assert_eq!(driver.inner.collective.expected().epoch, ordinal as u64 + 1);
        let current = driver.split_attention_workspace_v25.unwrap();
        assert_eq!(
            (current.stats.id, current.numerators.id),
            (workspace.stats.id, workspace.numerators.id)
        );
    }
    assert_eq!(driver.dispatch_counts(), [649 + 652]);
    // Eight 16-row prefill batches fall back; only 127 one-row decodes split.
    assert_eq!(7 * 613 + 616 + 127 * 652, 87_711);
    assert_eq!(87_711 - 83_139, 36 * 127);
    driver.close().unwrap();
}

#[test]
fn c1_split_attention_v25_packing_preserves_commands_scratch_and_fallbacks() {
    for (context, rows) in [(127, 1), (128, 1), (192, 1), (256, 1), (257, 1), (192, 16)] {
        for split_enabled in [false, true] {
            for selected in [Vec::new(), vec![rows as usize - 1]] {
                let mut baseline = None;
                for packed in [None, Some(false), Some(true)] {
                    let mut pool = wide_context_pool();
                    let mut driver = configured(&pool);
                    select(&mut driver, split_enabled).unwrap();
                    if let Some(packed) = packed {
                        driver
                            .configure_c1_split_packet_packing_v25(packed)
                            .unwrap();
                    }
                    let workspace = driver.split_attention_workspace_v25.unwrap();
                    let allocations = driver.inner.transports[0].buffers.len();
                    let batch = prepare_at(&mut pool, context, rows);
                    let split = split_enabled && rows == 1 && (128..=256).contains(&context);
                    let expected =
                        if split { 652 } else { 616 } - if selected.is_empty() { 3 } else { 0 };
                    assert_eq!(
                        driver
                            .expected_dispatch_counts_for_batch(&batch, selected.len())
                            .unwrap(),
                        [expected]
                    );
                    pool.begin_submission(&batch).unwrap();
                    let output = driver.execute_selected(&batch, &selected).unwrap();
                    let transport = &driver.inner.transports[0];
                    let recorded = (
                        transport.commands.clone(),
                        transport.reads.clone(),
                        transport.write_payloads.clone(),
                        transport.packet_preparations.clone(),
                        output.choices.clone(),
                        driver.inner.collective,
                        driver.inner.ranks[0].hidden.id,
                    );
                    if let Some(baseline) = &baseline {
                        assert_eq!(
                            &recorded, baseline,
                            "context {context}, rows {rows}, packed {packed:?}"
                        );
                    } else {
                        baseline = Some(recorded);
                    }
                    assert_eq!(transport.buffers.len(), allocations);
                    assert_eq!(driver.dispatch_counts(), [expected]);
                    if packed == Some(true) && rows == 1 && selected == [0] {
                        let packets = usize::try_from(expected).unwrap();
                        let mut expected_groups = vec![16; packets / 16];
                        expected_groups.push(packets % 16);
                        assert_eq!(groups(&driver), expected_groups);
                    } else {
                        assert_eq!(groups(&driver), [if split { 12 } else { 11 }, 6].repeat(36));
                    }
                    let partials = transport
                        .commands
                        .iter()
                        .enumerate()
                        .filter(|(_, command)| {
                            command.kernel == ENGINEERING_TP_SPLIT_ATTENTION_EXPORTS_V21[0]
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(partials.len(), if split { 36 } else { 0 });
                    let mut crosses_group = false;
                    for (index, partial) in partials {
                        let merge = &transport.commands[index + 1];
                        assert_eq!(merge.kernel, ENGINEERING_TP_SPLIT_ATTENTION_EXPORTS_V21[1]);
                        assert_eq!(buffer(partial, 5).0, workspace.stats.id);
                        assert_eq!(buffer(partial, 6).0, workspace.numerators.id);
                        assert_eq!(buffer(merge, 0).0, workspace.stats.id);
                        assert_eq!(buffer(merge, 1).0, workspace.numerators.id);
                        crosses_group |= index % 16 == 15;
                    }
                    if split && packed == Some(true) && selected == [0] {
                        assert!(
                            crosses_group,
                            "exercise partial/merge across a completion boundary"
                        );
                    }
                    assert!(driver.inner.packed_c1.is_none());
                    assert!(driver.inner.ordered_batches.as_ref().unwrap().is_empty());
                    pool.commit_batch(&batch, output.completion).unwrap();
                    pool.check_invariants().unwrap();
                    driver.close().unwrap();
                }
            }
        }
    }
}

#[test]
fn c1_split_attention_v25_packing_each_failed_frontier_commits_only_completed_metadata() {
    for ordinal in 1..=41 {
        for failure in [
            Failure::OrderedSubmitAt(ordinal),
            Failure::OrderedWaitAt(ordinal),
        ] {
            let mut pool = wide_context_pool();
            let mut driver = configured(&pool);
            select(&mut driver, true).unwrap();
            driver.configure_c1_split_packet_packing_v25(true).unwrap();
            let workspace = driver.split_attention_workspace_v25.unwrap();
            let original_hidden = driver.inner.ranks[0].hidden.id;
            let super::super::super::reduction::ReductionWorkspace::DeviceTp1(scratch) =
                driver.inner.reduction
            else {
                panic!("TP1 scratch")
            };
            let completed_packets = (ordinal - 1) * 16;
            let residuals = (0..36)
                .flat_map(|layer| [13 + 18 * layer, 19 + 18 * layer])
                .filter(|packet| *packet <= completed_packets)
                .count();
            let mut expected = driver.inner.collective;
            for _ in 0..residuals {
                let key = expected.expected();
                expected.arrive(0, key).unwrap();
                expected.advance().unwrap();
            }
            driver.inner.transports[0].failure = Some(failure);
            let batch = prepare_at(&mut pool, 192, 1);
            pool.begin_submission(&batch).unwrap();
            assert!(
                driver.execute_selected(&batch, &[0]).is_err(),
                "{failure:?}"
            );
            assert_eq!(
                driver.dispatch_counts(),
                [completed_packets as u64],
                "{failure:?}"
            );
            assert_eq!(driver.inner.collective, expected, "{failure:?}");
            assert_eq!(
                driver.inner.ranks[0].hidden.id,
                if residuals.is_multiple_of(2) {
                    original_hidden
                } else {
                    scratch.id
                }
            );
            assert!(driver.inner.transports[0].reads.is_empty());
            assert!(driver.poisoned && driver.completed_batches() == 0);
            assert!(driver.inner.packed_c1.is_none());
            assert!(driver.inner.ordered_batches.as_ref().unwrap().is_empty());
            let retained = driver.split_attention_workspace_v25.unwrap();
            assert_eq!(
                (retained.stats.id, retained.numerators.id),
                (workspace.stats.id, workspace.numerators.id)
            );
            assert!(driver.execute_selected(&batch, &[0]).is_err());
            pool.quarantine_batch(&batch).unwrap();
            driver.close().unwrap();
        }
    }
}

#[test]
fn c1_split_attention_v25_packing_selection_is_explicit_fresh_and_sealed() {
    for enabled in [false, true] {
        let mut driver = configured(&wide_context_pool());
        assert!(
            driver
                .configure_c1_split_packet_packing_v25(enabled)
                .is_err()
        );
        assert!(driver.c1_packet_packing_v22.is_none());
        select(&mut driver, true).unwrap();
        driver
            .configure_c1_split_packet_packing_v25(enabled)
            .unwrap();
        assert_eq!(
            driver.c1_packet_mode(),
            if enabled { "packed16-v22" } else { "baseline" }
        );
        assert!(
            driver
                .configure_c1_split_packet_packing_v25(enabled)
                .is_err()
        );
        assert!(
            driver
                .configure_c1_split_packet_packing_v25(!enabled)
                .is_err()
        );
        assert!(driver.configure_c1_packet_packing_v22(enabled).is_err());
        driver.close().unwrap();
    }
    for mutation in 0..8 {
        let mut driver = configured(&wide_context_pool());
        select(&mut driver, true).unwrap();
        match mutation {
            0 => driver.split_attention_workspace_v25 = None,
            1 => driver.last_batch = 1,
            2 => driver.completed_batches = 1,
            3 => driver.poisoned = true,
            4 => driver.inner.transports[0].ordered_supported = false,
            5 => driver.inner.sequences = Some(vec![Vec::new()]),
            6 => driver.inner.ordered_batches = None,
            7 => driver.admitted_query_hoist_v14 = None,
            _ => unreachable!(),
        }
        assert!(
            driver.configure_c1_split_packet_packing_v25(true).is_err(),
            "mutation {mutation}"
        );
        assert!(driver.c1_packet_packing_v22.is_none());
        assert!(driver.inner.transports[0].commands.is_empty());
        driver.close().unwrap();
    }
}

#[test]
fn c1_split_attention_v25_packing_rejects_wrong_residual_schedule_before_publication() {
    for split in [false, true] {
        let mut driver = configured(&wide_context_pool());
        select(&mut driver, split).unwrap();
        driver.configure_c1_split_packet_packing_v25(true).unwrap();
        driver.inner.hidden = vec![0; 4096];
        driver
            .inner
            .begin_packed_c1(if split {
                super::super::super::reduction::AttentionProducerSchedule::Split8V21
            } else {
                super::super::super::reduction::AttentionProducerSchedule::Baseline
            })
            .unwrap();
        driver.inner.packed_c1.as_mut().unwrap().producer_packets = if split { 10 } else { 11 };
        let collective = driver.inner.collective;
        let result = if split {
            driver
                .inner
                .reduce_device_tp1(0, Qwen3TensorParallelCollectiveV1::AttentionOutputSum)
        } else {
            driver.inner.reduce_split_attention_v25(0)
        };
        assert!(result.is_err());
        assert_eq!(driver.inner.collective, collective);
        assert!(groups(&driver).is_empty());
        assert!(driver.inner.ordered_batches.as_ref().unwrap().is_empty());
        driver.close().unwrap();
    }
}

#[test]
fn c1_split_attention_v25_packing_mixed_batches_reuse_scratch_across_rollover() {
    let mut pool = wide_context_pool();
    let mut driver = configured(&pool);
    select(&mut driver, true).unwrap();
    driver.configure_c1_split_packet_packing_v25(true).unwrap();
    driver.inner.transports[0].rollover_supported = true;
    let workspace = driver.split_attention_workspace_v25.unwrap();
    let allocations = driver.inner.transports[0].buffers.len();
    for (ordinal, (context, rows, selected)) in [
        (192, 1, vec![0]),
        (192, 16, Vec::new()),
        (257, 1, vec![0]),
        (192, 1, Vec::new()),
    ]
    .into_iter()
    .enumerate()
    {
        driver.inner.transports[0].queue_packets =
            fe2o3_kfd::engineering_wire::MAX_UNRETIRED_RING_PACKETS_V1 - 600;
        let batch = prepare_at(&mut pool, context, rows);
        pool.begin_submission(&batch).unwrap();
        let output = driver.execute_selected(&batch, &selected).unwrap();
        pool.commit_batch(&batch, output.completion).unwrap();
        pool.retire_sequence(batch.rows()[0].sequence(), false, 0)
            .unwrap();
        pool.check_invariants().unwrap();
        assert_eq!(driver.inner.collective.expected().epoch, ordinal as u64 + 1);
        assert!(driver.inner.packed_c1.is_none());
        assert!(driver.inner.ordered_batches.as_ref().unwrap().is_empty());
        let current = driver.split_attention_workspace_v25.unwrap();
        assert_eq!(
            (current.stats.id, current.numerators.id),
            (workspace.stats.id, workspace.numerators.id)
        );
    }
    let transport = &driver.inner.transports[0];
    assert_eq!(transport.buffers.len(), allocations);
    assert_eq!(transport.packet_preparations, [652, 616, 616, 652]);
    assert_eq!(transport.queue_epochs, 4);
    assert_eq!(driver.dispatch_counts(), [652 + 613 + 616 + 649]);
    driver.close().unwrap();
}

#[test]
fn c1_split_attention_v25_packing_readback_failure_keeps_completed_dispatches() {
    let mut pool = wide_context_pool();
    let mut driver = configured(&pool);
    select(&mut driver, true).unwrap();
    driver.configure_c1_split_packet_packing_v25(true).unwrap();
    driver.inner.transports[0].failure = Some(Failure::BadChoice);
    let batch = prepare_at(&mut pool, 192, 1);
    pool.begin_submission(&batch).unwrap();
    assert!(driver.execute_selected(&batch, &[0]).is_err());
    assert_eq!(driver.dispatch_counts(), [652]);
    assert_eq!(driver.inner.collective.expected().epoch, 1);
    assert!(driver.poisoned && driver.completed_batches() == 0);
    assert!(driver.inner.packed_c1.is_none());
    assert_eq!(driver.inner.transports[0].reads.len(), 1);
    pool.quarantine_batch(&batch).unwrap();
    driver.close().unwrap();
}
