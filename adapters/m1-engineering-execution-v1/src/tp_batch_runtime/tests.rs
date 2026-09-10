//! Host boundary tests with actual scheduler/pool/coordinator bodies and a fake GPU.
//! The fake stores tokens in physical pages to detect stale-prefix/causal mistakes;
//! its deterministic choices are not Qwen numerics or evidence of GPU execution.

use std::cell::RefCell;
use std::rc::Rc;

use super::*;
use crate::tp_paged::{
    EngineeringTpBatchCompletionV1, EngineeringTpPagedLimitsV1, EngineeringTpPoolScopeV1,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fault {
    None,
    PartialExecution,
    WrongChoiceCount,
    InvalidToken,
    ForeignCompletion,
    WrongRankCount,
    WrongDispatchDelta,
    RegressDispatch,
}

#[derive(Debug)]
struct FakeState {
    pages: Vec<[Option<u32>; 16]>,
    calls: Vec<Vec<(u32, u32, Vec<u32>)>>,
    counts: Vec<u64>,
    fault: Fault,
    close_calls: usize,
}

struct FakeRunner {
    pool: u64,
    state: Rc<RefCell<FakeState>>,
}

impl EngineeringTpBatchRunnerV2 for FakeRunner {
    fn execute_batch(
        &mut self,
        batch: &EngineeringTpPreparedBatchV1,
        output_rows: &[usize],
    ) -> TpResult<EngineeringTpBatchOutputV2> {
        if batch.pool_identity() != self.pool {
            return Err("fake runner pool binding mismatch".into());
        }
        let mut state = self.state.borrow_mut();
        state.calls.push(
            batch
                .rows()
                .iter()
                .map(|row| (row.token(), row.position(), row.physical_pages().to_vec()))
                .collect(),
        );
        for (index, row) in batch.rows().iter().enumerate() {
            let page = usize::try_from(row.writable_physical_page()).unwrap();
            let offset = usize::try_from(row.writable_token_offset()).unwrap();
            state.pages[page][offset] = Some(row.token());
            if state.fault == Fault::PartialExecution && index == 0 {
                return Err("injected failure after partial physical work".into());
            }
        }
        let mut choices = Vec::new();
        for row in batch.rows() {
            let mut choice = 0_u32;
            for position in 0..=row.position() {
                let page = row.physical_pages()[usize::try_from(position / 16).unwrap()];
                let token = state.pages[usize::try_from(page).unwrap()]
                    [usize::try_from(position % 16).unwrap()]
                .ok_or("fake runner read uninitialized causal prefix")?;
                choice = (choice * 31 + token) % 997;
            }
            choices.push(choice);
        }
        let mut choices = output_rows
            .iter()
            .map(|&row| choices[row])
            .collect::<Vec<_>>();
        for (rank, count) in state.counts.iter_mut().enumerate() {
            *count += if rank == 0 { 544 } else { 540 };
        }
        match state.fault {
            Fault::WrongChoiceCount => {
                choices.pop();
            }
            Fault::InvalidToken => choices.fill(u32::MAX),
            Fault::WrongRankCount => {
                state.counts.pop();
            }
            Fault::WrongDispatchDelta => state.counts[0] -= 1,
            Fault::RegressDispatch => state.counts[0] = 0,
            _ => {}
        }
        let completion = if state.fault == Fault::ForeignCompletion {
            let mut foreign = new_pool(64, 4);
            let sequence = foreign
                .open_sequence(foreign.scope(), &[1], 0)
                .unwrap()
                .sequence();
            let prepared = foreign
                .reserve_batch(&[EngineeringTpPageRowV1 {
                    sequence,
                    token: 1,
                    position: 0,
                }])
                .unwrap();
            EngineeringTpBatchCompletionV1::after_all_ranks(&prepared)
        } else {
            EngineeringTpBatchCompletionV1::after_all_ranks(batch)
        };
        Ok(EngineeringTpBatchOutputV2 {
            choices,
            completion,
        })
    }

    fn dispatch_counts(&self) -> Vec<u64> {
        self.state.borrow().counts.clone()
    }

    fn close(&mut self) -> TpResult<()> {
        self.state.borrow_mut().close_calls += 1;
        Ok(())
    }
}

fn new_pool(context: u32, pages: u32) -> EngineeringTpPagedPoolV1 {
    EngineeringTpPagedPoolV1::new(
        EngineeringTpPoolScopeV1 {
            model: [1; 32],
            session: [2; 32],
        },
        EngineeringTpPagedLimitsV1::new(context, 32, pages, 1000).unwrap(),
    )
    .unwrap()
}

fn fake(pool: &EngineeringTpPagedPoolV1, ranks: usize) -> (FakeRunner, Rc<RefCell<FakeState>>) {
    let state = Rc::new(RefCell::new(FakeState {
        pages: vec![[None; 16]; usize::try_from(pool.limits().physical_page_count()).unwrap()],
        calls: Vec::new(),
        counts: vec![0; ranks],
        fault: Fault::None,
        close_calls: 0,
    }));
    (
        FakeRunner {
            pool: pool.identity(),
            state: Rc::clone(&state),
        },
        state,
    )
}

fn runtime(
    pages: u32,
    rows: usize,
    retain: bool,
) -> (
    EngineeringTpBatchRuntimeV2<FakeRunner>,
    Rc<RefCell<FakeState>>,
) {
    let pool = new_pool(64, pages);
    let (gpu, state) = fake(&pool, 8);
    let scheduler = EngineeringTpSchedulerV1::new(1000, 64, rows, rows).unwrap();
    (
        EngineeringTpBatchRuntimeV2::new(gpu, pool, scheduler, rows, retain).unwrap(),
        state,
    )
}

fn input(prompt: &[u32], new_tokens: u32, tick: u64) -> TpRequestAdmissionV1 {
    TpRequestAdmissionV1 {
        prompt_tokens: prompt.to_vec(),
        max_new_tokens: new_tokens,
        cached_prefix_tokens: 0,
        arrival_tick: tick,
        arrival_ns: tick * 10,
    }
}

fn step(
    runtime: &mut EngineeringTpBatchRuntimeV2<FakeRunner>,
    tick: u64,
) -> EngineeringTpBatchReportV2 {
    runtime
        .step(tick, tick * 10, || tick * 10 + 1)
        .unwrap()
        .unwrap()
}

fn finish(
    runtime: &mut EngineeringTpBatchRuntimeV2<FakeRunner>,
    ids: &[TpRequestIdV1],
    mut tick: u64,
) -> u64 {
    for _ in 0..64 {
        if ids
            .iter()
            .all(|&id| runtime.request(id).unwrap().state() == TpRequestStateV1::Completed)
        {
            return tick;
        }
        step(runtime, tick);
        tick += 1;
    }
    panic!("bounded fake request trace did not complete");
}

#[test]
fn mixed_rows_support_continuous_arrival_early_retirement_and_generation_reuse() {
    let (mut runtime, state) = runtime(16, 4, true);
    let long = runtime.admit(input(&[1], 5, 0), 0, 0).unwrap().request;
    step(&mut runtime, 0);
    let short = runtime.admit(input(&[2, 3], 1, 1), 1, 10).unwrap().request;
    let report = step(&mut runtime, 1);
    assert_eq!(
        report.rows.iter().map(|row| row.kind).collect::<Vec<_>>(),
        [
            TpBatchRowKindV1::Decode,
            TpBatchRowKindV1::PrefillIntermediate,
            TpBatchRowKindV1::PrefillFinal
        ]
    );
    assert_eq!(report.outputs.len(), 2);
    assert_eq!(report.outputs[0].request, long);
    assert!(!report.outputs[0].finished);
    assert_eq!(report.outputs[1].request, short);
    assert!(report.outputs[1].finished);
    assert_eq!(
        report.rank_dispatch_counts,
        [544, 540, 540, 540, 540, 540, 540, 540]
    );
    assert_eq!(report.started_ns, 10);
    assert_eq!(report.completed_ns, 11);
    let retired = runtime.retire(short, 2).unwrap();
    assert_eq!(retired.output_timestamps_ns[0] - retired.arrival_ns, 1);
    let replacement = runtime.admit(input(&[4], 1, 2), 2, 20).unwrap().request;
    assert_eq!(replacement.slot, short.slot);
    assert!(replacement.generation > short.generation);
    assert!(runtime.request(short).is_err());
    assert_eq!(
        runtime.request(long).unwrap().state(),
        TpRequestStateV1::Decode
    );
    let tick = finish(&mut runtime, &[long, replacement], 2);
    assert_eq!(
        runtime.retire(long, tick).unwrap().generated_tokens.len(),
        5
    );
    runtime.retire(replacement, tick).unwrap();
    assert_eq!(runtime.retained_requests(), 0);
    assert_eq!(runtime.page_stats().sequences, 0);
    runtime.close().unwrap();
    assert_eq!(state.borrow().close_calls, 1);
}

#[test]
fn retired_radix_prefix_reuses_physical_tokens_and_matches_uncached_outputs() {
    let prompt = (1..=33).collect::<Vec<u32>>();
    let mut observed = Vec::new();
    for retain in [false, true] {
        let (mut runtime, state) = runtime(8, 16, retain);
        let cold = runtime.admit(input(&prompt, 2, 0), 0, 0).unwrap();
        assert_eq!(cold.cached_tokens, 0);
        let tick = finish(&mut runtime, &[cold.request], 0);
        let cold_record = runtime.retire(cold.request, tick).unwrap();
        let prior_calls = state.borrow().calls.len();
        let repeated = runtime
            .admit(input(&prompt, 2, tick), tick, tick * 10)
            .unwrap();
        assert_eq!(repeated.cached_tokens, if retain { 32 } else { 0 });
        assert_eq!(repeated.cached_pages, if retain { 2 } else { 0 });
        let first = step(&mut runtime, tick);
        assert_eq!(first.rows[0].absolute_position, repeated.cached_tokens);
        if retain {
            assert_eq!(first.rows.len(), 1);
            assert_eq!(first.rows[0].kind, TpBatchRowKindV1::PrefillFinal);
            assert_eq!(first.outputs.len(), 1);
        }
        let end = finish(&mut runtime, &[repeated.request], tick + 1);
        let repeated_record = runtime.retire(repeated.request, end).unwrap();
        assert_eq!(
            repeated_record.generated_tokens,
            cold_record.generated_tokens
        );
        if retain {
            assert_eq!(state.borrow().calls.len() - prior_calls, 2);
        }
        observed.push(repeated_record.generated_tokens);
        runtime.close().unwrap();
    }
    assert_eq!(observed[0], observed[1]);
}

#[test]
fn cancellation_keeps_slot_until_pool_release_and_does_not_cache_partial_request() {
    let (mut runtime, state) = runtime(4, 16, true);
    let id = runtime.admit(input(&[1; 33], 1, 0), 0, 0).unwrap().request;
    assert!(step(&mut runtime, 0).outputs.is_empty());
    assert_eq!(runtime.request(id).unwrap().committed_position, 16);
    assert!(runtime.retire(id, 1).is_err());
    runtime.cancel(id, 2).unwrap();
    assert_eq!(runtime.page_stats().sequences, 1);
    assert_eq!(runtime.retained_requests(), 1);
    let cancelled = runtime.retire(id, 1).unwrap();
    assert_eq!(cancelled.state(), TpRequestStateV1::Cancelled);
    assert!(cancelled.generated_tokens.is_empty());
    assert_eq!(runtime.page_stats().cached_pages, 0);
    assert_eq!(runtime.page_stats().free_pages, 4);
    let next = runtime.admit(input(&[1; 33], 1, 1), 1, 10).unwrap();
    assert_eq!(next.cached_tokens, 0);
    assert_eq!(next.request.slot, id.slot);
    assert!(next.request.generation > id.generation);
    assert!(runtime.cancel(id, 10).is_err());
    runtime.close().unwrap();
    assert_eq!(state.borrow().calls.len(), 1);
}

#[test]
fn out_of_pages_backs_off_before_submission_and_remains_usable() {
    let (mut runtime, state) = runtime(1, 16, false);
    let first = runtime.admit(input(&[1], 1, 0), 0, 0).unwrap().request;
    let second = runtime.admit(input(&[2], 1, 0), 0, 0).unwrap().request;
    let report = step(&mut runtime, 0);
    assert_eq!(report.rows.len(), 1);
    assert_eq!(report.outputs[0].request, first);
    assert_eq!(state.borrow().calls.len(), 1);
    assert!(
        report.batch_id > 1,
        "smaller budgets consume distinct scheduler batch IDs"
    );
    assert_eq!(runtime.request(second).unwrap().committed_position, 0);
    runtime.retire(first, 1).unwrap();
    assert_eq!(step(&mut runtime, 1).outputs[0].request, second);
    runtime.retire(second, 2).unwrap();
    assert_eq!(runtime.page_stats().free_pages, 1);
    runtime.close().unwrap();
}

#[test]
fn out_of_pages_evicts_unused_cache_before_shrinking_a_fitting_batch() {
    let (mut runtime, state) = runtime(2, 16, true);
    let prefix = runtime.admit(input(&[1; 17], 1, 0), 0, 0).unwrap().request;
    let tick = finish(&mut runtime, &[prefix], 0);
    runtime.retire(prefix, tick).unwrap();
    assert_eq!(runtime.page_stats().cached_pages, 1);
    assert_eq!(runtime.page_stats().free_pages, 1);
    runtime
        .admit(input(&[2], 1, tick), tick, tick * 10)
        .unwrap();
    runtime
        .admit(input(&[3], 1, tick), tick, tick * 10)
        .unwrap();
    let before = state.borrow().calls.len();
    let report = step(&mut runtime, tick);
    assert_eq!(report.rows.len(), 2);
    assert_eq!(report.outputs.len(), 2);
    assert_eq!(state.borrow().calls.len(), before + 1);
    assert_eq!(runtime.page_stats().evicted_pages, 1);
    assert_eq!(runtime.page_stats().cached_pages, 0);
    runtime.close().unwrap();
}

#[test]
fn pre_submission_backpressure_can_be_resolved_by_explicit_cancellation() {
    let (mut runtime, state) = runtime(1, 16, false);
    let occupying = runtime.admit(input(&[1], 20, 0), 0, 0).unwrap().request;
    step(&mut runtime, 0);
    let waiting = runtime.admit(input(&[2], 1, 1), 1, 10).unwrap().request;
    step(&mut runtime, 1);
    let before = state.borrow().calls.len();
    assert!(runtime.step(2, 20, || 21).is_err());
    assert_eq!(state.borrow().calls.len(), before);
    assert_eq!(runtime.request(waiting).unwrap().committed_position, 0);
    runtime.cancel(occupying, 21).unwrap();
    runtime.retire(occupying, 3).unwrap();
    assert_eq!(step(&mut runtime, 3).outputs[0].request, waiting);
    runtime.close().unwrap();
}

#[test]
fn submitted_failures_and_malformed_outputs_quarantine_before_any_publication() {
    for fault in [
        Fault::PartialExecution,
        Fault::WrongChoiceCount,
        Fault::InvalidToken,
        Fault::ForeignCompletion,
        Fault::WrongRankCount,
        Fault::WrongDispatchDelta,
    ] {
        let (mut runtime, state) = runtime(4, 16, true);
        let id = runtime.admit(input(&[1, 2], 1, 0), 0, 0).unwrap().request;
        state.borrow_mut().fault = fault;
        assert!(runtime.step(0, 0, || 1).is_err(), "{fault:?}");
        assert_eq!(
            runtime.request(id).unwrap().committed_position,
            0,
            "{fault:?}"
        );
        assert!(
            runtime.request(id).unwrap().generated_tokens.is_empty(),
            "{fault:?}"
        );
        assert!(
            runtime.request(id).unwrap().output_timestamps_ns.is_empty(),
            "{fault:?}"
        );
        assert_eq!(runtime.page_stats().quarantined_pages, 4, "{fault:?}");
        assert_eq!(runtime.page_stats().free_pages, 0, "{fault:?}");
        assert!(runtime.step(1, 10, || 11).is_err());
        assert!(runtime.admit(input(&[3], 1, 1), 1, 10).is_err());
        assert!(runtime.cancel(id, 10).is_err());
        assert!(runtime.retire(id, 1).is_err());
        assert_eq!(state.borrow().calls.len(), 1);
        runtime.close().unwrap();
        assert_eq!(state.borrow().close_calls, 1);
    }
}

#[test]
fn regressing_completion_clock_or_counter_does_not_advance_an_existing_stream() {
    for fault in [Fault::None, Fault::RegressDispatch] {
        let (mut runtime, state) = runtime(4, 16, false);
        let id = runtime.admit(input(&[1], 2, 0), 0, 0).unwrap().request;
        step(&mut runtime, 0);
        let before = runtime.request(id).unwrap().clone();
        state.borrow_mut().fault = fault;
        let completed = if fault == Fault::None { 9 } else { 11 };
        assert!(runtime.step(1, 10, || completed).is_err());
        assert_eq!(runtime.request(id).unwrap(), &before);
        assert_eq!(runtime.page_stats().quarantined_pages, 4);
        runtime.close().unwrap();
    }
}

#[test]
fn constructor_rejects_context_budget_and_initial_counter_mismatches() {
    for (scheduler_context, scheduler_rows, budget, ranks, nonzero) in [
        (32, 16, 16, 8, false),
        (64, 1, 16, 8, false),
        (64, 16, 0, 8, false),
        (64, 16, 17, 8, false),
        (64, 16, 16, 0, false),
        (64, 16, 16, 3, false),
        (64, 16, 16, 8, true),
    ] {
        let pool = new_pool(64, 4);
        let (gpu, state) = fake(&pool, ranks);
        if nonzero {
            state.borrow_mut().counts.fill(1);
        }
        let scheduler =
            EngineeringTpSchedulerV1::new(1000, scheduler_context, scheduler_rows, scheduler_rows)
                .unwrap();
        assert!(EngineeringTpBatchRuntimeV2::new(gpu, pool, scheduler, budget, true).is_err());
        assert_eq!(state.borrow().close_calls, 1);
    }
}

#[test]
fn failed_admission_releases_pool_sequence_and_ignores_claimed_cache_authority() {
    let (mut runtime, state) = runtime(4, 16, true);
    assert!(
        runtime
            .admit(
                TpRequestAdmissionV1 {
                    cached_prefix_tokens: 1,
                    ..input(&[1, 2], 1, 0)
                },
                0,
                0
            )
            .is_err()
    );
    assert_eq!(runtime.page_stats().sequences, 0);
    assert!(runtime.admit(input(&[1000], 1, 0), 0, 0).is_err());
    assert_eq!(runtime.page_stats().sequences, 0);
    assert_eq!(runtime.retained_requests(), 0);
    assert!(
        runtime
            .admit(
                TpRequestAdmissionV1 {
                    arrival_ns: 10,
                    ..input(&[1], 1, 0)
                },
                0,
                0
            )
            .is_err()
    );
    assert_eq!(runtime.page_stats().sequences, 0);
    let id = runtime.admit(input(&[1], 1, 0), 0, 0).unwrap().request;
    assert!(step(&mut runtime, 0).outputs[0].finished);
    runtime.retire(id, 1).unwrap();
    runtime.close().unwrap();
    assert!(runtime.admit(input(&[1], 1, 1), 1, 10).is_err());
    assert!(runtime.step(1, 10, || 11).is_err());
    assert_eq!(state.borrow().close_calls, 1);
}

#[test]
fn missing_internal_binding_aborts_scheduler_reservation_and_stops_reuse() {
    let (mut runtime, state) = runtime(4, 16, true);
    let id = runtime.admit(input(&[1], 1, 0), 0, 0).unwrap().request;
    runtime.bindings.clear();
    assert!(runtime.step(0, 0, || 1).is_err());
    assert!(runtime.poisoned);
    assert_eq!(
        runtime.scheduler.abort(1),
        Err(crate::tp_scheduler::TpSchedulerErrorV1::StaleBatch)
    );
    assert_eq!(runtime.request(id).unwrap().committed_position, 0);
    assert_eq!(runtime.page_stats().free_pages, 4);
    assert!(state.borrow().calls.is_empty());
    runtime.close().unwrap();
}
