//! Recording contracts only; these do not emulate kernel arithmetic or prove native parity.

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

fn select(driver: &mut EngineeringTpBatchExecutionV2<Recording>, enabled: bool) -> TpResult<()> {
    driver.configure_ordered64_kv_copy_bindings_v1(
        Fp32ArgmaxBindingV11::recording(),
        QueryHoistBindingV14::recording(),
        WaveRmsNormBindingV15::recording(),
        PrefillKvCopyBindingV27::recording(),
        SplitAttentionBindingV21::recording(),
        GemvPrefetchBindingV20::recording(),
        C1KvCopyBindingV19::recording(),
        enabled,
    )
}

#[test]
fn ordered64_kv_copy_build_matrix_is_closed_before_selection() {
    let mut driver = configured(&wide_pool());
    let result = select(&mut driver, true);
    assert_eq!(
        result.is_ok(),
        cfg!(all(
            feature = "c1-ordered64",
            not(feature = "model-timestamps")
        ))
    );
    if result.is_err() {
        assert!(driver.c1_kv_copy_v19.is_none());
        assert!(driver.prefill_kv_copy_v28.is_none());
        assert!(driver.fp32_argmax_v11.is_none());
        assert!(!driver.inner.ordered_batch_width.is_wide());
        assert!(driver.inner.transports[0].commands.is_empty());
    }
}

#[cfg(all(feature = "c1-ordered64", not(feature = "model-timestamps")))]
mod enabled {
    use super::*;
    const COPY: &str = crate::tp_artifact::ENGINEERING_TP_C1_KV_COPY_EXPORTS_V19[0];
    const PREFILL: &str = crate::tp_artifact::ENGINEERING_TP_PREFILL_KV_COPY_EXPORTS_V27[0];
    const LEGACY: &str = "ferric_qwen3_tp_batch32_paged_kv_append_v5";

    #[test]
    fn ordered64_kv_copy_admission_is_exact_nonpeer_and_preallocation() {
        let binding = C1KvCopyBindingV19::recording();
        let mut driver = configured(&wide_pool());
        let transport = &mut driver.inner.transports[0];
        transport.buffers.clear();
        let next = transport.next;
        let admit = super::super::super::ordered64_kv_copy_v1::admit;
        assert!(admit(std::slice::from_mut(transport), binding).is_err());
        transport.c1_kv_copy_v19_loaded = Some(binding.hsaco);
        let mut wrong = binding;
        wrong.hsaco[0] ^= 1;
        assert!(admit(std::slice::from_mut(transport), wrong).is_err());
        transport.ordered64_supported = false;
        assert!(admit(std::slice::from_mut(transport), binding).is_err());
        transport.ordered64_supported = true;
        transport.argmax_peer = Some((1, 0, 1));
        assert!(admit(std::slice::from_mut(transport), binding).is_err());
        transport.argmax_peer = None;
        admit(std::slice::from_mut(transport), binding).unwrap();
        assert!(transport.buffers.is_empty() && transport.commands.is_empty());
        assert_eq!(transport.next, next);
    }

    #[test]
    fn ordered64_kv_copy_complete_request_preserves_87711_packets_and_1973_groups() {
        let mut reference = None;
        for enabled in [false, true] {
            let limits = EngineeringTpPagedLimitsV1::new(256, 32, 16, 100).unwrap();
            let mut pool =
                EngineeringTpPagedPoolV1::new_wide32(wide_pool().scope(), limits).unwrap();
            let mut driver = configured(&pool);
            select(&mut driver, enabled).unwrap();
            assert_eq!(driver.c1_packet_mode(), "packed64-v29");
            assert_eq!(driver.prefill_kv_mode(), "parallel-prefill16-v27");
            assert_eq!(driver.split_attention_mode(), "split8-v21");
            assert_eq!(driver.partial_gemv_mode(), "baseline");
            assert_eq!(driver.split_attention_workspace_bytes(), 133_120);
            let allocations = driver.inner.transports[0]
                .buffers
                .keys()
                .copied()
                .collect::<Vec<_>>();
            let sequence = pool
                .open_sequence(pool.scope(), &(0..256).collect::<Vec<_>>(), 0)
                .unwrap()
                .sequence();
            let mut events = Vec::new();
            let mut choices = Vec::new();
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
                    652
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
                choices.extend_from_slice(&output.choices);
                let transport = &driver.inner.transports[0];
                let commands = &transport.commands[before..];
                assert_eq!(commands.len(), usize::try_from(expected).unwrap());
                let append = if rows == 16 {
                    PREFILL
                } else if enabled {
                    COPY
                } else {
                    LEGACY
                };
                assert_eq!(
                    commands
                        .iter()
                        .filter(|command| command.kernel == append)
                        .count(),
                    36
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
                assert_eq!(
                    groups,
                    if rows == 1 {
                        [vec![64; 10], vec![12]].concat()
                    } else {
                        [11, 6].repeat(36)
                    }
                );
                events.extend(transport.events.borrow().iter().cloned());
                transport.events.borrow_mut().clear();
                assert!(driver.inner.packed_c1.is_none());
                assert!(driver.inner.ordered_batches.as_ref().unwrap().is_empty());
                pool.commit_batch(&batch, output.completion).unwrap();
            }
            assert_eq!(driver.dispatch_counts(), [87_711]);
            assert_eq!(driver.completed_batches(), 135);
            assert_eq!(choices.len(), 128);
            assert_eq!(
                events
                    .iter()
                    .filter(|event| matches!(event, Event::OrderedSubmit(0, _)))
                    .count(),
                1973
            );
            let transport = &driver.inner.transports[0];
            assert_eq!(
                transport
                    .commands
                    .iter()
                    .filter(|command| command.kernel == COPY)
                    .count(),
                if enabled { 4572 } else { 0 }
            );
            assert_eq!(
                transport
                    .commands
                    .iter()
                    .filter(|command| command.kernel == PREFILL)
                    .count(),
                288
            );
            assert_eq!(
                transport.buffers.keys().copied().collect::<Vec<_>>(),
                allocations
            );
            let normalized = transport
                .commands
                .iter()
                .map(|command| (!matches!(command.kernel, COPY | LEGACY)).then(|| command.clone()))
                .collect::<Vec<_>>();
            for event in &mut events {
                if let Event::Submit(_, kernel) | Event::Wait(_, kernel) = event
                    && *kernel == COPY
                {
                    *kernel = LEGACY;
                }
            }
            let observed = (
                normalized,
                events,
                choices,
                transport.reads.clone(),
                transport.write_payloads.clone(),
                transport.packet_preparations.clone(),
            );
            if let Some(expected) = &reference {
                assert_eq!(&observed, expected);
            } else {
                reference = Some(observed);
            }
            pool.check_invariants().unwrap();
            driver.close().unwrap();
            assert_eq!(
                *driver.inner.transports[0].events.borrow(),
                [Event::Close(0)]
            );
        }
    }

    #[test]
    fn ordered64_kv_copy_invalid_bindings_and_state_are_transactional() {
        for mutation in 0..25 {
            let mut driver = configured(&wide_pool());
            match mutation {
                0 => driver.admitted_c1_kv_copy_v19 = None,
                1 => driver.admitted_c1_kv_copy_v19.as_mut().unwrap().hsaco[0] ^= 1,
                2 => driver.admitted_prefill_kv_copy_v27 = None,
                3 => driver.admitted_partial_gemv_v20 = None,
                4 => {
                    driver
                        .split_attention_workspace_v25
                        .as_mut()
                        .unwrap()
                        .image
                        .hsaco[0] ^= 1;
                }
                5 => {
                    driver
                        .split_attention_workspace_v25
                        .as_mut()
                        .unwrap()
                        .stats
                        .elements -= 1;
                }
                6 => driver.inner.ranks[0].layers[0].k_cache.elements -= 1,
                7 => driver.inner.ranks[0].v.elements -= 1,
                8 => driver.inner.row_capacity = 16,
                9 => driver.row_capacity = 16,
                10 => driver.last_batch = 1,
                11 => driver.completed_batches = 1,
                12 => driver.poisoned = true,
                13 => driver.inner.closed = true,
                14 => driver.inner.draft_v10 = true,
                15 => driver.inner.large_kv = true,
                16 => driver.inner.transports[0].ordered64_supported = false,
                17 => driver.inner.transports[0].argmax_peer = Some((1, 0, 1)),
                18 => driver.admitted_query_hoist_v14 = None,
                19 => driver.admitted_argmax_v11 = None,
                20 => driver.admitted_wave_rmsnorm_v15 = None,
                21 => driver.c1_packet_packing_v22 = Some(false),
                22 => driver.c1_split_attention_v25 = Some(false),
                23 => driver.prefill_kv_copy_v28 = Some(false),
                24 => driver.partial_gemv_v28 = Some(false),
                _ => unreachable!(),
            }
            let before = (
                driver.prefill_kv_copy_v28,
                driver.c1_split_attention_v25,
                driver.c1_packet_packing_v22,
                driver.partial_gemv_v28,
            );
            let buffers = driver.inner.transports[0]
                .buffers
                .keys()
                .copied()
                .collect::<Vec<_>>();
            assert!(select(&mut driver, true).is_err(), "mutation {mutation}");
            assert_eq!(
                (
                    driver.prefill_kv_copy_v28,
                    driver.c1_split_attention_v25,
                    driver.c1_packet_packing_v22,
                    driver.partial_gemv_v28
                ),
                before
            );
            assert!(driver.c1_kv_copy_v19.is_none() && driver.fp32_argmax_v11.is_none());
            assert!(driver.query_hoist_v14.is_none() && driver.wave_rmsnorm_v15.is_none());
            assert!(!driver.inner.ordered_batch_width.is_wide());
            assert!(driver.inner.transports[0].commands.is_empty());
            assert_eq!(
                driver.inner.transports[0]
                    .buffers
                    .keys()
                    .copied()
                    .collect::<Vec<_>>(),
                buffers
            );
        }
    }

    #[test]
    fn ordered64_kv_copy_legacy_selectors_stay_closed_and_selection_is_immutable() {
        for enabled in [false, true] {
            let mut driver = configured(&wide_pool());
            assert!(
                super::super::prefill_kv_copy_v28::select_composition(
                    &mut driver,
                    true,
                    true,
                    true
                )
                .is_err()
            );
            assert!(
                driver
                    .validate_split_storage_v25(SplitAttentionBindingV21::recording(), true)
                    .is_err()
            );
            select(&mut driver, enabled).unwrap();
            assert!(select(&mut driver, !enabled).is_err());
            assert!(driver.configure_c1_ordered64().is_err());
            assert!(driver.configure_c1_packet_packing_v22(true).is_err());
        }
    }

    #[test]
    fn ordered64_kv_copy_prefix_cow_preserves_every_other_slot_at_page_boundary() {
        let mut pool = wide_pool();
        let mut driver = configured(&pool);
        select(&mut driver, true).unwrap();
        let batch = prepare(&mut pool, 16);
        let sequence = batch.rows()[0].sequence();
        let prefix_page = batch.rows()[0].writable_physical_page();
        pool.begin_submission(&batch).unwrap();
        let output = driver.execute_selected(&batch, &[15]).unwrap();
        pool.commit_batch(&batch, output.completion).unwrap();
        let caches = driver.inner.ranks[0]
            .layers
            .iter()
            .flat_map(|layer| [layer.k_cache.id, layer.v_cache.id])
            .collect::<Vec<_>>();
        for id in &caches {
            driver.inner.transports[0]
                .buffers
                .get_mut(id)
                .unwrap()
                .fill(0x5a);
        }
        pool.retire_sequence(sequence, true, 1).unwrap();
        let hit = pool
            .open_sequence(pool.scope(), &(0..17).collect::<Vec<_>>(), 2)
            .unwrap();
        assert_eq!(hit.hit_tokens(), 16);
        let batch = pool
            .reserve_batch(&[EngineeringTpPageRowV1 {
                sequence: hit.sequence(),
                token: 16,
                position: 16,
            }])
            .unwrap();
        let row = &batch.rows()[0];
        assert_ne!(row.writable_physical_page(), prefix_page);
        let offset =
            (row.writable_physical_page() * 16 + row.writable_token_offset()) as usize * 2048;
        let expected = caches
            .iter()
            .map(|id| {
                let mut bytes = driver.inner.transports[0].buffers[id].clone();
                bytes[offset..offset + 2048].fill(0);
                (*id, bytes)
            })
            .collect::<Vec<_>>();
        pool.begin_submission(&batch).unwrap();
        let output = driver.execute_selected(&batch, &[0]).unwrap();
        for (id, bytes) in expected {
            assert_eq!(driver.inner.transports[0].buffers[&id], bytes);
        }
        pool.commit_batch(&batch, output.completion).unwrap();
        pool.check_invariants().unwrap();
        driver.close().unwrap();
    }

    #[test]
    fn ordered64_kv_copy_each_short_context_group_failure_cannot_complete_or_retry() {
        // Position zero uses the existing V14 fallback: 616 packets in ten groups.
        for ordinal in 1..=10 {
            for failure in [
                Failure::OrderedSubmitAt(ordinal),
                Failure::OrderedWaitAt(ordinal),
            ] {
                let mut pool = wide_pool();
                let mut driver = configured(&pool);
                select(&mut driver, true).unwrap();
                driver.inner.transports[0].failure = Some(failure);
                let batch = prepare(&mut pool, 1);
                pool.begin_submission(&batch).unwrap();
                assert!(
                    driver.execute_selected(&batch, &[0]).is_err(),
                    "{failure:?}"
                );
                assert_eq!(driver.completed_batches(), 0);
                assert!(driver.poisoned && driver.inner.packed_c1.is_none());
                assert!(driver.inner.ordered_batches.as_ref().unwrap().is_empty());
                assert!(driver.inner.transports[0].reads.is_empty());
                assert!(driver.execute_selected(&batch, &[0]).is_err());
                pool.quarantine_batch(&batch).unwrap();
                driver.close().unwrap();
            }
        }
    }

    #[test]
    fn ordered64_kv_copy_foreign_and_stale_batches_cannot_write() {
        let mut pool = wide_pool();
        let mut foreign = wide_pool();
        let mut driver = configured(&pool);
        select(&mut driver, true).unwrap();
        assert!(
            driver
                .execute_selected(&prepare(&mut foreign, 1), &[0])
                .is_err()
        );
        assert!(driver.inner.transports[0].commands.is_empty());
        let batch = prepare(&mut pool, 1);
        pool.begin_submission(&batch).unwrap();
        let output = driver.execute_selected(&batch, &[0]).unwrap();
        pool.commit_batch(&batch, output.completion).unwrap();
        let before = driver.inner.transports[0].commands.clone();
        assert!(driver.execute_selected(&batch, &[0]).is_err());
        assert_eq!(driver.inner.transports[0].commands, before);
        driver.close().unwrap();
    }
}
