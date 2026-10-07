//! Recording transport contracts, not native kernel or model numerical evidence.

use super::super::prefill_kv_copy_v28::{PrefillPageV28, admit};
use super::*;
use crate::tp_artifact::{
    Fp32ArgmaxBindingV11, PrefillKvCopyBindingV27, QueryHoistBindingV14, WaveRmsNormBindingV15,
};

const COPY: &str = crate::tp_artifact::ENGINEERING_TP_PREFILL_KV_COPY_EXPORTS_V27[0];
const LEGACY: &str = "ferric_qwen3_tp_batch32_paged_kv_append_v5";

#[test]
#[cfg(feature = "model-timestamps")]
fn model_timestamps_preserve_v28_commands_groups_and_bind_each_semantic_role() {
    use crate::model_timestamps::Operation;
    use crate::tp_scheduler::{TpBatchRowKindV1, TpBatchRowV1, TpRequestIdV1};

    for (rows, publish) in [(16, false), (16, true), (1, true)] {
        for copy in [false, true] {
            let mut reference = None;
            for diagnostic in [false, true] {
                let mut pool = wide_pool();
                let mut driver = configured(&pool);
                select(&mut driver, copy).unwrap();
                if diagnostic {
                    driver.inner.transports[0].timestamp_tags = Some(Vec::new());
                }
                let batch = prepare(&mut pool, rows);
                let selected = if publish {
                    vec![usize::try_from(rows - 1).unwrap()]
                } else {
                    vec![]
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
                driver.bind_numerical_rows(1, &scheduler_rows).unwrap();
                pool.begin_submission(&batch).unwrap();
                let output = driver.execute_selected(&batch, &selected).unwrap();
                let transport = &driver.inner.transports[0];
                let observed = (
                    transport.commands.clone(),
                    transport.events.borrow().clone(),
                    transport.packet_preparations.clone(),
                    output.choices.clone(),
                );
                if let Some(expected) = &reference {
                    assert_eq!(&observed, expected);
                } else {
                    reference = Some(observed);
                }
                if diagnostic {
                    let mut roles = vec![(None, Operation::Embedding)];
                    for layer in 0..36 {
                        roles.extend(
                            [
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
                            ]
                            .map(|operation| (Some(layer), operation)),
                        );
                    }
                    if publish {
                        roles.extend(
                            [
                                Operation::FinalNorm,
                                Operation::HeadProjection,
                                Operation::Argmax,
                            ]
                            .map(|operation| (None, operation)),
                        );
                    }
                    let tags = transport.timestamp_tags.as_ref().unwrap();
                    assert_eq!(tags.len(), transport.commands.len());
                    assert!(tags.iter().all(|tag| tag.batch_ordinal == 1));
                    assert_eq!(
                        tags.iter()
                            .map(|tag| (tag.layer, tag.operation))
                            .collect::<Vec<_>>(),
                        roles
                    );
                    assert_eq!(transport.timestamp_batches.len(), 1);
                    let bound = transport.timestamp_batches[0];
                    assert_eq!(bound.expected_packets, if publish { 616 } else { 613 });
                    assert_eq!(bound.request, [0, 1]);
                    assert_eq!(bound.decode_rows, u16::from(rows == 1));
                    assert_eq!(bound.prefill_rows, if rows == 1 { 0 } else { 16 });
                }
                pool.commit_batch(&batch, output.completion).unwrap();
                driver.close().unwrap();
            }
        }
    }
}

fn configured(pool: &EngineeringTpPagedPoolV1) -> EngineeringTpBatchExecutionV2<Recording> {
    let mut driver = super::wave_rmsnorm_v15::configured(pool);
    // The shared recording fixture has fixed 64-token storage; extend only this
    // candidate's fake storage when checking a complete 128-token prompt.
    let limits = pool.limits();
    driver.context_tokens = limits.context_tokens();
    driver.physical_pages = limits.physical_page_count();
    driver.table_stride = limits.page_table_stride();
    driver.inner.capacity = limits.context_tokens();
    driver.inner.sequence =
        TensorParallelSequenceV1::new(limits.context_tokens(), target().vocabulary_size).unwrap();
    let rank = &mut driver.inner.ranks[0];
    let transport = &mut driver.inner.transports[0];
    for layer in &mut rank.layers {
        for tensor in [&mut layer.k_cache, &mut layer.v_cache] {
            tensor.elements = limits.physical_token_capacity().unwrap() as usize * 1024;
            transport
                .buffers
                .get_mut(&tensor.id)
                .unwrap()
                .resize(tensor.elements * 2, 0);
        }
    }
    let table = &mut driver.page_tables[0];
    table.elements = driver.row_capacity * limits.page_table_stride() as usize;
    transport
        .buffers
        .get_mut(&table.id)
        .unwrap()
        .resize(table.elements * 4, 0);
    driver.admitted_query_hoist_v14 = Some(QueryHoistBindingV14::recording());
    driver.admitted_prefill_kv_copy_v27 = Some(PrefillKvCopyBindingV27::recording());
    driver
}

fn select(driver: &mut EngineeringTpBatchExecutionV2<Recording>, enabled: bool) -> TpResult<()> {
    driver.configure_prefill_copy_bindings_v28(
        Fp32ArgmaxBindingV11::recording(),
        QueryHoistBindingV14::recording(),
        WaveRmsNormBindingV15::recording(),
        PrefillKvCopyBindingV27::recording(),
        enabled,
    )
}

pub(super) fn composition_configured(
    pool: &EngineeringTpPagedPoolV1,
) -> EngineeringTpBatchExecutionV2<Recording> {
    let mut driver = configured(pool);
    driver
        .allocate_split_workspace_v25(crate::tp_artifact::SplitAttentionBindingV21::recording())
        .unwrap();
    driver
}

pub(super) fn select_composition(
    driver: &mut EngineeringTpBatchExecutionV2<Recording>,
    copy: bool,
    split: bool,
    packed: bool,
) -> TpResult<()> {
    driver.configure_prefill_decode_bindings_v28(
        Fp32ArgmaxBindingV11::recording(),
        QueryHoistBindingV14::recording(),
        WaveRmsNormBindingV15::recording(),
        PrefillKvCopyBindingV27::recording(),
        crate::tp_artifact::SplitAttentionBindingV21::recording(),
        copy,
        split,
        packed,
    )
}

#[test]
fn prefill_decode_composition_v28_all_eight_modes_preserve_full_request_order_and_counts() {
    use crate::tp_artifact::ENGINEERING_TP_SPLIT_ATTENTION_EXPORTS_V21 as SPLIT;

    for split in [false, true] {
        let mut reference = None;
        for copy in [false, true] {
            for packed in [false, true] {
                let limits = EngineeringTpPagedLimitsV1::new(256, 32, 16, 100).unwrap();
                let mut pool =
                    EngineeringTpPagedPoolV1::new_wide32(wide_pool().scope(), limits).unwrap();
                let mut driver = composition_configured(&pool);
                select_composition(&mut driver, copy, split, packed).unwrap();
                assert_eq!(driver.split_attention_workspace_bytes(), 133_120);
                let scratch = driver.split_attention_workspace_v25.unwrap();
                let allocations = driver.inner.transports[0]
                    .buffers
                    .keys()
                    .copied()
                    .collect::<Vec<_>>();
                driver.inner.transports[0].rollover_supported = true;
                let sequence = pool
                    .open_sequence(pool.scope(), &(0..256).collect::<Vec<_>>(), 0)
                    .unwrap()
                    .sequence();
                let mut groups = 0;
                let mut grouped_packets = 0;
                let mut choices = Vec::new();
                let mut crossed_boundary = false;
                for (ordinal, (first, rows)) in (0..128)
                    .step_by(16)
                    .map(|first| (first, 16))
                    .chain((128..255).map(|first| (first, 1)))
                    .enumerate()
                {
                    let batch_rows = (first..first + rows)
                        .map(|position| EngineeringTpPageRowV1 {
                            sequence,
                            token: position,
                            position,
                        })
                        .collect::<Vec<_>>();
                    let batch = pool.reserve_batch(&batch_rows).unwrap();
                    let selected = if first < 112 {
                        vec![]
                    } else {
                        vec![usize::try_from(rows - 1).unwrap()]
                    };
                    let expected = if rows == 1 && split {
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
                    for (index, partial) in commands
                        .iter()
                        .enumerate()
                        .filter(|(_, command)| command.kernel == SPLIT[0])
                    {
                        let merge = &commands[index + 1];
                        assert_eq!(merge.kernel, SPLIT[1]);
                        assert_eq!(
                            (buffer(partial, 5).0, buffer(merge, 0).0),
                            (scratch.stats.id, scratch.stats.id)
                        );
                        assert_eq!(
                            (buffer(partial, 6).0, buffer(merge, 1).0),
                            (scratch.numerators.id, scratch.numerators.id)
                        );
                        crossed_boundary |= packed && index % 16 == 15;
                    }
                    let actual_groups = transport
                        .events
                        .borrow()
                        .iter()
                        .filter_map(|event| match event {
                            Event::OrderedSubmit(0, count) => Some(*count),
                            _ => None,
                        })
                        .collect::<Vec<_>>();
                    let expected_groups = if packed && rows == 1 {
                        let mut counts = vec![16; usize::try_from(expected).unwrap() / 16];
                        counts.push(usize::try_from(expected).unwrap() % 16);
                        counts
                    } else {
                        [if split && rows == 1 { 12 } else { 11 }, 6].repeat(36)
                    };
                    assert_eq!(actual_groups, expected_groups);
                    groups += actual_groups.len();
                    grouped_packets += actual_groups.iter().sum::<usize>();
                    // Bound recording-only ordinal scans independently of request length.
                    transport.events.borrow_mut().clear();
                    assert!(driver.inner.packed_c1.is_none());
                    assert!(driver.inner.ordered_batches.as_ref().unwrap().is_empty());
                    assert_eq!(driver.inner.collective.expected().epoch, ordinal as u64 + 1);
                    pool.commit_batch(&batch, output.completion).unwrap();
                }
                let expected_packets = if split { 87_711 } else { 83_139 };
                assert_eq!(driver.dispatch_counts(), [expected_packets]);
                assert_eq!(driver.completed_batches, 135);
                assert_eq!(choices.len(), 128);
                assert_eq!(
                    groups,
                    if packed {
                        if split { 5783 } else { 5529 }
                    } else {
                        9720
                    }
                );
                assert_eq!(
                    usize::try_from(expected_packets).unwrap() - grouped_packets,
                    if packed { 11 } else { 519 }
                );
                assert_eq!(crossed_boundary, split && packed);
                let transport = &driver.inner.transports[0];
                assert_eq!(transport.packet_preparations.len(), 135);
                assert_eq!(
                    transport
                        .commands
                        .iter()
                        .filter(|command| command.kernel == COPY)
                        .count(),
                    if copy { 288 } else { 0 }
                );
                assert_eq!(
                    transport
                        .commands
                        .iter()
                        .filter(|command| command.kernel == LEGACY)
                        .count(),
                    36 * 135 - if copy { 288 } else { 0 }
                );
                assert_eq!(
                    transport
                        .commands
                        .iter()
                        .filter(|command| command.kernel == SPLIT[0])
                        .count(),
                    if split { 36 * 127 } else { 0 }
                );
                assert_eq!(
                    transport.buffers.keys().copied().collect::<Vec<_>>(),
                    allocations
                );
                let observed = (
                    transport
                        .commands
                        .iter()
                        .filter(|command| !matches!(command.kernel, COPY | LEGACY))
                        .cloned()
                        .collect::<Vec<_>>(),
                    choices,
                    transport.reads.clone(),
                    transport.write_payloads.clone(),
                );
                if let Some(reference) = &reference {
                    assert_eq!(&observed, reference);
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
    }
}

#[test]
fn prefill_decode_composition_v28_rejects_incompatible_state_before_any_selector_changes() {
    for mutation in 0..23 {
        let mut driver = composition_configured(&wide_pool());
        match mutation {
            0 => driver.admitted_prefill_kv_copy_v27 = None,
            1 => driver.split_attention_workspace_v25 = None,
            2 => {
                driver
                    .split_attention_workspace_v25
                    .as_mut()
                    .unwrap()
                    .stats
                    .elements -= 1;
            }
            3 => {
                driver
                    .split_attention_workspace_v25
                    .as_mut()
                    .unwrap()
                    .image
                    .hsaco[0] ^= 1;
            }
            4 => driver.inner.ranks[0].layers[0].k_cache.elements -= 1,
            5 => driver.inner.row_capacity = 16,
            6 => driver.row_capacity = 16,
            7 => driver.last_batch = 1,
            8 => driver.completed_batches = 1,
            9 => driver.poisoned = true,
            10 => driver.inner.closed = true,
            11 => driver.inner.draft_v10 = true,
            12 => driver.inner.large_kv = true,
            13 => driver.inner.ordered_batches = Some(Vec::new()),
            14 => driver.inner.transports[0].argmax_peer = Some((1, 0, 1)),
            15 => driver.admitted_query_hoist_v14 = None,
            16 => driver.admitted_argmax_v11 = None,
            17 => driver.admitted_wave_rmsnorm_v15 = None,
            18 => {
                driver.admitted_c1_kv_copy_v19 =
                    Some(crate::tp_artifact::C1KvCopyBindingV19::recording());
            }
            19 => driver.c1_packet_packing_v22 = Some(false),
            20 => driver.c1_split_attention_v25 = Some(false),
            21 => driver.prefill_kv_copy_v28 = Some(false),
            22 => driver.wave_attention = false,
            _ => unreachable!(),
        }
        let before = (
            driver.prefill_kv_copy_v28,
            driver.c1_split_attention_v25,
            driver.c1_packet_packing_v22,
        );
        assert!(
            select_composition(&mut driver, true, true, true).is_err(),
            "mutation {mutation}"
        );
        assert_eq!(
            (
                driver.prefill_kv_copy_v28,
                driver.c1_split_attention_v25,
                driver.c1_packet_packing_v22
            ),
            before
        );
        assert!(driver.query_hoist_v14.is_none());
        assert!(driver.wave_rmsnorm_v15.is_none());
        assert!(driver.fp32_argmax_v11.is_none());
        assert!(!driver.c1_wave_layers);
        assert!(driver.inner.transports[0].commands.is_empty());
    }
    for enabled in [false, true] {
        let mut driver = composition_configured(&wide_pool());
        assert!(select(&mut driver, enabled).is_err());
        select_composition(&mut driver, enabled, enabled, enabled).unwrap();
        assert!(select_composition(&mut driver, !enabled, !enabled, !enabled).is_err());
        assert!(driver.configure_c1_packet_packing_v22(enabled).is_err());
        assert!(
            driver
                .configure_c1_split_packet_packing_v25(enabled)
                .is_err()
        );
    }
}

#[test]
fn prefill_v28_replaces_only_eligible_append_packets_and_preserves_every_other_command() {
    for (rows, selected, eligible) in [
        (1, vec![0], false),
        (15, vec![14], false),
        (16, vec![], true),
        (16, vec![15], true),
        (16, vec![0], true),
        (16, vec![7], false),
        (16, vec![0, 15], false),
        (17, vec![16], false),
        (32, vec![31], false),
    ] {
        let mut reference = None;
        for enabled in [false, true] {
            let mut pool = wide_pool();
            let mut driver = configured(&pool);
            select(&mut driver, enabled).unwrap();
            assert_eq!(
                driver.prefill_kv_mode(),
                if enabled {
                    "parallel-prefill16-v27"
                } else {
                    "baseline"
                }
            );
            assert_eq!(driver.c1_packet_mode(), "baseline");
            assert_eq!(driver.kv_append_mode(), "baseline");
            let allocations = driver.inner.transports[0]
                .buffers
                .keys()
                .copied()
                .collect::<Vec<_>>();
            let batch = prepare(&mut pool, rows);
            pool.begin_submission(&batch).unwrap();
            let output = driver.execute_selected(&batch, &selected).unwrap();
            let transport = &driver.inner.transports[0];
            let copies = transport
                .commands
                .iter()
                .filter(|command| command.kernel == COPY)
                .count();
            assert_eq!(copies, if enabled && eligible { 36 } else { 0 });
            assert_eq!(
                transport
                    .commands
                    .iter()
                    .filter(|command| command.kernel == LEGACY)
                    .count(),
                36 - copies
            );
            assert_eq!(
                driver.dispatch_counts(),
                [if selected.is_empty() { 613 } else { 616 }]
            );
            assert_eq!(
                transport.buffers.keys().copied().collect::<Vec<_>>(),
                allocations
            );
            let observed = (
                transport
                    .commands
                    .iter()
                    .filter(|command| !matches!(command.kernel, COPY | LEGACY))
                    .cloned()
                    .collect::<Vec<_>>(),
                output.choices.clone(),
                transport.reads.clone(),
                transport.write_payloads.clone(),
                transport.packet_preparations.clone(),
            );
            if let Some(expected) = &reference {
                assert_eq!(&observed, expected);
            } else {
                reference = Some(observed);
            }
            pool.commit_batch(&batch, output.completion).unwrap();
            pool.check_invariants().unwrap();
            driver.close().unwrap();
        }
    }
}

#[test]
fn prefill_v28_page_views_copy_both_orders_bit_exactly_without_touching_other_pages() {
    let mut pool = wide_pool();
    let mut driver = configured(&pool);
    let batch = prepare(&mut pool, 16);
    for order in [
        (0..16).collect::<Vec<_>>(),
        std::iter::once(15).chain(0..15).collect(),
    ] {
        let page = PrefillPageV28::select(&batch, &order, 4, 64)
            .unwrap()
            .unwrap();
        let command = super::super::super::row_profile::bind_storage(
            32,
            false,
            page.command(&driver.inner.ranks[0], 0),
        )
        .unwrap();
        let transport = &mut driver.inner.transports[0];
        for chunk in 0..4_u32 {
            for (source, destination) in [(0, 2), (1, 3)] {
                let input = buffer(&command, source);
                let output = buffer(&command, destination);
                for (source_row, &logical_row) in order.iter().enumerate() {
                    let bytes = (0..1024_u32)
                        .flat_map(|column| {
                            let value = u16::try_from(
                                chunk * 16_384
                                    + u32::try_from(logical_row).unwrap() * 1024
                                    + column,
                            )
                            .unwrap();
                            (value ^ if source == 0 { 0 } else { 0xa55a }).to_le_bytes()
                        })
                        .collect::<Vec<_>>();
                    transport.buffers.get_mut(&input.0).unwrap()
                        [source_row * 2048..(source_row + 1) * 2048]
                        .copy_from_slice(&bytes);
                }
                transport.buffers.get_mut(&output.0).unwrap().fill(0xa5);
            }
            transport.submit(&command).unwrap();
            transport.wait().unwrap();
            for (source, destination) in [(0, 2), (1, 3)] {
                let output = buffer(&command, destination);
                let actual = &transport.buffers[&output.0];
                let expected = (0..16_384_u32)
                    .flat_map(|index| {
                        let value = u16::try_from(chunk * 16_384 + index).unwrap();
                        (value ^ if source == 0 { 0 } else { 0xa55a }).to_le_bytes()
                    })
                    .collect::<Vec<_>>();
                assert_eq!(&actual[output.1..output.1 + 32_768], expected);
                assert!(
                    actual[..output.1]
                        .iter()
                        .chain(&actual[output.1 + 32_768..])
                        .all(|byte| *byte == 0xa5)
                );
            }
        }
    }
    for order in [vec![], vec![0; 16], (1..17).collect()] {
        assert!(PrefillPageV28::select(&batch, &order, 4, 64).is_err());
    }
    for (pages, context) in [(0, 64), (513, 64), (4, 0), (4, 8193), (4, 15)] {
        assert!(
            PrefillPageV28::select(&batch, &(0..16).collect::<Vec<_>>(), pages, context).is_err()
        );
    }
}

#[test]
fn prefill_v28_reuses_cached_prefix_without_mutating_it() {
    let mut pool = wide_pool();
    let mut driver = configured(&pool);
    select(&mut driver, true).unwrap();
    let first = prepare(&mut pool, 16);
    let sequence = first.rows()[0].sequence();
    let prefix_page = first.rows()[0].writable_physical_page();
    pool.begin_submission(&first).unwrap();
    let result = driver.execute_selected(&first, &[15]).unwrap();
    pool.commit_batch(&first, result.completion).unwrap();
    let caches = driver.inner.ranks[0]
        .layers
        .iter()
        .flat_map(|layer| [layer.k_cache.id, layer.v_cache.id])
        .collect::<Vec<_>>();
    for id in &caches {
        driver.inner.transports[0].buffers.get_mut(id).unwrap()
            [prefix_page as usize * 32_768..(prefix_page as usize + 1) * 32_768]
            .fill(0x5a);
    }
    pool.retire_sequence(sequence, true, 1).unwrap();
    let hit = pool
        .open_sequence(pool.scope(), &(0..32).collect::<Vec<_>>(), 2)
        .unwrap();
    assert_eq!(hit.hit_tokens(), 16);
    assert_eq!(hit.physical_pages(), &[prefix_page]);
    let rows = (16..32)
        .map(|position| EngineeringTpPageRowV1 {
            sequence: hit.sequence(),
            token: position,
            position,
        })
        .collect::<Vec<_>>();
    let batch = pool.reserve_batch(&rows).unwrap();
    let suffix_page = batch.rows()[0].writable_physical_page();
    assert_ne!(suffix_page, prefix_page);
    let expected = caches
        .iter()
        .map(|id| {
            let mut data = driver.inner.transports[0].buffers[id].clone();
            data[suffix_page as usize * 32_768..(suffix_page as usize + 1) * 32_768].fill(0);
            (*id, data)
        })
        .collect::<Vec<_>>();
    pool.begin_submission(&batch).unwrap();
    let result = driver.execute_selected(&batch, &[15]).unwrap();
    for (id, data) in expected {
        assert_eq!(driver.inner.transports[0].buffers[&id], data);
    }
    assert_eq!(
        driver.inner.transports[0]
            .commands
            .iter()
            .filter(|command| command.kernel == COPY)
            .count(),
        72
    );
    pool.commit_batch(&batch, result.completion).unwrap();
    pool.check_invariants().unwrap();
    driver.close().unwrap();
}

#[test]
fn prefill_v28_admission_and_configuration_reject_incompatible_images_and_hybrids() {
    let binding = PrefillKvCopyBindingV27::recording();
    let mut driver = configured(&wide_pool());
    let transport = &mut driver.inner.transports[0];
    assert!(admit(std::slice::from_mut(transport), binding).is_err());
    transport.buffers.clear();
    assert!(admit(std::slice::from_mut(transport), binding).is_err());
    transport.prefill_kv_copy_v27_loaded = Some(binding.hsaco);
    admit(std::slice::from_mut(transport), binding).unwrap();
    transport.argmax_peer = Some((1, 0, 1));
    assert!(admit(std::slice::from_mut(transport), binding).is_err());
    assert!(transport.commands.is_empty());
    for mutation in 0..17 {
        let mut driver = configured(&wide_pool());
        match mutation {
            0 => driver.admitted_prefill_kv_copy_v27 = None,
            1 => {
                let mut binding = PrefillKvCopyBindingV27::recording();
                binding.hsaco[0] ^= 1;
                driver.admitted_prefill_kv_copy_v27 = Some(binding);
            }
            2 => driver.row_capacity = 16,
            3 => driver.inner.large_kv = true,
            4 => driver.physical_pages = 0,
            5 => driver.physical_pages = 513,
            6 => driver.context_tokens = 8193,
            7 => driver.inner.ranks[0].k_rotated.elements -= 1,
            8 => driver.inner.ranks[0].v.element_bytes = 4,
            9 => driver.inner.ranks[0].layers[0].k_cache.elements -= 1,
            10 => driver.inner.ranks[0].layers[35].v_cache.element_bytes = 4,
            11 => driver.last_batch = 1,
            12 => driver.inner.closed = true,
            13 => driver.c1_packet_packing_v22 = Some(false),
            14 => {
                driver.admitted_c1_kv_copy_v19 =
                    Some(crate::tp_artifact::C1KvCopyBindingV19::recording());
            }
            15 => driver.c1_split_attention_v25 = Some(false),
            16 => {
                driver.split_attention_workspace_v25 = Some(
                    super::super::c1_split_attention_v25::Workspace::allocate(
                        &mut driver.inner.transports[0],
                        crate::tp_artifact::SplitAttentionBindingV21::recording(),
                    )
                    .unwrap(),
                );
            }
            _ => unreachable!(),
        }
        assert!(select(&mut driver, true).is_err(), "mutation {mutation}");
        assert!(driver.prefill_kv_copy_v28.is_none());
        assert!(driver.query_hoist_v14.is_none());
        assert!(driver.inner.transports[0].commands.is_empty());
    }
    for enabled in [false, true] {
        let mut driver = configured(&wide_pool());
        select(&mut driver, enabled).unwrap();
        assert!(select(&mut driver, enabled).is_err());
        assert!(driver.configure_c1_packet_packing_v22(false).is_err());
    }
}

#[test]
fn prefill_v28_failure_never_commits_a_batch_and_poison_is_terminal() {
    for failure in [
        Failure::PreparePackets,
        Failure::KvCopySubmit,
        Failure::KvCopyWait,
        Failure::OrderedSubmitAt(1),
        Failure::OrderedWaitAt(1),
    ] {
        let mut pool = wide_pool();
        let mut driver = configured(&pool);
        select(&mut driver, true).unwrap();
        let batch = prepare(&mut pool, 16);
        pool.begin_submission(&batch).unwrap();
        driver.inner.transports[0].failure = Some(failure);
        assert!(driver.execute_selected(&batch, &[15]).is_err());
        assert!(driver.poisoned);
        assert_eq!(driver.completed_batches, 0);
        assert!(driver.execute_selected(&batch, &[15]).is_err());
        pool.quarantine_batch(&batch).unwrap();
        assert!(pool.check_invariants().is_err());
        driver.close().unwrap();
    }
}

#[test]
fn prefill_v28_eight_prompt_chunks_change_288_packets_without_changing_batch_counts() {
    for enabled in [false, true] {
        let limits = EngineeringTpPagedLimitsV1::new(256, 32, 16, 100).unwrap();
        let mut pool = EngineeringTpPagedPoolV1::new_wide32(wide_pool().scope(), limits).unwrap();
        let mut driver = configured(&pool);
        select(&mut driver, enabled).unwrap();
        let sequence = pool
            .open_sequence(pool.scope(), &(0..129).collect::<Vec<_>>(), 0)
            .unwrap()
            .sequence();
        for first in (0..128).step_by(16) {
            let rows = (first..first + 16)
                .map(|position| EngineeringTpPageRowV1 {
                    sequence,
                    token: position,
                    position,
                })
                .collect::<Vec<_>>();
            let batch = pool.reserve_batch(&rows).unwrap();
            pool.begin_submission(&batch).unwrap();
            let result = driver
                .execute_selected(&batch, if first == 112 { &[15] } else { &[] })
                .unwrap();
            pool.commit_batch(&batch, result.completion).unwrap();
        }
        assert_eq!(driver.dispatch_counts(), [4907]);
        assert_eq!(driver.completed_batches, 8);
        assert_eq!(
            driver.inner.transports[0]
                .commands
                .iter()
                .filter(|command| command.kernel == COPY)
                .count(),
            if enabled { 288 } else { 0 }
        );
        let batch = pool
            .reserve_batch(&[EngineeringTpPageRowV1 {
                sequence,
                token: 128,
                position: 128,
            }])
            .unwrap();
        pool.begin_submission(&batch).unwrap();
        let result = driver.execute_selected(&batch, &[0]).unwrap();
        pool.commit_batch(&batch, result.completion).unwrap();
        assert_eq!(driver.dispatch_counts(), [5523]);
        assert_eq!(driver.expected_dispatch_counts(1), [616]);
        assert_eq!(4907 + 127 * driver.expected_dispatch_counts(1)[0], 83_139);
        assert_eq!(
            driver.inner.transports[0]
                .commands
                .iter()
                .filter(|command| command.kernel == COPY)
                .count(),
            if enabled { 288 } else { 0 }
        );
        pool.check_invariants().unwrap();
        driver.close().unwrap();
    }
}

#[test]
fn prefill_v28_unaligned_and_mixed_sequence_batches_fall_back_but_foreign_stale_reject() {
    let mut pool = wide_pool();
    let mut driver = configured(&pool);
    select(&mut driver, true).unwrap();
    let sequence = pool
        .open_sequence(pool.scope(), &(0..32).collect::<Vec<_>>(), 0)
        .unwrap()
        .sequence();
    let first = pool
        .reserve_batch(&[EngineeringTpPageRowV1 {
            sequence,
            token: 0,
            position: 0,
        }])
        .unwrap();
    pool.begin_submission(&first).unwrap();
    let output = driver.execute_selected(&first, &[0]).unwrap();
    pool.commit_batch(&first, output.completion).unwrap();
    assert!(driver.execute_selected(&first, &[0]).is_err());
    let rows = (1..17)
        .map(|position| EngineeringTpPageRowV1 {
            sequence,
            token: position,
            position,
        })
        .collect::<Vec<_>>();
    let batch = pool.reserve_batch(&rows).unwrap();
    assert!(
        PrefillPageV28::select(&batch, &(0..16).collect::<Vec<_>>(), 4, 64)
            .unwrap()
            .is_none()
    );
    pool.begin_submission(&batch).unwrap();
    let output = driver.execute_selected(&batch, &[15]).unwrap();
    pool.commit_batch(&batch, output.completion).unwrap();
    assert!(
        !driver.inner.transports[0]
            .commands
            .iter()
            .any(|command| command.kernel == COPY)
    );
    driver.close().unwrap();

    let mut pool = wide_pool();
    let mut driver = configured(&pool);
    select(&mut driver, true).unwrap();
    let first = pool
        .open_sequence(pool.scope(), &(0..8).collect::<Vec<_>>(), 0)
        .unwrap()
        .sequence();
    let second = pool
        .open_sequence(pool.scope(), &(8..16).collect::<Vec<_>>(), 0)
        .unwrap()
        .sequence();
    let rows = [(first, 0), (second, 8)]
        .into_iter()
        .flat_map(|(sequence, token_base)| {
            (0..8).map(move |position| EngineeringTpPageRowV1 {
                sequence,
                token: token_base + position,
                position,
            })
        })
        .collect::<Vec<_>>();
    let batch = pool.reserve_batch(&rows).unwrap();
    assert!(
        PrefillPageV28::select(&batch, &(0..16).collect::<Vec<_>>(), 4, 64)
            .unwrap()
            .is_none()
    );
    pool.begin_submission(&batch).unwrap();
    let output = driver.execute_selected(&batch, &[7, 15]).unwrap();
    pool.commit_batch(&batch, output.completion).unwrap();
    assert!(
        !driver.inner.transports[0]
            .commands
            .iter()
            .any(|command| command.kernel == COPY)
    );
    let mut foreign = wide_pool();
    let count = driver.dispatch_counts();
    assert!(
        driver
            .execute_selected(&prepare(&mut foreign, 16), &[15])
            .is_err()
    );
    assert_eq!(driver.dispatch_counts(), count);
    driver.close().unwrap();
}

#[test]
fn prefill_v28_admission_and_selection_also_exclude_v25_in_the_reverse_order() {
    use super::super::c1_split_attention_v25::Workspace;
    use crate::tp_artifact::SplitAttentionBindingV21;

    for selected in [None, Some(false), Some(true)] {
        let mut driver = configured(&wide_pool());
        driver.prefill_kv_copy_v28 = selected;
        driver.split_attention_workspace_v25 = Some(
            Workspace::allocate(
                &mut driver.inner.transports[0],
                SplitAttentionBindingV21::recording(),
            )
            .unwrap(),
        );
        let configure = |driver: &mut EngineeringTpBatchExecutionV2<Recording>| {
            driver.configure_split_bindings_v25(
                Fp32ArgmaxBindingV11::recording(),
                QueryHoistBindingV14::recording(),
                WaveRmsNormBindingV15::recording(),
                SplitAttentionBindingV21::recording(),
                true,
            )
        };
        assert!(configure(&mut driver).is_err());
        if selected.is_some() {
            driver.admitted_prefill_kv_copy_v27 = None;
            assert!(configure(&mut driver).is_err());
        }
        assert!(driver.c1_split_attention_v25.is_none());
        assert!(driver.query_hoist_v14.is_none());
        assert!(driver.inner.transports[0].commands.is_empty());
    }
}
