use super::*;
use crate::finite_prefix_layer_wire_v1::tests::{bootstrap, control};

fn digest(text: &str) -> [u8; 32] {
    core::array::from_fn(|i| u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).unwrap())
}

fn fixture() -> (CaptureConfig, RunRecord, Vec<u8>) {
    let b = bootstrap(wire::Profile::Prefix284Mlp548);
    let image = FilePin {
        path: "/task/image".into(),
        bytes: 1,
        sha256: hash(&[1]),
    };
    let config = Config {
        schema: REQUEST_SCHEMA.into(),
        source: "/task/model".into(),
        worker: image.clone(),
        images: ImagePins {
            prefix: image.clone(),
            mlp: image.clone(),
            residual: image.clone(),
            tail: image.clone(),
        },
        expected_bundle_id: b.begin.scope.bundle_id,
        expected_model_id: b.begin.scope.model_id,
        device_ids: b.device_ids,
        session: b.begin.scope.session,
        prompt: PromptPins {
            manifest: FilePin {
                path: "/task/manifest".into(),
                bytes: 21318,
                sha256: digest("30d047aafbd7de2b94647154680b0b9d9fb75aae2fd90b081afe0de595a88600"),
            },
            text: FilePin {
                path: "/task/text".into(),
                bytes: 11224,
                sha256: digest("a43ef3619cb96c9c23d020e07630493ced7a588088033a53f1d612448375751a"),
            },
            tokens: FilePin {
                path: "/task/tokens".into(),
                bytes: 8192,
                sha256: digest("2e81cc60562760f972450ef9205a4e764fa3cce6289d85754e7dfbd7de4d4d02"),
            },
        },
        prefix_tiles_image: image.clone(),
        mlp_tiles_image: image,
        evidence_directory: "/task/evidence".into(),
        dispatch_timeout_ms: b.timeout_ms,
        child_deadline_ms: 3600000,
    };
    let payload = vec![0; wire::CAPTURE_BYTES];
    let record = RunRecord {
        child_pid: b.begin.scope.child_identity,
        profile_sha256: b.sha256().unwrap(),
        setup_commands: 1,
        close: wire::Response {
            protocol: wire::PROTOCOL,
            id: 2,
            profile_sha256: b.sha256().unwrap(),
            profile: b.profile,
            native_closed: true,
            completed_layers: 1,
            control: Some(control(b.profile)),
            capture: Some(wire::part(&payload)),
            gpu_execution: true,
            numerical_acceptance: false,
            performance_claim: false,
            production_authority: false,
        },
        bootstrap: b,
        child_exit_zero: true,
        process_group_absent: true,
    };
    (CaptureConfig(config), record, payload)
}

#[test]
fn capture_request_and_legacy_comparison_schemas_do_not_alias() {
    let (config, _, _) = fixture();
    let raw = serde_json::to_vec(&config).unwrap();
    CaptureConfig::parse(&raw).unwrap();
    assert!(Config::parse(&raw).is_err());
    let mut paired = config.0;
    paired.schema = "FerricFinitePrefixLayerComparisonRequestV1".into();
    let paired = serde_json::to_vec(&paired).unwrap();
    Config::parse(&paired).unwrap();
    assert!(CaptureConfig::parse(&paired).is_err());
}

#[test]
fn capture_request_rejects_extra_modes_and_injected_inputs() {
    let (config, _, _) = fixture();
    for key in [
        "mode",
        "forwards",
        "input_tokens",
        "hidden",
        "kernel_admission",
        "operational_currentness",
    ] {
        let mut value = serde_json::to_value(&config).unwrap();
        value[key] = true.into();
        assert!(CaptureConfig::parse(&serde_json::to_vec(&value).unwrap()).is_err());
    }
    assert!(CaptureConfig::parse(&[]).is_err());
    assert!(CaptureConfig::parse(&vec![b' '; 65537]).is_err());
}

#[test]
fn capture_request_refuses_identity_and_input_bounds_before_io() {
    let (config, _, _) = fixture();
    for which in 0..8 {
        let mut bad = config.clone();
        match which {
            0 => bad.0.device_ids = [7, 7],
            1 => bad.0.session = [0; 32],
            2 => bad.0.prompt.tokens.sha256 = [0; 32],
            3 => bad.0.source = "relative".into(),
            4 => bad.0.child_deadline_ms = 3600001,
            5 => bad.0.dispatch_timeout_ms = 10001,
            6 => bad.0.prefix_tiles_image.bytes = 0,
            _ => bad.0.mlp_tiles_image.bytes = (32 << 20) + 1,
        }
        assert!(CaptureConfig::parse(&serde_json::to_vec(&bad).unwrap()).is_err());
    }
}

#[test]
fn capture_entry_requires_explicit_optin_before_io() {
    let (config, _, _) = fixture();
    assert!(run_capture(config, false).err().unwrap().contains("opt-in"));
}

#[test]
fn capture_records_all_twenty_eight_stage_extents_offsets_and_digests() {
    let (config, mut record, mut payload) = fixture();
    let mut offset = 0;
    for i in 0..28 {
        let (_, bytes, _) = wire::STAGES[i % 14];
        // Leave KV outside the sole current slot untouched.
        payload[offset] = (i + 1) as u8;
        offset += bytes;
    }
    record.close.capture = Some(wire::part(&payload));
    let stages = closed_stages(&config.0, &record, &payload).unwrap();
    assert_eq!(stages.len(), 28);
    offset = 0;
    for (i, s) in stages.iter().enumerate() {
        let (name, bytes, width) = wire::STAGES[i % 14];
        assert_eq!(
            (
                s.rank,
                s.stage,
                s.offset,
                s.bytes,
                s.elements,
                s.element_bytes
            ),
            ((i / 14) as u32, name, offset, bytes, bytes / width, width)
        );
        assert_eq!(s.sha256, hash(&payload[offset..offset + bytes]));
        offset += bytes;
    }
    assert_eq!(offset, wire::CAPTURE_BYTES);
}

#[test]
fn capture_rejects_nonfinite_truncation_mutation_and_untouched_kv_writes() {
    for which in 0..5 {
        let (config, mut record, mut payload) = fixture();
        match which {
            0 => payload[..2].copy_from_slice(&0x7fc0_u16.to_le_bytes()),
            1 => {
                payload.pop();
            }
            2 => payload[0] = 1,
            3 => payload[8192 + 6144 + 4096 + 1024] = 1,
            _ => {
                let offset = wire::STAGES[..6]
                    .iter()
                    .map(|(_, bytes, _)| bytes)
                    .sum::<usize>();
                payload[offset..offset + 4].copy_from_slice(&f32::INFINITY.to_bits().to_le_bytes());
            }
        }
        if which != 2 {
            record.close.capture = Some(wire::part(&payload));
        }
        assert!(closed_stages(&config.0, &record, &payload).is_err());
    }
}

#[test]
fn capture_binds_current_images_child_scope_and_genuine_input() {
    for which in 0..10 {
        let (mut config, mut record, payload) = fixture();
        match which {
            0 => record.child_pid = record.child_pid.wrapping_add(1),
            1 => config.0.prefix_tiles_image.sha256 = [9; 32],
            2 => config.0.mlp_tiles_image.sha256 = [9; 32],
            3 => config.0.images.tail.sha256 = [9; 32],
            4 => config.0.images.prefix.sha256 = [9; 32],
            5 => record.bootstrap.input.token = 785,
            6 => record.bootstrap.input.generation = 2,
            7 => record.bootstrap.input.cache_metadata[0] = 1,
            8 => config.0.session = [9; 32],
            _ => config.0.device_ids.swap(0, 1),
        }
        if let Ok(sha) = record.bootstrap.sha256() {
            record.profile_sha256 = sha;
            record.close.profile_sha256 = sha;
        }
        assert!(closed_stages(&config.0, &record, &payload).is_err());
    }
}

#[test]
fn capture_requires_complete_terminal_close_and_reaped_child() {
    for which in 0..10 {
        let (config, mut record, payload) = fixture();
        match which {
            0 => record.child_exit_zero = false,
            1 => record.process_group_absent = false,
            2 => record.setup_commands = 0,
            3 => record.close.native_closed = false,
            4 => record.close.id = 1,
            5 => record.close.control = None,
            6 => record.close.control.as_mut().unwrap().prefix[0].truncate(22),
            7 => record.close.control.as_mut().unwrap().mlp[1][0] = 0,
            8 => record.close.profile_sha256 = [9; 32],
            _ => record.close.performance_claim = true,
        }
        assert!(closed_stages(&config.0, &record, &payload).is_err());
    }
}

#[test]
fn capture_observation_is_bounded_and_never_claims_comparison_or_numerical_acceptance() {
    let (config, mut record, payload) = fixture();
    let c = record.close.control.as_mut().unwrap();
    c.embedding_ns = [u64::MAX; 2];
    c.paired_ns = [[u64::MAX; 2]; 4];
    record.setup_commands = u64::MAX;
    let files = (0..11)
        .map(|i| evidence::File {
            name: format!("candidate-response-{i}.json"),
            bytes: evidence::LIMIT,
            sha256: [255; 32],
        })
        .collect();
    let observation = CaptureObservation::closed(config, record, &payload, files).unwrap();
    let raw = serde_json::to_vec(&observation).unwrap();
    assert!(raw.len() + 1 < 65536);
    let value: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(
        value["schema"],
        "FerricFinitePrefixLayerCaptureObservationV1"
    );
    assert_eq!(value["native_attempts"], 1);
    assert_eq!(value["retries"], 0);
    assert_eq!(value["completed_layers"], 1);
    assert_eq!(value["gpu_execution"], true);
    for name in [
        "paired_comparison_performed",
        "numerical_acceptance",
        "performance_claim",
        "production_authority",
        "full_forward",
    ] {
        assert_eq!(value[name], false, "{name}");
    }
    assert!(value.get("runs").is_none());
    assert!(value.get("bitwise_equal").is_none());
    assert!(
        value["stages"]
            .as_array()
            .unwrap()
            .iter()
            .all(|s| s.get("bit_mismatches").is_none())
    );
}
