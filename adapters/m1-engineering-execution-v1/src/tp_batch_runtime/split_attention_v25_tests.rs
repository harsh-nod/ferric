//! Batch-aware counter preflight with a fake runner; no kernel or Qwen numerical evidence.

use super::*;
use crate::tp_paged::{
    EngineeringTpBatchCompletionV1, EngineeringTpPagedLimitsV1, EngineeringTpPoolScopeV1,
};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
struct State {
    reject_preflight: bool,
    wrong_delta: bool,
    calls: Vec<&'static str>,
    dispatches: u64,
}

struct Runner(Rc<RefCell<State>>);

impl EngineeringTpBatchRunnerV2 for Runner {
    fn expected_dispatch_counts_for_batch(
        &self,
        batch: &EngineeringTpPreparedBatchV1,
        published: usize,
    ) -> TpResult<Vec<u64>> {
        let mut state = self.0.borrow_mut();
        state.calls.push("preflight");
        assert_eq!((batch.rows().len(), published), (1, 1));
        assert_eq!(batch.rows()[0].position(), 0);
        if state.reject_preflight {
            return Err("injected batch expectation rejection".into());
        }
        Ok(vec![652])
    }

    fn bind_numerical_rows(&mut self, _batch: u64, _rows: &[TpBatchRowV1]) -> TpResult<()> {
        self.0.borrow_mut().calls.push("bind");
        Ok(())
    }

    fn execute_batch(
        &mut self,
        batch: &EngineeringTpPreparedBatchV1,
        _output_rows: &[usize],
    ) -> TpResult<EngineeringTpBatchOutputV2> {
        let mut state = self.0.borrow_mut();
        state.calls.push("execute");
        let delta = if state.wrong_delta { 616 } else { 652 };
        state.dispatches += delta;
        Ok(EngineeringTpBatchOutputV2 {
            choices: vec![42],
            completion: EngineeringTpBatchCompletionV1::after_all_ranks(batch),
        })
    }

    fn dispatch_counts(&self) -> Vec<u64> {
        vec![self.0.borrow().dispatches]
    }
    fn close(&mut self) -> TpResult<()> {
        self.0.borrow_mut().calls.push("close");
        Ok(())
    }
}

fn fixture() -> (EngineeringTpBatchRuntimeV2<Runner>, Rc<RefCell<State>>) {
    let state = Rc::new(RefCell::new(State::default()));
    let pool = EngineeringTpPagedPoolV1::new(
        EngineeringTpPoolScopeV1 {
            model: [1; 32],
            session: [2; 32],
        },
        EngineeringTpPagedLimitsV1::new(64, 1, 4, 1000).unwrap(),
    )
    .unwrap();
    let scheduler = EngineeringTpSchedulerV1::new(1000, 64, 1, 1).unwrap();
    let mut runtime =
        EngineeringTpBatchRuntimeV2::new(Runner(state.clone()), pool, scheduler, 1, false).unwrap();
    runtime
        .admit(
            TpRequestAdmissionV1 {
                prompt_tokens: vec![1],
                max_new_tokens: 1,
                cached_prefix_tokens: 0,
                arrival_tick: 0,
                arrival_ns: 0,
            },
            0,
            0,
        )
        .unwrap();
    (runtime, state)
}

#[test]
fn split_attention_v25_batch_preflight_aborts_without_submission_or_progress_and_can_retry() {
    let (mut runtime, state) = fixture();
    state.borrow_mut().reject_preflight = true;
    let pages = runtime.pool.stats();
    assert!(
        runtime
            .step(1, 10, || panic!("no completion after rejected preflight"))
            .is_err()
    );
    assert_eq!(state.borrow().calls, ["preflight"]);
    assert_eq!(state.borrow().dispatches, 0);
    assert_eq!(runtime.pool.stats(), pages);
    assert!(!runtime.poisoned);
    runtime.pool.check_invariants().unwrap();
    state.borrow_mut().reject_preflight = false;
    let report = runtime.step(2, 20, || 30).unwrap().unwrap();
    assert_eq!(report.rank_dispatch_counts, [652]);
    assert_eq!(
        state.borrow().calls,
        ["preflight", "preflight", "bind", "execute"]
    );
    runtime.close().unwrap();
}

#[test]
fn split_attention_v25_runtime_uses_new_independent_counts_and_rejects_legacy_delta() {
    for wrong_delta in [false, true] {
        let (mut runtime, state) = fixture();
        state.borrow_mut().wrong_delta = wrong_delta;
        let result = runtime.step(1, 10, || 20);
        assert_eq!(result.is_ok(), !wrong_delta);
        assert_eq!(runtime.poisoned, wrong_delta);
        assert_eq!(state.borrow().calls, ["preflight", "bind", "execute"]);
        if wrong_delta {
            assert!(runtime.pool.stats().quarantined_pages > 0);
        }
        runtime.close().unwrap();
    }
}
