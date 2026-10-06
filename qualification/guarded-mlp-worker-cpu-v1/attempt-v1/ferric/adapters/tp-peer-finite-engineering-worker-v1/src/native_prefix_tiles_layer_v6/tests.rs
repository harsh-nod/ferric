use super::*;
#[test]
fn projection_model_profile_is_distinct_and_fixed_to_prefix284() {
    let old = profile(true).sha256();
    let new = profile(true).with_projection([42; 32]).unwrap();
    assert_ne!(new.sha256(), old);
    assert_eq!(
        new.sha256(),
        crate::finite_projection_residual_layer_wire_v1::profile_sha256(old, [42; 32]).unwrap()
    );
    assert!(profile(false).with_projection([42; 32]).is_err());
    assert!(profile(true).with_projection([0; 32]).is_err());
    assert!(new.with_projection([43; 32]).is_err());
}
#[test]
fn projection_model_rejects_old_hash_before_metadata_and_poison_is_terminal() {
    let p = profile(true).with_projection([42; 32]).unwrap();
    let mut gate = Gate::default();
    let mut f = Fake {
        tiles: true,
        ..Default::default()
    };
    assert!(
        gate.run(&p, profile(true).sha256(), &input(), &mut f)
            .is_err()
    );
    assert!(f.events.is_empty());
    assert!(f.poison);
    assert!(!gate.complete);
    assert!(gate.run(&p, p.sha256(), &input(), &mut f).is_err());
    assert!(f.events.is_empty());
}
#[test]
fn projection_model_preserves_every_failure_boundary_and_consuming_close() {
    let p = profile(true).with_projection([42; 32]).unwrap();
    for fail in 0..5 {
        let mut gate = Gate::default();
        let mut f = Fake {
            tiles: true,
            fail: Some(fail),
            ..Default::default()
        };
        assert!(gate.run(&p, p.sha256(), &input(), &mut f).is_err());
        assert!(f.poison);
        assert!(!gate.complete);
        assert_eq!(f.events.len(), fail + 1);
        assert!(close_pending::<()>(gate.complete, None, || panic!("premature close")).is_err());
    }
    let mut gate = Gate::default();
    let mut f = Fake {
        tiles: true,
        ..Default::default()
    };
    let got = gate.run(&p, p.sha256(), &input(), &mut f).unwrap();
    assert_eq!(got.profile_sha256, p.sha256());
    assert!(gate.complete);
    assert!(!f.poison);
    assert_eq!(
        close_pending(gate.complete, Some(got), || Ok(()))
            .unwrap()
            .input_token,
        9112
    );
}
fn input() -> ForwardInput {
    ForwardInput {
        registration: [7; 32],
        generation: 1,
        token: 9112,
        cache_metadata: core::array::from_fn(|i| if i == 0 { 0 } else { (i - 1) as u32 }),
        rotary_bits: [0; 128],
    }
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
fn profile(tiles: bool) -> Profile {
    Profile::new(
        &scope(),
        [7; 32],
        tiles.then_some([8; 32]),
        [9; 32],
        &input(),
        1000,
        [11, 12],
    )
    .unwrap()
}
fn run(tiles: bool) -> layer::Run {
    layer::Run {
        completion: layer::Completion {
            prefix: if tiles {
                layer::PrefixObservation::Tiles284([[284; 284]; 2])
            } else {
                layer::PrefixObservation::Baseline22([[22; 22]; 2])
            },
            mlp: [[548; 548]; 2],
            paired_ns: [[0; 2]; 4],
        },
        capture: layer::Capture {
            prefix: core::array::from_fn(|_| core::array::from_fn(|_| Vec::new())),
            first_residual: [Vec::new(), Vec::new()],
            mlp: core::array::from_fn(|_| core::array::from_fn(|_| Vec::new())),
            final_hidden: [Vec::new(), Vec::new()],
        },
    }
}
#[derive(Default)]
struct Fake {
    events: Vec<&'static str>,
    fail: Option<usize>,
    tiles: bool,
    poison: bool,
}
impl Fake {
    fn step(&mut self, event: &'static str) -> Result<()> {
        let n = self.events.len();
        self.events.push(event);
        if self.fail == Some(n) {
            Err("injected".into())
        } else {
            Ok(())
        }
    }
}
impl Backend for Fake {
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
    fn layer(&mut self) -> Result<layer::Run> {
        self.step("layer")?;
        Ok(run(self.tiles))
    }
    fn fence(&mut self) -> Result<()> {
        self.step("fence")
    }
    fn poison(&mut self) {
        self.poison = true;
    }
}
#[test]
fn prefix_model_profile_binds_two_images_and_full_preopen_input() {
    let old = profile(false);
    let new = profile(true);
    assert_ne!(old.sha256(), new.sha256());
    for tiles in [false, true] {
        let p = profile(tiles);
        p.validate_input(p.sha256(), &input()).unwrap();
    }
    for change in 0..5 {
        let mut v = input();
        match change {
            0 => v.token += 1,
            1 => v.rotary_bits[127] = 1,
            2 => v.cache_metadata.swap(1, 144),
            3 => v.registration = [5; 32],
            _ => v.generation = 2,
        };
        assert!(new.validate_input(new.sha256(), &v).is_err());
    }
    assert!(new.validate_input(old.sha256(), &input()).is_err());
    assert_ne!(
        new.sha256(),
        Profile::new(
            &scope(),
            [7; 32],
            Some([6; 32]),
            [9; 32],
            &input(),
            1000,
            [11, 12]
        )
        .unwrap()
        .sha256()
    );
    assert_ne!(
        new.sha256(),
        Profile::new(
            &scope(),
            [7; 32],
            Some([8; 32]),
            [6; 32],
            &input(),
            1000,
            [11, 12]
        )
        .unwrap()
        .sha256()
    );
    assert_ne!(
        new.sha256(),
        Profile::new(
            &scope(),
            [7; 32],
            Some([8; 32]),
            [9; 32],
            &input(),
            1000,
            [12, 11]
        )
        .unwrap()
        .sha256()
    );
}
#[test]
fn prefix_model_input_bounds_refuse_before_any_metadata_or_device_action() {
    for change in 0..8 {
        let mut v = input();
        match change {
            0 => v.generation = 0,
            1 => v.generation = 2,
            2 => v.cache_metadata[0] = 1,
            3 => v.token = 151936,
            4 => v.cache_metadata[1] = 144,
            5 => v.cache_metadata[1] = 1,
            6 => v.rotary_bits[127] = 0x7f800000,
            _ => v.registration = [0; 32],
        };
        assert!(
            Profile::new(
                &scope(),
                [7; 32],
                Some([8; 32]),
                [9; 32],
                &v,
                1000,
                [11, 12]
            )
            .is_err()
        );
    }
    for devices in [[0, 12], [11, 11]] {
        assert!(
            Profile::new(
                &scope(),
                [7; 32],
                Some([8; 32]),
                [9; 32],
                &input(),
                1000,
                devices
            )
            .is_err()
        );
    }
    for timeout in [0, 10001] {
        assert!(
            Profile::new(
                &scope(),
                [7; 32],
                Some([8; 32]),
                [9; 32],
                &input(),
                timeout,
                [11, 12]
            )
            .is_err()
        );
    }
    assert!(
        Profile::new(
            &scope(),
            [7; 32],
            Some([0; 32]),
            [9; 32],
            &input(),
            1000,
            [11, 12]
        )
        .is_err()
    );
}
#[test]
fn prefix_model_runs_exactly_one_layer_and_retains_distinct_completion() {
    for tiles in [false, true] {
        let p = profile(tiles);
        let mut gate = Gate::default();
        let mut f = Fake {
            tiles,
            ..Default::default()
        };
        let got = gate.run(&p, p.sha256(), &input(), &mut f).unwrap();
        assert!(gate.complete);
        assert!(!f.poison);
        assert_eq!(
            f.events,
            ["metadata", "embedding", "begin", "layer", "fence"]
        );
        assert_eq!(got.input_token, 9112);
        assert_eq!(
            matches!(
                got.layer.completion.prefix,
                layer::PrefixObservation::Tiles284(_)
            ),
            tiles
        );
        assert!(gate.run(&p, p.sha256(), &input(), &mut f).is_err());
        assert!(!gate.complete);
        assert!(f.poison);
        assert_eq!(f.events.len(), 5);
    }
}
#[test]
fn prefix_model_failure_at_every_boundary_never_commits_or_retries() {
    let p = profile(true);
    for fail in 0..5 {
        let mut gate = Gate::default();
        let mut f = Fake {
            tiles: true,
            fail: Some(fail),
            ..Default::default()
        };
        assert!(gate.run(&p, p.sha256(), &input(), &mut f).is_err());
        assert!(!gate.complete);
        assert!(f.poison);
        assert_eq!(f.events.len(), fail + 1);
        f.fail = None;
        assert!(gate.run(&p, p.sha256(), &input(), &mut f).is_err());
        assert_eq!(f.events.len(), fail + 1);
    }
    let mut f = Fake {
        tiles: false,
        ..Default::default()
    };
    assert!(
        Gate::default()
            .run(&p, p.sha256(), &input(), &mut f)
            .is_err()
    );
    assert_eq!(f.events.len(), 4);
    assert!(f.poison);
    let mut f = Fake::default();
    assert!(Gate::default().run(&p, [0; 32], &input(), &mut f).is_err());
    assert!(f.events.is_empty());
    assert!(f.poison);
}
#[test]
fn prefix_model_capture_is_unavailable_until_successful_close() {
    let mut calls = 0;
    assert!(
        close_pending(false, Some(7), || {
            calls += 1;
            Ok(())
        })
        .is_err()
    );
    assert_eq!(calls, 0);
    assert!(
        close_pending::<usize>(true, None, || {
            calls += 1;
            Ok(())
        })
        .is_err()
    );
    assert_eq!(calls, 0);
    assert!(
        close_pending(true, Some(7), || {
            calls += 1;
            Err("close failure".into())
        })
        .is_err()
    );
    assert_eq!(calls, 1);
    assert_eq!(
        close_pending(true, Some(7), || {
            calls += 1;
            Ok(())
        })
        .unwrap(),
        7
    );
    assert_eq!(calls, 2);
}
