use super::*;
#[test]
fn guarded_reuse_backend_profile_matches_wire_and_cannot_change_default_or_tf4() {
    let old = profile(Mode::Autoregressive { first: 11 });
    let old_hash = old.sha256();
    assert!(!old.reusable_arenas());
    let reused = old.with_reusable_arenas().unwrap();
    assert!(reused.reusable_arenas());
    assert_ne!(reused.sha256(), old_hash);
    assert_eq!(profile(Mode::Autoregressive { first: 11 }).sha256(), old_hash);
    assert!(reused.with_reusable_arenas().is_err());
    assert!(profile(Mode::TeacherForced([11, 12, 13, 14])).with_reusable_arenas().is_err());
    let mut b = crate::finite_guarded_mlp_decode_wire_v1::tests::bootstrap(
        crate::finite_guarded_mlp_decode_wire_v1::InputMode::Autoregressive);
    b.schema = crate::finite_guarded_mlp_decode_wire_v1::REUSE_SCHEMA.into();
    let native = Profile::new(&b.decode.scope, b.decode.registration,
        b.decode.prefix_image.sha256, b.decode.tiles_image.sha256, b.projection_image.sha256,
        Mode::Autoregressive { first: b.decode.input_tokens[0] },
        b.decode.timeout_ms, b.decode.device_ids).unwrap().with_reusable_arenas().unwrap();
    assert_eq!(native.sha256(), b.sha256().unwrap());
}
fn scope() -> Scope {
    Scope {
        bundle_id: [1; 32],
        model_id: [2; 32],
        session: [3; 32],
        pool_identity: 4,
        group_id: 5,
        child_identity: 6,
    }
}
fn profile(mode: Mode) -> Profile {
    Profile::new(
        &scope(),
        [7; 32],
        [8; 32],
        [9; 32],
        [10; 32],
        mode,
        1000,
        [11, 12],
    )
    .unwrap()
}
fn input(position: u32, token: u32) -> ForwardInput {
    ForwardInput {
        registration: [7; 32],
        generation: u64::from(position) + 1,
        token,
        cache_metadata: core::array::from_fn(|i| if i == 0 { position } else { (i - 1) as u32 }),
        rotary_bits: [0; 128],
    }
}
struct Fake {
    events: Vec<String>,
    fail: Option<usize>,
    poisoned: bool,
    output: u32,
    commits: usize,
}
impl Fake {
    fn new() -> Self {
        Self {
            events: Vec::new(),
            fail: None,
            poisoned: false,
            output: 0,
            commits: 0,
        }
    }
    fn step(&mut self, s: impl Into<String>) -> Result<()> {
        let n = self.events.len();
        self.events.push(s.into());
        if self.fail == Some(n) {
            Err("injected route failure".into())
        } else {
            Ok(())
        }
    }
}
impl sequence::Backend for Fake {
    fn metadata(&mut self, _: &ForwardInput) -> Result<()> {
        self.step("metadata")
    }
    fn embedding(&mut self, _: u32) -> Result<[u64; 2]> {
        self.step("embedding")?;
        Ok([1, 2])
    }
    fn begin(&mut self, _: &ForwardInput) -> Result<()> {
        self.step("bank")
    }
    fn layer(&mut self, index: usize) -> Result<layer::Completion> {
        self.step(format!("guarded{index}"))?;
        Ok(layer::Completion {
            prefix_states: [[0; 284]; 2],
            prefix_ns: [3, 4],
            guarded: fe2o3_kfd::Gfx950EngineeringPeerGuardedMlpObservationV1 {
                prefixes: [[0; 548]; 2],
                guards: [[1, 0, 1, 0]; 2],
                observed_queue_frontiers: [(5, 3); 2],
                segment_host_ns: 5,
            },
        })
    }
    fn tail(&mut self) -> Result<(u32, [u64; 3])> {
        self.step("tail")?;
        Ok((self.output, [1, 2, 3]))
    }
    fn fence(&mut self) -> Result<()> {
        self.step("fence")
    }
    fn commit(&mut self) -> Result<()> {
        self.step("commit")?;
        self.commits += 1;
        Ok(())
    }
    fn poison(&mut self) {
        self.poisoned = true;
    }
}
#[test]
fn guarded_model_profile_is_distinct_and_binds_all_images_scope_and_modes() {
    for mode in [
        crate::finite_guarded_mlp_decode_wire_v1::InputMode::TeacherForced,
        crate::finite_guarded_mlp_decode_wire_v1::InputMode::Autoregressive,
    ] {
        let b = crate::finite_guarded_mlp_decode_wire_v1::tests::bootstrap(mode);
        let native_mode = match mode {
            crate::finite_guarded_mlp_decode_wire_v1::InputMode::TeacherForced => {
                Mode::TeacherForced(b.decode.input_tokens.as_slice().try_into().unwrap())
            }
            crate::finite_guarded_mlp_decode_wire_v1::InputMode::Autoregressive => {
                Mode::Autoregressive {
                    first: b.decode.input_tokens[0],
                }
            }
        };
        let native = Profile::new(
            &b.decode.scope,
            b.decode.registration,
            b.decode.prefix_image.sha256,
            b.decode.tiles_image.sha256,
            b.projection_image.sha256,
            native_mode,
            b.decode.timeout_ms,
            b.decode.device_ids,
        )
        .unwrap();
        assert_eq!(native.sha256(), b.sha256().unwrap());
    }
    let mode = Mode::TeacherForced([11, 12, 13, 14]);
    let p = profile(mode);
    let old = super::super::prefix_tiles_decode_v6::Profile::new(
        &scope(),
        [7; 32],
        [8; 32],
        [9; 32],
        mode,
        1000,
        [11, 12],
    )
    .unwrap();
    assert_ne!(p.sha256(), old.sha256());
    assert_ne!(
        p.sha256(),
        profile(Mode::Autoregressive { first: 11 }).sha256()
    );
    for field in 0..8 {
        let mut s = scope();
        let mut hashes = [[7; 32], [8; 32], [9; 32], [10; 32]];
        let mut devices = [11, 12];
        let mut timeout = 1000;
        let mut mode = mode;
        match field {
            0 => s.model_id = [0; 32],
            1..=4 => hashes[field - 1] = [0; 32],
            5 => devices = [11, 11],
            6 => timeout = 10001,
            _ => mode = Mode::Autoregressive { first: 151936 },
        }
        assert!(
            Profile::new(
                &s, hashes[0], hashes[1], hashes[2], hashes[3], mode, timeout, devices
            )
            .is_err()
        );
    }
}
#[test]
fn guarded_model_sequence_runs_all_36_layers_without_a_duplicate_residual_step() {
    for mode in [
        Mode::TeacherForced([11, 12, 13, 14]),
        Mode::Autoregressive { first: 11 },
    ] {
        let p = profile(mode);
        let mut seq = sequence::Sequence::new(&p);
        let mut b = Fake::new();
        let mut token = 11;
        for position in 0..4 {
            if let Mode::TeacherForced(t) = mode {
                token = t[position as usize];
            }
            b.output = 100 + position;
            let c = seq
                .run(&mut b, p.sha256(), &input(position, token))
                .unwrap();
            assert_eq!(c.layers.len(), 36);
            assert_eq!(c.output_token, 100 + position);
            token = c.output_token;
        }
        assert!(seq.exhausted());
        assert_eq!(b.commits, 4);
        assert_eq!(b.events.len(), 4 * 42);
        for chunk in b.events.chunks(42) {
            assert_eq!(&chunk[..3], ["metadata", "embedding", "bank"]);
            for index in 0..36 {
                assert_eq!(chunk[3 + index], format!("guarded{index}"));
            }
            assert_eq!(&chunk[39..], ["tail", "fence", "commit"]);
        }
        assert!(!b.poisoned);
    }
}
#[test]
fn guarded_model_sequence_failure_at_each_stage_prevents_commit_and_retry() {
    let p = profile(Mode::TeacherForced([11, 12, 13, 14]));
    for fail in 0..42 {
        let mut seq = sequence::Sequence::new(&p);
        let mut b = Fake::new();
        b.fail = Some(fail);
        assert!(seq.run(&mut b, p.sha256(), &input(0, 11)).is_err());
        assert!(b.poisoned);
        assert_eq!(b.commits, 0);
        assert_eq!(b.events.len(), fail + 1);
        let before = b.events.len();
        assert!(seq.run(&mut b, p.sha256(), &input(0, 11)).is_err());
        assert_eq!(b.events.len(), before);
    }
}
#[test]
fn guarded_model_sequence_refuses_wrong_scope_token_pages_and_nonfinite_rotary_before_effects() {
    let p = profile(Mode::TeacherForced([11, 12, 13, 14]));
    for case in 0..8 {
        let mut seq = sequence::Sequence::new(&p);
        let mut b = Fake::new();
        let mut v = input(0, 11);
        let mut pin = p.sha256();
        match case {
            0 => pin = [0; 32],
            1 => v.registration = [0; 32],
            2 => v.generation = 2,
            3 => v.cache_metadata[0] = 1,
            4 => v.token = 12,
            5 => v.cache_metadata[1] = 144,
            6 => v.cache_metadata[1] = v.cache_metadata[2],
            _ => v.rotary_bits[0] = f32::NAN.to_bits(),
        }
        assert!(seq.run(&mut b, pin, &v).is_err());
        assert!(b.events.is_empty());
        assert!(b.poisoned);
    }
}
