//! Actual coordinator rollback with an explicit fake phase consumer, no device execution.

use super::*;

struct PhaseRunner {
    inner: FakeRunner,
    pending: Option<(u64, u64)>,
    bindings: Vec<(u64, Vec<TpBatchRowKindV1>)>,
    abandons: usize,
    fail_bind: bool,
    fail_count: bool,
    fail_numerical: bool,
}

impl EngineeringTpBatchRunnerV2 for PhaseRunner {
    fn bind_dispatch_rows(
        &mut self,
        batch: &EngineeringTpPreparedBatchV1,
        rows: &[TpBatchRowV1],
    ) -> TpResult<()> {
        if self.pending.is_some() {
            return Err("stale phase binding".into());
        }
        self.pending = Some((batch.pool_identity(), batch.id()));
        self.bindings
            .push((batch.id(), rows.iter().map(|row| row.kind).collect()));
        if self.fail_bind {
            Err("injected phase binding failure".into())
        } else {
            Ok(())
        }
    }
    fn abandon_dispatch_rows(&mut self) {
        self.pending = None;
        self.abandons += 1;
    }
    fn bind_numerical_rows(&mut self, _: u64, _: &[TpBatchRowV1]) -> TpResult<()> {
        if self.fail_numerical {
            Err("injected post-submission diagnostic failure".into())
        } else {
            Ok(())
        }
    }
    fn expected_dispatch_counts_for_selection(
        &self,
        batch: &EngineeringTpPreparedBatchV1,
        _: &[usize],
    ) -> TpResult<Vec<u64>> {
        assert_eq!(self.pending, Some((batch.pool_identity(), batch.id())));
        if self.fail_count {
            Err("injected dispatch preflight failure".into())
        } else {
            Ok(vec![544])
        }
    }
    fn row_capacity(&self) -> usize {
        self.inner.row_capacity()
    }
    fn execute_batch(
        &mut self,
        batch: &EngineeringTpPreparedBatchV1,
        output_rows: &[usize],
    ) -> TpResult<EngineeringTpBatchOutputV2> {
        assert_eq!(
            self.pending.take(),
            Some((batch.pool_identity(), batch.id()))
        );
        self.inner.execute_batch(batch, output_rows)
    }
    fn dispatch_counts(&self) -> Vec<u64> {
        self.inner.dispatch_counts()
    }
    fn close(&mut self) -> TpResult<()> {
        self.pending = None;
        self.inner.close()
    }
}

fn phase_runtime() -> EngineeringTpBatchRuntimeV2<PhaseRunner> {
    let pool = new_pool(64, 4);
    let (inner, _) = fake(&pool, 1);
    let gpu = PhaseRunner {
        inner,
        pending: None,
        bindings: Vec::new(),
        abandons: 0,
        fail_bind: false,
        fail_count: false,
        fail_numerical: false,
    };
    let scheduler = EngineeringTpSchedulerV1::new(1000, 64, 16, 16).unwrap();
    EngineeringTpBatchRuntimeV2::new(gpu, pool, scheduler, 16, false).unwrap()
}

#[test]
fn dispatch_phase_preflight_abort_clears_binding_and_allows_fresh_retry() {
    for fail_bind in [false, true] {
        let mut runtime = phase_runtime();
        runtime.admit(input(&[1], 2, 0), 0, 0).unwrap();
        runtime.gpu.fail_bind = fail_bind;
        runtime.gpu.fail_count = !fail_bind;
        assert!(runtime.step(0, 0, || 1).is_err());
        assert!(!runtime.poisoned);
        assert_eq!(runtime.gpu.pending, None);
        assert_eq!(runtime.gpu.abandons, 1);
        assert!(runtime.gpu.inner.state.borrow().calls.is_empty());
        runtime.gpu.fail_bind = false;
        runtime.gpu.fail_count = false;
        let first = runtime.step(1, 10, || 11).unwrap().unwrap();
        assert_eq!(first.rows[0].kind, TpBatchRowKindV1::PrefillFinal);
        assert_eq!(runtime.gpu.pending, None);
        let next = runtime.step(2, 20, || 21).unwrap().unwrap();
        assert_eq!(next.rows[0].kind, TpBatchRowKindV1::Decode);
        assert_eq!(runtime.gpu.bindings.len(), 3);
        assert!(
            runtime
                .gpu
                .bindings
                .windows(2)
                .all(|pair| pair[0].0 < pair[1].0)
        );
        assert_eq!(runtime.gpu.pending, None);
    }
}

#[test]
fn dispatch_phase_submitted_failure_clears_binding_and_stops_reuse() {
    let mut runtime = phase_runtime();
    runtime.admit(input(&[1], 2, 0), 0, 0).unwrap();
    runtime.gpu.fail_numerical = true;
    assert!(runtime.step(0, 0, || 1).is_err());
    assert!(runtime.poisoned);
    assert_eq!(runtime.gpu.pending, None);
    assert_eq!(runtime.gpu.abandons, 1);
    assert!(runtime.gpu.inner.state.borrow().calls.is_empty());
    assert!(runtime.step(1, 10, || 11).is_err());
}
