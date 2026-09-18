//! Recording contracts only; numerical and throughput acceptance require GPU runs.

use super::*;
use crate::tp_artifact::{Fp32ArgmaxBindingV11, WaveRmsNormBindingV15};

const WAVE: &str = crate::tp_artifact::ENGINEERING_TP_WAVE_RMSNORM_EXPORTS_V15[0];

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
    // Both arms need the real norm extents; other weights remain synthetic.
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

fn select(driver: &mut EngineeringTpBatchExecutionV2<Recording>, wave: bool) -> TpResult<()> {
    driver.configure_full_forward_rmsnorm_binding_v15(
        Fp32ArgmaxBindingV11::recording(),
        WaveRmsNormBindingV15::recording(),
        wave,
    )
}

#[test]
fn full_forward_norm_changes_only_73_hidden_norm_packets_across_two_epochs() {
    for selected in [Vec::new(), vec![0]] {
        let mut traces = Vec::new();
        for wave in [false, true] {
            let mut pool = pool();
            let mut driver = configured(&pool);
            let allocations = driver.inner.transports[0].buffers.clone();
            select(&mut driver, wave).unwrap();
            assert_eq!(
                driver.rmsnorm_mode(),
                if wave { "wave-v15" } else { "baseline" }
            );
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
                let mut hidden_norms = 0;
                let mut qk_norms = 0;
                for (index, packet) in packets.iter_mut().enumerate() {
                    let hidden_site = index == 613
                        || (index >= 1 && index < 613 && matches!((index - 1) % 17, 0 | 11));
                    if hidden_site {
                        hidden_norms += 1;
                        assert_eq!(packet.kernel, if wave { WAVE } else { RMSNORM });
                        assert_eq!(packet.grid_workgroups, 1);
                        assert_eq!(packet.workgroup_size, 64);
                        assert_eq!(scalar(packet, 5), 1);
                        assert_eq!(scalar(packet, 6), 4096);
                        assert_eq!(buffer(packet, 0).2, 4096);
                        assert_eq!(buffer(packet, 4).2, 4096);
                        packet.kernel = RMSNORM;
                    } else if packet.kernel == RMSNORM {
                        qk_norms += 1;
                        assert_eq!(scalar(packet, 6), 128);
                    }
                }
                assert_eq!((hidden_norms, qk_norms), (73, 72));
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
            "only the 73 hidden norm symbols may differ"
        );
    }
}

#[test]
fn full_forward_norm_rejects_drift_before_sealing() {
    for wave in [false, true] {
        for mutation in 0..23 {
            let mut driver = configured(&pool());
            match mutation {
                0 => driver.admitted_argmax_v11 = None,
                1 => driver.admitted_wave_rmsnorm_v15 = None,
                2 => driver.admitted_wave_rmsnorm_v15.as_mut().unwrap().hsaco[0] ^= 1,
                3 => driver.row_capacity = 32,
                4 => driver.inner.row_capacity = 32,
                5 => driver.wave_attention = false,
                6 => driver.projection_configured = false,
                7 => {
                    driver.projection.mode =
                        super::super::super::EngineeringTpProjectionModeV3::Wave
                }
                8 => driver.head_profile_configured = false,
                9 => driver.fp32_logits = None,
                10 => driver.fp32_argmax_v11 = driver.admitted_argmax_v11,
                11 => driver.wave_rmsnorm_v15 = driver.admitted_wave_rmsnorm_v15,
                12 => driver.inner.transports[0].full_forward_supported = false,
                13 => driver.inner.transports[0].rollover_supported = true,
                14 => driver.inner.transports[0].argmax_peer = Some((0, 1, 0)),
                15 => driver.context_tokens = 128,
                16 => driver.physical_pages = 5,
                17 => driver.last_batch = 1,
                18 => driver.completed_batches = 1,
                19 => driver.poisoned = true,
                20 => driver.inner.closed = true,
                21 => driver.inner.ordered_batches = Some(Vec::new()),
                22 => {
                    driver.admitted_query_hoist_v14 =
                        Some(crate::tp_artifact::QueryHoistBindingV14::recording())
                }
                _ => unreachable!(),
            }
            let state = (driver.fp32_argmax_v11, driver.wave_rmsnorm_v15);
            assert!(select(&mut driver, wave).is_err(), "mutation {mutation}");
            assert!(!driver.inner.full_forward_enabled);
            assert_eq!(state, (driver.fp32_argmax_v11, driver.wave_rmsnorm_v15));
        }
    }
}

#[test]
fn full_forward_norm_admission_does_not_open_other_selectors() {
    let mut driver = configured(&pool());
    assert!(driver.configure_scalar_v3_full_forward().is_err());
    assert!(driver.configure_mfma_v7_full_forward().is_err());
    assert!(driver.configure_mfma_v7_wave_full_forward().is_err());
    assert!(
        driver
            .configure_full_forward_argmax_binding_v11(Fp32ArgmaxBindingV11::recording(), true)
            .is_err()
    );
    select(&mut driver, true).unwrap();
    assert!(select(&mut driver, true).is_err());
}

#[test]
fn full_forward_norm_failures_poison_without_commit() {
    for wave in [false, true] {
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
            select(&mut driver, wave).unwrap();
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
fn full_forward_norm_preallocation_requires_exact_loaded_image_and_transport() {
    for mutation in 0..9 {
        let mut driver = fixture(1, &pool());
        let image = WaveRmsNormBindingV15::recording();
        let transport = &mut driver.inner.transports[0];
        transport.buffers.clear();
        transport.wave_rmsnorm_v15_loaded = Some(image.hsaco);
        match mutation {
            0 => transport.wave_rmsnorm_v15_loaded = None,
            1 => transport.wave_rmsnorm_v15_loaded.as_mut().unwrap()[0] ^= 1,
            2 => transport.argmax_peer = Some((0, 1, 0)),
            3 => transport.full_forward_supported = false,
            4 => transport.rollover_supported = true,
            5 => {
                transport.allocate(4).unwrap();
            }
            _ => (),
        }
        let allocations = transport.buffers.clone();
        let result = validate_full_forward_rmsnorm_binding_v15(
            &mut driver.inner.transports,
            if mutation == 6 { 32 } else { 16 },
            mutation == 7,
            image,
        );
        assert_eq!(result.is_ok(), mutation == 8, "mutation {mutation}");
        assert_eq!(driver.inner.transports[0].buffers, allocations);
    }
}
