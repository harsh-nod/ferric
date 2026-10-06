use super::*;

#[test]
fn prefix_decode_device_image_binding_requires_pristine_sequence() {
    let mut sequence = sequence::Sequence::new(&profile(Mode::TeacherForced([11, 12, 13, 14])));
    assert!(sequence.pristine());
    let mut backend = Mock::default();
    sequence
        .run(
            &mut backend,
            profile(Mode::TeacherForced([11, 12, 13, 14])).sha256(),
            &input(0, 11),
        )
        .unwrap();
    assert!(!sequence.pristine());
}

#[test]
fn prefix_decode_device_image_failure_poison_prevents_any_later_backend_call() {
    let p = profile(Mode::TeacherForced([11, 12, 13, 14]));
    let mut sequence = sequence::Sequence::new(&p);
    sequence.poison();
    assert!(!sequence.pristine());
    assert!(!sequence.exhausted());
    let mut backend = Mock::default();
    assert!(
        sequence
            .run(&mut backend, p.sha256(), &input(0, 11))
            .is_err()
    );
    assert!(backend.events.is_empty());
    assert!(backend.poisoned);
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
fn input(position: u32, token: u32) -> ForwardInput {
    ForwardInput {
        registration: [7; 32],
        generation: u64::from(position) + 1,
        token,
        cache_metadata: core::array::from_fn(|i| if i == 0 { position } else { (i - 1) as u32 }),
        rotary_bits: [0; 128],
    }
}
fn profile(mode: Mode) -> Profile {
    Profile::new(&scope(), [7; 32], [8; 32], [9; 32], mode, 1000, [11, 12]).unwrap()
}
#[derive(Default)]
struct Mock {
    events: Vec<String>,
    fail: Option<usize>,
    poisoned: bool,
    output: u32,
    commits: usize,
}
impl Mock {
    fn step(&mut self, s: impl Into<String>) -> Result<()> {
        let n = self.events.len();
        self.events.push(s.into());
        if self.fail == Some(n) {
            Err("injected".into())
        } else {
            Ok(())
        }
    }
}
impl sequence::Backend for Mock {
    fn metadata(&mut self, _: &ForwardInput) -> Result<()> {
        self.step("metadata")
    }
    fn embedding(&mut self, _: u32) -> Result<[u64; 2]> {
        self.step("embedding")?;
        Ok([1, 2])
    }
    fn begin(&mut self, _: &ForwardInput) -> Result<()> {
        self.step("begin")
    }
    fn layer(&mut self, index: usize) -> Result<layer::Completion> {
        self.step(format!("layer{index}"))?;
        Ok(layer::Completion {
            prefix_states: [[284; 284]; 2],
            mlp_states: [[548; 548]; 2],
            timing: crate::resident_layer::prefix_tiles_decode_v6::Timing::Paired(
                [[index as u64; 2]; 4],
            ),
        })
    }
    fn tail(&mut self) -> Result<(u32, [u64; 3])> {
        self.step("tail")?;
        Ok((self.output, [3, 4, 5]))
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
fn prefix_decode_profile_binds_scope_devices_both_images_mode_and_timeout() {
    let original = profile(Mode::TeacherForced([9112, 67, 25, 576]));
    for change in 0..13 {
        let mut s = scope();
        let mut reg = [7; 32];
        let mut prefix = [8; 32];
        let mut mlp = [9; 32];
        let mut mode = Mode::TeacherForced([9112, 67, 25, 576]);
        let mut timeout = 1000;
        let mut devices = [11, 12];
        match change {
            0 => s.bundle_id[0] ^= 1,
            1 => s.model_id[0] ^= 1,
            2 => s.session[0] ^= 1,
            3 => s.pool_identity += 1,
            4 => s.group_id += 1,
            5 => s.child_identity += 1,
            6 => reg[0] ^= 1,
            7 => prefix[0] ^= 1,
            8 => mlp[0] ^= 1,
            9 => mode = Mode::TeacherForced([9112, 67, 25, 577]),
            10 => mode = Mode::Autoregressive { first: 9112 },
            11 => timeout += 1,
            _ => devices.swap(0, 1),
        }
        assert_ne!(
            original.sha256(),
            Profile::new(&s, reg, prefix, mlp, mode, timeout, devices)
                .unwrap()
                .sha256()
        );
    }
    let layer0 = super::super::prefix_tiles_layer_v6::Profile::new(
        &scope(),
        [7; 32],
        Some([8; 32]),
        [9; 32],
        &input(0, 9112),
        1000,
        [11, 12],
    )
    .unwrap();
    assert_ne!(original.sha256(), layer0.sha256());
    let old = super::super::tiles_decode_v1::Profile::new(
        &scope(),
        [7; 32],
        [9; 32],
        super::super::tiles_decode_v1::Mode::TeacherForced([9112, 67, 25, 576]),
        1000,
        [11, 12],
    )
    .unwrap();
    assert_ne!(original.sha256(), old.sha256());
}
#[test]
fn prefix_decode_profile_rejects_missing_scope_images_devices_and_invalid_modes() {
    for change in 0..12 {
        let mut s = scope();
        let mut reg = [7; 32];
        let mut prefix = [8; 32];
        let mut mlp = [9; 32];
        let mut mode = Mode::Autoregressive { first: 9112 };
        let mut timeout = 1000;
        let mut devices = [11, 12];
        match change {
            0 => s.bundle_id = [0; 32],
            1 => s.model_id = [0; 32],
            2 => s.session = [0; 32],
            3 => s.pool_identity = 0,
            4 => s.child_identity = 0,
            5 => reg = [0; 32],
            6 => prefix = [0; 32],
            7 => mlp = [0; 32],
            8 => mode = Mode::Autoregressive { first: 151936 },
            9 => timeout = 0,
            10 => timeout = 10001,
            _ => devices = [11, 11],
        }
        assert!(Profile::new(&s, reg, prefix, mlp, mode, timeout, devices).is_err());
    }
    assert!(
        Profile::new(
            &scope(),
            [7; 32],
            [8; 32],
            [9; 32],
            Mode::TeacherForced([1, 2, 3, u32::MAX]),
            1000,
            [0, 12]
        )
        .is_err()
    );
}
#[test]
fn prefix_decode_sequence_runs_all36_layers_tail_fence_and_commit_four_times() {
    let tokens = [9112, 67, 25, 576];
    let p = profile(Mode::TeacherForced(tokens));
    let mut s = sequence::Sequence::new(&p);
    let mut b = Mock::default();
    for position in 0..4 {
        b.events.clear();
        b.output = 100 + position;
        let c = s
            .run(
                &mut b,
                p.sha256(),
                &input(position, tokens[position as usize]),
            )
            .unwrap();
        assert_eq!(
            (c.generation, c.position, c.input_token, c.output_token),
            (
                u64::from(position) + 1,
                position,
                tokens[position as usize],
                100 + position
            )
        );
        assert_eq!(c.profile_sha256, p.sha256());
        assert_eq!(c.layers.len(), 36);
        let expected = std::iter::once("metadata".to_string())
            .chain(["embedding".into(), "begin".into()])
            .chain((0..36).map(|i| format!("layer{i}")))
            .chain(["tail".into(), "fence".into(), "commit".into()])
            .collect::<Vec<_>>();
        assert_eq!(b.events, expected);
        assert_eq!(b.commits, position as usize + 1);
    }
    assert!(s.exhausted());
    assert!(!b.poisoned);
    let before = b.events.len();
    assert!(s.run(&mut b, p.sha256(), &input(4, 0)).is_err());
    assert_eq!(b.events.len(), before);
    assert!(b.poisoned);
}
#[test]
fn prefix_decode_autoregressive_uses_only_its_own_committed_outputs() {
    let p = profile(Mode::Autoregressive { first: 9112 });
    let mut s = sequence::Sequence::new(&p);
    let mut b = Mock::default();
    for (position, token) in [9112, 67, 25, 576].into_iter().enumerate() {
        b.output = [67, 25, 576, 2701][position];
        s.run(&mut b, p.sha256(), &input(position as u32, token))
            .unwrap();
    }
    assert!(s.exhausted());
    let mut s = sequence::Sequence::new(&p);
    let mut b = Mock {
        output: 67,
        ..Mock::default()
    };
    s.run(&mut b, p.sha256(), &input(0, 9112)).unwrap();
    b.events.clear();
    assert!(s.run(&mut b, p.sha256(), &input(1, 25)).is_err());
    assert!(b.events.is_empty());
    assert!(b.poisoned);
}
#[test]
fn prefix_decode_bad_identity_position_pages_and_rotary_refuse_before_metadata() {
    let p = profile(Mode::TeacherForced([9112, 67, 25, 576]));
    for change in 0..8 {
        let mut s = sequence::Sequence::new(&p);
        let mut b = Mock::default();
        let mut i = input(0, 9112);
        let mut hash = p.sha256();
        match change {
            0 => hash[0] ^= 1,
            1 => i.registration[0] ^= 1,
            2 => i.generation = 2,
            3 => i.cache_metadata[0] = 1,
            4 => i.token = 151936,
            5 => i.rotary_bits[127] = f32::NAN.to_bits(),
            6 => i.cache_metadata[144] = 144,
            _ => i.cache_metadata[2] = 0,
        }
        assert!(s.run(&mut b, hash, &i).is_err());
        assert!(b.events.is_empty());
        assert!(b.poisoned);
        assert!(s.run(&mut b, p.sha256(), &input(0, 9112)).is_err());
        assert!(b.events.is_empty());
    }
    let mut s = sequence::Sequence::new(&p);
    let mut b = Mock::default();
    s.run(&mut b, p.sha256(), &input(0, 9112)).unwrap();
    b.events.clear();
    let mut i = input(1, 67);
    i.cache_metadata.swap(1, 2);
    assert!(s.run(&mut b, p.sha256(), &i).is_err());
    assert!(b.events.is_empty());
}
#[test]
fn prefix_decode_every_forward_failure_poisoned_before_publication_or_retry() {
    let p = profile(Mode::Autoregressive { first: 9112 });
    for fail in 0..42 {
        let mut s = sequence::Sequence::new(&p);
        let mut b = Mock {
            fail: Some(fail),
            ..Mock::default()
        };
        assert!(s.run(&mut b, p.sha256(), &input(0, 9112)).is_err());
        assert_eq!(b.events.len(), fail + 1);
        assert_eq!(b.commits, 0);
        assert!(b.poisoned);
        assert!(!s.exhausted());
        let before = b.events.len();
        b.fail = None;
        assert!(s.run(&mut b, p.sha256(), &input(0, 9112)).is_err());
        assert_eq!(b.events.len(), before);
    }
    let mut s = sequence::Sequence::new(&p);
    let mut b = Mock {
        output: 151936,
        ..Mock::default()
    };
    assert!(s.run(&mut b, p.sha256(), &input(0, 9112)).is_err());
    assert_eq!(b.events.last().map(String::as_str), Some("tail"));
    assert_eq!(b.commits, 0);
}
#[test]
fn prefix_decode_close_requires_both_complete_frontiers() {
    for count in 0..6 {
        for exhausted in [false, true] {
            for between in [false, true] {
                assert_eq!(
                    close_ready(exhausted, between, count).is_ok(),
                    exhausted && between && count == 4
                );
            }
        }
    }
}
#[test]
fn prefix_decode_reuses_full_finite_bf16_logits_and_lowest_index_tie_guard() {
    assert_eq!(36 * 8192 + 8192 + 151936 * 2, 606976);
    let mut logits = vec![0; 151936 * 2];
    for index in [7, 9] {
        logits[index * 2..index * 2 + 2].copy_from_slice(&0x3f80u16.to_le_bytes());
    }
    super::super::checked_argmax(&logits, 7).unwrap();
    assert!(super::super::checked_argmax(&logits, 9).is_err());
    logits[151935 * 2..].copy_from_slice(&0x7f80u16.to_le_bytes());
    assert!(super::super::checked_argmax(&logits, 7).is_err());
}
