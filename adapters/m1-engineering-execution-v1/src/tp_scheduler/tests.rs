use super::*;

#[test]
fn explicit_wide_scheduler_emits_one_thirty_two_row_physical_batch() {
    assert!(EngineeringTpSchedulerV1::new(1000, 64, 32, 32).is_err());
    assert!(EngineeringTpSchedulerV1::new_wide32(1000, 64, 33, 32).is_err());
    let mut scheduler = EngineeringTpSchedulerV1::new_wide32(1000, 64, 32, 32).unwrap();
    scheduler.admit(admission(&[1; 32], 2), 0).unwrap();
    let (batch, outputs) = step(&mut scheduler, 0, 32);
    assert_eq!(batch.rows().len(), 32);
    assert_eq!(outputs.len(), 1);
    assert_eq!(batch.rows()[31].kind, TpBatchRowKindV1::PrefillFinal);
    assert!(
        batch.rows()[..31]
            .iter()
            .all(|row| row.kind == TpBatchRowKindV1::PrefillIntermediate)
    );
}

fn scheduler(rows: usize, chunk: usize) -> EngineeringTpSchedulerV1 {
    EngineeringTpSchedulerV1::new(1000, 8192, rows, chunk).unwrap()
}

fn admission(prompt: &[u32], outputs: u32) -> TpRequestAdmissionV1 {
    TpRequestAdmissionV1 {
        prompt_tokens: prompt.to_vec(),
        max_new_tokens: outputs,
        cached_prefix_tokens: 0,
        arrival_tick: 0,
        arrival_ns: 0,
    }
}

fn choices(batch: &TpScheduledBatchV1) -> Vec<TpRowChoiceV1> {
    batch
        .rows()
        .iter()
        .enumerate()
        .filter(|(_, row)| row.kind != TpBatchRowKindV1::PrefillIntermediate)
        .map(|(row_index, row)| TpRowChoiceV1 {
            row_index,
            token_id: row.absolute_position + 19,
        })
        .collect()
}

fn step(
    scheduler: &mut EngineeringTpSchedulerV1,
    tick: u64,
    budget: usize,
) -> (TpScheduledBatchV1, Vec<TpOutputEventV1>) {
    let batch = scheduler.prepare(tick, tick * 10, budget).unwrap().unwrap();
    let events = scheduler
        .complete(batch.id(), &choices(&batch), tick * 10 + 1)
        .unwrap();
    (batch, events)
}

fn snapshot(scheduler: &EngineeringTpSchedulerV1) -> String {
    format!("{scheduler:?}")
}

#[test]
fn admission_bounds_are_atomic_and_leave_a_final_prompt_row() {
    for config in [
        (0, 8192, 16, 16),
        (1000, 0, 16, 16),
        (1000, 8193, 16, 16),
        (1000, 8192, 0, 1),
        (1000, 8192, 17, 1),
        (1000, 8192, 16, 0),
        (1000, 8192, 2, 3),
    ] {
        assert!(matches!(
            EngineeringTpSchedulerV1::new(config.0, config.1, config.2, config.3),
            Err(TpSchedulerErrorV1::InvalidConfiguration)
        ));
    }
    let mut scheduler = scheduler(16, 16);
    for request in [
        admission(&[], 1),
        admission(&[1], 0),
        admission(&[1000], 1),
        admission(&vec![1; 8193], 1),
        admission(&[1], u32::MAX),
        admission(&vec![1; 8192], 2),
        TpRequestAdmissionV1 {
            cached_prefix_tokens: 1,
            ..admission(&[1], 1)
        },
    ] {
        let before = snapshot(&scheduler);
        assert_eq!(
            scheduler.admit(request, 0),
            Err(TpSchedulerErrorV1::InvalidAdmission)
        );
        assert_eq!(snapshot(&scheduler), before);
    }
    assert!(scheduler.admit(admission(&vec![1; 8192], 1), 0).is_ok());
    assert!(scheduler.admit(admission(&[1], 8192), 0).is_ok());
}

#[test]
fn chunked_prefill_publishes_only_final_row_then_consumes_last_choice() {
    let mut scheduler = scheduler(2, 2);
    let id = scheduler.admit(admission(&[1, 2, 3, 4, 5], 3), 0).unwrap();
    let (first, events) = step(&mut scheduler, 0, 2);
    assert_eq!(
        first
            .rows()
            .iter()
            .map(|row| row.token_id)
            .collect::<Vec<_>>(),
        [1, 2]
    );
    assert!(
        first
            .rows()
            .iter()
            .all(|row| row.kind == TpBatchRowKindV1::PrefillIntermediate)
    );
    assert!(events.is_empty());
    assert_eq!(scheduler.request(id).unwrap().committed_position, 2);
    assert!(step(&mut scheduler, 1, 2).1.is_empty());
    let (final_prompt, first_output) = step(&mut scheduler, 2, 2);
    assert_eq!(
        final_prompt.rows(),
        &[TpBatchRowV1 {
            request: id,
            token_id: 5,
            absolute_position: 4,
            kind: TpBatchRowKindV1::PrefillFinal
        }]
    );
    assert_eq!(first_output[0].output_index, 0);
    assert!(!first_output[0].finished);
    let (decode, second_output) = step(&mut scheduler, 3, 2);
    assert_eq!(
        decode.rows(),
        &[TpBatchRowV1 {
            request: id,
            token_id: first_output[0].token_id,
            absolute_position: 5,
            kind: TpBatchRowKindV1::Decode
        }]
    );
    assert_eq!(second_output[0].output_index, 1);
    assert!(step(&mut scheduler, 4, 2).1[0].finished);
    let request = scheduler.request(id).unwrap();
    assert_eq!(request.state(), TpRequestStateV1::Completed);
    assert_eq!(request.committed_position, 7);
    assert_eq!(request.output_timestamps_ns, [21, 31, 41]);
    assert_eq!(request.generated_tokens, [23, 24, 25]);
    assert!(scheduler.prepare(5, 50, 2).unwrap().is_none());
}

#[test]
fn full_prefix_hit_still_executes_final_prompt_for_first_output() {
    let mut scheduler = scheduler(16, 16);
    let id = scheduler
        .admit(
            TpRequestAdmissionV1 {
                cached_prefix_tokens: 4,
                ..admission(&[1, 2, 3, 4, 5], 1)
            },
            0,
        )
        .unwrap();
    assert_eq!(scheduler.request(id).unwrap().committed_position, 4);
    let (batch, events) = step(&mut scheduler, 0, 16);
    assert_eq!(batch.rows().len(), 1);
    assert_eq!(batch.rows()[0].absolute_position, 4);
    assert_eq!(batch.rows()[0].kind, TpBatchRowKindV1::PrefillFinal);
    assert_eq!(events.len(), 1);
    assert!(events[0].finished);
    assert_eq!(scheduler.request(id).unwrap().cached_prefix_tokens, 4);
}

#[test]
fn continuous_admission_finishes_and_reuses_one_slot_while_peer_decodes() {
    let mut scheduler = scheduler(16, 16);
    let long = scheduler.admit(admission(&[1], 5), 0).unwrap();
    step(&mut scheduler, 0, 16);
    let short = scheduler
        .admit(
            TpRequestAdmissionV1 {
                arrival_tick: 1,
                arrival_ns: 2,
                ..admission(&[2], 1)
            },
            2,
        )
        .unwrap();
    let (batch, events) = step(&mut scheduler, 1, 16);
    assert_eq!(
        batch.rows().iter().map(|row| row.kind).collect::<Vec<_>>(),
        [TpBatchRowKindV1::Decode, TpBatchRowKindV1::PrefillFinal]
    );
    assert!(!events[0].finished);
    assert!(events[1].finished);
    let retired = scheduler.retire(short).unwrap();
    assert_eq!(retired.output_timestamps_ns[0] - retired.arrival_ns, 9);
    let replacement = scheduler.admit(admission(&[3], 1), 12).unwrap();
    assert_eq!(replacement.slot, short.slot);
    assert_eq!(replacement.generation, short.generation + 1);
    assert_eq!(
        scheduler.request(short),
        Err(TpSchedulerErrorV1::StaleRequest)
    );
    assert_eq!(
        scheduler.cancel(short, 12),
        Err(TpSchedulerErrorV1::StaleRequest)
    );
    let (next, _) = step(&mut scheduler, 2, 16);
    assert!(
        next.rows()
            .iter()
            .any(|row| row.request == long && row.kind == TpBatchRowKindV1::Decode)
    );
    assert!(next.rows().iter().any(|row| row.request == replacement));
    assert_eq!(scheduler.retained_requests(), 2);
    assert_eq!(
        scheduler.request(long).unwrap().state(),
        TpRequestStateV1::Decode
    );
}

#[test]
fn slots_include_unretired_terminals_and_cancel_requires_explicit_retirement() {
    let mut scheduler = scheduler(16, 16);
    let ids = (0..32)
        .map(|_| scheduler.admit(admission(&[1], 1), 0).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        scheduler.admit(admission(&[1], 1), 0),
        Err(TpSchedulerErrorV1::AtCapacity)
    );
    step(&mut scheduler, 0, 16);
    assert_eq!(
        scheduler.admit(admission(&[1], 1), 1),
        Err(TpSchedulerErrorV1::AtCapacity)
    );
    assert_eq!(
        scheduler.retire(ids[31]),
        Err(TpSchedulerErrorV1::InvalidState)
    );
    scheduler.cancel(ids[31], 2).unwrap();
    assert_eq!(
        scheduler.request(ids[31]).unwrap().state(),
        TpRequestStateV1::Cancelled
    );
    assert_eq!(
        scheduler.cancel(ids[31], 2),
        Err(TpSchedulerErrorV1::InvalidState)
    );
    let record = scheduler.retire(ids[31]).unwrap();
    assert_eq!(record.cancelled_ns, Some(2));
    assert_eq!(scheduler.retained_requests(), 31);
    assert_eq!(
        scheduler.admit(admission(&[2], 1), 2).unwrap().slot,
        ids[31].slot
    );
}

#[test]
fn pending_batch_freezes_membership_and_abort_preserves_progress_and_fairness() {
    let mut scheduler = scheduler(2, 2);
    let id = scheduler.admit(admission(&[1, 2, 3], 1), 0).unwrap();
    let batch = scheduler.prepare(0, 0, 2).unwrap().unwrap();
    assert_eq!(scheduler.request(id).unwrap().committed_position, 0);
    assert_eq!(scheduler.cancel(id, 0), Err(TpSchedulerErrorV1::Busy));
    assert_eq!(scheduler.retire(id), Err(TpSchedulerErrorV1::Busy));
    assert_eq!(
        scheduler.admit(admission(&[4], 1), 0),
        Err(TpSchedulerErrorV1::Busy)
    );
    assert_eq!(scheduler.prepare(0, 0, 2), Err(TpSchedulerErrorV1::Busy));
    scheduler.abort(batch.id()).unwrap();
    let retry = scheduler.prepare(1, 1, 2).unwrap().unwrap();
    assert_eq!(retry.rows(), batch.rows());
    assert_eq!(retry.id(), batch.id() + 1);
    assert_eq!(scheduler.request(id).unwrap().committed_position, 0);
}

#[test]
fn hostile_completion_never_partially_publishes_an_earlier_request() {
    let mut scheduler = scheduler(4, 2);
    let first = scheduler.admit(admission(&[1, 2], 1), 0).unwrap();
    let second = scheduler.admit(admission(&[3, 4], 1), 0).unwrap();
    let batch = scheduler.prepare(0, 10, 4).unwrap().unwrap();
    let valid = [
        TpRowChoiceV1 {
            row_index: 1,
            token_id: 7,
        },
        TpRowChoiceV1 {
            row_index: 3,
            token_id: 8,
        },
    ];
    let before = snapshot(&scheduler);
    for invalid in [
        vec![],
        vec![valid[0]],
        vec![valid[0], valid[0]],
        vec![
            valid[0],
            TpRowChoiceV1 {
                row_index: 3,
                token_id: 1000,
            },
        ],
        vec![
            valid[0],
            TpRowChoiceV1 {
                row_index: 0,
                token_id: 9,
            },
        ],
        vec![
            valid[0],
            TpRowChoiceV1 {
                row_index: 4,
                token_id: 9,
            },
        ],
    ] {
        assert_eq!(
            scheduler.validate_completion(batch.id(), &invalid, 20),
            Err(TpSchedulerErrorV1::InvalidChoices)
        );
        assert_eq!(snapshot(&scheduler), before);
        assert_eq!(
            scheduler.complete(batch.id(), &invalid, 20),
            Err(TpSchedulerErrorV1::InvalidChoices)
        );
        assert_eq!(snapshot(&scheduler), before);
    }
    assert_eq!(
        scheduler.complete(batch.id(), &valid, 9),
        Err(TpSchedulerErrorV1::InvalidClock)
    );
    assert_eq!(snapshot(&scheduler), before);
    let events = scheduler
        .complete(batch.id(), &[valid[1], valid[0]], 20)
        .unwrap();
    assert_eq!(
        events.iter().map(|event| event.request).collect::<Vec<_>>(),
        [first, second]
    );
    assert_eq!(scheduler.request(first).unwrap().generated_tokens, [7]);
    assert_eq!(scheduler.request(second).unwrap().generated_tokens, [8]);
}

#[test]
fn completion_preflight_is_read_only_and_shares_commit_validation() {
    let mut scheduler = scheduler(2, 2);
    assert_eq!(scheduler.context_limit(), 8192);
    assert_eq!(scheduler.max_batch_rows(), 2);
    let id = scheduler.admit(admission(&[1, 2], 1), 0).unwrap();
    let batch = scheduler.prepare(0, 10, 2).unwrap().unwrap();
    let choices = choices(&batch);
    let before = snapshot(&scheduler);
    assert_eq!(
        scheduler.validate_completion(batch.id(), &choices, 9),
        Err(TpSchedulerErrorV1::InvalidClock)
    );
    assert_eq!(
        scheduler.validate_completion(batch.id() + 1, &choices, 20),
        Err(TpSchedulerErrorV1::StaleBatch)
    );
    scheduler
        .validate_completion(batch.id(), &choices, 20)
        .unwrap();
    assert_eq!(snapshot(&scheduler), before);
    assert_eq!(scheduler.request(id).unwrap().committed_position, 0);
    assert!(scheduler.request(id).unwrap().generated_tokens.is_empty());
    scheduler.complete(batch.id(), &choices, 20).unwrap();
    assert_eq!(
        scheduler.validate_completion(batch.id(), &choices, 20),
        Err(TpSchedulerErrorV1::StaleBatch)
    );
}

#[test]
fn stale_batch_generation_cannot_commit_abort_or_poison_current_work() {
    let mut scheduler = scheduler(1, 1);
    scheduler.admit(admission(&[1], 2), 0).unwrap();
    let batch = scheduler.prepare(0, 0, 1).unwrap().unwrap();
    let before = snapshot(&scheduler);
    assert_eq!(
        scheduler.complete(batch.id() + 1, &choices(&batch), 1),
        Err(TpSchedulerErrorV1::StaleBatch)
    );
    assert_eq!(
        scheduler.abort(batch.id() + 1),
        Err(TpSchedulerErrorV1::StaleBatch)
    );
    assert_eq!(
        scheduler.fail(batch.id() + 1),
        Err(TpSchedulerErrorV1::StaleBatch)
    );
    assert_eq!(snapshot(&scheduler), before);
    scheduler.complete(batch.id(), &choices(&batch), 1).unwrap();
    assert_eq!(
        scheduler.complete(batch.id(), &choices(&batch), 1),
        Err(TpSchedulerErrorV1::StaleBatch)
    );
}

#[test]
fn submitted_failure_quarantines_all_reuse_without_committed_progress() {
    let mut scheduler = scheduler(2, 2);
    let id = scheduler.admit(admission(&[1, 2], 1), 0).unwrap();
    let batch = scheduler.prepare(0, 0, 2).unwrap().unwrap();
    scheduler.fail(batch.id()).unwrap();
    assert!(scheduler.is_poisoned());
    assert_eq!(scheduler.request(id).unwrap().committed_position, 0);
    assert!(scheduler.request(id).unwrap().generated_tokens.is_empty());
    assert_eq!(
        scheduler.abort(batch.id()),
        Err(TpSchedulerErrorV1::Poisoned)
    );
    assert_eq!(
        scheduler.complete(batch.id(), &choices(&batch), 1),
        Err(TpSchedulerErrorV1::Poisoned)
    );
    assert_eq!(scheduler.cancel(id, 1), Err(TpSchedulerErrorV1::Poisoned));
    assert_eq!(scheduler.retire(id), Err(TpSchedulerErrorV1::Poisoned));
    assert_eq!(
        scheduler.admit(admission(&[3], 1), 1),
        Err(TpSchedulerErrorV1::Poisoned)
    );
    assert_eq!(
        scheduler.prepare(1, 1, 2),
        Err(TpSchedulerErrorV1::Poisoned)
    );
}

#[test]
fn thirty_two_requests_get_prefill_progress_under_sustained_decode() {
    let mut scheduler = scheduler(16, 16);
    let ids = (0..32)
        .map(|_| scheduler.admit(admission(&[1], 100), 0).unwrap())
        .collect::<Vec<_>>();
    for tick in 0..17 {
        let (batch, _) = step(&mut scheduler, tick, 16);
        assert!(batch.rows().len() <= 16);
        if tick > 0 {
            assert_eq!(
                batch.rows().last().unwrap().kind,
                TpBatchRowKindV1::PrefillFinal
            );
        }
    }
    assert!(
        ids.iter()
            .all(|&id| scheduler.request(id).unwrap().state() == TpRequestStateV1::Decode)
    );
    let mut observed = [false; 32];
    for tick in 17..19 {
        let (batch, _) = step(&mut scheduler, tick, 16);
        assert_eq!(batch.rows().len(), 16);
        for row in batch.rows() {
            observed[usize::from(row.request.slot)] = true;
        }
    }
    assert!(observed.into_iter().all(|value| value));
}

#[test]
fn one_row_budget_alternates_decode_and_prefill_without_abort_consuming_turn() {
    let mut scheduler = scheduler(1, 1);
    let decode = scheduler.admit(admission(&[1], 100), 0).unwrap();
    step(&mut scheduler, 0, 1);
    let prefill = scheduler.admit(admission(&[2, 3, 4, 5], 1), 1).unwrap();
    for tick in 1..7 {
        let batch = scheduler.prepare(tick, tick * 10, 1).unwrap().unwrap();
        let expected = if tick % 2 == 1 { decode } else { prefill };
        assert_eq!(batch.rows()[0].request, expected);
        scheduler.abort(batch.id()).unwrap();
        let retry = scheduler.prepare(tick, tick * 10, 1).unwrap().unwrap();
        assert_eq!(retry.rows(), batch.rows());
        scheduler
            .complete(retry.id(), &choices(&retry), tick * 10 + 1)
            .unwrap();
    }
    assert_eq!(scheduler.request(prefill).unwrap().committed_position, 3);
}

#[test]
fn prefill_chunks_rotate_without_request_interleaving_inside_a_chunk() {
    let mut scheduler = scheduler(16, 16);
    let ids = (0..32)
        .map(|_| scheduler.admit(admission(&[1; 32], 1), 0).unwrap())
        .collect::<Vec<_>>();
    for tick in 0..32 {
        let (batch, events) = step(&mut scheduler, tick, 16);
        assert!(events.is_empty());
        assert_eq!(batch.rows().len(), 16);
        assert!(
            batch
                .rows()
                .iter()
                .all(|row| row.request == ids[usize::try_from(tick).unwrap()])
        );
        assert_eq!(
            batch
                .rows()
                .iter()
                .map(|row| row.absolute_position)
                .collect::<Vec<_>>(),
            (0..16).collect::<Vec<_>>()
        );
    }
    assert!(
        ids.iter()
            .all(|&id| scheduler.request(id).unwrap().committed_position == 16)
    );
}

#[test]
fn arrival_tick_and_timestamp_are_preserved_for_measurement() {
    let mut scheduler = scheduler(16, 16);
    assert_eq!(
        scheduler.admit(
            TpRequestAdmissionV1 {
                arrival_ns: 11,
                ..admission(&[1], 1)
            },
            10
        ),
        Err(TpSchedulerErrorV1::InvalidClock)
    );
    let id = scheduler
        .admit(
            TpRequestAdmissionV1 {
                arrival_tick: 5,
                arrival_ns: 3,
                ..admission(&[1], 2)
            },
            10,
        )
        .unwrap();
    assert!(scheduler.prepare(4, 11, 16).unwrap().is_none());
    let batch = scheduler.prepare(5, 12, 16).unwrap().unwrap();
    let event = scheduler
        .complete(batch.id(), &choices(&batch), 20)
        .unwrap()[0];
    assert_eq!(
        event.completed_ns - scheduler.request(id).unwrap().arrival_ns,
        17
    );
    assert_eq!(batch.tick(), 5);
    assert_eq!(batch.prepared_ns(), 12);
    let batch = scheduler.prepare(6, 21, 16).unwrap().unwrap();
    scheduler
        .complete(batch.id(), &choices(&batch), 30)
        .unwrap();
    assert_eq!(
        scheduler.request(id).unwrap().output_timestamps_ns,
        [20, 30]
    );
}

#[test]
fn clocks_budgets_and_counter_exhaustion_fail_without_mutation() {
    let mut scheduler = scheduler(16, 16);
    assert!(scheduler.prepare(5, 10, 16).unwrap().is_none());
    let before = snapshot(&scheduler);
    assert_eq!(
        scheduler.prepare(4, 10, 16),
        Err(TpSchedulerErrorV1::InvalidClock)
    );
    assert_eq!(
        scheduler.prepare(5, 9, 16),
        Err(TpSchedulerErrorV1::InvalidClock)
    );
    assert_eq!(
        scheduler.prepare(5, 10, 0),
        Err(TpSchedulerErrorV1::InvalidBudget)
    );
    assert_eq!(
        scheduler.prepare(5, 10, 17),
        Err(TpSchedulerErrorV1::InvalidBudget)
    );
    assert_eq!(snapshot(&scheduler), before);
    let id = scheduler.admit(admission(&[1], 1), 10).unwrap();
    scheduler.last_batch_id = u64::MAX;
    let before = snapshot(&scheduler);
    assert_eq!(
        scheduler.prepare(5, 10, 16),
        Err(TpSchedulerErrorV1::BatchOverflow)
    );
    assert_eq!(snapshot(&scheduler), before);
    assert_eq!(
        scheduler.cancel(id, 9),
        Err(TpSchedulerErrorV1::InvalidClock)
    );
    assert_eq!(snapshot(&scheduler), before);
    let mut scheduler = EngineeringTpSchedulerV1::new(1000, 8192, 16, 16).unwrap();
    scheduler.slots[0].generation = u64::MAX;
    assert_eq!(scheduler.admit(admission(&[1], 1), 0).unwrap().slot, 1);
    for slot in &mut scheduler.slots {
        slot.request = None;
        slot.generation = u64::MAX;
    }
    let before = snapshot(&scheduler);
    assert_eq!(
        scheduler.admit(admission(&[1], 1), 0),
        Err(TpSchedulerErrorV1::GenerationOverflow)
    );
    assert_eq!(snapshot(&scheduler), before);
}

#[test]
fn context_boundary_executes_exact_final_position_and_never_overruns() {
    let mut scheduler = scheduler(16, 16);
    let id = scheduler
        .admit(
            TpRequestAdmissionV1 {
                cached_prefix_tokens: 8190,
                ..admission(&vec![1; 8191], 2)
            },
            0,
        )
        .unwrap();
    let first = scheduler.prepare(0, 0, 16).unwrap().unwrap();
    assert_eq!(first.rows()[0].absolute_position, 8190);
    scheduler
        .complete(
            first.id(),
            &[TpRowChoiceV1 {
                row_index: 0,
                token_id: 1,
            }],
            1,
        )
        .unwrap();
    let second = scheduler.prepare(1, 2, 16).unwrap().unwrap();
    assert_eq!(second.rows()[0].absolute_position, 8191);
    scheduler
        .complete(
            second.id(),
            &[TpRowChoiceV1 {
                row_index: 0,
                token_id: 2,
            }],
            3,
        )
        .unwrap();
    assert_eq!(scheduler.request(id).unwrap().committed_position, 8192);
    assert_eq!(
        scheduler.request(id).unwrap().state(),
        TpRequestStateV1::Completed
    );
    assert!(scheduler.prepare(2, 4, 16).unwrap().is_none());
}

#[test]
fn identical_arrival_and_completion_traces_are_deterministic() {
    let mut left = scheduler(16, 4);
    let mut right = scheduler(16, 4);
    for tick in 0..20 {
        if tick < 4 {
            let request = TpRequestAdmissionV1 {
                arrival_tick: tick,
                arrival_ns: tick * 10,
                ..admission(&[1, 2, 3, 4, 5], 10)
            };
            assert_eq!(
                left.admit(request.clone(), tick * 10),
                right.admit(request, tick * 10)
            );
        }
        let budget = usize::try_from(tick % 16 + 1).unwrap();
        let l = left.prepare(tick, tick * 10, budget).unwrap();
        let r = right.prepare(tick, tick * 10, budget).unwrap();
        assert_eq!(l, r);
        if let Some(batch) = l {
            assert_eq!(
                left.complete(batch.id(), &choices(&batch), tick * 10 + 1),
                right.complete(batch.id(), &choices(&batch), tick * 10 + 1)
            );
        }
        assert_eq!(snapshot(&left), snapshot(&right));
    }
}
