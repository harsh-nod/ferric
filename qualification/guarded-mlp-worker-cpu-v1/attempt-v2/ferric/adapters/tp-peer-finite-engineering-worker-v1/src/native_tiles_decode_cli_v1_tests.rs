use super::*;
use crate::native_catalog::forward::tiles_decode_v1::{
    Completion as ActualCompletion, Profile as ActualProfile,
};
use crate::resident_layer::tiles_decode_v1::Completion as LayerCompletion;
fn args(mode: InputMode) -> Vec<OsString> {
    [
        "--engineering-native-tiles-decode-v1",
        "--allow-unauthenticated-machine-code",
        "--devices",
        "11,22",
        "--timeout-ms",
        "1000",
        "--mode",
        if mode == InputMode::TeacherForced {
            "teacher-forced"
        } else {
            "autoregressive"
        },
    ]
    .into_iter()
    .map(OsString::from)
    .collect()
}
fn bootstrap(mode: InputMode) -> Bootstrap {
    Bootstrap {
        protocol: 1,
        profile: wire::Profile::TilesDecodeFourForwardV1,
        device_ids: [11, 22],
        timeout_ms: 1000,
        scope: setup::Scope {
            bundle_id: [1; 32],
            model_id: [2; 32],
            session: [3; 32],
            pool_identity: 1,
            group_id: 0,
            child_identity: std::process::id(),
        },
        registration: [4; 32],
        mode,
        input_tokens: if mode == InputMode::TeacherForced {
            vec![9112, 2190, 3772, 220]
        } else {
            vec![9112]
        },
        tiles_image: part(&[1]),
    }
}
fn request(b: &Bootstrap, id: u64) -> wire::Request {
    wire::Request {
        protocol: 1,
        id,
        device_ids: b.device_ids,
        session: b.scope.session,
        registration: b.registration,
        profile_sha256: b.sha256().unwrap(),
        command: if id == 5 {
            Command::Close
        } else {
            Command::Forward {
                generation: id,
                token: b.input(id as u32 - 1, Some(0)).unwrap(),
                cache_metadata: core::iter::once(id as u32 - 1).chain(0..144).collect(),
                rotary_bits: vec![0; 128],
            }
        },
    }
}
fn states() -> ([u32; 22], [u32; 548]) {
    let mut prefix = [64; 22];
    prefix[..6].copy_from_slice(&[1, 0, 65535, 65535, 0x55555555, 0]);
    let mut tiles = [0; 548];
    tiles[..4].copy_from_slice(&[1, 0, 0, 31]);
    tiles[4..9].copy_from_slice(&[1, 96, 96, 1, 64]);
    tiles[9..14].copy_from_slice(&[1, 96, 96, 1, 64]);
    tiles[14..23].fill(u32::MAX);
    tiles[22] = 3;
    tiles[23..32].fill(u32::MAX);
    tiles[31] = 3;
    tiles[32..290].fill(1);
    tiles[290..].fill(64);
    (prefix, tiles)
}
struct Fake {
    profile: [u8; 32],
    inputs: Vec<u32>,
    closes: usize,
    fail: Option<u64>,
    bad_state: bool,
    close_fail: bool,
    applied_admission: Option<wire::AdmissionReceipt>,
}
impl Fake {
    fn new(b: &Bootstrap) -> Self {
        Self {
            profile: b.sha256().unwrap(),
            inputs: vec![],
            closes: 0,
            fail: None,
            bad_state: false,
            close_fail: false,
            applied_admission: b.profile.admission().receipt(),
        }
    }
}
impl Backend for Fake {
    fn applied_admission(&self) -> io::Result<Option<wire::AdmissionReceipt>> {
        Ok(self.applied_admission)
    }
    fn run(&mut self, input: &ForwardInput) -> io::Result<Run> {
        self.inputs.push(input.token);
        if self.fail == Some(input.generation) {
            return Err(io::Error::other("injected run"));
        }
        let (prefix, tiles) = states();
        let mut layers: Vec<_> = (0..36)
            .map(|_| LayerCompletion {
                prefix_states: [prefix; 2],
                tiles_states: [tiles; 2],
                paired_ns: [[1; 2]; 4],
            })
            .collect();
        if self.bad_state {
            layers[35].tiles_states[1][547] = 63;
        }
        Ok(Run {
            completion: ActualCompletion {
                profile_sha256: self.profile,
                generation: input.generation,
                position: input.cache_metadata[0],
                input_token: input.token,
                output_token: 0,
                embedding_ns: [1; 2],
                layers,
                tail_ns: [2; 3],
            },
            layer_hidden: vec![vec![0; 8192]; 36],
            final_normalized: vec![0; 8192],
            logits: vec![0; 303872],
        })
    }
    fn close(&mut self) -> io::Result<()> {
        self.closes += 1;
        if self.close_fail {
            Err(io::Error::other("injected close"))
        } else {
            Ok(())
        }
    }
}
fn input(b: &Bootstrap, count: u64) -> Vec<u8> {
    let mut bytes = Vec::new();
    for id in 1..=count {
        wire::write_request(&mut bytes, &mut FrameBudget::new(), &request(b, id)).unwrap();
    }
    bytes
}

#[test]
fn tiles_cached_cli_requires_exact_option_and_matching_bootstrap_before_begin() {
    for m in [InputMode::TeacherForced, InputMode::Autoregressive] {
        let mut cached_args = args(m);
        cached_args.extend(["--kernel-admission", "cached-immutable"].map(OsString::from));
        assert_eq!(
            parse_args(&cached_args).unwrap().admission,
            wire::KernelAdmission::CachedImmutable
        );
        for bad in ["baseline", "operational", "shared-full", "true", ""] {
            cached_args[9] = bad.into();
            assert!(parse_args(&cached_args).is_err());
        }
        cached_args[9] = "cached-immutable".into();
        let baseline_args = args(m);
        for requested in [
            wire::KernelAdmission::Baseline,
            wire::KernelAdmission::CachedImmutable,
        ] {
            let mut b = bootstrap(m);
            b.profile = requested.profile();
            let options = parse_args(if requested.is_baseline() {
                &cached_args
            } else {
                &baseline_args
            })
            .unwrap();
            let mut bytes = Vec::new();
            wire::write_bootstrap(&mut bytes, &mut FrameBudget::new(), &b, &[1]).unwrap();
            assert!(
                prepare(&options, &mut bytes.as_slice(), &mut FrameBudget::new())
                    .err()
                    .unwrap()
                    .to_string()
                    .contains("admission differs before open")
            );
        }
        let mut b = bootstrap(m);
        b.profile = wire::KernelAdmission::CachedImmutable.profile();
        let actual = ActualProfile::new_with_admission(
            &b.scope,
            b.registration,
            b.tiles_image.sha256,
            mode(&b).unwrap(),
            b.timeout_ms,
            b.device_ids,
            wire::KernelAdmission::CachedImmutable,
        )
        .unwrap();
        assert_eq!(actual.sha256(), b.sha256().unwrap());
    }
}

#[test]
fn tiles_cached_protocol_records_applied_policy_on_four_frames_and_close() {
    for m in [InputMode::TeacherForced, InputMode::Autoregressive] {
        let mut b = bootstrap(m);
        b.profile = wire::KernelAdmission::CachedImmutable.profile();
        let mut fake = Fake::new(&b);
        let mut output = Vec::new();
        serve(
            &mut fake,
            &mut input(&b, 5).as_slice(),
            &mut output,
            &b,
            &mut FrameBudget::new(),
        )
        .unwrap();
        assert_eq!(fake.inputs.len(), 4);
        assert_eq!(fake.closes, 1);
        let mut raw = output.as_slice();
        let mut budget = FrameBudget::new();
        for id in 1..=5 {
            let (r, c, bytes) = wire::read_response(&mut raw, &mut budget).unwrap().unwrap();
            assert_eq!(r.id, id);
            assert_eq!(
                r.applied_admission,
                wire::KernelAdmission::CachedImmutable.receipt()
            );
            assert_eq!(c.is_some(), id < 5);
            assert_eq!(bytes.len(), if id < 5 { OBSERVATION_BYTES } else { 0 });
        }
        assert!(raw.is_empty());
    }
}

#[test]
fn tiles_protocol_refuses_missing_extra_or_weakened_applied_policy_before_work() {
    for admission in [
        wire::KernelAdmission::Baseline,
        wire::KernelAdmission::CachedImmutable,
    ] {
        let mut b = bootstrap(InputMode::TeacherForced);
        b.profile = admission.profile();
        let opposite = if admission.is_baseline() {
            wire::KernelAdmission::CachedImmutable.receipt()
        } else {
            None
        };
        let mut weakened = wire::KernelAdmission::CachedImmutable.receipt().unwrap();
        weakened.operational_currentness = true;
        for receipt in [opposite, Some(weakened)] {
            let mut fake = Fake::new(&b);
            fake.applied_admission = receipt;
            let mut out = Vec::new();
            assert!(
                serve(
                    &mut fake,
                    &mut input(&b, 5).as_slice(),
                    &mut out,
                    &b,
                    &mut FrameBudget::new()
                )
                .is_err()
            );
            assert!(fake.inputs.is_empty() && out.is_empty());
            assert_eq!(fake.closes, 0);
        }
    }
}
#[test]
fn tiles_cli_exact_arguments_exclude_other_profiles() {
    for m in [InputMode::TeacherForced, InputMode::Autoregressive] {
        assert_eq!(
            parse_args(&args(m)).unwrap(),
            NativeOptions {
                devices: [11, 22],
                timeout_ms: 1000,
                mode: m,
                admission: wire::KernelAdmission::Baseline,
            }
        );
        assert!(crate::native_rearm_smoke_cli_v1::parse_args(&args(m)).is_err());
        assert!(crate::native_cli_v1::parse_args(&args(m)).is_err());
    }
    for (i, v) in [
        (0, "--engineering-native-rearm-smoke-v1"),
        (1, "--missing"),
        (3, "11,11"),
        (5, "10001"),
        (7, "automatic"),
    ] {
        let mut bad = args(InputMode::TeacherForced);
        bad[i] = v.into();
        assert!(parse_args(&bad).is_err());
    }
    let mut bad = args(InputMode::TeacherForced);
    bad.push("--capture-layer0".into());
    assert!(parse_args(&bad).is_err());
}
#[test]
fn tiles_bootstrap_hash_exactly_matches_real_backend_profile_for_both_modes() {
    for m in [InputMode::TeacherForced, InputMode::Autoregressive] {
        let b = bootstrap(m);
        let actual = ActualProfile::new(
            &b.scope,
            b.registration,
            b.tiles_image.sha256,
            mode(&b).unwrap(),
            b.timeout_ms,
            b.device_ids,
        )
        .unwrap();
        assert_eq!(actual.sha256(), b.sha256().unwrap());
    }
}
#[test]
fn tiles_prepare_rejects_mode_or_pid_before_begin_and_open() {
    for mismatch in 0..2 {
        let mut b = bootstrap(InputMode::TeacherForced);
        if mismatch == 0 {
            b.scope.child_identity += 1;
        }
        let options = parse_args(&args(if mismatch == 1 {
            InputMode::Autoregressive
        } else {
            InputMode::TeacherForced
        }))
        .unwrap();
        let mut bytes = Vec::new();
        wire::write_bootstrap(&mut bytes, &mut FrameBudget::new(), &b, &[1]).unwrap();
        assert!(
            prepare(&options, &mut bytes.as_slice(), &mut FrameBudget::new())
                .err()
                .unwrap()
                .to_string()
                .contains("bootstrap")
        );
    }
}
#[test]
fn tiles_prepare_requires_same_registration_and_complete_tail_before_open() {
    for registration_mismatch in [false, true] {
        let mut b = bootstrap(InputMode::TeacherForced);
        let pin = part(b"x");
        b.registration = pin.sha256;
        if registration_mismatch {
            b.registration[0] ^= 1;
        }
        let begin = setup::Begin {
            scope: b.scope.clone(),
            registration: pin,
            source_program: pin,
            uploads: pin,
            prefix_image: pin,
            mlp_image: pin,
            residual_image: pin,
            tail_image: registration_mismatch.then_some(pin),
        };
        let request = setup::Request {
            protocol: 1,
            id: 1,
            device_ids: b.device_ids,
            session: b.scope.session,
            command: setup::Command::Begin(begin),
        };
        let mut bytes = Vec::new();
        wire::write_bootstrap(&mut bytes, &mut FrameBudget::new(), &b, &[1]).unwrap();
        setup::write_request(
            &mut bytes,
            &request,
            if registration_mismatch {
                b"xxxxxxx"
            } else {
                b"xxxxxx"
            },
        )
        .unwrap();
        let error = prepare(
            &parse_args(&args(b.mode)).unwrap(),
            &mut bytes.as_slice(),
            &mut FrameBudget::new(),
        )
        .err()
        .unwrap();
        assert!(
            error
                .to_string()
                .contains("registration/tail differs before open")
        );
    }
}
#[test]
fn tiles_protocol_four_actual_tf_or_own_ar_inputs_then_native_close() {
    for m in [InputMode::TeacherForced, InputMode::Autoregressive] {
        let b = bootstrap(m);
        let mut fake = Fake::new(&b);
        let mut output = Vec::new();
        serve(
            &mut fake,
            &mut input(&b, 5).as_slice(),
            &mut output,
            &b,
            &mut FrameBudget::new(),
        )
        .unwrap();
        assert_eq!(
            fake.inputs,
            if m == InputMode::TeacherForced {
                vec![9112, 2190, 3772, 220]
            } else {
                vec![9112, 0, 0, 0]
            }
        );
        assert_eq!(fake.closes, 1);
        let mut reader = output.as_slice();
        let mut budget = FrameBudget::new();
        let mut chain = Chain::new(b.registration, b.sha256().unwrap());
        for id in 1..=4 {
            let (r, c, p) = wire::read_response(&mut reader, &mut budget)
                .unwrap()
                .unwrap();
            assert_eq!(r.id, id);
            assert_eq!(c.unwrap().encode().len(), 166504);
            assert_eq!(p.len(), 606976);
            let Event::Completed(v) = r.event else {
                unreachable!()
            };
            assert_eq!(chain.advance(&v), v.chain);
        }
        let (r, c, p) = wire::read_response(&mut reader, &mut budget)
            .unwrap()
            .unwrap();
        assert_eq!(
            r.event,
            Event::Closed {
                completed_forwards: 4,
                transcript_sha256: chain.digest()
            }
        );
        assert!(r.native_closed && c.is_none() && p.is_empty() && reader.is_empty());
    }
}
#[test]
fn tiles_wrong_trajectory_or_identity_stops_before_second_dispatch() {
    for m in [InputMode::TeacherForced, InputMode::Autoregressive] {
        for identity in [false, true] {
            let b = bootstrap(m);
            let mut bytes = input(&b, 1);
            let mut r = request(&b, 2);
            if identity {
                r.profile_sha256[0] ^= 1;
            } else if let Command::Forward { token, .. } = &mut r.command {
                *token = if m == InputMode::TeacherForced {
                    0
                } else {
                    2190
                };
            }
            wire::write_request(&mut bytes, &mut FrameBudget::new(), &r).unwrap();
            let mut fake = Fake::new(&b);
            assert!(
                serve(
                    &mut fake,
                    &mut bytes.as_slice(),
                    &mut Vec::new(),
                    &b,
                    &mut FrameBudget::new()
                )
                .is_err()
            );
            assert_eq!(fake.inputs, [9112]);
            assert_eq!(fake.closes, 0);
        }
    }
}
#[test]
fn tiles_each_forward_failure_missing_close_or_close_failure_never_retries() {
    for failure in 1..=6 {
        let b = bootstrap(InputMode::TeacherForced);
        let mut fake = Fake::new(&b);
        fake.fail = (failure <= 4).then_some(failure);
        fake.close_fail = failure == 6;
        let bytes = input(&b, if failure == 5 { 4 } else { 5 });
        let mut output = Vec::new();
        assert!(
            serve(
                &mut fake,
                &mut bytes.as_slice(),
                &mut output,
                &b,
                &mut FrameBudget::new()
            )
            .is_err()
        );
        assert_eq!(fake.inputs.len(), failure.min(4) as usize);
        assert_eq!(fake.closes, usize::from(failure == 6));
        let mut reader = output.as_slice();
        let mut budget = FrameBudget::new();
        while let Some((r, _, _)) = wire::read_response(&mut reader, &mut budget).unwrap() {
            assert!(!r.native_closed);
        }
    }
}
#[test]
fn tiles_late_state_or_output_write_loss_blocks_following_calls() {
    let b = bootstrap(InputMode::TeacherForced);
    let mut fake = Fake::new(&b);
    fake.bad_state = true;
    let mut output = Vec::new();
    assert!(
        serve(
            &mut fake,
            &mut input(&b, 5).as_slice(),
            &mut output,
            &b,
            &mut FrameBudget::new()
        )
        .is_err()
    );
    assert!(output.is_empty());
    assert_eq!(fake.inputs.len(), 1);
    assert_eq!(fake.closes, 0);
    struct Lost;
    impl Write for Lost {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("write lost"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut fake = Fake::new(&b);
    assert!(
        serve(
            &mut fake,
            &mut input(&b, 5).as_slice(),
            &mut Lost,
            &b,
            &mut FrameBudget::new()
        )
        .is_err()
    );
    assert_eq!(fake.inputs.len(), 1);
    assert_eq!(fake.closes, 0);
}
