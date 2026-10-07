use super::*;
use crate::finite_guarded_mlp_long_wire_v2::{Profile, tests as fixture};
use std::panic::{AssertUnwindSafe, catch_unwind};

struct Mock {
    template: Control,
    position: u32,
    effects: usize,
    fail_at: Option<usize>,
    panic_at: Option<usize>,
    expire_after: Option<usize>,
    poisoned: usize,
    committed: u64,
    closed: bool,
    layer: usize,
    banks: [u32; 2],
    calls: Vec<&'static str>,
    bad_output: bool,
    bad_terminal: bool,
}
impl Mock {
    fn new() -> Self {
        Self {
            template: fixture::control(1),
            position: 0,
            effects: 0,
            fail_at: None,
            panic_at: None,
            expire_after: None,
            poisoned: 0,
            committed: 0,
            closed: false,
            layer: 0,
            banks: [0; 2],
            calls: Vec::new(),
            bad_output: false,
            bad_terminal: false,
        }
    }
    fn effect(&mut self, name: &'static str) -> io::Result<()> {
        self.effects += 1;
        self.calls.push(name);
        assert_ne!(self.panic_at, Some(self.effects), "injected unwind");
        if self.fail_at == Some(self.effects) {
            return Err(io::Error::other("injected effect failure"));
        }
        Ok(())
    }
}
impl Backend for Mock {
    fn check_deadline(&mut self) -> io::Result<()> {
        if self.expire_after.is_some_and(|n| self.effects >= n) {
            Err(io::Error::other("fixed deadline"))
        } else {
            Ok(())
        }
    }
    fn metadata(&mut self, r: &Request) -> io::Result<()> {
        self.effect("metadata")?;
        self.position = (r.id - 1) as u32;
        Ok(())
    }
    fn embedding(&mut self, _: u32) -> io::Result<[u64; 2]> {
        self.effect("embedding")?;
        Ok([1, 2])
    }
    fn begin(&mut self, step: BankStep) -> io::Result<()> {
        self.effect("begin")?;
        let bank = step.bank as usize;
        assert_eq!(self.banks[bank] + 1, step.local_generation);
        assert_eq!(step.logical_page * 16 + step.page_offset, self.position);
        self.banks[bank] = step.local_generation;
        self.layer = 0;
        Ok(())
    }
    fn layer(&mut self, index: usize) -> io::Result<LayerObservation> {
        self.effect("layer")?;
        assert_eq!(self.layer, index);
        self.layer += 1;
        let mut row = self.template.layers[index].clone();
        row.guards = [[self.position / 2 + 1, 0, 1, 0]; 2];
        row.observed_queue_frontiers = [(
            u64::from(self.position) * 1000 + index as u64 * 10 + 10,
            u64::from(self.position) * 1000 + index as u64 * 10 + 8,
        ); 2];
        if self.bad_terminal {
            row.guards[0][2] = 0;
        }
        Ok(row)
    }
    fn tail(&mut self) -> io::Result<Tail> {
        self.effect("tail")?;
        assert_eq!(self.layer, 36);
        let output = self.position % 101 + 3;
        Ok(Tail {
            output_token: output + u32::from(self.bad_output),
            host_ns: [3, 4, 5],
            observation: fixture::payload(output),
        })
    }
    fn fence(&mut self) -> io::Result<()> {
        self.effect("fence")
    }
    fn commit(&mut self, generation: u64) -> io::Result<()> {
        self.effect("commit")?;
        assert_eq!(generation, self.committed + 1);
        self.committed = generation;
        Ok(())
    }
    fn close(&mut self) -> io::Result<()> {
        self.effect("close")?;
        self.closed = true;
        Ok(())
    }
    fn poison(&mut self) {
        self.poisoned += 1;
    }
}
fn drive(profile: Profile) -> (Sequence, Mock, Bootstrap) {
    let b = fixture::bootstrap(profile);
    let mut sequence = Sequence::new(b.clone()).unwrap();
    let mut backend = Mock::new();
    let mut previous = 0;
    for position in 0..profile.forwards() {
        let r = fixture::request(&b, position, previous);
        let result = sequence.run(&mut backend, &r).unwrap();
        previous = result.frame.completion.output_token;
        assert_eq!(sequence.completed(), position + 1);
        assert_eq!(result.frame.completion.captured, profile.captures(position));
        assert_eq!(
            result.frame.completion.bank.local_generation,
            position / 2 + 1
        );
    }
    (sequence, backend, b)
}

#[test]
fn long_sequence_full_2303_mock_preserves_order_banks_and_256_own_outputs() {
    let (mut s, mut backend, b) = drive(Profile::Full2303);
    assert_eq!(backend.effects, 2303 * 42);
    assert_eq!(backend.banks, [1152, 1151]);
    assert_eq!(s.output_tokens().len(), 256);
    assert_eq!(
        s.output_tokens(),
        &(2047..2303).map(|p| p % 101 + 3).collect::<Vec<u32>>()
    );
    let digest = s.digest();
    s.close(&mut backend, &fixture::close_request(&b), digest)
        .unwrap();
    assert!(s.is_closed() && backend.closed);
    assert_eq!(backend.poisoned, 0);
    assert_eq!(
        &backend.calls[..4],
        &["metadata", "embedding", "begin", "layer"]
    );
    assert_eq!(&backend.calls[39..42], &["tail", "fence", "commit"]);
}

#[test]
fn long_sequence_readiness40_is_prompt_only_with_distinct_healthy_close() {
    let (mut s, mut backend, b) = drive(Profile::Readiness40);
    assert!(s.output_tokens().is_empty());
    assert_eq!(backend.banks, [20, 20]);
    assert_eq!(s.completed(), 40);
    let digest = s.digest();
    s.close(&mut backend, &fixture::close_request(&b), digest)
        .unwrap();
    assert!(s.is_closed());
    assert_eq!(backend.effects, 40 * 42 + 1);
}

#[test]
fn long_sequence_every_effect_failure_is_terminal_without_retry_or_commit() {
    let b = fixture::bootstrap(Profile::Readiness40);
    let r = fixture::request(&b, 0, 0);
    for ordinal in 1..=42 {
        let mut s = Sequence::new(b.clone()).unwrap();
        let mut backend = Mock::new();
        backend.fail_at = Some(ordinal);
        assert!(s.run(&mut backend, &r).is_err());
        assert_eq!(backend.effects, ordinal);
        assert_eq!(backend.committed, 0);
        assert_eq!(s.completed(), 0);
        assert_eq!(backend.poisoned, 1);
        backend.fail_at = None;
        assert!(s.run(&mut backend, &r).is_err());
        assert_eq!(backend.effects, ordinal);
    }
}

#[test]
fn long_sequence_every_post_effect_deadline_stops_before_next_operation() {
    let b = fixture::bootstrap(Profile::Readiness40);
    let r = fixture::request(&b, 0, 0);
    for ordinal in 0..=42 {
        let mut s = Sequence::new(b.clone()).unwrap();
        let mut backend = Mock::new();
        backend.expire_after = Some(ordinal);
        assert!(s.run(&mut backend, &r).is_err());
        assert_eq!(backend.effects, ordinal);
        assert_eq!(s.completed(), 0);
        assert_eq!(backend.poisoned, 1);
        assert_eq!(backend.committed, u64::from(ordinal == 42));
    }
}

#[test]
fn long_sequence_unwind_and_publication_failure_quarantine_existing_work() {
    let b = fixture::bootstrap(Profile::Readiness40);
    let r = fixture::request(&b, 0, 0);
    for ordinal in [1, 3, 20, 40, 42] {
        let mut s = Sequence::new(b.clone()).unwrap();
        let mut backend = Mock::new();
        backend.panic_at = Some(ordinal);
        assert!(catch_unwind(AssertUnwindSafe(|| s.run(&mut backend, &r))).is_err());
        assert_eq!(backend.poisoned, 1);
        assert_eq!(s.completed(), 0);
        backend.panic_at = None;
        assert!(s.run(&mut backend, &r).is_err());
        assert_eq!(backend.effects, ordinal);
    }
    let mut s = Sequence::new(b.clone()).unwrap();
    let mut backend = Mock::new();
    s.run(&mut backend, &r).unwrap();
    s.cancel(&mut backend);
    assert!(s.run(&mut backend, &fixture::request(&b, 1, 3)).is_err());
    assert_eq!(backend.effects, 42);
}

#[test]
fn long_sequence_bad_terminal_or_device_argmax_refuses_before_fence_and_commit() {
    let b = fixture::bootstrap(Profile::Readiness40);
    for bad_terminal in [false, true] {
        let mut s = Sequence::new(b.clone()).unwrap();
        let mut backend = Mock::new();
        backend.bad_terminal = bad_terminal;
        backend.bad_output = !bad_terminal;
        assert!(s.run(&mut backend, &fixture::request(&b, 0, 0)).is_err());
        assert_eq!(backend.effects, 40);
        assert_eq!(backend.committed, 0);
        assert_eq!(s.completed(), 0);
        assert_eq!(backend.poisoned, 1);
    }
}

#[test]
fn long_sequence_wrong_scope_order_and_premature_close_never_touch_backend() {
    let b = fixture::bootstrap(Profile::Readiness40);
    for bad in [fixture::request(&b, 1, 0), fixture::close_request(&b)] {
        let mut s = Sequence::new(b.clone()).unwrap();
        let mut backend = Mock::new();
        assert!(s.run(&mut backend, &bad).is_err());
        assert_eq!(backend.effects, 0);
        assert_eq!(backend.poisoned, 1);
    }
    let mut s = Sequence::new(b.clone()).unwrap();
    let mut backend = Mock::new();
    let digest = s.digest();
    assert!(
        s.close(&mut backend, &fixture::close_request(&b), digest)
            .is_err()
    );
    assert_eq!(backend.effects, 0);
    let mut wrong = fixture::request(&b, 0, 0);
    wrong.registration[0] ^= 1;
    let mut s = Sequence::new(b).unwrap();
    assert!(s.run(&mut backend, &wrong).is_err());
    assert_eq!(backend.effects, 0);
}

#[test]
fn long_sequence_close_failure_deadline_and_second_close_remain_terminal() {
    for expire in [false, true] {
        let (mut s, mut backend, b) = drive(Profile::Readiness40);
        let next = backend.effects + 1;
        if expire {
            backend.expire_after = Some(next);
        } else {
            backend.fail_at = Some(next);
        }
        let digest = s.digest();
        assert!(
            s.close(&mut backend, &fixture::close_request(&b), digest)
                .is_err()
        );
        assert!(!s.is_closed());
        assert_eq!(backend.effects, next);
        assert_eq!(backend.poisoned, 1);
    }
    let (mut s, mut backend, b) = drive(Profile::Readiness40);
    let digest = s.digest();
    s.close(&mut backend, &fixture::close_request(&b), digest)
        .unwrap();
    let before = backend.effects;
    assert!(
        s.close(&mut backend, &fixture::close_request(&b), digest)
            .is_err()
    );
    assert_eq!(backend.effects, before);
    assert!(!s.is_closed());
}
