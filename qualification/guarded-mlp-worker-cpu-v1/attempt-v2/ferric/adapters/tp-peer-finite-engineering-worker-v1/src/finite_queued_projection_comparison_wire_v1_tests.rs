use super::*;
use crate::finite_forward_wire_v1::LayerObservation;
use std::io::Cursor;

pub(crate) fn bootstrap() -> Bootstrap {
    Bootstrap {
        protocol: PROTOCOL,
        profile: Profile::FiniteThenQueuedProjectionsLayerZeroV1,
        device_ids: [10, 20],
        scope: Scope {
            bundle_id: [1; 32],
            model_id: [2; 32],
            session: [3; 32],
            pool_identity: 4,
            group_id: 0,
            child_identity: 5,
        },
        timeout_ms: 1000,
        token: TOKEN,
    }
}
pub(crate) fn control() -> Control {
    let mut prefix = [64; 22];
    prefix[..6].copy_from_slice(&[1, 0, 65535, 65535, 0x55555555, 0]);
    let mut mlp = [64; 11];
    mlp[..6].copy_from_slice(&[1, 0, 31, 31, 0x155, 0]);
    Control {
        embedding_ns: [1, 2],
        layers: std::array::from_fn(|_| LayerObservation {
            prefix_states: [prefix; 2],
            mlp_states: [mlp; 2],
            paired_ns: [[3, 4], [5, 6], [7, 8], [9, 10]],
        }),
        tail_ns: [11, 12, 13],
    }
}
pub(crate) fn report(control: &Control) -> ProjectionReport {
    ProjectionReport {
        schema: "FerricFiniteQueuedProjectionComparisonV1".into(),
        generation: 1,
        position: 0,
        layer: 0,
        finite_states: control.layers[0].prefix_states,
        finite_prefix_host_ns: control.layers[0].paired_ns[0],
        queued_stage_host_ns: [[21, 22]; 4],
        equality: std::array::from_fn(|i| {
            std::array::from_fn(|rank| OutputEquality {
                stage: Stage::ALL[i],
                rank: rank as u32,
                bytes: [4096, 1024, 1024, 16384][i],
                words: [2048, 512, 512, 4096][i],
                sha256: [i as u8 + 1; 32],
            })
        }),
        queued_semantic_state: false,
        numerical_acceptance: false,
        performance_claim: false,
        production_authority: false,
    }
}
pub(crate) fn request(close: bool) -> Request {
    let boot = bootstrap();
    Request {
        protocol: PROTOCOL,
        id: if close { 2 } else { 1 },
        device_ids: boot.device_ids,
        session: boot.scope.session,
        registration: [9; 32],
        profile_sha256: boot.sha256().unwrap(),
        command: if close {
            Command::Close
        } else {
            Command::Forward {
                generation: 1,
                token: TOKEN,
                cache_metadata: (0..145).map(|i| if i == 0 { 0 } else { i - 1 }).collect(),
                rotary_bits: vec![0; 128],
            }
        },
    }
}
fn fixture() -> (Response, Control, Vec<u8>, Vec<u8>) {
    let req = request(false);
    let control = control();
    let main = vec![0; OBSERVATION_BYTES];
    let report = serde_json::to_vec(&report(&control)).unwrap();
    let mut c = Completion {
        generation: 1,
        position: 0,
        input_token: TOKEN,
        output_token: 0,
        control: part(&control.encode()),
        observation: part(&main),
        capture: Payload::from_bytes(&main).unwrap(),
        comparison: part(&report),
        chain: [0; 32],
    };
    c.chain = Chain::new(req.registration, req.profile_sha256).advance(&c);
    (
        Response {
            protocol: PROTOCOL,
            id: 1,
            device_ids: req.device_ids,
            session: req.session,
            registration: req.registration,
            profile_sha256: req.profile_sha256,
            event: Event::Completed(c),
            native_closed: false,
            gpu_execution: true,
            numerical_acceptance: false,
            performance_claim: false,
            production_authority: false,
        },
        control,
        main,
        report,
    )
}

#[test]
fn explicit_projection_bootstrap_binds_pid_scope_and_does_not_consume_setup_bytes() {
    let boot = bootstrap();
    boot.validate([10, 20], 1000, 5).unwrap();
    assert_eq!(boot.scope.group_id, 0);
    for change in 0..8 {
        let mut bad = boot.clone();
        match change {
            0 => bad.protocol = 2,
            1 => bad.device_ids = [10, 10],
            2 => bad.scope.child_identity = 6,
            3 => bad.scope.model_id = [0; 32],
            4 => bad.timeout_ms = 1001,
            5 => bad.token += 1,
            6 => bad.scope.pool_identity = 0,
            _ => bad.scope.session = [0; 32],
        }
        assert!(bad.validate([10, 20], 1000, 5).is_err());
    }
    let mut changed = boot.clone();
    changed.scope.session[0] ^= 1;
    assert_ne!(changed.sha256().unwrap(), boot.sha256().unwrap());
    let raw = serde_json::to_string(&boot).unwrap();
    assert!(
        serde_json::from_str::<Bootstrap>(&raw.replace(
            "finite_then_queued_projections_layer_zero_v1",
            "rearm_four_forward_v1"
        ))
        .is_err()
    );
    let mut framed = Vec::new();
    write_bootstrap(&mut framed, &mut FrameBudget::new(), &boot).unwrap();
    framed.extend_from_slice(b"unchanged setup follows");
    let mut reader = framed.as_slice();
    assert_eq!(
        read_bootstrap(&mut reader, &mut FrameBudget::new()).unwrap(),
        Some(boot)
    );
    assert_eq!(reader, b"unchanged setup follows");
    let mut extra = serde_json::to_value(bootstrap()).unwrap();
    extra["norm_image"] = serde_json::json!({"bytes":1,"sha256":[1]});
    assert!(serde_json::from_value::<Bootstrap>(extra).is_err());
}

#[test]
fn only_one_first_forward_and_explicit_close_are_admitted() {
    for close in [false, true] {
        let req = request(close);
        let mut raw = Vec::new();
        write_request(&mut raw, &mut FrameBudget::new(), &req).unwrap();
        assert_eq!(
            read_request(&mut Cursor::new(raw), &mut FrameBudget::new())
                .unwrap()
                .unwrap(),
            req
        );
    }
    for change in 0..7 {
        let mut req = request(false);
        let Command::Forward {
            generation,
            token,
            cache_metadata,
            rotary_bits,
        } = &mut req.command
        else {
            panic!()
        };
        match change {
            0 => *generation = 2,
            1 => *token += 1,
            2 => cache_metadata[0] = 1,
            3 => cache_metadata[2] = cache_metadata[1],
            4 => {
                cache_metadata.pop();
            }
            5 => rotary_bits[0] = f32::NAN.to_bits(),
            _ => {
                rotary_bits.pop();
            }
        }
        assert!(req.validate().is_err());
    }
    let mut req = request(true);
    req.id = 1;
    assert!(req.validate().is_err());
}

#[test]
fn full_main_control_and_comparison_roundtrip_preserves_separate_provenance() {
    let (value, control, main, report) = fixture();
    let mut raw = Vec::new();
    let mut budget = FrameBudget::new();
    write_response(
        &mut raw,
        &mut budget,
        &value,
        Some(&control),
        &main,
        &report,
    )
    .unwrap();
    assert!(raw.len() < 1 << 20);
    let (decoded, capture, body, comparison) =
        read_response(&mut Cursor::new(&raw), &mut FrameBudget::new())
            .unwrap()
            .unwrap();
    assert_eq!(decoded, value);
    assert_eq!(capture.unwrap().encode(), control.encode());
    assert_eq!(body, main);
    assert_eq!(comparison, report);
    let parsed = validate_comparison(&comparison, &control).unwrap();
    assert_eq!(
        parsed
            .equality
            .iter()
            .flatten()
            .map(|row| row.words)
            .sum::<u32>(),
        14336
    );
    assert_eq!(parsed.finite_prefix_host_ns, [3, 4]);
    assert_eq!(parsed.queued_stage_host_ns, [[21, 22]; 4]);
}

#[test]
fn comparison_report_rejects_wrong_finite_state_timing_stage_rank_and_claims() {
    let control = control();
    let good = report(&control);
    for change in 0..13 {
        let mut bad = good.clone();
        match change {
            0 => bad.schema.push('_'),
            1 => bad.generation = 2,
            2 => bad.position = 1,
            3 => bad.layer = 1,
            4 => bad.finite_states[0][4] ^= 3,
            5 => bad.finite_prefix_host_ns[0] += 1,
            6 => bad.equality[1][0].stage = Stage::Value,
            7 => bad.equality[1][0].rank = 1,
            8 => bad.equality[0][0].bytes += 2,
            9 => bad.equality[3][1].words += 1,
            10 => bad.equality[0][0].sha256 = [0; 32],
            11 => bad.queued_semantic_state = true,
            _ => bad.performance_claim = true,
        }
        assert!(bad.validate(&control).is_err());
    }
    let mut raw = serde_json::to_string(&good).unwrap();
    raw.insert_str(1, "\"generation\":1,");
    assert!(validate_comparison(raw.as_bytes(), &control).is_err());
    assert!(validate_comparison(&vec![b' '; COMPARISON_JSON_BYTES + 1], &control).is_err());
    let raw = serde_json::to_string(&good)
        .unwrap()
        .replace("\"layer\":0", "\"layer\":0,\"extra\":false");
    assert!(validate_comparison(raw.as_bytes(), &control).is_err());
}

#[test]
fn corruption_truncation_nonfinite_and_wrong_argmax_are_rejected() {
    let (value, control, main, report) = fixture();
    let mut raw = Vec::new();
    write_response(
        &mut raw,
        &mut FrameBudget::new(),
        &value,
        Some(&control),
        &main,
        &report,
    )
    .unwrap();
    for end in [1, 3, raw.len() - 1] {
        assert!(read_response(&mut Cursor::new(&raw[..end]), &mut FrameBudget::new()).is_err());
    }
    let start = u32::from_le_bytes(raw[..4].try_into().unwrap()) as usize + 4;
    for offset in [
        start,
        start + CONTROL_BYTES,
        start + CONTROL_BYTES + OBSERVATION_BYTES,
    ] {
        let mut bad = raw.clone();
        bad[offset] ^= 1;
        assert!(read_response(&mut Cursor::new(bad), &mut FrameBudget::new()).is_err());
    }
    let Event::Completed(c) = value.event else {
        panic!()
    };
    let mut bad = main.clone();
    bad[..2].copy_from_slice(&0x7f80u16.to_le_bytes());
    let mut changed = c.clone();
    changed.observation = part(&bad);
    changed.capture = Payload::from_bytes(&bad).unwrap();
    assert!(main_payload(&changed, &bad).is_err());
    changed = c;
    changed.output_token = 1;
    assert!(main_payload(&changed, &main).is_err());
}

#[test]
fn chain_binds_comparison_body_and_close_is_not_an_old_profile_completion() {
    let (value, control, main, report) = fixture();
    let Event::Completed(c) = &value.event else {
        panic!()
    };
    let mut changed = c.clone();
    changed.comparison.sha256[0] ^= 1;
    assert_ne!(
        Chain::new(value.registration, value.profile_sha256).advance(c),
        Chain::new(value.registration, value.profile_sha256).advance(&changed)
    );
    let mut close = value.clone();
    close.id = 2;
    close.native_closed = true;
    close.event = Event::Closed {
        completed_forwards: 1,
        transcript_sha256: c.chain,
    };
    let mut raw = Vec::new();
    write_response(&mut raw, &mut FrameBudget::new(), &close, None, &[], &[]).unwrap();
    assert_eq!(
        read_response(&mut Cursor::new(raw), &mut FrameBudget::new())
            .unwrap()
            .unwrap()
            .0,
        close
    );
    assert!(
        write_response(
            &mut Vec::new(),
            &mut FrameBudget::new(),
            &close,
            Some(&control),
            &main,
            &report
        )
        .is_err()
    );
    close.event = Event::Closed {
        completed_forwards: 2,
        transcript_sha256: c.chain,
    };
    assert!(close.body_bytes().is_err());
}

#[test]
fn mlp_profile_and_projection_profile_are_not_interchangeable() {
    use crate::finite_queued_mlp_comparison_wire_v1 as mlp;
    let old = mlp::tests::bootstrap();
    let new = bootstrap();
    assert!(serde_json::from_slice::<Bootstrap>(&serde_json::to_vec(&old).unwrap()).is_err());
    assert!(serde_json::from_slice::<mlp::Bootstrap>(&serde_json::to_vec(&new).unwrap()).is_err());
    assert_ne!(old.sha256().unwrap(), new.sha256().unwrap());
    assert_ne!(
        mlp::Chain::new([9; 32], [10; 32]).digest(),
        Chain::new([9; 32], [10; 32]).digest()
    );
    let control = control();
    let old_report = serde_json::to_vec(&mlp::tests::report(&control)).unwrap();
    let new_report = serde_json::to_vec(&report(&control)).unwrap();
    assert!(validate_comparison(&old_report, &control).is_err());
    assert!(mlp::validate_comparison(&new_report, &control).is_err());
}
