use super::super::tests::Temp;
use super::*;

fn config() -> ProjectionDecodeConfig {
    ProjectionDecodeConfig {
        schema: REQUEST_SCHEMA.into(),
        decode: super::super::tests::config(),
        projection_residual_image: FilePin {
            path: "/task/projection.hsaco".into(),
            bytes: 1,
            sha256: hash(&[10]),
        },
    }
}

fn completed(
    path: &std::path::Path,
    selected: bool,
) -> (
    ProjectionDecodeConfig,
    Observation,
    projection_wire::Bootstrap,
) {
    completed_for(path, selected, wire::InputMode::TeacherForced)
}

fn completed_for(
    path: &std::path::Path,
    selected: bool,
    mode: wire::InputMode,
) -> (
    ProjectionDecodeConfig,
    Observation,
    projection_wire::Bootstrap,
) {
    let mut config = config();
    config.decode.evidence_directory = path.into();
    config.decode.mode = mode;
    let b = wire::tests::bootstrap(mode);
    let wrapped = bootstrap(b.clone(), &config.projection_residual_image).unwrap();
    let profile = if selected {
        wrapped.sha256().unwrap()
    } else {
        b.sha256().unwrap()
    };
    let mut evidence = evidence::Evidence::create(path).unwrap();
    let mut chain = wire::Chain::new(b.registration, profile);
    let mut checked = wire::Chain::new(b.registration, profile);
    let mut previous = None;
    let mut inputs = Vec::new();
    for position in 0..4 {
        inputs.push(b.input(position, previous).unwrap());
        let mut request = wire::tests::request(&b, position, previous);
        request.profile_sha256 = profile;
        let (response, control, payload) =
            wire::tests::completed(&request, &mut chain, position + 7);
        validate_completion(&request, &response, &control, &payload, &mut checked).unwrap();
        evidence
            .append(&request, &response, &control, &payload)
            .unwrap();
        previous = Some(position + 7);
    }
    let mut close = wire::tests::close(&b, chain.digest()).1;
    close.profile_sha256 = profile;
    let actual = Observation {
        schema: "FerricFinitePrefixDecodeObservationV1",
        request: config.decode.clone(),
        child_pid: b.scope.child_identity,
        registration_sha256: b.registration,
        source_program_sha256: b.begin.source_program.sha256,
        upload_manifest_sha256: b.begin.uploads.sha256,
        bootstrap: b,
        profile_sha256: profile,
        setup_commands: 1,
        completed_forwards: 4,
        input_tokens: inputs,
        observed_output_tokens: vec![7, 8, 9, 10],
        page_permutation: (0..144).collect(),
        transcript_sha256: chain.digest(),
        request_stream_bytes: 1,
        response_stream_bytes: 3_723_444,
        files: evidence.finish(&[]).unwrap(),
        close,
        child_exit_zero: true,
        process_group_absent: true,
        native_closed: true,
        gpu_execution: true,
        numerical_acceptance: false,
        performance_claim: false,
        production_authority: false,
        full_long_workload: false,
    };
    (config, actual, wrapped)
}

#[test]
fn projection_decode_parent_config_is_separate_and_closed() {
    let c = config();
    let bytes = serde_json::to_vec(&c).unwrap();
    ProjectionDecodeConfig::parse(&bytes).unwrap();
    assert!(Config::parse(&bytes).is_err());
    assert!(ProjectionDecodeConfig::parse(&serde_json::to_vec(&c.decode).unwrap()).is_err());
    for key in ["fallback", "host_policy", "device_sidecar", "forwards"] {
        let mut value = serde_json::to_value(&c).unwrap();
        value[key] = true.into();
        assert!(ProjectionDecodeConfig::parse(&serde_json::to_vec(&value).unwrap()).is_err());
    }
    let mut ar = c;
    ar.decode.mode = wire::InputMode::Autoregressive;
    assert!(ar.validate().is_ok());
    assert!(ar.decode.validate().is_ok());
}

#[test]
fn projection_ar4_parent_publishes_actual_own_output_trajectory_without_parity() {
    let temp = Temp::new();
    let path = temp.0.join("ar");
    let (config, actual, bootstrap) = completed_for(&path, true, wire::InputMode::Autoregressive);
    assert_eq!(actual.input_tokens, [9112, 7, 8, 9]);
    assert_eq!(bootstrap.decode.input_tokens, [9112]);
    let mut observation = ProjectionDecodeObservation::closed(config, actual, bootstrap).unwrap();
    observation.publish().unwrap();
    let value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(path.join("complete.json")).unwrap()).unwrap();
    assert_eq!(
        value["schema"],
        "FerricFiniteProjectionResidualDecodeObservationV1"
    );
    assert_eq!(
        observation.request.decode.mode,
        wire::InputMode::Autoregressive
    );
    assert_eq!(observation.observed_output_tokens, [7, 8, 9, 10]);
    assert_eq!(observation.files.frames.len(), 4);
    assert!(
        observation.native_closed
            && observation.child_exit_zero
            && observation.process_group_absent
    );
    assert!(!observation.paired_comparison_performed && !observation.numerical_acceptance);
    assert_eq!(std::fs::read_dir(path).unwrap().count(), 14);
}

#[test]
fn projection_ar4_parent_refuses_wrong_seed_previous_output_or_mode_before_summary() {
    for change in 0..8 {
        let temp = Temp::new();
        let path = temp.0.join("ar");
        let (mut config, mut actual, mut bootstrap) =
            completed_for(&path, true, wire::InputMode::Autoregressive);
        match change {
            0..=3 => actual.input_tokens[change] ^= 1,
            4 => actual.observed_output_tokens[1] ^= 1,
            5 => actual.observed_output_tokens[3] = VOCABULARY,
            6 => config.decode.mode = wire::InputMode::TeacherForced,
            _ => bootstrap.projection_residual_image.sha256[0] ^= 1,
        }
        assert!(ProjectionDecodeObservation::closed(config, actual, bootstrap).is_err());
        assert!(!path.join("complete.json").exists());
    }
}

#[test]
fn projection_ar4_parent_retains_payload_control_close_and_reap_refusals() {
    let b = wire::tests::bootstrap(wire::InputMode::Autoregressive);
    let profile = bootstrap(b.clone(), &config().projection_residual_image)
        .unwrap()
        .sha256()
        .unwrap();
    let mut request = wire::tests::request(&b, 3, Some(9));
    request.profile_sha256 = profile;
    let (response, control, payload) =
        wire::tests::completed(&request, &mut wire::Chain::new(b.registration, profile), 10);
    for change in 0..3 {
        let (mut reply, mut control, mut payload) =
            (response.clone(), control.clone(), payload.clone());
        match change {
            0 => reply.profile_sha256 = b.sha256().unwrap(),
            1 => control.layers[35].tiles_states[1][547] = 0,
            _ => {
                payload.pop();
            }
        }
        assert!(
            validate_completion(
                &request,
                &reply,
                &control,
                &payload,
                &mut wire::Chain::new(b.registration, profile)
            )
            .is_err()
        );
    }
    for change in 0..3 {
        let temp = Temp::new();
        let path = temp.0.join("ar");
        let (config, mut actual, bootstrap) =
            completed_for(&path, true, wire::InputMode::Autoregressive);
        match change {
            0 => actual.child_exit_zero = false,
            1 => actual.process_group_absent = false,
            _ => actual.close.native_closed = false,
        }
        assert!(ProjectionDecodeObservation::closed(config, actual, bootstrap).is_err());
        assert!(!path.join("complete.json").exists());
    }
}

#[test]
fn projection_decode_parent_rejects_invalid_or_original_copy_image() {
    for change in 0..6 {
        let mut c = config();
        match change {
            0 => c.projection_residual_image.bytes = 0,
            1 => c.projection_residual_image.bytes = wire::MAX_IMAGE_BYTES as u64 + 1,
            2 => c.projection_residual_image.sha256 = [0; 32],
            3 => c.projection_residual_image.path = "relative".into(),
            4 => c.projection_residual_image.sha256 = c.decode.images.residual.sha256,
            _ => c.schema = "FerricFinitePrefixDecodeRequestV1".into(),
        }
        assert!(c.validate().is_err());
    }
}

#[test]
fn projection_decode_parent_image_custody_refuses_changed_bytes_and_extent() {
    let temp = Temp::new();
    let mut c = config();
    c.projection_residual_image.path = temp.0.join("candidate");
    std::fs::write(&c.projection_residual_image.path, [10]).unwrap();
    assert_eq!(
        c.projection_residual_image.read(32 << 20, true).unwrap(),
        [10]
    );
    std::fs::write(&c.projection_residual_image.path, [11]).unwrap();
    assert!(c.projection_residual_image.read(32 << 20, false).is_err());
    std::fs::write(&c.projection_residual_image.path, [10, 10]).unwrap();
    assert!(c.projection_residual_image.read(32 << 20, false).is_err());
}

#[test]
fn projection_decode_parent_bootstrap_transports_extra_image_and_binds_profile() {
    let b = wire::tests::bootstrap(wire::InputMode::TeacherForced);
    let selected = bootstrap(b.clone(), &config().projection_residual_image).unwrap();
    assert_ne!(selected.sha256().unwrap(), b.sha256().unwrap());
    assert_eq!(selected.decode.begin.residual_image, b.begin.residual_image);
    let mut bytes = Vec::new();
    projection_wire::write_bootstrap(
        &mut bytes,
        &mut wire::FrameBudget::new(),
        &selected,
        &[8],
        &[9],
        &[10],
    )
    .unwrap();
    let (seen, mlp, prefix, candidate) =
        projection_wire::read_bootstrap(&mut &bytes[..], &mut wire::FrameBudget::new())
            .unwrap()
            .unwrap();
    assert_eq!(seen, selected);
    assert_eq!((mlp, prefix, candidate), (vec![8], vec![9], vec![10]));
    assert!(wire::read_bootstrap(&mut &bytes[..], &mut wire::FrameBudget::new()).is_err());
    let mut changed = selected;
    changed.projection_residual_image.sha256[0] ^= 1;
    assert_ne!(seen.sha256().unwrap(), changed.sha256().unwrap());
}

#[test]
fn projection_decode_parent_selector_isolates_all_legacy_diagnostics() {
    assert_eq!(
        worker_flag(None, false).unwrap(),
        "--engineering-native-prefix-decode-v1"
    );
    assert_eq!(
        worker_flag(None, true).unwrap(),
        "--engineering-native-projection-residual-decode-v1"
    );
    let path = PathBuf::from("/task/sidecar");
    for diagnostic in [
        HostDiagnostic::V1(path.clone()),
        HostDiagnostic::Device(path.clone()),
        HostDiagnostic::DeviceClock(path.clone()),
        HostDiagnostic::V2(
            path,
            crate::prefix_decode_host_observation_v2::Policy::Baseline,
        ),
    ] {
        assert_eq!(
            worker_flag(Some(&diagnostic), false).unwrap(),
            diagnostic.launch_flag()
        );
        assert!(worker_flag(Some(&diagnostic), true).is_err());
    }
}

#[test]
fn projection_decode_parent_optin_refuses_before_files_or_child() {
    assert!(
        run_projection_decode(config(), false)
            .err()
            .unwrap()
            .contains("opt-in")
    );
}

#[test]
fn projection_decode_parent_new_profile_preserves_payload_control_and_close_refusals() {
    let b = wire::tests::bootstrap(wire::InputMode::TeacherForced);
    let profile = bootstrap(b.clone(), &config().projection_residual_image)
        .unwrap()
        .sha256()
        .unwrap();
    let mut request = wire::tests::request(&b, 0, None);
    request.profile_sha256 = profile;
    let (response, control, payload) =
        wire::tests::completed(&request, &mut wire::Chain::new(b.registration, profile), 7);
    for change in 0..4 {
        let (mut s, mut c, mut raw) = (response.clone(), control.clone(), payload.clone());
        match change {
            0 => s.profile_sha256 = b.sha256().unwrap(),
            1 => c.layers[35].tiles_states[1][547] = 0,
            2 => {
                raw.pop();
            }
            _ => {
                if let wire::Event::Completed(v) = &mut s.event {
                    v.output_token = 8;
                }
            }
        }
        assert!(
            validate_completion(
                &request,
                &s,
                &c,
                &raw,
                &mut wire::Chain::new(b.registration, profile)
            )
            .is_err()
        );
    }
    let (mut request, mut response) = wire::tests::close(&b, [8; 32]);
    request.profile_sha256 = profile;
    response.profile_sha256 = profile;
    validate_close(&request, &response, None, &[], [8; 32]).unwrap();
    response.profile_sha256 = b.sha256().unwrap();
    assert!(validate_close(&request, &response, None, &[], [8; 32]).is_err());
}

#[test]
fn projection_decode_parent_publishes_only_new_closed_schema_and_exact_budget() {
    let temp = Temp::new();
    let path = temp.0.join("native");
    let (config, actual, bootstrap) = completed(&path, true);
    assert!(!path.join("complete.json").exists());
    let mut observation = ProjectionDecodeObservation::closed(config, actual, bootstrap).unwrap();
    observation.publish().unwrap();
    let bytes = std::fs::read(path.join("complete.json")).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        value["schema"],
        "FerricFiniteProjectionResidualDecodeObservationV1"
    );
    assert_eq!(
        value["bootstrap"]["schema"],
        "FerricProjectionResidualDecodeBootstrapV1"
    );
    assert_eq!(value["completed_forwards"], 4);
    assert_eq!(observation.files.frames.len(), 4);
    assert_eq!(bytes.len() as u64, observation.files.summary_bytes);
    assert_eq!(std::fs::read_dir(&path).unwrap().count(), 14);
    let total: u64 = std::fs::read_dir(&path)
        .unwrap()
        .map(|row| row.unwrap().metadata().unwrap().len())
        .sum();
    assert_eq!(total, observation.files.total_bytes);
    assert!(total < 8 << 20);
    for flag in [
        "paired_comparison_performed",
        "numerical_acceptance",
        "performance_claim",
        "production_authority",
        "full_long_workload",
    ] {
        assert_eq!(value[flag], false);
    }
    assert!(value.get("bitwise_equal").is_none());
}

#[test]
fn projection_decode_parent_refuses_incomplete_reap_or_changed_final_evidence() {
    for change in 0..9 {
        let temp = Temp::new();
        let path = temp.0.join("native");
        let (config, mut actual, mut bootstrap) = completed(&path, true);
        match change {
            0 => actual.native_closed = false,
            1 => actual.child_exit_zero = false,
            2 => actual.process_group_absent = false,
            3 => actual.profile_sha256 = actual.bootstrap.sha256().unwrap(),
            4 => bootstrap.projection_residual_image.sha256[0] ^= 1,
            5 => actual.completed_forwards = 3,
            6 => actual.close.native_closed = false,
            7 => {
                std::fs::write(&actual.files.frames[3].observation.path, [0]).unwrap();
            }
            _ => {
                std::fs::write(path.join("unexpected"), [0]).unwrap();
            }
        }
        assert!(ProjectionDecodeObservation::closed(config, actual, bootstrap).is_err());
        assert!(!path.join("complete.json").exists());
    }
}

#[test]
fn projection_decode_parent_old_summary_serialization_is_byte_unchanged() {
    let temp = Temp::new();
    let path = temp.0.join("old");
    let (_, mut old, _) = completed(&path, false);
    let mut expected = Vec::new();
    for _ in 0..8 {
        expected = serde_json::to_vec(&old).unwrap();
        expected.push(b'\n');
        if old.files.summary_bytes == expected.len() as u64 {
            break;
        }
        old.files.summary_bytes = expected.len() as u64;
        old.files.total_bytes = old.files.bytes_before_summary + expected.len() as u64;
    }
    old.files.summary_bytes = 0;
    old.files.total_bytes = old.files.bytes_before_summary;
    evidence::publish(&mut old).unwrap();
    assert_eq!(std::fs::read(path.join("complete.json")).unwrap(), expected);
}

fn host_fixture(
    actual: &Observation,
    wrapped: &projection_wire::Bootstrap,
) -> crate::projection_residual_decode_host_observation_v1::Report {
    let mut report = crate::projection_residual_decode_host_observation_v1::tests::report();
    report.bootstrap = wrapped.clone();
    report.worker_sha256 = actual.request.worker.sha256;
    report.child_pid = actual.child_pid;
    report.profile_sha256 = actual.profile_sha256;
    report.completions = actual
        .files
        .frames
        .iter()
        .map(|f| match &f.response.event {
            wire::Event::Completed(c) => c.clone(),
            _ => panic!("fixture completion"),
        })
        .collect();
    report.transcript_sha256 = actual.transcript_sha256;
    report
}
#[test]
fn projection_host_parent_joins_actual_selected_frames_after_close() {
    let temp = Temp::new();
    let path = temp.0.join("ar");
    let (config, actual, wrapped) = completed_for(&path, true, wire::InputMode::Autoregressive);
    let report = host_fixture(&actual, &wrapped);
    let sidecar = temp.0.join("sidecar.json");
    std::fs::write(&sidecar, serde_json::to_vec(&report).unwrap()).unwrap();
    let pin = super::super::projection_host::validate(
        &sidecar,
        &actual,
        &config.projection_residual_image,
    )
    .unwrap();
    assert_eq!(pin.path, sidecar);
    assert_eq!(report.serialization_host_ns, [10; 4]);
    assert!(!path.join("complete.json").exists());
}
#[test]
fn projection_host_parent_rejects_worker_image_profile_or_frame_substitution() {
    for change in 0..5 {
        let temp = Temp::new();
        let path = temp.0.join("ar");
        let (mut config, mut actual, wrapped) =
            completed_for(&path, true, wire::InputMode::Autoregressive);
        let mut report = host_fixture(&actual, &wrapped);
        match change {
            0 => report.worker_sha256[0] ^= 1,
            1 => config.projection_residual_image.sha256[0] ^= 1,
            2 => actual.profile_sha256[0] ^= 1,
            3 => actual.transcript_sha256[0] ^= 1,
            _ => actual.files.frames[0].observation.sha256[0] ^= 1,
        }
        let sidecar = temp.0.join("sidecar.json");
        std::fs::write(&sidecar, serde_json::to_vec(&report).unwrap()).unwrap();
        assert!(
            super::super::projection_host::validate(
                &sidecar,
                &actual,
                &config.projection_residual_image
            )
            .is_err()
        );
    }
}
#[test]
fn projection_host_parent_refuses_early_close_reap_and_non_ar() {
    for change in 0..4 {
        let temp = Temp::new();
        let path = temp.0.join("ar");
        let (config, mut actual, wrapped) =
            completed_for(&path, true, wire::InputMode::Autoregressive);
        let report = host_fixture(&actual, &wrapped);
        match change {
            0 => actual.native_closed = false,
            1 => actual.child_exit_zero = false,
            2 => actual.process_group_absent = false,
            _ => actual.request.mode = wire::InputMode::TeacherForced,
        }
        let sidecar = temp.0.join("sidecar.json");
        std::fs::write(&sidecar, serde_json::to_vec(&report).unwrap()).unwrap();
        assert!(
            super::super::projection_host::validate(
                &sidecar,
                &actual,
                &config.projection_residual_image
            )
            .is_err()
        );
    }
}
#[test]
fn projection_host_parent_selector_isolated_from_plain_and_legacy_modes() {
    let c = config();
    let path = std::path::PathBuf::from("/tmp/host.json");
    let new = HostDiagnostic::Projection(path.clone(), c.projection_residual_image);
    assert_eq!(
        worker_flag(Some(&new), true).unwrap(),
        "--engineering-native-projection-residual-decode-host-v1"
    );
    assert!(worker_flag(Some(&new), false).is_err());
    assert_eq!(
        worker_flag(None, true).unwrap(),
        "--engineering-native-projection-residual-decode-v1"
    );
    let old = HostDiagnostic::V1(path);
    assert!(worker_flag(Some(&old), true).is_err());
    assert_eq!(
        worker_flag(Some(&old), false).unwrap(),
        "--engineering-native-prefix-decode-host-v1"
    );
}

#[path = "projection_shared_host_tests.rs"]
mod shared_host;
