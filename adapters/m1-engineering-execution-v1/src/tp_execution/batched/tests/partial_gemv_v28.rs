//! Recording contracts only; the transport does not emulate V20 arithmetic.

use super::super::partial_gemv_v28::{admit, select_partial};
use super::*;
use crate::tp_artifact::{
    Fp32ArgmaxBindingV11, GemvPrefetchBindingV20, PrefillKvCopyBindingV27, QueryHoistBindingV14,
    SplitAttentionBindingV21, WaveRmsNormBindingV15,
};

const V20: &str = crate::tp_artifact::ENGINEERING_TP_GEMV_PREFETCH_EXPORTS_V20[1];
const V5: &str = "ferric_qwen3_tp_batch32_wave_gemv_partial_f32_v5";

fn configured(pool: &EngineeringTpPagedPoolV1) -> EngineeringTpBatchExecutionV2<Recording> {
    let mut driver = super::prefill_kv_copy_v28::composition_configured(pool);
    driver.admitted_partial_gemv_v20 = Some(GemvPrefetchBindingV20::recording());
    // Match the real admitted slice extents in both arms, sharing fake weights across layers.
    for (kind, elements) in [
        (Qwen3TensorKind::OutputProjection, 4096 * 4096),
        (Qwen3TensorKind::DownProjection, 4096 * 12_288),
    ] {
        let mut weight = driver.inner.ranks[0].layers[0].weight(kind);
        weight.elements = elements;
        let bytes = driver.inner.transports[0]
            .buffers
            .get_mut(&weight.id)
            .unwrap();
        bytes.resize(bytes.len().max(elements * 2), 0xa5);
        for layer in &mut driver.inner.ranks[0].layers {
            for (candidate, tensor) in &mut layer.weights {
                if *candidate == kind {
                    *tensor = weight;
                }
            }
        }
    }
    driver
}

fn select(
    driver: &mut EngineeringTpBatchExecutionV2<Recording>,
    copy: bool,
    split: bool,
    packed: bool,
    mode: EngineeringTpPartialGemvModeV28,
) -> TpResult<()> {
    driver.configure_partial_gemv_bindings_v28(
        Fp32ArgmaxBindingV11::recording(),
        QueryHoistBindingV14::recording(),
        WaveRmsNormBindingV15::recording(),
        PrefillKvCopyBindingV27::recording(),
        SplitAttentionBindingV21::recording(),
        GemvPrefetchBindingV20::recording(),
        copy,
        split,
        packed,
        mode,
    )
}

fn prepare_after_prompt(
    pool: &mut EngineeringTpPagedPoolV1,
    prompt_tokens: u32,
) -> EngineeringTpPreparedBatchV1 {
    let prompt = vec![1; usize::try_from(prompt_tokens).unwrap()];
    let sequence = pool
        .open_sequence(pool.scope(), &prompt, 0)
        .unwrap()
        .sequence();
    // Commit synthetic pool metadata only; these prefix rows make no numerical claim.
    for start in (0..prompt_tokens).step_by(32) {
        let rows = (start..(start + 32).min(prompt_tokens))
            .map(|position| EngineeringTpPageRowV1 {
                sequence,
                token: 1,
                position,
            })
            .collect::<Vec<_>>();
        let batch = pool.reserve_batch(&rows).unwrap();
        pool.begin_submission(&batch).unwrap();
        pool.commit_batch(
            &batch,
            crate::tp_paged::EngineeringTpBatchCompletionV1::after_all_ranks(&batch),
        )
        .unwrap();
    }
    pool.reserve_batch(&[EngineeringTpPageRowV1 {
        sequence,
        token: 42,
        position: prompt_tokens,
    }])
    .unwrap()
}

#[test]
fn partial_v28_image_admission_requires_exact_loaded_nonpeer_tp1_before_allocation() {
    for mutation in 0..5 {
        let mut driver = fixture(if mutation == 4 { 2 } else { 1 }, &wide_pool());
        for transport in &mut driver.inner.transports {
            transport.buffers.clear();
            transport.partial_gemv_v20_loaded = Some(GemvPrefetchBindingV20::recording().hsaco);
        }
        match mutation {
            1 => driver.inner.transports[0].partial_gemv_v20_loaded = None,
            2 => driver.inner.transports[0].partial_gemv_v20_loaded = Some([0; 32]),
            3 => driver.inner.transports[0].argmax_peer = Some((0, 1, 1)),
            _ => {}
        }
        assert_eq!(
            admit(
                &mut driver.inner.transports,
                GemvPrefetchBindingV20::recording()
            )
            .is_ok(),
            mutation == 0
        );
        assert!(
            driver
                .inner
                .transports
                .iter()
                .all(|transport| transport.commands.is_empty())
        );
    }
    assert!(admit::<Recording>(&mut [], GemvPrefetchBindingV20::recording()).is_err());
    let mut driver = fixture(1, &wide_pool());
    driver.inner.transports[0].partial_gemv_v20_loaded =
        Some(GemvPrefetchBindingV20::recording().hsaco);
    assert!(
        admit(
            &mut driver.inner.transports,
            GemvPrefetchBindingV20::recording()
        )
        .is_err()
    );
}

#[test]
fn partial_v28_selection_is_fresh_atomic_and_composes_all_existing_modes() {
    for copy in [false, true] {
        for split in [false, true] {
            for packed in [false, true] {
                for enabled in [false, true] {
                    let mode = EngineeringTpPartialGemvModeV28::from(enabled);
                    assert_eq!(
                        mode,
                        if enabled {
                            EngineeringTpPartialGemvModeV28::Prefetch4
                        } else {
                            EngineeringTpPartialGemvModeV28::Baseline
                        }
                    );
                    let mut driver =
                        super::prefill_kv_copy_v28::composition_configured(&wide_pool());
                    assert!(select(&mut driver, copy, split, packed, mode).is_err());
                    assert!(driver.prefill_kv_copy_v28.is_none());
                    driver.admitted_partial_gemv_v20 = Some(GemvPrefetchBindingV20::recording());
                    select(&mut driver, copy, split, packed, mode).unwrap();
                    assert_eq!(
                        driver.partial_gemv_mode(),
                        if enabled {
                            "partial-prefetch4-v20"
                        } else {
                            "baseline"
                        }
                    );
                    assert_eq!(driver.prefill_kv_copy_v28, Some(copy));
                    assert_eq!(driver.c1_split_attention_v25, Some(split));
                    assert_eq!(driver.c1_packet_packing_v22, Some(packed));
                    assert!(select(&mut driver, copy, split, packed, (!enabled).into()).is_err());
                    assert_eq!(driver.partial_gemv_v28, Some(enabled));
                }
            }
        }
    }
    let mut driver = super::prefill_kv_copy_v28::composition_configured(&wide_pool());
    driver.admitted_partial_gemv_v20 = Some(GemvPrefetchBindingV20::recording());
    driver.admitted_prefill_kv_copy_v27 = None;
    assert!(
        select(
            &mut driver,
            true,
            true,
            true,
            EngineeringTpPartialGemvModeV28::Prefetch4
        )
        .is_err()
    );
    assert!(driver.partial_gemv_v28.is_none() && driver.prefill_kv_copy_v28.is_none());
}

#[test]
fn partial_v28_changes_only_72_single_row_roots_and_preserves_all_other_command_bytes() {
    use crate::tp_artifact::ENGINEERING_TP_SPLIT_ATTENTION_EXPORTS_V21 as SPLIT;

    for (rows, prefix, split) in [
        (1, 0, false),
        (16, 0, false),
        (17, 0, false),
        (32, 0, false),
        (1, 126, false),
        (1, 127, true),
        (1, 128, true),
        (1, 255, true),
        (1, 256, false),
    ] {
        for publish in [false, true] {
            let mut reference = None;
            for enabled in [false, true] {
                let limits = EngineeringTpPagedLimitsV1::new(512, 32, 32, 100).unwrap();
                let mut pool =
                    EngineeringTpPagedPoolV1::new_wide32(wide_pool().scope(), limits).unwrap();
                let mut driver = configured(&pool);
                select(&mut driver, true, true, true, enabled.into()).unwrap();
                let allocations = driver.inner.transports[0]
                    .buffers
                    .iter()
                    .map(|(&id, bytes)| (id, bytes.len()))
                    .collect::<Vec<_>>();
                let batch = if prefix == 0 {
                    prepare(&mut pool, rows)
                } else {
                    prepare_after_prompt(&mut pool, prefix)
                };
                assert_eq!(batch.rows()[0].position(), prefix);
                let selected = if publish {
                    vec![usize::try_from(rows - 1).unwrap()]
                } else {
                    vec![]
                };
                let expected = if split {
                    if publish { 652 } else { 649 }
                } else if publish {
                    616
                } else {
                    613
                };
                assert_eq!(
                    driver
                        .expected_dispatch_counts_for_batch(&batch, selected.len())
                        .unwrap(),
                    [expected]
                );
                pool.begin_submission(&batch).unwrap();
                let output = driver.execute_selected(&batch, &selected).unwrap();
                let transport = &driver.inner.transports[0];
                let mut commands = transport.commands.clone();
                for kernel in SPLIT {
                    assert_eq!(
                        commands
                            .iter()
                            .filter(|command| command.kernel == kernel)
                            .count(),
                        if split { 36 } else { 0 }
                    );
                }
                let changed = commands
                    .iter()
                    .filter(|command| command.kernel == V20)
                    .count();
                assert_eq!(changed, if enabled && rows == 1 { 72 } else { 0 });
                for command in commands.iter_mut().filter(|command| command.kernel == V20) {
                    assert_eq!(scalar(command, 3), 1);
                    assert!(matches!(
                        (scalar(command, 4), scalar(command, 5), scalar(command, 7)),
                        (4096, 4096, 1) | (4096, 12_288, 2)
                    ));
                    command.kernel = V5;
                }
                assert_eq!(commands.len(), usize::try_from(expected).unwrap());
                assert_eq!(
                    transport
                        .buffers
                        .iter()
                        .map(|(&id, bytes)| (id, bytes.len()))
                        .collect::<Vec<_>>(),
                    allocations
                );
                let groups = transport
                    .events
                    .borrow()
                    .iter()
                    .filter_map(|event| match event {
                        Event::OrderedSubmit(_, count) | Event::OrderedWait(_, count) => {
                            Some(*count)
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                let observed = (
                    commands,
                    output.choices.clone(),
                    transport.reads.clone(),
                    transport.write_payloads.clone(),
                    transport.packet_preparations.clone(),
                    groups,
                );
                if let Some(expected) = &reference {
                    assert_eq!(&observed, expected);
                } else {
                    reference = Some(observed);
                }
                pool.commit_batch(&batch, output.completion).unwrap();
                pool.check_invariants().unwrap();
                driver.close().unwrap();
            }
        }
    }
}

#[test]
fn partial_v28_one_token_prefill_tail_uses_the_same_single_row_route() {
    use crate::tp_artifact::ENGINEERING_TP_SPLIT_ATTENTION_EXPORTS_V21 as SPLIT;

    let mut pool = wide_pool();
    let mut driver = configured(&pool);
    select(
        &mut driver,
        true,
        true,
        true,
        EngineeringTpPartialGemvModeV28::Prefetch4,
    )
    .unwrap();
    let sequence = pool
        .open_sequence(pool.scope(), &(0..17).collect::<Vec<_>>(), 0)
        .unwrap()
        .sequence();
    for (start, rows, expected_partials) in [(0, 16, 0), (16, 1, 72)] {
        let batch = pool
            .reserve_batch(
                &(start..start + rows)
                    .map(|position| EngineeringTpPageRowV1 {
                        sequence,
                        token: position,
                        position,
                    })
                    .collect::<Vec<_>>(),
            )
            .unwrap();
        let before = driver.inner.transports[0].commands.len();
        pool.begin_submission(&batch).unwrap();
        let selected = if rows == 1 { vec![0] } else { vec![] };
        let expected_packets = if rows == 1 { 616 } else { 613 };
        assert_eq!(
            driver
                .expected_dispatch_counts_for_batch(&batch, selected.len())
                .unwrap(),
            [expected_packets]
        );
        let output = driver.execute_selected(&batch, &selected).unwrap();
        let commands = &driver.inner.transports[0].commands[before..];
        assert_eq!(commands.len(), usize::try_from(expected_packets).unwrap());
        assert_eq!(
            commands
                .iter()
                .filter(|command| command.kernel == V20)
                .count(),
            expected_partials
        );
        assert!(
            !commands
                .iter()
                .any(|command| SPLIT.contains(&command.kernel))
        );
        pool.commit_batch(&batch, output.completion).unwrap();
    }
    assert_eq!(driver.completed_batches, 2);
    assert_eq!(driver.dispatch_counts(), [613 + 616]);
    driver.close().unwrap();
}

#[test]
fn partial_v28_submit_wait_and_nonfinite_failures_never_commit_or_replay() {
    for failure in [
        Failure::PartialGemvSubmit,
        Failure::PartialGemvWait,
        Failure::NonfinitePartial,
    ] {
        let mut pool = wide_pool();
        let mut driver = configured(&pool);
        select(
            &mut driver,
            true,
            true,
            true,
            EngineeringTpPartialGemvModeV28::Prefetch4,
        )
        .unwrap();
        driver.inner.transports[0].failure = Some(failure);
        let batch = prepare(&mut pool, 1);
        pool.begin_submission(&batch).unwrap();
        assert!(driver.execute_selected(&batch, &[0]).is_err());
        assert_eq!(driver.completed_batches, 0);
        let commands = driver.inner.transports[0].commands.len();
        assert!(driver.execute_selected(&batch, &[0]).is_err());
        assert_eq!(driver.inner.transports[0].commands.len(), commands);
        assert!(pool.begin_submission(&batch).is_err());
        driver.close().unwrap();
    }
}

#[test]
fn partial_v28_selector_does_not_replace_bf16_head_multirow_or_unselected_commands() {
    for kernel in [
        "ferric_qwen3_tp_wave_gemv_partial_f32_v3",
        "ferric_qwen3_tp_wave_gemv_bf16_v3",
        "ferric_qwen3_tp_mfma_head_f32_v7",
    ] {
        for rows in [1, 2, 16] {
            let tensor = Tensor {
                id: 1,
                elements: 1,
                element_bytes: 2,
            };
            let command = matrix(kernel, tensor, tensor, tensor, [rows, 4096, 4096, 1, 1]);
            assert_eq!(select_partial(false, command.clone()), command);
            let mut expected = command.clone();
            if kernel == "ferric_qwen3_tp_wave_gemv_partial_f32_v3" && rows == 1 {
                expected.kernel = V20;
            }
            assert_eq!(select_partial(true, command), expected);
        }
    }
}
