//! Recording and byte-layout contracts, not native projection arithmetic evidence.

use super::super::packed_gate_up_r2::{PackedWeight, Workspace, admit};
use super::*;
use crate::tp_artifact::{
    C1KvCopyBindingV19, Fp32ArgmaxBindingV11, GemvPrefetchBindingV20, PrefillKvCopyBindingV27,
    QueryHoistBindingV14, SplitAttentionBindingV21, WaveRmsNormBindingV15,
};
use crate::tp_artifact::{ENGINEERING_TP_PACKED_BF16_EXPORTS_R2 as ROOTS, PackedBf16BindingR2};

fn configured(pool: &EngineeringTpPagedPoolV1) -> EngineeringTpBatchExecutionV2<Recording> {
    let mut driver = super::prefill_kv_copy_v28::composition_configured(pool);
    driver.admitted_c1_kv_copy_v19 = Some(C1KvCopyBindingV19::recording());
    driver.admitted_partial_gemv_v20 = Some(GemvPrefetchBindingV20::recording());
    driver.inner.transports[0].ordered64_supported = true;
    driver.inner.transports[0].rollover_supported = true;
    // Exercise the common composition checks in every feature build; the new
    // packed selector independently rejects builds without its dedicated feature.
    driver
        .configure_ordered64_kv_copy_checked_bindings_v1(
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
    // Share synthetic matrices across layers to bound recording-test memory.
    // Production setup rejects repeated original identities and authenticates all 72 sources.
    let mut original = driver.inner.ranks[0].layers[0].weight(Qwen3TensorKind::GateProjection);
    original.elements = 12_288 * 4096;
    driver.inner.transports[0]
        .buffers
        .get_mut(&original.id)
        .unwrap()
        .resize(original.elements * 2, 0);
    let gate = allocate_tensor(&mut driver.inner.transports[0], 12_288 * 2048, 4).unwrap();
    let up = allocate_tensor(&mut driver.inner.transports[0], 12_288 * 2048, 4).unwrap();
    let scratch = allocate_tensor(&mut driver.inner.transports[0], 2048, 4).unwrap();
    let mut weights = BTreeMap::new();
    for (index, layer) in driver.inner.ranks[0].layers.iter_mut().enumerate() {
        for (kind, tensor) in &mut layer.weights {
            if matches!(
                kind,
                Qwen3TensorKind::GateProjection | Qwen3TensorKind::UpProjection
            ) {
                *tensor = original;
            }
        }
        for (tag, packed) in [(4, gate), (5, up)] {
            weights.insert(
                (u32::try_from(index).unwrap(), tag),
                PackedWeight {
                    original,
                    packed,
                    source_sha256: [u8::try_from(tag).unwrap(); 32],
                },
            );
        }
    }
    driver.packed_gate_up_r2 = Some(Workspace {
        binding: PackedBf16BindingR2::recording(),
        weights,
        scratch,
        selected: None,
        bytes: 7_247_757_312,
    });
    driver
}

fn select(driver: &mut EngineeringTpBatchExecutionV2<Recording>, enabled: bool) -> TpResult<()> {
    driver.configure_packed_gate_up_binding_r2(PackedBf16BindingR2::recording(), enabled)
}

#[cfg(all(feature = "c1-ordered64", not(feature = "model-timestamps")))]
fn prepare_at(pool: &mut EngineeringTpPagedPoolV1, prefix: u32) -> EngineeringTpPreparedBatchV1 {
    let sequence = pool
        .open_sequence(pool.scope(), &vec![1; prefix as usize], 0)
        .unwrap()
        .sequence();
    for start in (0..prefix).step_by(32) {
        let rows = (start..(start + 32).min(prefix))
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
            EngineeringTpBatchCompletionV1::after_all_ranks(&batch),
        )
        .unwrap();
    }
    pool.reserve_batch(&[EngineeringTpPageRowV1 {
        sequence,
        token: 42,
        position: prefix,
    }])
    .unwrap()
}

#[test]
fn packed_gate_up_image_admission_is_exact_fresh_nonpeer_tp1() {
    for mutation in 0..5 {
        let mut driver = fixture(if mutation == 4 { 2 } else { 1 }, &wide_pool());
        for transport in &mut driver.inner.transports {
            transport.buffers.clear();
            transport.packed_bf16_r2_loaded = Some(PackedBf16BindingR2::recording().hsaco());
        }
        match mutation {
            1 => driver.inner.transports[0].packed_bf16_r2_loaded = None,
            2 => driver.inner.transports[0].packed_bf16_r2_loaded = Some([0; 32]),
            3 => driver.inner.transports[0].argmax_peer = Some((0, 1, 1)),
            _ => {}
        }
        assert_eq!(
            admit(
                &mut driver.inner.transports,
                PackedBf16BindingR2::recording()
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
    assert!(admit::<Recording>(&mut [], PackedBf16BindingR2::recording()).is_err());
    let mut driver = fixture(1, &wide_pool());
    driver.inner.transports[0].packed_bf16_r2_loaded =
        Some(PackedBf16BindingR2::recording().hsaco());
    assert!(
        admit(
            &mut driver.inner.transports,
            PackedBf16BindingR2::recording()
        )
        .is_err()
    );
}

#[test]
fn packed_gate_up_selector_feature_matrix_is_closed_without_mutation() {
    let mut driver = configured(&wide_pool());
    let result = select(&mut driver, true);
    let admitted = cfg!(all(
        feature = "c1-ordered64",
        not(feature = "model-timestamps")
    ));
    assert_eq!(result.is_ok(), admitted);
    assert_eq!(
        driver.packed_gate_up_r2.as_ref().unwrap().selected,
        admitted.then_some(true)
    );
    assert_eq!(driver.kv_append_mode(), "parallel-c1-v19");
    assert!(driver.inner.transports[0].commands.is_empty());
    driver.close().unwrap();
}

#[test]
fn packed_gate_up_token_transport_rejects_before_image_admission_or_allocation() {
    let mut driver = fixture(1, &wide_pool());
    let transport = &mut driver.inner.transports[0];
    transport.ordered64_supported = true;
    transport.token_program_supported = true;
    transport.packed_bf16_r2_loaded = None;
    transport.buffers.clear();
    transport.events.borrow_mut().clear();
    let error = admit(
        &mut driver.inner.transports,
        PackedBf16BindingR2::recording(),
    )
    .unwrap_err();
    assert!(error.contains("fixed 652-packet token programs"));
    let transport = &driver.inner.transports[0];
    assert!(transport.buffers.is_empty());
    assert!(transport.commands.is_empty());
    assert!(transport.events.borrow().is_empty());
    driver.close().unwrap();
}

#[test]
fn packed_gate_up_token_transport_rejects_both_selections_without_mutation() {
    for enabled in [false, true] {
        let mut driver = configured(&wide_pool());
        driver.inner.transports[0].token_program_supported = true;
        let transport = &driver.inner.transports[0];
        let before = (
            transport.buffers.keys().copied().collect::<Vec<_>>(),
            transport.write_payloads.len(),
            transport.commands.len(),
            transport.events.borrow().clone(),
        );
        assert!(select(&mut driver, enabled).is_err());
        assert!(
            driver
                .packed_gate_up_r2
                .as_ref()
                .unwrap()
                .selected
                .is_none()
        );
        assert!(driver.inner.packed_c1.is_none());
        let transport = &driver.inner.transports[0];
        assert_eq!(
            (
                transport.buffers.keys().copied().collect::<Vec<_>>(),
                transport.write_payloads.len(),
                transport.commands.len(),
                transport.events.borrow().clone(),
            ),
            before
        );
        driver.close().unwrap();
    }
}

#[test]
#[cfg(all(feature = "c1-token-program", not(feature = "model-timestamps")))]
fn packed_gate_up_ffn_cannot_start_a_fixed_token_program() {
    use crate::tp_execution::reduction::{AttentionProducerSchedule, FeedForwardProducerSchedule};
    let mut driver = configured(&wide_pool());
    select(&mut driver, true).unwrap();
    driver.inner.transports[0].token_program_supported = true;
    driver.inner.transports[0].events.borrow_mut().clear();
    let before = (
        driver.inner.collective,
        driver.inner.ranks[0].hidden.id,
        driver.inner.transports[0]
            .buffers
            .keys()
            .copied()
            .collect::<Vec<_>>(),
        driver.inner.transports[0].write_payloads.len(),
    );
    assert!(
        driver
            .inner
            .begin_packed_c1_with_ffn(
                AttentionProducerSchedule::Split8V21,
                FeedForwardProducerSchedule::PackedGateUpR2,
            )
            .is_err()
    );
    assert!(driver.inner.packed_c1.is_none());
    assert!(driver.inner.ordered_batches.as_ref().unwrap().is_empty());
    assert!(driver.inner.transports[0].commands.is_empty());
    assert!(driver.inner.transports[0].events.borrow().is_empty());
    assert_eq!(
        (
            driver.inner.collective,
            driver.inner.ranks[0].hidden.id,
            driver.inner.transports[0]
                .buffers
                .keys()
                .copied()
                .collect::<Vec<_>>(),
            driver.inner.transports[0].write_payloads.len(),
        ),
        before
    );
    driver.close().unwrap();
}

#[test]
#[cfg(all(feature = "c1-ordered64", not(feature = "model-timestamps")))]
fn packed_gate_up_selection_is_explicit_atomic_and_cannot_relax_existing_profiles() {
    let mut baseline = super::prefill_kv_copy_v28::composition_configured(&wide_pool());
    assert_eq!(baseline.packed_gate_up_mode(), "baseline");
    assert_eq!(baseline.packed_gate_up_weight_bytes(), 0);
    assert_eq!(baseline.packed_gate_up_activation_scratch_bytes(), 0);
    assert!(select(&mut baseline, true).is_err());
    for mutation in 0..20 {
        let mut driver = configured(&wide_pool());
        assert_eq!(
            driver.packed_gate_up_mode(),
            "unconfigured-packed-gate-up-r2"
        );
        assert_eq!(driver.packed_gate_up_activation_scratch_bytes(), 8192);
        match mutation {
            0 => {}
            1 => driver.last_batch = 1,
            2 => driver.c1_wave_layers = false,
            3 => driver.c1_packet_packing_v22 = Some(false),
            4 => driver.partial_gemv_v28 = Some(true),
            5 => driver.poisoned = true,
            6 => driver.row_capacity = 16,
            7 => driver.inner.transports[0].argmax_peer = Some((0, 1, 1)),
            8 => driver.c1_kv_copy_v19 = None,
            9 => driver.admitted_c1_kv_copy_v19 = None,
            10 => driver.prefill_kv_copy_v28 = Some(false),
            11 => driver.c1_split_attention_v25 = Some(false),
            12 => driver.admitted_partial_gemv_v20 = None,
            13 => driver.prefill32_pages_v1 = Some(false),
            14 => driver.inner.ordered_batch_width = OrderedBatchWidth::Packets16,
            15 => driver.inner.transports[0].ordered64_supported = false,
            16 => driver.packed_gate_up_r2.as_mut().unwrap().bytes -= 1,
            17 => driver.packed_gate_up_r2.as_mut().unwrap().scratch.elements -= 1,
            18 => driver.query_hoist_v14 = None,
            _ => driver.inner.ranks[0].dispatches = 1,
        }
        assert_eq!(select(&mut driver, true).is_ok(), mutation == 0);
        assert_eq!(
            driver.packed_gate_up_r2.as_ref().unwrap().selected,
            if mutation == 0 { Some(true) } else { None }
        );
        if mutation == 0 {
            assert_eq!(driver.packed_gate_up_mode(), "packed-gate-up-u32-r2");
            assert_eq!(driver.packed_gate_up_weight_bytes(), 7_247_757_312);
            assert_eq!(driver.packed_gate_up_source_identities().len(), 72);
            assert!(select(&mut driver, false).is_err());
        }
    }
}

#[test]
#[cfg(all(feature = "c1-ordered64", not(feature = "model-timestamps")))]
fn packed_gate_up_rejects_unselected_execution_and_stale_or_aliased_weight_bindings() {
    let mut pool = wide_pool();
    let mut driver = configured(&pool);
    let batch = prepare(&mut pool, 1);
    assert!(driver.execute_selected(&batch, &[0]).is_err());
    assert!(driver.inner.transports[0].commands.is_empty());
    select(&mut driver, true).unwrap();
    for mutation in 0..5 {
        let workspace = driver.packed_gate_up_r2.as_mut().unwrap();
        let original = workspace.weights[&(0, 4)];
        let weight = workspace.weights.get_mut(&(0, 4)).unwrap();
        match mutation {
            0 => weight.original.id += 1,
            1 => weight.original.elements -= 1,
            2 => weight.packed.element_bytes = 2,
            3 => weight.packed.id = workspace.scratch.id,
            _ => weight.packed.id = driver.inner.ranks[0].up.id,
        }
        assert!(workspace.commands(&driver.inner.ranks[0], 0).is_err());
        workspace.weights.insert((0, 4), original);
    }
    assert!(driver.inner.transports[0].commands.is_empty());
    assert!(
        driver
            .packed_gate_up_r2
            .as_ref()
            .unwrap()
            .commands(&driver.inner.ranks[0], 36)
            .is_err()
    );
    driver.close().unwrap();
}

#[test]
#[cfg(all(feature = "c1-ordered64", not(feature = "model-timestamps")))]
fn packed_gate_up_changes_only_two_roots_and_one_pack_per_layer_for_published_c1() {
    for (rows, prefix, publish) in [
        (1, 0, true),
        (1, 127, true),
        (1, 0, false),
        (2, 0, true),
        (32, 0, true),
    ] {
        let mut reference = None;
        for enabled in [false, true] {
            let limits = EngineeringTpPagedLimitsV1::new(256, 32, 16, 100).unwrap();
            let mut pool =
                EngineeringTpPagedPoolV1::new_wide32(wide_pool().scope(), limits).unwrap();
            let mut driver = configured(&pool);
            select(&mut driver, enabled).unwrap();
            let batch = if prefix == 0 {
                prepare(&mut pool, rows)
            } else {
                prepare_at(&mut pool, prefix)
            };
            let output_rows = if publish {
                vec![rows as usize - 1]
            } else {
                vec![]
            };
            let active = enabled && rows == 1 && publish;
            let base = if prefix == 127 {
                652
            } else if publish {
                616
            } else {
                613
            };
            let expected = base + if active { 36 } else { 0 };
            assert_eq!(
                driver
                    .expected_dispatch_counts_for_batch(&batch, output_rows.len())
                    .unwrap(),
                [expected]
            );
            pool.begin_submission(&batch).unwrap();
            let output = driver.execute_selected(&batch, &output_rows).unwrap();
            let commands = &driver.inner.transports[0].commands;
            assert_eq!(commands.len() as u64, expected);
            assert_eq!(
                driver.inner.transports[0].packet_preparations,
                [expected + if publish { 0 } else { 3 }]
            );
            assert_eq!(
                commands
                    .iter()
                    .filter(|command| command.kernel == ROOTS[1])
                    .count(),
                if active { 36 } else { 0 }
            );
            assert_eq!(
                commands
                    .iter()
                    .filter(|command| command.kernel == ROOTS[0])
                    .count(),
                if active { 72 } else { 0 }
            );
            for (index, command) in commands
                .iter()
                .enumerate()
                .filter(|(_, command)| command.kernel == ROOTS[1])
            {
                assert_eq!(
                    commands[index - 1].kernel,
                    crate::tp_artifact::ENGINEERING_TP_WAVE_RMSNORM_EXPORTS_V15[0]
                );
                assert_eq!(buffer(&commands[index - 1], 4).0, buffer(command, 0).0);
                for (next, role) in [(index + 1, 4), (index + 2, 5)] {
                    assert_eq!(commands[next].kernel, ROOTS[0]);
                    assert_eq!(scalar(&commands[next], 7), role);
                    assert_eq!(buffer(&commands[next], 0).0, buffer(command, 1).0);
                }
            }
            let filtered = commands
                .iter()
                .filter(|command| command.kernel != ROOTS[1])
                .cloned()
                .collect::<Vec<_>>();
            if let Some(original) = &reference {
                let original: &Vec<EngineeringTpDispatchV1> = original;
                assert_eq!(filtered.len(), original.len());
                for (mut actual, expected_command) in filtered.into_iter().zip(original) {
                    if actual.kernel == ROOTS[0] {
                        assert_eq!(
                            expected_command.kernel,
                            "ferric_qwen3_tp_batch32_wave_gemv_bf16_v5"
                        );
                        assert_eq!(actual.arguments[3..], expected_command.arguments[3..]);
                        assert_eq!(buffer(&actual, 2).0, buffer(expected_command, 2).0);
                        assert_eq!(buffer(&actual, 2).1, 0);
                        assert_eq!(buffer(&actual, 2).2, 12_288);
                        actual.kernel = expected_command.kernel;
                        actual.arguments[..3].copy_from_slice(&expected_command.arguments[..3]);
                    }
                    assert_eq!(&actual, expected_command);
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
#[cfg(all(feature = "c1-ordered64", not(feature = "model-timestamps")))]
fn packed_gate_up_repackages_each_layer_in_each_epoch_without_new_hot_allocations() {
    let mut pool = wide_pool();
    let mut driver = configured(&pool);
    select(&mut driver, true).unwrap();
    let first = prepare(&mut pool, 1);
    let sequence = first.rows()[0].sequence();
    let allocations = driver.inner.transports[0]
        .buffers
        .iter()
        .map(|(&id, bytes)| (id, bytes.len()))
        .collect::<Vec<_>>();
    for batch in [Some(first), None] {
        let batch = batch.unwrap_or_else(|| {
            pool.reserve_batch(&[EngineeringTpPageRowV1 {
                sequence,
                token: 42,
                position: 1,
            }])
            .unwrap()
        });
        let before = driver.inner.transports[0].commands.len();
        pool.begin_submission(&batch).unwrap();
        let output = driver.execute_selected(&batch, &[0]).unwrap();
        let commands = &driver.inner.transports[0].commands[before..];
        assert_eq!(
            commands
                .iter()
                .filter(|command| command.kernel == ROOTS[1])
                .count(),
            36
        );
        assert_eq!(commands.len(), 652);
        assert_eq!(
            driver.inner.transports[0]
                .buffers
                .iter()
                .map(|(&id, bytes)| (id, bytes.len()))
                .collect::<Vec<_>>(),
            allocations
        );
        pool.commit_batch(&batch, output.completion).unwrap();
    }
    assert_eq!(driver.completed_batches(), 2);
    assert_eq!(driver.dispatch_counts(), [1304]);
    driver.close().unwrap();
}

#[test]
#[cfg(all(feature = "c1-ordered64", not(feature = "model-timestamps")))]
fn packed_gate_up_failure_never_publishes_commits_or_replays_a_partial_epoch() {
    for failure in [
        Failure::PackedRootSubmit(0),
        Failure::PackedRootSubmit(1),
        Failure::PackedRootWait(0),
        Failure::PackedRootWait(1),
        Failure::OrderedWaitAt(1),
    ] {
        let mut pool = wide_pool();
        let mut driver = configured(&pool);
        select(&mut driver, true).unwrap();
        driver.inner.transports[0].failure = Some(failure);
        let batch = prepare(&mut pool, 1);
        pool.begin_submission(&batch).unwrap();
        assert!(driver.execute_selected(&batch, &[0]).is_err());
        assert!(driver.poisoned && driver.inner.packed_c1.is_none());
        assert_eq!(driver.completed_batches(), 0);
        let count = driver.inner.transports[0].commands.len();
        assert!(driver.execute_selected(&batch, &[0]).is_err());
        assert_eq!(driver.inner.transports[0].commands.len(), count);
        assert!(pool.begin_submission(&batch).is_err());
        driver.close().unwrap();
    }
}

#[test]
fn packed_gate_up_producer_budget_is_six_only_in_the_candidate_schedule() {
    use super::super::super::reduction::{AttentionProducerSchedule, FeedForwardProducerSchedule};
    for (schedule, expected) in [
        (FeedForwardProducerSchedule::Baseline, 5),
        (FeedForwardProducerSchedule::PackedGateUpR2, 6),
    ] {
        for observed in [5, 6, 7] {
            let mut driver = super::prefill_kv_copy_v28::composition_configured(&wide_pool());
            super::prefill_kv_copy_v28::select_composition(&mut driver, true, true, true).unwrap();
            driver.inner.hidden.resize(4096, 0);
            driver
                .inner
                .begin_packed_c1_with_ffn(AttentionProducerSchedule::Baseline, schedule)
                .unwrap();
            let state = driver.inner.packed_c1.as_mut().unwrap();
            let key = state.collective.expected();
            state.collective.arrive(0, key).unwrap();
            state.collective.advance().unwrap();
            state.producer_packets = observed;
            assert_eq!(
                driver
                    .inner
                    .reduce(0, Qwen3TensorParallelCollectiveV1::FeedForwardDownSum)
                    .is_ok(),
                observed == expected
            );
            driver.inner.discard_packed_c1();
            driver.close().unwrap();
        }
    }
}

#[test]
#[cfg(all(feature = "c1-ordered64", not(feature = "model-timestamps")))]
fn packed_gate_up_and_prefill32_selectors_reject_hybrid_in_both_orders() {
    for enabled in [false, true] {
        let mut driver = configured(&wide_pool());
        assert!(driver.configure_prefill32_pages_binding_v1(
            PrefillKvCopyBindingV27::recording(), enabled,
        ).is_err());
        assert!(driver.prefill32_pages_v1.is_none());
        select(&mut driver, enabled).unwrap();
        assert!(driver.configure_prefill32_pages_binding_v1(
            PrefillKvCopyBindingV27::recording(), enabled,
        ).is_err());
        assert!(driver.prefill32_pages_v1.is_none());
        driver.close().unwrap();

        let mut driver = configured(&wide_pool());
        driver.prefill32_pages_v1 = Some(enabled);
        assert!(select(&mut driver, enabled).is_err());
        assert!(
            driver
                .packed_gate_up_r2
                .as_ref()
                .unwrap()
                .selected
                .is_none()
        );
        assert!(driver.inner.transports[0].commands.is_empty());
        driver.close().unwrap();
    }
}

#[test]
#[cfg(all(feature = "c1-ordered64", not(feature = "model-timestamps")))]
fn packed_gate_up_complete_request_preserves_v19_v27_split8_and_1973_groups() {
    const COPY: &str = crate::tp_artifact::ENGINEERING_TP_C1_KV_COPY_EXPORTS_V19[0];
    const PREFILL: &str = crate::tp_artifact::ENGINEERING_TP_PREFILL_KV_COPY_EXPORTS_V27[0];
    for enabled in [false, true] {
        let limits = EngineeringTpPagedLimitsV1::new(256, 32, 16, 160).unwrap();
        let mut pool = EngineeringTpPagedPoolV1::new_wide32(wide_pool().scope(), limits).unwrap();
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
        let mut output_count = 0;
        for (first, rows) in (0..128)
            .step_by(16)
            .map(|first| (first, 16))
            .chain((128..255).map(|first| (first, 1)))
        {
            let batch = pool
                .reserve_batch(
                    &(first..first + rows)
                        .map(|position| EngineeringTpPageRowV1 {
                            sequence,
                            token: position,
                            position,
                        })
                        .collect::<Vec<_>>(),
                )
                .unwrap();
            let selected = if first < 112 {
                vec![]
            } else {
                vec![usize::try_from(rows - 1).unwrap()]
            };
            let expected = if rows == 1 {
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
            output_count += output.choices.len();
            let transport = &driver.inner.transports[0];
            let commands = &transport.commands[before..];
            assert_eq!(commands.len() as u64, expected);
            assert!(
                transport
                    .events
                    .borrow()
                    .iter()
                    .all(|event| !matches!(event, Event::TokenSubmit(..) | Event::TokenWait(..)))
            );
            assert_eq!(
                commands
                    .iter()
                    .filter(|command| command.kernel == if rows == 1 { COPY } else { PREFILL })
                    .count(),
                36
            );
            assert_eq!(
                commands
                    .iter()
                    .filter(|command| command.kernel == ROOTS[1])
                    .count(),
                if enabled && rows == 1 { 36 } else { 0 }
            );
            assert_eq!(
                commands
                    .iter()
                    .filter(|command| command.kernel == ROOTS[0])
                    .count(),
                if enabled && rows == 1 { 72 } else { 0 }
            );
            for root in crate::tp_artifact::ENGINEERING_TP_SPLIT_ATTENTION_EXPORTS_V21 {
                assert_eq!(
                    commands
                        .iter()
                        .filter(|command| command.kernel == root)
                        .count(),
                    if rows == 1 { 36 } else { 0 }
                );
            }
            assert!(commands.iter().all(|command| {
                !crate::tp_artifact::ENGINEERING_TP_GEMV_PREFETCH_EXPORTS_V20
                    .contains(&command.kernel)
            }));
            let observed = transport
                .events
                .borrow()
                .iter()
                .filter_map(|event| match event {
                    Event::OrderedSubmit(0, count) => Some(*count),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(
                observed,
                if rows == 1 {
                    [vec![64; 10], vec![if enabled { 48 } else { 12 }]].concat()
                } else {
                    [11, 6].repeat(36)
                }
            );
            groups += observed.len();
            transport.events.borrow_mut().clear();
            pool.commit_batch(&batch, output.completion).unwrap();
        }
        assert_eq!(groups, 1973);
        assert_eq!(output_count, 128);
        assert_eq!(driver.completed_batches(), 135);
        assert_eq!(
            driver.dispatch_counts(),
            [if enabled { 92_283 } else { 87_711 }]
        );
        assert_eq!(driver.kv_append_mode(), "parallel-c1-v19");
        assert_eq!(driver.prefill_kv_mode(), "parallel-prefill16-v27");
        assert_eq!(driver.split_attention_mode(), "split8-v21");
        assert_eq!(driver.partial_gemv_mode(), "baseline");
        assert_eq!(driver.packed_gate_up_weight_bytes(), 7_247_757_312);
        assert_eq!(driver.packed_gate_up_activation_scratch_bytes(), 8192);
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
        assert_eq!(
            *driver.inner.transports[0].events.borrow(),
            [Event::Close(0)]
        );
    }
}

#[path = "packed_gate_up_setup_r3.rs"]
mod setup_r3;
