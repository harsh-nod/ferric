//! Actual routing with synthetic byte writers, not RMSNorm emulation or Qwen parity.

use super::*;
use crate::tp_artifact::{Fp32ArgmaxBindingV11, QueryHoistBindingV14, WaveRmsNormBindingV15};

const V15: &str = crate::tp_artifact::ENGINEERING_TP_WAVE_RMSNORM_EXPORTS_V15[0];

fn configured(pool: &EngineeringTpPagedPoolV1) -> EngineeringTpBatchExecutionV2<Recording> {
    let mut driver = fixture(1, pool);
    driver.configure_reduction(EngineeringTpReductionModeV3::DeviceTp1V3).unwrap();
    driver.configure_output_head_pruning(true).unwrap();
    let original = driver.inner.ranks[0].global(Qwen3TensorKind::LanguageModelHead);
    let transposed = allocate_tensor(&mut driver.inner.transports[0], 1, 2).unwrap();
    driver.projection = super::super::super::projection::ProjectionPolicy::synthetic_mfma_for_recording(original.id, transposed);
    driver.projection_configured = true;
    driver.configure_wave_attention(true).unwrap();
    driver.configure_head_precision_v8(true).unwrap();
    driver.admitted_argmax_v11 = Some(Fp32ArgmaxBindingV11::recording());
    driver.admitted_wave_rmsnorm_v15 = Some(WaveRmsNormBindingV15::recording());
    // Supply real norm extents in both arms; other weights retain the existing synthetic fixture.
    let norm_weight = allocate_tensor(&mut driver.inner.transports[0], 4096, 2).unwrap();
    for layer in &mut driver.inner.ranks[0].layers {
        for (kind, weight) in &mut layer.weights {
            if matches!(kind, Qwen3TensorKind::InputLayerNorm | Qwen3TensorKind::PostAttentionLayerNorm) {
                *weight = norm_weight;
            }
        }
    }
    for (kind, weight) in &mut driver.inner.ranks[0].globals {
        if *kind == Qwen3TensorKind::FinalNorm { *weight = norm_weight; }
    }
    driver
}

fn select(driver: &mut EngineeringTpBatchExecutionV2<Recording>) -> TpResult<()> {
    driver.configure_ordered_c1_wave_rmsnorm_binding_v15(
        Fp32ArgmaxBindingV11::recording(), WaveRmsNormBindingV15::recording(),
    )
}

fn allocations(driver: &EngineeringTpBatchExecutionV2<Recording>) -> Vec<(u64, usize)> {
    driver.inner.transports[0].buffers.iter().map(|(&id, bytes)| (id, bytes.len())).collect()
}

#[test]
fn wave_rmsnorm_v15_changes_only_three_norm_sites_for_all_rows_and_head_selections() {
    for rows in [1, 16, 17, 32] {
        for selected in [Vec::new(), vec![rows as usize - 1], (0..rows as usize).collect()] {
            let mut recordings = Vec::new();
            for enabled in [false, true] {
                let mut pool = wide_pool();
                let mut driver = configured(&pool);
                let before = allocations(&driver);
                if enabled { select(&mut driver).unwrap(); } else {
                    driver.configure_ordered_c1_wave_layers_fp32_argmax_binding_v11(Fp32ArgmaxBindingV11::recording()).unwrap();
                }
                assert_eq!(driver.rmsnorm_mode(), if enabled { "wave-v15" } else { "baseline" });
                assert_eq!(driver.attention_mode(), "wave");
                assert_eq!(driver.layer_projection_mode(), "c1-wave");
                assert_eq!(driver.fp32_argmax_mode(), "wave-v11");
                let batch = prepare(&mut pool, rows);
                pool.begin_submission(&batch).unwrap();
                let output = driver.execute_selected(&batch, &selected).unwrap();
                assert_eq!(driver.dispatch_counts(), [if selected.is_empty() { 613 } else { 616 }]);
                let transport = &driver.inner.transports[0];
                let mut commands = transport.commands.clone();
                let mut norm_rows = Vec::new();
                let mut qk_norms = 0;
                for command in &mut commands {
                    if command.kernel == if enabled { V15 } else { RMSNORM } && scalar(command, 6) == 4096 {
                        norm_rows.push(scalar(command, 5));
                        assert_eq!(command.workgroup_size, 64);
                        assert_eq!(command.grid_workgroups, scalar(command, 5));
                        assert_eq!(buffer(command, 0).2, scalar(command, 5) as usize * 4096);
                        assert_eq!(buffer(command, 4).2, buffer(command, 0).2);
                        assert_eq!(buffer(command, 2).2, 4096);
                        assert_eq!((buffer(command, 1).2, buffer(command, 3).2), (0, 0));
                        assert_eq!(command.arguments[7], EngineeringTpArgumentV1::F32(1.0e-6));
                        assert_eq!(scalar(command, 8), 0);
                        command.kernel = RMSNORM;
                    } else if command.kernel == RMSNORM {
                        assert_eq!(scalar(command, 6), 128);
                        qk_norms += 1;
                    }
                }
                let mut expected_rows = vec![rows; 72];
                if !selected.is_empty() { expected_rows.push(u32::try_from(selected.len()).unwrap()); }
                assert_eq!(norm_rows, expected_rows);
                assert_eq!(qk_norms, 72);
                assert_eq!(commands.iter().filter(|c| c.kernel == "ferric_qwen3_tp_batch32_wave_paged_gqa_bf16_v5").count(), 36);
                let mut events = transport.events.borrow().clone();
                for event in &mut events {
                    if let Event::Submit(_, kernel) | Event::Wait(_, kernel) = event && *kernel == V15 { *kernel = RMSNORM; }
                }
                recordings.push((commands, transport.reads.clone(), transport.write_payloads.clone(), transport.packet_preparations.clone(), events, output.choices.clone()));
                assert_eq!(allocations(&driver), before);
                pool.commit_batch(&batch, output.completion).unwrap();
                pool.check_invariants().unwrap();
                driver.close().unwrap();
            }
            assert_eq!(recordings[0], recordings[1], "rows {rows}, selected {selected:?}");
        }
    }
}

#[test]
fn wave_rmsnorm_v15_terminal_rejection_is_atomic() {
    for mutation in 0..33 {
        let mut driver = configured(&wide_pool());
        match mutation {
            0 => driver.admitted_wave_rmsnorm_v15 = None,
            1 => { let mut b = WaveRmsNormBindingV15::recording(); b.hsaco[0] ^= 1; driver.admitted_wave_rmsnorm_v15 = Some(b); }
            2 => driver.wave_rmsnorm_v15 = Some(WaveRmsNormBindingV15::recording()),
            3 => driver.admitted_argmax_v11 = None,
            4 => { let mut b = Fp32ArgmaxBindingV11::recording(); b.hsaco[0] ^= 1; driver.admitted_argmax_v11 = Some(b); }
            5 => driver.row_capacity = 16,
            6 => driver.inner.draft_v10 = true,
            7 => driver.fp32_logits = None,
            8 => driver.head_profile_configured = false,
            9 => driver.inner.large_kv = true,
            10 => driver.inner.sequences = Some(vec![Vec::new()]),
            11 => driver.inner.ordered_batches = Some(Vec::new()),
            12 => driver.wave_attention = false,
            13 => driver.prune_output_head = false,
            14 => driver.projection.mode = super::super::super::EngineeringTpProjectionModeV3::Baseline,
            15 => driver.projection.mode = super::super::super::EngineeringTpProjectionModeV3::Wave,
            16 => driver.last_batch = 1,
            17 => driver.completed_batches = 1,
            18 => driver.poisoned = true,
            19 => driver.inner.closed = true,
            20 => driver.inner.transports[0].argmax_peer = Some((1, 0, 1)),
            21 => driver.inner.transports.clear(),
            22 => driver.inner.ranks.clear(),
            23 => driver.inner.reduction = super::super::super::reduction::ReductionWorkspace::Baseline,
            24 => driver.inner.transports[0].ordered_supported = false,
            25 => driver.inner.ranks.push(fixture(1, &wide_pool()).inner.ranks.pop().unwrap()),
            26 => driver.inner.transports.push(fixture(1, &wide_pool()).inner.transports.pop().unwrap()),
            27 => driver.c1_wave_layers = true,
            28 => driver.fp32_argmax_v11 = Some(Fp32ArgmaxBindingV11::recording()),
            29 => driver.projection.mode = super::super::super::EngineeringTpProjectionModeV3::Auto,
            30 => driver.query_hoist_v14 = Some(QueryHoistBindingV14::recording()),
            31 => driver.admitted_query_hoist_v14 = Some(QueryHoistBindingV14::recording()),
            32 => driver.row_capacity = 1,
            _ => unreachable!(),
        }
        let before = (driver.wave_rmsnorm_v15, driver.query_hoist_v14, driver.c1_wave_layers, driver.fp32_argmax_v11, driver.inner.ordered_batches.clone());
        let io = |driver: &EngineeringTpBatchExecutionV2<Recording>| driver.inner.transports.iter()
            .map(|t| (t.buffers.len(), t.commands.clone(), t.reads.clone(), t.writes.clone(), t.events.borrow().clone()))
            .collect::<Vec<_>>();
        let io_before = io(&driver);
        assert!(select(&mut driver).is_err(), "mutation {mutation}");
        assert_eq!(before, (driver.wave_rmsnorm_v15, driver.query_hoist_v14, driver.c1_wave_layers, driver.fp32_argmax_v11, driver.inner.ordered_batches.clone()));
        assert_eq!(io_before, io(&driver));
    }
}

#[test]
fn wave_rmsnorm_v15_binding_requires_loaded_identity_before_allocation() {
    for mutation in 0..10 {
        let mut driver = fixture(if mutation == 7 { 2 } else if mutation == 8 { 8 } else { 1 }, &wide_pool());
        for transport in &mut driver.inner.transports {
            transport.buffers.clear();
            transport.wave_rmsnorm_v15_loaded = Some(WaveRmsNormBindingV15::recording().hsaco);
        }
        let mut binding = WaveRmsNormBindingV15::recording();
        match mutation {
            0 => driver.inner.transports[0].wave_rmsnorm_v15_loaded = None,
            1 => binding.hsaco[0] ^= 1,
            2 => driver.inner.transports[0].argmax_peer = Some((1, 0, 1)),
            3 => driver.inner.transports.clear(),
            6 => { driver.inner.transports[0].allocate(4).unwrap(); }
            _ => (),
        }
        let before = driver.inner.transports.iter().map(|t| t.buffers.len()).collect::<Vec<_>>();
        let result = validate_wave_rmsnorm_binding_v15(&mut driver.inner.transports, if mutation == 4 { 16 } else { 32 }, mutation == 5, binding);
        assert_eq!(result.is_ok(), mutation == 9);
        assert_eq!(before, driver.inner.transports.iter().map(|t| t.buffers.len()).collect::<Vec<_>>());
        if mutation == 9 {
            assert!(driver.inner.transports[0].require_loaded_image(binding.hsaco, &crate::tp_artifact::ENGINEERING_TP_QUERY_HOIST_EXPORTS_V14).is_err());
        }
    }
}

#[test]
fn wave_rmsnorm_v15_is_opt_in_and_freezes_legacy_selectors() {
    assert_eq!(fixture(1, &wide_pool()).rmsnorm_mode(), "baseline");
    let mut driver = configured(&wide_pool());
    assert_eq!(driver.rmsnorm_mode(), "baseline");
    driver.configure_ordered_c1_wave_layers_fp32_argmax_binding_v11(Fp32ArgmaxBindingV11::recording()).unwrap();
    assert!(select(&mut driver).is_err());
    assert_eq!(driver.rmsnorm_mode(), "baseline");
    let mut driver = configured(&wide_pool());
    select(&mut driver).unwrap();
    assert!(select(&mut driver).is_err());
    assert!(driver.configure_ordered_c1_wave_layers_fp32_argmax_binding_v11(Fp32ArgmaxBindingV11::recording()).is_err());
    assert!(driver.configure_ordered_c1_wave_query_hoist_binding_v14(Fp32ArgmaxBindingV11::recording(), QueryHoistBindingV14::recording()).is_err());
    assert!(driver.configure_fp32_argmax_binding_v11(Fp32ArgmaxBindingV11::recording()).is_err());
    for enabled in [false, true] {
        assert!(driver.configure_wave_attention(enabled).is_err());
        assert!(driver.configure_output_head_pruning(enabled).is_err());
        assert!(driver.configure_head_precision_v8(enabled).is_err());
        assert!(driver.configure_ordered_batches(enabled).is_err());
        assert!(driver.configure_dispatch_sequences(enabled).is_err());
    }
    assert_eq!(driver.rmsnorm_mode(), "wave-v15");
    assert_eq!(driver.attention_mode(), "wave");
}

#[test]
fn wave_rmsnorm_v15_invalid_batch_metadata_publishes_no_work() {
    for mutation in 0..6 {
        let mut pool = wide_pool();
        let mut driver = configured(&pool);
        select(&mut driver).unwrap();
        let mut foreign = wide_pool();
        let batch = if mutation == 0 { prepare(&mut foreign, 1) } else if mutation == 1 {
            let sequence = pool.open_sequence(pool.scope(), &[u32::MAX], 0).unwrap().sequence();
            pool.reserve_batch(&[EngineeringTpPageRowV1 { sequence, token: u32::MAX, position: 0 }]).unwrap()
        } else { prepare(&mut pool, 17) };
        match mutation { 2 => driver.context_tokens = 16, 3 => driver.physical_pages = 1, 4 => driver.last_batch = batch.id(), _ => () }
        let before = allocations(&driver);
        assert!(driver.execute_selected(&batch, &[if mutation == 5 { 17 } else { 0 }]).is_err());
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
fn wave_rmsnorm_v15_failures_at_each_norm_site_never_publish_completion() {
    for ordinal in [0, 1, 72] {
        for failure in [Failure::RmsNormSubmit(ordinal), Failure::RmsNormWait(ordinal)] {
            let mut pool = wide_pool();
            let mut driver = configured(&pool);
            select(&mut driver).unwrap();
            driver.inner.transports[0].failure = Some(failure);
            let batch = prepare(&mut pool, 1);
            pool.begin_submission(&batch).unwrap();
            let error = driver.execute_selected(&batch, &[0]).err().expect("injected norm failure");
            assert!(error.contains("injected Wave RMSNorm"), "{failure:?}: {error}");
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
}

#[test]
fn wave_rmsnorm_v15_row_binding_is_exact_and_preserves_legacy_norms() {
    use crate::tp_execution::row_profile::{bind, bind_mode, bind_storage};
    for rows in [1, 16, 17, 32] {
        let driver = configured(&wide_pool());
        let rank = &driver.inner.ranks[0];
        let legacy = norm(rank, rank.hidden, rank.global(Qwen3TensorKind::FinalNorm), rank.normalized, rows, 4096);
        let mut command = legacy.clone();
        command.kernel = V15;
        assert_eq!(bind_mode(false, 32, false, command.clone()).unwrap(), command);
        assert!(bind_mode(true, 32, false, command.clone()).is_err());
        assert!(bind_storage(32, true, command.clone()).is_err());
        for capacity in [1, 16] { assert!(bind(capacity, command.clone()).is_err()); }
        for mutation in 0..13 {
            let mut bad = command.clone();
            match mutation {
                0 => bad.workgroup_size = 32,
                1 => bad.grid_workgroups += 1,
                2 => bad.arguments[5] = EngineeringTpArgumentV1::U32(0),
                3 => bad.arguments[5] = EngineeringTpArgumentV1::U32(33),
                4 => bad.arguments[6] = EngineeringTpArgumentV1::U32(128),
                5 => bad.arguments[6] = EngineeringTpArgumentV1::U32(1024),
                6 => bad.arguments[7] = EngineeringTpArgumentV1::F32(0.0),
                7 => bad.arguments[7] = EngineeringTpArgumentV1::F32(f32::NAN),
                8 => bad.arguments[7] = EngineeringTpArgumentV1::F32(f32::from_bits(1.0e-6_f32.to_bits() + 1)),
                9 => bad.arguments[8] = EngineeringTpArgumentV1::U32(1),
                10 => { bad.arguments.pop(); }
                11 => bad.arguments.push(EngineeringTpArgumentV1::U32(0)),
                12 => bad.arguments[5] = EngineeringTpArgumentV1::F32(1.0),
                _ => unreachable!(),
            }
            assert!(bind(32, bad).is_err(), "rows {rows}, mutation {mutation}");
        }
        for index in 0..5 {
            for mutation in 0..5 {
                let mut bad = command.clone();
                let EngineeringTpArgumentV1::Buffer { offset, elements, element_bytes, access, .. } = &mut bad.arguments[index] else { unreachable!() };
                match mutation {
                    0 => *offset = 2,
                    1 => *elements += 1,
                    2 => *element_bytes = 4,
                    3 => *access = if index >= 3 { EngineeringTpBufferAccessV1::Read } else { EngineeringTpBufferAccessV1::Write },
                    4 => bad.arguments[index] = EngineeringTpArgumentV1::U32(0),
                    _ => unreachable!(),
                }
                assert!(bind(32, bad).is_err(), "buffer {index}, mutation {mutation}");
            }
        }
        for width in [128, 1024, 4096] {
            let mut old = legacy.clone();
            old.arguments[6] = EngineeringTpArgumentV1::U32(width);
            for behavior in [0, 1] {
                old.arguments[8] = EngineeringTpArgumentV1::U32(behavior);
                assert_eq!(bind(16, old.clone()).unwrap(), old);
                assert_eq!(bind(32, old.clone()).unwrap(), old);
            }
        }
    }
}
