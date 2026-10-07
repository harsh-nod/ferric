//! Source/recording contracts, not native V27 or full-model numerical evidence.

use super::*;
use crate::tp_artifact::{
    C1KvCopyBindingV19, Fp32ArgmaxBindingV11, GemvPrefetchBindingV20, PrefillKvCopyBindingV27,
    QueryHoistBindingV14, SplitAttentionBindingV21, WaveRmsNormBindingV15,
};

fn configured(pool: &EngineeringTpPagedPoolV1) -> EngineeringTpBatchExecutionV2<Recording> {
    let mut driver = super::prefill_kv_copy_v28::composition_configured(pool);
    driver.admitted_c1_kv_copy_v19 = Some(C1KvCopyBindingV19::recording());
    driver.admitted_partial_gemv_v20 = Some(GemvPrefetchBindingV20::recording());
    driver.inner.transports[0].ordered64_supported = true;
    driver.inner.transports[0].rollover_supported = true;
    driver
}

fn select_kv(driver: &mut EngineeringTpBatchExecutionV2<Recording>, copy: bool) -> TpResult<()> {
    driver.configure_ordered64_kv_copy_bindings_v1(
        Fp32ArgmaxBindingV11::recording(),
        QueryHoistBindingV14::recording(),
        WaveRmsNormBindingV15::recording(),
        PrefillKvCopyBindingV27::recording(),
        SplitAttentionBindingV21::recording(),
        GemvPrefetchBindingV20::recording(),
        C1KvCopyBindingV19::recording(),
        copy,
    )
}

#[test]
fn prefill32_page_pair_selector_build_matrix_does_not_mutate_other_profiles() {
    let mut driver = configured(&wide_pool());
    let prior = select_kv(&mut driver, true);
    if cfg!(all(
        feature = "c1-ordered64",
        not(feature = "model-timestamps")
    )) {
        prior.unwrap();
    }
    let result =
        driver.configure_prefill32_pages_binding_v1(PrefillKvCopyBindingV27::recording(), true);
    assert_eq!(
        result.is_ok(),
        cfg!(all(
            feature = "c1-ordered64",
            not(feature = "model-timestamps")
        ))
    );
    if result.is_err() {
        assert!(driver.prefill32_pages_v1.is_none());
        assert!(driver.inner.transports[0].commands.is_empty());
    }
}

#[cfg(all(feature = "c1-ordered64", not(feature = "model-timestamps")))]
mod enabled {
    use super::*;
    const COPY: &str = crate::tp_artifact::ENGINEERING_TP_PREFILL_KV_COPY_EXPORTS_V27[0];
    const C1: &str = crate::tp_artifact::ENGINEERING_TP_C1_KV_COPY_EXPORTS_V19[0];
    const LEGACY: &str = "ferric_qwen3_tp_batch32_paged_kv_append_v5";

    fn selected(
        pool: &EngineeringTpPagedPoolV1,
        pair: bool,
        decode: bool,
    ) -> EngineeringTpBatchExecutionV2<Recording> {
        let mut driver = configured(pool);
        select_kv(&mut driver, decode).unwrap();
        driver
            .configure_prefill32_pages_binding_v1(PrefillKvCopyBindingV27::recording(), pair)
            .unwrap();
        driver
    }

    #[test]
    fn prefill32_page_pair_explicit_control_and_candidate_preserve_decode_selection_and_allocations()
     {
        for pair in [false, true] {
            for decode in [false, true] {
                let mut driver = configured(&wide_pool());
                select_kv(&mut driver, decode).unwrap();
                let allocations = driver.inner.transports[0]
                    .buffers
                    .keys()
                    .copied()
                    .collect::<Vec<_>>();
                assert_eq!(driver.prefill32_pages_mode(), "baseline");
                driver
                    .configure_prefill32_pages_binding_v1(
                        PrefillKvCopyBindingV27::recording(),
                        pair,
                    )
                    .unwrap();
                assert_eq!(driver.prefill32_pages_v1, Some(pair));
                assert_eq!(driver.c1_kv_copy_v19.is_some(), decode);
                assert_eq!(driver.prefill_kv_mode(), "parallel-prefill16-v27");
                assert_eq!(driver.c1_packet_mode(), "packed64-v29");
                assert_eq!(driver.split_attention_mode(), "split8-v21");
                assert_eq!(driver.partial_gemv_mode(), "baseline");
                assert_eq!(
                    driver.prefill32_pages_mode(),
                    if pair {
                        "parallel-prefill32-two-pages-v27"
                    } else {
                        "baseline"
                    }
                );
                assert!(
                    driver
                        .configure_prefill32_pages_binding_v1(
                            PrefillKvCopyBindingV27::recording(),
                            pair
                        )
                        .is_err()
                );
                assert_eq!(
                    driver.inner.transports[0]
                        .buffers
                        .keys()
                        .copied()
                        .collect::<Vec<_>>(),
                    allocations
                );
                assert!(driver.inner.transports[0].commands.is_empty());
            }
        }
    }

    #[test]
    fn prefill32_page_pair_selection_rejects_drift_before_mutation() {
        for mutation in 0..28 {
            let mut driver = configured(&wide_pool());
            select_kv(&mut driver, true).unwrap();
            let mut binding = PrefillKvCopyBindingV27::recording();
            match mutation {
                0 => binding.hsaco[0] ^= 1,
                1 => driver.last_batch = 1,
                2 => driver.completed_batches = 1,
                3 => driver.poisoned = true,
                4 => driver.inner.closed = true,
                5 => driver.inner.large_kv = true,
                6 => driver.row_capacity = 16,
                7 => driver.inner.row_capacity = 16,
                8 => driver.admitted_prefill_kv_copy_v27 = None,
                9 => driver.prefill_kv_copy_v28 = Some(false),
                10 => driver.c1_split_attention_v25 = Some(false),
                11 => driver.split_attention_workspace_v25 = None,
                12 => driver.partial_gemv_v28 = Some(true),
                13 => driver.admitted_partial_gemv_v20 = None,
                14 => driver.admitted_c1_kv_copy_v19 = None,
                15 => driver.c1_packet_packing_v22 = Some(false),
                16 => driver.inner.transports[0].ordered64_supported = false,
                17 => driver.inner.transports[0].argmax_peer = Some((1, 0, 1)),
                18 => driver.inner.ranks[0].dispatches = 1,
                19 => driver.inner.ordered_batches = None,
                20 => driver.prune_output_head = false,
                21 => driver.query_hoist_v14 = None,
                22 => driver.wave_rmsnorm_v15 = None,
                23 => driver.inner.ranks[0].k_rotated.elements -= 1,
                24 => driver.inner.ranks[0].layers[35].v_cache.elements -= 1,
                25 => driver.context_tokens = 31,
                26 => driver.physical_pages = 1,
                27 => {
                    driver.inner.ranks[0].layers[35].v_cache.id =
                        driver.inner.ranks[0].k_rotated.id;
                }
                _ => unreachable!(),
            }
            assert!(
                driver
                    .configure_prefill32_pages_binding_v1(binding, true)
                    .is_err(),
                "mutation {mutation}"
            );
            assert!(driver.prefill32_pages_v1.is_none());
            assert!(driver.inner.transports[0].commands.is_empty());
        }
    }

    #[test]
    fn prefill32_page_pair_uses_actual_prepared_pages_and_terminal_physical_row_zero() {
        let mut pool = wide_pool();
        let driver = selected(&pool, true, true);
        let batch = prepare(&mut pool, 32);
        for terminal in [false, true] {
            let output = if terminal { vec![31] } else { vec![] };
            let plan = driver.prefill32_plan_v1(&batch, &output).unwrap().unwrap();
            let order = plan.execution_order();
            assert_eq!(
                order,
                if terminal {
                    std::iter::once(31)
                        .chain(16..31)
                        .chain(0..16)
                        .collect::<Vec<_>>()
                } else {
                    (0..32).collect()
                }
            );
            let commands = plan.commands(&driver.inner.ranks[0], 0).unwrap();
            for (half, command) in commands.iter().enumerate() {
                let logical = if terminal { 1 - half } else { half };
                let row = &batch.rows()[logical * 16];
                assert_eq!(buffer(command, 0).1, half * 32_768);
                assert_eq!(buffer(command, 1).1, half * 32_768);
                assert_eq!(
                    buffer(command, 2).1,
                    row.writable_physical_page() as usize * 32_768
                );
                assert_eq!(buffer(command, 3).1, buffer(command, 2).1);
                assert_eq!(
                    (scalar(command, 4), scalar(command, 5)),
                    (row.position(), row.writable_physical_page())
                );
                assert_eq!(scalar(command, 7), u32::from(terminal && half == 0));
                assert_eq!(buffer(command, 0).2, 16_384);
            }
            assert_ne!(buffer(&commands[0], 2).1, buffer(&commands[1], 2).1);
        }
        // Advance real pool reservations using synthetic host completions only;
        // no GPU tensor allocation or numerical boundary evidence is implied.
        let limits = EngineeringTpPagedLimitsV1::new(8192, 32, 512, 100).unwrap();
        let mut boundary_pool =
            EngineeringTpPagedPoolV1::new_wide32(wide_pool().scope(), limits).unwrap();
        let sequence = boundary_pool
            .open_sequence(boundary_pool.scope(), &vec![1; 8192], 0)
            .unwrap()
            .sequence();
        for first in (0..8160).step_by(32) {
            let batch = boundary_pool
                .reserve_batch(
                    &(first..first + 32)
                        .map(|position| EngineeringTpPageRowV1 {
                            sequence,
                            token: 1,
                            position,
                        })
                        .collect::<Vec<_>>(),
                )
                .unwrap();
            boundary_pool.begin_submission(&batch).unwrap();
            boundary_pool
                .commit_batch(
                    &batch,
                    crate::tp_paged::EngineeringTpBatchCompletionV1::after_all_ranks(&batch),
                )
                .unwrap();
        }
        let boundary = boundary_pool
            .reserve_batch(
                &(8160..8192)
                    .map(|position| EngineeringTpPageRowV1 {
                        sequence,
                        token: 1,
                        position,
                    })
                    .collect::<Vec<_>>(),
            )
            .unwrap();
        let select = super::super::super::prefill32_pages_v1::PrefillPagePairV1::select;
        for output in [vec![], vec![31]] {
            let plan = select(&boundary, &output, 512, 8192).unwrap().unwrap();
            assert_eq!(
                plan.execution_order()[0],
                if output.is_empty() { 0 } else { 31 }
            );
            assert!(select(&boundary, &output, 512, 8191).is_err());
            assert!(select(&boundary, &output, 511, 8192).is_err());
        }
        boundary_pool.abort_batch(&boundary).unwrap();
        boundary_pool.check_invariants().unwrap();
    }

    #[test]
    fn prefill32_page_pair_copy_model_covers_all_bf16_bits_and_preserves_other_pages() {
        let mut pool = wide_pool();
        let mut driver = selected(&pool, true, true);
        let batch = prepare(&mut pool, 32);
        for output in [vec![], vec![31]] {
            let plan = driver.prefill32_plan_v1(&batch, &output).unwrap().unwrap();
            let order = plan.execution_order();
            let commands = plan.commands(&driver.inner.ranks[0], 0).unwrap();
            let transport = &mut driver.inner.transports[0];
            for high in [0_u16, 0x8000] {
                for (source, destination) in [(0, 2), (1, 3)] {
                    let input = buffer(&commands[0], source).0;
                    let target = buffer(&commands[0], destination).0;
                    for (physical, logical) in order.iter().copied().enumerate() {
                        let bytes = (0..1024)
                            .flat_map(|channel| {
                                let value = u16::try_from(logical * 1024 + channel).unwrap() | high;
                                (value ^ if source == 0 { 0 } else { 0xa55a }).to_le_bytes()
                            })
                            .collect::<Vec<_>>();
                        transport.buffers.get_mut(&input).unwrap()
                            [physical * 2048..(physical + 1) * 2048]
                            .copy_from_slice(&bytes);
                    }
                    transport.buffers.get_mut(&target).unwrap().fill(0x5a);
                }
                for command in &commands {
                    transport.submit(command).unwrap();
                    transport.wait().unwrap();
                }
                for (source, destination) in [(0, 2), (1, 3)] {
                    let target = buffer(&commands[0], destination).0;
                    let actual = &transport.buffers[&target];
                    let mut expected = vec![0x5a; actual.len()];
                    for (logical, row) in batch.rows().iter().enumerate() {
                        let start = row.writable_physical_page() as usize * 32_768
                            + row.writable_token_offset() as usize * 2048;
                        let bytes = (0..1024)
                            .flat_map(|channel| {
                                let value = u16::try_from(logical * 1024 + channel).unwrap() | high;
                                (value ^ if source == 0 { 0 } else { 0xa55a }).to_le_bytes()
                            })
                            .collect::<Vec<_>>();
                        expected[start..start + 2048].copy_from_slice(&bytes);
                    }
                    assert_eq!(actual, &expected);
                }
            }
        }
    }

    #[test]
    fn prefill32_page_pair_half_binder_is_separate_and_rejects_all_view_drift() {
        let mut pool = wide_pool();
        let driver = selected(&pool, true, true);
        let batch = prepare(&mut pool, 32);
        let plan = driver.prefill32_plan_v1(&batch, &[]).unwrap().unwrap();
        let commands = plan.commands(&driver.inner.ranks[0], 0).unwrap();
        let bind = super::super::super::super::row_profile::bind_prefill_page_half_v1;
        let legacy = super::super::super::super::row_profile::bind_storage;
        assert!(legacy(32, false, commands[0].clone()).is_ok());
        assert!(legacy(32, false, commands[1].clone()).is_err());
        for command in &commands {
            assert_eq!(
                bind(32, false, [32_768; 2], command.clone()).unwrap(),
                *command
            );
            assert!(bind(16, false, [32_768; 2], command.clone()).is_err());
            assert!(bind(32, true, [32_768; 2], command.clone()).is_err());
            assert!(bind(32, false, [16_384, 32_768], command.clone()).is_err());
        }
        for mutation in 0..14 {
            let mut command = commands[1].clone();
            match mutation {
                0..=8 => {
                    let index = if mutation == 2 {
                        1
                    } else if mutation == 6 {
                        2
                    } else {
                        0
                    };
                    let EngineeringTpArgumentV1::Buffer {
                        id,
                        offset,
                        elements,
                        element_bytes,
                        access,
                    } = &mut command.arguments[index]
                    else {
                        unreachable!()
                    };
                    match mutation {
                        0 => *offset = 2,
                        1 => *offset = 65_536,
                        2 => *offset = 0,
                        3 => *elements = 16_383,
                        4 => *element_bytes = 4,
                        5 => *access = EngineeringTpBufferAccessV1::Write,
                        6 => *offset += 2,
                        7 => *id = 0,
                        8 => *id = buffer(&commands[1], 1).0,
                        _ => unreachable!(),
                    }
                }
                9 => command.arguments[4] = EngineeringTpArgumentV1::U32(17),
                10 => command.arguments[7] = EngineeringTpArgumentV1::U32(2),
                11 => command.grid_workgroups = 255,
                12 => command.kernel = LEGACY,
                13 => {
                    command.arguments.pop();
                }
                _ => unreachable!(),
            }
            assert!(
                bind(32, false, [32_768; 2], command).is_err(),
                "mutation {mutation}"
            );
        }
    }

    #[test]
    fn prefill32_page_pair_metadata_order_and_all_forward_consumers_are_consistent() {
        for output in [vec![], vec![31]] {
            let mut pool = wide_pool();
            let mut driver = selected(&pool, true, true);
            let batch = prepare(&mut pool, 32);
            let order = driver
                .prefill32_plan_v1(&batch, &output)
                .unwrap()
                .unwrap()
                .execution_order();
            let expected = if output.is_empty() { 649 } else { 652 };
            assert_eq!(
                driver
                    .expected_dispatch_counts_for_selection(&batch, &output)
                    .unwrap(),
                [expected]
            );
            assert!(
                driver
                    .expected_dispatch_counts_for_batch(&batch, output.len())
                    .is_err()
            );
            pool.begin_submission(&batch).unwrap();
            let result = driver.execute_selected(&batch, &output).unwrap();
            let rank = &driver.inner.ranks[0];
            let transport = &driver.inner.transports[0];
            assert_eq!(transport.packet_preparations, [652]);
            assert_eq!(transport.commands.len(), usize::try_from(expected).unwrap());
            assert_eq!(
                transport.u32_values(rank.token.id, 32),
                order
                    .iter()
                    .map(|row| batch.rows()[*row].token())
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                transport.u32_values(driver.positions[0].id, 32),
                order
                    .iter()
                    .map(|row| batch.rows()[*row].position())
                    .collect::<Vec<_>>()
            );
            let tables =
                transport.u32_values(driver.page_tables[0].id, 32 * driver.table_stride as usize);
            for (physical, &logical) in order.iter().enumerate() {
                let row = &batch.rows()[logical];
                let start = physical * driver.table_stride as usize;
                assert_eq!(
                    &tables[start..start + row.physical_pages().len()],
                    row.physical_pages()
                );
                let (cos, sin) = rope_bytes(row.position(), target().rope_theta);
                assert_eq!(
                    &transport.buffers[&rank.cos.id][physical * 256..(physical + 1) * 256],
                    &cos
                );
                assert_eq!(
                    &transport.buffers[&rank.sin.id][physical * 256..(physical + 1) * 256],
                    &sin
                );
            }
            let groups = transport
                .events
                .borrow()
                .iter()
                .filter_map(|event| match event {
                    Event::OrderedSubmit(0, count) => Some(*count),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(groups, [12, 6].repeat(36));
            assert_eq!(
                transport
                    .commands
                    .iter()
                    .filter(|command| command.kernel == COPY)
                    .count(),
                72
            );
            assert!(
                !transport
                    .commands
                    .iter()
                    .any(|command| command.kernel == LEGACY || command.kernel == C1)
            );
            for command in &transport.commands {
                if command.kernel == "ferric_qwen3_tp_batch32_mfma_gemm_bf16_v5"
                    || command.kernel == "ferric_qwen3_tp_batch32_mfma_gemm_partial_f32_v5"
                {
                    assert_eq!(scalar(command, 3), 32);
                }
                if command.kernel == "ferric_qwen3_tp_batch32_rope_v5" {
                    assert_eq!(scalar(command, 7), 32);
                }
            }
            assert_eq!(result.choices.len(), output.len());
            if !output.is_empty() {
                let head = &transport.commands[transport.commands.len() - 3..];
                assert_eq!(buffer(&head[0], 0).1, 0);
                assert_eq!(scalar(&head[0], 5), 1);
                assert_eq!(scalar(&head[1], 3), 1);
                assert_eq!(scalar(&head[2], 2), 1);
            }
            assert!(driver.inner.packed_c1.is_none());
            assert!(driver.inner.ordered_batches.as_ref().unwrap().is_empty());
            pool.commit_batch(&batch, result.completion).unwrap();
            pool.check_invariants().unwrap();
        }
    }

    #[test]
    fn prefill32_page_pair_invalid_selection_or_late_layer_storage_fails_before_preparation() {
        for output in [
            vec![7],
            vec![0],
            vec![30],
            vec![0, 31],
            vec![31, 31],
            vec![32],
        ] {
            let mut pool = wide_pool();
            let mut driver = selected(&pool, true, true);
            let batch = prepare(&mut pool, 32);
            let before = driver.inner.transports[0].write_payloads.len();
            assert!(
                driver
                    .expected_dispatch_counts_for_selection(&batch, &output)
                    .is_err()
            );
            assert!(driver.execute_selected(&batch, &output).is_err());
            assert!(driver.inner.transports[0].packet_preparations.is_empty());
            assert!(driver.inner.transports[0].commands.is_empty());
            assert_eq!(driver.inner.transports[0].write_payloads.len(), before);
            assert_eq!(driver.last_batch, 0);
        }
        for alias in [false, true] {
            let mut pool = wide_pool();
            let mut driver = selected(&pool, true, true);
            let batch = prepare(&mut pool, 32);
            if alias {
                driver.inner.ranks[0].layers[35].v_cache.id = driver.inner.ranks[0].k_rotated.id;
            } else {
                driver.inner.ranks[0].layers[35].v_cache.elements -= 1;
            }
            assert!(
                driver
                    .expected_dispatch_counts_for_selection(&batch, &[31])
                    .is_err()
            );
            assert!(driver.execute_selected(&batch, &[31]).is_err());
            assert!(driver.inner.transports[0].packet_preparations.is_empty());
            assert!(driver.inner.transports[0].commands.is_empty());
        }
    }

    #[test]
    fn prefill32_page_pair_mixed_sequence_and_control_keep_legacy_schedule() {
        for pair in [false, true] {
            let mut pool = wide_pool();
            let mut driver = selected(&pool, pair, true);
            let mut rows = Vec::new();
            for _ in 0..2 {
                let sequence = pool
                    .open_sequence(pool.scope(), &(0..16).collect::<Vec<_>>(), 0)
                    .unwrap()
                    .sequence();
                rows.extend((0..16).map(|position| EngineeringTpPageRowV1 {
                    sequence,
                    token: position,
                    position,
                }));
            }
            let batch = pool.reserve_batch(&rows).unwrap();
            assert!(driver.prefill32_plan_v1(&batch, &[]).unwrap().is_none());
            assert_eq!(
                driver
                    .expected_dispatch_counts_for_selection(&batch, &[])
                    .unwrap(),
                [613]
            );
            pool.begin_submission(&batch).unwrap();
            let result = driver.execute_selected(&batch, &[]).unwrap();
            let transport = &driver.inner.transports[0];
            assert_eq!(transport.packet_preparations, [616]);
            assert_eq!(
                transport
                    .commands
                    .iter()
                    .filter(|command| command.kernel == LEGACY)
                    .count(),
                36
            );
            assert!(
                !transport
                    .commands
                    .iter()
                    .any(|command| command.kernel == COPY)
            );
            let groups = transport
                .events
                .borrow()
                .iter()
                .filter_map(|event| match event {
                    Event::OrderedSubmit(0, count) => Some(*count),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(groups, [11, 6].repeat(36));
            pool.commit_batch(&batch, result.completion).unwrap();
        }
    }

    #[test]
    fn prefill32_page_pair_complete_request_has_exact_counts_and_unchanged_v19_decode() {
        for pair in [false, true] {
            let limits = EngineeringTpPagedLimitsV1::new(256, 32, 16, 100).unwrap();
            let mut pool =
                EngineeringTpPagedPoolV1::new_wide32(wide_pool().scope(), limits).unwrap();
            let mut driver = selected(&pool, pair, true);
            let sequence = pool
                .open_sequence(pool.scope(), &(0..256).collect::<Vec<_>>(), 0)
                .unwrap()
                .sequence();
            let (mut packets, mut grouped, mut groups, mut copies, mut decode_copies) =
                (0, 0, 0, 0, 0);
            for (first, count) in (0..128)
                .step_by(32)
                .map(|first| (first, 32))
                .chain((128..255).map(|first| (first, 1)))
            {
                let batch = pool
                    .reserve_batch(
                        &(first..first + count)
                            .map(|position| EngineeringTpPageRowV1 {
                                sequence,
                                token: position,
                                position,
                            })
                            .collect::<Vec<_>>(),
                    )
                    .unwrap();
                let output = if first < 96 {
                    vec![]
                } else {
                    vec![count as usize - 1]
                };
                let expected = if count == 1 {
                    652
                } else if output.is_empty() {
                    if pair { 649 } else { 613 }
                } else if pair {
                    652
                } else {
                    616
                };
                assert_eq!(
                    driver
                        .expected_dispatch_counts_for_selection(&batch, &output)
                        .unwrap(),
                    [expected]
                );
                pool.begin_submission(&batch).unwrap();
                // This request alone stays below the real ring limit. Exercise
                // preparation rollover at both the prefill and decode boundary.
                if first == 0 || first == 128 {
                    driver.inner.transports[0].queue_packets =
                        fe2o3_kfd::engineering_wire::MAX_UNRETIRED_RING_PACKETS_V1 - 600;
                }
                let result = driver.execute_selected(&batch, &output).unwrap();
                let transport = &mut driver.inner.transports[0];
                assert_eq!(transport.commands.len(), usize::try_from(expected).unwrap());
                assert_eq!(
                    *transport.packet_preparations.last().unwrap(),
                    if count == 1 || pair { 652 } else { 616 }
                );
                let sizes = transport
                    .events
                    .borrow()
                    .iter()
                    .filter_map(|event| match event {
                        Event::OrderedSubmit(0, count) => Some(*count),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                assert_eq!(
                    sizes,
                    if count == 1 {
                        [vec![64; 10], vec![12]].concat()
                    } else {
                        [if pair { 12 } else { 11 }, 6].repeat(36)
                    }
                );
                copies += transport
                    .commands
                    .iter()
                    .filter(|command| command.kernel == COPY)
                    .count();
                decode_copies += transport
                    .commands
                    .iter()
                    .filter(|command| command.kernel == C1)
                    .count();
                packets += transport.commands.len();
                grouped += sizes.iter().sum::<usize>();
                groups += sizes.len();
                transport.commands.clear();
                transport.events.borrow_mut().clear();
                pool.commit_batch(&batch, result.completion).unwrap();
            }
            assert_eq!(driver.completed_batches(), 131);
            assert_eq!(packets, if pair { 85_403 } else { 85_259 });
            assert_eq!(groups, 1_685);
            assert_eq!(grouped, packets - 7);
            assert_eq!(copies, if pair { 288 } else { 0 });
            assert_eq!(decode_copies, 127 * 36);
            assert_eq!(driver.dispatch_counts(), [packets as u64]);
            assert_eq!(driver.inner.transports[0].queue_epochs, 2);
            assert_eq!(driver.inner.transports[0].queue_packets, 127 * 652);
            pool.check_invariants().unwrap();
        }
    }

    #[test]
    fn prefill32_page_pair_unaligned_fallback_and_foreign_stale_batches_fail_closed() {
        let mut pool = wide_pool();
        let mut driver = selected(&pool, true, true);
        let sequence = pool
            .open_sequence(pool.scope(), &(0..48).collect::<Vec<_>>(), 0)
            .unwrap()
            .sequence();
        let first = pool
            .reserve_batch(&[EngineeringTpPageRowV1 {
                sequence,
                token: 0,
                position: 0,
            }])
            .unwrap();
        pool.begin_submission(&first).unwrap();
        let result = driver.execute_selected(&first, &[0]).unwrap();
        pool.commit_batch(&first, result.completion).unwrap();
        let batch = pool
            .reserve_batch(
                &(1..33)
                    .map(|position| EngineeringTpPageRowV1 {
                        sequence,
                        token: position,
                        position,
                    })
                    .collect::<Vec<_>>(),
            )
            .unwrap();
        assert!(driver.prefill32_plan_v1(&batch, &[31]).unwrap().is_none());
        assert_eq!(
            driver
                .expected_dispatch_counts_for_selection(&batch, &[31])
                .unwrap(),
            [616]
        );
        pool.begin_submission(&batch).unwrap();
        let result = driver.execute_selected(&batch, &[31]).unwrap();
        pool.commit_batch(&batch, result.completion).unwrap();
        assert!(
            !driver.inner.transports[0]
                .commands
                .iter()
                .any(|command| command.kernel == COPY)
        );
        let preparations = driver.inner.transports[0].packet_preparations.clone();
        let count = driver.dispatch_counts();
        assert!(
            driver
                .expected_dispatch_counts_for_selection(&batch, &[31])
                .is_err()
        );
        assert!(driver.execute_selected(&batch, &[31]).is_err());
        let mut foreign = wide_pool();
        let foreign = prepare(&mut foreign, 32);
        assert!(
            driver
                .expected_dispatch_counts_for_selection(&foreign, &[31])
                .is_err()
        );
        assert!(driver.execute_selected(&foreign, &[31]).is_err());
        assert_eq!(driver.dispatch_counts(), count);
        assert_eq!(driver.inner.transports[0].packet_preparations, preparations);
    }

    #[test]
    fn prefill32_page_pair_cached_prefix_is_not_a_writable_destination() {
        let mut pool = wide_pool();
        let mut driver = selected(&pool, true, true);
        let prefix = prepare(&mut pool, 16);
        let sequence = prefix.rows()[0].sequence();
        let page = prefix.rows()[0].writable_physical_page();
        pool.begin_submission(&prefix).unwrap();
        let result = driver.execute_selected(&prefix, &[15]).unwrap();
        pool.commit_batch(&prefix, result.completion).unwrap();
        let caches = driver.inner.ranks[0]
            .layers
            .iter()
            .flat_map(|layer| [layer.k_cache.id, layer.v_cache.id])
            .collect::<Vec<_>>();
        for id in &caches {
            driver.inner.transports[0].buffers.get_mut(id).unwrap()
                [page as usize * 32_768..(page as usize + 1) * 32_768]
                .fill(0x5a);
        }
        pool.retire_sequence(sequence, true, 1).unwrap();
        let hit = pool
            .open_sequence(pool.scope(), &(0..48).collect::<Vec<_>>(), 2)
            .unwrap();
        assert_eq!(hit.hit_tokens(), 16);
        assert_eq!(hit.physical_pages(), &[page]);
        let batch = pool
            .reserve_batch(
                &(16..48)
                    .map(|position| EngineeringTpPageRowV1 {
                        sequence: hit.sequence(),
                        token: position,
                        position,
                    })
                    .collect::<Vec<_>>(),
            )
            .unwrap();
        let plan = driver.prefill32_plan_v1(&batch, &[31]).unwrap().unwrap();
        for command in plan.commands(&driver.inner.ranks[0], 0).unwrap() {
            assert_ne!(scalar(&command, 5), page);
        }
        assert_ne!(
            batch.rows()[0].writable_physical_page(),
            batch.rows()[16].writable_physical_page()
        );
        assert_eq!(
            driver
                .expected_dispatch_counts_for_selection(&batch, &[31])
                .unwrap(),
            [652]
        );
        pool.begin_submission(&batch).unwrap();
        let result = driver.execute_selected(&batch, &[31]).unwrap();
        for id in &caches {
            assert!(
                driver.inner.transports[0].buffers[id]
                    [page as usize * 32_768..(page as usize + 1) * 32_768]
                    .iter()
                    .all(|byte| *byte == 0x5a)
            );
        }
        pool.commit_batch(&batch, result.completion).unwrap();
        pool.check_invariants().unwrap();
    }

    #[test]
    fn prefill32_page_pair_failed_allocation_validation_does_not_append_either_packet() {
        let mut pool = wide_pool();
        let mut driver = selected(&pool, true, true);
        let batch = prepare(&mut pool, 32);
        let plan = driver.prefill32_plan_v1(&batch, &[]).unwrap().unwrap();
        let original = dispatch(EMBEDDING, 1, vec![]);
        driver.inner.ordered_batches = Some(vec![original.clone(); 7]);
        driver.inner.hidden.resize(32 * 4096, 0);
        driver.inner.ranks[0].layers[0].v_cache.elements -= 1;
        assert!(plan.enqueue(&mut driver.inner, 0).is_err());
        assert_eq!(
            driver.inner.ordered_batches.as_ref().unwrap(),
            &vec![original; 7]
        );
        assert!(driver.inner.transports[0].commands.is_empty());
    }

    #[test]
    fn prefill32_page_pair_copy_wait_failure_poisoning_prevents_reuse() {
        let mut pool = wide_pool();
        let mut driver = selected(&pool, true, true);
        let batch = prepare(&mut pool, 32);
        driver.inner.transports[0].failure = Some(Failure::KvCopyWait);
        pool.begin_submission(&batch).unwrap();
        assert!(driver.execute_selected(&batch, &[31]).is_err());
        assert!(driver.poisoned);
        assert_eq!(driver.completed_batches(), 0);
        let count = driver.inner.transports[0].commands.len();
        assert!(driver.execute_selected(&batch, &[31]).is_err());
        assert_eq!(driver.inner.transports[0].commands.len(), count);
        driver.close().unwrap();
    }
}
