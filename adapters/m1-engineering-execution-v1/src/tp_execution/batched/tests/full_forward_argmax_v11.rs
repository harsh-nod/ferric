//! Host recording contracts, not GPU numerical or performance evidence.

use super::super::super::{EngineeringTpProjectionModeV3, ReductionWorkspace};
use super::*;
use crate::tp_artifact::{ENGINEERING_TP_FP32_ARGMAX32_EXPORTS_V11, Fp32ArgmaxBindingV11};

const WAVE: &str = ENGINEERING_TP_FP32_ARGMAX32_EXPORTS_V11[0];

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
    // Recording fixtures bypass constructors; represent the retained admission here.
    driver.admitted_argmax_v11 = Some(Fp32ArgmaxBindingV11::recording());
    driver
}

#[test]
fn full_forward_argmax_v11_changes_only_last_packet_and_preserves_two_epoch_commit() {
    for selected in [Vec::new(), vec![0]] {
        let mut traces = Vec::new();
        for wave in [false, true] {
            let mut pool = pool();
            let mut driver = configured(&pool);
            let allocations = driver.inner.transports[0].buffers.clone();
            driver
                .configure_full_forward_argmax_binding_v11(Fp32ArgmaxBindingV11::recording(), wave)
                .unwrap();
            assert_eq!(driver.inner.transports[0].buffers, allocations);
            assert_eq!(driver.fp32_head_workspace_bytes(), 9_723_904);
            let mut choices = Vec::new();
            for epoch in 1..=2 {
                let batch = prepare(&mut pool, 1);
                pool.begin_submission(&batch).unwrap();
                let output = driver.execute_selected(&batch, &selected).unwrap();
                choices.push(output.choices.clone());
                assert_eq!(driver.dispatch_counts(), [616 * epoch]);
                assert_eq!(driver.inner.collective.expected().epoch, epoch);
                assert_eq!(driver.inner.collective.expected().layer, 0);
                assert!(driver.inner.full_forward.is_none());
                pool.commit_batch(&batch, output.completion).unwrap();
            }
            let transport = &driver.inner.transports[0];
            assert_eq!(transport.packet_preparations, [616, 616]);
            assert_eq!(transport.reads.len(), 2);
            let mut commands = transport.commands.clone();
            for packets in commands.chunks_exact_mut(616) {
                assert_eq!(packets[614].kernel, FP32_MFMA_HEAD);
                assert_eq!(packets[615].kernel, if wave { WAVE } else { FP32_ARGMAX });
                assert_eq!(buffer(&packets[614], 2), buffer(&packets[615], 0));
                assert_eq!(buffer(&packets[615], 0).2, 16 * 151_936);
                assert_eq!(buffer(&packets[615], 0).3, 4);
                assert_eq!(buffer(&packets[615], 1).2, 16);
                assert_eq!(packets[615].grid_workgroups, 1);
                assert_eq!(packets[615].workgroup_size, 64);
                assert_eq!(scalar(&packets[615], 2), 1);
                assert_eq!(
                    packets
                        .iter()
                        .filter(|p| p.kernel == "ferric_qwen3_tp_wave_paged_gqa_bf16_v3")
                        .count(),
                    36
                );
                packets[615].kernel = FP32_ARGMAX;
            }
            let events = transport
                .events
                .borrow()
                .iter()
                .map(|event| match event {
                    Event::Submit(rank, name) if *name == WAVE => Event::Submit(*rank, FP32_ARGMAX),
                    Event::Wait(rank, name) if *name == WAVE => Event::Wait(*rank, FP32_ARGMAX),
                    other => other.clone(),
                })
                .collect::<Vec<_>>();
            traces.push((
                commands,
                transport.reads.clone(),
                transport.write_payloads.clone(),
                transport.buffers.clone(),
                events,
                choices,
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
            "only the final argmax packet and its events may differ"
        );
    }
}

#[test]
fn full_forward_argmax_v11_failures_never_commit_either_mode() {
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
            driver
                .configure_full_forward_argmax_binding_v11(Fp32ArgmaxBindingV11::recording(), wave)
                .unwrap();
            let hidden = driver.inner.ranks[0].hidden;
            let ReductionWorkspace::DeviceTp1(scratch) = driver.inner.reduction else {
                panic!("device residual required")
            };
            let cursor = driver.inner.collective;
            let host = driver.inner.hidden.clone();
            let logits = driver.fp32_logits;
            driver.inner.transports[0].failure = Some(failure);
            let batch = prepare(&mut pool, 1);
            pool.begin_submission(&batch).unwrap();
            assert!(driver.execute(&batch).is_err(), "{failure:?}");
            assert_eq!(driver.inner.ranks[0].hidden, hidden);
            let ReductionWorkspace::DeviceTp1(after) = driver.inner.reduction else {
                panic!("device residual required")
            };
            assert_eq!(after, scratch);
            assert_eq!(driver.inner.collective, cursor);
            assert_eq!(driver.inner.hidden, host);
            assert_eq!(driver.fp32_logits, logits);
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
fn full_forward_argmax_v11_requires_exact_admission_and_profile_before_sealing() {
    for wave in [false, true] {
        for mutation in 0..34 {
            let mut driver = configured(&pool());
            let mut binding = Fp32ArgmaxBindingV11::recording();
            match mutation {
                0 => driver.admitted_argmax_v11 = None,
                1 => binding.hsaco[0] ^= 1,
                2 => driver.projection.mode = EngineeringTpProjectionModeV3::Wave,
                3 => driver.projection.mode = EngineeringTpProjectionModeV3::Auto,
                4 => driver.fp32_argmax_v11 = Some(binding),
                5 => driver.row_capacity = 32,
                6 => driver.inner.row_capacity = 32,
                7 => driver.wave_attention = false,
                8 => driver.head_profile_configured = false,
                9 => driver.fp32_logits = None,
                10 => driver.fp32_logits.as_mut().unwrap().elements -= 1,
                11 => driver.fp32_logits.as_mut().unwrap().element_bytes = 2,
                12 => driver.projection_configured = false,
                13 => driver.projection.mode = EngineeringTpProjectionModeV3::Baseline,
                14 => driver.prune_output_head = true,
                15 => driver.inner.reduction = ReductionWorkspace::default(),
                16 => driver.inner.sequences = Some(vec![Vec::new()]),
                17 => driver.inner.ordered_batches = Some(Vec::new()),
                18 => driver.inner.transports[0].full_forward_supported = false,
                19 => driver.inner.transports[0].rollover_supported = true,
                20 => driver.inner.transports[0].argmax_peer = Some((0, 1, 0)),
                21 => driver.inner.draft_v10 = true,
                22 => driver.inner.large_kv = true,
                23 => driver.last_batch = 1,
                24 => driver.completed_batches = 1,
                25 => driver.poisoned = true,
                26 => driver.inner.closed = true,
                27 => driver.context_tokens = 63,
                28 => driver.physical_pages = 5,
                29 => driver.table_stride = 5,
                30 => driver.c1_wave_layers = true,
                31 => {
                    driver.admitted_query_hoist_v14 =
                        Some(crate::tp_artifact::QueryHoistBindingV14::recording());
                }
                32 => driver.inner.transports.clear(),
                33 => driver.inner.ranks.clear(),
                _ => unreachable!(),
            }
            let active = driver.fp32_argmax_v11;
            assert!(
                driver
                    .configure_full_forward_argmax_binding_v11(binding, wave)
                    .is_err(),
                "mutation {mutation}"
            );
            assert!(!driver.inner.full_forward_enabled);
            assert_eq!(driver.fp32_argmax_v11, active);
        }
    }
}

#[test]
fn full_forward_argmax_v11_does_not_open_legacy_selectors_or_unsealed_execution() {
    let mut pool = pool();
    let mut driver = configured(&pool);
    let image = Fp32ArgmaxBindingV11::recording();
    assert!(driver.configure_mfma_v7_wave_full_forward().is_err());
    assert!(driver.configure_mfma_v7_full_forward().is_err());
    assert!(driver.configure_scalar_v3_full_forward().is_err());
    assert!(driver.configure_fp32_argmax_binding_v11(image).is_err());
    assert!(
        driver
            .configure_wave_attention_fp32_argmax_binding_v11(image)
            .is_err()
    );
    let batch = prepare(&mut pool, 1);
    pool.begin_submission(&batch).unwrap();
    assert!(driver.execute(&batch).is_err());
    assert_eq!(driver.dispatch_counts(), [0]);
    assert!(driver.inner.transports[0].commands.is_empty());
    driver
        .configure_full_forward_argmax_binding_v11(image, false)
        .unwrap();
    assert!(
        driver
            .configure_full_forward_argmax_binding_v11(image, true)
            .is_err()
    );
    assert!(driver.configure_wave_attention(false).is_err());
    assert!(driver.configure_head_precision_v7(true).is_err());
    assert!(driver.configure_output_head_pruning(true).is_err());
    assert!(
        driver
            .configure_reduction(EngineeringTpReductionModeV3::HostStagedV1)
            .is_err()
    );
}

#[test]
fn full_forward_argmax_v11_preallocation_guard_preserves_old_capacity32_helper() {
    for mutation in 0..10 {
        let mut driver = fixture(if mutation == 9 { 2 } else { 1 }, &pool());
        for transport in &mut driver.inner.transports {
            transport.buffers.clear();
            transport.argmax_v11_loaded = Some(Fp32ArgmaxBindingV11::recording().hsaco);
        }
        let mut image = Fp32ArgmaxBindingV11::recording();
        match mutation {
            0 => driver.inner.transports[0].argmax_v11_loaded = None,
            1 => image.hsaco[0] ^= 1,
            2 => driver.inner.transports[0].argmax_peer = Some((0, 1, 0)),
            3 => driver.inner.transports.clear(),
            4 => driver.inner.transports[0].full_forward_supported = false,
            5 => driver.inner.transports[0].rollover_supported = true,
            6 => {
                driver.inner.transports[0].allocate(4).unwrap();
            }
            _ => (),
        }
        let sizes = driver
            .inner
            .transports
            .iter()
            .map(|t| t.buffers.len())
            .collect::<Vec<_>>();
        assert!(
            validate_full_forward_argmax_binding_v11(
                &mut driver.inner.transports,
                if mutation == 7 { 32 } else { 16 },
                mutation == 8,
                image
            )
            .is_err()
        );
        assert_eq!(
            driver
                .inner
                .transports
                .iter()
                .map(|t| t.buffers.len())
                .collect::<Vec<_>>(),
            sizes
        );
    }
    let mut driver = fixture(1, &pool());
    let transport = &mut driver.inner.transports[0];
    transport.buffers.clear();
    transport.argmax_v11_loaded = Some(Fp32ArgmaxBindingV11::recording().hsaco);
    validate_full_forward_argmax_binding_v11(
        &mut driver.inner.transports,
        16,
        false,
        Fp32ArgmaxBindingV11::recording(),
    )
    .unwrap();
    assert!(
        validate_argmax_binding_v11(
            &mut driver.inner.transports,
            16,
            false,
            Fp32ArgmaxBindingV11::recording()
        )
        .is_err()
    );
}

#[test]
fn full_forward_argmax_v11_dispatch_has_exact_capacity16_single_row_geometry() {
    let mut pool = pool();
    let mut driver = configured(&pool);
    driver
        .configure_full_forward_argmax_binding_v11(Fp32ArgmaxBindingV11::recording(), true)
        .unwrap();
    let batch = prepare(&mut pool, 1);
    pool.begin_submission(&batch).unwrap();
    let output = driver.execute(&batch).unwrap();
    pool.commit_batch(&batch, output.completion).unwrap();
    let last = driver.inner.transports[0].commands.last().unwrap();
    for mutation in 0..11 {
        let mut command = last.clone();
        match mutation {
            0 => command.grid_workgroups = 2,
            1 => command.workgroup_size = 32,
            2 => command.arguments[2] = EngineeringTpArgumentV1::U32(0),
            3 => command.arguments[2] = EngineeringTpArgumentV1::U32(2),
            4 => {
                command.arguments.pop();
            }
            5 => command.arguments.push(EngineeringTpArgumentV1::U32(1)),
            6..=10 => {
                let EngineeringTpArgumentV1::Buffer {
                    offset,
                    elements,
                    element_bytes,
                    access,
                    ..
                } = &mut command.arguments[usize::from(mutation == 10)]
                else {
                    unreachable!()
                };
                match mutation {
                    6 => *offset = 1,
                    7 => *elements -= 1,
                    8 => *element_bytes = 2,
                    9 => *access = EngineeringTpBufferAccessV1::Write,
                    10 => *access = EngineeringTpBufferAccessV1::Read,
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        }
        assert!(
            super::super::super::row_profile::bind(16, command).is_err(),
            "mutation {mutation}"
        );
    }
    assert!(super::super::super::row_profile::bind(16, last.clone()).is_ok());
    assert!(super::super::super::row_profile::bind(8, last.clone()).is_err());
}

#[test]
fn full_forward_argmax_v11_rejects_multiple_physical_rows_without_submission() {
    for wave in [false, true] {
        let mut pool = pool();
        let mut driver = configured(&pool);
        driver
            .configure_full_forward_argmax_binding_v11(Fp32ArgmaxBindingV11::recording(), wave)
            .unwrap();
        let batch = prepare(&mut pool, 2);
        pool.begin_submission(&batch).unwrap();
        assert!(driver.execute_selected(&batch, &[0]).is_err());
        assert_eq!(driver.dispatch_counts(), [0]);
        assert!(driver.inner.transports[0].commands.is_empty());
        assert_eq!((driver.last_batch, driver.completed_batches), (0, 0));
    }
}
