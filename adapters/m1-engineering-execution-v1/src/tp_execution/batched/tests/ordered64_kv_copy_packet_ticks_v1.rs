//! Recording contracts only; these do not emulate kernels or qualify GPU timing.

use super::*;
use crate::tp_artifact::{
    C1KvCopyBindingV19, Fp32ArgmaxBindingV11, GemvPrefetchBindingV20, PrefillKvCopyBindingV27,
    QueryHoistBindingV14, SplitAttentionBindingV21, WaveRmsNormBindingV15,
};

pub(super) fn configured(
    pool: &EngineeringTpPagedPoolV1,
) -> EngineeringTpBatchExecutionV2<Recording> {
    let mut driver = super::prefill_kv_copy_v28::composition_configured(pool);
    driver.admitted_c1_kv_copy_v19 = Some(C1KvCopyBindingV19::recording());
    driver.admitted_partial_gemv_v20 = Some(GemvPrefetchBindingV20::recording());
    driver.inner.transports[0].ordered64_supported = true;
    driver.inner.transports[0].rollover_supported = true;
    driver.inner.transports[0].timestamp_tags = Some(Vec::new());
    driver.inner.transports[0].timestamp_ordered64 = true;
    driver
}

pub(super) fn select(
    driver: &mut EngineeringTpBatchExecutionV2<Recording>,
    enabled: bool,
) -> TpResult<()> {
    driver.configure_ordered64_kv_copy_packet_tick_bindings_v1(
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
fn v19_packet_ticks_admit_only_marked_actual_nonpeer_image_before_allocation() {
    let binding = C1KvCopyBindingV19::recording();
    for mutation in 0..6 {
        let mut driver = configured(&wide_pool());
        let transport = &mut driver.inner.transports[0];
        transport.buffers.clear();
        transport.c1_kv_copy_v19_loaded = Some(binding.hsaco);
        let next = transport.next;
        match mutation {
            0 => transport.c1_kv_copy_v19_loaded = None,
            1 => transport.c1_kv_copy_v19_loaded.as_mut().unwrap()[0] ^= 1,
            2 => transport.timestamp_tags = None,
            3 => transport.timestamp_ordered64 = false,
            4 => transport.ordered64_supported = false,
            5 => transport.argmax_peer = Some((1, 0, 1)),
            _ => unreachable!(),
        }
        assert!(
            super::super::ordered64_kv_copy_packet_ticks_v1::admit(
                std::slice::from_mut(transport),
                binding,
            )
            .is_err()
        );
        assert!(transport.buffers.is_empty() && transport.commands.is_empty());
        assert_eq!(transport.next, next);
    }
    let mut driver = configured(&wide_pool());
    let transport = &mut driver.inner.transports[0];
    transport.buffers.clear();
    transport.c1_kv_copy_v19_loaded = Some(binding.hsaco);
    super::super::ordered64_kv_copy_packet_ticks_v1::admit(
        std::slice::from_mut(transport),
        binding,
    )
    .unwrap();
    assert!(
        super::super::ordered64_kv_copy_v1::admit(std::slice::from_mut(transport), binding)
            .is_err()
    );
    assert!(transport.buffers.is_empty() && transport.commands.is_empty());
}

#[test]
fn v19_packet_ticks_selection_is_transactional_and_legacy_feature_gate_stays_closed() {
    for mutation in 0..12 {
        let mut driver = configured(&wide_pool());
        match mutation {
            0 => driver.admitted_c1_kv_copy_v19 = None,
            1 => driver.admitted_c1_kv_copy_v19.as_mut().unwrap().hsaco[0] ^= 1,
            2 => driver.admitted_prefill_kv_copy_v27 = None,
            3 => driver.admitted_partial_gemv_v20 = None,
            4 => driver.inner.transports[0].timestamp_tags = None,
            5 => driver.inner.transports[0].timestamp_ordered64 = false,
            6 => driver.inner.transports[0].ordered64_supported = false,
            7 => driver.inner.transports[0].argmax_peer = Some((1, 0, 1)),
            8 => driver.prefill32_pages_v1 = Some(false),
            9 => driver.prefill32_pages_v1 = Some(true),
            10 => driver.poisoned = true,
            11 => driver.inner.ranks[0].layers[0].k_cache.elements -= 1,
            _ => unreachable!(),
        }
        let before = (
            driver.dispatch_counts(),
            driver.inner.ordered_batch_width,
            driver.inner.transports[0]
                .buffers
                .keys()
                .copied()
                .collect::<Vec<_>>(),
        );
        assert!(select(&mut driver, true).is_err(), "mutation {mutation}");
        assert_eq!(
            (
                driver.dispatch_counts(),
                driver.inner.ordered_batch_width,
                driver.inner.transports[0]
                    .buffers
                    .keys()
                    .copied()
                    .collect::<Vec<_>>()
            ),
            before
        );
        assert!(
            driver.c1_kv_copy_v19.is_none()
                && driver.prefill_kv_copy_v28.is_none()
                && driver.c1_split_attention_v25.is_none()
                && driver.partial_gemv_v28.is_none()
        );
        assert!(driver.inner.transports[0].commands.is_empty());
    }
    let mut driver = configured(&wide_pool());
    assert!(select(&mut driver, false).is_err());
    select(&mut driver, true).unwrap();
    assert_eq!(driver.kv_append_mode(), "parallel-c1-v19");
    assert_eq!(driver.prefill_kv_mode(), "parallel-prefill16-v27");
    assert_eq!(driver.c1_packet_mode(), "packed64-v29");
    assert_eq!(driver.partial_gemv_mode(), "baseline");
    assert_eq!(driver.split_attention_workspace_bytes(), 133_120);
    assert!(select(&mut driver, true).is_err());
    assert!(driver.configure_c1_ordered64().is_err());
}

#[test]
fn v19_packet_ticks_complete_request_preserves_every_command_and_publication() {
    use crate::model_timestamps::Operation;
    use crate::tp_scheduler::{TpBatchRowKindV1, TpBatchRowV1, TpRequestIdV1};
    const COPY: &str = crate::tp_artifact::ENGINEERING_TP_C1_KV_COPY_EXPORTS_V19[0];
    const PREFILL: &str = crate::tp_artifact::ENGINEERING_TP_PREFILL_KV_COPY_EXPORTS_V27[0];
    let mut reference = None;
    for diagnostic in [false, true] {
        let limits = EngineeringTpPagedLimitsV1::new(256, 32, 16, 100).unwrap();
        let mut pool = EngineeringTpPagedPoolV1::new_wide32(wide_pool().scope(), limits).unwrap();
        let mut driver = configured(&pool);
        select(&mut driver, true).unwrap();
        if !diagnostic {
            // Reference commands use the same selected route, with recording tags disabled.
            driver.inner.transports[0].timestamp_tags = None;
            driver.inner.transports[0].timestamp_ordered64 = false;
        }
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
                    } else if selected.contains(&index) {
                        TpBatchRowKindV1::PrefillFinal
                    } else {
                        TpBatchRowKindV1::PrefillIntermediate
                    },
                })
                .collect::<Vec<_>>();
            driver
                .bind_numerical_rows(driver.completed_batches() + 1, &scheduler_rows)
                .unwrap();
            pool.begin_submission(&batch).unwrap();
            let output = driver.execute_selected(&batch, &selected).unwrap();
            choices.extend_from_slice(&output.choices);
            let transport = &driver.inner.transports[0];
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
        // The recording transport expands ordered waits into per-packet submits.
        let submitted_packets = events
            .iter()
            .filter(|event| matches!(event, Event::Submit(0, _)))
            .count();
        let ordered_packets = events
            .iter()
            .filter_map(|event| match event {
                Event::OrderedSubmit(0, count) => Some(*count),
                _ => None,
            })
            .sum::<usize>();
        assert_eq!(submitted_packets, 87_711);
        assert_eq!(ordered_packets, 87_700);
        assert_eq!(submitted_packets.checked_sub(ordered_packets), Some(11));
        let transport = &driver.inner.transports[0];
        assert_eq!(
            transport
                .commands
                .iter()
                .filter(|command| command.kernel == COPY)
                .count(),
            4572
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
        if diagnostic {
            let tags = transport.timestamp_tags.as_ref().unwrap();
            assert_eq!(tags.len(), 87_711);
            assert_eq!(
                tags.iter()
                    .filter(|tag| tag.operation == Operation::KvAppend)
                    .count(),
                4860
            );
            assert_eq!(transport.timestamp_batches.len(), 135);
            assert_eq!(
                transport
                    .timestamp_batches
                    .iter()
                    .map(|batch| batch.expected_packets)
                    .sum::<u64>(),
                87_711
            );
        }
        let observed = (
            transport.commands.clone(),
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
