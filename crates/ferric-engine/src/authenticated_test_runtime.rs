use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ModelQueueFailureV1 {
    CurrentnessReleased,
    CurrentnessQuarantined,
    Wait,
    ReadbackReleased,
    ReadbackQuarantined,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ModelQueuePhaseV1 {
    Prepared,
    Published,
    Completed,
    Recycled,
    Released,
    Destroyed,
    Quarantined,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ModelQueueSnapshotV1 {
    pub(crate) phase: ModelQueuePhaseV1,
    pub(crate) submits: usize,
    pub(crate) waits: usize,
    pub(crate) recycles: usize,
    pub(crate) readbacks: usize,
    pub(crate) releases: usize,
    pub(crate) destroys: usize,
}

#[derive(Debug)]
struct ModelQueueStateV1 {
    phase: ModelQueuePhaseV1,
    failures: VecDeque<Option<ModelQueueFailureV1>>,
    readbacks: VecDeque<Vec<ModelMemberReadbackV1>>,
    active_failure: Option<ModelQueueFailureV1>,
    submits: usize,
    waits: usize,
    recycles: usize,
    readback_count: usize,
    releases: usize,
    destroys: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct ModelQueueV1(Rc<RefCell<ModelQueueStateV1>>);

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ModelMemberReadbackV1 {
    pub(crate) accepted: u8,
    pub(crate) emitted: Vec<u32>,
}

impl ModelQueueV1 {
    pub(crate) fn new(
        failures: impl IntoIterator<Item = Option<ModelQueueFailureV1>>,
        readbacks: impl IntoIterator<Item = Vec<ModelMemberReadbackV1>>,
    ) -> Self {
        Self(Rc::new(RefCell::new(ModelQueueStateV1 {
            phase: ModelQueuePhaseV1::Prepared,
            failures: failures.into_iter().collect(),
            readbacks: readbacks.into_iter().collect(),
            active_failure: None,
            submits: 0,
            waits: 0,
            recycles: 0,
            readback_count: 0,
            releases: 0,
            destroys: 0,
        })))
    }

    pub(crate) fn snapshot(&self) -> ModelQueueSnapshotV1 {
        let state = self.0.borrow();
        ModelQueueSnapshotV1 {
            phase: state.phase,
            submits: state.submits,
            waits: state.waits,
            recycles: state.recycles,
            readbacks: state.readback_count,
            releases: state.releases,
            destroys: state.destroys,
        }
    }

    pub(crate) fn publish_for_rollover(&self) {
        let mut state = self.0.borrow_mut();
        assert_eq!(state.phase, ModelQueuePhaseV1::Prepared);
        state.submits += 1;
        state.phase = ModelQueuePhaseV1::Published;
    }

    fn submit(&self) -> Option<ModelQueueFailureV1> {
        let mut state = self.0.borrow_mut();
        assert!(matches!(
            state.phase,
            ModelQueuePhaseV1::Prepared | ModelQueuePhaseV1::Released
        ));
        let failure = state.failures.pop_front().unwrap_or(None);
        if matches!(
            failure,
            Some(
                ModelQueueFailureV1::CurrentnessReleased
                    | ModelQueueFailureV1::CurrentnessQuarantined
            )
        ) {
            state.phase = ModelQueuePhaseV1::Prepared;
            return failure;
        }
        state.submits += 1;
        state.active_failure = failure;
        state.phase = ModelQueuePhaseV1::Published;
        None
    }

    fn wait(&self) -> Result<(), ModelQueueFailureV1> {
        let mut state = self.0.borrow_mut();
        assert_eq!(state.phase, ModelQueuePhaseV1::Published);
        if state.active_failure == Some(ModelQueueFailureV1::Wait) {
            state.phase = ModelQueuePhaseV1::Quarantined;
            return Err(ModelQueueFailureV1::Wait);
        }
        state.waits += 1;
        state.phase = ModelQueuePhaseV1::Completed;
        Ok(())
    }

    fn recycle(&self) {
        let mut state = self.0.borrow_mut();
        assert_eq!(state.phase, ModelQueuePhaseV1::Completed);
        state.recycles += 1;
        state.phase = ModelQueuePhaseV1::Recycled;
    }

    fn readback(&self) -> Result<Vec<ModelMemberReadbackV1>, ModelQueueFailureV1> {
        let mut state = self.0.borrow_mut();
        assert_eq!(state.phase, ModelQueuePhaseV1::Recycled);
        state.readback_count += 1;
        match state.active_failure.take() {
            Some(failure @ (ModelQueueFailureV1::ReadbackReleased
            | ModelQueueFailureV1::ReadbackQuarantined)) => Err(failure),
            Some(ModelQueueFailureV1::Wait) => unreachable!(),
            Some(
                ModelQueueFailureV1::CurrentnessReleased
                | ModelQueueFailureV1::CurrentnessQuarantined,
            ) => unreachable!(),
            None => Ok(state.readbacks.pop_front().unwrap_or_default()),
        }
    }

    pub(crate) fn release_generation(&self) {
        let mut state = self.0.borrow_mut();
        assert_eq!(state.phase, ModelQueuePhaseV1::Recycled);
        state.releases += 1;
        state.phase = ModelQueuePhaseV1::Released;
    }

    fn destroy(&self, releases_cleanly: bool) {
        let mut state = self.0.borrow_mut();
        assert!(matches!(
            state.phase,
            ModelQueuePhaseV1::Prepared
                | ModelQueuePhaseV1::Recycled
                | ModelQueuePhaseV1::Released
        ));
        assert_eq!(state.destroys, 0);
        state.destroys = 1;
        state.phase = if releases_cleanly {
            ModelQueuePhaseV1::Destroyed
        } else {
            ModelQueuePhaseV1::Quarantined
        };
    }
}

#[derive(Debug)]
pub(crate) struct ModelPreparedQueueV1 {
    pub(crate) queue: ModelQueueV1,
}

#[derive(Debug)]
pub(crate) struct ModelPublishedQueueV1 {
    queue: ModelQueueV1,
}

#[derive(Debug)]
pub(crate) struct ModelCompletedQueueV1 {
    queue: ModelQueueV1,
}

#[derive(Debug)]
pub(crate) struct ModelRecycledQueueV1 {
    queue: ModelQueueV1,
}

#[derive(Debug)]
pub(crate) struct ModelDiagnosticV1 {
    pub(crate) queue: ModelQueueV1,
    pub(crate) members: Vec<ModelMemberReadbackV1>,
}

#[derive(Debug)]
pub(crate) struct ModelSubmitFailureV1 {
    pub(crate) prepared: ModelPreparedQueueV1,
    pub(crate) releases_cleanly: bool,
}

#[derive(Debug)]
pub(crate) struct ModelWaitFailureV1 {
    _published: ModelPublishedQueueV1,
}

#[derive(Debug)]
pub(crate) struct ModelReadbackFailureV1 {
    pub(crate) recycled: ModelRecycledQueueV1,
    pub(crate) releases_cleanly: bool,
}

impl ModelPreparedQueueV1 {
    pub(crate) fn new(queue: ModelQueueV1) -> Self {
        Self { queue }
    }

    pub(crate) fn submit(self) -> Result<ModelPublishedQueueV1, ModelSubmitFailureV1> {
        match self.queue.submit() {
            Some(ModelQueueFailureV1::CurrentnessReleased) => Err(ModelSubmitFailureV1 {
                prepared: self,
                releases_cleanly: true,
            }),
            Some(ModelQueueFailureV1::CurrentnessQuarantined) => Err(ModelSubmitFailureV1 {
                prepared: self,
                releases_cleanly: false,
            }),
            Some(_) => unreachable!(),
            None => Ok(ModelPublishedQueueV1 { queue: self.queue }),
        }
    }

    pub(crate) fn destroy(self, releases_cleanly: bool) {
        self.queue.destroy(releases_cleanly);
    }
}

impl ModelPublishedQueueV1 {
    pub(crate) fn from_published(queue: ModelQueueV1) -> Self {
        Self { queue }
    }

    pub(crate) fn wait(self) -> Result<ModelCompletedQueueV1, ModelWaitFailureV1> {
        if self.queue.wait().is_err() {
            return Err(ModelWaitFailureV1 { _published: self });
        }
        Ok(ModelCompletedQueueV1 { queue: self.queue })
    }
}

impl ModelCompletedQueueV1 {
    pub(crate) fn recycle(self) -> ModelRecycledQueueV1 {
        self.queue.recycle();
        ModelRecycledQueueV1 { queue: self.queue }
    }
}

impl ModelRecycledQueueV1 {
    pub(crate) fn readback(self) -> Result<ModelDiagnosticV1, ModelReadbackFailureV1> {
        match self.queue.readback() {
            Ok(members) => Ok(ModelDiagnosticV1 {
                queue: self.queue,
                members,
            }),
            Err(ModelQueueFailureV1::ReadbackReleased) => Err(ModelReadbackFailureV1 {
                recycled: self,
                releases_cleanly: true,
            }),
            Err(ModelQueueFailureV1::ReadbackQuarantined) => Err(ModelReadbackFailureV1 {
                recycled: self,
                releases_cleanly: false,
            }),
            Err(_) => unreachable!(),
        }
    }

    pub(crate) fn destroy(self, releases_cleanly: bool) {
        self.queue.destroy(releases_cleanly);
    }
}
