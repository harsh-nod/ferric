use super::*;
use crate::projection_residual_decode_host_observation_v1 as data;

fn shared_fixture(
    actual: &Observation,
    wrapped: &projection_wire::Bootstrap,
) -> data::SharedReport {
    let mut observation = host_fixture(actual, wrapped);
    observation.schema = data::SHARED_SCHEMA.into();
    for snapshot in &mut observation.snapshots {
        snapshot.shared_full_currentness = true;
    }
    data::SharedReport {
        schema: data::SHARED_ENVELOPE.into(),
        policy: data::Policy::SharedFull,
        configuration_host_ns: 9000,
        observation,
    }
}

#[test]
fn projection_shared_parent_validates_full_envelope_and_refuses_v1_cross_use() {
    let temp = Temp::new();
    let path = temp.0.join("ar");
    let (config, actual, wrapped) = completed_for(&path, true, wire::InputMode::Autoregressive);
    let report = shared_fixture(&actual, &wrapped);
    let raw = serde_json::to_vec(&report).unwrap();
    let sidecar = temp.0.join("shared.json");
    std::fs::write(&sidecar, &raw).unwrap();
    let pin = super::super::super::projection_host::validate_shared(
        &sidecar,
        &actual,
        &config.projection_residual_image,
    )
    .unwrap();
    assert_eq!(pin.bytes, raw.len() as u64);
    assert_eq!(pin.sha256, hash(&raw));
    assert!(
        super::super::super::projection_host::validate(
            &sidecar,
            &actual,
            &config.projection_residual_image,
        )
        .is_err()
    );
    std::fs::write(
        &sidecar,
        serde_json::to_vec(&host_fixture(&actual, &wrapped)).unwrap(),
    )
    .unwrap();
    assert!(
        super::super::super::projection_host::validate_shared(
            &sidecar,
            &actual,
            &config.projection_residual_image,
        )
        .is_err()
    );
    assert!(!path.join("complete.json").exists());
}

#[test]
fn projection_shared_parent_rejects_worker_projection_profile_and_payload_changes() {
    for change in 0..6 {
        let temp = Temp::new();
        let path = temp.0.join("ar");
        let (mut config, mut actual, wrapped) =
            completed_for(&path, true, wire::InputMode::Autoregressive);
        let mut report = shared_fixture(&actual, &wrapped);
        match change {
            0 => report.observation.worker_sha256[0] ^= 1,
            1 => config.projection_residual_image.sha256[0] ^= 1,
            2 => actual.profile_sha256[0] ^= 1,
            3 => actual.transcript_sha256[0] ^= 1,
            4 => actual.files.frames[0].observation.sha256[0] ^= 1,
            _ => {
                std::fs::write(&actual.files.frames[3].observation.path, [0]).unwrap();
            }
        }
        let sidecar = temp.0.join("shared.json");
        std::fs::write(&sidecar, serde_json::to_vec(&report).unwrap()).unwrap();
        assert!(
            super::super::super::projection_host::validate_shared(
                &sidecar,
                &actual,
                &config.projection_residual_image,
            )
            .is_err()
        );
    }
}

#[test]
fn projection_shared_parent_requires_close_reap_ar_and_exact_policy() {
    for change in 0..7 {
        let temp = Temp::new();
        let path = temp.0.join("ar");
        let (config, mut actual, wrapped) =
            completed_for(&path, true, wire::InputMode::Autoregressive);
        let mut report = shared_fixture(&actual, &wrapped);
        match change {
            0 => actual.native_closed = false,
            1 => actual.child_exit_zero = false,
            2 => actual.process_group_absent = false,
            3 => actual.request.mode = wire::InputMode::TeacherForced,
            4 => report.policy = data::Policy::DefaultFull,
            5 => report.observation.snapshots[4].shared_full_currentness = false,
            _ => report.observation.completions[2].input_token ^= 1,
        }
        let sidecar = temp.0.join("shared.json");
        std::fs::write(&sidecar, serde_json::to_vec(&report).unwrap()).unwrap();
        assert!(
            super::super::super::projection_host::validate_shared(
                &sidecar,
                &actual,
                &config.projection_residual_image,
            )
            .is_err()
        );
    }
}

#[test]
fn projection_shared_parent_selector_keeps_plain_and_v1_argv_unchanged() {
    let c = config();
    let path = std::path::PathBuf::from("/tmp/shared.json");
    let shared =
        HostDiagnostic::ProjectionShared(path.clone(), c.projection_residual_image.clone());
    let default = HostDiagnostic::Projection(path.clone(), c.projection_residual_image);
    assert_eq!(
        worker_flag(Some(&shared), true).unwrap(),
        data::Policy::SharedFull.worker_flag()
    );
    assert!(worker_flag(Some(&shared), false).is_err());
    assert_eq!(
        worker_flag(Some(&default), true).unwrap(),
        data::Policy::DefaultFull.worker_flag()
    );
    assert_eq!(
        worker_flag(None, true).unwrap(),
        "--engineering-native-projection-residual-decode-v1"
    );
    let mut new = std::process::Command::new("worker");
    shared.append_args(&mut new);
    let mut old = std::process::Command::new("worker");
    default.append_args(&mut old);
    assert_eq!(
        new.get_args().collect::<Vec<_>>(),
        old.get_args().collect::<Vec<_>>()
    );
    assert_eq!(
        new.get_args().collect::<Vec<_>>(),
        [
            std::ffi::OsStr::new("--host-sidecar"),
            std::ffi::OsStr::new("/tmp/shared.json"),
        ]
    );
}

#[test]
fn projection_shared_parent_refuses_missing_malformed_and_oversized_sidecars() {
    let temp = Temp::new();
    let path = temp.0.join("ar");
    let (config, actual, _) = completed_for(&path, true, wire::InputMode::Autoregressive);
    let sidecar = temp.0.join("missing.json");
    assert!(
        super::super::super::projection_host::validate_shared(
            &sidecar,
            &actual,
            &config.projection_residual_image,
        )
        .is_err()
    );
    for raw in [b"{}".to_vec(), vec![b' '; data::MAX_BYTES + 1]] {
        std::fs::write(&sidecar, raw).unwrap();
        assert!(
            super::super::super::projection_host::validate_shared(
                &sidecar,
                &actual,
                &config.projection_residual_image,
            )
            .is_err()
        );
    }
    assert!(!path.join("complete.json").exists());
}
