//! Synthetic files only; no child, native opener or GPU is invoked.
use super::super::{
    evidence,
    tests::{Temp, config},
};
use super::*;
use crate::prefix_decode_device_observation_v1::tests::report;

fn bind(pin: &mut FilePin, part: Part) {
    pin.bytes = u64::from(part.bytes);
    pin.sha256 = part.sha256;
}
pub(crate) fn fixture(path: &Path, mode: wire::InputMode) -> (Observation, data::Report) {
    let report = report(mode);
    let b = &report.bootstrap;
    let mut evidence = evidence::Evidence::create(path).unwrap();
    let mut chain = wire::Chain::new(b.registration, b.sha256().unwrap());
    let mut previous = None;
    let mut inputs = Vec::new();
    for position in 0..4 {
        inputs.push(b.input(position, previous).unwrap());
        let request = wire::tests::request(b, position, previous);
        let (reply, control, payload) =
            wire::tests::completed(&request, &mut chain, 100 + position);
        evidence
            .append(&request, &reply, &control, &payload)
            .unwrap();
        previous = Some(100 + position);
    }
    let files = evidence.finish(&[]).unwrap();
    let mut request = config();
    request.evidence_directory = path.into();
    request.mode = mode;
    request.worker.sha256 = report.worker_sha256;
    request.expected_bundle_id = b.scope.bundle_id;
    request.expected_model_id = b.scope.model_id;
    request.session = b.scope.session;
    request.device_ids = b.device_ids;
    request.dispatch_timeout_ms = b.timeout_ms;
    bind(&mut request.prefix_image, b.prefix_image);
    bind(&mut request.tiles_image, b.tiles_image);
    bind(&mut request.images.prefix, b.begin.prefix_image);
    bind(&mut request.images.mlp, b.begin.mlp_image);
    bind(&mut request.images.residual, b.begin.residual_image);
    bind(&mut request.images.tail, b.begin.tail_image.unwrap());
    let observation = Observation {
        schema: "FerricFinitePrefixDecodeObservationV1",
        request,
        child_pid: b.scope.child_identity,
        registration_sha256: b.registration,
        source_program_sha256: b.begin.source_program.sha256,
        upload_manifest_sha256: b.begin.uploads.sha256,
        bootstrap: b.clone(),
        profile_sha256: b.sha256().unwrap(),
        setup_commands: 1,
        completed_forwards: 4,
        input_tokens: inputs,
        observed_output_tokens: vec![100, 101, 102, 103],
        page_permutation: (0..144).collect(),
        transcript_sha256: chain.digest(),
        request_stream_bytes: 1,
        response_stream_bytes: 3723444,
        files,
        close: wire::tests::close(b, chain.digest()).1,
        child_exit_zero: true,
        process_group_absent: true,
        native_closed: true,
        gpu_execution: true,
        numerical_acceptance: false,
        performance_claim: false,
        production_authority: false,
        full_long_workload: false,
    };
    (observation, report)
}
fn write_report(path: &Path, report: &data::Report) {
    std::fs::write(path, serde_json::to_vec(report).unwrap()).unwrap();
}

#[test]
fn prefix_device_parent_retained_tf_ar_controls_and1172_rows_validate_before_summary() {
    for mode in [
        wire::InputMode::TeacherForced,
        wire::InputMode::Autoregressive,
    ] {
        let t = Temp::new();
        let (mut observation, report) = fixture(&t.0.join("native"), mode);
        let path = sidecar_path(&observation.request).unwrap();
        preflight(&path).unwrap();
        write_report(&path, &report);
        let pin = validate(&path, &observation).unwrap();
        assert!(pin.bytes > 65536 && pin.bytes <= data::MAX_BYTES as u64);
        assert_eq!(report.rows.len(), 1172);
        assert!(
            !observation
                .request
                .evidence_directory
                .join("complete.json")
                .exists()
        );
        evidence::publish(&mut observation).unwrap();
        assert_eq!(
            std::fs::read_dir(&observation.request.evidence_directory)
                .unwrap()
                .count(),
            14
        );
        assert!(observation.files.total_bytes + pin.bytes <= 8 << 20);
    }
}

#[test]
fn prefix_device_parent_refuses_unclosed_unreaped_wrong_worker_pid_profile_or_scope() {
    for change in 0..14 {
        let t = Temp::new();
        let (mut o, report) = fixture(&t.0.join("native"), wire::InputMode::TeacherForced);
        let path = sidecar_path(&o.request).unwrap();
        write_report(&path, &report);
        validate(&path, &o).unwrap();
        match change {
            0 => o.child_exit_zero = false,
            1 => o.process_group_absent = false,
            2 => o.native_closed = false,
            3 => o.completed_forwards = 3,
            4 => o.request.worker.sha256[0] ^= 1,
            5 => o.child_pid += 1,
            6 => o.profile_sha256[0] ^= 1,
            7 => o.transcript_sha256[0] ^= 1,
            8 => o.request.expected_model_id[0] ^= 1,
            9 => o.request.expected_bundle_id[0] ^= 1,
            10 => o.request.session[0] ^= 1,
            11 => o.source_program_sha256[0] ^= 1,
            12 => o.upload_manifest_sha256[0] ^= 1,
            _ => o.registration_sha256[0] ^= 1,
        }
        assert!(validate(&path, &o).is_err(), "{change}");
        assert!(!o.request.evidence_directory.join("complete.json").exists());
    }
}

#[test]
fn prefix_device_parent_binds_all_six_original_and_selected_image_parts_without_truncation() {
    let t = Temp::new();
    let (mut o, report) = fixture(&t.0.join("native"), wire::InputMode::TeacherForced);
    let path = sidecar_path(&o.request).unwrap();
    write_report(&path, &report);
    let original = o.request.clone();
    for image in 0..6 {
        for extent in [false, true] {
            o.request = original.clone();
            let pin = match image {
                0 => &mut o.request.prefix_image,
                1 => &mut o.request.tiles_image,
                2 => &mut o.request.images.prefix,
                3 => &mut o.request.images.mlp,
                4 => &mut o.request.images.residual,
                _ => &mut o.request.images.tail,
            };
            if extent {
                pin.bytes += 1_u64 << 32;
            } else {
                pin.sha256[0] ^= 1;
            }
            assert!(validate(&path, &o).is_err(), "{image}/{extent}");
        }
    }
}

#[test]
fn prefix_device_parent_checks_actual_control_for_every_forward_and_stage() {
    let t = Temp::new();
    let (o, report) = fixture(&t.0.join("native"), wire::InputMode::TeacherForced);
    let path = sidecar_path(&o.request).unwrap();
    for position in 0..4 {
        for slot in [0, 1, 2, 3, 4, 6, 8, 289, 290, 291, 292] {
            let mut changed = report.clone();
            changed.rows[position * data::PER_FORWARD + slot].host_elapsed_ns ^= 1;
            // Structurally valid raw rows must still agree with the actual Control.
            changed.validate().unwrap();
            write_report(&path, &changed);
            assert!(validate(&path, &o).is_err(), "{position}/{slot}");
        }
    }
    write_report(&path, &report);
    validate(&path, &o).unwrap();
}

#[test]
fn prefix_device_parent_refuses_modified_capture_control_or_frame_completion() {
    for change in 0..5 {
        let t = Temp::new();
        let (mut o, report) = fixture(&t.0.join("native"), wire::InputMode::TeacherForced);
        let path = sidecar_path(&o.request).unwrap();
        write_report(&path, &report);
        match change {
            0 => std::fs::write(&o.files.frames[3].control.path, [0]).unwrap(),
            1 => std::fs::write(&o.files.frames[2].observation.path, [0]).unwrap(),
            2 => o.files.frames[0].control.sha256[0] ^= 1,
            3 => o.files.frames[1].observation.sha256[0] ^= 1,
            _ => {
                if let wire::Event::Completed(c) = &mut o.files.frames[0].response.event {
                    c.output_token += 1;
                }
            }
        }
        assert!(validate(&path, &o).is_err());
        assert!(!o.request.evidence_directory.join("complete.json").exists());
    }
}

#[test]
fn prefix_device_parent_refuses_tail_copy_substitution_and_false_claims() {
    let t = Temp::new();
    let (o, report) = fixture(&t.0.join("native"), wire::InputMode::TeacherForced);
    let path = sidecar_path(&o.request).unwrap();
    for change in 0..7 {
        let mut changed = report.clone();
        match change {
            0 => changed.images.tail[0] ^= 1,
            1 => changed.images.copy[0] ^= 1,
            2 => changed.calibrated_nanoseconds = true,
            3 => changed.overlap_claim = true,
            4 => changed.numerical_acceptance = true,
            5 => changed.performance_claim = true,
            _ => changed.production_authority = true,
        }
        write_report(&path, &changed);
        assert!(validate(&path, &o).is_err());
    }
}

#[test]
fn prefix_device_parent_requires_absent_bounded_regular_sidecar_and_optin() {
    let t = Temp::new();
    let (mut o, report) = fixture(&t.0.join("native"), wire::InputMode::TeacherForced);
    let path = sidecar_path(&o.request).unwrap();
    assert_eq!(path, t.0.join("native-device-v1.json"));
    assert!(validate(&path, &o).is_err());
    preflight(&path).unwrap();
    write_report(&path, &report);
    assert!(preflight(&path).is_err());
    let link = t.0.join("link.json");
    std::os::unix::fs::symlink(&path, &link).unwrap();
    assert!(validate(&link, &o).is_err());
    o.files.bytes_before_summary = 8 << 20;
    assert!(validate(&path, &o).is_err());
    std::fs::write(&path, vec![b' '; data::MAX_BYTES + 1]).unwrap();
    assert!(validate(&path, &o).is_err());
    assert!(
        run(
            Request {
                schema: REQUEST_SCHEMA.into(),
                decode: config()
            },
            false
        )
        .is_err()
    );
}

#[test]
fn prefix_device_parent_request_is_closed_and_distinct_from_host_and_plain() {
    let request = Request {
        schema: REQUEST_SCHEMA.into(),
        decode: config(),
    };
    let raw = serde_json::to_vec(&request).unwrap();
    Request::parse(&raw).unwrap();
    assert!(Config::parse(&raw).is_err());
    assert!(super::super::host_policy_v2::Request::parse(&raw).is_err());
    for field in ["policy", "raw_timestamps", "calibrated_nanoseconds"] {
        let mut value = serde_json::to_value(&request).unwrap();
        value[field] = true.into();
        assert!(Request::parse(&serde_json::to_vec(&value).unwrap()).is_err());
    }
    let mut changed = request.clone();
    changed.schema = "FerricFinitePrefixDecodeHostPolicyRequestV2".into();
    assert!(Request::parse(&serde_json::to_vec(&changed).unwrap()).is_err());
    let mut duplicate = b"{\"schema\":\"duplicate\",".to_vec();
    duplicate.extend_from_slice(&raw[1..]);
    assert!(Request::parse(&duplicate).is_err());
    assert!(Request::parse(&[]).is_err());
    assert!(Request::parse(&vec![b' '; 65537]).is_err());
}

#[test]
fn prefix_device_parent_worker_selector_keeps_old_argv_and_only_device_sidecar() {
    use super::super::HostDiagnostic;
    use crate::prefix_decode_host_observation_v2::Policy;
    use std::process::Command;
    let cases = [
        (
            HostDiagnostic::Device(PathBuf::from("/tmp/device.json")),
            vec![
                "--engineering-native-prefix-decode-device-v1",
                "--device-sidecar",
                "/tmp/device.json",
            ],
        ),
        (
            HostDiagnostic::V1(PathBuf::from("/tmp/host.json")),
            vec![
                "--engineering-native-prefix-decode-host-v1",
                "--host-sidecar",
                "/tmp/host.json",
            ],
        ),
        (
            HostDiagnostic::V2(
                PathBuf::from("/tmp/policy.json"),
                Policy::SharedFullCurrentness,
            ),
            vec![
                "--engineering-native-prefix-decode-host-v2",
                "--host-sidecar",
                "/tmp/policy.json",
                "--host-policy",
                "shared-full-currentness",
            ],
        ),
    ];
    for (diagnostic, expected) in cases {
        let mut command = Command::new("/not-executed");
        command.arg(diagnostic.launch_flag());
        diagnostic.append_args(&mut command);
        assert_eq!(
            command
                .get_args()
                .map(|value| value.to_str().unwrap())
                .collect::<Vec<_>>(),
            expected
        );
    }
}
