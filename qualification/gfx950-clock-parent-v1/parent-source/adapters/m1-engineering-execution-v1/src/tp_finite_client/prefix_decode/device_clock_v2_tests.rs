//! Synthetic retained files only; no worker or native clock sampler is launched.
use super::super::{
    device_v1::tests::fixture as raw_fixture,
    evidence,
    tests::{Temp, config},
    wire,
};
use super::*;
use crate::prefix_decode_device_observation_v1 as raw_data;

fn fixture(path: &Path, mode: wire::InputMode) -> (Observation, data::Report) {
    let (observation, raw) = raw_fixture(path, mode);
    let samples = (0..data::SAMPLE_COUNT)
        .map(|index| {
            let position = index / 4;
            let rank = index % 2;
            let post = index % 4 >= 2;
            data::Sample {
                generation: position as u64 + 1,
                position: position as u32,
                endpoint: if post {
                    data::Endpoint::Post
                } else {
                    data::Endpoint::Pre
                },
                rank: rank as u32,
                row_boundary: ((position + usize::from(post)) * raw_data::PER_FORWARD) as u32,
                group_incarnation: raw.group_incarnation,
                unique_id: raw.ranks[rank].unique_id,
                queue_epoch: raw.ranks[rank].queue_epoch,
                gpu_id: rank as u32 + 11,
                gpu_clock_counter: 0,
                cpu_clock_counter: u64::MAX - index as u64,
                system_clock_counter: index as u64,
                system_clock_frequency_hz: 1_000_000_000,
                host_started_ns: index as u64 * 2,
                host_finished_ns: index as u64 * 2 + 1,
            }
        })
        .collect();
    let report = data::Report {
        schema: data::SCHEMA.into(),
        raw,
        samples,
        raw_clock_counters: true,
        clock_domain_validated: false,
        calibrated_nanoseconds: false,
        cross_device_clock_alignment: false,
        overlap_claim: false,
        numerical_acceptance: false,
        performance_claim: false,
        production_authority: false,
        full_model_acceptance: false,
    };
    report.validate().unwrap();
    (observation, report)
}
fn write_report(path: &Path, report: &data::Report) {
    std::fs::write(path, serde_json::to_vec(report).unwrap()).unwrap();
}

#[test]
fn prefix_clock_parent_tf_ar_joins_sixteen_samples_and_raw_controls_before_summary() {
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
        assert_eq!(report.samples.len(), 16);
        assert_eq!(report.raw.rows.len(), 1172);
        assert!(pin.bytes > 65536 && pin.bytes <= data::MAX_BYTES as u64);
        assert!(
            !observation
                .request
                .evidence_directory
                .join("complete.json")
                .exists()
        );
        evidence::publish(&mut observation).unwrap();
        assert!(observation.files.total_bytes + pin.bytes <= 8 << 20);
    }
}

#[test]
fn prefix_clock_parent_refuses_unclosed_unreaped_or_wrong_child_and_source() {
    let t = Temp::new();
    let (observation, report) = fixture(&t.0.join("native"), wire::InputMode::TeacherForced);
    let path = sidecar_path(&observation.request).unwrap();
    write_report(&path, &report);
    for change in 0..9 {
        let (mut o, _) = fixture(
            &t.0.join(format!("case-{change}")),
            wire::InputMode::TeacherForced,
        );
        match change {
            0 => o.child_exit_zero = false,
            1 => o.process_group_absent = false,
            2 => o.native_closed = false,
            3 => o.completed_forwards = 3,
            4 => o.child_pid += 1,
            5 => o.request.worker.sha256[0] ^= 1,
            6 => o.profile_sha256[0] ^= 1,
            7 => o.transcript_sha256[0] ^= 1,
            _ => o.source_program_sha256[0] ^= 1,
        }
        assert!(validate(&path, &o).is_err(), "{change}");
        assert!(!o.request.evidence_directory.join("complete.json").exists());
    }
}

#[test]
fn prefix_clock_parent_actual_control_join_survives_v2_wrapping() {
    let t = Temp::new();
    let (o, report) = fixture(&t.0.join("native"), wire::InputMode::TeacherForced);
    let path = sidecar_path(&o.request).unwrap();
    for position in 0..4 {
        for slot in [0, 1, 2, 3, 4, 6, 8, 289, 290, 291, 292] {
            let mut changed = report.clone();
            changed.raw.rows[position * raw_data::PER_FORWARD + slot].host_elapsed_ns ^= 1;
            changed.validate().unwrap();
            write_report(&path, &changed);
            assert!(validate(&path, &o).is_err(), "{position}/{slot}");
        }
    }
    write_report(&path, &report);
    validate(&path, &o).unwrap();
}

#[test]
fn prefix_clock_parent_capture_and_completion_joins_remain_mandatory() {
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
fn prefix_clock_parent_refuses_bad_samples_at_every_position_and_rank() {
    let t = Temp::new();
    let (o, report) = fixture(&t.0.join("native"), wire::InputMode::TeacherForced);
    let path = sidecar_path(&o.request).unwrap();
    for index in 0..16 {
        for change in 0..9 {
            let mut changed = report.clone();
            let sample = &mut changed.samples[index];
            match change {
                0 => sample.generation ^= 1,
                1 => sample.position ^= 1,
                2 => sample.rank ^= 1,
                3 => sample.row_boundary ^= 1,
                4 => sample.group_incarnation ^= 1,
                5 => sample.unique_id ^= 1,
                6 => sample.queue_epoch ^= 1,
                7 => sample.system_clock_frequency_hz = 0,
                _ => sample.host_started_ns = sample.host_finished_ns + 1,
            }
            write_report(&path, &changed);
            assert!(validate(&path, &o).is_err(), "{index}/{change}");
        }
    }
    for remove in [false, true] {
        let mut changed = report.clone();
        if remove {
            changed.samples.pop();
        } else {
            changed.samples.push(changed.samples[0].clone());
        }
        write_report(&path, &changed);
        assert!(validate(&path, &o).is_err());
    }
}

#[test]
fn prefix_clock_parent_closed_schemas_and_nonclaims_reject_v1_fallback() {
    let t = Temp::new();
    let (o, report) = fixture(&t.0.join("native"), wire::InputMode::TeacherForced);
    let path = sidecar_path(&o.request).unwrap();
    write_report(&path, &report);
    assert!(device_v1::validate(&path, &o).is_err());
    std::fs::write(&path, report.raw.encode().unwrap()).unwrap();
    assert!(validate(&path, &o).is_err());
    for field in [
        "clock_domain_validated",
        "calibrated_nanoseconds",
        "cross_device_clock_alignment",
        "overlap_claim",
        "numerical_acceptance",
        "performance_claim",
        "production_authority",
        "full_model_acceptance",
        "unknown",
    ] {
        let mut value = serde_json::to_value(&report).unwrap();
        value[field] = true.into();
        std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(validate(&path, &o).is_err(), "{field}");
    }
}

#[test]
fn prefix_clock_parent_charges_entire_sidecar_and_preserves_path_file_bounds() {
    let t = Temp::new();
    let (mut o, report) = fixture(&t.0.join("native"), wire::InputMode::TeacherForced);
    let path = sidecar_path(&o.request).unwrap();
    assert_eq!(path, t.0.join("native-device-clock-v2.json"));
    assert!(validate(&path, &o).is_err());
    preflight(&path).unwrap();
    write_report(&path, &report);
    let pin = validate(&path, &o).unwrap();
    assert!(preflight(&path).is_err());
    let link = t.0.join("link.json");
    std::os::unix::fs::symlink(&path, &link).unwrap();
    assert!(validate(&link, &o).is_err());
    o.files.bytes_before_summary = (8 << 20) - evidence::SUMMARY_LIMIT as u64 - pin.bytes;
    validate(&path, &o).unwrap();
    o.files.bytes_before_summary += 1;
    assert!(validate(&path, &o).is_err());
    o.files.bytes_before_summary = u64::MAX;
    assert!(validate(&path, &o).is_err());
    std::fs::write(&path, vec![b' '; data::MAX_BYTES + 1]).unwrap();
    assert!(validate(&path, &o).is_err());
}

#[test]
fn prefix_clock_parent_request_and_optin_are_distinct_from_all_existing_modes() {
    let request = Request {
        schema: REQUEST_SCHEMA.into(),
        decode: config(),
    };
    let raw = serde_json::to_vec(&request).unwrap();
    Request::parse(&raw).unwrap();
    assert!(Config::parse(&raw).is_err());
    assert!(device_v1::Request::parse(&raw).is_err());
    assert!(super::super::host_policy_v2::Request::parse(&raw).is_err());
    for schema in [
        device_v1::REQUEST_SCHEMA,
        "FerricFinitePrefixDecodeHostPolicyRequestV2",
    ] {
        let mut changed = request.clone();
        changed.schema = schema.into();
        assert!(Request::parse(&serde_json::to_vec(&changed).unwrap()).is_err());
    }
    for field in [
        "policy",
        "raw_timestamps",
        "calibrated_nanoseconds",
        "unknown",
    ] {
        let mut changed = serde_json::to_value(&request).unwrap();
        changed[field] = true.into();
        assert!(Request::parse(&serde_json::to_vec(&changed).unwrap()).is_err());
    }
    let mut duplicate = b"{\"schema\":\"duplicate\",".to_vec();
    duplicate.extend_from_slice(&raw[1..]);
    assert!(Request::parse(&duplicate).is_err());
    assert!(Request::parse(&[]).is_err());
    assert!(Request::parse(&vec![b' '; 65537]).is_err());
    assert!(run(request, false).is_err());
}

#[test]
fn prefix_clock_parent_worker_argv_is_exact_without_changing_old_modes() {
    use super::super::HostDiagnostic;
    use crate::prefix_decode_host_observation_v2::Policy;
    use std::process::Command;
    let cases = [
        (
            HostDiagnostic::DeviceClock(PathBuf::from("/tmp/clock.json")),
            vec![
                "--engineering-native-prefix-decode-device-clock-v2",
                "--device-sidecar",
                "/tmp/clock.json",
            ],
        ),
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

#[test]
fn prefix_clock_parent_keeps_raw_zero_and_decreasing_counters_without_conversion() {
    let t = Temp::new();
    let (o, mut report) = fixture(&t.0.join("native"), wire::InputMode::TeacherForced);
    let path = sidecar_path(&o.request).unwrap();
    for (index, sample) in report.samples.iter_mut().enumerate() {
        sample.gpu_clock_counter = u64::MAX - index as u64;
        sample.cpu_clock_counter = 0;
        sample.system_clock_counter = 0;
    }
    write_report(&path, &report);
    validate(&path, &o).unwrap();
    assert!(!report.clock_domain_validated && !report.calibrated_nanoseconds);
}
