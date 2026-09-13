//! Actual host routing with synthetic bytes, not device arithmetic or Qwen parity.

use super::*;
use crate::tp_artifact::{Fp32ArgmaxBindingV11, QueryHoistBindingV14};

const V5: &str = "ferric_qwen3_tp_batch32_wave_paged_gqa_bf16_v5";
const V14: &str = crate::tp_artifact::ENGINEERING_TP_QUERY_HOIST_EXPORTS_V14[0];

fn configured(pool: &EngineeringTpPagedPoolV1) -> EngineeringTpBatchExecutionV2<Recording> {
    let mut driver = fixture(1, pool);
    driver
        .configure_reduction(EngineeringTpReductionModeV3::DeviceTp1V3)
        .unwrap();
    driver.configure_output_head_pruning(true).unwrap();
    let original = driver.inner.ranks[0].global(Qwen3TensorKind::LanguageModelHead);
    let transposed = allocate_tensor(&mut driver.inner.transports[0], 1, 2).unwrap();
    driver.projection =
        super::super::super::projection::ProjectionPolicy::synthetic_mfma_for_recording(
            original.id,
            transposed,
        );
    driver.projection_configured = true;
    driver.configure_wave_attention(true).unwrap();
    driver.configure_head_precision_v8(true).unwrap();
    driver.admitted_argmax_v11 = Some(Fp32ArgmaxBindingV11::recording());
    driver.admitted_query_hoist_v14 = Some(QueryHoistBindingV14::recording());
    driver
}

fn select(driver: &mut EngineeringTpBatchExecutionV2<Recording>) -> TpResult<()> {
    driver.configure_ordered_c1_wave_query_hoist_binding_v14(
        Fp32ArgmaxBindingV11::recording(),
        QueryHoistBindingV14::recording(),
    )
}

fn allocations(driver: &EngineeringTpBatchExecutionV2<Recording>) -> Vec<(u64, usize)> {
    driver.inner.transports[0]
        .buffers
        .iter()
        .map(|(&id, bytes)| (id, bytes.len()))
        .collect()
}

#[test]
fn v14_canary_chunk_and_decode_recordings_keep_same_images_allocations_and_head() {
    // Small synthetic context exercises the canary's three packet shapes, not Qwen arithmetic.
    let mut recordings = Vec::new();
    for enabled in [false, true] {
        let mut pool = wide_pool();
        let mut driver = configured(&pool);
        let transport = &mut driver.inner.transports[0];
        transport.query_hoist_v14_loaded = Some(QueryHoistBindingV14::recording().hsaco);
        transport.argmax_v11_loaded = Some(Fp32ArgmaxBindingV11::recording().hsaco);
        let before = allocations(&driver);
        if enabled {
            select(&mut driver).unwrap();
        } else {
            driver
                .configure_ordered_c1_wave_layers_fp32_argmax_binding_v11(
                    Fp32ArgmaxBindingV11::recording(),
                )
                .unwrap();
        }
        let sequence = pool
            .open_sequence(pool.scope(), &[7; 32], 0)
            .unwrap()
            .sequence();
        let mut position = 0;
        for (rows, publish) in [(16, false), (16, true), (1, true)] {
            let input = (0..rows)
                .map(|offset| EngineeringTpPageRowV1 {
                    sequence,
                    token: 7,
                    position: position + offset,
                })
                .collect::<Vec<_>>();
            let batch = pool.reserve_batch(&input).unwrap();
            pool.begin_submission(&batch).unwrap();
            let selected = if publish {
                vec![rows as usize - 1]
            } else {
                Vec::new()
            };
            let output = driver.execute_selected(&batch, &selected).unwrap();
            assert_eq!(output.choices.len(), usize::from(publish));
            pool.commit_batch(&batch, output.completion).unwrap();
            position += rows;
            assert_eq!(allocations(&driver), before);
        }
        assert_eq!(driver.completed_batches(), 3);
        assert_eq!(driver.dispatch_counts(), [613 + 616 + 616]);
        assert_eq!(pool.committed_position(sequence).unwrap(), 33);
        assert_eq!(driver.layer_projection_mode(), "c1-wave");
        assert_eq!(driver.fp32_argmax_mode(), "wave-v11");
        let transport = &driver.inner.transports[0];
        let mut commands = transport.commands.clone();
        let mut changed = 0;
        for command in &mut commands {
            if command.kernel == if enabled { V14 } else { V5 } {
                changed += 1;
                command.kernel = V5;
            }
        }
        assert_eq!(changed, 3 * 36);
        recordings.push((
            commands,
            transport.write_payloads.clone(),
            transport.reads.clone(),
            transport.packet_preparations.clone(),
            transport.argmax_v11_loaded,
            transport.query_hoist_v14_loaded,
        ));
        pool.retire_sequence(sequence, false, 1).unwrap();
        pool.check_invariants().unwrap();
        assert_eq!(pool.stats().free_pages, 4);
        assert_eq!(pool.stats().quarantined_pages, 0);
        driver.close().unwrap();
    }
    assert_eq!(recordings[0], recordings[1]);
}

#[test]
fn v14_changes_only_attention_root_for_all_active_rows_and_head_selections() {
    for rows in [1, 16, 17, 32] {
        for selected in [
            Vec::new(),
            vec![rows as usize - 1],
            (0..rows as usize).collect(),
        ] {
            let mut recordings = Vec::new();
            for enabled in [false, true] {
                let mut pool = wide_pool();
                let mut driver = configured(&pool);
                let before = allocations(&driver);
                if enabled {
                    select(&mut driver).unwrap();
                } else {
                    driver
                        .configure_ordered_c1_wave_layers_fp32_argmax_binding_v11(
                            Fp32ArgmaxBindingV11::recording(),
                        )
                        .unwrap();
                }
                assert_eq!(
                    driver.attention_mode(),
                    if enabled { "query-hoist-v14" } else { "wave" }
                );
                assert_eq!(driver.layer_projection_mode(), "c1-wave");
                assert_eq!(driver.fp32_argmax_mode(), "wave-v11");
                assert_eq!(allocations(&driver), before);
                let batch = prepare(&mut pool, rows);
                pool.begin_submission(&batch).unwrap();
                let output = driver.execute_selected(&batch, &selected).unwrap();
                assert_eq!(
                    driver.dispatch_counts(),
                    [if selected.is_empty() { 613 } else { 616 }]
                );
                let transport = &driver.inner.transports[0];
                let mut commands = transport.commands.clone();
                let mut changed = 0;
                for command in &mut commands {
                    if command.kernel == if enabled { V14 } else { V5 } {
                        changed += 1;
                        assert_eq!(command.workgroup_size, 64);
                        assert_eq!(command.grid_workgroups, rows * 32);
                        assert_eq!(
                            (6..11).map(|i| scalar(command, i)).collect::<Vec<_>>(),
                            [rows, 1, 4, 4, rows]
                        );
                        command.kernel = V5;
                    }
                }
                assert_eq!(changed, 36);
                let mut events = transport.events.borrow().clone();
                for event in &mut events {
                    if let Event::Submit(_, kernel) | Event::Wait(_, kernel) = event
                        && *kernel == V14
                    {
                        *kernel = V5;
                    }
                }
                recordings.push((
                    commands,
                    transport.reads.clone(),
                    transport.write_payloads.clone(),
                    transport.packet_preparations.clone(),
                    events,
                    output.choices.clone(),
                ));
                assert_eq!(allocations(&driver), before);
                pool.commit_batch(&batch, output.completion).unwrap();
                pool.check_invariants().unwrap();
                driver.close().unwrap();
            }
            // Normalize only the changed GQA name in commands and recording submit/wait events.
            assert_eq!(
                recordings[0], recordings[1],
                "rows {rows}, selected {selected:?}"
            );
        }
    }
}

#[test]
fn v14_preserves_mixed_position_causal_bound_and_decode_after_prefill() {
    let mut recordings = Vec::new();
    for enabled in [false, true] {
        let mut pool = wide_pool();
        let mut driver = configured(&pool);
        if enabled {
            select(&mut driver).unwrap();
        } else {
            driver
                .configure_ordered_c1_wave_layers_fp32_argmax_binding_v11(
                    Fp32ArgmaxBindingV11::recording(),
                )
                .unwrap();
        }
        let first = prepare(&mut pool, 16);
        let continued = first.rows()[0].sequence();
        pool.begin_submission(&first).unwrap();
        let output = driver.execute_selected(&first, &[15]).unwrap();
        pool.commit_batch(&first, output.completion).unwrap();
        let fresh = pool
            .open_sequence(pool.scope(), &[99], 0)
            .unwrap()
            .sequence();
        let second = pool
            .reserve_batch(&[
                EngineeringTpPageRowV1 {
                    sequence: fresh,
                    token: 99,
                    position: 0,
                },
                EngineeringTpPageRowV1 {
                    sequence: continued,
                    token: 100,
                    position: 16,
                },
            ])
            .unwrap();
        pool.begin_submission(&second).unwrap();
        let output = driver.execute_selected(&second, &[1]).unwrap();
        pool.commit_batch(&second, output.completion).unwrap();
        assert_eq!(driver.completed_batches(), 2);
        assert_eq!(driver.dispatch_counts(), [1232]);
        let transport = &driver.inner.transports[0];
        assert_eq!(transport.u32_values(driver.positions[0].id, 2), [16, 0]);
        let mut commands = transport.commands.clone();
        let attention = commands
            .iter()
            .rev()
            .find(|c| c.kernel == if enabled { V14 } else { V5 })
            .unwrap();
        assert_eq!(scalar(attention, 10), 17);
        for command in &mut commands {
            if command.kernel == V14 {
                command.kernel = V5;
            }
        }
        recordings.push((commands, transport.write_payloads.clone(), output.choices));
        driver.close().unwrap();
    }
    assert_eq!(recordings[0], recordings[1]);
}

#[test]
fn v14_terminal_rejection_keeps_all_route_and_io_state_unchanged() {
    for mutation in 0..30 {
        let mut driver = configured(&wide_pool());
        match mutation {
            0 => driver.admitted_query_hoist_v14 = None,
            1 => {
                let mut b = QueryHoistBindingV14::recording();
                b.hsaco[0] ^= 1;
                driver.admitted_query_hoist_v14 = Some(b);
            }
            2 => driver.query_hoist_v14 = Some(QueryHoistBindingV14::recording()),
            3 => driver.admitted_argmax_v11 = None,
            4 => {
                let mut b = Fp32ArgmaxBindingV11::recording();
                b.hsaco[0] ^= 1;
                driver.admitted_argmax_v11 = Some(b);
            }
            5 => driver.row_capacity = 16,
            6 => driver.inner.draft_v10 = true,
            7 => driver.fp32_logits = None,
            8 => driver.head_profile_configured = false,
            9 => driver.inner.large_kv = true,
            10 => driver.inner.sequences = Some(vec![Vec::new()]),
            11 => driver.inner.ordered_batches = Some(Vec::new()),
            12 => driver.wave_attention = false,
            13 => driver.prune_output_head = false,
            14 => {
                driver.projection.mode =
                    super::super::super::EngineeringTpProjectionModeV3::Baseline;
            }
            15 => driver.projection.mode = super::super::super::EngineeringTpProjectionModeV3::Wave,
            16 => driver.last_batch = 1,
            17 => driver.completed_batches = 1,
            18 => driver.poisoned = true,
            19 => driver.inner.closed = true,
            20 => driver.inner.transports[0].argmax_peer = Some((1, 0, 1)),
            21 => driver.inner.transports.clear(),
            22 => driver.inner.ranks.clear(),
            23 => {
                driver.inner.reduction =
                    super::super::super::reduction::ReductionWorkspace::Baseline;
            }
            24 => driver.inner.transports[0].ordered_supported = false,
            25 => driver
                .inner
                .ranks
                .push(fixture(1, &wide_pool()).inner.ranks.pop().unwrap()),
            26 => driver
                .inner
                .transports
                .push(fixture(1, &wide_pool()).inner.transports.pop().unwrap()),
            27 => driver.c1_wave_layers = true,
            28 => driver.fp32_argmax_v11 = Some(Fp32ArgmaxBindingV11::recording()),
            29 => driver.projection.mode = super::super::super::EngineeringTpProjectionModeV3::Auto,
            _ => unreachable!(),
        }
        let before = (
            driver.query_hoist_v14,
            driver.c1_wave_layers,
            driver.fp32_argmax_v11,
            driver.inner.ordered_batches.clone(),
        );
        let io_before = driver
            .inner
            .transports
            .iter()
            .map(|t| {
                (
                    t.buffers.len(),
                    t.commands.clone(),
                    t.reads.clone(),
                    t.writes.clone(),
                    t.events.borrow().clone(),
                )
            })
            .collect::<Vec<_>>();
        assert!(select(&mut driver).is_err(), "mutation {mutation}");
        assert_eq!(
            before,
            (
                driver.query_hoist_v14,
                driver.c1_wave_layers,
                driver.fp32_argmax_v11,
                driver.inner.ordered_batches.clone()
            )
        );
        assert_eq!(
            io_before,
            driver
                .inner
                .transports
                .iter()
                .map(|t| (
                    t.buffers.len(),
                    t.commands.clone(),
                    t.reads.clone(),
                    t.writes.clone(),
                    t.events.borrow().clone()
                ))
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn v14_constructor_binding_checks_loaded_identity_before_allocation() {
    for mutation in 0..9 {
        let world = if mutation == 7 {
            2
        } else if mutation == 8 {
            8
        } else {
            1
        };
        let mut driver = fixture(world, &wide_pool());
        for transport in &mut driver.inner.transports {
            transport.buffers.clear();
            transport.query_hoist_v14_loaded = Some(QueryHoistBindingV14::recording().hsaco);
        }
        let mut binding = QueryHoistBindingV14::recording();
        match mutation {
            0 => driver.inner.transports[0].query_hoist_v14_loaded = None,
            1 => binding.hsaco[0] ^= 1,
            2 => driver.inner.transports[0].argmax_peer = Some((1, 0, 1)),
            3 => driver.inner.transports.clear(),
            6 => {
                driver.inner.transports[0].allocate(4).unwrap();
            }
            _ => (),
        }
        let before = driver
            .inner
            .transports
            .iter()
            .map(|t| t.buffers.len())
            .collect::<Vec<_>>();
        assert!(
            validate_query_hoist_binding_v14(
                &mut driver.inner.transports,
                if mutation == 4 { 16 } else { 32 },
                mutation == 5,
                binding
            )
            .is_err()
        );
        assert_eq!(
            before,
            driver
                .inner
                .transports
                .iter()
                .map(|t| t.buffers.len())
                .collect::<Vec<_>>()
        );
    }
    let mut driver = fixture(1, &wide_pool());
    let transport = &mut driver.inner.transports[0];
    transport.buffers.clear();
    transport.query_hoist_v14_loaded = Some(QueryHoistBindingV14::recording().hsaco);
    assert!(
        transport
            .require_loaded_image(
                QueryHoistBindingV14::recording().hsaco,
                &crate::tp_artifact::ENGINEERING_TP_FP32_ARGMAX32_EXPORTS_V11
            )
            .is_err()
    );
    validate_query_hoist_binding_v14(
        &mut driver.inner.transports,
        32,
        false,
        QueryHoistBindingV14::recording(),
    )
    .unwrap();
    assert!(driver.inner.transports[0].buffers.is_empty());
}

#[test]
fn v14_is_opt_in_terminal_and_does_not_broaden_legacy_selectors() {
    let baseline = fixture(1, &wide_pool());
    assert_eq!(baseline.attention_mode(), "baseline");
    let mut driver = configured(&wide_pool());
    assert_eq!(driver.attention_mode(), "wave");
    driver
        .configure_ordered_c1_wave_layers_fp32_argmax_binding_v11(Fp32ArgmaxBindingV11::recording())
        .unwrap();
    assert!(select(&mut driver).is_err());
    assert_eq!(driver.attention_mode(), "wave");
    let mut driver = configured(&wide_pool());
    select(&mut driver).unwrap();
    assert!(select(&mut driver).is_err());
    assert!(
        driver
            .configure_ordered_c1_wave_layers_fp32_argmax_binding_v11(
                Fp32ArgmaxBindingV11::recording()
            )
            .is_err()
    );
    assert!(
        driver
            .configure_ordered_wave_attention_fp32_argmax_binding_v11(
                Fp32ArgmaxBindingV11::recording()
            )
            .is_err()
    );
    assert!(
        driver
            .configure_wave_attention_fp32_argmax_binding_v11(Fp32ArgmaxBindingV11::recording())
            .is_err()
    );
    assert!(
        driver
            .configure_fp32_argmax_binding_v11(Fp32ArgmaxBindingV11::recording())
            .is_err()
    );
    for enabled in [false, true] {
        assert!(driver.configure_wave_attention(enabled).is_err());
        assert!(driver.configure_output_head_pruning(enabled).is_err());
        assert!(driver.configure_head_precision_v8(enabled).is_err());
        assert!(driver.configure_ordered_batches(enabled).is_err());
        assert!(driver.configure_dispatch_sequences(enabled).is_err());
    }
    assert!(
        driver
            .configure_reduction(EngineeringTpReductionModeV3::DeviceTp1V3)
            .is_err()
    );
    assert_eq!(driver.attention_mode(), "query-hoist-v14");
    assert_eq!(driver.layer_projection_mode(), "c1-wave");
}

#[test]
fn v14_invalid_rows_metadata_and_tokens_publish_no_transport_work() {
    for mutation in 0..6 {
        let mut pool = wide_pool();
        let mut driver = configured(&pool);
        select(&mut driver).unwrap();
        let mut foreign = wide_pool();
        let batch = if mutation == 0 {
            prepare(&mut foreign, 1)
        } else if mutation == 1 {
            let sequence = pool
                .open_sequence(pool.scope(), &[u32::MAX], 0)
                .unwrap()
                .sequence();
            pool.reserve_batch(&[EngineeringTpPageRowV1 {
                sequence,
                token: u32::MAX,
                position: 0,
            }])
            .unwrap()
        } else {
            prepare(&mut pool, 17)
        };
        match mutation {
            2 => driver.context_tokens = 16,
            3 => driver.physical_pages = 1,
            4 => driver.last_batch = batch.id(),
            _ => (),
        }
        let before = allocations(&driver);
        let selected = if mutation == 5 { vec![17] } else { vec![0] };
        assert!(
            driver.execute_selected(&batch, &selected).is_err(),
            "mutation {mutation}"
        );
        assert_eq!(driver.completed_batches(), 0);
        assert_eq!(driver.dispatch_counts(), [0]);
        assert_eq!(allocations(&driver), before);
        let transport = &driver.inner.transports[0];
        assert!(transport.commands.is_empty() && transport.writes.is_empty());
        assert!(transport.packet_preparations.is_empty());
        assert!(driver.inner.ordered_batches.as_ref().unwrap().is_empty());
        driver.close().unwrap();
    }
}

#[test]
fn v14_attention_failures_are_terminal_and_never_publish_completion() {
    for failure in [
        Failure::AttentionSubmit,
        Failure::AttentionWait,
        Failure::OrderedSubmit,
        Failure::OrderedWait,
        Failure::ArgmaxSubmit,
        Failure::ArgmaxWait,
        Failure::BadChoice,
    ] {
        let mut pool = wide_pool();
        let mut driver = configured(&pool);
        select(&mut driver).unwrap();
        driver.inner.transports[0].failure = Some(failure);
        let batch = prepare(&mut pool, 1);
        pool.begin_submission(&batch).unwrap();
        assert!(
            driver.execute_selected(&batch, &[0]).is_err(),
            "{failure:?}"
        );
        assert!(driver.poisoned);
        assert_eq!(driver.completed_batches(), 0);
        assert!(driver.inner.ordered_batches.as_ref().unwrap().is_empty());
        assert!(driver.inner.transports[0].pending.is_none());
        assert!(driver.inner.transports[0].pending_ordered.is_none());
        assert!(driver.execute_selected(&batch, &[0]).is_err());
        pool.quarantine_batch(&batch).unwrap();
        assert_eq!(pool.stats().quarantined_pages, 4);
        assert_eq!(pool.stats().free_pages, 0);
        driver.close().unwrap();
    }
}

#[test]
fn v14_direct_row_binding_rejects_wrong_geometry_and_leaves_v5_mapping_unchanged() {
    use crate::tp_execution::row_profile::bind_mode;
    let mut pool = wide_pool();
    let mut driver = configured(&pool);
    select(&mut driver).unwrap();
    let batch = prepare(&mut pool, 17);
    pool.begin_submission(&batch).unwrap();
    driver.execute_selected(&batch, &[16]).unwrap();
    let original = driver.inner.transports[0]
        .commands
        .iter()
        .find(|c| c.kernel == V14)
        .unwrap()
        .clone();
    assert_eq!(
        bind_mode(false, 32, false, original.clone()).unwrap(),
        original
    );
    for mutation in 0..15 {
        let mut command = original.clone();
        match mutation {
            0 => {
                command.arguments.pop();
            }
            1 => command.arguments[6] = EngineeringTpArgumentV1::U32(0),
            2 => command.arguments[6] = EngineeringTpArgumentV1::U32(33),
            3 => command.arguments[7] = EngineeringTpArgumentV1::U32(2),
            4 => command.arguments[7] = EngineeringTpArgumentV1::U32(8),
            5 => command.arguments[8] = EngineeringTpArgumentV1::U32(0),
            6 => command.arguments[9] = EngineeringTpArgumentV1::U32(513),
            7 => command.arguments[10] = EngineeringTpArgumentV1::U32(8193),
            8 => command.arguments[10] = EngineeringTpArgumentV1::U32(65),
            9 => command.workgroup_size = 32,
            10 => command.grid_workgroups += 1,
            11 => command.arguments[0] = EngineeringTpArgumentV1::U32(0),
            12 => {
                if let EngineeringTpArgumentV1::Buffer { element_bytes, .. } =
                    &mut command.arguments[3]
                {
                    *element_bytes = 2;
                }
            }
            13 => {
                if let EngineeringTpArgumentV1::Buffer { access, .. } = &mut command.arguments[5] {
                    *access = EngineeringTpBufferAccessV1::Read;
                }
            }
            14 => {
                if let EngineeringTpArgumentV1::Buffer { access, .. } = &mut command.arguments[0] {
                    *access = EngineeringTpBufferAccessV1::Write;
                }
            }
            _ => unreachable!(),
        }
        assert!(
            bind_mode(false, 32, false, command).is_err(),
            "mutation {mutation}"
        );
    }
    assert!(bind_mode(false, 16, false, original.clone()).is_err());
    assert!(bind_mode(false, 32, true, original.clone()).is_err());
    assert!(bind_mode(true, 32, false, original.clone()).is_err());
    let mut legacy = original;
    legacy.kernel = "ferric_qwen3_tp_wave_paged_gqa_bf16_v3";
    assert_eq!(
        bind_mode(false, 32, false, legacy.clone()).unwrap().kernel,
        V5
    );
    assert_eq!(bind_mode(false, 16, false, legacy.clone()).unwrap(), legacy);
    driver.close().unwrap();
}
