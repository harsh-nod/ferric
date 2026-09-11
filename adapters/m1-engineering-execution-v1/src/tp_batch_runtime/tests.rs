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
    row_capacity: usize,
    state: Rc<RefCell<FakeState>>,
}

impl EngineeringTpBatchRunnerV2 for FakeRunner {
    fn row_capacity(&self) -> usize {
        self.row_capacity
    }
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
            row_capacity: pool.row_capacity(),
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

#[test]
fn unsupported_diagnostic_snapshot_poison_stops_runtime_but_allows_close() {
    let (mut runtime, state) = runtime(4, 16, false);
    assert!(runtime.runtime_diagnostic_snapshot().is_err());
    assert!(runtime.poisoned);
    assert!(runtime.admit(input(&[1], 1, 0), 0, 0).is_err());
    assert!(state.borrow().calls.is_empty());
    runtime.close().unwrap();
    assert_eq!(state.borrow().close_calls, 1);
    assert!(runtime.runtime_diagnostic_snapshot().is_err());
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

fn replica_schedule_chunks(requests: usize, budget: usize) -> Vec<Vec<(usize, usize)>> {
    let decode = |slots: &[usize]| slots.iter().map(|&slot| (slot, 1)).collect::<Vec<_>>();
    if requests <= 2 {
        let slots = (0..requests).collect::<Vec<_>>();
        let mut batches = vec![slots.iter().map(|&slot| (slot, 5)).collect()];
        batches.extend(vec![decode(&slots); 7]);
        return batches;
    }
    assert_eq!(requests, 8);
    if budget == 16 {
        // A partial prefill advances the round-robin cursor past that request.
        let mut batches = vec![
            vec![(0, 5), (1, 5), (2, 5), (3, 1)],
            vec![(0, 1), (1, 1), (2, 1), (4, 5), (5, 5), (6, 3)],
            vec![
                (4, 1),
                (5, 1),
                (0, 1),
                (1, 1),
                (2, 1),
                (7, 5),
                (3, 4),
                (6, 2),
            ],
        ];
        batches.extend(vec![decode(&[3, 4, 5, 6, 7, 0, 1, 2]); 5]);
        batches.push(decode(&[3, 4, 5, 6, 7]));
        batches.push(decode(&[3, 6, 7]));
        batches
    } else {
        assert_eq!(budget, 32);
        let mut batches = vec![
            vec![(0, 5), (1, 5), (2, 5), (3, 5), (4, 5), (5, 5), (6, 2)],
            vec![
                (0, 1),
                (1, 1),
                (2, 1),
                (3, 1),
                (4, 1),
                (5, 1),
                (7, 5),
                (6, 3),
            ],
        ];
        batches.extend(vec![decode(&[6, 7, 0, 1, 2, 3, 4, 5]); 6]);
        batches.push(decode(&[6, 7]));
        batches
    }
}

#[test]
fn replica_cohorts_preserve_fixed_work_and_exact_real_coordinator_schedules() {
    let prompt = [2, 3, 5, 7, 11];
    let mut choice = prompt
        .iter()
        .fold(0, |value, token| (value * 31 + token) % 997);
    let mut reference = Vec::new();
    for _ in 0..8 {
        reference.push(choice);
        choice = (choice * 32) % 997;
    }
    for (world, requests) in [(8, 8), (2, 2), (1, 1)] {
        for budget in [16, 32] {
            let limits = EngineeringTpPagedLimitsV1::new(64, 32, 16, 1000).unwrap();
            let scope = EngineeringTpPoolScopeV1 {
                model: [1; 32],
                session: [2; 32],
            };
            let pool = if budget == 32 {
                EngineeringTpPagedPoolV1::new_wide32(scope, limits)
            } else {
                EngineeringTpPagedPoolV1::new(scope, limits)
            }
            .unwrap();
            let (gpu, state) = fake(&pool, world);
            let scheduler = if budget == 32 {
                EngineeringTpSchedulerV1::new_wide32(1000, 64, budget, budget)
            } else {
                EngineeringTpSchedulerV1::new(1000, 64, budget, budget)
            }
            .unwrap();
            let mut runtime = if budget == 32 {
                EngineeringTpBatchRuntimeV2::new_wide32(gpu, pool, scheduler, budget, false)
            } else {
                EngineeringTpBatchRuntimeV2::new(gpu, pool, scheduler, budget, false)
            }
            .unwrap();
            let ids = (0..requests)
                .map(|index| {
                    let hit = runtime.admit(input(&prompt, 8, 0), 0, 0).unwrap();
                    assert_eq!(
                        (hit.request.slot, hit.request.generation),
                        (u8::try_from(index).unwrap(), 1)
                    );
                    assert_eq!((hit.cached_tokens, hit.cached_pages), (0, 0));
                    hit.request
                })
                .collect::<Vec<_>>();
            let schedule = replica_schedule_chunks(requests, budget);
            let mut positions = vec![0; requests];
            let mut outputs = vec![Vec::new(); requests];
            let mut retired = vec![false; requests];
            let mut total_rows = 0;
            for (tick, chunks) in schedule.iter().enumerate() {
                let report = step(&mut runtime, u64::try_from(tick).unwrap());
                let expected = chunks
                    .iter()
                    .flat_map(|&(slot, count)| vec![slot; count])
                    .collect::<Vec<_>>();
                assert_eq!(report.rows.len(), expected.len());
                assert!(report.rows.len() <= budget);
                total_rows += report.rows.len();
                let mut produced = report.outputs.iter();
                for (row, slot) in report.rows.iter().zip(expected) {
                    let position = positions[slot];
                    assert_eq!(row.request, ids[slot]);
                    assert_eq!(usize::try_from(row.absolute_position).unwrap(), position);
                    let (token, kind) = match position {
                        0..=3 => (prompt[position], TpBatchRowKindV1::PrefillIntermediate),
                        4 => (prompt[position], TpBatchRowKindV1::PrefillFinal),
                        _ => (reference[position - 5], TpBatchRowKindV1::Decode),
                    };
                    assert_eq!((row.token_id, row.kind), (token, kind));
                    if kind != TpBatchRowKindV1::PrefillIntermediate {
                        let output = produced.next().unwrap();
                        let index = outputs[slot].len();
                        assert_eq!(output.request, ids[slot]);
                        assert_eq!(usize::try_from(output.output_index).unwrap(), index);
                        assert_eq!(output.token_id, reference[index]);
                        assert_eq!(output.completed_ns, u64::try_from(tick).unwrap() * 10 + 1);
                        assert_eq!(output.finished, index == 7);
                        outputs[slot].push(output.completed_ns);
                    }
                    positions[slot] += 1;
                }
                assert!(produced.next().is_none());
                assert_eq!(report.rank_dispatch_counts.len(), world);
                assert_eq!(report.rank_dispatch_counts[0], 544);
                assert!(
                    report.rank_dispatch_counts[1..]
                        .iter()
                        .all(|&count| count == 540)
                );
                for index in 0..requests {
                    if !retired[index] && outputs[index].len() == 8 {
                        let record = runtime
                            .retire(ids[index], u64::try_from(tick).unwrap())
                            .unwrap();
                        assert_eq!(record.state(), TpRequestStateV1::Completed);
                        assert_eq!(record.generated_tokens, reference);
                        assert_eq!(record.output_timestamps_ns, outputs[index]);
                        assert_eq!(record.cached_prefix_tokens, 0);
                        assert_eq!(record.cancelled_ns, None);
                        retired[index] = true;
                    }
                }
            }
            assert_eq!(positions, vec![12; requests]);
            assert_eq!(total_rows * (8 / requests), 96);
            assert_eq!(
                outputs.iter().map(Vec::len).sum::<usize>() * (8 / requests),
                64
            );
            assert!(retired.iter().all(|value| *value));
            let page_totals = runtime.page_stats();
            assert_eq!(
                (
                    page_totals.free_pages,
                    page_totals.cached_pages,
                    page_totals.retained_pages
                ),
                (16, 0, 0)
            );
            assert_eq!((page_totals.prefix_hits, page_totals.hit_tokens), (0, 0));
            assert!(runtime.step(20, 200, || 201).unwrap().is_none());
            runtime.close().unwrap();
            assert_eq!(state.borrow().close_calls, 1);
            assert_eq!(state.borrow().calls.len(), schedule.len());
        }
    }
}

#[test]
fn wide_runtime_checks_physical_capacity_and_executes_one_transaction() {
    for physical_capacity in [16, 32] {
        let pool = EngineeringTpPagedPoolV1::new_wide32(
            EngineeringTpPoolScopeV1 {
                model: [1; 32],
                session: [2; 32],
            },
            EngineeringTpPagedLimitsV1::new(64, 32, 4, 1000).unwrap(),
        )
        .unwrap();
        let (mut gpu, state) = fake(&pool, 8);
        gpu.row_capacity = physical_capacity;
        let scheduler = EngineeringTpSchedulerV1::new_wide32(1000, 64, 32, 32).unwrap();
        let runtime = EngineeringTpBatchRuntimeV2::new_wide32(gpu, pool, scheduler, 32, true);
        if physical_capacity == 16 {
            assert!(runtime.is_err());
            assert_eq!(state.borrow().close_calls, 1);
            continue;
        }
        let mut runtime = runtime.unwrap();
        runtime.admit(input(&[1; 32], 2, 0), 0, 0).unwrap();
        let report = step(&mut runtime, 0);
        assert_eq!(report.rows.len(), 32);
        assert_eq!(report.outputs.len(), 1);
        assert_eq!(state.borrow().calls.len(), 1);
        assert_eq!(state.borrow().calls[0].len(), 32);
        assert_eq!(
            report.rank_dispatch_counts,
            [544, 540, 540, 540, 540, 540, 540, 540]
        );
        runtime.close().unwrap();
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

fn fixed_workload_prompts() -> [Vec<u32>; 4] {
    let base = [785, 6722, 315, 9625, 374];
    let continuation = [
        12095, 13, 576, 6722, 315, 15344, 374, 21718, 13, 576, 6722, 315, 17689, 374,
    ];
    [12, 0, 0, 14].map(|length| {
        base.iter()
            .chain(&continuation[..length])
            .copied()
            .collect()
    })
}

fn fixed_workload_chunks(budget: usize, chunk: usize, cache: bool) -> Vec<Vec<(usize, u32, u32)>> {
    let mut batches = if chunk == 16 {
        vec![
            vec![(0, 0, 16)],
            vec![(1, 0, 5), (0, 16, 1)],
            vec![(0, 17, 1), (1, 5, 1), (2, 0, 5)],
        ]
    } else {
        vec![
            vec![(0, 0, 17)],
            vec![(0, 17, 1), (1, 0, 5)],
            vec![(1, 5, 1), (2, 0, 5)],
        ]
    };
    if cache {
        batches.extend([vec![(1, 6, 1), (3, 16, 3)], vec![(3, 19, 1)]]);
    } else {
        let first = u32::try_from((budget - 1).min(chunk).min(19)).unwrap();
        batches.push(vec![(1, 6, 1), (3, 0, first)]);
        if first < 19 {
            batches.push(vec![(3, first, 19 - first)]);
        }
        batches.push(vec![(3, 19, 1)]);
    }
    batches
}

fn fixed_workload_retire(
    runtime: &mut EngineeringTpBatchRuntimeV2<FakeRunner>,
    active: &mut Vec<(usize, TpRequestIdV1)>,
    records: &mut [Option<TpRequestRecordV1>; 4],
    tick: u64,
) {
    let mut done = Vec::new();
    for &(index, id) in active.iter() {
        if matches!(
            runtime.request(id).unwrap().state(),
            TpRequestStateV1::Completed | TpRequestStateV1::Cancelled
        ) {
            assert!(records[index].is_none());
            records[index] = Some(runtime.retire(id, tick).unwrap());
            assert!(runtime.request(id).is_err());
            done.push(id);
        }
    }
    active.retain(|(_, id)| !done.contains(id));
    assert_eq!(runtime.retained_requests(), active.len());
    assert_eq!(runtime.page_stats().sequences as usize, active.len());
}

fn fixed_workload_run(budget: usize, chunk: usize, cache: bool, cancelled_limit: u32) {
    let limits = EngineeringTpPagedLimitsV1::new(128, 32, 64, 1000).unwrap();
    let scope = EngineeringTpPoolScopeV1 {
        model: [1; 32],
        session: [2; 32],
    };
    let pool = if budget == 16 {
        EngineeringTpPagedPoolV1::new(scope, limits)
    } else {
        EngineeringTpPagedPoolV1::new_wide32(scope, limits)
    }
    .unwrap();
    let (gpu, state) = fake(&pool, 8);
    let scheduler = if budget == 16 {
        EngineeringTpSchedulerV1::new(151_936, 128, budget, chunk)
    } else {
        EngineeringTpSchedulerV1::new_wide32(151_936, 128, budget, chunk)
    }
    .unwrap();
    let mut runtime = if budget == 16 {
        EngineeringTpBatchRuntimeV2::new(gpu, pool, scheduler, budget, cache)
    } else {
        EngineeringTpBatchRuntimeV2::new_wide32(gpu, pool, scheduler, budget, cache)
    }
    .unwrap();
    let prompts = fixed_workload_prompts();
    let requested = [2, 3, cancelled_limit, 2];
    let expected = fixed_workload_chunks(budget, chunk, cache);
    let expected_ids = if chunk == 16 {
        [(0, 1), (1, 1), (2, 1), (0, 2)]
    } else {
        [(0, 1), (1, 1), (0, 2), (2, 1)]
    };
    let mut ids = [None; 4];
    let mut active = Vec::new();
    let mut records = [None, None, None, None];
    let mut observed_stats = Vec::new();
    let mut observed_outputs = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];

    // Match the controller: admit, cancel, retire, step, snapshot, retire.
    for tick in 0..=expected.len() {
        let time = u64::try_from(tick).unwrap();
        if tick < prompts.len() {
            let hit = runtime
                .admit(
                    input(&prompts[tick], requested[tick], time),
                    time,
                    time * 10,
                )
                .unwrap();
            assert_eq!(
                (hit.request.slot, hit.request.generation),
                expected_ids[tick]
            );
            assert_eq!(hit.cached_tokens, if cache && tick == 3 { 16 } else { 0 });
            assert_eq!(hit.cached_pages, u32::from(cache && tick == 3));
            ids[tick] = Some(hit.request);
            active.push((tick, hit.request));
        }
        for &(index, id) in &active {
            if index == 2 && tick >= 3 {
                runtime.cancel(id, time * 10).unwrap();
            }
        }
        fixed_workload_retire(&mut runtime, &mut active, &mut records, time);
        if active.is_empty() {
            assert_eq!(tick, expected.len());
            break;
        }
        let rows = expected[tick]
            .iter()
            .flat_map(|&(index, start, count)| {
                (start..start + count).map(move |position| (index, position))
            })
            .collect::<Vec<_>>();
        let report = step(&mut runtime, time);
        assert_eq!(report.rows.len(), rows.len());
        assert!(report.rows.len() <= budget);
        assert_eq!(report.started_ns, time * 10);
        assert_eq!(report.completed_ns, time * 10 + 1);
        assert_eq!(report.batch_id, time + 1);
        assert_eq!(report.pool_batch_id, time + 1);
        assert_eq!(
            report.rank_dispatch_counts,
            [544, 540, 540, 540, 540, 540, 540, 540]
        );
        let mut output_index = 0;
        for (row, &(index, position)) in report.rows.iter().zip(&rows) {
            let position_index = usize::try_from(position).unwrap();
            let prompt = &prompts[index];
            let kind = if position_index + 1 < prompt.len() {
                TpBatchRowKindV1::PrefillIntermediate
            } else if position_index < prompt.len() {
                TpBatchRowKindV1::PrefillFinal
            } else {
                TpBatchRowKindV1::Decode
            };
            let token = if position_index < prompt.len() {
                prompt[position_index]
            } else {
                observed_outputs[index][position_index - prompt.len()]
            };
            assert_eq!(
                (row.request, row.absolute_position, row.kind, row.token_id),
                (ids[index].unwrap(), position, kind, token)
            );
            if kind != TpBatchRowKindV1::PrefillIntermediate {
                let output = &report.outputs[output_index];
                assert_eq!(output.request, ids[index].unwrap());
                assert_eq!(
                    usize::try_from(output.output_index).unwrap(),
                    observed_outputs[index].len()
                );
                assert_eq!(output.completed_ns, time * 10 + 1);
                observed_outputs[index].push(output.token_id);
                assert_eq!(
                    output.finished,
                    observed_outputs[index].len() == requested[index] as usize
                );
                output_index += 1;
            }
        }
        assert_eq!(report.outputs.len(), output_index);
        let fake_state = state.borrow();
        assert_eq!(fake_state.calls.len(), tick + 1);
        let call = &fake_state.calls[tick];
        assert_eq!(call.len(), rows.len());
        for ((token, position, pages), row) in call.iter().zip(&report.rows) {
            assert_eq!((*token, *position), (row.token_id, row.absolute_position));
            assert!(pages.len() > usize::try_from(position / 16).unwrap());
        }
        if cache && tick == 3 {
            assert_eq!(call[1].2[0], fake_state.calls[0][0].2[0]);
        }
        drop(fake_state);
        observed_stats.push(runtime.page_stats());
        fixed_workload_retire(&mut runtime, &mut active, &mut records, time);
    }

    let mut retained = if chunk == 16 {
        vec![1, 3, 4]
    } else {
        vec![2, 3, 2 + u32::from(cache)]
    };
    retained.push(if cache || (budget == 32 && chunk == 32) {
        3
    } else {
        2
    });
    retained.resize(expected.len(), 2);
    for (tick, (stats, retained)) in observed_stats.iter().zip(retained).enumerate() {
        let cached = u32::from(cache && tick >= if chunk == 16 { 3 } else { 2 });
        let hit = u64::from(cache && tick >= 3);
        assert_eq!(
            (stats.retained_pages, stats.free_pages, stats.cached_pages),
            (retained, 64 - retained, cached)
        );
        assert_eq!(
            (stats.prefix_hits, stats.hit_tokens, stats.hit_pages),
            (hit, hit * 16, hit)
        );
        assert_eq!((stats.evicted_pages, stats.quarantined_pages), (0, 0));
    }
    for (index, record) in records.into_iter().enumerate() {
        let record = record.unwrap();
        let count = [2, 3, 1, 2][index];
        let mut choice = prompts[index]
            .iter()
            .fold(0, |value, token| (value * 31 + token) % 997);
        let mut fake_choices = Vec::new();
        for _ in 0..count {
            fake_choices.push(choice);
            choice = (choice * 32) % 997;
        }
        assert_eq!(record.generated_tokens, fake_choices);
        assert_eq!(record.generated_tokens, observed_outputs[index]);
        assert_eq!(
            record.committed_position as usize,
            prompts[index].len() + count - 1
        );
        let timestamps = expected
            .iter()
            .enumerate()
            .filter(|(_, chunks)| {
                chunks.iter().any(|&(request, start, length)| {
                    request == index && (start + length) as usize >= prompts[index].len()
                })
            })
            .map(|(tick, _)| tick as u64 * 10 + 1)
            .collect::<Vec<_>>();
        assert_eq!(record.output_timestamps_ns, timestamps);
        assert_eq!(record.arrival_tick, index as u64);
        assert_eq!(record.arrival_ns, index as u64 * 10);
        assert_eq!(
            record.cached_prefix_tokens,
            if cache && index == 3 { 16 } else { 0 }
        );
        assert_eq!(
            record.cancelled_ns,
            if index == 2 { Some(30) } else { None }
        );
        assert_eq!(
            record.state(),
            if index == 2 {
                TpRequestStateV1::Cancelled
            } else {
                TpRequestStateV1::Completed
            }
        );
    }
    assert_eq!(runtime.retained_requests(), 0);
    let page_snapshot = runtime.page_stats();
    assert_eq!(page_snapshot.sequences, 0);
    assert_eq!(page_snapshot.cached_pages, u32::from(cache));
    assert_eq!(page_snapshot.retained_pages, u32::from(cache));
    assert_eq!(page_snapshot.free_pages, 64 - u32::from(cache));
    assert_eq!(
        state.borrow().calls.iter().map(Vec::len).sum::<usize>(),
        if cache { 34 } else { 50 }
    );
    assert!(runtime.step(20, 200, || 201).unwrap().is_none());
    runtime.close().unwrap();
    assert_eq!(state.borrow().close_calls, 1);
    assert!(runtime.step(21, 210, || 211).is_err());
}

#[test]
fn fixed_workload_actual_coordinator_matches_cache_and_wide_schedules() {
    for (budget, chunk) in [(16, 16), (17, 17), (32, 16), (32, 32)] {
        for cache in [false, true] {
            // The controller's existing requested-four cancellation has the
            // same schedule as the requested-three variant: both cancel at 3.
            for cancelled_limit in [3, 4] {
                fixed_workload_run(budget, chunk, cache, cancelled_limit);
            }
        }
    }
}
