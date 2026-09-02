//! Executable lower-queue state machine used by authenticated lifecycle tests.
//!
//! This module exists only in unit-test builds. It models the native ownership
//! transitions, rejects double submission/destruction, and records every
//! effect so higher-level tests exercise consuming APIs rather than merely
//! checking their types.

use std::{cell::RefCell, collections::VecDeque, rc::Rc};

use ferric_spec::TokenId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InjectedQueueFailureV1 {
    CurrentnessReleased,
    CurrentnessQuarantined,
    Wait,
    ReadbackReleased,
    ReadbackQuarantined,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InjectedQueuePhaseV1 {
    Prepared,
    Published,
    Completed,
    Recycled,
    Released,
    Destroyed,
    Quarantined,
}

#[derive(Debug)]
pub struct InjectedQueueStateV1 {
    pub phase: InjectedQueuePhaseV1,
    pub submits: usize,
    pub waits: usize,
    pub recycles: usize,
    pub readbacks: usize,
    pub releases: usize,
    pub destroys: usize,
    pub failures: VecDeque<Option<InjectedQueueFailureV1>>,
    pub observations: VecDeque<Vec<TokenId>>,
}

#[derive(Clone, Debug)]
pub struct InjectedQueueV1(Rc<RefCell<InjectedQueueStateV1>>);

impl InjectedQueueV1 {
    pub fn new(
        failures: impl IntoIterator<Item = Option<InjectedQueueFailureV1>>,
        observations: impl IntoIterator<Item = Vec<TokenId>>,
    ) -> Self {
        Self(Rc::new(RefCell::new(InjectedQueueStateV1 {
            phase: InjectedQueuePhaseV1::Prepared,
            submits: 0,
            waits: 0,
            recycles: 0,
            readbacks: 0,
            releases: 0,
            destroys: 0,
            failures: failures.into_iter().collect(),
            observations: observations.into_iter().collect(),
        })))
    }

    pub fn snapshot(&self) -> InjectedQueueSnapshotV1 {
        let state = self.0.borrow();
        InjectedQueueSnapshotV1 {
            phase: state.phase,
            submits: state.submits,
            waits: state.waits,
            recycles: state.recycles,
            readbacks: state.readbacks,
            releases: state.releases,
            destroys: state.destroys,
        }
    }

    pub fn begin_generation(&self) -> Option<InjectedQueueFailureV1> {
        let mut state = self.0.borrow_mut();
        assert!(matches!(
            state.phase,
            InjectedQueuePhaseV1::Prepared | InjectedQueuePhaseV1::Released
        ));
        let failure = state.failures.pop_front().flatten();
        if matches!(
            failure,
            Some(
                InjectedQueueFailureV1::CurrentnessReleased
                    | InjectedQueueFailureV1::CurrentnessQuarantined
            )
        ) {
            state.phase = InjectedQueuePhaseV1::Prepared;
            return failure;
        }
        state.submits += 1;
        state.phase = InjectedQueuePhaseV1::Published;
        failure
    }

    pub fn publish_for_resume(&self) {
        let mut state = self.0.borrow_mut();
        assert_eq!(state.phase, InjectedQueuePhaseV1::Prepared);
        state.submits += 1;
        state.phase = InjectedQueuePhaseV1::Published;
    }

    pub fn wait(&self) {
        let mut state = self.0.borrow_mut();
        assert_eq!(state.phase, InjectedQueuePhaseV1::Published);
        state.waits += 1;
        state.phase = InjectedQueuePhaseV1::Completed;
    }

    pub fn quarantine_progress(&self) {
        let mut state = self.0.borrow_mut();
        assert_eq!(state.phase, InjectedQueuePhaseV1::Published);
        state.phase = InjectedQueuePhaseV1::Quarantined;
    }

    pub fn recycle(&self) {
        let mut state = self.0.borrow_mut();
        assert_eq!(state.phase, InjectedQueuePhaseV1::Completed);
        state.recycles += 1;
        state.phase = InjectedQueuePhaseV1::Recycled;
    }

    pub fn readback(&self) -> Vec<TokenId> {
        let mut state = self.0.borrow_mut();
        assert_eq!(state.phase, InjectedQueuePhaseV1::Recycled);
        state.readbacks += 1;
        state
            .observations
            .pop_front()
            .expect("each injected generation has exact observations")
    }

    pub fn release_generation(&self) {
        let mut state = self.0.borrow_mut();
        assert_eq!(state.phase, InjectedQueuePhaseV1::Recycled);
        state.releases += 1;
        state.phase = InjectedQueuePhaseV1::Released;
    }

    pub fn destroy(&self, release: bool) -> bool {
        let mut state = self.0.borrow_mut();
        assert!(matches!(
            state.phase,
            InjectedQueuePhaseV1::Prepared
                | InjectedQueuePhaseV1::Recycled
                | InjectedQueuePhaseV1::Released
        ));
        assert_eq!(
            state.destroys, 0,
            "queue custody can be destroyed only once"
        );
        state.destroys += 1;
        state.phase = if release {
            InjectedQueuePhaseV1::Destroyed
        } else {
            InjectedQueuePhaseV1::Quarantined
        };
        release
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InjectedQueueSnapshotV1 {
    pub phase: InjectedQueuePhaseV1,
    pub submits: usize,
    pub waits: usize,
    pub recycles: usize,
    pub readbacks: usize,
    pub releases: usize,
    pub destroys: usize,
}
