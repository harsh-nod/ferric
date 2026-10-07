//! Recording contracts only; these do not establish native model numerical parity.

use super::super::packed_down_r1::{PackedWeight, Workspace, admit};
use super::*;
#[cfg(all(
    feature = "c1-ordered64",
    not(feature = "model-timestamps"),
    not(feature = "c1-token-program")
))]
use crate::tp_artifact::ENGINEERING_TP_PACKED_DOWN_EXPORTS_R1 as ROOTS;
use crate::tp_artifact::{GemvPrefetchBindingV20, PackedDownBindingR1};
#[cfg(all(
    feature = "c1-ordered64",
    not(feature = "model-timestamps"),
    not(feature = "c1-token-program")
))]
use crate::tp_scheduler::{TpBatchRowKindV1, TpBatchRowV1, TpRequestIdV1};

#[path = "packed_down_setup_r1.rs"]
mod setup;

fn configured(pool: &EngineeringTpPagedPoolV1) -> EngineeringTpBatchExecutionV2<Recording> {
    let mut driver = super::prefill_kv_copy_v28::composition_configured(pool);
    driver.admitted_partial_gemv_v20 = Some(GemvPrefetchBindingV20::recording());
    super::prefill_kv_copy_v28::select_composition(&mut driver, true, true, true).unwrap();
    driver.partial_gemv_v28 = Some(false);
    driver.inner.transports[0].ordered64_supported = true;
    driver.inner.transports[0].rollover_supported = true;
    driver.inner.ordered_batch_width = OrderedBatchWidth::Packets64;
    // Shared synthetic storage bounds test memory; production setup rejects shared sources.
    let mut original = driver.inner.ranks[0].layers[0].weight(Qwen3TensorKind::DownProjection);
    original.elements = 4096 * 12_288;
    driver.inner.transports[0]
        .buffers
        .get_mut(&original.id)
        .unwrap()
        .resize(original.elements * 2, 0);
    let packed = allocate_tensor(&mut driver.inner.transports[0], 4096 * 6144, 4).unwrap();
    let scratch = allocate_tensor(&mut driver.inner.transports[0], 6144, 4).unwrap();
    let mut weights = BTreeMap::new();
    for (index, layer) in driver.inner.ranks[0].layers.iter_mut().enumerate() {
        for (kind, tensor) in &mut layer.weights {
            if *kind == Qwen3TensorKind::DownProjection {
                *tensor = original;
            }
        }
        weights.insert(
            u32::try_from(index).unwrap(),
            PackedWeight {
                original,
                packed,
                source_sha256: [2; 32],
            },
        );
    }
    driver.packed_down_r1 = Some(Workspace {
        binding: PackedDownBindingR1::recording(),
        weights,
        scratch,
        selected: None,
        bytes: 3_623_878_656,
        batch: None,
    });
    driver
}

fn select(driver: &mut EngineeringTpBatchExecutionV2<Recording>, enabled: bool) -> TpResult<()> {
    driver.configure_packed_down_binding_r1(PackedDownBindingR1::recording(), enabled)
}

#[cfg(all(
    feature = "c1-ordered64",
    not(feature = "model-timestamps"),
    not(feature = "c1-token-program")
))]
fn rows(batch: &EngineeringTpPreparedBatchV1, kind: TpBatchRowKindV1) -> Vec<TpBatchRowV1> {
    batch
        .rows()
        .iter()
        .enumerate()
        .map(|(index, row)| TpBatchRowV1 {
            request: TpRequestIdV1 {
                slot: 0,
                generation: 1,
            },
            token_id: row.token(),
            absolute_position: row.position(),
            kind: if index + 1 == batch.rows().len() {
                kind
            } else {
                TpBatchRowKindV1::PrefillIntermediate
            },
        })
        .collect()
}

#[test]
fn packed_down_feature_matrix_and_image_admission_are_closed() {
    let ordinary = cfg!(all(
        feature = "c1-ordered64",
        not(feature = "model-timestamps"),
        not(feature = "c1-token-program")
    ));
    for enabled in [false, true] {
        let mut driver = configured(&wide_pool());
        assert_eq!(select(&mut driver, enabled).is_ok(), ordinary);
        if !ordinary {
            assert_eq!(driver.packed_down_r1.as_ref().unwrap().selected, None);
        }
    }
    for mutation in 0..6 {
        let mut driver = fixture(1, &wide_pool());
        let transport = &mut driver.inner.transports[0];
        transport.buffers.clear();
        transport.ordered64_supported = true;
        transport.packed_down_r1_loaded = Some(PackedDownBindingR1::recording().hsaco());
        match mutation {
            1 => transport.packed_down_r1_loaded = None,
            2 => transport.ordered64_supported = false,
            3 => transport.argmax_peer = Some((0, 1, 1)),
            4 => transport.token_program_supported = true,
            5 => {
                transport.buffers.insert(999, vec![0]);
            }
            _ => {}
        }
        assert_eq!(
            admit(
                &mut driver.inner.transports,
                PackedDownBindingR1::recording()
            )
            .is_ok(),
            ordinary && mutation == 0
        );
    }
}

#[test]
fn packed_down_ordinary_constructor_has_no_storage_or_phase_binding() {
    let mut pool = wide_pool();
    let mut driver = fixture(1, &pool);
    assert_eq!(driver.packed_down_mode(), "baseline");
    assert_eq!(driver.packed_down_weight_bytes(), 0);
    assert_eq!(driver.packed_down_activation_scratch_bytes(), 0);
    let batch = prepare(&mut pool, 1);
    driver.bind_dispatch_rows(&batch, &[]).unwrap();
    driver.abandon_dispatch_rows();
    assert!(
        driver
            .packed_down_active_for_batch(&batch, 1)
            .is_ok_and(|active| !active)
    );
}

#[cfg(all(
    feature = "c1-ordered64",
    not(feature = "model-timestamps"),
    not(feature = "c1-token-program")
))]
mod ordinary {
    use super::*;

    fn prepare_at_128(pool: &mut EngineeringTpPagedPoolV1) -> EngineeringTpPreparedBatchV1 {
        let sequence = pool
            .open_sequence(pool.scope(), &[1; 129], 0)
            .unwrap()
            .sequence();
        // Prefix metadata only; the recording source makes no numerical claim.
        for first in (0..128).step_by(32) {
            let batch = pool
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
            pool.begin_submission(&batch).unwrap();
            pool.commit_batch(
                &batch,
                EngineeringTpBatchCompletionV1::after_all_ranks(&batch),
            )
            .unwrap();
        }
        pool.reserve_batch(&[EngineeringTpPageRowV1 {
            sequence,
            token: 1,
            position: 128,
        }])
        .unwrap()
    }

    #[test]
    fn packed_down_one_token_prefill_tail_at_decode_position_keeps_baseline_and_close_clears_binding()
     {
        for kind in [TpBatchRowKindV1::PrefillFinal, TpBatchRowKindV1::Decode] {
            let limits = EngineeringTpPagedLimitsV1::new(256, 32, 16, 160).unwrap();
            let mut pool =
                EngineeringTpPagedPoolV1::new_wide32(wide_pool().scope(), limits).unwrap();
            let mut driver = configured(&pool);
            select(&mut driver, true).unwrap();
            let batch = prepare_at_128(&mut pool);
            driver
                .bind_dispatch_rows(&batch, &rows(&batch, kind))
                .unwrap();
            assert_eq!(
                driver
                    .expected_dispatch_counts_for_batch(&batch, 1)
                    .unwrap(),
                [if kind == TpBatchRowKindV1::Decode {
                    688
                } else {
                    652
                }]
            );
            driver.close().unwrap();
            assert!(driver.packed_down_r1.as_ref().unwrap().batch.is_none());
            assert!(
                driver
                    .bind_dispatch_rows(&batch, &rows(&batch, kind))
                    .is_err()
            );
        }
    }

    #[test]
    fn packed_down_complete_request_repackages_every_decode_epoch_without_hot_allocation() {
        for enabled in [false, true] {
            let limits = EngineeringTpPagedLimitsV1::new(256, 32, 16, 160).unwrap();
            let mut pool =
                EngineeringTpPagedPoolV1::new_wide32(wide_pool().scope(), limits).unwrap();
            let mut driver = configured(&pool);
            select(&mut driver, enabled).unwrap();
            let allocations = driver.inner.transports[0]
                .buffers
                .keys()
                .copied()
                .collect::<Vec<_>>();
            let sequence = pool
                .open_sequence(pool.scope(), &(0..256).collect::<Vec<_>>(), 0)
                .unwrap()
                .sequence();
            let mut groups = 0;
            let mut outputs = 0;
            for (first, count) in (0..128)
                .step_by(16)
                .map(|first| (first, 16))
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
                let phase = if first < 112 {
                    TpBatchRowKindV1::PrefillIntermediate
                } else if first < 128 {
                    TpBatchRowKindV1::PrefillFinal
                } else {
                    TpBatchRowKindV1::Decode
                };
                let selected = if first < 112 {
                    vec![]
                } else {
                    vec![count as usize - 1]
                };
                driver
                    .bind_dispatch_rows(&batch, &rows(&batch, phase))
                    .unwrap();
                let expected = if count == 1 {
                    if enabled { 688 } else { 652 }
                } else if selected.is_empty() {
                    613
                } else {
                    616
                };
                assert_eq!(
                    driver
                        .expected_dispatch_counts_for_batch(&batch, selected.len())
                        .unwrap(),
                    [expected]
                );
                let before = driver.inner.transports[0].commands.len();
                pool.begin_submission(&batch).unwrap();
                let result = driver.execute_selected(&batch, &selected).unwrap();
                outputs += result.choices.len();
                let transport = &driver.inner.transports[0];
                let commands = &transport.commands[before..];
                assert_eq!(commands.len() as u64, expected);
                for root in ROOTS {
                    assert_eq!(
                        commands
                            .iter()
                            .filter(|command| command.kernel == root)
                            .count(),
                        if enabled && count == 1 { 36 } else { 0 }
                    );
                }
                assert_eq!(
                    commands
                        .iter()
                        .filter(|command| command.kernel
                            == if count == 1 {
                                "ferric_qwen3_tp_batch32_paged_kv_append_v5"
                            } else {
                                crate::tp_artifact::ENGINEERING_TP_PREFILL_KV_COPY_EXPORTS_V27[0]
                            })
                        .count(),
                    36
                );
                let observed = transport
                    .events
                    .borrow()
                    .iter()
                    .filter_map(|event| match event {
                        Event::OrderedSubmit(0, packets) => Some(*packets),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                assert_eq!(
                    observed,
                    if count == 1 {
                        [vec![64; 10], vec![if enabled { 48 } else { 12 }]].concat()
                    } else {
                        [11, 6].repeat(36)
                    }
                );
                groups += observed.len();
                transport.events.borrow_mut().clear();
                pool.commit_batch(&batch, result.completion).unwrap();
            }
            assert_eq!(
                (groups, outputs, driver.completed_batches()),
                (1973, 128, 135)
            );
            assert_eq!(
                driver.dispatch_counts(),
                [if enabled { 92_283 } else { 87_711 }]
            );
            assert_eq!(driver.kv_append_mode(), "baseline");
            assert_eq!(driver.partial_gemv_mode(), "baseline");
            assert_eq!(
                driver.inner.transports[0]
                    .buffers
                    .keys()
                    .copied()
                    .collect::<Vec<_>>(),
                allocations
            );
            pool.check_invariants().unwrap();
            driver.close().unwrap();
        }
    }

    #[test]
    fn packed_down_each_ordered_group_failure_consumes_binding_without_commit_or_replay() {
        for ordinal in 1..=11 {
            for failure in [
                Failure::OrderedSubmitAt(ordinal),
                Failure::OrderedWaitAt(ordinal),
            ] {
                let limits = EngineeringTpPagedLimitsV1::new(256, 32, 16, 160).unwrap();
                let mut pool =
                    EngineeringTpPagedPoolV1::new_wide32(wide_pool().scope(), limits).unwrap();
                let mut driver = configured(&pool);
                select(&mut driver, true).unwrap();
                driver.inner.transports[0].failure = Some(failure);
                let batch = prepare_at_128(&mut pool);
                driver
                    .bind_dispatch_rows(&batch, &rows(&batch, TpBatchRowKindV1::Decode))
                    .unwrap();
                assert_eq!(
                    driver
                        .expected_dispatch_counts_for_batch(&batch, 1)
                        .unwrap(),
                    [688]
                );
                pool.begin_submission(&batch).unwrap();
                assert!(driver.execute_selected(&batch, &[0]).is_err());
                assert!(driver.poisoned && driver.inner.packed_c1.is_none());
                assert!(driver.packed_down_r1.as_ref().unwrap().batch.is_none());
                assert_eq!(driver.completed_batches(), 0);
                assert_eq!(driver.dispatch_counts(), [((ordinal - 1) * 64) as u64]);
                assert_eq!(driver.inner.transports[0].packet_preparations, [688]);
                let observed = driver.inner.transports[0]
                    .events
                    .borrow()
                    .iter()
                    .filter_map(|event| match event {
                        Event::OrderedSubmit(0, packets) => Some(*packets),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                assert_eq!(
                    observed,
                    if ordinal == 11 {
                        [vec![64; 10], vec![48]].concat()
                    } else {
                        vec![64; ordinal]
                    }
                );
                let submitted = driver.inner.transports[0].commands.len();
                assert!(driver.execute_selected(&batch, &[0]).is_err());
                assert_eq!(driver.inner.transports[0].commands.len(), submitted);
                pool.quarantine_batch(&batch).unwrap();
                driver.close().unwrap();
            }
        }
    }

    #[test]
    fn packed_down_rejects_mixed_requests_and_multirow_decode_then_allows_fresh_retry() {
        for enabled in [false, true] {
            for mixed_request in [false, true] {
                let mut pool = wide_pool();
                let mut driver = configured(&pool);
                select(&mut driver, enabled).unwrap();
                let batch = prepare(&mut pool, 2);
                let mut invalid = rows(&batch, TpBatchRowKindV1::PrefillFinal);
                if mixed_request {
                    invalid[1].request.generation += 1;
                } else {
                    invalid
                        .iter_mut()
                        .for_each(|row| row.kind = TpBatchRowKindV1::Decode);
                }
                assert!(driver.bind_dispatch_rows(&batch, &invalid).is_err());
                assert!(driver.packed_down_r1.as_ref().unwrap().batch.is_none());
                assert!(
                    driver
                        .expected_dispatch_counts_for_batch(&batch, 1)
                        .is_err()
                );
                assert!(driver.inner.transports[0].commands.is_empty());
                assert!(driver.inner.transports[0].packet_preparations.is_empty());
                let retry_rows = batch
                    .rows()
                    .iter()
                    .map(|row| EngineeringTpPageRowV1 {
                        sequence: row.sequence(),
                        token: row.token(),
                        position: row.position(),
                    })
                    .collect::<Vec<_>>();
                pool.abort_batch(&batch).unwrap();
                let retry = pool.reserve_batch(&retry_rows).unwrap();
                assert!(retry.id() > batch.id());
                driver
                    .bind_dispatch_rows(&retry, &rows(&retry, TpBatchRowKindV1::PrefillFinal))
                    .unwrap();
                assert_eq!(
                    driver
                        .expected_dispatch_counts_for_batch(&retry, 1)
                        .unwrap(),
                    [616]
                );
                pool.begin_submission(&retry).unwrap();
                let output = driver.execute_selected(&retry, &[1]).unwrap();
                assert!(driver.packed_down_r1.as_ref().unwrap().batch.is_none());
                assert_eq!(driver.completed_batches(), 1);
                assert_eq!(driver.dispatch_counts(), [616]);
                assert!(
                    driver.inner.transports[0]
                        .commands
                        .iter()
                        .all(|command| !ROOTS.contains(&command.kernel))
                );
                pool.commit_batch(&retry, output.completion).unwrap();
                pool.check_invariants().unwrap();
                driver.close().unwrap();
            }
        }
    }

    #[test]
    fn packed_down_selection_is_atomic_and_rejects_unsupported_compositions() {
        for mutation in 0..18 {
            let mut driver = configured(&wide_pool());
            match mutation {
                0 => {}
                1 => driver.prefill16_ordered_v1 = Some(false),
                2 => driver.prefill32_pages_v1 = Some(false),
                3 => driver.partial_gemv_v28 = Some(true),
                4 => driver.c1_split_attention_v25 = Some(false),
                5 => driver.c1_packet_packing_v22 = Some(false),
                6 => driver.prefill_kv_copy_v28 = Some(false),
                7 => {
                    driver.admitted_c1_kv_copy_v19 =
                        Some(crate::tp_artifact::C1KvCopyBindingV19::recording())
                }
                8 => driver.inner.transports[0].token_program_supported = true,
                9 => driver.inner.transports[0].ordered64_supported = false,
                10 => driver.last_batch = 1,
                11 => driver.completed_batches = 1,
                12 => driver.poisoned = true,
                13 => driver.inner.closed = true,
                14 => driver.packed_down_r1.as_mut().unwrap().bytes -= 1,
                15 => driver.packed_down_r1.as_mut().unwrap().scratch.elements -= 1,
                16 => driver
                    .packed_down_r1
                    .as_mut()
                    .unwrap()
                    .weights
                    .remove(&0)
                    .map(|_| ())
                    .unwrap(),
                _ => driver.inner.ordered_batch_width = OrderedBatchWidth::Packets16,
            }
            assert_eq!(select(&mut driver, true).is_ok(), mutation == 0);
            assert_eq!(
                driver.packed_down_r1.as_ref().unwrap().selected,
                if mutation == 0 { Some(true) } else { None }
            );
            if mutation == 0 {
                assert!(select(&mut driver, false).is_err());
                assert_eq!(driver.packed_down_mode(), "packed-down-u32-r1");
                assert_eq!(driver.packed_down_weight_bytes(), 3_623_878_656);
                assert_eq!(driver.packed_down_activation_scratch_bytes(), 24_576);
                assert_eq!(driver.packed_down_source_identities().len(), 36);
            }
        }
    }

    #[test]
    fn packed_down_phase_binding_is_exact_nonconsuming_preflight_and_consumed_before_errors() {
        for enabled in [false, true] {
            let mut pool = wide_pool();
            let mut driver = configured(&pool);
            select(&mut driver, enabled).unwrap();
            let batch = prepare(&mut pool, 1);
            assert!(
                driver
                    .expected_dispatch_counts_for_batch(&batch, 1)
                    .is_err()
            );
            assert!(driver.execute_selected(&batch, &[0]).is_err());
            let scheduled = rows(&batch, TpBatchRowKindV1::Decode);
            let mut wrong = scheduled.clone();
            wrong[0].token_id += 1;
            assert!(driver.bind_dispatch_rows(&batch, &wrong).is_err());
            driver.bind_dispatch_rows(&batch, &scheduled).unwrap();
            assert!(driver.bind_dispatch_rows(&batch, &scheduled).is_err());
            assert!(driver.packed_down_r1.as_ref().unwrap().batch.is_none());
            driver.bind_dispatch_rows(&batch, &scheduled).unwrap();
            for _ in 0..2 {
                assert_eq!(
                    driver
                        .expected_dispatch_counts_for_batch(&batch, 1)
                        .unwrap(),
                    [if enabled { 652 } else { 616 }]
                );
            }
            assert!(driver.execute_selected(&batch, &[]).is_err());
            assert!(driver.packed_down_r1.as_ref().unwrap().batch.is_none());
            assert!(driver.inner.transports[0].commands.is_empty());
            assert!(driver.inner.transports[0].packet_preparations.is_empty());
            driver.bind_dispatch_rows(&batch, &scheduled).unwrap();
            driver.abandon_dispatch_rows();
            driver.bind_dispatch_rows(&batch, &scheduled).unwrap();
            assert!(
                driver
                    .take_packed_down_selection(&batch, &[0])
                    .is_ok_and(|active| active == enabled)
            );
            assert!(driver.take_packed_down_selection(&batch, &[0]).is_err());
        }
    }

    #[test]
    fn packed_down_changes_only_decode_down_and_adds_one_pack_after_each_swiglu() {
        for (count, kind) in [
            (1, TpBatchRowKindV1::Decode),
            (1, TpBatchRowKindV1::PrefillFinal),
            (1, TpBatchRowKindV1::PrefillIntermediate),
            (16, TpBatchRowKindV1::PrefillFinal),
        ] {
            let mut reference: Option<Vec<EngineeringTpDispatchV1>> = None;
            for enabled in [false, true] {
                let mut pool = wide_pool();
                let mut driver = configured(&pool);
                select(&mut driver, enabled).unwrap();
                let batch = prepare(&mut pool, count);
                let scheduled = rows(&batch, kind);
                let outputs = if kind == TpBatchRowKindV1::PrefillIntermediate {
                    vec![]
                } else {
                    vec![count as usize - 1]
                };
                driver.bind_dispatch_rows(&batch, &scheduled).unwrap();
                let active = enabled && kind == TpBatchRowKindV1::Decode;
                let expected =
                    if outputs.is_empty() { 613 } else { 616 } + if active { 36 } else { 0 };
                assert_eq!(
                    driver
                        .expected_dispatch_counts_for_batch(&batch, outputs.len())
                        .unwrap(),
                    [expected]
                );
                pool.begin_submission(&batch).unwrap();
                let output = driver.execute_selected(&batch, &outputs).unwrap();
                let commands = &driver.inner.transports[0].commands;
                assert_eq!(commands.len() as u64, expected);
                assert_eq!(
                    commands
                        .iter()
                        .filter(|command| command.kernel == ROOTS[0])
                        .count(),
                    if active { 36 } else { 0 }
                );
                assert_eq!(
                    commands
                        .iter()
                        .filter(|command| command.kernel == ROOTS[1])
                        .count(),
                    if active { 36 } else { 0 }
                );
                for (index, command) in commands
                    .iter()
                    .enumerate()
                    .filter(|(_, command)| command.kernel == ROOTS[1])
                {
                    assert_eq!(
                        commands[index - 1].kernel,
                        "ferric_qwen3_tp_batch32_swiglu_bf16_f32_v5"
                    );
                    assert_eq!(buffer(&commands[index - 1], 2).0, buffer(command, 0).0);
                    assert_eq!(commands[index + 1].kernel, ROOTS[0]);
                    assert_eq!(buffer(&commands[index + 1], 0).0, buffer(command, 1).0);
                    assert_eq!(
                        (
                            buffer(&commands[index + 1], 2).2,
                            buffer(&commands[index + 1], 2).3
                        ),
                        (4096, 4)
                    );
                    assert_eq!(scalar(&commands[index + 1], 7), 2);
                }
                let filtered = commands
                    .iter()
                    .filter(|command| command.kernel != ROOTS[1])
                    .cloned()
                    .collect::<Vec<_>>();
                if let Some(original) = &reference {
                    assert_eq!(filtered.len(), original.len());
                    for (mut actual, expected) in filtered.into_iter().zip(original) {
                        if actual.kernel == ROOTS[0] {
                            assert_eq!(
                                expected.kernel,
                                "ferric_qwen3_tp_batch32_wave_gemv_partial_f32_v5"
                            );
                            assert_eq!(actual.arguments[3..], expected.arguments[3..]);
                            assert_eq!(buffer(&actual, 2).0, buffer(expected, 2).0);
                            assert_eq!(buffer(&actual, 2).1, buffer(expected, 2).1);
                            assert_eq!(buffer(&actual, 2).3, buffer(expected, 2).3);
                            assert!(buffer(expected, 2).2 >= 4096);
                            actual.kernel = expected.kernel;
                            actual.arguments[..3].clone_from_slice(&expected.arguments[..3]);
                        }
                        assert_eq!(&actual, expected);
                    }
                } else {
                    reference = Some(filtered);
                }
                pool.commit_batch(&batch, output.completion).unwrap();
                driver.close().unwrap();
            }
        }
    }

    #[test]
    fn packed_down_commands_reject_stale_sources_and_aliases_before_submission() {
        let mut driver = configured(&wide_pool());
        select(&mut driver, true).unwrap();
        assert!(
            driver
                .packed_down_r1
                .as_ref()
                .unwrap()
                .commands(&driver.inner.ranks[0], 0)
                .is_ok()
        );
        assert!(
            driver
                .packed_down_r1
                .as_ref()
                .unwrap()
                .commands(&driver.inner.ranks[0], 36)
                .is_err()
        );
        driver
            .packed_down_r1
            .as_mut()
            .unwrap()
            .weights
            .get_mut(&0)
            .unwrap()
            .packed
            .id = driver.inner.ranks[0].activation.id;
        assert!(
            driver
                .packed_down_r1
                .as_ref()
                .unwrap()
                .commands(&driver.inner.ranks[0], 0)
                .is_err()
        );
        assert!(driver.inner.transports[0].commands.is_empty());
    }
}
