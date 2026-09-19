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
