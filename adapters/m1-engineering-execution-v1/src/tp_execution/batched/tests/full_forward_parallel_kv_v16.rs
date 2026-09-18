//! Recording contracts only; GPU numerical and performance gates remain separate.

use super::*;
use crate::tp_artifact::{Fp32ArgmaxBindingV11, ParallelKvBindingV16, WaveRmsNormBindingV15};

const PARALLEL: &str = crate::tp_artifact::ENGINEERING_TP_PARALLEL_KV_EXPORTS_V16[0];

fn configured(pool: &EngineeringTpPagedPoolV1) -> EngineeringTpBatchExecutionV2<Recording> {
    let mut driver = fixture(1, pool);
    driver
        .configure_reduction(EngineeringTpReductionModeV3::DeviceTp1V3)
        .unwrap();
    let original = driver.inner.ranks[0].global(Qwen3TensorKind::LanguageModelHead);
    let transposed = allocate_tensor(&mut driver.inner.transports[0], 1, 2).unwrap();
    driver.projection =
        super::super::super::projection::ProjectionPolicy::synthetic_mfma_for_recording(
            original.id,
            transposed,
        );
    driver.projection_configured = true;
    driver.configure_wave_attention(true).unwrap();
    driver.configure_head_precision_v7(true).unwrap();
    driver.admitted_argmax_v11 = Some(Fp32ArgmaxBindingV11::recording());
    driver.admitted_wave_rmsnorm_v15 = Some(WaveRmsNormBindingV15::recording());
    driver.admitted_parallel_kv_v16 = Some(ParallelKvBindingV16::recording());
    let norm_weight = allocate_tensor(&mut driver.inner.transports[0], 4096, 2).unwrap();
    for layer in &mut driver.inner.ranks[0].layers {
        for (kind, weight) in &mut layer.weights {
            if matches!(
                kind,
                Qwen3TensorKind::InputLayerNorm | Qwen3TensorKind::PostAttentionLayerNorm
            ) {
                *weight = norm_weight;
            }
        }
    }
    for (kind, weight) in &mut driver.inner.ranks[0].globals {
        if *kind == Qwen3TensorKind::FinalNorm {
            *weight = norm_weight;
        }
    }
    driver
}

fn select(driver: &mut EngineeringTpBatchExecutionV2<Recording>, parallel: bool) -> TpResult<()> {
    driver.configure_full_forward_parallel_kv_binding_v16(
        Fp32ArgmaxBindingV11::recording(),
        WaveRmsNormBindingV15::recording(),
        ParallelKvBindingV16::recording(),
        parallel,
    )
}

#[test]
fn parallel_kv_v16_changes_only_36_symbols_and_grids_across_two_epochs() {
    for selected in [Vec::new(), vec![0]] {
        let mut traces = Vec::new();
        for parallel in [false, true] {
            let mut pool = pool();
            let mut driver = configured(&pool);
            let allocations = driver.inner.transports[0].buffers.clone();
            select(&mut driver, parallel).unwrap();
            assert_eq!(
                driver.kv_append_mode(),
                if parallel { "parallel-v16" } else { "baseline" }
            );
            assert_eq!(driver.rmsnorm_mode(), "wave-v15");
            assert_eq!(driver.inner.transports[0].buffers, allocations);
            for epoch in 1..=2 {
                let batch = prepare(&mut pool, 1);
                pool.begin_submission(&batch).unwrap();
                let output = driver.execute_selected(&batch, &selected).unwrap();
                assert_eq!(driver.dispatch_counts(), [616 * epoch]);
                assert_eq!(driver.inner.collective.expected().epoch, epoch);
                pool.commit_batch(&batch, output.completion).unwrap();
            }
            let transport = &driver.inner.transports[0];
            assert_eq!(transport.packet_preparations, [616, 616]);
            assert_eq!(transport.reads.len(), 2);
            let mut commands = transport.commands.clone();
            for packets in commands.chunks_exact_mut(616) {
                let mut copies = 0;
                for packet in packets.iter_mut() {
                    if packet.kernel == APPEND || packet.kernel == PARALLEL {
                        copies += 1;
                        assert_eq!(packet.kernel, if parallel { PARALLEL } else { APPEND });
                        assert_eq!(packet.grid_workgroups, if parallel { 64 } else { 1 });
                        assert_eq!(packet.workgroup_size, 64);
                        assert_eq!(
                            (
                                scalar(packet, 6),
                                scalar(packet, 7),
                                scalar(packet, 8),
                                scalar(packet, 9)
                            ),
                            (1, 1, 4, 4)
                        );
                        for (index, extent) in
                            [16384, 16384, 16, 64, 65536, 65536].into_iter().enumerate()
                        {
                            assert_eq!(buffer(packet, index).2, extent);
                        }
                        packet.kernel = APPEND;
                        packet.grid_workgroups = 1;
                    }
                }
                assert_eq!(copies, 36);
                assert_eq!(
                    packets
                        .iter()
                        .filter(|packet| packet.kernel
                            == crate::tp_artifact::ENGINEERING_TP_WAVE_RMSNORM_EXPORTS_V15[0])
                        .count(),
                    73
                );
                assert_eq!(
                    packets[615].kernel,
                    crate::tp_artifact::ENGINEERING_TP_FP32_ARGMAX32_EXPORTS_V11[0]
                );
            }
            traces.push((
                commands,
                transport.reads.clone(),
                transport.write_payloads.clone(),
                transport.buffers.clone(),
                driver.inner.hidden.clone(),
                driver.inner.ranks[0].hidden,
                driver.inner.collective,
                driver.last_batch,
                driver.completed_batches,
            ));
            driver.close().unwrap();
        }
        assert!(
            traces[0] == traces[1],
            "only the 36 KV symbols and grids may differ"
        );
    }
}

#[test]
fn parallel_kv_v16_rejects_drift_before_sealing() {
    for parallel in [false, true] {
        for mutation in 0..30 {
            let mut driver = configured(&pool());
            match mutation {
                0 => driver.admitted_argmax_v11 = None,
                1 => driver.admitted_wave_rmsnorm_v15 = None,
                2 => driver.admitted_parallel_kv_v16 = None,
                3 => driver.admitted_argmax_v11.as_mut().unwrap().hsaco[0] ^= 1,
                4 => driver.admitted_wave_rmsnorm_v15.as_mut().unwrap().hsaco[0] ^= 1,
                5 => driver.admitted_parallel_kv_v16.as_mut().unwrap().hsaco[0] ^= 1,
                6 => driver.row_capacity = 32,
                7 => driver.inner.row_capacity = 32,
                8 => driver.wave_attention = false,
                9 => driver.projection_configured = false,
                10 => {
                    driver.projection.mode =
                        super::super::super::EngineeringTpProjectionModeV3::Wave
                }
                11 => driver.head_profile_configured = false,
                12 => driver.fp32_logits = None,
                13 => driver.fp32_argmax_v11 = driver.admitted_argmax_v11,
                14 => driver.wave_rmsnorm_v15 = driver.admitted_wave_rmsnorm_v15,
                15 => driver.parallel_kv_v16 = driver.admitted_parallel_kv_v16,
                16 => driver.inner.transports[0].full_forward_supported = false,
                17 => driver.inner.transports[0].rollover_supported = true,
                18 => driver.inner.transports[0].argmax_peer = Some((0, 1, 0)),
                19 => driver.context_tokens = 128,
                20 => driver.physical_pages = 5,
                21 => driver.table_stride = 5,
                22 => driver.last_batch = 1,
                23 => driver.completed_batches = 1,
                24 => driver.poisoned = true,
                25 => driver.inner.closed = true,
                26 => driver.inner.ordered_batches = Some(Vec::new()),
                27 => {
                    driver.admitted_query_hoist_v14 =
                        Some(crate::tp_artifact::QueryHoistBindingV14::recording())
                }
                28 => driver.inner.large_kv = true,
                29 => driver.prune_output_head = true,
                _ => unreachable!(),
            }
            let state = (
                driver.fp32_argmax_v11,
                driver.wave_rmsnorm_v15,
                driver.parallel_kv_v16,
            );
            assert!(
                select(&mut driver, parallel).is_err(),
                "mutation {mutation}"
            );
            assert!(!driver.inner.full_forward_enabled);
            assert_eq!(
                state,
                (
                    driver.fp32_argmax_v11,
                    driver.wave_rmsnorm_v15,
                    driver.parallel_kv_v16
                )
            );
        }
    }
}

#[test]
fn parallel_kv_v16_admission_does_not_open_other_selectors_or_unsealed_execution() {
    let mut pool = pool();
    let mut driver = configured(&pool);
    assert!(driver.configure_scalar_v3_ordered_batches().is_err());
    assert!(driver.configure_scalar_v3_full_forward().is_err());
    assert!(driver.configure_mfma_v7_full_forward().is_err());
    assert!(driver.configure_mfma_v7_wave_full_forward().is_err());
    assert!(
        driver
            .configure_full_forward_argmax_binding_v11(Fp32ArgmaxBindingV11::recording(), true)
            .is_err()
    );
    for norm in [false, true] {
        assert!(
            driver
                .configure_full_forward_rmsnorm_binding_v15(
                    Fp32ArgmaxBindingV11::recording(),
                    WaveRmsNormBindingV15::recording(),
                    norm
                )
                .is_err()
        );
    }
    let batch = prepare(&mut pool, 1);
    assert!(driver.execute(&batch).is_err());
    assert_eq!(driver.dispatch_counts(), [0]);
    select(&mut driver, true).unwrap();
    assert!(select(&mut driver, false).is_err());
}

#[test]
fn parallel_kv_v16_failures_poison_without_commit() {
    for parallel in [false, true] {
        for failure in [
            Failure::FullForwardSubmit,
            Failure::FullForwardWait,
            Failure::Read,
            Failure::BadChoice,
            Failure::Write,
            Failure::PreparePackets,
        ] {
            let mut pool = pool();
            let mut driver = configured(&pool);
            select(&mut driver, parallel).unwrap();
            let before = (
                driver.inner.ranks[0].hidden,
                driver.inner.collective,
                driver.inner.hidden.clone(),
            );
            driver.inner.transports[0].failure = Some(failure);
            let batch = prepare(&mut pool, 1);
            pool.begin_submission(&batch).unwrap();
            assert!(driver.execute(&batch).is_err(), "{failure:?}");
            assert_eq!(
                before,
                (
                    driver.inner.ranks[0].hidden,
                    driver.inner.collective,
                    driver.inner.hidden.clone()
                )
            );
            assert_eq!(driver.dispatch_counts(), [0]);
            assert_eq!((driver.last_batch, driver.completed_batches), (0, 0));
            assert!(driver.poisoned);
            assert!(driver.execute(&batch).is_err());
            pool.quarantine_batch(&batch).unwrap();
            driver.close().unwrap();
        }
    }
}

#[test]
fn parallel_kv_v16_preallocation_requires_exact_loaded_image_and_transport() {
    for mutation in 0..11 {
        let mut driver = fixture(1, &pool());
        let image = ParallelKvBindingV16::recording();
        let transport = &mut driver.inner.transports[0];
        transport.buffers.clear();
        transport.parallel_kv_v16_loaded = Some(image.hsaco);
        match mutation {
            0 => transport.parallel_kv_v16_loaded = None,
            1 => transport.parallel_kv_v16_loaded.as_mut().unwrap()[0] ^= 1,
            2 => transport.argmax_peer = Some((0, 1, 0)),
            3 => transport.full_forward_supported = false,
            4 => transport.rollover_supported = true,
            5 => {
                transport.allocate(4).unwrap();
            }
            _ => (),
        }
        let allocations = transport.buffers.clone();
        let result = validate_full_forward_parallel_kv_binding_v16(
            &mut driver.inner.transports,
            if mutation == 6 { 32 } else { 16 },
            mutation == 7,
            if mutation == 8 { 128 } else { 64 },
            if mutation == 9 { 5 } else { 4 },
            image,
        );
        assert_eq!(result.is_ok(), mutation == 10, "mutation {mutation}");
        assert_eq!(driver.inner.transports[0].buffers, allocations);
    }
    let image = ParallelKvBindingV16::recording();
    assert!(
        validate_full_forward_parallel_kv_binding_v16::<Recording>(
            &mut [],
            16,
            false,
            64,
            4,
            image
        )
        .is_err()
    );
    let mut driver = fixture(2, &pool());
    assert!(
        validate_full_forward_parallel_kv_binding_v16(
            &mut driver.inner.transports,
            16,
            false,
            64,
            4,
            image
        )
        .is_err()
    );
}

#[test]
fn parallel_kv_v16_route_rejects_all_geometry_and_buffer_drift() {
    use super::super::super::row_profile::{bind_mode, bind_storage};
    use crate::tp_execution::EngineeringTpBufferAccessV1::{Read, Write};
    let mut pool = pool();
    let mut driver = configured(&pool);
    select(&mut driver, true).unwrap();
    let batch = prepare(&mut pool, 1);
    driver.execute(&batch).unwrap();
    let command = driver.inner.transports[0]
        .commands
        .iter()
        .find(|packet| packet.kernel == PARALLEL)
        .unwrap()
        .clone();
    assert_eq!(bind_storage(16, false, command.clone()).unwrap(), command);
    for capacity in [0, 1, 32] {
        assert!(bind_storage(capacity, false, command.clone()).is_err());
    }
    assert!(bind_storage(16, true, command.clone()).is_err());
    assert!(bind_storage(32, true, command.clone()).is_err());
    assert!(bind_mode(true, 32, false, command.clone()).is_err());
    for scalar_index in 6..10 {
        let mut changed = command.clone();
        changed.arguments[scalar_index] = EngineeringTpArgumentV1::U32(0);
        assert!(bind_storage(16, false, changed).is_err());
    }
    for mutation in 0..4 {
        let mut changed = command.clone();
        match mutation {
            0 => changed.grid_workgroups = 1,
            1 => changed.workgroup_size = 32,
            2 => {
                changed.arguments.pop();
            }
            3 => changed.arguments.push(EngineeringTpArgumentV1::U32(0)),
            _ => unreachable!(),
        }
        assert!(bind_storage(16, false, changed).is_err());
    }
    for index in 0..6 {
        for mutation in 0..5 {
            let mut changed = command.clone();
            if let EngineeringTpArgumentV1::Buffer {
                offset,
                elements,
                element_bytes,
                access,
                ..
            } = &mut changed.arguments[index]
            {
                match mutation {
                    0 => *offset = 2,
                    1 => *elements -= 1,
                    2 => *elements += 1,
                    3 => *element_bytes = 1,
                    4 => *access = if index >= 4 { Read } else { Write },
                    _ => unreachable!(),
                }
            } else {
                panic!("expected buffer");
            }
            assert!(
                bind_storage(16, false, changed).is_err(),
                "buffer {index}, mutation {mutation}"
            );
        }
    }
}
