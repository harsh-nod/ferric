//! Recording-only host contracts, not kernel emulation or native numerical evidence.

use super::*;
use crate::tp_artifact::{ENGINEERING_TP_FP32_ARGMAX32_EXPORTS_V11, Fp32ArgmaxBindingV11};

const WAVE: &str = ENGINEERING_TP_FP32_ARGMAX32_EXPORTS_V11[0];

fn configured(
    pool: &EngineeringTpPagedPoolV1,
    mfma: bool,
) -> EngineeringTpBatchExecutionV2<Recording> {
    let mut driver = fixture(1, pool);
    driver
        .configure_reduction(EngineeringTpReductionModeV3::DeviceTp1V3)
        .unwrap();
    driver.configure_output_head_pruning(true).unwrap();
    if mfma {
        let original = driver.inner.ranks[0].global(Qwen3TensorKind::LanguageModelHead);
        let transposed = allocate_tensor(&mut driver.inner.transports[0], 1, 2).unwrap();
        driver.projection =
            super::super::super::projection::ProjectionPolicy::synthetic_mfma_for_recording(
                original.id,
                transposed,
            );
    }
    driver.configure_head_precision_v8(true).unwrap();
    // The synthetic fixture bypasses constructors; represent the already checked image only here.
    driver.admitted_argmax_v11 = Some(Fp32ArgmaxBindingV11::recording());
    driver
}

#[test]
fn v11_selector_changes_only_argmax_for_full_sparse_and_empty_selected_rows() {
    for rows in [1, 16, 17, 32] {
        for mfma in [false, true] {
            for selected in [
                Vec::new(),
                (0..rows as usize).collect(),
                vec![rows as usize - 1],
            ] {
                let mut traces = Vec::new();
                for wave in [false, true] {
                    let mut pool = wide_pool();
                    let mut driver = configured(&pool, mfma);
                    let buffers = driver.inner.transports[0].buffers.len();
                    if wave {
                        driver
                            .configure_fp32_argmax_binding_v11(Fp32ArgmaxBindingV11::recording())
                            .unwrap();
                    }
                    assert_eq!(driver.inner.transports[0].buffers.len(), buffers);
                    let batch = prepare(&mut pool, rows);
                    pool.begin_submission(&batch).unwrap();
                    let output = driver.execute_selected(&batch, &selected).unwrap();
                    assert_eq!(output.choices.len(), selected.len());
                    let expected = if selected.is_empty() { 613 } else { 616 };
                    assert_eq!(driver.dispatch_counts(), [expected]);
                    pool.commit_batch(&batch, output.completion).unwrap();
                    let mut commands = driver.inner.transports[0].commands.clone();
                    if let Some(last) = commands.last_mut().filter(|_| !selected.is_empty()) {
                        assert_eq!(
                            last.kernel,
                            if wave {
                                WAVE
                            } else {
                                "ferric_qwen3_tp_batch32_argmax_f32_v8"
                            }
                        );
                        assert_eq!(last.grid_workgroups, u32::try_from(selected.len()).unwrap());
                        assert_eq!(last.workgroup_size, 64);
                        assert_eq!(buffer(last, 0).3, 4);
                        last.kernel = "ferric_qwen3_tp_batch32_argmax_f32_v8";
                    }
                    traces.push(commands);
                }
                assert_eq!(traces[0], traces[1]);
            }
        }
    }
}

#[test]
fn v11_missing_binding_unsupported_and_late_selection_fail_without_dispatch() {
    for mutation in 0..14 {
        let pool = wide_pool();
        let mut driver = configured(&pool, false);
        match mutation {
            0 => driver.admitted_argmax_v11 = None,
            1 => driver.row_capacity = 16,
            2 => driver.inner.draft_v10 = true,
            3 => driver.fp32_logits = None,
            4 => driver.head_profile_configured = false,
            5 => driver.inner.large_kv = true,
            6 => driver.inner.sequences = Some(vec![Vec::new()]),
            7 => driver.inner.ordered_batches = Some(Vec::new()),
            8 => driver.wave_attention = true,
            9 => driver.last_batch = 1,
            10 => driver.completed_batches = 1,
            11 => driver.poisoned = true,
            12 => driver.inner.closed = true,
            13 => {
                let mut other = Fp32ArgmaxBindingV11::recording();
                other.hsaco[0] ^= 1;
                driver.admitted_argmax_v11 = Some(other);
            }
            _ => unreachable!(),
        }
        assert!(
            driver
                .configure_fp32_argmax_binding_v11(Fp32ArgmaxBindingV11::recording())
                .is_err()
        );
        assert_eq!(driver.dispatch_counts(), [0]);
        assert!(driver.fp32_argmax_v11.is_none());
    }
    let mut driver = configured(&wide_pool(), false);
    driver
        .configure_fp32_argmax_binding_v11(Fp32ArgmaxBindingV11::recording())
        .unwrap();
    assert!(
        driver
            .configure_fp32_argmax_binding_v11(Fp32ArgmaxBindingV11::recording())
            .is_err()
    );
    assert!(
        driver
            .configure_reduction(EngineeringTpReductionModeV3::DeviceTp1V3)
            .is_err()
    );
    assert!(driver.configure_output_head_pruning(false).is_err());
    assert!(driver.configure_wave_attention(true).is_err());
    assert!(driver.configure_dispatch_sequences(true).is_err());
    assert!(driver.configure_ordered_batches(true).is_err());
    assert!(driver.configure_head_precision_v8(true).is_err());
}

#[test]
fn v11_recording_enforces_the_real_workers_preallocation_image_guard() {
    let mut driver = fixture(1, &wide_pool());
    let transport = &mut driver.inner.transports[0];
    transport.argmax_v11_loaded = Some(Fp32ArgmaxBindingV11::recording().hsaco);
    let image = Fp32ArgmaxBindingV11::recording();
    assert!(
        transport
            .require_loaded_image(image.hsaco, &[WAVE])
            .is_err()
    );
    transport.buffers.clear();
    transport
        .require_loaded_image(image.hsaco, &[WAVE])
        .unwrap();
    transport.allocate(4).unwrap();
    assert!(
        transport
            .require_loaded_image(image.hsaco, &[WAVE])
            .is_err()
    );
    // Selection checks only the previously retained binding, never the now-nonfresh worker.
    let mut driver = configured(&wide_pool(), false);
    driver.configure_fp32_argmax_binding_v11(image).unwrap();
}

#[test]
fn v11_actual_constructor_admission_helper_rejects_before_any_allocation() {
    for mutation in 0..7 {
        let mut driver = fixture(if mutation == 6 { 2 } else { 1 }, &wide_pool());
        for transport in &mut driver.inner.transports {
            transport.buffers.clear();
            transport.argmax_v11_loaded = Some(Fp32ArgmaxBindingV11::recording().hsaco);
        }
        let mut binding = Fp32ArgmaxBindingV11::recording();
        match mutation {
            0 => driver.inner.transports[0].argmax_v11_loaded = None,
            1 => binding.hsaco[0] ^= 1,
            2 => driver.inner.transports[0].argmax_peer = Some((1, 0, 1)),
            3 => {
                driver.inner.transports.clear();
            }
            _ => (),
        }
        let rows = if mutation == 4 { 16 } else { 32 };
        let result =
            validate_argmax_binding_v11(&mut driver.inner.transports, rows, mutation == 5, binding);
        assert!(result.is_err());
        assert!(
            driver
                .inner
                .transports
                .iter()
                .all(|transport| transport.buffers.is_empty())
        );
    }
    let mut driver = fixture(1, &wide_pool());
    driver.inner.transports[0].buffers.clear();
    driver.inner.transports[0].argmax_v11_loaded = Some(Fp32ArgmaxBindingV11::recording().hsaco);
    validate_argmax_binding_v11(
        &mut driver.inner.transports,
        32,
        false,
        Fp32ArgmaxBindingV11::recording(),
    )
    .unwrap();
    assert!(driver.inner.transports[0].buffers.is_empty());
    driver.inner.transports[0].allocate(4).unwrap();
    assert!(
        validate_argmax_binding_v11(
            &mut driver.inner.transports,
            32,
            false,
            Fp32ArgmaxBindingV11::recording()
        )
        .is_err()
    );
}

#[test]
fn v11_submit_wait_and_invalid_choice_never_mint_a_successful_completion() {
    for failure in [
        Failure::ArgmaxSubmit,
        Failure::ArgmaxWait,
        Failure::BadChoice,
    ] {
        let mut pool = wide_pool();
        let mut driver = configured(&pool, false);
        driver
            .configure_fp32_argmax_binding_v11(Fp32ArgmaxBindingV11::recording())
            .unwrap();
        driver.inner.transports[0].failure = Some(failure);
        let batch = prepare(&mut pool, 1);
        pool.begin_submission(&batch).unwrap();
        assert!(driver.execute_selected(&batch, &[0]).is_err());
        assert!(driver.poisoned);
        assert_eq!(driver.completed_batches(), 0);
        pool.quarantine_batch(&batch).unwrap();
        assert!(driver.execute_selected(&batch, &[0]).is_err());
        driver.close().unwrap();
    }
}

#[test]
fn v11_nested_host_scopes_preserve_commands_and_skip_pruned_prefill() {
    for selected in [Vec::new(), vec![0]] {
        let mut traces = Vec::new();
        for enabled in [false, true] {
            let mut pool = wide_pool();
            let mut driver = configured(&pool, false);
            driver
                .configure_fp32_argmax_binding_v11(Fp32ArgmaxBindingV11::recording())
                .unwrap();
            let timing = if enabled {
                crate::host_timing::HostTiming::enabled()
            } else {
                crate::host_timing::HostTiming::default()
            };
            driver.configure_host_timing(timing.clone()).unwrap();
            let batch = prepare(&mut pool, 1);
            pool.begin_submission(&batch).unwrap();
            let output = driver.execute_selected(&batch, &selected).unwrap();
            pool.commit_batch(&batch, output.completion).unwrap();
            traces.push(driver.inner.transports[0].commands.clone());
            if enabled {
                let snapshot = timing.snapshot();
                assert_eq!(snapshot["incomplete"], false);
                assert_eq!(snapshot["active_records"], 0);
                assert_attention_operation_spans(&snapshot, batch.id(), 36);
                for label in [
                    "output_head",
                    "output_head_normalization",
                    "output_head_projection",
                    "output_head_argmax",
                ] {
                    let records = snapshot["records"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter(|row| row["label"] == label)
                        .collect::<Vec<_>>();
                    assert_eq!(records.len(), usize::from(!selected.is_empty()));
                    if !records.is_empty() {
                        assert_eq!(records[0]["count"], 1);
                    }
                }
            }
        }
        assert_eq!(traces[0], traces[1]);
    }
}
