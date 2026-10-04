use super::super::{
    evidence,
    tests::{Temp, config},
};
use super::*;
use crate::prefix_decode_host_observation_v2::tests::{report, report_for};

fn validate(path: &Path, observation: &Observation) -> Result<FilePin> {
    super::validate(path, observation, Policy::Baseline)
}

fn fixture(path: &Path) -> (Observation, data::Report) {
    let report = report(wire::InputMode::TeacherForced);
    let b = &report.bootstrap;
    let mut ev = evidence::Evidence::create(path).unwrap();
    let mut chain = wire::Chain::new(b.registration, b.sha256().unwrap());
    for position in 0..4 {
        let r = wire::tests::request(b, position, None);
        let (s, c, bytes) = wire::tests::completed(&r, &mut chain, 100 + position);
        ev.append(&r, &s, &c, &bytes).unwrap();
    }
    let files = ev.finish(&[]).unwrap();
    let mut config = config();
    config.evidence_directory = path.into();
    config.worker.sha256 = report.worker_sha256;
    config.prefix_image.sha256 = b.prefix_image.sha256;
    config.tiles_image.sha256 = b.tiles_image.sha256;
    let observation = Observation {
        schema: "FerricFinitePrefixDecodeObservationV1",
        request: config,
        child_pid: b.scope.child_identity,
        registration_sha256: b.registration,
        source_program_sha256: b.begin.source_program.sha256,
        upload_manifest_sha256: b.begin.uploads.sha256,
        bootstrap: b.clone(),
        profile_sha256: b.sha256().unwrap(),
        setup_commands: 1,
        completed_forwards: 4,
        input_tokens: b.input_tokens.clone(),
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
#[test]
fn prefix_host_parent_actual_file_binding_keeps_native14_census_and8mib_bound() {
    let t = Temp::new();
    let (mut observation, report) = fixture(&t.0.join("native"));
    let path = sidecar_path(&observation.request).unwrap();
    preflight(&path).unwrap();
    std::fs::write(&path, serde_json::to_vec(&report).unwrap()).unwrap();
    let pin = validate(&path, &observation).unwrap();
    assert!(pin.bytes <= data::MAX_BYTES as u64);
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
    assert_eq!(5_623_968 + data::MAX_BYTES, 5_689_504);
    assert!(5_689_504 < 8 << 20);
}
#[test]
fn prefix_host_parent_refuses_unclosed_unreaped_and_wrong_worker_model_images() {
    let t = Temp::new();
    let (observation, report) = fixture(&t.0.join("native"));
    let path = sidecar_path(&observation.request).unwrap();
    std::fs::write(&path, serde_json::to_vec(&report).unwrap()).unwrap();
    for change in 0..13 {
        let mut o = fixture(&t.0.join(format!("native-{change}"))).0;
        validate(&path, &o).unwrap();
        match change {
            0 => o.child_exit_zero = false,
            1 => o.process_group_absent = false,
            2 => o.native_closed = false,
            3 => o.request.worker.sha256[0] ^= 1,
            4 => o.request.expected_model_id[0] ^= 1,
            5 => o.request.prefix_image.sha256[0] ^= 1,
            6 => o.request.tiles_image.bytes += 1,
            7 => o.request.session[0] ^= 1,
            8 => o.source_program_sha256[0] ^= 1,
            9 => o.upload_manifest_sha256[0] ^= 1,
            10 => o.child_pid += 1,
            11 => {
                o.request.prefix_image.bytes =
                    u64::from(report.bootstrap.prefix_image.bytes) + (1_u64 << 32);
                assert!(u32::try_from(o.request.prefix_image.bytes).is_err());
                assert_eq!(
                    o.request.prefix_image.bytes % (1_u64 << 32),
                    u64::from(report.bootstrap.prefix_image.bytes)
                );
            }
            _ => {
                o.request.tiles_image.bytes =
                    u64::from(report.bootstrap.tiles_image.bytes) + (1_u64 << 32);
                assert!(u32::try_from(o.request.tiles_image.bytes).is_err());
                assert_eq!(
                    o.request.tiles_image.bytes % (1_u64 << 32),
                    u64::from(report.bootstrap.tiles_image.bytes)
                );
            }
        }
        assert!(validate(&path, &o).is_err());
        assert!(!o.request.evidence_directory.join("complete.json").exists());
    }
}
#[test]
fn prefix_host_parent_control_payload_drift_and_missing_sidecar_refuse_publication() {
    let t = Temp::new();
    let (mut o, report) = fixture(&t.0.join("native"));
    let path = sidecar_path(&o.request).unwrap();
    assert!(validate(&path, &o).is_err());
    std::fs::write(&path, serde_json::to_vec(&report).unwrap()).unwrap();
    let original = o.files.frames[0].response.clone();
    if let wire::Event::Completed(c) = &mut o.files.frames[0].response.event {
        c.output_token += 1;
    }
    assert!(validate(&path, &o).is_err());
    o.files.frames[0].response = original;
    std::fs::write(&o.files.frames[3].control.path, [0]).unwrap();
    assert!(validate(&path, &o).is_err());
    assert!(!o.request.evidence_directory.join("complete.json").exists());
}
#[test]
fn prefix_host_parent_sidecar_path_collisions_links_and_extent_are_closed() {
    let t = Temp::new();
    let path = t.0.join("sidecar.json");
    preflight(&path).unwrap();
    std::fs::write(&path, [1]).unwrap();
    assert!(preflight(&path).is_err());
    assert!(pin_file(&path, 0).is_err());
    let link = t.0.join("link.json");
    std::os::unix::fs::symlink(&path, &link).unwrap();
    assert!(preflight(&link).is_err());
    assert!(pin_file(&link, 64).is_err());
    let mut config = config();
    config.evidence_directory = t.0.join("native");
    assert_eq!(
        sidecar_path(&config).unwrap(),
        t.0.join("native-host-policy-v2.json")
    );
    assert!(
        run(
            Request {
                schema: REQUEST_SCHEMA.into(),
                policy: Policy::Baseline,
                decode: config
            },
            false
        )
        .is_err()
    );
}

#[test]
fn prefix_host_v2_parent_policy_is_explicit_closed_and_request_bound() {
    for policy in [
        Policy::Baseline,
        Policy::ImmutableAdmissionCache,
        Policy::SharedFullCurrentness,
    ] {
        let request = Request {
            schema: REQUEST_SCHEMA.into(),
            policy,
            decode: config(),
        };
        let raw = serde_json::to_vec(&request).unwrap();
        assert_eq!(Request::parse(&raw).unwrap().policy, policy);
        assert!(Config::parse(&raw).is_err());
        let mut changed = serde_json::to_value(&request).unwrap();
        changed.as_object_mut().unwrap().remove("policy");
        assert!(Request::parse(&serde_json::to_vec(&changed).unwrap()).is_err());
        for name in ["both", "auto", "operational-currentness", "baseline "] {
            let mut changed = serde_json::to_value(&request).unwrap();
            changed["policy"] = name.into();
            assert!(Request::parse(&serde_json::to_vec(&changed).unwrap()).is_err());
        }
        let mut changed = serde_json::to_value(&request).unwrap();
        changed["profile"] = true.into();
        assert!(Request::parse(&serde_json::to_vec(&changed).unwrap()).is_err());
        let mut changed = request.clone();
        changed.schema = "FerricFinitePrefixDecodeRequestV1".into();
        assert!(Request::parse(&serde_json::to_vec(&changed).unwrap()).is_err());
    }
    assert!(Request::parse(&[]).is_err());
    assert!(Request::parse(&vec![b' '; 65537]).is_err());
}

#[test]
fn prefix_host_v2_parent_actual_policy_must_equal_request_and_rejects_v1() {
    for policy in [
        Policy::Baseline,
        Policy::ImmutableAdmissionCache,
        Policy::SharedFullCurrentness,
    ] {
        let t = Temp::new();
        let (observation, _) = fixture(&t.0.join("native"));
        let path = sidecar_path(&observation.request).unwrap();
        let report = report_for(wire::InputMode::TeacherForced, policy);
        std::fs::write(&path, serde_json::to_vec(&report).unwrap()).unwrap();
        super::validate(&path, &observation, policy).unwrap();
        assert!(super::super::host_observation::validate(&path, &observation).is_err());
        for other in [
            Policy::Baseline,
            Policy::ImmutableAdmissionCache,
            Policy::SharedFullCurrentness,
        ] {
            if policy != other {
                assert!(super::validate(&path, &observation, other).is_err());
            }
        }
        let old =
            crate::prefix_decode_host_observation_v1::tests::report(wire::InputMode::TeacherForced);
        std::fs::write(&path, serde_json::to_vec(&old).unwrap()).unwrap();
        assert!(super::validate(&path, &observation, policy).is_err());
    }
}

#[test]
fn prefix_host_v2_parent_worker_arguments_bind_one_explicit_policy() {
    use super::super::HostDiagnostic;
    use std::process::Command;
    for policy in [
        Policy::Baseline,
        Policy::ImmutableAdmissionCache,
        Policy::SharedFullCurrentness,
    ] {
        let request = HostDiagnostic::V2(PathBuf::from("/tmp/host-v2.json"), policy);
        let mut command = Command::new("/not-executed");
        command.arg(request.launch_flag());
        request.append_args(&mut command);
        let actual = command
            .get_args()
            .map(|a| a.to_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            actual,
            [
                "--engineering-native-prefix-decode-host-v2",
                "--host-sidecar",
                "/tmp/host-v2.json",
                "--host-policy",
                policy.name()
            ]
        );
    }
    let old = HostDiagnostic::V1(PathBuf::from("/tmp/host-v1.json"));
    let mut command = Command::new("/not-executed");
    command.arg(old.launch_flag());
    old.append_args(&mut command);
    assert_eq!(
        command
            .get_args()
            .map(|a| a.to_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "--engineering-native-prefix-decode-host-v1",
            "--host-sidecar",
            "/tmp/host-v1.json"
        ]
    );
}
