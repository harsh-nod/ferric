//! Authored host command/commit checks; not native execution evidence.

use super::*;

fn select(driver: &mut EngineeringTpBatchExecutionV2<Recording>) {
    driver.inner.transports[0].ordered64_supported = true;
    driver.configure_c1_packet_packing_v22(true).unwrap();
    driver.configure_c1_ordered64().unwrap();
}

#[test]
fn c1_ordered64_preserves_flattened_commands_addresses_io_and_noneligible_groups() {
    for (rows, selected) in [
        (1, vec![0]),
        (1, vec![]),
        (16, vec![15]),
        (17, vec![16]),
        (32, vec![31]),
    ] {
        let mut expected = None;
        for wide in [false, true] {
            let mut pool = wide_pool();
            let mut driver = configured(&pool);
            assert_eq!(
                driver.inner.ordered_batch_width,
                OrderedBatchWidth::Packets16
            );
            assert_eq!(driver.inner.ordered_batch_width.bound(), 16);
            assert!(!driver.inner.ordered_batch_width.is_wide());
            if wide {
                select(&mut driver);
            } else {
                driver.configure_c1_packet_packing_v22(true).unwrap();
            }
            assert_eq!(driver.inner.ordered_batch_width.is_wide(), wide);
            assert_eq!(
                driver.inner.ordered_batch_width.bound(),
                if wide { 64 } else { 16 }
            );
            let allocation_extents = driver.inner.transports[0]
                .buffers
                .iter()
                .map(|(&id, bytes)| (id, bytes.len()))
                .collect::<Vec<_>>();
            let batch = prepare(&mut pool, rows);
            pool.begin_submission(&batch).unwrap();
            let output = driver.execute_selected(&batch, &selected).unwrap();
            let transport = &driver.inner.transports[0];
            let actual = (
                transport.commands.clone(),
                transport.reads.clone(),
                transport.write_payloads.clone(),
                transport.packet_preparations.clone(),
                output.choices.clone(),
                driver.inner.collective,
                driver.inner.ranks[0].hidden.id,
                scratch_id(&driver),
            );
            if let Some(expected) = &expected {
                assert_eq!(&actual, expected, "rows={rows}, selected={selected:?}");
            } else {
                expected = Some(actual);
            }
            assert_eq!(
                driver.dispatch_counts(),
                [if selected.is_empty() { 613 } else { 616 }]
            );
            if rows == 1 && selected == [0] {
                let mut expected_groups =
                    vec![if wide { 64 } else { 16 }; if wide { 9 } else { 38 }];
                expected_groups.push(if wide { 40 } else { 8 });
                assert_eq!(groups(&driver), expected_groups);
            } else {
                assert_eq!(groups(&driver), [11, 6].repeat(36));
            }
            assert_eq!(
                allocation_extents,
                transport
                    .buffers
                    .iter()
                    .map(|(&id, bytes)| (id, bytes.len()))
                    .collect::<Vec<_>>()
            );
            assert!(driver.inner.packed_c1.is_none());
            assert!(driver.inner.ordered_batches.as_ref().unwrap().is_empty());
            pool.commit_batch(&batch, output.completion).unwrap();
            pool.check_invariants().unwrap();
            driver.close().unwrap();
        }
    }
}

#[test]
fn c1_ordered64_composed_652_packet_decode_has_identical_commands_in_eleven_groups() {
    use super::super::prefill_kv_copy_v28::{composition_configured, select_composition};
    let mut expected = None;
    for wide in [false, true] {
        let limits = EngineeringTpPagedLimitsV1::new(512, 32, 32, 100).unwrap();
        let mut pool = EngineeringTpPagedPoolV1::new_wide32(wide_pool().scope(), limits).unwrap();
        let mut driver = composition_configured(&pool);
        select_composition(&mut driver, true, true, true).unwrap();
        if wide {
            driver.inner.transports[0].ordered64_supported = true;
            driver.configure_c1_ordered64().unwrap();
        }
        let sequence = pool
            .open_sequence(pool.scope(), &[1; 128], 0)
            .unwrap()
            .sequence();
        // Prefix metadata only; the recording transport makes no numerical claim.
        for start in (0..128).step_by(32) {
            let rows = (start..start + 32)
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
        let batch = pool
            .reserve_batch(&[EngineeringTpPageRowV1 {
                sequence,
                token: 42,
                position: 128,
            }])
            .unwrap();
        assert_eq!(
            driver
                .expected_dispatch_counts_for_batch(&batch, 1)
                .unwrap(),
            [652]
        );
        pool.begin_submission(&batch).unwrap();
        let output = driver.execute_selected(&batch, &[0]).unwrap();
        let transport = &driver.inner.transports[0];
        let actual = (
            transport.commands.clone(),
            transport.write_payloads.clone(),
            transport.reads.clone(),
            output.choices.clone(),
            driver.inner.collective,
            driver.inner.ranks[0].hidden.id,
            scratch_id(&driver),
        );
        if let Some(expected) = &expected {
            assert_eq!(&actual, expected);
        } else {
            expected = Some(actual);
        }
        let mut expected_groups = vec![if wide { 64 } else { 16 }; if wide { 10 } else { 40 }];
        expected_groups.push(12);
        assert_eq!(groups(&driver), expected_groups);
        assert_eq!(driver.dispatch_counts(), [652]);
        pool.commit_batch(&batch, output.completion).unwrap();
        pool.check_invariants().unwrap();
        driver.close().unwrap();
    }
}

#[test]
#[cfg(feature = "model-timestamps")]
fn packet_ticks_preserve_composed_commands_payloads_groups_and_split_boundaries() {
    use super::super::ordered64_kv_copy_packet_ticks_v1::{configured, select};
    use crate::model_timestamps::Operation;
    use crate::tp_scheduler::{TpBatchRowKindV1, TpBatchRowV1, TpRequestIdV1};
    for (rows, position) in [(16, 0), (1, 126), (1, 127), (1, 128), (1, 255), (1, 256)] {
        let mut reference = None;
        for diagnostic in [false, true] {
            let limits = EngineeringTpPagedLimitsV1::new(512, 32, 32, 100).unwrap();
            let mut pool =
                EngineeringTpPagedPoolV1::new_wide32(wide_pool().scope(), limits).unwrap();
            let mut driver = configured(&pool);
            select(&mut driver, true).unwrap();
            if !diagnostic {
                driver.inner.transports[0].timestamp_tags = None;
                driver.inner.transports[0].timestamp_ordered64 = false;
            }
            let sequence = pool
                .open_sequence(pool.scope(), &vec![1; (position + rows) as usize], 0)
                .unwrap()
                .sequence();
            for start in (0..position).step_by(32) {
                let prefix = (start..(start + 32).min(position))
                    .map(|position| EngineeringTpPageRowV1 {
                        sequence,
                        token: 1,
                        position,
                    })
                    .collect::<Vec<_>>();
                let batch = pool.reserve_batch(&prefix).unwrap();
                pool.begin_submission(&batch).unwrap();
                pool.commit_batch(
                    &batch,
                    crate::tp_paged::EngineeringTpBatchCompletionV1::after_all_ranks(&batch),
                )
                .unwrap();
            }
            let physical = (position..position + rows)
                .map(|position| EngineeringTpPageRowV1 {
                    sequence,
                    token: 1,
                    position,
                })
                .collect::<Vec<_>>();
            let batch = pool.reserve_batch(&physical).unwrap();
            let selected = vec![(rows - 1) as usize];
            let scheduler_rows = batch
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
                    kind: if rows == 1 {
                        TpBatchRowKindV1::Decode
                    } else if index == (rows - 1) as usize {
                        TpBatchRowKindV1::PrefillFinal
                    } else {
                        TpBatchRowKindV1::PrefillIntermediate
                    },
                })
                .collect::<Vec<_>>();
            driver.bind_numerical_rows(1, &scheduler_rows).unwrap();
            pool.begin_submission(&batch).unwrap();
            let output = driver.execute_selected(&batch, &selected).unwrap();
            let transport = &driver.inner.transports[0];
            let observed = (
                transport.commands.clone(),
                transport.events.borrow().clone(),
                transport.write_payloads.clone(),
                transport.reads.clone(),
                transport.packet_preparations.clone(),
                groups(&driver),
                output.choices.clone(),
            );
            if let Some(expected) = &reference {
                assert_eq!(&observed, expected);
            } else {
                reference = Some(observed);
            }
            let split = rows == 1 && (127..=255).contains(&position);
            let packets = if split { 652 } else { 616 };
            assert_eq!(driver.dispatch_counts(), [packets]);
            if diagnostic {
                let tags = transport.timestamp_tags.as_ref().unwrap();
                assert_eq!(tags.len(), usize::try_from(packets).unwrap());
                assert_eq!(tags.len(), transport.commands.len());
                assert!(tags.iter().all(|tag| tag.batch_ordinal == 1));
                assert_eq!(
                    tags.iter()
                        .filter(|tag| tag.operation == Operation::Attention)
                        .count(),
                    if split { 72 } else { 36 }
                );
                assert_eq!(
                    tags.iter()
                        .filter(|tag| tag.operation == Operation::KvAppend)
                        .count(),
                    36
                );
                assert_eq!(transport.timestamp_batches.len(), 1);
                assert_eq!(transport.timestamp_batches[0].expected_packets, packets);
            }
            pool.commit_batch(&batch, output.completion).unwrap();
            pool.check_invariants().unwrap();
            driver.close().unwrap();
        }
    }
}

#[test]
#[cfg(feature = "model-timestamps")]
fn composed_packet_ticks_require_explicit_transport_marker_and_exact_modes() {
    use super::super::prefill_kv_copy_v28::{composition_configured, select_composition};
    use crate::tp_scheduler::{TpBatchRowKindV1, TpBatchRowV1, TpRequestIdV1};
    for (marker, copy, split, wide) in [
        (false, true, true, true),
        (true, false, true, true),
        (true, true, false, true),
        (true, true, true, false),
    ] {
        let pool = wide_pool();
        let mut driver = composition_configured(&pool);
        select_composition(&mut driver, copy, split, true).unwrap();
        driver.inner.transports[0].ordered64_supported = true;
        if wide {
            driver.configure_c1_ordered64().unwrap();
        }
        driver.inner.transports[0].timestamp_tags = Some(Vec::new());
        driver.inner.transports[0].timestamp_ordered64 = marker;
        assert!(
            driver
                .bind_numerical_rows(
                    1,
                    &[TpBatchRowV1 {
                        request: TpRequestIdV1 {
                            slot: 0,
                            generation: 1
                        },
                        token_id: 1,
                        absolute_position: 128,
                        kind: TpBatchRowKindV1::Decode
                    }]
                )
                .is_err()
        );
        assert!(driver.inner.transports[0].commands.is_empty());
        assert!(driver.inner.transports[0].timestamp_batches.is_empty());
        driver.close().unwrap();
    }
}

#[test]
#[cfg(feature = "model-timestamps")]
fn composed_packet_ticks_require_selected_v19_and_baseline_gemv() {
    use super::super::prefill_kv_copy_v28::{composition_configured, select_composition};
    use crate::tp_artifact::C1KvCopyBindingV19;
    use crate::tp_scheduler::{TpBatchRowKindV1, TpBatchRowV1, TpRequestIdV1};
    for diagnostic in [false, true] {
        for (admitted, selected, gemv) in [
            (false, false, None),
            (false, false, Some(false)),
            (true, false, Some(false)),
            (false, true, Some(false)),
            (true, true, Some(false)),
            (false, false, Some(true)),
        ] {
            let pool = wide_pool();
            let mut driver = composition_configured(&pool);
            select_composition(&mut driver, true, true, true).unwrap();
            driver.inner.transports[0].ordered64_supported = true;
            driver.configure_c1_ordered64().unwrap();
            // Exercise admission independently of the constructor's earlier exclusions.
            driver.admitted_c1_kv_copy_v19 = admitted.then(C1KvCopyBindingV19::recording);
            driver.c1_kv_copy_v19 = selected.then(C1KvCopyBindingV19::recording);
            driver.partial_gemv_v28 = gemv;
            if diagnostic {
                driver.inner.transports[0].timestamp_tags = Some(Vec::new());
                driver.inner.transports[0].timestamp_ordered64 = true;
            }
            let forbidden = !admitted || !selected || gemv != Some(false);
            let result = driver.bind_numerical_rows(
                1,
                &[TpBatchRowV1 {
                    request: TpRequestIdV1 {
                        slot: 0,
                        generation: 1,
                    },
                    token_id: 1,
                    absolute_position: 128,
                    kind: TpBatchRowKindV1::Decode,
                }],
            );
            assert_eq!(result.is_err(), diagnostic && forbidden);
            assert!(driver.inner.transports[0].commands.is_empty());
            assert_eq!(driver.dispatch_counts(), [0]);
            assert_eq!(
                driver.inner.transports[0].timestamp_batches.len(),
                usize::from(diagnostic && !forbidden)
            );
            assert!(
                driver.inner.transports[0]
                    .timestamp_tags
                    .as_ref()
                    .is_none_or(Vec::is_empty)
            );
            driver.close().unwrap();
        }
    }
}

#[test]
fn c1_ordered64_rejects_missing_capability_nonpacked_peer_and_late_selection() {
    for case in 0..6 {
        let mut pool = wide_pool();
        let mut driver = configured(&pool);
        driver.inner.transports[0].ordered64_supported = true;
        if case != 0 {
            driver.configure_c1_packet_packing_v22(case != 1).unwrap();
        }
        match case {
            2 => driver.inner.transports[0].ordered64_supported = false,
            3 => driver.inner.transports[0].argmax_peer = Some((0, 1, 1)),
            4 => driver.configure_c1_ordered64().unwrap(),
            5 => {
                let batch = prepare(&mut pool, 1);
                pool.begin_submission(&batch).unwrap();
                let output = driver.execute_selected(&batch, &[0]).unwrap();
                pool.commit_batch(&batch, output.completion).unwrap();
            }
            _ => {}
        }
        let before = (
            driver.inner.ordered_batch_width,
            driver.dispatch_counts(),
            groups(&driver),
        );
        assert!(driver.configure_c1_ordered64().is_err(), "case={case}");
        assert_eq!(
            (
                driver.inner.ordered_batch_width,
                driver.dispatch_counts(),
                groups(&driver)
            ),
            before
        );
        driver.close().unwrap();
    }
}

#[test]
fn c1_ordered64_failed_group_only_commits_prior_completed_metadata() {
    for ordinal in 1..=10 {
        for failure in [
            Failure::OrderedSubmitAt(ordinal),
            Failure::OrderedWaitAt(ordinal),
        ] {
            let mut pool = wide_pool();
            let mut driver = configured(&pool);
            select(&mut driver);
            let original_hidden = driver.inner.ranks[0].hidden.id;
            let original_scratch = scratch_id(&driver);
            let completed_packets = (ordinal - 1) * 64;
            let residuals = (0..36)
                .flat_map(|layer| [12 + 17 * layer, 18 + 17 * layer])
                .filter(|packet| *packet <= completed_packets)
                .count();
            let mut collective = driver.inner.collective;
            for _ in 0..residuals {
                let key = collective.expected();
                collective.arrive(0, key).unwrap();
                collective.advance().unwrap();
            }
            driver.inner.transports[0].failure = Some(failure);
            let batch = prepare(&mut pool, 1);
            pool.begin_submission(&batch).unwrap();
            assert!(driver.execute_selected(&batch, &[0]).is_err());
            assert_eq!(driver.dispatch_counts(), [completed_packets as u64]);
            assert_eq!(driver.inner.collective, collective);
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
            assert!(driver.execute_selected(&batch, &[0]).is_err());
            pool.quarantine_batch(&batch).unwrap();
            driver.close().unwrap();
        }
    }
}
