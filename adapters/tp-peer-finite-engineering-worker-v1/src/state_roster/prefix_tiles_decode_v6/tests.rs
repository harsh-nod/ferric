use super::*;

pub(super) fn prefix_done() -> [u32; 284] {
    let mut v = [64; 284];
    v[..4].copy_from_slice(&[1, 0, 0, 31]);
    v[4..14].copy_from_slice(&[1, 48, 1, 16, 64, 1, 48, 1, 16, 64]);
    v[14..24].fill(u32::MAX);
    v[18] = 3;
    v[23] = 3;
    v[24..154].fill(1);
    v
}
fn mlp_done() -> [u32; 548] {
    let mut v = [64; 548];
    v[..4].copy_from_slice(&[1, 0, 0, 31]);
    v[4..14].copy_from_slice(&[1, 96, 96, 1, 64, 1, 96, 96, 1, 64]);
    v[14..32].fill(u32::MAX);
    v[22] = 3;
    v[31] = 3;
    v[32..290].fill(64);
    v
}
struct Alloc {
    calls: usize,
    fail: Option<usize>,
    wrong: bool,
    events: Vec<(bool, usize)>,
}
impl Allocator for Alloc {
    type Prefix = usize;
    type Mlp = usize;
    fn preflight(&mut self, counts: &[usize]) -> Result<Vec<usize>> {
        self.step()?;
        Ok(if self.wrong {
            vec![0, 0]
        } else if counts == [144, 144] {
            vec![570, 566]
        } else {
            COUNTS.to_vec()
        })
    }
    fn prefix(&mut self, rank: usize) -> Result<usize> {
        self.step()?;
        self.events.push((true, rank));
        Ok(self.calls)
    }
    fn mlp(&mut self, rank: usize) -> Result<usize> {
        self.step()?;
        self.events.push((false, rank));
        Ok(self.calls)
    }
}
impl Alloc {
    fn new() -> Self {
        Self {
            calls: 0,
            fail: None,
            wrong: false,
            events: Vec::new(),
        }
    }
    fn step(&mut self) -> Result<()> {
        let call = self.calls;
        self.calls += 1;
        if self.fail == Some(call) {
            Err("injected".into())
        } else {
            Ok(())
        }
    }
}
#[test]
fn prefix_decode_roster_exact_two_banks_all36_and_distinct_catalog() {
    let mut a = Alloc::new();
    let (states, ids) = allocate(&mut a, [2; 32]).unwrap();
    assert_eq!((states.len(), ids.len(), a.calls), (72, 288, 290));
    for (slot, rows) in ids.chunks_exact(4).enumerate() {
        for (offset, id) in rows.iter().enumerate() {
            assert_eq!(id.reservation_id, (slot * 4 + offset + 1) as u64);
            assert_eq!(
                (id.forward, id.layer, id.rank),
                ((slot / 36) as u32, (slot % 36) as u32, (offset % 2) as u32)
            );
            assert_eq!(
                id.kind,
                if offset < 2 {
                    WorkerKind::PrefixTilesV6
                } else {
                    WorkerKind::MlpTilesV2
                }
            );
        }
    }
    for pair in a.events.chunks_exact(4) {
        assert_eq!(pair, [(true, 0), (true, 1), (false, 0), (false, 1)]);
    }
}
#[test]
fn prefix_decode_allocation_refuses_wrong_census_and_stops_on_every_error() {
    for fail in 0..290 {
        let mut a = Alloc::new();
        a.fail = Some(fail);
        assert!(allocate(&mut a, [2; 32]).is_err());
        assert_eq!(a.calls, fail + 1);
    }
    let mut a = Alloc::new();
    a.wrong = true;
    assert!(allocate(&mut a, [2; 32]).is_err());
    assert!(a.events.is_empty());
    let mut a = Alloc::new();
    assert!(allocate(&mut a, [0; 32]).is_err());
    assert_eq!(a.calls, 0);
}
#[test]
fn prefix_decode_initial_and_terminal_profiles_remain_exact() {
    assert_eq!(prefix_initial().iter().filter(|v| **v == 1).count(), 2);
    assert_eq!(mlp_initial().iter().filter(|v| **v == 1).count(), 2);
    assert!(prefix_terminal(&prefix_done()));
    assert!(mlp_terminal(&mlp_done()));
    for index in 0..284 {
        let mut v = prefix_done();
        v[index] = if index == 1 || index == 2 { 1 } else { 0 };
        assert!(!prefix_terminal(&v));
    }
    for index in 0..548 {
        let mut v = mlp_done();
        v[index] = if index == 1 || index == 2 { 1 } else { 0 };
        assert!(!mlp_terminal(&v));
    }
}

struct Memory {
    p: Vec<[[u32; 284]; 2]>,
    m: Vec<[[u32; 548]; 2]>,
    events: Vec<&'static str>,
    calls: usize,
    fail: Option<usize>,
    corrupt_readback: bool,
}
impl Memory {
    fn new() -> Self {
        Self {
            p: vec![[prefix_initial(); 2]; 72],
            m: vec![[mlp_initial(); 2]; 72],
            events: Vec::new(),
            calls: 0,
            fail: None,
            corrupt_readback: false,
        }
    }
    fn tick(&mut self, event: &'static str) -> Result<()> {
        self.events.push(event);
        let at = self.calls;
        self.calls += 1;
        if self.fail == Some(at) {
            Err("injected".into())
        } else {
            Ok(())
        }
    }
    fn done(&mut self, bank: usize) {
        for slot in bank * 36..(bank + 1) * 36 {
            self.p[slot] = [prefix_done(); 2];
            self.m[slot] = [mlp_done(); 2];
        }
    }
    fn clear(&mut self) {
        self.events.clear();
        self.calls = 0;
        self.fail = None;
    }
}
impl reuse::Backend for Memory {
    fn fence(&mut self) -> Result<()> {
        self.tick("fence")
    }
    fn prefix(&mut self, b: usize, l: usize, r: usize) -> Result<[u32; 284]> {
        self.tick("prefix")?;
        Ok(self.p[b * 36 + l][r])
    }
    fn mlp(&mut self, b: usize, l: usize, r: usize) -> Result<[u32; 548]> {
        self.tick("mlp")?;
        Ok(self.m[b * 36 + l][r])
    }
    fn reset_prefix(&mut self, b: usize, l: usize, r: usize, e: &[u32; 284]) -> Result<()> {
        self.tick("reset_prefix")?;
        assert_eq!(&self.p[b * 36 + l][r], e);
        self.p[b * 36 + l][r] = prefix_initial();
        if self.corrupt_readback {
            self.p[b * 36 + l][r][2] = 0;
        }
        Ok(())
    }
    fn reset_mlp(&mut self, b: usize, l: usize, r: usize, e: &[u32; 548]) -> Result<()> {
        self.tick("reset_mlp")?;
        assert_eq!(&self.m[b * 36 + l][r], e);
        self.m[b * 36 + l][r] = mlp_initial();
        Ok(())
    }
}
fn retired_two() -> (reuse::Ledger, Memory) {
    let mut ledger = reuse::Ledger::new([1; 32], [2; 32]).unwrap();
    let mut b = Memory::new();
    for g in 1..=2 {
        let bank = ledger
            .prepare(&mut b, [1; 32], [2; 32], g, (g - 1) as u32)
            .unwrap();
        b.done(bank);
        ledger.commit(g, Ok(())).unwrap();
    }
    b.clear();
    (ledger, b)
}
#[test]
fn prefix_decode_reuse_validates_all144_before_stores_and_reuses_both_banks() {
    let (mut ledger, mut b) = retired_two();
    for g in 3..=4 {
        b.clear();
        let bank = ledger
            .prepare(&mut b, [1; 32], [2; 32], g, (g - 1) as u32)
            .unwrap();
        assert_eq!(bank, ((g - 1) % 2) as usize);
        assert_eq!(b.events.len(), 434);
        assert_eq!(b.events[0], "fence");
        assert!(
            b.events[1..145]
                .iter()
                .all(|v| *v == "prefix" || *v == "mlp")
        );
        assert!(b.events[145..289].iter().all(|v| v.starts_with("reset_")));
        assert_eq!(b.events[433], "fence");
        b.done(bank);
        ledger.commit(g, Ok(())).unwrap();
    }
    assert_eq!(ledger.completed, 4);
    assert!(ledger.between());
    let before = b.calls;
    assert!(ledger.prepare(&mut b, [1; 32], [2; 32], 5, 4).is_err());
    assert_eq!(before, b.calls);
}
#[test]
fn prefix_decode_bad_last_state_never_resets_and_failure_permanently_poisons() {
    let (mut ledger, mut b) = retired_two();
    b.m[35][1][547] = 0;
    assert!(ledger.prepare(&mut b, [1; 32], [2; 32], 3, 2).is_err());
    assert!(!b.events.iter().any(|v| v.starts_with("reset_")));
    let before = b.calls;
    b.m[35][1] = mlp_done();
    assert!(ledger.prepare(&mut b, [1; 32], [2; 32], 3, 2).is_err());
    assert_eq!(b.calls, before);
    assert!(!ledger.between());
}
#[test]
fn prefix_decode_every_rearm_error_stops_without_committing_or_retrying() {
    for fail in 0..434 {
        let (mut ledger, mut b) = retired_two();
        b.fail = Some(fail);
        assert!(ledger.prepare(&mut b, [1; 32], [2; 32], 3, 2).is_err());
        assert_eq!(b.calls, fail + 1);
        assert_eq!(ledger.completed, 2);
        assert!(!ledger.between());
        let before = b.calls;
        assert!(ledger.prepare(&mut b, [1; 32], [2; 32], 3, 2).is_err());
        assert_eq!(b.calls, before);
        assert!(ledger.commit(3, Ok(())).is_err());
    }
    let (mut ledger, mut b) = retired_two();
    b.corrupt_readback = true;
    assert!(ledger.prepare(&mut b, [1; 32], [2; 32], 3, 2).is_err());
    assert_ne!(b.events.last(), Some(&"fence"));
}
#[test]
fn prefix_decode_scope_generation_active_and_commit_failures_never_advance() {
    for (reg, model, g, p) in [
        ([0; 32], [2; 32], 1, 0),
        ([1; 32], [0; 32], 1, 0),
        ([1; 32], [2; 32], 2, 0),
        ([1; 32], [2; 32], 1, 1),
    ] {
        let mut l = reuse::Ledger::new([1; 32], [2; 32]).unwrap();
        let mut b = Memory::new();
        assert!(l.prepare(&mut b, reg, model, g, p).is_err());
        assert!(b.events.is_empty());
    }
    let mut l = reuse::Ledger::new([1; 32], [2; 32]).unwrap();
    let mut b = Memory::new();
    l.prepare(&mut b, [1; 32], [2; 32], 1, 0).unwrap();
    assert!(l.commit(1, Err("tail fence".into())).is_err());
    assert_eq!(l.completed, 0);
    assert!(!l.between());
    let mut l = reuse::Ledger::new([1; 32], [2; 32]).unwrap();
    assert!(l.commit(1, Ok(())).is_err());
    assert_eq!(l.completed, 0);
}
#[test]
fn prefix_decode_layer_gate_keeps_all36_phases_and_bank_indices() {
    // Invalid phase must reject before touching any native state or exposing a borrow.
    for prefix in [false, true] {
        let mut roster = Roster {
            states: Vec::new(),
            gate: Gate::new(),
            ledger: reuse::Ledger::new([1; 32], [2; 32]).unwrap(),
        };
        if prefix {
            assert!(roster.take_prefix(0).is_err());
        } else {
            assert!(roster.take_mlp(0).is_err());
        }
        assert_eq!(roster.gate.phase, Phase::Terminal);
        assert!(!roster.between());
    }
    let mut gate = Gate::new();
    for bank in 0..2 {
        gate.begin(bank as u64 + 1, bank as u32).unwrap();
        for layer in 0..36 {
            let slot = gate
                .advance(layer, Phase::PrefixReady, Phase::PrefixInFlight)
                .unwrap();
            assert_eq!(slot, bank * 36 + layer);
            gate.phase = Phase::FirstResidualReady;
            gate.advance(
                layer,
                Phase::FirstResidualReady,
                Phase::FirstResidualInFlight,
            )
            .unwrap();
            gate.phase = Phase::MlpReady;
            gate.advance(layer, Phase::MlpReady, Phase::MlpInFlight)
                .unwrap();
            gate.phase = Phase::FinalResidualReady;
            gate.advance(
                layer,
                Phase::FinalResidualReady,
                Phase::FinalResidualInFlight,
            )
            .unwrap();
            gate.layer += 1;
            gate.phase = if layer == 35 {
                Phase::Tail
            } else {
                Phase::PrefixReady
            };
        }
        gate.commit().unwrap();
    }
    assert_eq!(gate.phase, Phase::Exhausted);
    assert!(gate.begin(3, 2).is_err());
    assert_eq!(gate.phase, Phase::Terminal);
    let mut gate = Gate::new();
    gate.begin(1, 0).unwrap();
    assert!(
        gate.advance(1, Phase::PrefixReady, Phase::PrefixInFlight)
            .is_err()
    );
    assert_eq!(gate.phase, Phase::Terminal);
}
