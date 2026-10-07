use super::*;
#[test]
fn guarded_reuse_retains_all_72_bound_handles_and_exact_arena_plateau() {
    struct Handle { identity: usize, arena: Option<usize>, uses: u64 }
    let inputs = (0..SLOTS).collect::<Vec<_>>();
    let mut bindings = Vec::new();
    let mut bound = consume_slots(slots(), &inputs, |identity, input| {
        assert_eq!(identity, *input);
        bindings.push(identity);
        Ok(Handle { identity, arena: None, uses: 0 })
    }).unwrap();
    assert_eq!(bindings, inputs);
    let mut l = Ledger::with_mode([1; 32], [2; 32], ArenaMode::ReuseRetired).unwrap();
    let mut allocated = 0;
    let mut counts = BOUND_COUNTS;
    let mut actions = Vec::new();
    for forward in 1..=4 {
        l.begin([1; 32], [2; 32], forward, (forward - 1) as u32, |bank, rearm| {
            actions.push((bank, rearm));
            Ok((forward - 1) / 2 + 1)
        }).unwrap();
        for layer in 0..LAYERS {
            let index = l.expect(layer, Phase::Prefix).unwrap();
            assert_eq!(bound[index].prefixes, [2 * index, 2 * index + 1]);
            l.phase = Phase::PrefixInFlight;
            l.expect(layer, Phase::PrefixInFlight).unwrap();
            l.phase = Phase::Guarded;
            dispatch_slot(&mut l, &mut bound, layer, |pair, additional, before, generation| {
                assert_eq!(before, counts);
                assert_eq!(pair.identity, index);
                assert_eq!(pair.uses + 1, generation);
                let previous = pair.arena;
                if pair.arena.is_none() {
                    pair.arena = Some(allocated);
                    allocated += 1;
                    counts = counts.map(|v| v + 1);
                }
                assert_eq!(additional, usize::from(previous.is_none()));
                assert_eq!(pair.arena, Some(index));
                pair.uses += 1;
                Ok(())
            }).unwrap();
            assert_eq!(l.counts(), counts);
        }
        l.commit(forward, Ok(())).unwrap();
        assert_eq!(counts, if forward == 1 { [751, 747] } else { REUSE_MAX_COUNTS });
    }
    assert_eq!(actions, [(0, false), (1, false), (0, true), (1, true)]);
    assert_eq!(allocated, 72);
    assert_eq!(bindings.len(), 72);
    assert!(bound.iter().all(|slot| slot.pair.uses == 2));
    assert_eq!(l.dispatches, 144);
    assert_eq!(l.phase, Phase::Exhausted);
    assert!(l.begin([1; 32], [2; 32], 5, 4, |_, _| panic!("bounded AR4")).is_err());
}

#[test]
fn guarded_reuse_rejects_stale_generation_missing_slot_and_failed_dispatch_without_retry() {
    for case in 0..6 {
        let mut l = Ledger::with_mode([1; 32], [2; 32], ArenaMode::ReuseRetired).unwrap();
        l.begin([1; 32], [2; 32], 1, 0, |_, _| Ok(1)).unwrap();
        l.phase = Phase::Guarded;
        let mut bound = slots();
        match case {
            0 => l.last_generation[0] = 1,
            1 => l.generation = 2,
            2 => { bound.pop(); },
            3 => l.layer = 1,
            _ => (),
        }
        let mut calls = 0;
        let result: Result<()> = dispatch_slot(&mut l, &mut bound, 0, |_, _, _, _| {
            calls += 1;
            Err(if case == 4 { "pre-dispatch census drift" } else { "runtime pending peer or partial dispatch" }.into())
        });
        assert!(result.is_err());
        assert_eq!(calls, usize::from(case >= 4));
        assert_eq!(l.phase, Phase::Terminal);
        assert_eq!(l.dispatches, 0);
        assert_eq!(l.counts(), BOUND_COUNTS);
        assert!(dispatch_slot(&mut l, &mut bound, 0, |_, _, _, _| -> Result<()> {
            panic!("failed pair must not retry or fall back to fresh")
        }).is_err());
    }
}

#[test]
fn guarded_reuse_bank_refusal_precedes_every_slot_dispatch_and_keeps_fresh_default() {
    for actual in [Ok(2), Err("mixed-bank retirement refused".into())] {
        let mut l = Ledger::with_mode([1; 32], [2; 32], ArenaMode::ReuseRetired).unwrap();
        let mut actual = Some(actual);
        assert!(l.begin([1; 32], [2; 32], 1, 0, |_, _| actual.take().unwrap()).is_err());
        assert!(dispatch_slot(&mut l, &mut slots(), 0, |_, _, _, _| -> Result<()> {
            panic!("bank admission must precede arena reset/dispatch")
        }).is_err());
        assert_eq!(l.arena_allocations, 0);
    }
    let mut fresh = Ledger::new([1; 32], [2; 32]).unwrap();
    assert_eq!(fresh.mode, ArenaMode::Fresh);
    for forward in 1..=4 {
        fresh.begin([1; 32], [2; 32], forward, (forward - 1) as u32,
            |_, _| Ok((forward - 1) / 2 + 1)).unwrap();
        complete_layers(&mut fresh);
        fresh.commit(forward, Ok(())).unwrap();
    }
    assert_eq!(fresh.counts(), MAX_COUNTS);
}

struct Fake {
    counts: [usize; 2],
    calls: usize,
    fail: Option<usize>,
}
impl Fake {
    fn new() -> Self {
        Self {
            counts: [570, 566],
            calls: 0,
            fail: None,
        }
    }
    fn step(&mut self) -> Result<usize> {
        let n = self.calls;
        self.calls += 1;
        if self.fail == Some(n) {
            Err("injected allocation".into())
        } else {
            Ok(n)
        }
    }
}
impl Allocator for Fake {
    type Prefix = (usize, usize);
    type Storage = usize;
    fn counts(&mut self, _: &[usize]) -> Result<Vec<usize>> {
        Ok(self.counts.to_vec())
    }
    fn prefix(&mut self, rank: usize) -> Result<Self::Prefix> {
        let n = self.step()?;
        self.counts[rank] += 1;
        Ok((rank, n))
    }
    fn storage(&mut self) -> Result<usize> {
        let n = self.step()?;
        self.counts = self.counts.map(|v| v + 1);
        Ok(n)
    }
}
#[test]
fn guarded_roster_allocates_72_opaque_pairs_and_144_prefixes() {
    let mut f = Fake::new();
    let slots = allocate(&mut f, [1; 32]).unwrap();
    assert_eq!(slots.len(), 72);
    assert_eq!(f.calls, 216);
    assert_eq!(f.counts, STATE_COUNTS);
    for (i, s) in slots.iter().enumerate() {
        assert_eq!(s.prefixes, [(0, 3 * i), (1, 3 * i + 1)]);
        assert_eq!(s.pair, 3 * i + 2);
    }
    let ids = identities([1; 32]);
    assert_eq!(ids.len(), 288);
    for (i, id) in ids.iter().enumerate() {
        assert_eq!(id.reservation_id, i as u64 + 1);
        assert_eq!(id.forward, (i / 144) as u32);
        assert_eq!(id.layer, ((i / 4) % 36) as u32);
        assert_eq!(id.rank, (i % 2) as u32);
        assert_eq!(
            id.kind,
            if i % 4 < 2 {
                WorkerKind::PrefixTilesV6
            } else {
                WorkerKind::GuardedMlpCombinedV1
            }
        );
    }
}
#[test]
fn guarded_roster_refuses_scope_census_and_each_partial_allocation() {
    let mut f = Fake::new();
    assert!(allocate(&mut f, [0; 32]).is_err());
    assert_eq!(f.calls, 0);
    let mut f = Fake::new();
    f.counts[0] += 1;
    assert!(allocate(&mut f, [1; 32]).is_err());
    assert_eq!(f.calls, 0);
    for fail in 0..216 {
        let mut f = Fake::new();
        f.fail = Some(fail);
        assert!(allocate(&mut f, [1; 32]).is_err());
        assert_eq!(f.calls, fail + 1);
    }
}
fn slots() -> Vec<Slot<usize, usize>> {
    (0..72)
        .map(|i| Slot {
            prefixes: [2 * i, 2 * i + 1],
            pair: i,
        })
        .collect()
}
#[test]
fn guarded_roster_consumes_all_bindings_and_never_publishes_a_partial_roster() {
    let inputs = (0..72).collect::<Vec<_>>();
    let mut calls = Vec::new();
    let bound = consume_slots(slots(), &inputs, |pair, input| {
        calls.push(pair);
        assert_eq!(pair, *input);
        Ok(pair + 100)
    })
    .unwrap();
    assert_eq!(calls, inputs);
    assert_eq!(bound.len(), 72);
    for (i, s) in bound.iter().enumerate() {
        assert_eq!(s.prefixes, [2 * i, 2 * i + 1]);
        assert_eq!(s.pair, i + 100);
    }
    for n in [0, 35, 71, 73] {
        let wrong = (0..n).collect::<Vec<_>>();
        let mut calls = 0;
        assert!(
            consume_slots(slots(), &wrong, |_, _| {
                calls += 1;
                Ok(())
            })
            .is_err()
        );
        assert_eq!(calls, 0);
    }
    for fail in 0..72 {
        let mut calls = 0;
        assert!(
            consume_slots(slots(), &inputs, |pair, _| {
                calls += 1;
                if pair == fail {
                    Err("binding".into())
                } else {
                    Ok(pair)
                }
            })
            .is_err()
        );
        assert_eq!(calls, fail + 1);
    }
}
fn complete_layers(l: &mut Ledger) {
    for i in 0..36 {
        assert_eq!(l.expect(i, Phase::Prefix).unwrap(), l.bank * 36 + i);
        l.phase = Phase::PrefixInFlight;
        l.expect(i, Phase::PrefixInFlight).unwrap();
        l.phase = Phase::Guarded;
        l.expect(i, Phase::Guarded).unwrap();
        l.phase = Phase::GuardedInFlight;
        l.guarded_complete(i).unwrap();
    }
}
#[test]
fn guarded_roster_distinguishes_global_forwards_from_local_bank_generations() {
    let mut l = Ledger::new([1; 32], [2; 32]).unwrap();
    let mut actions = Vec::new();
    for forward in 1..=4 {
        l.begin(
            [1; 32],
            [2; 32],
            forward,
            (forward - 1) as u32,
            |bank, rearm| {
                actions.push((bank, rearm));
                Ok((forward - 1) / 2 + 1)
            },
        )
        .unwrap();
        assert_eq!(l.generation, (forward - 1) / 2 + 1);
        complete_layers(&mut l);
        l.commit(forward, Ok(())).unwrap();
        assert_eq!(l.counts(), BOUND_COUNTS.map(|n| n + forward as usize * 36));
    }
    assert_eq!(actions, vec![(0, false), (1, false), (0, true), (1, true)]);
    assert_eq!(l.counts(), [859, 855]);
    assert_eq!(l.phase, Phase::Exhausted);
    assert!(
        l.begin([1; 32], [2; 32], 5, 4, |_, _| panic!(
            "exhausted bank action"
        ))
        .is_err()
    );
}
#[test]
fn guarded_roster_invalid_order_or_mixed_bank_failure_is_terminal() {
    for case in 0..6 {
        let mut l = Ledger::new([1; 32], [2; 32]).unwrap();
        let mut calls = 0;
        let result = l.begin(
            if case == 0 { [3; 32] } else { [1; 32] },
            if case == 1 { [3; 32] } else { [2; 32] },
            if case == 2 { 2 } else { 1 },
            if case == 3 { 1 } else { 0 },
            |_, _| {
                calls += 1;
                if case == 4 {
                    Err("bank refused before reset".into())
                } else {
                    Ok(2)
                }
            },
        );
        assert!(result.is_err());
        assert_eq!(l.phase, Phase::Terminal);
        assert_eq!(l.dispatches, 0);
        assert_eq!(calls, usize::from(case >= 4));
        assert!(
            l.begin([1; 32], [2; 32], 1, 0, |_, _| panic!("terminal reuse"))
                .is_err()
        );
    }
    let mut l = Ledger::new([1; 32], [2; 32]).unwrap();
    l.begin([1; 32], [2; 32], 1, 0, |_, _| Ok(1)).unwrap();
    assert!(l.guarded_complete(0).is_err());
    assert_eq!(l.phase, Phase::Terminal);
    let mut l = Ledger::new([1; 32], [2; 32]).unwrap();
    l.begin([1; 32], [2; 32], 1, 0, |_, _| Ok(1)).unwrap();
    complete_layers(&mut l);
    assert!(l.commit(1, Err("fence failed".into())).is_err());
    assert_eq!(l.completed, 0);
}
