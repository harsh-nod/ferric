use super::*;
use std::io::{Cursor, Read};

use wire::{BufferAccessV1, DispatchTimestampTicksV1, PointerFixupV1};

fn entry() -> OrderedBatchDispatchV1 {
    OrderedBatchDispatchV1 {
        kernel: 99,
        payload_bytes: 4,
        workgroup: [64, 1, 1],
        grid: [256, 1, 1],
        pointers: vec![PointerFixupV1 {
            kernarg_offset: 0,
            buffer: u64::MAX,
            buffer_offset: 16,
            extent_bytes: 32,
            access: BufferAccessV1::Read,
        }],
    }
}

fn ordered(count: usize) -> CommandV1 {
    CommandV1::DispatchOrderedBatch {
        dispatches: vec![entry(); count],
        timeout_ms: 60_000,
    }
}

fn single() -> CommandV1 {
    let item = entry();
    CommandV1::Dispatch {
        kernel: item.kernel,
        payload_bytes: item.payload_bytes,
        workgroup: item.workgroup,
        grid: item.grid,
        pointers: item.pointers,
        timeout_ms: 60_000,
    }
}

fn tag() -> PacketTag {
    PacketTag {
        batch_ordinal: 1,
        layer: Some(3),
        operation: Operation::QueryProjection,
    }
}

fn initialized() -> Collector {
    let mut collector = Collector::new(7, 36).unwrap();
    collector
        .register_kernel(KernelIdentity {
            kernel: 99,
            symbol: "actual_loaded_gemv".into(),
            object_sha256: [42; 32],
        })
        .unwrap();
    collector
}

fn fixture(count: u64) -> Collector {
    let mut collector = initialized();
    collector
        .begin_batch(BatchIdentity {
            ordinal: 1,
            scheduler_batch_id: 900,
            request: [0, 1],
            prefill_rows: 0,
            decode_rows: 1,
            expected_packets: count,
        })
        .unwrap();
    collector
}

fn begin(collector: &mut Collector, count: usize) -> CommandV1 {
    collector
        .begin_group(ordered(count), vec![tag(); count], &vec![0; 4 * count])
        .unwrap()
}

fn response(collector: &Collector, count: usize) -> ResponseV1 {
    ResponseV1::DispatchOrderedBatch64ProfiledCompleted {
        device_unique_id: collector.device,
        queue_epoch: collector.epoch,
        elapsed_ns: 12345,
        timestamps: (0..count)
            .map(|index| DispatchTimestampTicksV1 {
                packet_id: collector.frontier + index as u64,
                kernel: 99,
                start_tick: 100 + index as u64,
                end_tick: 120 + index as u64,
            })
            .collect(),
    }
}

#[test]
fn original_ordered_group_is_not_expanded_or_repacked() {
    for count in [1, 16] {
        let mut collector = fixture(count as u64);
        let command = begin(&mut collector, count);
        assert_eq!(
            command,
            CommandV1::DispatchOrderedBatch64Profiled {
                dispatches: vec![entry(); count],
                timeout_ms: 60_000,
            }
        );
        collector
            .complete_group(&response(&collector, count), &[])
            .unwrap();
        let capture = collector.finish(count as u64).unwrap();
        assert_eq!(capture.groups, [[0, 1, count as u64]]);
        assert_eq!(capture.records.len(), count);
    }
}

fn wide_fixture(count: u64) -> Collector {
    let mut collector = fixture(count);
    collector.max_group_packets = MAX_ORDERED64_GROUP_PACKETS;
    collector
}

fn wide_command(count: usize) -> CommandV1 {
    CommandV1::DispatchOrderedBatch64 {
        dispatches: vec![entry(); count],
        timeout_ms: 60_000,
    }
}

#[test]
fn ordered64_keeps_original_groups_records_and_uncalibrated_units() {
    assert_eq!(
        Collector::new_ordered64(7, 36).unwrap().max_group_packets,
        64
    );
    for count in [1, 11, 12, 17, 64] {
        let mut collector = wide_fixture(count as u64);
        let original = wide_command(count);
        let profiled = collector
            .begin_group(original, vec![tag(); count], &vec![0; 4 * count])
            .unwrap();
        assert_eq!(
            profiled,
            CommandV1::DispatchOrderedBatch64Profiled {
                dispatches: vec![entry(); count],
                timeout_ms: 60_000,
            }
        );
        collector
            .complete_group(&response(&collector, count), &[])
            .unwrap();
        let capture = collector.finish(count as u64).unwrap();
        assert_eq!(capture.groups, [[0, 2, count as u64]]);
        assert_eq!(capture.records.len(), count);
        let value: serde_json::Value =
            serde_json::from_slice(&capture.to_bounded_json().unwrap()).unwrap();
        assert_eq!(value["schema"], "FerricOrdered64RawPacketIntervalsV1");
        assert_eq!(value["original_max_group_packets"], 64);
        assert_eq!(
            value["group_columns"][1],
            "original_publication_0_single_2_ordered64"
        );
        assert_eq!(value["tick_unit"], "raw_device_ticks_frequency_unspecified");
    }
}

#[test]
fn ordered64_mode_rejects_legacy_repacking_and_out_of_bound_groups() {
    for command in [ordered(16), wide_command(0), wide_command(65)] {
        let mut collector = wide_fixture(100);
        assert!(
            collector
                .begin_group(command, vec![tag(); 16], &[0; 64])
                .is_err()
        );
        assert!(collector.poisoned);
        assert!(collector.records.is_empty());
    }
}

#[test]
fn ordered64_preserves_single_publication_and_rejects_tail_identity_drift() {
    let mut collector = wide_fixture(65);
    collector
        .begin_group(single(), vec![tag()], &[0; 4])
        .unwrap();
    collector
        .complete_group(&response(&collector, 1), &[])
        .unwrap();
    collector
        .begin_group(wide_command(64), vec![tag(); 64], &[0; 256])
        .unwrap();
    let ResponseV1::DispatchOrderedBatch64ProfiledCompleted {
        mut timestamps,
        device_unique_id,
        queue_epoch,
        elapsed_ns,
    } = response(&collector, 64)
    else {
        panic!("timestamp response")
    };
    timestamps[63].packet_id += 1;
    assert!(
        collector
            .complete_group(
                &ResponseV1::DispatchOrderedBatch64ProfiledCompleted {
                    timestamps,
                    device_unique_id,
                    queue_epoch,
                    elapsed_ns,
                },
                &[]
            )
            .is_err()
    );
    assert!(collector.finish(65).is_err());
}

#[test]
fn ordered64_finalization_rejects_pending_mismatched_and_poisoned_capture() {
    for failure in 0..4 {
        let mut collector = wide_fixture(64);
        collector
            .begin_group(wide_command(64), vec![tag(); 64], &[0; 256])
            .unwrap();
        if failure != 0 {
            collector
                .complete_group(&response(&collector, 64), &[])
                .unwrap();
        }
        if failure == 3 {
            collector.poison();
        }
        let actual_dispatches = match failure {
            1 => 63,
            2 => 65,
            _ => 64,
        };
        assert!(collector.finish(actual_dispatches).is_err());
    }
}

#[test]
fn single_publication_stays_one_packet_and_is_distinguished_from_ordered_one() {
    let mut collector = fixture(1);
    collector
        .begin_group(single(), vec![tag()], &[1, 2, 3, 4])
        .unwrap();
    collector
        .complete_group(&response(&collector, 1), &[])
        .unwrap();
    assert_eq!(collector.finish(1).unwrap().groups, [[0, 0, 1]]);
}

#[test]
fn local_limit_rejects_empty_and_seventeen_even_though_wire_allows_sixty_four() {
    for count in [0, 17, 64] {
        let mut collector = fixture(100);
        assert!(
            collector
                .begin_group(ordered(count), vec![tag(); count], &vec![0; count * 4])
                .is_err()
        );
        assert!(collector.poisoned);
        assert!(collector.records.is_empty());
    }
}

#[test]
fn already_regrouped_and_sequence_commands_are_not_diagnostic_inputs() {
    for command in [
        CommandV1::DispatchOrderedBatch64 {
            dispatches: vec![entry()],
            timeout_ms: 60_000,
        },
        CommandV1::DispatchOrderedBatch64Profiled {
            dispatches: vec![entry()],
            timeout_ms: 60_000,
        },
        CommandV1::DispatchSequence { dispatches: vec![] },
    ] {
        let mut collector = fixture(1);
        assert!(
            collector
                .begin_group(command, vec![tag()], &[0; 4])
                .is_err()
        );
    }
}

#[test]
fn wrong_payload_tag_count_or_deadline_fail_before_pending_publication() {
    for mutation in 0..4 {
        let mut collector = fixture(1);
        let mut command = ordered(1);
        let tags = if mutation == 0 { vec![] } else { vec![tag()] };
        let payload = if mutation == 1 { vec![] } else { vec![0; 4] };
        if let CommandV1::DispatchOrderedBatch { timeout_ms, .. } = &mut command {
            if mutation == 2 {
                *timeout_ms = 0;
            } else if mutation == 3 {
                *timeout_ms = 600_001;
            }
        }
        assert!(collector.begin_group(command, tags, &payload).is_err());
        assert!(collector.pending.is_none());
        assert_eq!(collector.frontier, 0);
    }
}

#[test]
fn unknown_loaded_kernel_is_rejected() {
    let mut collector = fixture(1);
    let mut dispatch = entry();
    dispatch.kernel = 100;
    let command = CommandV1::DispatchOrderedBatch {
        dispatches: vec![dispatch],
        timeout_ms: 60_000,
    };
    assert!(
        collector
            .begin_group(command, vec![tag()], &[0; 4])
            .is_err()
    );
}

#[test]
fn attribution_rejects_stale_batch_missing_layer_and_global_role_with_layer() {
    for changed in [
        PacketTag {
            batch_ordinal: 2,
            ..tag()
        },
        PacketTag {
            layer: None,
            ..tag()
        },
        PacketTag {
            layer: Some(36),
            ..tag()
        },
        PacketTag {
            operation: Operation::Embedding,
            ..tag()
        },
    ] {
        let mut collector = fixture(1);
        assert!(
            collector
                .begin_group(single(), vec![changed], &[0; 4])
                .is_err()
        );
    }
    let mut collector = fixture(1);
    assert!(
        collector
            .begin_group(
                single(),
                vec![PacketTag {
                    layer: None,
                    operation: Operation::Embedding,
                    ..tag()
                }],
                &[0; 4],
            )
            .is_ok()
    );
}

#[test]
fn all_response_identity_failures_are_atomic_and_terminal() {
    for mutation in 0..9 {
        let mut collector = fixture(2);
        begin(&mut collector, 2);
        let mut reply = response(&collector, 2);
        if let ResponseV1::DispatchOrderedBatch64ProfiledCompleted {
            device_unique_id,
            queue_epoch,
            timestamps,
            ..
        } = &mut reply
        {
            match mutation {
                0 => *device_unique_id += 1,
                1 => *queue_epoch += 1,
                2 => {
                    timestamps.pop();
                }
                3 => timestamps.push(timestamps[0].clone()),
                4 => timestamps[1].packet_id += 1,
                5 => timestamps[1].kernel += 1,
                6 => timestamps[1].start_tick = 0,
                7 => timestamps[1].end_tick = timestamps[1].start_tick,
                8 => timestamps[1].end_tick = timestamps[1].start_tick - 1,
                _ => unreachable!(),
            }
        }
        assert!(collector.complete_group(&reply, &[]).is_err());
        assert_eq!(collector.frontier, 0);
        assert_eq!(collector.batch_packets, 0);
        assert!(collector.records.is_empty());
        assert!(collector.groups.is_empty());
        assert!(
            collector
                .begin_group(single(), vec![tag()], &[0; 4])
                .is_err()
        );
        assert!(collector.finish(0).is_err());
    }
}

#[test]
fn legacy_acknowledgement_and_extra_payload_are_not_accepted() {
    for reply in [
        ResponseV1::Dispatched { elapsed_ns: 1 },
        ResponseV1::DispatchOrderedBatchCompleted {
            completed_dispatches: 1,
            elapsed_ns: 1,
        },
        ResponseV1::DispatchOrderedBatch64Completed {
            completed_dispatches: 1,
            elapsed_ns: 1,
        },
        ResponseV1::Error {
            message: "failed".into(),
            fatal: false,
        },
    ] {
        let mut collector = fixture(1);
        begin(&mut collector, 1);
        assert!(collector.complete_group(&reply, &[]).is_err());
        assert!(collector.poisoned);
    }
    let mut collector = fixture(1);
    begin(&mut collector, 1);
    assert!(
        collector
            .complete_group(&response(&collector, 1), &[0])
            .is_err()
    );
}

#[test]
fn overlap_is_not_mistaken_for_invalid_clock_ordering() {
    let mut collector = fixture(2);
    begin(&mut collector, 2);
    let mut reply = response(&collector, 2);
    if let ResponseV1::DispatchOrderedBatch64ProfiledCompleted { timestamps, .. } = &mut reply {
        timestamps[1].start_tick = 90;
        timestamps[1].end_tick = 140;
    }
    collector.complete_group(&reply, &[]).unwrap();
    let summary = collector.finish(2).unwrap().summarize();
    assert_eq!(summary[0].sum_raw_dispatch_interval_ticks_nonadditive, "70");
    assert_eq!(summary[0].min_raw_dispatch_interval_ticks, 20);
    assert_eq!(summary[0].max_raw_dispatch_interval_ticks, 50);
}

#[test]
fn pending_publication_cannot_be_reused_or_rolled_over() {
    for rollover in [false, true] {
        let mut collector = fixture(2);
        begin(&mut collector, 1);
        if rollover {
            assert!(collector.begin_rollover().is_err());
        } else {
            assert!(
                collector
                    .begin_group(single(), vec![tag()], &[0; 4])
                    .is_err()
            );
        }
        assert!(collector.poisoned);
    }
}

#[test]
fn rollover_preserves_batch_progress_and_resets_only_acknowledged_epoch_frontier() {
    let mut collector = fixture(2);
    begin(&mut collector, 1);
    collector
        .complete_group(&response(&collector, 1), &[])
        .unwrap();
    assert_eq!(
        collector.begin_rollover().unwrap(),
        CommandV1::RolloverQueue {
            expected_epoch: 0,
            expected_completed_packets: 1
        }
    );
    collector
        .complete_rollover(
            &ResponseV1::QueueRolledOver {
                retired_packets: 1,
                queue_epoch: 1,
            },
            &[],
        )
        .unwrap();
    begin(&mut collector, 1);
    collector
        .complete_group(&response(&collector, 1), &[])
        .unwrap();
    let capture = collector.finish(2).unwrap();
    assert_eq!(capture.records[0][5..7], [0, 0]);
    assert_eq!(capture.records[1][5..7], [1, 0]);
    assert_eq!(capture.summarize().len(), 2);
}

#[test]
fn rollover_acknowledgement_must_be_requested_and_exact() {
    for mutation in 0..5 {
        let mut collector = fixture(1);
        begin(&mut collector, 1);
        collector
            .complete_group(&response(&collector, 1), &[])
            .unwrap();
        if mutation != 0 {
            collector.begin_rollover().unwrap();
        }
        let reply = if mutation == 4 {
            ResponseV1::Closed
        } else {
            ResponseV1::QueueRolledOver {
                retired_packets: u64::from(mutation != 1),
                queue_epoch: if mutation == 2 { 2 } else { 1 },
            }
        };
        let payload: &[u8] = if mutation == 3 { &[0] } else { &[] };
        assert!(collector.complete_rollover(&reply, payload).is_err());
        assert_eq!((collector.epoch, collector.frontier), (0, 1));
        assert!(collector.poisoned);
    }
}

#[test]
fn capture_packet_and_ring_limits_fail_closed_before_send() {
    let mut collector = fixture(2);
    collector.records.resize(MAX_CAPTURE_PACKETS, [0; 9]);
    assert!(
        collector
            .begin_group(single(), vec![tag()], &[0; 4])
            .is_err()
    );
    let mut collector = fixture(2);
    collector.frontier = wire::MAX_UNRETIRED_RING_PACKETS_V1 - 1;
    assert!(
        collector
            .begin_group(ordered(2), vec![tag(); 2], &[0; 8])
            .is_err()
    );
}

#[test]
fn batch_envelope_rejects_mixed_empty_oversize_and_stale_ordinals() {
    for (ordinal, prefill_rows, decode_rows, expected_packets) in [
        (2, 0, 1, 1),
        (1, 0, 0, 1),
        (1, 1, 1, 1),
        (1, 17, 0, 1),
        (1, 0, 1, 0),
        (1, 0, 1, MAX_CAPTURE_PACKETS as u64 + 1),
    ] {
        let mut collector = initialized();
        assert!(
            collector
                .begin_batch(BatchIdentity {
                    ordinal,
                    scheduler_batch_id: 900,
                    request: [0, 1],
                    prefill_rows,
                    decode_rows,
                    expected_packets,
                })
                .is_err()
        );
    }
}

#[test]
fn a_single_prefill_row_is_not_misclassified_as_decode() {
    let mut collector = initialized();
    collector
        .begin_batch(BatchIdentity {
            ordinal: 1,
            scheduler_batch_id: 901,
            request: [0, 1],
            prefill_rows: 1,
            decode_rows: 0,
            expected_packets: 1,
        })
        .unwrap();
    begin(&mut collector, 1);
    collector
        .complete_group(&response(&collector, 1), &[])
        .unwrap();
    assert_eq!(collector.finish(1).unwrap().summarize()[0].phase, "prefill");
}

#[test]
fn a_second_request_or_stale_scheduler_batch_cannot_be_hidden_as_warmup() {
    for (request, scheduler_batch_id) in [([1, 1], 901), ([0, 2], 901), ([0, 1], 900)] {
        let mut collector = fixture(1);
        begin(&mut collector, 1);
        collector
            .complete_group(&response(&collector, 1), &[])
            .unwrap();
        assert!(
            collector
                .begin_batch(BatchIdentity {
                    ordinal: 2,
                    scheduler_batch_id,
                    request,
                    prefill_rows: 0,
                    decode_rows: 1,
                    expected_packets: 1,
                })
                .is_err()
        );
    }
}

#[test]
fn same_request_can_advance_from_prefill_to_decode_without_resetting_packet_frontier() {
    let mut collector = initialized();
    collector
        .begin_batch(BatchIdentity {
            ordinal: 1,
            scheduler_batch_id: 900,
            request: [0, 1],
            prefill_rows: 1,
            decode_rows: 0,
            expected_packets: 1,
        })
        .unwrap();
    begin(&mut collector, 1);
    collector
        .complete_group(&response(&collector, 1), &[])
        .unwrap();
    collector
        .begin_batch(BatchIdentity {
            ordinal: 2,
            scheduler_batch_id: 901,
            request: [0, 1],
            prefill_rows: 0,
            decode_rows: 1,
            expected_packets: 1,
        })
        .unwrap();
    collector
        .begin_group(
            single(),
            vec![PacketTag {
                batch_ordinal: 2,
                ..tag()
            }],
            &[0; 4],
        )
        .unwrap();
    collector
        .complete_group(&response(&collector, 1), &[])
        .unwrap();
    let capture = collector.finish(2).unwrap();
    assert_eq!(capture.records[1][6], 1);
    assert_eq!(capture.summarize().len(), 2);
}

#[test]
fn batch_packet_expectation_is_independent_of_timestamps_received() {
    let mut collector = fixture(2);
    begin(&mut collector, 1);
    collector
        .complete_group(&response(&collector, 1), &[])
        .unwrap();
    assert!(collector.finish(1).is_err());
    let mut collector = fixture(1);
    assert!(
        collector
            .begin_group(ordered(2), vec![tag(); 2], &[0; 8])
            .is_err()
    );
    let mut collector = fixture(2);
    assert!(
        collector
            .begin_batch(BatchIdentity {
                ordinal: 2,
                scheduler_batch_id: 901,
                request: [0, 1],
                prefill_rows: 0,
                decode_rows: 1,
                expected_packets: 1,
            })
            .is_err()
    );
}

#[test]
fn independent_driver_count_must_match_and_pending_work_cannot_finish() {
    let mut collector = fixture(1);
    begin(&mut collector, 1);
    assert!(collector.finish(1).is_err());
    let mut collector = fixture(1);
    begin(&mut collector, 1);
    collector
        .complete_group(&response(&collector, 1), &[])
        .unwrap();
    assert!(collector.finish(2).is_err());
}

#[test]
fn explicit_transport_or_cleanup_failure_prevents_capture() {
    let mut collector = fixture(1);
    begin(&mut collector, 1);
    collector
        .complete_group(&response(&collector, 1), &[])
        .unwrap();
    collector.poison();
    assert!(collector.finish(1).is_err());
}

#[test]
fn same_kernel_has_distinct_query_key_and_value_attribution() {
    let mut collector = fixture(3);
    let tags = [
        Operation::QueryProjection,
        Operation::KeyProjection,
        Operation::ValueProjection,
    ]
    .into_iter()
    .map(|operation| PacketTag { operation, ..tag() })
    .collect();
    collector.begin_group(ordered(3), tags, &[0; 12]).unwrap();
    collector
        .complete_group(&response(&collector, 3), &[])
        .unwrap();
    let summaries = collector.finish(3).unwrap().summarize();
    assert_eq!(summaries.len(), 3);
    assert_eq!(summaries[0].operation, "query_projection");
    assert_eq!(summaries[1].operation, "key_projection");
    assert_eq!(summaries[2].operation, "value_projection");
    assert!(summaries.iter().all(|summary| summary.kernel == 99));
}

#[test]
fn source_kernel_identity_is_bounded_immutable_and_never_inferred() {
    for symbol in ["", "../kernel", "kernel\0"] {
        let mut collector = Collector::new(7, 36).unwrap();
        assert!(
            collector
                .register_kernel(KernelIdentity {
                    kernel: 99,
                    symbol: symbol.into(),
                    object_sha256: [42; 32],
                })
                .is_err()
        );
    }
    let mut collector = initialized();
    assert!(
        collector
            .register_kernel(KernelIdentity {
                kernel: 99,
                symbol: "other_symbol".into(),
                object_sha256: [43; 32],
            })
            .is_err()
    );
    let mut collector = fixture(1);
    assert!(
        collector
            .register_kernel(KernelIdentity {
                kernel: 100,
                symbol: "late_kernel".into(),
                object_sha256: [43; 32],
            })
            .is_err()
    );
}

#[test]
fn summary_covers_every_operation_at_the_maximum_batch_ordinal() {
    let operations = [
        Operation::Embedding,
        Operation::InputNorm,
        Operation::QueryProjection,
        Operation::KeyProjection,
        Operation::ValueProjection,
        Operation::QueryNorm,
        Operation::KeyNorm,
        Operation::Rope,
        Operation::KvAppend,
        Operation::Attention,
        Operation::AttentionOutput,
        Operation::AttentionResidual,
        Operation::PostAttentionNorm,
        Operation::GateProjection,
        Operation::UpProjection,
        Operation::SwiGlu,
        Operation::DownProjection,
        Operation::FeedForwardResidual,
        Operation::FinalNorm,
        Operation::HeadProjection,
        Operation::Argmax,
    ];
    assert_eq!(operations.len(), OPERATIONS.len());
    let packets_per_batch = u64::try_from(operations.len()).unwrap();
    let mut collector = initialized();
    for index in 0..MAX_CAPTURE_BATCHES {
        let ordinal = u64::try_from(index).unwrap() + 1;
        let prefill = index.is_multiple_of(2);
        collector
            .begin_batch(BatchIdentity {
                ordinal,
                scheduler_batch_id: ordinal,
                request: [0, 1],
                prefill_rows: u16::from(prefill),
                decode_rows: u16::from(!prefill),
                expected_packets: packets_per_batch,
            })
            .unwrap();
        for group in operations.chunks(MAX_GROUP_PACKETS) {
            let tags = group
                .iter()
                .map(|&operation| PacketTag {
                    batch_ordinal: ordinal,
                    layer: operation.is_layer().then_some(0),
                    operation,
                })
                .collect();
            collector
                .begin_group(ordered(group.len()), tags, &vec![0; 4 * group.len()])
                .unwrap();
            collector
                .complete_group(&response(&collector, group.len()), &[])
                .unwrap();
        }
    }
    let capture = collector
        .finish(u64::try_from(MAX_CAPTURE_BATCHES).unwrap() * packets_per_batch)
        .unwrap();
    assert_eq!(capture.batches.len(), MAX_CAPTURE_BATCHES);
    assert_eq!(
        capture.records.len(),
        MAX_CAPTURE_BATCHES * operations.len()
    );
    assert_eq!(
        capture.records.last().unwrap()[1],
        u64::try_from(MAX_CAPTURE_BATCHES).unwrap()
    );
    let summaries = capture.summarize();
    assert_eq!(summaries.len(), 2 * operations.len());
    for phase in ["prefill", "decode"] {
        for operation in OPERATIONS {
            let matching: Vec<_> = summaries
                .iter()
                .filter(|summary| summary.phase == phase && summary.operation == operation)
                .collect();
            assert_eq!(matching.len(), 1);
            assert_eq!(matching[0].count, MAX_CAPTURE_BATCHES / 2);
            assert_eq!(matching[0].kernel, 99);
            assert_eq!(matching[0].min_raw_dispatch_interval_ticks, 20);
            assert_eq!(matching[0].max_raw_dispatch_interval_ticks, 20);
        }
    }
}

#[test]
fn typed_latest_header_round_trip_preserves_payload_and_u64_fixups() {
    let mut collector = fixture(16);
    let command = begin(&mut collector, 16);
    let mut bytes = Vec::new();
    wire::write_header_v1(&mut bytes, &command).unwrap();
    bytes.extend_from_slice(&[0xa5; 64]);
    let mut cursor = Cursor::new(bytes);
    assert_eq!(
        wire::read_header_v1::<CommandV1>(&mut cursor).unwrap(),
        Some(command)
    );
    let mut payload = [0; 64];
    cursor.read_exact(&mut payload).unwrap();
    assert_eq!(payload, [0xa5; 64]);
    assert_eq!(
        wire::read_header_v1::<CommandV1>(&mut cursor).unwrap(),
        None
    );
    let reply = response(&collector, 16);
    let mut bytes = Vec::new();
    wire::write_header_v1(&mut bytes, &reply).unwrap();
    assert_eq!(
        wire::read_header_v1::<ResponseV1>(&mut Cursor::new(bytes)).unwrap(),
        Some(reply)
    );
}

#[test]
fn serialized_capture_claims_only_raw_transport_intervals_and_has_compact_rows() {
    let mut collector = fixture(1);
    begin(&mut collector, 1);
    collector
        .complete_group(&response(&collector, 1), &[])
        .unwrap();
    let capture = collector.finish(1).unwrap();
    let document: serde_json::Value =
        serde_json::from_slice(&capture.to_bounded_json().unwrap()).unwrap();
    assert_eq!(document["schema"], "FerricModelRawDispatchIntervalsV1");
    assert_eq!(document["core_revision"], CORE_REVISION);
    assert_eq!(
        document["tick_unit"],
        "raw_device_ticks_frequency_unspecified"
    );
    assert_eq!(
        document["records"][0],
        serde_json::json!([0, 1, 4, 2, 99, 0, 0, 100, 120])
    );
    assert!(document.get("elapsed_ns").is_none());
    assert!(document.get("performance_qualified").is_none());
}

#[test]
fn output_byte_cap_fails_without_partial_serialized_capture() {
    let mut writer = BoundedBytes(Vec::new());
    writer.write_all(&vec![0; MAX_CAPTURE_BYTES]).unwrap();
    assert!(writer.write_all(&[1]).is_err());
    assert_eq!(writer.0.len(), MAX_CAPTURE_BYTES);
}
