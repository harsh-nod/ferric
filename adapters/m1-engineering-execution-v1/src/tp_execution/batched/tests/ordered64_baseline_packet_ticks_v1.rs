//! Recording contracts, not GPU emulation or numerical/timing qualification.

use super::*;
use crate::tp_artifact::{
    C1KvCopyBindingV19, Fp32ArgmaxBindingV11, GemvPrefetchBindingV20, PrefillKvCopyBindingV27,
    QueryHoistBindingV14, SplitAttentionBindingV21, WaveRmsNormBindingV15,
};
use crate::tp_scheduler::{TpBatchRowKindV1, TpBatchRowV1, TpRequestIdV1};

fn configured(
    pool: &EngineeringTpPagedPoolV1,
    timestamps: bool,
) -> EngineeringTpBatchExecutionV2<Recording> {
    let mut driver = super::prefill_kv_copy_v28::composition_configured(pool);
    driver.admitted_partial_gemv_v20 = Some(GemvPrefetchBindingV20::recording());
    driver.inner.transports[0].ordered64_supported = true;
    driver.inner.transports[0].rollover_supported = true;
    driver.inner.transports[0].timestamp_tags = timestamps.then(Vec::new);
    driver.inner.transports[0].timestamp_ordered64 = timestamps;
    driver
        .configure_partial_gemv_bindings_v28(
            Fp32ArgmaxBindingV11::recording(),
            QueryHoistBindingV14::recording(),
            WaveRmsNormBindingV15::recording(),
            PrefillKvCopyBindingV27::recording(),
            SplitAttentionBindingV21::recording(),
            GemvPrefetchBindingV20::recording(),
            true,
            true,
            true,
            EngineeringTpPartialGemvModeV28::Baseline,
        )
        .unwrap();
    driver.configure_c1_ordered64().unwrap();
    driver
}

fn scheduler_row() -> TpBatchRowV1 {
    TpBatchRowV1 {
        request: TpRequestIdV1 {
            slot: 0,
            generation: 1,
        },
        token_id: 1,
        absolute_position: 128,
        kind: TpBatchRowKindV1::Decode,
    }
}

#[test]
fn baseline_packet_marker_requires_fresh_exact_composition_and_transport() {
    for mutation in 0..17 {
        let mut driver = configured(&wide_pool(), true);
        let allocations = driver.inner.transports[0]
            .buffers
            .keys()
            .copied()
            .collect::<Vec<_>>();
        match mutation {
            0 => driver.inner.transports[0].timestamp_tags = None,
            1 => driver.inner.transports[0].timestamp_ordered64 = false,
            2 => driver.inner.transports[0].ordered64_supported = false,
            3 => driver.inner.transports[0].argmax_peer = Some((1, 0, 1)),
            4 => driver.admitted_c1_kv_copy_v19 = Some(C1KvCopyBindingV19::recording()),
            5 => driver.c1_kv_copy_v19 = Some(C1KvCopyBindingV19::recording()),
            6 => driver.prefill_kv_copy_v28 = Some(false),
            7 => driver.c1_split_attention_v25 = Some(false),
            8 => driver.partial_gemv_v28 = Some(true),
            9 => driver.prefill32_pages_v1 = Some(false),
            10 => driver.prefill16_ordered_v1 = Some(false),
            11 => driver.completed_batches = 1,
            12 => driver.poisoned = true,
            13 => driver.admitted_prefill_kv_copy_v27 = None,
            14 => driver.admitted_partial_gemv_v20 = None,
            15 => driver.query_hoist_v14 = None,
            16 => driver.inner.closed = true,
            _ => unreachable!(),
        }
        assert!(
            driver
                .configure_ordered64_baseline_packet_ticks_v1()
                .is_err(),
            "mutation {mutation}"
        );
        assert!(!driver.baseline_packet_ticks_v1);
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
    let mut driver = configured(&wide_pool(), true);
    driver
        .configure_ordered64_baseline_packet_ticks_v1()
        .unwrap();
    assert!(
        driver
            .configure_ordered64_baseline_packet_ticks_v1()
            .is_err()
    );
}

#[test]
fn baseline_scheduler_binding_requires_marker_and_rejects_v19_drift() {
    let mut unmarked = configured(&wide_pool(), true);
    assert!(unmarked.bind_numerical_rows(1, &[scheduler_row()]).is_err());
    assert!(unmarked.inner.transports[0].timestamp_batches.is_empty());
    for mutation in 0..3 {
        let mut driver = configured(&wide_pool(), true);
        driver
            .configure_ordered64_baseline_packet_ticks_v1()
            .unwrap();
        match mutation {
            0 => driver.admitted_c1_kv_copy_v19 = Some(C1KvCopyBindingV19::recording()),
            1 => driver.c1_kv_copy_v19 = Some(C1KvCopyBindingV19::recording()),
            2 => driver.inner.transports[0].timestamp_ordered64 = false,
            _ => unreachable!(),
        }
        assert!(driver.bind_numerical_rows(1, &[scheduler_row()]).is_err());
        assert!(driver.inner.transports[0].timestamp_batches.is_empty());
    }
}

#[test]
fn baseline_packet_complete_128_128_preserves_every_command_and_publication() {
    use crate::model_timestamps::Operation;
    const V19: &str = crate::tp_artifact::ENGINEERING_TP_C1_KV_COPY_EXPORTS_V19[0];
    const PREFILL: &str = crate::tp_artifact::ENGINEERING_TP_PREFILL_KV_COPY_EXPORTS_V27[0];
    let mut reference = None;
    for diagnostic in [false, true] {
        let limits = EngineeringTpPagedLimitsV1::new(256, 32, 16, 100).unwrap();
        let mut pool = EngineeringTpPagedPoolV1::new_wide32(wide_pool().scope(), limits).unwrap();
        let mut driver = configured(&pool, diagnostic);
        let allocations = driver.inner.transports[0]
            .buffers
            .keys()
            .copied()
            .collect::<Vec<_>>();
        if diagnostic {
            driver
                .configure_ordered64_baseline_packet_ticks_v1()
                .unwrap();
        }
        assert!(driver.admitted_c1_kv_copy_v19.is_none() && driver.c1_kv_copy_v19.is_none());
        assert_eq!(driver.kv_append_mode(), "baseline");
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
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, Event::Submit(0, _)))
                .count(),
            87_711
        );
        let transport = &driver.inner.transports[0];
        assert_eq!(
            transport
                .commands
                .iter()
                .filter(|command| command.kernel == V19)
                .count(),
            0
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
        assert!(transport.c1_kv_copy_v19_loaded.is_none());
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
