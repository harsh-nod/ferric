//! Host graph/ownership checks only; synthetic results are not MFMA/model parity.

use super::*;
use crate::tp_artifact::SplitKDownBindingR1;

#[test]
fn splitk_down_default_has_no_workspace_and_image_admission_is_closed() {
    let mut driver = fixture(1, &wide_pool());
    assert_eq!(driver.splitk_down_mode(), "baseline");
    assert_eq!(driver.splitk_down_scratch_bytes(), 0);
    let ordinary = cfg!(all(
        feature = "c1-ordered64",
        not(feature = "model-timestamps"),
        not(feature = "c1-token-program")
    ));
    for mutation in 0..5 {
        let transport = &mut driver.inner.transports[0];
        transport.buffers.clear();
        transport.ordered64_supported = mutation != 1;
        transport.token_program_supported = mutation == 2;
        transport.argmax_peer = (mutation == 3).then_some((0, 1, 1));
        transport.splitk_down_r1_loaded =
            (mutation != 4).then_some(SplitKDownBindingR1::recording().hsaco());
        assert_eq!(
            super::super::splitk_down_r1::admit(
                &mut driver.inner.transports,
                SplitKDownBindingR1::recording(),
            )
            .is_ok(),
            ordinary && mutation == 0
        );
        assert!(driver.inner.transports[0].commands.is_empty());
    }
}

#[test]
fn splitk_constructor_admission_failure_closes_every_owned_transport() {
    for fail_close in [false, true] {
        let mut driver = fixture(2, &wide_pool());
        if fail_close {
            driver.inner.transports[0].failure = Some(Failure::Close);
        }
        let before = driver
            .inner
            .transports
            .iter()
            .map(|transport| (transport.next, transport.buffers.len()))
            .collect::<Vec<_>>();
        let error = super::super::splitk_down_r1::admit_owned(
            &mut driver.inner.transports,
            SplitKDownBindingR1::recording(),
        )
        .unwrap_err();
        assert!(error.contains("ordinary nonpeer TP1 ordered64"));
        assert_eq!(
            error.contains("split-K admission close: injected close failure"),
            fail_close
        );
        for (transport, expected) in driver.inner.transports.iter().zip(before) {
            assert_eq!(transport.close_attempts, 1);
            assert_eq!((transport.next, transport.buffers.len()), expected);
            assert!(transport.commands.is_empty());
        }
        assert!(driver.splitk_down_r1.is_none());
    }
    let mut driver = fixture(1, &wide_pool());
    driver.inner.transports[0].ordered64_supported = true;
    let next = driver.inner.transports[0].next;
    assert!(
        super::super::splitk_down_r1::admit_owned(
            &mut driver.inner.transports,
            SplitKDownBindingR1::recording(),
        )
        .is_err()
    );
    assert_eq!(driver.inner.transports[0].next, next);
    assert_eq!(driver.inner.transports[0].close_attempts, 1);
    assert!(driver.splitk_down_r1.is_none());
}

#[test]
fn splitk_constructor_allocation_failure_and_alias_close_without_installing_workspace() {
    for mutation in 0..3 {
        let mut driver = fixture(1, &wide_pool());
        let alias = driver.inner.ranks[0].partial.id;
        let transport = &mut driver.inner.transports[0];
        let next = transport.next;
        let buffers = transport
            .buffers
            .iter()
            .map(|(id, bytes)| (*id, bytes.len()))
            .collect::<Vec<_>>();
        transport.failure = Some(match mutation {
            0 => Failure::AllocateAt(next),
            1 => Failure::AllocateAlias(alias),
            _ => Failure::AllocateAndCloseAt(next),
        });
        let error = driver
            .allocate_splitk_down_workspace(SplitKDownBindingR1::recording())
            .unwrap_err();
        assert!(error.contains(if mutation == 1 {
            "aliased resident storage"
        } else {
            "injected allocation failure"
        }));
        assert_eq!(
            error.contains("split-K setup close: rank 0: injected close failure"),
            mutation == 2
        );
        assert!(driver.poisoned && driver.splitk_down_r1.is_none());
        let transport = &driver.inner.transports[0];
        assert_eq!(transport.close_attempts, 1);
        assert_eq!(transport.next, next);
        assert_eq!(
            transport
                .buffers
                .iter()
                .map(|(id, bytes)| (*id, bytes.len()))
                .collect::<Vec<_>>(),
            buffers
        );
        assert!(transport.commands.is_empty());
        assert_eq!(
            transport
                .events
                .borrow()
                .iter()
                .filter(|event| matches!(event, Event::Close(0)))
                .count(),
            usize::from(mutation != 2)
        );
        // Failed closure stays an error, never a successful resource-retirement claim.
        if mutation != 2 {
            assert!(driver.inner.closed);
        }
    }
}

#[cfg(all(
    feature = "c1-ordered64",
    not(feature = "model-timestamps"),
    not(feature = "c1-token-program")
))]
mod ordinary {
    use super::*;
    use crate::tp_artifact::ENGINEERING_TP_SPLITK_DOWN_EXPORTS_R1 as ROOTS;
    use crate::tp_artifact::{
        C1KvCopyBindingV19, Fp32ArgmaxBindingV11, GemvPrefetchBindingV20, PrefillKvCopyBindingV27,
        QueryHoistBindingV14, SplitAttentionBindingV21, WaveRmsNormBindingV15,
    };
    use crate::tp_scheduler::{TpBatchRowKindV1, TpBatchRowV1, TpRequestIdV1};

    fn configured(pool: &EngineeringTpPagedPoolV1) -> EngineeringTpBatchExecutionV2<Recording> {
        let mut driver = super::super::prefill_kv_copy_v28::composition_configured(pool);
        driver.admitted_c1_kv_copy_v19 = Some(C1KvCopyBindingV19::recording());
        driver.admitted_partial_gemv_v20 = Some(GemvPrefetchBindingV20::recording());
        driver.inner.transports[0].ordered64_supported = true;
        driver.inner.transports[0].rollover_supported = true;
        driver
            .configure_ordered64_kv_copy_bindings_v1(
                Fp32ArgmaxBindingV11::recording(),
                QueryHoistBindingV14::recording(),
                WaveRmsNormBindingV15::recording(),
                PrefillKvCopyBindingV27::recording(),
                SplitAttentionBindingV21::recording(),
                GemvPrefetchBindingV20::recording(),
                C1KvCopyBindingV19::recording(),
                true,
            )
            .unwrap();
        for layer in &mut driver.inner.ranks[0].layers {
            let transport = &mut driver.inner.transports[0];
            let original = Tensor {
                id: transport.next,
                elements: 4096 * 12_288,
                element_bytes: 2,
            };
            let transposed = Tensor {
                id: transport.next + 1,
                ..original
            };
            transport.next += 2;
            for tensor in [original, transposed] {
                transport.buffers.insert(tensor.id, Vec::new());
                transport
                    .logical_weight_bytes
                    .insert(tensor.id, tensor.elements * 2);
            }
            for (kind, tensor) in &mut layer.weights {
                if *kind == Qwen3TensorKind::DownProjection {
                    *tensor = original;
                }
            }
            driver
                .projection
                .insert_recording_transposed(original.id, transposed);
        }
        driver
            .allocate_splitk_down_workspace(SplitKDownBindingR1::recording())
            .unwrap();
        driver
    }

    fn select(
        driver: &mut EngineeringTpBatchExecutionV2<Recording>,
        enabled: bool,
    ) -> TpResult<()> {
        driver.configure_splitk_down_binding_r1(SplitKDownBindingR1::recording(), enabled)
    }

    fn scheduled(
        batch: &EngineeringTpPreparedBatchV1,
        kind: TpBatchRowKindV1,
    ) -> Vec<TpBatchRowV1> {
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

    fn decode_batch(pool: &mut EngineeringTpPagedPoolV1) -> EngineeringTpPreparedBatchV1 {
        let sequence = pool
            .open_sequence(pool.scope(), &[1; 129], 0)
            .unwrap()
            .sequence();
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

    fn large_pool() -> EngineeringTpPagedPoolV1 {
        EngineeringTpPagedPoolV1::new_wide32(
            wide_pool().scope(),
            EngineeringTpPagedLimitsV1::new(256, 32, 16, 160).unwrap(),
        )
        .unwrap()
    }

    // Recording expands each completed frame into its actual per-command events.
    // Match that stream exactly so frame membership is not inferred from index%64.
    fn ordered_frames(
        events: &[Event],
        commands: &[EngineeringTpDispatchV1],
    ) -> Vec<std::ops::Range<usize>> {
        let mut frames = Vec::new();
        let mut event_index = 0;
        let mut first_command = 0;
        while event_index < events.len() {
            let count = match events.get(event_index) {
                Some(Event::OrderedSubmit(0, count)) => *count,
                _ => panic!("decode event must begin an ordered frame"),
            };
            assert!((1..=64).contains(&count));
            assert_eq!(events[event_index + 1], Event::OrderedWait(0, count));
            event_index += 2;
            let end = first_command + count;
            for command in &commands[first_command..end] {
                assert_eq!(events[event_index], Event::Submit(0, command.kernel));
                assert_eq!(events[event_index + 1], Event::Wait(0, command.kernel));
                event_index += 2;
            }
            frames.push(first_command..end);
            first_command = end;
        }
        assert_eq!(first_command, commands.len());
        frames
    }

    #[test]
    fn splitk_selection_rejects_profile_weight_and_scratch_drift_without_submission() {
        for mutation in 0..16 {
            let mut driver = configured(&large_pool());
            let last = driver.inner.ranks[0].layers[35].weight(Qwen3TensorKind::DownProjection);
            match mutation {
                0 => driver.c1_kv_copy_v19 = None,
                1 => driver.admitted_c1_kv_copy_v19 = None,
                2 => driver.partial_gemv_v28 = Some(true),
                3 => driver.prefill32_pages_v1 = Some(true),
                4 => driver.prefill16_ordered_v1 = Some(true),
                5 => driver.inner.transports[0].token_program_supported = true,
                6 => driver.inner.transports[0].argmax_peer = Some((0, 1, 1)),
                7 => driver.inner.ordered_batch_width = OrderedBatchWidth::Packets16,
                8 => driver.last_batch = 1,
                9 => driver.completed_batches = 1,
                10 => {
                    driver.splitk_down_r1.as_mut().unwrap().scratch.id =
                        driver.inner.ranks[0].activation.id
                }
                11 => driver.splitk_down_r1.as_mut().unwrap().scratch.elements -= 1,
                12 => driver.projection.insert_recording_transposed(
                    last.id,
                    Tensor {
                        elements: last.elements - 1,
                        ..last
                    },
                ),
                13 => {
                    driver.projection =
                        super::super::super::super::projection::ProjectionPolicy::default()
                }
                14 => {
                    driver.inner.ranks[0].layers[35]
                        .weights
                        .iter_mut()
                        .find(|(kind, _)| *kind == Qwen3TensorKind::DownProjection)
                        .unwrap()
                        .1
                        .id = u64::MAX
                }
                _ => {
                    let first =
                        driver.inner.ranks[0].layers[0].weight(Qwen3TensorKind::DownProjection);
                    driver.inner.ranks[0].layers[35]
                        .weights
                        .iter_mut()
                        .find(|(kind, _)| *kind == Qwen3TensorKind::DownProjection)
                        .unwrap()
                        .1 = first;
                }
            }
            assert!(select(&mut driver, true).is_err(), "mutation{mutation}");
            assert_eq!(driver.splitk_down_r1.as_ref().unwrap().selected, None);
            assert!(driver.inner.transports[0].commands.is_empty());
        }
        let mut driver = configured(&large_pool());
        select(&mut driver, false).unwrap();
        assert!(select(&mut driver, true).is_err());
        assert_eq!(driver.splitk_down_mode(), "baseline");
        assert_eq!(driver.splitk_down_scratch_bytes(), 131_072);
    }

    #[test]
    fn splitk_one_row_prefill_tail_at128_remains_baseline() {
        for kind in [TpBatchRowKindV1::PrefillFinal, TpBatchRowKindV1::Decode] {
            let mut pool = large_pool();
            let mut driver = configured(&pool);
            select(&mut driver, true).unwrap();
            let batch = decode_batch(&mut pool);
            driver
                .bind_dispatch_rows(&batch, &scheduled(&batch, kind))
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
            pool.begin_submission(&batch).unwrap();
            driver.execute_selected(&batch, &[0]).unwrap();
            assert_eq!(
                driver.inner.transports[0]
                    .commands
                    .iter()
                    .filter(|command| command.kernel == ROOTS[0])
                    .count(),
                if kind == TpBatchRowKindV1::Decode {
                    36
                } else {
                    0
                }
            );
            driver.close().unwrap();
            assert!(driver.splitk_down_r1.as_ref().unwrap().batch.is_none());
        }
    }

    #[test]
    fn splitk_scheduler_binding_is_exact_single_request_and_consumed_once() {
        let mut pool = large_pool();
        let mut driver = configured(&pool);
        select(&mut driver, true).unwrap();
        let batch = decode_batch(&mut pool);
        assert!(
            driver
                .expected_dispatch_counts_for_batch(&batch, 1)
                .is_err()
        );
        assert!(driver.execute_selected(&batch, &[0]).is_err());
        for mutation in 0..3 {
            let mut rows = scheduled(&batch, TpBatchRowKindV1::Decode);
            match mutation {
                0 => rows[0].token_id += 1,
                1 => rows[0].absolute_position += 1,
                _ => {
                    rows.clear();
                }
            }
            assert!(driver.bind_dispatch_rows(&batch, &rows).is_err());
        }
        let rows = scheduled(&batch, TpBatchRowKindV1::Decode);
        driver.bind_dispatch_rows(&batch, &rows).unwrap();
        assert!(driver.bind_dispatch_rows(&batch, &rows).is_err());
        assert!(driver.splitk_down_r1.as_ref().unwrap().batch.is_none());
        driver.bind_dispatch_rows(&batch, &rows).unwrap();
        assert!(driver.execute_selected(&batch, &[]).is_err());
        assert!(driver.splitk_down_r1.as_ref().unwrap().batch.is_none());
        driver.bind_dispatch_rows(&batch, &rows).unwrap();
        driver.abandon_dispatch_rows();
        assert!(driver.splitk_down_r1.as_ref().unwrap().batch.is_none());
        assert!(driver.inner.transports[0].commands.is_empty());
    }

    #[test]
    fn splitk_mixed_requests_and_multirow_decode_are_rejected_before_ipc() {
        let mut pool = large_pool();
        let mut driver = configured(&pool);
        select(&mut driver, true).unwrap();
        let batch = prepare(&mut pool, 16);
        let mut rows = scheduled(&batch, TpBatchRowKindV1::PrefillFinal);
        rows[0].request.generation += 1;
        assert!(driver.bind_dispatch_rows(&batch, &rows).is_err());
        let mut rows = scheduled(&batch, TpBatchRowKindV1::PrefillFinal);
        rows[0].kind = TpBatchRowKindV1::Decode;
        assert!(driver.bind_dispatch_rows(&batch, &rows).is_err());
        assert!(driver.inner.transports[0].commands.is_empty());
    }

    #[test]
    fn splitk_complete_request_preserves_prefill_v19_and_exact_ordered_group_counts() {
        let mut baseline_prefill = Vec::new();
        for enabled in [false, true] {
            let mut pool = large_pool();
            let mut driver = configured(&pool);
            select(&mut driver, enabled).unwrap();
            let next_allocation = driver.inner.transports[0].next;
            assert_eq!(
                driver.splitk_down_resident_weight_bytes().unwrap(),
                3_623_878_656
            );
            let weight_ids = driver.splitk_down_r1.as_ref().unwrap().weights.clone();
            let scratch = driver.splitk_down_r1.as_ref().unwrap().scratch.id;
            let sequence = pool
                .open_sequence(pool.scope(), &(0..256).collect::<Vec<_>>(), 0)
                .unwrap()
                .sequence();
            let mut groups = 0;
            let mut outputs = 0;
            let mut crossing_pairs = 0;
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
                let kind = if first < 112 {
                    TpBatchRowKindV1::PrefillIntermediate
                } else if first < 128 {
                    TpBatchRowKindV1::PrefillFinal
                } else {
                    TpBatchRowKindV1::Decode
                };
                let selected = if first < 112 {
                    vec![]
                } else {
                    vec![usize::try_from(count - 1).unwrap()]
                };
                driver
                    .bind_dispatch_rows(&batch, &scheduled(&batch, kind))
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
                let output = driver.execute_selected(&batch, &selected).unwrap();
                outputs += output.choices.len();
                let transport = &driver.inner.transports[0];
                let commands = &transport.commands[before..];
                assert_eq!(commands.len() as u64, expected);
                if first < 128 {
                    if enabled {
                        assert_eq!(commands, baseline_prefill[(first / 16) as usize]);
                    } else {
                        baseline_prefill.push(commands.to_vec());
                    }
                }
                let partials = commands
                    .iter()
                    .enumerate()
                    .filter(|(_, command)| command.kernel == ROOTS[0])
                    .collect::<Vec<_>>();
                assert_eq!(partials.len(), if enabled && count == 1 { 36 } else { 0 });
                let frames = if count == 1 {
                    ordered_frames(&transport.events.borrow(), commands)
                } else {
                    Vec::new()
                };
                for (layer, &(index, partial)) in partials.iter().enumerate() {
                    let merge = &commands[index + 1];
                    assert_eq!(merge.kernel, ROOTS[1]);
                    assert_eq!(buffer(partial, 0).0, driver.inner.ranks[0].activation.id);
                    assert_eq!(buffer(partial, 1).0, weight_ids[layer].1.id);
                    assert_eq!(buffer(partial, 2).0, scratch);
                    assert_eq!(buffer(merge, 0).0, scratch);
                    assert_eq!(buffer(merge, 1).0, driver.inner.ranks[0].partial.id);
                    assert_eq!(
                        buffer(&commands[index + 2], 0).0,
                        driver.inner.ranks[0].partial.id
                    );
                    assert_eq!(
                        commands[index - 1].kernel,
                        "ferric_qwen3_tp_batch32_swiglu_bf16_f32_v5"
                    );
                    let partial_frame = frames
                        .iter()
                        .position(|frame| frame.contains(&index))
                        .unwrap();
                    let merge_frame = frames
                        .iter()
                        .position(|frame| frame.contains(&(index + 1)))
                        .unwrap();
                    if partial_frame != merge_frame {
                        assert_eq!(merge_frame, partial_frame + 1);
                        assert_eq!(frames[partial_frame].end, index + 1);
                        assert_eq!(frames[merge_frame].start, index + 1);
                        crossing_pairs += 1;
                    }
                }
                assert_eq!(
                    commands
                        .iter()
                        .filter(|command| command.kernel
                            == crate::tp_artifact::ENGINEERING_TP_C1_KV_COPY_EXPORTS_V19[0])
                        .count(),
                    if count == 1 { 36 } else { 0 }
                );
                assert!(
                    !commands
                        .iter()
                        .any(|command| command.kernel
                            == "ferric_qwen3_tp_batch32_paged_kv_append_v5")
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
                pool.commit_batch(&batch, output.completion).unwrap();
            }
            assert_eq!(
                (groups, outputs, driver.completed_batches()),
                (1973, 128, 135)
            );
            assert_eq!(
                driver.dispatch_counts(),
                [if enabled { 92_283 } else { 87_711 }]
            );
            assert_eq!(driver.inner.transports[0].queue_epochs, 0);
            assert_eq!(
                driver.inner.transports[0].queue_packets,
                if enabled { 92_283 } else { 87_711 }
            );
            assert_eq!(driver.inner.transports[0].packet_preparations.len(), 135);
            assert_eq!(driver.inner.transports[0].next, next_allocation);
            assert!(driver.inner.transports[0].writes.iter().all(|(id, _)| {
                weight_ids
                    .iter()
                    .all(|(original, transposed)| *id != original.id && *id != transposed.id)
            }));
            assert!(
                driver.inner.transports[0]
                    .reads
                    .iter()
                    .all(|(id, _)| *id != scratch)
            );
            assert_eq!(crossing_pairs, if enabled { 127 } else { 0 });
            assert_eq!(driver.kv_append_mode(), "parallel-c1-v19");
            driver.close().unwrap();
        }
    }

    #[test]
    fn splitk_queue_rollover_and_counter_limits_use_the_selected_packet_budget() {
        let limit = fe2o3_kfd::engineering_wire::MAX_UNRETIRED_RING_PACKETS_V1;
        for enabled in [false, true] {
            let mut pool = large_pool();
            let mut driver = configured(&pool);
            select(&mut driver, enabled).unwrap();
            let batch = decode_batch(&mut pool);
            driver
                .bind_dispatch_rows(&batch, &scheduled(&batch, TpBatchRowKindV1::Decode))
                .unwrap();
            let expected = if enabled { 688 } else { 652 };
            // Simulate the retired tail of a prior queue lifetime, as existing
            // split-attention rollover fixtures do; this is not native evidence.
            driver.inner.transports[0].queue_packets = limit - expected + 1;
            pool.begin_submission(&batch).unwrap();
            driver.execute_selected(&batch, &[0]).unwrap();
            assert_eq!(driver.inner.transports[0].packet_preparations, [expected]);
            assert_eq!(driver.inner.transports[0].queue_epochs, 1);
            assert_eq!(driver.inner.transports[0].queue_packets, expected);
            assert_eq!(driver.completed_batches(), 1);
            driver.close().unwrap();

            for counter_overflow in [false, true] {
                let mut pool = large_pool();
                let mut driver = configured(&pool);
                select(&mut driver, enabled).unwrap();
                let batch = decode_batch(&mut pool);
                driver
                    .bind_dispatch_rows(&batch, &scheduled(&batch, TpBatchRowKindV1::Decode))
                    .unwrap();
                driver.inner.transports[0].rollover_supported = false;
                driver.completed_batches = if counter_overflow {
                    u64::MAX
                } else {
                    limit / expected
                };
                let prior = driver.completed_batches;
                let writes = driver.inner.transports[0].writes.len();
                let error = driver.execute_selected(&batch, &[0]).err().unwrap();
                assert!(error.contains(if counter_overflow {
                    "batch counter overflow"
                } else {
                    "no-ring-rollover budget"
                }));
                assert_eq!(driver.completed_batches(), prior);
                assert_eq!(driver.last_batch, 0);
                assert!(driver.inner.transports[0].packet_preparations.is_empty());
                assert!(driver.inner.transports[0].commands.is_empty());
                assert_eq!(driver.inner.transports[0].writes.len(), writes);
                driver.close().unwrap();
            }
        }
    }

    #[test]
    fn splitk_partial_merge_failures_poison_before_later_scratch_reuse() {
        for failure in [
            Failure::SplitKDownSubmit(0),
            Failure::SplitKDownSubmit(1),
            Failure::SplitKDownWait(0),
            Failure::SplitKDownWait(1),
            Failure::PreparePackets,
        ] {
            let mut pool = large_pool();
            let mut driver = configured(&pool);
            select(&mut driver, true).unwrap();
            let batch = decode_batch(&mut pool);
            driver
                .bind_dispatch_rows(&batch, &scheduled(&batch, TpBatchRowKindV1::Decode))
                .unwrap();
            driver.inner.transports[0].failure = Some(failure);
            pool.begin_submission(&batch).unwrap();
            assert!(driver.execute_selected(&batch, &[0]).is_err());
            assert!(driver.poisoned && driver.inner.packed_c1.is_none());
            assert_eq!(driver.completed_batches(), 0);
            let submitted = driver.inner.transports[0].commands.len();
            assert!(
                driver
                    .bind_dispatch_rows(&batch, &scheduled(&batch, TpBatchRowKindV1::Decode))
                    .is_err()
            );
            assert_eq!(driver.inner.transports[0].commands.len(), submitted);
            driver.close().unwrap();
        }
    }

    #[test]
    fn splitk_actual_cross_frame_pair_refuses_publication_or_completion_before_merge() {
        let mut pool = large_pool();
        let mut successful = configured(&pool);
        select(&mut successful, true).unwrap();
        let batch = decode_batch(&mut pool);
        successful
            .bind_dispatch_rows(&batch, &scheduled(&batch, TpBatchRowKindV1::Decode))
            .unwrap();
        pool.begin_submission(&batch).unwrap();
        successful.execute_selected(&batch, &[0]).unwrap();
        let transport = &successful.inner.transports[0];
        let frames = ordered_frames(&transport.events.borrow(), &transport.commands);
        let crossings = frames
            .windows(2)
            .enumerate()
            .filter_map(|(index, pair)| {
                let merge_index = pair[1].start;
                (transport.commands[pair[0].end - 1].kernel == ROOTS[0]
                    && transport.commands[merge_index].kernel == ROOTS[1])
                    .then_some((index + 2, merge_index))
            })
            .collect::<Vec<_>>();
        assert_eq!(crossings.len(), 1);
        let (merge_frame_ordinal, merge_index) = crossings[0];
        successful.close().unwrap();
        for failure in [
            Failure::OrderedSubmitAt(merge_frame_ordinal),
            Failure::OrderedWaitAt(merge_frame_ordinal),
        ] {
            let mut pool = large_pool();
            let mut driver = configured(&pool);
            select(&mut driver, true).unwrap();
            let batch = decode_batch(&mut pool);
            driver
                .bind_dispatch_rows(&batch, &scheduled(&batch, TpBatchRowKindV1::Decode))
                .unwrap();
            driver.inner.transports[0].failure = Some(failure);
            pool.begin_submission(&batch).unwrap();
            assert!(driver.execute_selected(&batch, &[0]).is_err());
            assert!(driver.poisoned && driver.inner.packed_c1.is_none());
            assert_eq!(driver.completed_batches(), 0);
            let transport = &driver.inner.transports[0];
            assert_eq!(transport.commands.len(), merge_index);
            assert_eq!(transport.commands.last().unwrap().kernel, ROOTS[0]);
            let events = transport.events.borrow();
            assert_eq!(
                events
                    .iter()
                    .filter(|event| matches!(event, Event::OrderedSubmit(..)))
                    .count(),
                merge_frame_ordinal
            );
            assert_eq!(
                events
                    .iter()
                    .filter(|event| matches!(event, Event::OrderedWait(..)))
                    .count(),
                merge_frame_ordinal - usize::from(matches!(failure, Failure::OrderedSubmitAt(_)))
            );
            drop(events);
            assert!(
                driver
                    .bind_dispatch_rows(&batch, &scheduled(&batch, TpBatchRowKindV1::Decode))
                    .is_err()
            );
            assert_eq!(driver.inner.transports[0].commands.len(), merge_index);
            driver.close().unwrap();
        }
    }

    #[test]
    fn splitk_last_layer_rejects_changed_weight_and_scratch_before_command_creation() {
        let mut driver = configured(&large_pool());
        select(&mut driver, true).unwrap();
        let workspace = driver.splitk_down_r1.as_mut().unwrap();
        workspace.weights[35].1.id ^= 1;
        assert!(
            workspace
                .commands(&driver.inner.ranks[0], &driver.projection, 35)
                .is_err()
        );
        assert!(driver.inner.transports[0].commands.is_empty());
    }
}
