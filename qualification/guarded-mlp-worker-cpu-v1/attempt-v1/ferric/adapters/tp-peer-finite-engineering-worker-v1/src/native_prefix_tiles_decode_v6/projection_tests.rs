use super::*;
use crate::finite_projection_residual_decode_wire_v1 as wire;
fn profile() -> Profile {
    let b = wire::tests::bootstrap();
    Profile::new(
        &b.decode.scope,
        b.decode.registration,
        b.decode.prefix_image.sha256,
        b.decode.tiles_image.sha256,
        Mode::TeacherForced(b.decode.input_tokens.try_into().unwrap()),
        b.decode.timeout_ms,
        b.decode.device_ids,
    )
    .unwrap()
}
fn input(position: u32) -> ForwardInput {
    let b = wire::tests::bootstrap();
    ForwardInput {
        registration: b.decode.registration,
        generation: u64::from(position) + 1,
        token: b.decode.input_tokens[position as usize],
        cache_metadata: core::array::from_fn(|i| if i == 0 { position } else { (i - 1) as u32 }),
        rotary_bits: [0; 128],
    }
}
#[derive(Default)]
struct Fake {
    calls: usize,
    layers: usize,
    poisoned: bool,
    fail: Option<usize>,
}
impl Fake {
    fn call(&mut self) -> Result<()> {
        let n = self.calls;
        self.calls += 1;
        if self.fail == Some(n) {
            Err("injected candidate failure".into())
        } else {
            Ok(())
        }
    }
}
impl sequence::Backend for Fake {
    fn metadata(&mut self, _: &ForwardInput) -> Result<()> {
        self.call()
    }
    fn embedding(&mut self, _: u32) -> Result<[u64; 2]> {
        self.call()?;
        Ok([1; 2])
    }
    fn begin(&mut self, _: &ForwardInput) -> Result<()> {
        self.call()
    }
    fn layer(&mut self, _: usize) -> Result<layer::Completion> {
        self.call()?;
        self.layers += 1;
        Ok(layer::Completion {
            prefix_states: [[0; 284]; 2],
            mlp_states: [[0; 548]; 2],
            timing: crate::resident_layer::prefix_tiles_decode_v6::Timing::Paired([[1; 2]; 4]),
        })
    }
    fn tail(&mut self) -> Result<(u32, [u64; 3])> {
        self.call()?;
        Ok((6 + (self.layers / 36) as u32, [1; 3]))
    }
    fn fence(&mut self) -> Result<()> {
        self.call()
    }
    fn commit(&mut self) -> Result<()> {
        self.call()
    }
    fn poison(&mut self) {
        self.poisoned = true;
    }
}
#[test]
fn projection_decode_profile_matches_wire_and_rejects_reselection_and_zero() {
    let b = wire::tests::bootstrap();
    let p = profile()
        .with_projection(b.projection_residual_image.sha256)
        .unwrap();
    assert_eq!(p.sha256(), b.sha256().unwrap());
    assert_ne!(p.sha256(), profile().sha256());
    assert!(p.with_projection([2; 32]).is_err());
    assert!(profile().with_projection([0; 32]).is_err());
}

fn ar_profile() -> Profile {
    let b = wire::tests::ar_bootstrap();
    Profile::new(
        &b.decode.scope,
        b.decode.registration,
        b.decode.prefix_image.sha256,
        b.decode.tiles_image.sha256,
        Mode::Autoregressive {
            first: b.decode.input_tokens[0],
        },
        b.decode.timeout_ms,
        b.decode.device_ids,
    )
    .unwrap()
    .with_projection(b.projection_residual_image.sha256)
    .unwrap()
}

fn ar_input(position: u32) -> ForwardInput {
    let mut value = input(position);
    value.token = if position == 0 { 9112 } else { position + 6 };
    value
}

#[test]
fn projection_ar4_sequence_commits_four_distinct_outputs_and_only_then_closes() {
    let p = ar_profile();
    assert_eq!(p.sha256(), wire::tests::ar_bootstrap().sha256().unwrap());
    assert_ne!(
        p.sha256(),
        profile()
            .with_projection(wire::tests::bootstrap().projection_residual_image.sha256)
            .unwrap()
            .sha256()
    );
    let mut sequence = sequence::Sequence::new(&p);
    let mut backend = Fake::default();
    for position in 0..4 {
        assert!(close_ready(sequence.exhausted(), true, u64::from(position)).is_err());
        let c = sequence
            .run(&mut backend, p.sha256(), &ar_input(position))
            .unwrap();
        assert_eq!(c.input_token, ar_input(position).token);
        assert_eq!(c.output_token, position + 7);
        assert_eq!(c.layers.len(), 36);
    }
    assert_eq!(backend.layers, 144);
    close_ready(sequence.exhausted(), true, 4).unwrap();
    let calls = backend.calls;
    assert!(
        sequence
            .run(&mut backend, p.sha256(), &ar_input(3))
            .is_err()
    );
    assert_eq!(backend.calls, calls);
    assert!(backend.poisoned);
}

#[test]
fn projection_ar4_sequence_wrong_prior_token_poison_precedes_every_backend_call() {
    for wrong_at in 0..4 {
        let p = ar_profile();
        let mut sequence = sequence::Sequence::new(&p);
        let mut backend = Fake::default();
        for position in 0..wrong_at {
            sequence
                .run(&mut backend, p.sha256(), &ar_input(position))
                .unwrap();
        }
        let calls = backend.calls;
        let mut wrong = ar_input(wrong_at);
        wrong.token ^= 1;
        assert!(sequence.run(&mut backend, p.sha256(), &wrong).is_err());
        assert_eq!(backend.calls, calls);
        assert!(backend.poisoned && !sequence.exhausted());
        assert!(
            sequence
                .run(&mut backend, p.sha256(), &ar_input(wrong_at))
                .is_err()
        );
        assert_eq!(backend.calls, calls);
        assert!(close_ready(sequence.exhausted(), true, 4).is_err());
    }
}

#[test]
fn projection_ar4_sequence_image_profile_mismatch_and_backend_failure_stay_terminal() {
    let p = ar_profile();
    for failure in [None, Some(0), Some(39), Some(41)] {
        let mut sequence = sequence::Sequence::new(&p);
        let mut backend = Fake {
            fail: failure,
            ..Fake::default()
        };
        let profile = if failure.is_none() {
            [42; 32]
        } else {
            p.sha256()
        };
        assert!(sequence.run(&mut backend, profile, &ar_input(0)).is_err());
        assert_eq!(backend.calls, failure.map_or(0, |n| n + 1));
        let calls = backend.calls;
        backend.fail = None;
        assert!(
            sequence
                .run(&mut backend, p.sha256(), &ar_input(0))
                .is_err()
        );
        assert_eq!(backend.calls, calls);
        assert!(backend.poisoned && !sequence.exhausted());
    }
}
#[test]
fn projection_decode_load_error_terminalizes_phase_and_poisons_states() {
    let mut phase = Phase::LayersSealed;
    let mut poisoned = false;
    let error = finish_projection_load::<()>(Err("actual load error".into()), &mut phase, || {
        poisoned = true
    })
    .unwrap_err();
    assert_eq!(error, "actual load error");
    assert_eq!(phase, Phase::Terminal);
    assert!(poisoned);
    let mut phase = Phase::LayersSealed;
    assert_eq!(
        finish_projection_load(Ok(7), &mut phase, || panic!("successful load poisoned")),
        Ok(7)
    );
    assert_eq!(phase, Phase::LayersSealed);
}
#[test]
fn projection_decode_sequence_rejects_old_profile_before_any_backend_work() {
    let p = profile().with_projection([42; 32]).unwrap();
    let mut sequence = sequence::Sequence::new(&p);
    let mut backend = Fake::default();
    assert!(
        sequence
            .run(&mut backend, profile().sha256(), &input(0))
            .is_err()
    );
    assert_eq!(backend.calls, 0);
    assert!(backend.poisoned);
    assert!(sequence.run(&mut backend, p.sha256(), &input(0)).is_err());
    assert_eq!(backend.calls, 0);
}
#[test]
fn projection_decode_sequence_binds_four_completions_and_unchanged_close_frontier() {
    let p = profile().with_projection([42; 32]).unwrap();
    let mut sequence = sequence::Sequence::new(&p);
    let mut backend = Fake::default();
    for position in 0..4 {
        assert!(close_ready(sequence.exhausted(), true, u64::from(position)).is_err());
        let c = sequence
            .run(&mut backend, p.sha256(), &input(position))
            .unwrap();
        assert_eq!(c.profile_sha256, p.sha256());
        assert_eq!(c.layers.len(), 36);
    }
    assert_eq!(backend.layers, 144);
    close_ready(sequence.exhausted(), true, 4).unwrap();
    assert!(close_ready(sequence.exhausted(), false, 4).is_err());
    assert!(!backend.poisoned);
}
#[test]
fn projection_decode_sequence_every_stage_failure_prevents_retry_and_close() {
    let p = profile().with_projection([42; 32]).unwrap();
    for fail in 0..42 {
        let mut sequence = sequence::Sequence::new(&p);
        let mut backend = Fake {
            fail: Some(fail),
            ..Fake::default()
        };
        assert!(sequence.run(&mut backend, p.sha256(), &input(0)).is_err());
        assert_eq!(backend.calls, fail + 1);
        assert!(backend.poisoned);
        assert!(!sequence.exhausted());
        assert!(close_ready(sequence.exhausted(), true, 4).is_err());
        backend.fail = None;
        assert!(sequence.run(&mut backend, p.sha256(), &input(0)).is_err());
        assert_eq!(backend.calls, fail + 1);
    }
}
