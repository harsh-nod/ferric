use super::*;
use crate::finite_prefix_layer_wire_v1::tests::{bootstrap, control};

fn digest(text: &str) -> [u8; 32] {
    core::array::from_fn(|i| u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).unwrap())
}

fn fixture() -> (
    ProjectionCaptureConfig,
    RunRecord,
    projection_wire::Bootstrap,
    Vec<u8>,
) {
    let layer = bootstrap(wire::Profile::Prefix284Mlp548);
    let image = FilePin {
        path: "/task/image".into(),
        bytes: 1,
        sha256: hash(&[1]),
    };
    let projection = FilePin {
        path: "/task/projection".into(),
        bytes: 1,
        sha256: hash(&[2]),
    };
    let config = ProjectionCaptureConfig {
        schema: REQUEST_SCHEMA.into(),
        projection_residual_image: projection.clone(),
        layer: Config {
            schema: LAYER_SCHEMA.into(),
            source: "/task/model".into(),
            worker: image.clone(),
            images: ImagePins {
                prefix: image.clone(),
                mlp: image.clone(),
                residual: image.clone(),
                tail: image.clone(),
            },
            expected_bundle_id: layer.begin.scope.bundle_id,
            expected_model_id: layer.begin.scope.model_id,
            device_ids: layer.device_ids,
            session: layer.begin.scope.session,
            prompt: PromptPins {
                manifest: FilePin {
                    path: "/task/manifest".into(),
                    bytes: 21318,
                    sha256: digest(
                        "30d047aafbd7de2b94647154680b0b9d9fb75aae2fd90b081afe0de595a88600",
                    ),
                },
                text: FilePin {
                    path: "/task/text".into(),
                    bytes: 11224,
                    sha256: digest(
                        "a43ef3619cb96c9c23d020e07630493ced7a588088033a53f1d612448375751a",
                    ),
                },
                tokens: FilePin {
                    path: "/task/tokens".into(),
                    bytes: 8192,
                    sha256: digest(
                        "2e81cc60562760f972450ef9205a4e764fa3cce6289d85754e7dfbd7de4d4d02",
                    ),
                },
            },
            prefix_tiles_image: image.clone(),
            mlp_tiles_image: image,
            evidence_directory: "/task/evidence".into(),
            dispatch_timeout_ms: layer.timeout_ms,
            child_deadline_ms: 3600000,
        },
    };
    let outer = projection_wire::Bootstrap {
        schema: projection_wire::SCHEMA.into(),
        layer: layer.clone(),
        projection_residual_image: pinned_part(&projection),
    };
    let payload = vec![0; wire::CAPTURE_BYTES];
    let profile = outer.sha256().unwrap();
    let record = RunRecord {
        child_pid: layer.begin.scope.child_identity,
        bootstrap: layer.clone(),
        profile_sha256: profile,
        setup_commands: 1,
        child_exit_zero: true,
        process_group_absent: true,
        close: wire::Response {
            protocol: wire::PROTOCOL,
            id: 2,
            profile_sha256: profile,
            profile: layer.profile,
            native_closed: true,
            completed_layers: 1,
            control: Some(control(layer.profile)),
            capture: Some(wire::part(&payload)),
            gpu_execution: true,
            numerical_acceptance: false,
            performance_claim: false,
            production_authority: false,
        },
    };
    (config, record, outer, payload)
}

#[test]
fn projection_request_is_separate_and_rejects_legacy_or_unknown_fields() {
    let (config, _, _, _) = fixture();
    let raw = serde_json::to_vec(&config).unwrap();
    ProjectionCaptureConfig::parse(&raw).unwrap();
    assert!(CaptureConfig::parse(&raw).is_err());
    assert!(Config::parse(&raw).is_err());
    assert!(ProjectionCaptureConfig::parse(&serde_json::to_vec(&config.layer).unwrap()).is_err());
    for key in [
        "hidden",
        "profile",
        "kernel_admission",
        "forwards",
        "operational_currentness",
    ] {
        let mut value = serde_json::to_value(&config).unwrap();
        value[key] = true.into();
        assert!(ProjectionCaptureConfig::parse(&serde_json::to_vec(&value).unwrap()).is_err());
        let mut value = serde_json::to_value(&config).unwrap();
        value["layer"][key] = true.into();
        assert!(ProjectionCaptureConfig::parse(&serde_json::to_vec(&value).unwrap()).is_err());
    }
}

#[test]
fn projection_request_checks_extra_image_and_original_bounds_before_io() {
    let (config, _, _, _) = fixture();
    for which in 0..9 {
        let mut bad = config.clone();
        match which {
            0 => bad.projection_residual_image.path = "relative".into(),
            1 => bad.projection_residual_image.bytes = 0,
            2 => bad.projection_residual_image.bytes = (32 << 20) + 1,
            3 => bad.projection_residual_image.sha256 = [0; 32],
            4 => bad.projection_residual_image = bad.layer.images.residual.clone(),
            5 => bad.layer.schema = "FerricFinitePrefixLayerComparisonRequestV1".into(),
            6 => bad.layer.child_deadline_ms = 3600001,
            7 => bad.layer.device_ids = [7, 7],
            _ => bad.layer.prompt.tokens.sha256 = [0; 32],
        }
        assert!(ProjectionCaptureConfig::parse(&serde_json::to_vec(&bad).unwrap()).is_err());
    }
    assert!(ProjectionCaptureConfig::parse(&[]).is_err());
    assert!(ProjectionCaptureConfig::parse(&vec![b' '; 65537]).is_err());
    assert!(
        run_projection_capture(config, false)
            .err()
            .unwrap()
            .contains("opt-in")
    );
}

#[test]
fn projection_worker_argv_is_distinct_and_legacy_argv_is_unchanged() {
    let (config, _, _, _) = fixture();
    for prefix in [false, true] {
        let command = worker_command(&config.layer, prefix, false).unwrap();
        let args = command
            .get_args()
            .map(|x| x.to_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            args,
            [
                "--engineering-native-prefix-layer-v1",
                "--allow-unauthenticated-machine-code",
                "--devices",
                "7,9",
                "--timeout-ms",
                "1000",
                "--profile",
                if prefix {
                    "prefix284-mlp548"
                } else {
                    "baseline22-mlp548"
                }
            ]
        );
    }
    let command = worker_command(&config.layer, true, true).unwrap();
    assert_eq!(
        command
            .get_args()
            .map(|x| x.to_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "--engineering-native-projection-residual-layer-v1",
            "--allow-unauthenticated-machine-code",
            "--devices",
            "7,9",
            "--timeout-ms",
            "1000"
        ]
    );
    assert!(worker_command(&config.layer, false, true).is_err());
}

#[test]
fn projection_bootstrap_transports_extra_image_without_changing_copy_begin() {
    let (config, _, b, _) = fixture();
    let mut bytes = Vec::new();
    projection_wire::write_bootstrap(&mut bytes, &mut wire::Budget::new(), &b, &[1], &[1], &[2])
        .unwrap();
    let (observed, mlp, prefix, projection) =
        projection_wire::read_bootstrap(&mut bytes.as_slice(), &mut wire::Budget::new()).unwrap();
    assert_eq!(observed, b);
    assert_eq!((mlp, prefix, projection), (vec![1], vec![1], vec![2]));
    assert_eq!(
        observed.layer.begin.residual_image,
        pinned_part(&config.layer.images.residual)
    );
    assert_ne!(
        observed.layer.begin.residual_image,
        observed.projection_residual_image
    );
    assert_ne!(observed.sha256().unwrap(), observed.layer.sha256().unwrap());
}

#[test]
fn projection_close_rejects_old_profile_and_changed_extra_or_inner_images() {
    for which in 0..6 {
        let (mut config, mut record, mut b, payload) = fixture();
        match which {
            0 => {
                record.profile_sha256 = b.layer.sha256().unwrap();
                record.close.profile_sha256 = record.profile_sha256;
            }
            1 => config.projection_residual_image.sha256 = [9; 32],
            2 => b.projection_residual_image.sha256 = [9; 32],
            3 => config.layer.images.residual.sha256 = [9; 32],
            4 => b.layer.prefix_image.as_mut().unwrap().sha256 = [9; 32],
            _ => record.close.profile_sha256 = b.layer.sha256().unwrap(),
        }
        assert!(
            ProjectionCaptureObservation::closed(config, record, b, &payload, Vec::new()).is_err()
        );
    }
}

#[test]
fn projection_close_retains_terminal_control_and_reaped_child_requirements() {
    for which in 0..9 {
        let (config, mut record, b, payload) = fixture();
        match which {
            0 => record.child_exit_zero = false,
            1 => record.process_group_absent = false,
            2 => record.close.native_closed = false,
            3 => record.close.id = 1,
            4 => record.close.control = None,
            5 => record.close.control.as_mut().unwrap().prefix[0].truncate(22),
            6 => record.close.control.as_mut().unwrap().mlp[1][0] = 0,
            7 => record.setup_commands = 0,
            _ => record.close.numerical_acceptance = true,
        }
        assert!(
            ProjectionCaptureObservation::closed(config, record, b, &payload, Vec::new()).is_err()
        );
    }
}

#[test]
fn projection_close_rejects_nonfinite_mutated_and_outside_slot_captures() {
    for which in 0..4 {
        let (config, mut record, b, mut payload) = fixture();
        match which {
            0 => payload[..2].copy_from_slice(&0x7fc0_u16.to_le_bytes()),
            1 => payload[8192 + 6144 + 4096 + 1024] = 1,
            2 => {
                payload.pop();
            }
            _ => payload[0] = 1,
        }
        if which != 3 {
            record.close.capture = Some(wire::part(&payload));
        }
        assert!(
            ProjectionCaptureObservation::closed(config, record, b, &payload, Vec::new()).is_err()
        );
    }
}

#[test]
fn projection_capture_has_twenty_eight_exact_stage_hashes_and_no_numerical_authority() {
    let (config, record, b, payload) = fixture();
    let observation =
        ProjectionCaptureObservation::closed(config, record, b, &payload, Vec::new()).unwrap();
    assert_eq!(observation.stages.len(), 28);
    let mut offset = 0;
    for (i, stage) in observation.stages.iter().enumerate() {
        let (name, bytes, width) = wire::STAGES[i % 14];
        assert_eq!(
            (
                stage.rank,
                stage.stage,
                stage.offset,
                stage.bytes,
                stage.elements,
                stage.element_bytes
            ),
            ((i / 14) as u32, name, offset, bytes, bytes / width, width)
        );
        assert_eq!(stage.sha256, hash(&payload[offset..offset + bytes]));
        offset += bytes;
    }
    assert_eq!(offset, wire::CAPTURE_BYTES);
    let value = serde_json::to_value(observation).unwrap();
    assert_eq!(
        value["schema"],
        "FerricFiniteProjectionResidualLayerCaptureObservationV1"
    );
    assert_eq!(value["native_attempts"], 1);
    assert_eq!(value["retries"], 0);
    assert_eq!(value["completed_layers"], 1);
    for field in [
        "paired_comparison_performed",
        "numerical_acceptance",
        "performance_claim",
        "production_authority",
        "full_forward",
    ] {
        assert_eq!(value[field], false, "{field}");
    }
    assert!(value.get("runs").is_none());
    assert!(value.get("bitwise_equal").is_none());
}

#[test]
fn projection_capture_observation_fits_original_summary_reserve() {
    let (config, mut record, b, payload) = fixture();
    let control = record.close.control.as_mut().unwrap();
    control.embedding_ns = [u64::MAX; 2];
    control.paired_ns = [[u64::MAX; 2]; 4];
    record.setup_commands = u64::MAX;
    let files = (0..11)
        .map(|i| evidence::File {
            name: format!("candidate-response-{i}.json"),
            bytes: evidence::LIMIT,
            sha256: [255; 32],
        })
        .collect();
    let observation =
        ProjectionCaptureObservation::closed(config, record, b, &payload, files).unwrap();
    assert!(serde_json::to_vec(&observation).unwrap().len() + 1 < 65536);
}
