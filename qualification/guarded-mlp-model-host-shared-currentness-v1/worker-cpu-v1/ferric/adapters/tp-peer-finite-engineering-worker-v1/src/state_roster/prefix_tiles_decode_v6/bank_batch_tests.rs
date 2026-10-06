use super::super::tests::{mlp_done, prefix_done};
use super::*;
use std::cell::Cell;

fn numeric_states() -> Vec<Layer<(usize, u32), (usize, u32)>> {
    (0..BANKS * LAYERS)
        .map(|slot| Layer {
            prefix: [(0, slot as u32), (1, slot as u32)],
            mlp: [(0, slot as u32), (1, slot as u32)],
        })
        .collect()
}

fn values() -> Vec<BankValue> {
    let mut values = Vec::with_capacity(LAYERS * 4);
    for layer in 0..LAYERS {
        values.push(BankValue::Prefix([layer as u32 * 4; 284]));
        values.push(BankValue::Prefix([layer as u32 * 4 + 1; 284]));
        values.push(BankValue::Mlp([layer as u32 * 4 + 2; 548]));
        values.push(BankValue::Mlp([layer as u32 * 4 + 3; 548]));
    }
    values
}

#[test]
fn bank_gather_selects_exact36_layers_and_prefix_then_mlp_rank_order() {
    let states = numeric_states();
    for bank in 0..BANKS {
        let entries = gather_bank(
            &states,
            bank,
            |rank, state| Ok((true, rank, state.0, state.1)),
            |rank, state| Ok((false, rank, state.0, state.1)),
        )
        .unwrap();
        assert_eq!(entries.len(), 144);
        for (layer, entries) in entries.chunks_exact(4).enumerate() {
            let slot = (bank * LAYERS + layer) as u32;
            assert_eq!(
                entries,
                [
                    (true, 0, 0, slot),
                    (true, 1, 1, slot),
                    (false, 0, 0, slot),
                    (false, 1, 1, slot),
                ]
            );
        }
    }
}

#[test]
fn bank_gather_rejects_wrong_roster_or_bank_before_any_state_access() {
    for (count, bank) in [(0, 0), (71, 0), (73, 0), (72, 2), (72, usize::MAX)] {
        let mut states = numeric_states();
        states.resize_with(count, || Layer {
            prefix: [(0, 0), (1, 0)],
            mlp: [(0, 0), (1, 0)],
        });
        let calls = Cell::new(0);
        let result = gather_bank(
            &states,
            bank,
            |_, _| {
                calls.set(calls.get() + 1);
                Ok(())
            },
            |_, _| {
                calls.set(calls.get() + 1);
                Ok(())
            },
        );
        assert!(result.is_err());
        assert_eq!(calls.get(), 0);
    }
}

#[test]
fn bank_gather_stops_on_each_failed_entry_without_returning_partial_requests() {
    let states = numeric_states();
    for fail in 0..144 {
        let calls = Cell::new(0);
        let entry = |_: usize, _: &(usize, u32)| {
            let at = calls.get();
            calls.set(at + 1);
            if at == fail {
                Err("injected gather".into())
            } else {
                Ok(())
            }
        };
        assert!(gather_bank(&states, 1, entry, entry).is_err());
        assert_eq!(calls.get(), fail + 1);
    }
}

#[test]
fn bank_gather_owner_refusal_keeps_last_rank_from_entering_runtime_batch() {
    for prefix in [false, true] {
        let mut states = numeric_states();
        if prefix {
            states[71].prefix[1].0 = 0;
        } else {
            states[71].mlp[1].0 = 0;
        }
        let checked = |rank, state: &(usize, u32)| {
            if state.0 != rank {
                Err("owner/rank mismatch".into())
            } else {
                Ok(state.1)
            }
        };
        assert!(gather_bank(&states, 1, checked, checked).is_err());
    }
}

#[test]
fn bank_decode_retains_every_word_in_both_typed_rank_arrays() {
    let snapshots = decode_bank(values()).unwrap();
    assert_eq!(snapshots.len(), 36);
    for (layer, snapshot) in snapshots.iter().enumerate() {
        assert_eq!(snapshot.prefix[0], [layer as u32 * 4; 284]);
        assert_eq!(snapshot.prefix[1], [layer as u32 * 4 + 1; 284]);
        assert_eq!(snapshot.mlp[0], [layer as u32 * 4 + 2; 548]);
        assert_eq!(snapshot.mlp[1], [layer as u32 * 4 + 3; 548]);
    }
}

#[test]
fn bank_decode_refuses_missing_extra_and_every_wrong_typed_slot() {
    let mut short = values();
    short.pop();
    assert!(decode_bank(short).is_err());
    let mut extra = values();
    extra.push(BankValue::Prefix([0; 284]));
    assert!(decode_bank(extra).is_err());
    for index in 0..144 {
        let mut changed = values();
        changed[index] = if index % 4 < 2 {
            BankValue::Mlp([0; 548])
        } else {
            BankValue::Prefix([0; 284])
        };
        assert!(decode_bank(changed).is_err(), "typed slot {index}");
    }
}

struct BankMemory {
    events: Vec<&'static str>,
    scan: usize,
    counts: [usize; 2],
    fail_scan: Option<usize>,
    corrupt_scan: Option<usize>,
    corrupt_kind: bool,
    prior_terminal: bool,
}

impl BankMemory {
    fn new() -> Self {
        Self {
            events: Vec::new(),
            scan: 0,
            counts: [LAYERS; 2],
            fail_scan: None,
            corrupt_scan: None,
            corrupt_kind: false,
            prior_terminal: false,
        }
    }
}

impl Backend for BankMemory {
    fn fence(&mut self) -> Result<()> {
        self.events.push("outer_fence");
        Ok(())
    }

    fn snapshot_bank(&mut self, _bank: usize) -> Result<Vec<Snapshot>> {
        self.events.push("snapshot_bank");
        let scan = self.scan;
        self.scan += 1;
        if self.fail_scan == Some(scan) {
            return Err("injected whole-bank failure".into());
        }
        let terminal = self.prior_terminal && scan == 0;
        let p = if terminal {
            prefix_done()
        } else {
            prefix_initial()
        };
        let m = if terminal { mlp_done() } else { mlp_initial() };
        let mut result: Vec<_> = (0..self.counts[scan])
            .map(|_| Snapshot {
                prefix: [p; 2],
                mlp: [m; 2],
            })
            .collect();
        if self.corrupt_scan == Some(scan) {
            let last = result.last_mut().unwrap();
            if self.corrupt_kind {
                last.prefix[1][283] ^= 1;
            } else {
                last.mlp[1][547] ^= 1;
            }
        }
        Ok(result)
    }

    fn reset_prefix(&mut self, _: usize, _: usize, _: usize, expected: &[u32; 284]) -> Result<()> {
        assert!(prefix_terminal(expected));
        self.events.push("reset_prefix");
        Ok(())
    }

    fn reset_mlp(&mut self, _: usize, _: usize, _: usize, expected: &[u32; 548]) -> Result<()> {
        assert!(mlp_terminal(expected));
        self.events.push("reset_mlp");
        Ok(())
    }
}

fn retired() -> Ledger {
    Ledger {
        registration: [1; 32],
        model: [2; 32],
        completed: 2,
        banks: [1, 2],
        phase: Phase::Between,
    }
}

#[test]
fn bank_ledger_uses_two_snapshots_and_preserves_outer_fences_and144_rearms() {
    let mut ledger = retired();
    let mut backend = BankMemory::new();
    backend.prior_terminal = true;
    assert_eq!(
        ledger
            .prepare(&mut backend, [1; 32], [2; 32], 3, 2)
            .unwrap(),
        0
    );
    assert_eq!(backend.scan, 2);
    assert_eq!(backend.events.len(), 148);
    assert_eq!(&backend.events[..2], ["outer_fence", "snapshot_bank"]);
    for pair in backend.events[2..146].chunks_exact(2) {
        assert_eq!(pair, ["reset_prefix", "reset_mlp"]);
    }
    assert_eq!(&backend.events[146..], ["snapshot_bank", "outer_fence"]);
    assert_eq!(ledger.banks, [3, 2]);
    assert_eq!(ledger.phase, Phase::Active(3));
    assert_eq!(ledger.completed, 2);
}

#[test]
fn bank_ledger_fresh_generation_still_performs_both_scans_without_stores() {
    let mut ledger = Ledger::new([1; 32], [2; 32]).unwrap();
    let mut backend = BankMemory::new();
    assert_eq!(
        ledger
            .prepare(&mut backend, [1; 32], [2; 32], 1, 0)
            .unwrap(),
        0
    );
    assert_eq!(
        backend.events,
        [
            "outer_fence",
            "snapshot_bank",
            "snapshot_bank",
            "outer_fence"
        ]
    );
}

#[test]
fn bank_ledger_each_failed_snapshot_is_terminal_without_publishing_generation() {
    for fail in 0..2 {
        let mut ledger = retired();
        let mut backend = BankMemory::new();
        backend.prior_terminal = true;
        backend.fail_scan = Some(fail);
        assert!(
            ledger
                .prepare(&mut backend, [1; 32], [2; 32], 3, 2)
                .is_err()
        );
        assert_eq!(ledger.banks, [1, 2]);
        assert_eq!(ledger.completed, 2);
        assert_eq!(ledger.phase, Phase::Terminal);
        assert_eq!(backend.events.last(), Some(&"snapshot_bank"));
        if fail == 0 {
            assert!(
                !backend
                    .events
                    .iter()
                    .any(|event| event.starts_with("reset_"))
            );
        }
        let count = backend.events.len();
        assert!(
            ledger
                .prepare(&mut backend, [1; 32], [2; 32], 3, 2)
                .is_err()
        );
        assert_eq!(backend.events.len(), count);
    }
}

#[test]
fn bank_ledger_rejects_missing_or_extra_layers_in_either_scan() {
    for scan in 0..2 {
        for count in [0, 35, 37] {
            let mut ledger = retired();
            let mut backend = BankMemory::new();
            backend.prior_terminal = true;
            backend.counts[scan] = count;
            assert!(
                ledger
                    .prepare(&mut backend, [1; 32], [2; 32], 3, 2)
                    .is_err()
            );
            assert_eq!(ledger.banks, [1, 2]);
            assert_eq!(ledger.phase, Phase::Terminal);
            if scan == 0 {
                assert!(
                    !backend
                        .events
                        .iter()
                        .any(|event| event.starts_with("reset_"))
                );
            }
        }
    }
}

#[test]
fn bank_ledger_last_state_corruption_never_escapes_either_complete_scan() {
    for scan in 0..2 {
        for prefix in [false, true] {
            let mut ledger = retired();
            let mut backend = BankMemory::new();
            backend.prior_terminal = true;
            backend.corrupt_scan = Some(scan);
            backend.corrupt_kind = prefix;
            assert!(
                ledger
                    .prepare(&mut backend, [1; 32], [2; 32], 3, 2)
                    .is_err()
            );
            assert_eq!(ledger.banks, [1, 2]);
            assert_eq!(ledger.phase, Phase::Terminal);
            assert_eq!(backend.events.last(), Some(&"snapshot_bank"));
            if scan == 0 {
                assert!(
                    !backend
                        .events
                        .iter()
                        .any(|event| event.starts_with("reset_"))
                );
            }
        }
    }
}
