use super::*;
use crate::finite_guarded_mlp_readiness_forward_durations_v1::ForwardRow;
use std::cell::RefCell;
use std::rc::Rc;

struct Clock {
    values: [u128; 10],
    next: usize,
    fail: Option<usize>,
    panic: Option<usize>,
    events: Rc<RefCell<Vec<&'static str>>>,
}
impl Clock {
    fn new(events: Rc<RefCell<Vec<&'static str>>>) -> Self {
        Self {
            values: std::array::from_fn(|i| (i * (i + 1) / 2) as u128),
            next: 0,
            fail: None,
            panic: None,
            events,
        }
    }
}
impl timing::Clock for Clock {
    fn now(&mut self) -> io::Result<u128> {
        let index = self.next;
        self.next += 1;
        self.events.borrow_mut().push("clock");
        assert_ne!(self.panic, Some(index), "injected clock unwind");
        if self.fail == Some(index) {
            return Err(io::Error::other("injected clock refusal"));
        }
        Ok(self.values[index])
    }
}
struct Measured {
    inner: Mock,
    selected: bool,
    deadlines: usize,
    fail_deadline: Option<usize>,
    sink_calls: usize,
    fail_sink: bool,
    panic_sink: bool,
    rows: Vec<ForwardRow>,
    events: Rc<RefCell<Vec<&'static str>>>,
}
impl Measured {
    fn new() -> Self {
        Self {
            inner: Mock::new(),
            selected: true,
            deadlines: 0,
            fail_deadline: None,
            sink_calls: 0,
            fail_sink: false,
            panic_sink: false,
            rows: Vec::new(),
            events: Rc::new(RefCell::new(Vec::new())),
        }
    }
    fn event(&self, name: &'static str) {
        self.events.borrow_mut().push(name);
    }
}
impl Backend for Measured {
    fn check_deadline(&mut self) -> io::Result<()> {
        self.event("deadline");
        self.deadlines += 1;
        if self.fail_deadline == Some(self.deadlines) {
            return Err(io::Error::other("selected original deadline"));
        }
        self.inner.check_deadline()
    }
    fn metadata(&mut self, r: &Request) -> io::Result<()> {
        self.event("metadata");
        self.inner.metadata(r)
    }
    fn embedding(&mut self, token: u32) -> io::Result<[u64; 2]> {
        self.event("embedding");
        self.inner.embedding(token)
    }
    fn begin(&mut self, step: BankStep) -> io::Result<()> {
        self.event("begin");
        self.inner.begin(step)
    }
    fn layer(&mut self, index: usize) -> io::Result<LayerObservation> {
        self.event("layer");
        self.inner.layer(index)
    }
    fn tail(&mut self) -> io::Result<Tail> {
        self.event("tail");
        self.inner.tail()
    }
    fn fence(&mut self) -> io::Result<()> {
        self.event("fence");
        self.inner.fence()
    }
    fn commit(&mut self, generation: u64) -> io::Result<()> {
        self.event("commit");
        self.inner.commit(generation)
    }
    fn close(&mut self) -> io::Result<()> {
        self.event("close");
        self.inner.close()
    }
    fn poison(&mut self) {
        self.inner.poison();
    }
    fn forward_timing_selected(&self) -> bool {
        self.selected
    }
    fn record_forward_timing(&mut self, row: ForwardRow) -> io::Result<()> {
        self.event("sink");
        self.sink_calls += 1;
        assert!(!self.panic_sink, "injected sink unwind");
        if self.fail_sink {
            return Err(io::Error::other("injected sink refusal"));
        }
        row.validate()?;
        self.rows.push(row);
        Ok(())
    }
}
fn request0() -> (Bootstrap, Request) {
    let b = fixture::bootstrap(Profile::Readiness40Position5);
    let r = fixture::request(&b, 0, 0);
    (b, r)
}

#[test]
fn forward_phase_sequence_exact_ten_stamps_and_old_effect_deadline_trace() {
    let (b, r) = request0();
    let mut s = Sequence::new(b).unwrap();
    let mut backend = Measured::new();
    let mut clock = Clock::new(backend.events.clone());
    s.run_inner(&mut backend, &r, &mut clock).unwrap();
    assert_eq!(clock.next, 10);
    assert_eq!(backend.inner.effects, 42);
    assert_eq!(backend.deadlines, 46);
    assert_eq!(
        backend.rows,
        vec![ForwardRow {
            position: 0,
            phase_ns: [1, 2, 3, 4, 5, 6, 7, 8, 9],
            forward_body_ns: 45,
        }]
    );
    let mut expected = vec!["clock", "deadline", "deadline", "clock"];
    for effect in ["metadata", "embedding", "begin"] {
        expected.extend([effect, "deadline", "clock"]);
    }
    for _ in 0..36 {
        expected.extend(["layer", "deadline"]);
    }
    expected.extend(["clock", "tail", "deadline", "clock", "deadline", "clock"]);
    expected.extend([
        "fence", "deadline", "clock", "commit", "deadline", "deadline", "clock", "sink",
    ]);
    assert_eq!(*backend.events.borrow(), expected);
    assert_eq!(backend.inner.poisoned, 0);
}

#[test]
fn forward_phase_sequence_disabled_routes_never_call_clock_or_sink() {
    for profile in [
        Profile::Readiness40,
        Profile::Readiness40Position5,
        Profile::Full2303,
    ] {
        let b = fixture::bootstrap(profile);
        let r = fixture::request(&b, 0, 0);
        let mut s = Sequence::new(b.clone()).unwrap();
        let mut backend = Measured::new();
        backend.selected = false;
        let mut clock = Clock::new(backend.events.clone());
        clock.panic = Some(0);
        s.run_inner(&mut backend, &r, &mut clock).unwrap();
        assert_eq!((clock.next, backend.sink_calls), (0, 0));
        assert_eq!((backend.inner.effects, backend.deadlines), (42, 46));
        let mut original = Mock::new();
        Sequence::new(b)
            .unwrap()
            .run_inner(&mut original, &r, &mut clock)
            .unwrap();
        assert_eq!(original.calls, backend.inner.calls);
        assert_eq!(clock.next, 0);
    }
}

#[test]
fn forward_phase_sequence_every_clock_error_and_unwind_poison() {
    for panic in [false, true] {
        for index in 0..10 {
            let (b, r) = request0();
            let mut s = Sequence::new(b).unwrap();
            let mut backend = Measured::new();
            let mut clock = Clock::new(backend.events.clone());
            if panic {
                clock.panic = Some(index);
            } else {
                clock.fail = Some(index);
            }
            let result = catch_unwind(AssertUnwindSafe(|| {
                s.run_inner(&mut backend, &r, &mut clock)
            }));
            assert!(if panic {
                result.is_err()
            } else {
                result.unwrap().is_err()
            });
            assert_eq!(clock.next, index + 1);
            assert_eq!(backend.inner.poisoned, 1);
            assert_eq!(backend.sink_calls, 0);
            assert_eq!(
                backend.inner.effects,
                [0, 0, 1, 2, 3, 39, 40, 40, 41, 42][index]
            );
            assert_eq!(backend.inner.committed, u64::from(index == 9));
            let before = backend.inner.effects;
            let mut retry = Clock::new(backend.events.clone());
            assert!(s.run_inner(&mut backend, &r, &mut retry).is_err());
            assert_eq!(backend.inner.effects, before);
        }
    }
}

#[test]
fn forward_phase_sequence_sink_error_and_unwind_after_commit_are_terminal() {
    for panic in [false, true] {
        let (b, r) = request0();
        let mut s = Sequence::new(b.clone()).unwrap();
        let mut backend = Measured::new();
        backend.fail_sink = !panic;
        backend.panic_sink = panic;
        let mut clock = Clock::new(backend.events.clone());
        let result = catch_unwind(AssertUnwindSafe(|| {
            s.run_inner(&mut backend, &r, &mut clock)
        }));
        assert!(if panic {
            result.is_err()
        } else {
            result.unwrap().is_err()
        });
        assert_eq!(
            (backend.inner.effects, backend.deadlines, clock.next),
            (42, 46, 10)
        );
        assert_eq!(
            (
                backend.inner.committed,
                backend.inner.poisoned,
                backend.sink_calls
            ),
            (1, 1, 1)
        );
        assert!(backend.rows.is_empty());
        backend.fail_sink = false;
        backend.panic_sink = false;
        assert!(s.run(&mut backend, &fixture::request(&b, 1, 0)).is_err());
        let digest = s.digest();
        assert!(
            s.close(&mut backend, &fixture::close_request(&b), digest)
                .is_err()
        );
        assert_eq!(backend.inner.effects, 42);
        assert!(!s.is_closed() && !backend.inner.closed);
    }
}

#[test]
fn forward_phase_sequence_effect_and_every_deadline_refusal_never_publish() {
    for deadline in 1..=46 {
        let (b, r) = request0();
        let mut s = Sequence::new(b).unwrap();
        let mut backend = Measured::new();
        backend.fail_deadline = Some(deadline);
        let mut clock = Clock::new(backend.events.clone());
        assert!(s.run_inner(&mut backend, &r, &mut clock).is_err());
        assert_eq!(backend.deadlines, deadline);
        assert_eq!((backend.inner.poisoned, backend.sink_calls), (1, 0));
    }
    for effect in 1..=42 {
        let (b, r) = request0();
        let mut s = Sequence::new(b).unwrap();
        let mut backend = Measured::new();
        backend.inner.fail_at = Some(effect);
        let mut clock = Clock::new(backend.events.clone());
        assert!(s.run_inner(&mut backend, &r, &mut clock).is_err());
        assert_eq!(backend.inner.effects, effect);
        assert_eq!((backend.inner.poisoned, backend.sink_calls), (1, 0));
    }
}

#[test]
fn forward_phase_sequence_clock_arithmetic_refusal_is_inside_attempt() {
    for case in 0..3 {
        let (b, r) = request0();
        let mut s = Sequence::new(b).unwrap();
        let mut backend = Measured::new();
        let mut clock = Clock::new(backend.events.clone());
        match case {
            0 => clock.values[5] = 0,
            1 => clock.values[9] = u128::from(u64::MAX) + 1,
            _ => clock.values[9] = 3_600_000_000_001,
        }
        assert!(s.run_inner(&mut backend, &r, &mut clock).is_err());
        assert_eq!(
            (backend.inner.effects, backend.deadlines, clock.next),
            (42, 46, 10)
        );
        assert_eq!(
            (
                backend.inner.committed,
                backend.inner.poisoned,
                backend.sink_calls
            ),
            (1, 1, 0)
        );
    }
}

#[test]
fn forward_phase_sequence_forty_rows_and_real_close_keep_own_transcript() {
    let b = fixture::bootstrap(Profile::Readiness40Position5);
    let mut s = Sequence::new(b.clone()).unwrap();
    let mut backend = Measured::new();
    for position in 0..40 {
        let mut clock = Clock::new(backend.events.clone());
        s.run_inner(&mut backend, &fixture::request(&b, position, 0), &mut clock)
            .unwrap();
        assert_eq!(clock.next, 10);
        assert_eq!(backend.rows[position as usize].position, position);
    }
    assert_eq!(
        (backend.inner.effects, backend.deadlines),
        (40 * 42, 40 * 46)
    );
    assert_eq!(backend.rows.len(), 40);
    let digest = s.digest();
    s.close(&mut backend, &fixture::close_request(&b), digest)
        .unwrap();
    assert!(s.is_closed() && backend.inner.closed);
    assert_eq!(backend.inner.poisoned, 0);
    assert_eq!(backend.rows.len(), 40);
}
