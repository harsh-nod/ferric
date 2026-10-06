use super::*;

#[test]
fn cached_admission_requires_one_successful_cache_only_configuration() {
    for mode in [
        Mode::TeacherForced([9112, 2190, 3772, 220]),
        Mode::Autoregressive { first: 9112 },
    ] {
        let mut baseline = profile(mode);
        assert!(baseline.applied_admission().is_err());
        baseline
            .configure_admission(|_, _| panic!("baseline configured native performance"))
            .unwrap();
        assert_eq!(baseline.applied_admission().unwrap(), None);
        assert!(baseline.configure_admission(|_, _| Ok(())).is_err());
        let mut cached = Profile::new_with_admission(
            &scope(),
            [6; 32],
            [7; 32],
            mode,
            1000,
            [101, 102],
            KernelAdmission::CachedImmutable,
        )
        .unwrap();
        assert_ne!(baseline.sha256(), cached.sha256());
        assert!(cached.applied_admission().is_err());
        let mut calls = 0;
        cached
            .configure_admission(|cache, operational| {
                calls += 1;
                assert!(cache);
                assert!(!operational);
                Ok(())
            })
            .unwrap();
        assert_eq!(calls, 1);
        assert_eq!(
            cached.applied_admission().unwrap(),
            KernelAdmission::CachedImmutable.receipt()
        );
        assert!(
            cached
                .configure_admission(|_, _| panic!("reconfigured"))
                .is_err()
        );
    }
}

#[test]
fn failed_cached_admission_has_no_witness_or_retry() {
    let mut p = Profile::new_with_admission(
        &scope(),
        [6; 32],
        [7; 32],
        Mode::Autoregressive { first: 9112 },
        1000,
        [101, 102],
        KernelAdmission::CachedImmutable,
    )
    .unwrap();
    assert!(
        p.configure_admission(|_, _| Err("injected full-currentness/configuration failure".into()))
            .is_err()
    );
    assert!(p.applied_admission().is_err());
    assert!(
        p.configure_admission(|_, _| panic!("retried after native failure"))
            .is_err()
    );
}
fn scope() -> Scope {
    Scope {
        bundle_id: [1; 32],
        model_id: [2; 32],
        session: [3; 32],
        pool_identity: 4,
        group_id: 0,
        child_identity: 5,
    }
}
fn profile(mode: Mode) -> Profile {
    Profile::new(&scope(), [6; 32], [7; 32], mode, 1000, [101, 102]).unwrap()
}
fn input(g: u64, token: u32) -> ForwardInput {
    ForwardInput {
        registration: [6; 32],
        generation: g,
        token,
        cache_metadata: core::array::from_fn(|i| {
            if i == 0 {
                g as u32 - 1
            } else {
                143 - (i as u32 - 1)
            }
        }),
        rotary_bits: [0; 128],
    }
}
struct Fake {
    events: Vec<String>,
    fail: Option<usize>,
    poisoned: bool,
    output: u32,
}
impl Fake {
    fn new() -> Self {
        Self {
            events: vec![],
            fail: None,
            poisoned: false,
            output: 67,
        }
    }
    fn step(&mut self, s: String) -> Result<()> {
        let n = self.events.len();
        self.events.push(s);
        if self.fail == Some(n) {
            Err("native cutoff".into())
        } else {
            Ok(())
        }
    }
}
impl sequence::Backend for Fake {
    fn metadata(&mut self, _: &ForwardInput) -> Result<()> {
        self.step("metadata".into())
    }
    fn embedding(&mut self, t: u32) -> Result<[u64; 2]> {
        self.step(format!("embedding{t}"))?;
        Ok([1, 2])
    }
    fn begin(&mut self, _: &ForwardInput) -> Result<()> {
        self.step("begin".into())
    }
    fn layer(&mut self, i: usize) -> Result<layer::Completion> {
        self.step(format!("layer{i}"))?;
        Ok(layer::Completion {
            prefix_states: [[22; 22]; 2],
            tiles_states: [[548; 548]; 2],
            paired_ns: [[0; 2]; 4],
        })
    }
    fn tail(&mut self) -> Result<(u32, [u64; 3])> {
        self.step("tail".into())?;
        Ok((self.output, [3, 4, 5]))
    }
    fn fence(&mut self) -> Result<()> {
        self.step("fence".into())
    }
    fn commit(&mut self) -> Result<()> {
        self.step("commit".into())
    }
    fn poison(&mut self) {
        self.poisoned = true;
    }
}
#[test]
fn four_teacher_forced_forwards_retain_distinct_complete_v2_states_and_phase_order() {
    let tokens = [9112, 2190, 3772, 220];
    let p = profile(Mode::TeacherForced(tokens));
    let mut s = sequence::Sequence::new(&p);
    let mut b = Fake::new();
    for (position, token) in tokens.into_iter().enumerate() {
        b.events.clear();
        assert!(!s.exhausted());
        let result = s
            .run(&mut b, p.sha256(), &input(position as u64 + 1, token))
            .unwrap();
        assert_eq!(result.profile_sha256, p.sha256());
        assert_eq!(result.layers.len(), 36);
        assert_eq!(result.layers[35].tiles_states, [[548; 548]; 2]);
        assert_eq!(result.position, position as u32);
        assert_eq!(result.input_token, token);
        assert_eq!(b.events.len(), 42);
        assert_eq!(b.events[0], "metadata");
        assert_eq!(b.events[2], "begin");
        assert_eq!(b.events[3], "layer0");
        assert_eq!(b.events[38], "layer35");
        assert_eq!(&b.events[39..], ["tail", "fence", "commit"]);
    }
    assert!(s.exhausted());
    let events = b.events.len();
    assert!(s.run(&mut b, p.sha256(), &input(5, 67)).is_err());
    assert_eq!(events, b.events.len());
    assert!(!s.exhausted());
}
#[test]
fn autoregressive_input_must_be_own_checked_previous_output_not_prompt_substitution() {
    let p = profile(Mode::Autoregressive { first: 9112 });
    let mut s = sequence::Sequence::new(&p);
    let mut b = Fake::new();
    let mut token = 9112;
    for g in 1..=4 {
        b.output = 60 + g as u32;
        let done = s.run(&mut b, p.sha256(), &input(g, token)).unwrap();
        token = done.output_token;
    }
    assert!(s.exhausted());
    let mut s = sequence::Sequence::new(&p);
    let mut b = Fake::new();
    s.run(&mut b, p.sha256(), &input(1, 9112)).unwrap();
    let n = b.events.len();
    assert!(s.run(&mut b, p.sha256(), &input(2, 2190)).is_err());
    assert_eq!(b.events.len(), n);
}
#[test]
fn any_native_cutoff_including_tail_fence_and_commit_permanently_withholds_completion() {
    for n in 0..42 {
        let p = profile(Mode::Autoregressive { first: 9112 });
        let mut s = sequence::Sequence::new(&p);
        let mut b = Fake::new();
        b.fail = Some(n);
        assert!(s.run(&mut b, p.sha256(), &input(1, 9112)).is_err());
        assert!(b.poisoned);
        assert_eq!(b.events.len(), n + 1);
        assert!(!s.exhausted());
        b.fail = None;
        assert!(s.run(&mut b, p.sha256(), &input(2, 67)).is_err());
        assert_eq!(b.events.len(), n + 1);
        assert!(s.run(&mut b, p.sha256(), &input(1, 9112)).is_err());
        assert_eq!(b.events.len(), n + 1);
    }
}
#[test]
fn input_profile_scope_generation_pages_rotary_and_token_refuse_before_metadata() {
    for mutation in 0..9 {
        let p = profile(Mode::Autoregressive { first: 9112 });
        let mut s = sequence::Sequence::new(&p);
        let mut b = Fake::new();
        let mut i = input(1, 9112);
        let mut digest = p.sha256();
        match mutation {
            0 => digest[0] ^= 1,
            1 => i.registration = [9; 32],
            2 => i.generation = 2,
            3 => i.cache_metadata[0] = 1,
            4 => i.cache_metadata[1] = 144,
            5 => i.cache_metadata[2] = i.cache_metadata[1],
            6 => i.rotary_bits[127] = f32::NAN.to_bits(),
            7 => i.token = 151936,
            _ => i.token = 0,
        }
        assert!(s.run(&mut b, digest, &i).is_err());
        assert!(b.events.is_empty());
        assert!(b.poisoned);
    }
}
#[test]
fn changing_any_retained_page_permutation_after_commit_is_not_a_new_kv_trajectory() {
    for g in 2..=4 {
        let p = profile(Mode::Autoregressive { first: 9112 });
        let mut s = sequence::Sequence::new(&p);
        let mut b = Fake::new();
        for prior in 1..g {
            s.run(
                &mut b,
                p.sha256(),
                &input(prior, if prior == 1 { 9112 } else { 67 }),
            )
            .unwrap();
        }
        let mut next = input(g, 67);
        next.cache_metadata.swap(1, 144);
        let n = b.events.len();
        assert!(s.run(&mut b, p.sha256(), &next).is_err());
        assert_eq!(b.events.len(), n);
    }
}
#[test]
fn output_bounds_refuse_before_fence_or_commit() {
    let p = profile(Mode::Autoregressive { first: 9112 });
    let mut s = sequence::Sequence::new(&p);
    let mut b = Fake::new();
    b.output = 151936;
    assert!(s.run(&mut b, p.sha256(), &input(1, 9112)).is_err());
    assert_eq!(b.events.last().unwrap(), "tail");
    assert!(!s.exhausted());
    assert!(b.poisoned);
}
#[test]
fn profile_digest_binds_original_source_distinct_image_scope_mode_and_deadline() {
    let mode = Mode::TeacherForced([9112, 2190, 3772, 220]);
    let p = profile(mode);
    for mutation in 0..11 {
        let mut s = scope();
        let mut source = [6; 32];
        let mut image = [7; 32];
        let mut mode = mode;
        let mut timeout = 1000;
        match mutation {
            0 => s.bundle_id = [8; 32],
            1 => s.model_id = [8; 32],
            2 => s.session = [8; 32],
            3 => s.pool_identity = 8,
            4 => s.group_id = 8,
            5 => s.child_identity = 8,
            6 => source = [8; 32],
            7 => image = [8; 32],
            8 => mode = Mode::Autoregressive { first: 9112 },
            9 => mode = Mode::TeacherForced([9112, 2190, 3772, 221]),
            _ => timeout = 999,
        }
        assert_ne!(
            p.sha256(),
            Profile::new(&s, source, image, mode, timeout, [101, 102])
                .unwrap()
                .sha256()
        );
    }
    assert!(Profile::new(&scope(), [0; 32], [7; 32], mode, 1000, [101, 102]).is_err());
    assert!(Profile::new(&scope(), [6; 32], [0; 32], mode, 1000, [101, 102]).is_err());
    for t in [0, 10001] {
        assert!(Profile::new(&scope(), [6; 32], [7; 32], mode, t, [101, 102]).is_err());
    }
    assert!(
        Profile::new(
            &scope(),
            [6; 32],
            [7; 32],
            Mode::TeacherForced([0, 1, 2, 151936]),
            1000,
            [101, 102]
        )
        .is_err()
    );
    assert_ne!(
        p.sha256(),
        Profile::new(&scope(), [6; 32], [7; 32], mode, 1000, [102, 101])
            .unwrap()
            .sha256()
    );
    assert!(Profile::new(&scope(), [6; 32], [7; 32], mode, 1000, [101, 101]).is_err());
}
