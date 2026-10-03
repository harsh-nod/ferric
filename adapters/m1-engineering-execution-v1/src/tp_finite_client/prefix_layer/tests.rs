use super::*;
use crate::finite_prefix_layer_wire_v1::tests::{bootstrap, control};
fn digest(text: &str) -> [u8; 32] {
    core::array::from_fn(|i| u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).unwrap())
}
fn config() -> Config {
    let p = FilePin {
        path: "/task/image".into(),
        bytes: 1,
        sha256: [1; 32],
    };
    Config {
        schema: "FerricFinitePrefixLayerComparisonRequestV1".into(),
        source: "/task/model".into(),
        worker: p.clone(),
        images: ImagePins {
            prefix: p.clone(),
            mlp: p.clone(),
            residual: p.clone(),
            tail: p.clone(),
        },
        expected_bundle_id: [1; 32],
        expected_model_id: [2; 32],
        device_ids: [7, 9],
        session: [3; 32],
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
        prefix_tiles_image: p.clone(),
        mlp_tiles_image: p,
        evidence_directory: "/task/evidence".into(),
        dispatch_timeout_ms: 10000,
        child_deadline_ms: 3600000,
    }
}
#[test]
fn request_is_distinct_closed_and_has_no_cached_or_long_mode() {
    let c = config();
    c.validate().unwrap();
    let raw = serde_json::to_vec(&c).unwrap();
    Config::parse(&raw).unwrap();
    for key in [
        "mode",
        "forwards",
        "kernel_admission",
        "operational_currentness",
    ] {
        let mut value = serde_json::to_value(&c).unwrap();
        value[key] = true.into();
        assert!(Config::parse(&serde_json::to_vec(&value).unwrap()).is_err());
    }
    for which in 0..7 {
        let mut bad = c.clone();
        match which {
            0 => bad.schema = "FerricFiniteTilesDecodeRequestV1".into(),
            1 => bad.device_ids = [7, 7],
            2 => bad.dispatch_timeout_ms = 10001,
            3 => bad.child_deadline_ms = 3600001,
            4 => bad.prefix_tiles_image.bytes = 0,
            5 => bad.mlp_tiles_image.bytes = (32 << 20) + 1,
            _ => bad.prompt.tokens.sha256 = [0; 32],
        };
        assert!(bad.validate().is_err());
    }
    assert!(run(c, false).err().unwrap().contains("opt-in"));
}
#[test]
fn real_fresh_pool_metadata_binds_one_token_and_full_page_permutation() {
    let scope = EngineeringTpPoolScopeV1 {
        model: [2; 32],
        session: [3; 32],
    };
    let limits = EngineeringTpPagedLimitsV1::new(2304, 32, 144, 1024).unwrap();
    let mut pool = EngineeringTpPagedPoolV1::new(scope, limits).unwrap();
    let sequence = pool.open_sequence(scope, &[9112], 0).unwrap();
    let batch = pool
        .reserve_batch(&[EngineeringTpPageRowV1 {
            sequence: sequence.sequence(),
            token: 9112,
            position: 0,
        }])
        .unwrap();
    pool.begin_submission(&batch).unwrap();
    let input = metadata(&batch, [4; 32], 1000000).unwrap();
    input.validate().unwrap();
    assert_eq!(input.cache_metadata[1], batch.rows()[0].physical_pages()[0]);
    assert_eq!(input.token, 9112);
    assert!(metadata(&batch, [4; 32], 999999).is_err());
    pool.quarantine_batch(&batch).unwrap();
}
#[test]
fn all_twenty_eight_payload_rows_compare_bits_with_the_correct_width() {
    let a = vec![0; wire::CAPTURE_BYTES];
    let equal = compare(&a, &a).unwrap();
    assert_eq!(equal.len(), 28);
    assert!(equal.iter().all(|v| v.bit_mismatches == 0));
    let mut offset = 0;
    for i in 0..28 {
        let (_, n, width) = wire::STAGES[i % 14];
        let mut b = a.clone();
        b[offset] = 1;
        let rows = compare(&a, &b).unwrap();
        assert_eq!(rows[i].bit_mismatches, 1);
        assert_eq!(rows[i].elements, n / width);
        assert_eq!(rows[i].element_bytes, width);
        assert_eq!(rows[i].rank, (i / 14) as u32);
        assert_eq!(rows.iter().map(|v| v.bit_mismatches).sum::<usize>(), 1);
        offset += n;
    }
}
#[test]
fn only_current_physical_kv_slot_may_change_on_either_rank() {
    let mut b = bootstrap(wire::Profile::Prefix284Mlp548);
    b.input.cache_metadata.swap(1, 144);
    let raw = vec![0; wire::CAPTURE_BYTES];
    untouched_kv(&b.input, &raw).unwrap();
    let per_rank = wire::CAPTURE_BYTES / 2;
    let prefix_bytes = 8192 + 6144 + 4096;
    let slot = 143 * 16 * 512 * 2;
    for rank in 0..2 {
        for stage in 0..2 {
            let base = rank * per_rank + prefix_bytes + stage * 2359296;
            let mut good = raw.clone();
            good[base + slot] = 1;
            untouched_kv(&b.input, &good).unwrap();
            good[base] = 1;
            assert!(untouched_kv(&b.input, &good).is_err());
        }
    }
}
#[test]
fn capture_nonfinite_and_truncation_are_not_bitwise_acceptance() {
    let a = vec![0; wire::CAPTURE_BYTES];
    let mut bad = a.clone();
    bad[..2].copy_from_slice(&0x7fc0_u16.to_le_bytes());
    assert!(compare(&a, &bad).is_err());
    assert!(compare(&a, &a[..a.len() - 1]).is_err());
}
#[test]
fn close_response_has_exact_own_request_scope_and_terminal_kind() {
    let b = bootstrap(wire::Profile::Prefix284Mlp548);
    let payload = vec![0; wire::CAPTURE_BYTES];
    let request = wire::Request {
        protocol: 1,
        id: 2,
        profile_sha256: b.sha256().unwrap(),
        command: wire::Command::Close,
    };
    let r = wire::Response {
        protocol: 1,
        id: 2,
        profile_sha256: request.profile_sha256,
        profile: b.profile,
        native_closed: true,
        completed_layers: 1,
        control: Some(control(b.profile)),
        capture: Some(wire::part(&payload)),
        gpu_execution: true,
        numerical_acceptance: false,
        performance_claim: false,
        production_authority: false,
    };
    response(&request, &b, &r).unwrap();
    for which in 0..4 {
        let mut bad = r.clone();
        match which {
            0 => bad.profile_sha256 = [9; 32],
            1 => bad.native_closed = false,
            2 => bad.control.as_mut().unwrap().prefix[0].truncate(22),
            _ => bad.performance_claim = true,
        };
        assert!(response(&request, &b, &bad).is_err());
    }
}
#[test]
fn maximum_valid_controls_and_summary_stay_below_header_budget() {
    let mut runs = Vec::new();
    for profile in [
        wire::Profile::Baseline22Mlp548,
        wire::Profile::Prefix284Mlp548,
    ] {
        let b = bootstrap(profile);
        let mut c = control(profile);
        c.embedding_ns = [u64::MAX; 2];
        c.paired_ns = [[u64::MAX; 2]; 4];
        c.validate(profile).unwrap();
        let close = wire::Response {
            protocol: 1,
            id: 2,
            profile_sha256: b.sha256().unwrap(),
            profile,
            native_closed: true,
            completed_layers: 1,
            control: Some(c),
            capture: Some(setup_wire::Part {
                bytes: wire::CAPTURE_BYTES as u32,
                sha256: [255; 32],
            }),
            gpu_execution: true,
            numerical_acceptance: false,
            performance_claim: false,
            production_authority: false,
        };
        assert!(serde_json::to_vec(&close).unwrap().len() < wire::HEADER_LIMIT);
        runs.push(RunRecord {
            child_pid: u32::MAX,
            profile_sha256: b.sha256().unwrap(),
            bootstrap: b,
            setup_commands: u64::MAX,
            close,
            child_exit_zero: true,
            process_group_absent: true,
        });
    }
    let capture = vec![0; wire::CAPTURE_BYTES];
    let observation = Observation {
        schema: "FerricFinitePrefixLayerComparisonObservationV1",
        request: config(),
        runs: runs.try_into().ok().unwrap(),
        stages: compare(&capture, &capture).unwrap(),
        files: (0..22)
            .map(|i| evidence::File {
                name: format!("candidate-response-{i}.json"),
                bytes: evidence::LIMIT,
                sha256: [255; 32],
            })
            .collect(),
        bitwise_equal: true,
        completed_layers_per_run: 1,
        native_closed: true,
        gpu_execution: true,
        numerical_acceptance: false,
        performance_claim: false,
        production_authority: false,
        full_forward: false,
    };
    assert!(serde_json::to_vec(&observation).unwrap().len() + 1 < 65536);
}
