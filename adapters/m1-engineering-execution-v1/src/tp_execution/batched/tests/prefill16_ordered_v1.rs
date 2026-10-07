//! Recording-transport identity and failure checks, not GPU numerical evidence.

use super::super::super::reduction::{AttentionProducerSchedule, ReductionWorkspace};
use super::prefill_kv_copy_v28::{composition_configured, select_composition};
use super::*;

fn configured(pool: &EngineeringTpPagedPoolV1) -> EngineeringTpBatchExecutionV2<Recording> {
    let mut driver = composition_configured(pool);
    select_composition(&mut driver, true, true, true).unwrap();
    driver.inner.transports[0].ordered64_supported = true;
    driver.configure_c1_ordered64().unwrap();
    driver
}

fn groups(driver: &EngineeringTpBatchExecutionV2<Recording>) -> Vec<usize> {
    driver.inner.transports[0]
        .events
        .borrow()
        .iter()
        .filter_map(|event| match event {
            Event::OrderedSubmit(0, count) => Some(*count),
            _ => None,
        })
        .collect()
}

fn scratch_id(driver: &EngineeringTpBatchExecutionV2<Recording>) -> u64 {
    let ReductionWorkspace::DeviceTp1(scratch) = driver.inner.reduction else {
        panic!("TP1 scratch");
    };
    scratch.id
}

#[test]
fn prefill16_ordered_preserves_every_command_address_write_read_and_fallback() {
    for rows in [1, 15, 16, 17, 32] {
        for selected in [
            Vec::new(),
            vec![0],
            vec![rows as usize - 1],
            (0..rows as usize).collect(),
        ] {
            let mut baseline = None;
            let mut baseline_groups = Vec::new();
            for selection in [None, Some(false), Some(true)] {
                let mut pool = wide_pool();
                let mut driver = configured(&pool);
                if let Some(enabled) = selection {
                    driver.configure_prefill16_ordered_v1(enabled).unwrap();
                }
                let extents = driver.inner.transports[0]
                    .buffers
                    .iter()
                    .map(|(&id, bytes)| (id, bytes.len()))
                    .collect::<Vec<_>>();
                let batch = prepare(&mut pool, rows);
                pool.begin_submission(&batch).unwrap();
                let output = driver.execute_selected(&batch, &selected).unwrap();
                let transport = &driver.inner.transports[0];
                let observed = (
                    transport.commands.clone(),
                    transport.write_payloads.clone(),
                    transport.reads.clone(),
                    transport.packet_preparations.clone(),
                    output.choices.clone(),
                    driver.inner.collective,
                    driver.inner.ranks[0].hidden.id,
                    scratch_id(&driver),
                );
                if let Some(expected) = &baseline {
                    assert_eq!(&observed, expected, "rows={rows}, selected={selected:?}");
                } else {
                    baseline = Some(observed);
                    baseline_groups = groups(&driver);
                }
                let eligible = selection == Some(true)
                    && rows == 16
                    && (selected.is_empty() || selected == [15]);
                if eligible {
                    assert_eq!(groups(&driver), [vec![64; 9], vec![36]].concat());
                } else {
                    assert_eq!(groups(&driver), baseline_groups);
                }
                assert_eq!(
                    driver.dispatch_counts(),
                    [if selected.is_empty() { 613 } else { 616 }]
                );
                assert_eq!(
                    extents,
                    transport
                        .buffers
                        .iter()
                        .map(|(&id, bytes)| (id, bytes.len()))
                        .collect::<Vec<_>>()
                );
                assert!(driver.inner.packed_c1.is_none());
                assert!(driver.inner.ordered_batches.as_ref().unwrap().is_empty());
                assert!(transport.pending.is_none() && transport.pending_ordered.is_none());
                pool.commit_batch(&batch, output.completion).unwrap();
                pool.check_invariants().unwrap();
                driver.close().unwrap();
            }
        }
    }
}

#[test]
fn prefill16_ordered_each_failed_group_commits_only_completed_metadata() {
    for ordinal in 1..=10 {
        for failure in [
            Failure::OrderedSubmitAt(ordinal),
            Failure::OrderedWaitAt(ordinal),
        ] {
            let mut pool = wide_pool();
            let mut driver = configured(&pool);
            driver.configure_prefill16_ordered_v1(true).unwrap();
            let original_hidden = driver.inner.ranks[0].hidden.id;
            let original_scratch = scratch_id(&driver);
            let completed_layer_packets = (ordinal - 1) * 64;
            let residuals = (0..36)
                .flat_map(|layer| [11 + 17 * layer, 17 + 17 * layer])
                .filter(|packet| *packet <= completed_layer_packets)
                .count();
            let mut committed = driver.inner.collective;
            for _ in 0..residuals {
                let key = committed.expected();
                committed.arrive(0, key).unwrap();
                committed.advance().unwrap();
            }
            driver.inner.transports[0].failure = Some(failure);
            let batch = prepare(&mut pool, 16);
            pool.begin_submission(&batch).unwrap();
            assert!(
                driver.execute_selected(&batch, &[15]).is_err(),
                "{failure:?}"
            );
            assert_eq!(
                driver.dispatch_counts(),
                [1 + completed_layer_packets as u64]
            );
            assert_eq!(driver.inner.collective, committed, "{failure:?}");
            assert_eq!(
                driver.inner.ranks[0].hidden.id,
                if residuals.is_multiple_of(2) {
                    original_hidden
                } else {
                    original_scratch
                }
            );
            assert_eq!(
                scratch_id(&driver),
                if residuals.is_multiple_of(2) {
                    original_scratch
                } else {
                    original_hidden
                }
            );
            assert!(driver.poisoned && driver.inner.packed_c1.is_none());
            assert!(driver.inner.ordered_batches.as_ref().unwrap().is_empty());
            assert!(driver.inner.transports[0].reads.is_empty());
            assert_eq!(driver.completed_batches(), 0);
            let events = driver.inner.transports[0].events.borrow().clone();
            assert!(driver.execute_selected(&batch, &[15]).is_err());
            assert_eq!(*driver.inner.transports[0].events.borrow(), events);
            pool.quarantine_batch(&batch).unwrap();
            assert!(pool.stats().quarantined_pages > 0);
            driver.close().unwrap();
        }
    }
}

#[test]
fn prefill16_ordered_incomplete_residual_rejects_before_a_full_queue_is_published() {
    let mut driver = configured(&wide_pool());
    driver.configure_prefill16_ordered_v1(true).unwrap();
    driver.inner.hidden = vec![0; 16 * 4096];
    driver.inner.begin_packed_prefill16().unwrap();
    driver.inner.packed_c1.as_mut().unwrap().producer_packets = 9;
    let command = dispatch("unpublished-sentinel", 1, Vec::new());
    *driver.inner.ordered_batches.as_mut().unwrap() = vec![command; 64];
    let before = (
        driver.inner.collective,
        driver.inner.ranks[0].hidden.id,
        scratch_id(&driver),
    );
    assert!(
        driver
            .inner
            .reduce_device_tp1(0, Qwen3TensorParallelCollectiveV1::AttentionOutputSum)
            .is_err()
    );
    assert_eq!(
        (
            driver.inner.collective,
            driver.inner.ranks[0].hidden.id,
            scratch_id(&driver)
        ),
        before
    );
    assert_eq!(driver.inner.ordered_batches.as_ref().unwrap().len(), 64);
    assert!(groups(&driver).is_empty());
    assert!(driver.inner.finish_packed_c1().is_err());
    driver.close().unwrap();
    assert!(driver.inner.packed_c1.is_none());
    assert!(driver.inner.ordered_batches.as_ref().unwrap().is_empty());
}

#[test]
fn prefill16_ordered_readback_failure_retains_completed_device_state_and_poison() {
    let mut pool = wide_pool();
    let mut driver = configured(&pool);
    driver.configure_prefill16_ordered_v1(true).unwrap();
    driver.inner.transports[0].failure = Some(Failure::BadChoice);
    let batch = prepare(&mut pool, 16);
    pool.begin_submission(&batch).unwrap();
    assert!(driver.execute_selected(&batch, &[15]).is_err());
    assert_eq!(driver.dispatch_counts(), [616]);
    assert_eq!(driver.inner.collective.expected().epoch, 1);
    assert_eq!(driver.completed_batches(), 0);
    assert!(driver.poisoned && driver.inner.packed_c1.is_none());
    assert_eq!(driver.inner.transports[0].reads.len(), 1);
    pool.quarantine_batch(&batch).unwrap();
    driver.close().unwrap();
}

#[test]
fn prefill16_ordered_selection_is_explicit_atomic_and_rejects_incompatible_profiles() {
    for enabled in [false, true] {
        let mut driver = configured(&wide_pool());
        assert!(driver.prefill16_ordered_v1.is_none());
        assert_eq!(driver.prefill16_ordered_mode_v1(), "residual-frontiers-v1");
        driver.configure_prefill16_ordered_v1(enabled).unwrap();
        assert_eq!(
            driver.prefill16_ordered_mode_v1(),
            if enabled {
                "prefill16-ordered64-v1"
            } else {
                "residual-frontiers-v1"
            }
        );
        assert!(driver.configure_prefill16_ordered_v1(enabled).is_err());
        assert!(driver.configure_prefill16_ordered_v1(!enabled).is_err());
        driver.close().unwrap();
    }
    for case in 0..20 {
        let mut driver = configured(&wide_pool());
        match case {
            0 => driver.inner.ordered_batch_width = OrderedBatchWidth::Packets16,
            1 => driver.inner.transports[0].ordered64_supported = false,
            2 => driver.inner.transports[0].token_program_supported = true,
            3 => driver.inner.transports[0].argmax_peer = Some((0, 1, 1)),
            4 => driver.prefill_kv_copy_v28 = Some(false),
            5 => driver.c1_split_attention_v25 = Some(false),
            6 => driver.c1_packet_packing_v22 = Some(false),
            7 => driver.partial_gemv_v28 = Some(true),
            8 => driver.prefill32_pages_v1 = Some(true),
            9 => driver.inner.large_kv = true,
            10 => driver.inner.draft_v10 = true,
            11 => driver.inner.sequences = Some(vec![Vec::new()]),
            12 => driver.inner.ordered_batches = None,
            13 => driver.inner.reduction = ReductionWorkspace::Baseline,
            14 => driver.row_capacity = 16,
            15 => driver.admitted_prefill_kv_copy_v27 = None,
            16 => driver.admitted_query_hoist_v14 = None,
            17 => driver.last_batch = 1,
            18 => driver.poisoned = true,
            19 => driver.inner.closed = true,
            _ => unreachable!(),
        }
        let before = driver.inner.transports[0].events.borrow().clone();
        let allocations = driver.inner.transports[0].buffers.len();
        assert!(
            driver.configure_prefill16_ordered_v1(true).is_err(),
            "case={case}"
        );
        assert!(driver.prefill16_ordered_v1.is_none());
        assert_eq!(*driver.inner.transports[0].events.borrow(), before);
        assert_eq!(driver.inner.transports[0].buffers.len(), allocations);
    }
}

#[test]
fn prefill16_ordered_changed_profile_rejects_before_metadata_or_dispatch() {
    let mut pool = wide_pool();
    let mut driver = configured(&pool);
    driver.configure_prefill16_ordered_v1(true).unwrap();
    driver.inner.transports[0].token_program_supported = true;
    let before = (
        driver.inner.transports[0].commands.clone(),
        driver.inner.transports[0].write_payloads.clone(),
    );
    let batch = prepare(&mut pool, 16);
    pool.begin_submission(&batch).unwrap();
    assert!(driver.execute_selected(&batch, &[15]).is_err());
    assert_eq!(
        (
            driver.inner.transports[0].commands.clone(),
            driver.inner.transports[0].write_payloads.clone()
        ),
        before
    );
    assert!(driver.poisoned && driver.inner.packed_c1.is_none());
    assert_eq!(driver.dispatch_counts(), [0]);
    pool.quarantine_batch(&batch).unwrap();
    driver.close().unwrap();
}

#[test]
fn prefill16_ordered_eight_chunks_are_eighty_groups_and_decode_rollover_is_unchanged() {
    let mut baseline = None;
    for enabled in [false, true] {
        let limits = EngineeringTpPagedLimitsV1::new(512, 32, 32, 100).unwrap();
        let mut pool = EngineeringTpPagedPoolV1::new_wide32(wide_pool().scope(), limits).unwrap();
        let mut driver = configured(&pool);
        driver.configure_prefill16_ordered_v1(enabled).unwrap();
        driver.inner.transports[0].rollover_supported = true;
        let sequence = pool
            .open_sequence(pool.scope(), &[1; 128], 0)
            .unwrap()
            .sequence();
        let mut choices = Vec::new();
        for start in (0..128).step_by(16) {
            driver.inner.transports[0].queue_packets =
                fe2o3_kfd::engineering_wire::MAX_UNRETIRED_RING_PACKETS_V1 - 600;
            let rows = (start..start + 16)
                .map(|position| EngineeringTpPageRowV1 {
                    sequence,
                    token: 1,
                    position,
                })
                .collect::<Vec<_>>();
            let batch = pool.reserve_batch(&rows).unwrap();
            pool.begin_submission(&batch).unwrap();
            let output = driver
                .execute_selected(&batch, if start == 112 { &[15] } else { &[] })
                .unwrap();
            choices.extend(output.choices);
            pool.commit_batch(&batch, output.completion).unwrap();
            pool.check_invariants().unwrap();
            assert!(driver.inner.packed_c1.is_none());
            assert!(driver.inner.ordered_batches.as_ref().unwrap().is_empty());
        }
        assert_eq!(groups(&driver).len(), if enabled { 80 } else { 576 });
        assert_eq!(driver.dispatch_counts(), [4907]);
        driver.inner.transports[0].queue_packets =
            fe2o3_kfd::engineering_wire::MAX_UNRETIRED_RING_PACKETS_V1 - 600;
        let batch = pool
            .reserve_batch(&[EngineeringTpPageRowV1 {
                sequence,
                token: 42,
                position: 128,
            }])
            .unwrap();
        pool.begin_submission(&batch).unwrap();
        let output = driver.execute_selected(&batch, &[0]).unwrap();
        choices.extend(output.choices);
        pool.commit_batch(&batch, output.completion).unwrap();
        pool.check_invariants().unwrap();
        assert_eq!(driver.dispatch_counts(), [4907 + 652]);
        assert_eq!(
            &groups(&driver)[if enabled { 80 } else { 576 }..],
            &[vec![64; 10], vec![12]].concat()
        );
        assert_eq!(driver.inner.transports[0].queue_epochs, 9);
        let transport = &driver.inner.transports[0];
        let observed = (
            transport.commands.clone(),
            transport.write_payloads.clone(),
            transport.reads.clone(),
            transport.packet_preparations.clone(),
            choices,
            driver.inner.collective,
            driver.inner.ranks[0].hidden.id,
            scratch_id(&driver),
        );
        if let Some(expected) = &baseline {
            assert_eq!(&observed, expected);
        } else {
            baseline = Some(observed);
        }
        assert!(driver.configure_prefill16_ordered_v1(!enabled).is_err());
        driver.close().unwrap();
    }
}

#[test]
fn prefill16_ordered_inner_begin_rejects_non16_or_existing_staged_state() {
    for rows in [1, 15, 17, 32] {
        let mut driver = configured(&wide_pool());
        driver.inner.hidden = vec![0; rows * 4096];
        assert!(driver.inner.begin_packed_prefill16().is_err());
        assert!(driver.inner.packed_c1.is_none());
        assert!(groups(&driver).is_empty());
    }
    let mut driver = configured(&wide_pool());
    driver.inner.hidden = vec![0; 4096];
    driver
        .inner
        .begin_packed_c1(AttentionProducerSchedule::Baseline)
        .unwrap();
    driver.inner.hidden.resize(16 * 4096, 0);
    assert!(driver.inner.begin_packed_prefill16().is_err());
    assert!(driver.inner.packed_c1.is_some());
    assert!(groups(&driver).is_empty());
    driver.close().unwrap();
}

#[test]
fn prefill16_ordered_mixed_sequences_keep_original_residual_frontiers() {
    let mut baseline = None;
    for enabled in [false, true] {
        let mut pool = wide_pool();
        let mut driver = configured(&pool);
        driver.configure_prefill16_ordered_v1(enabled).unwrap();
        let mut rows = Vec::new();
        for token in [1, 2] {
            let sequence = pool
                .open_sequence(pool.scope(), &[token; 8], 0)
                .unwrap()
                .sequence();
            rows.extend((0..8).map(|position| EngineeringTpPageRowV1 {
                sequence,
                token,
                position,
            }));
        }
        let batch = pool.reserve_batch(&rows).unwrap();
        pool.begin_submission(&batch).unwrap();
        let output = driver.execute_selected(&batch, &[15]).unwrap();
        assert_eq!(groups(&driver), [11, 6].repeat(36));
        let transport = &driver.inner.transports[0];
        let observed = (
            transport.commands.clone(),
            transport.write_payloads.clone(),
            transport.reads.clone(),
            output.choices.clone(),
        );
        if let Some(expected) = &baseline {
            assert_eq!(&observed, expected);
        } else {
            baseline = Some(observed);
        }
        pool.commit_batch(&batch, output.completion).unwrap();
        pool.check_invariants().unwrap();
        driver.close().unwrap();
    }
}
