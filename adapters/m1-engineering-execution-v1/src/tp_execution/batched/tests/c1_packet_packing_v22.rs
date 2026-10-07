//! Host command equivalence and publication failure checks, not native numerical evidence.

use super::super::super::reduction::ReductionWorkspace;
use super::*;
use crate::tp_artifact::{Fp32ArgmaxBindingV11, QueryHoistBindingV14, WaveRmsNormBindingV15};

#[cfg(feature = "c1-ordered64")]
#[path = "c1_ordered64.rs"]
mod ordered64;

fn configured(pool: &EngineeringTpPagedPoolV1) -> EngineeringTpBatchExecutionV2<Recording> {
    let mut driver = super::wave_rmsnorm_v15::configured(pool);
    driver.admitted_query_hoist_v14 = Some(QueryHoistBindingV14::recording());
    driver
        .configure_ordered_c1_wave_target_bindings_v17(
            Fp32ArgmaxBindingV11::recording(),
            QueryHoistBindingV14::recording(),
            WaveRmsNormBindingV15::recording(),
            EngineeringTpWaveTargetModeV17::Combined,
        )
        .unwrap();
    driver
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

fn scratch_id(driver: &EngineeringTpBatchExecutionV2<Recording>) -> u64 {
    let ReductionWorkspace::DeviceTp1(scratch) = driver.inner.reduction else {
        panic!("TP1 scratch");
    };
    scratch.id
}

#[test]
fn c1_packet_packing_v22_preserves_every_command_buffer_address_and_non_c1_path() {
    for rows in [1, 16, 17, 32] {
        for selected in [
            Vec::new(),
            vec![rows as usize - 1],
            (0..rows as usize).collect(),
        ] {
            let mut baseline = None;
            for selection in [None, Some(false), Some(true)] {
                let mut pool = wide_pool();
                let mut driver = configured(&pool);
                let allocations = driver.inner.transports[0]
                    .buffers
                    .iter()
                    .map(|(&id, bytes)| (id, bytes.len()))
                    .collect::<Vec<_>>();
                if let Some(enabled) = selection {
                    driver.configure_c1_packet_packing_v22(enabled).unwrap();
                }
                let batch = prepare(&mut pool, rows);
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
                    scratch_id(&driver),
                );
                if let Some(expected) = &baseline {
                    assert_eq!(
                        &recorded, expected,
                        "rows {rows}, selected {selected:?}, mode {selection:?}"
                    );
                } else {
                    baseline = Some(recorded);
                }
                assert_eq!(
                    driver.dispatch_counts(),
                    [if selected.is_empty() { 613 } else { 616 }]
                );
                if selection == Some(true) && rows == 1 && selected == [0] {
                    let mut expected = vec![16; 38];
                    expected.push(8);
                    assert_eq!(groups(&driver), expected);
                } else {
                    assert_eq!(groups(&driver), [11, 6].repeat(36));
                }
                assert_eq!(
                    allocations,
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
fn c1_packet_packing_v22_each_failed_group_commits_only_completed_metadata_and_poisoned_state() {
    for ordinal in 1..=39 {
        for failure in [
            Failure::OrderedSubmitAt(ordinal),
            Failure::OrderedWaitAt(ordinal),
        ] {
            let mut pool = wide_pool();
            let mut driver = configured(&pool);
            driver.configure_c1_packet_packing_v22(true).unwrap();
            let original_hidden = driver.inner.ranks[0].hidden.id;
            let original_scratch = scratch_id(&driver);
            let mut expected = driver.inner.collective;
            let completed_packets = (ordinal - 1) * 16;
            let residuals = (0..36)
                .flat_map(|layer| [12 + 17 * layer, 18 + 17 * layer])
                .filter(|packet| *packet <= completed_packets)
                .count();
            for _ in 0..residuals {
                let key = expected.expected();
                expected.arrive(0, key).unwrap();
                expected.advance().unwrap();
            }
            driver.inner.transports[0].failure = Some(failure);
            let batch = prepare(&mut pool, 1);
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
            assert!(driver.inner.transports[0].reads.is_empty());
            assert_eq!(driver.completed_batches(), 0);
            assert!(driver.poisoned && driver.inner.packed_c1.is_none());
            assert!(driver.inner.ordered_batches.as_ref().unwrap().is_empty());
            assert!(driver.execute_selected(&batch, &[0]).is_err());
            pool.quarantine_batch(&batch).unwrap();
            assert_eq!(pool.stats().quarantined_pages, 4);
            driver.close().unwrap();
        }
    }
}

#[test]
fn c1_packet_packing_v22_readback_failure_does_not_claim_device_rollback() {
    let mut pool = wide_pool();
    let mut driver = configured(&pool);
    driver.configure_c1_packet_packing_v22(true).unwrap();
    driver.inner.transports[0].failure = Some(Failure::BadChoice);
    let batch = prepare(&mut pool, 1);
    pool.begin_submission(&batch).unwrap();
    assert!(driver.execute_selected(&batch, &[0]).is_err());
    assert_eq!(driver.dispatch_counts(), [616]);
    assert_eq!(driver.inner.collective.expected().epoch, 1);
    assert_eq!(driver.completed_batches(), 0);
    assert!(driver.poisoned && driver.inner.packed_c1.is_none());
    assert_eq!(driver.inner.transports[0].reads.len(), 1);
    pool.quarantine_batch(&batch).unwrap();
    driver.close().unwrap();
}

#[test]
fn c1_packet_packing_v22_rejects_incomplete_producer_before_residual_publication() {
    let mut driver = configured(&wide_pool());
    driver.configure_c1_packet_packing_v22(true).unwrap();
    driver.inner.hidden = vec![0; 4096];
    driver
        .inner
        .begin_packed_c1(super::super::super::reduction::AttentionProducerSchedule::Baseline)
        .unwrap();
    driver.inner.packed_c1.as_mut().unwrap().producer_packets = 9;
    let committed = driver.inner.collective;
    let hidden = driver.inner.ranks[0].hidden.id;
    assert!(
        driver
            .inner
            .reduce_device_tp1(0, Qwen3TensorParallelCollectiveV1::AttentionOutputSum)
            .is_err()
    );
    assert_eq!(driver.inner.collective, committed);
    assert_eq!(driver.inner.ranks[0].hidden.id, hidden);
    assert!(groups(&driver).is_empty());
    assert!(driver.inner.ordered_batches.as_ref().unwrap().is_empty());
    assert!(driver.inner.finish_packed_c1().is_err());
    driver.close().unwrap();
    assert!(driver.inner.packed_c1.is_none());
}

#[test]
fn c1_packet_packing_v22_alternates_with_multirow_and_headless_batches_across_rollover() {
    let mut baseline = None;
    for enabled in [false, true] {
        let mut pool = wide_pool();
        let mut driver = configured(&pool);
        driver.configure_c1_packet_packing_v22(enabled).unwrap();
        driver.inner.transports[0].rollover_supported = true;
        let mut choices = Vec::new();
        for (ordinal, (rows, selected)) in
            [(16, Vec::new()), (1, vec![0]), (32, vec![31]), (1, vec![0])]
                .into_iter()
                .enumerate()
        {
            driver.inner.transports[0].queue_packets =
                fe2o3_kfd::engineering_wire::MAX_UNRETIRED_RING_PACKETS_V1 - 600;
            let batch = prepare(&mut pool, rows);
            pool.begin_submission(&batch).unwrap();
            let output = driver.execute_selected(&batch, &selected).unwrap();
            choices.push(output.choices);
            pool.commit_batch(&batch, output.completion).unwrap();
            pool.retire_sequence(batch.rows()[0].sequence(), false, 0)
                .unwrap();
            pool.check_invariants().unwrap();
            assert_eq!(driver.inner.collective.expected().epoch, ordinal as u64 + 1);
            assert!(driver.inner.packed_c1.is_none());
            assert!(driver.inner.ordered_batches.as_ref().unwrap().is_empty());
        }
        let transport = &driver.inner.transports[0];
        assert_eq!(transport.packet_preparations, [616; 4]);
        assert_eq!(transport.queue_epochs, 4);
        assert_eq!(driver.dispatch_counts(), [613 + 3 * 616]);
        let recorded = (
            transport.commands.clone(),
            transport.reads.clone(),
            transport.write_payloads.clone(),
            choices,
        );
        if let Some(expected) = &baseline {
            assert_eq!(&recorded, expected);
        } else {
            baseline = Some(recorded);
        }
        assert_eq!(
            groups(&driver).len(),
            if enabled { 72 * 2 + 39 * 2 } else { 72 * 4 }
        );
        driver.close().unwrap();
    }
}

#[test]
fn c1_packet_packing_v22_selection_is_explicit_atomic_and_terminal() {
    for enabled in [false, true] {
        let mut driver = configured(&wide_pool());
        assert!(driver.c1_packet_packing_v22.is_none());
        assert_eq!(driver.c1_packet_mode(), "baseline");
        driver.configure_c1_packet_packing_v22(enabled).unwrap();
        assert_eq!(
            driver.c1_packet_mode(),
            if enabled { "packed16-v22" } else { "baseline" }
        );
        assert!(driver.configure_c1_packet_packing_v22(enabled).is_err());
        assert!(driver.configure_c1_packet_packing_v22(!enabled).is_err());
        driver.close().unwrap();
    }
    for mutation in 0..28 {
        let mut driver = configured(&wide_pool());
        match mutation {
            0 => driver.admitted_query_hoist_v14 = None,
            1 => driver.query_hoist_v14 = None,
            2 => driver.admitted_wave_rmsnorm_v15 = None,
            3 => driver.wave_rmsnorm_v15 = None,
            4 => driver.admitted_argmax_v11 = None,
            5 => driver.fp32_argmax_v11 = None,
            6 => driver.row_capacity = 16,
            7 => driver.inner.row_capacity = 16,
            8 => driver.inner.draft_v10 = true,
            9 => driver.inner.large_kv = true,
            10 => driver.inner.sequences = Some(vec![Vec::new()]),
            11 => driver.inner.ordered_batches = None,
            12 => driver.wave_attention = false,
            13 => driver.c1_wave_layers = false,
            14 => driver.prune_output_head = false,
            15 => driver.fp32_logits = None,
            16 => driver.last_batch = 1,
            17 => driver.completed_batches = 1,
            18 => driver.poisoned = true,
            19 => driver.inner.closed = true,
            20 => driver.inner.transports[0].ordered_supported = false,
            21 => driver.inner.transports[0].argmax_peer = Some((1, 0, 1)),
            22 => driver.inner.reduction = ReductionWorkspace::Baseline,
            23 => driver.inner.plan = Qwen3TensorParallelPlanV1::new(target(), 8).unwrap(),
            24 => {
                driver.projection.mode =
                    super::super::super::EngineeringTpProjectionModeV3::Baseline;
            }
            25 => driver.head_profile_configured = false,
            26 => {
                let mut binding = QueryHoistBindingV14::recording();
                binding.hsaco[0] ^= 1;
                driver.admitted_query_hoist_v14 = Some(binding);
            }
            27 => driver.inner.ranks.clear(),
            _ => unreachable!(),
        }
        let commands = driver.inner.transports[0].commands.clone();
        let allocations = driver.inner.transports[0].buffers.len();
        assert!(
            driver.configure_c1_packet_packing_v22(true).is_err(),
            "mutation {mutation}"
        );
        assert!(driver.c1_packet_packing_v22.is_none());
        assert!(driver.inner.packed_c1.is_none());
        assert_eq!(driver.inner.transports[0].commands, commands);
        assert_eq!(driver.inner.transports[0].buffers.len(), allocations);
    }
}
