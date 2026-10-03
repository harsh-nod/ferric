use super::*;
fn prefix_done() -> [u32; 22] {
    let mut p = [64; 22];
    p[..6].copy_from_slice(&[1, 0, 65535, 65535, 0x55555555, 0]);
    p
}
fn tiles_done() -> [u32; WORDS] {
    let mut m = [0; WORDS];
    m[..4].copy_from_slice(&[1, 0, 0, 31]);
    m[4..9].copy_from_slice(&[1, 96, 96, 1, 64]);
    m[9..14].copy_from_slice(&[1, 96, 96, 1, 64]);
    m[14..23].fill(u32::MAX);
    m[22] = 3;
    m[23..32].fill(u32::MAX);
    m[31] = 3;
    m[32..290].fill(64);
    m[290..].fill(64);
    m
}
struct Alloc {
    events: Vec<(bool, usize)>,
    counts: Vec<usize>,
    fail: Option<usize>,
    preflight: usize,
}
impl Allocator for Alloc {
    type Prefix = (usize, usize);
    type Tiles = (usize, usize);
    fn preflight(&mut self, c: &[usize]) -> Result<Vec<usize>> {
        assert_eq!(c, [144, 144]);
        self.preflight += 1;
        Ok(self.counts.clone())
    }
    fn prefix(&mut self, r: usize) -> Result<Self::Prefix> {
        self.add(true, r)
    }
    fn tiles(&mut self, r: usize) -> Result<Self::Tiles> {
        self.add(false, r)
    }
}
impl Alloc {
    fn new() -> Self {
        Self {
            events: vec![],
            counts: vec![570, 566],
            fail: None,
            preflight: 0,
        }
    }
    fn add(&mut self, p: bool, r: usize) -> Result<(usize, usize)> {
        let id = self.events.len();
        self.events.push((p, r));
        if self.fail == Some(id) {
            Err("allocation".into())
        } else {
            Ok((id, r))
        }
    }
}
#[test]
fn replacement_roster_has_two_banks_and_only_distinct_tiles_roles() {
    let mut a = Alloc::new();
    let (states, ids) = allocate(&mut a, [3; 32]).unwrap();
    assert_eq!(a.preflight, 1);
    assert_eq!(states.len(), 72);
    assert_eq!(ids.len(), 288);
    assert_eq!(a.events.len(), 288);
    for (slot, s) in states.iter().enumerate() {
        for rank in 0..2 {
            assert_eq!(s.prefix[rank], (slot * 4 + rank, rank));
            assert_eq!(s.mlp[rank], (slot * 4 + 2 + rank, rank));
        }
    }
    for (n, row) in ids.iter().enumerate() {
        assert_eq!(row.reservation_id, n as u64 + 1);
        assert_eq!(row.model, [3; 32]);
        assert_eq!(row.forward, (n / 144) as u32);
        assert_eq!(row.layer, ((n % 144) / 4) as u32);
        assert_eq!(row.rank, (n % 2) as u32);
        assert_eq!(
            row.kind,
            if n % 4 < 2 {
                WorkerKind::PrefixV5
            } else {
                WorkerKind::MlpTilesV2
            }
        );
    }
}
#[test]
fn reserve_failure_stops_at_actual_allocation_and_never_falls_back() {
    for index in [0, 1, 2, 3, 142, 143, 286, 287] {
        let mut a = Alloc::new();
        a.fail = Some(index);
        assert!(allocate(&mut a, [3; 32]).is_err());
        assert_eq!(a.events.len(), index + 1);
    }
    let mut a = Alloc::new();
    a.counts = vec![0];
    assert!(allocate(&mut a, [3; 32]).is_err());
    assert!(a.events.is_empty());
    assert!(allocate(&mut Alloc::new(), [0; 32]).is_err());
}
struct Bank {
    p: Vec<[[u32; 22]; 2]>,
    m: Vec<[[u32; WORDS]; 2]>,
    events: Vec<String>,
    fail: Option<usize>,
    resets: usize,
}
impl Bank {
    fn new() -> Self {
        Self {
            p: vec![[core::array::from_fn(|i| u32::from(i < 2)); 2]; 72],
            m: vec![[initial(); 2]; 72],
            events: vec![],
            fail: None,
            resets: 0,
        }
    }
    fn done(&mut self, b: usize) {
        for l in 0..36 {
            self.p[b * 36 + l] = [prefix_done(); 2];
            self.m[b * 36 + l] = [tiles_done(); 2];
        }
    }
    fn step(&mut self, event: String) -> Result<()> {
        let n = self.events.len();
        self.events.push(event);
        if self.fail == Some(n) {
            Err("bank injected".into())
        } else {
            Ok(())
        }
    }
}
impl reuse::Backend for Bank {
    fn fence(&mut self) -> Result<()> {
        self.step("fence".into())
    }
    fn prefix(&mut self, b: usize, l: usize, r: usize) -> Result<[u32; 22]> {
        self.step(format!("p{b}/{l}/{r}"))?;
        Ok(self.p[b * 36 + l][r])
    }
    fn tiles(&mut self, b: usize, l: usize, r: usize) -> Result<[u32; WORDS]> {
        self.step(format!("m{b}/{l}/{r}"))?;
        Ok(self.m[b * 36 + l][r])
    }
    fn reset_prefix(&mut self, b: usize, l: usize, r: usize, x: &[u32; 22]) -> Result<()> {
        self.step(format!("rp{b}/{l}/{r}"))?;
        assert_eq!(*x, self.p[b * 36 + l][r]);
        self.p[b * 36 + l][r] = core::array::from_fn(|i| u32::from(i < 2));
        self.resets += 1;
        Ok(())
    }
    fn reset_tiles(&mut self, b: usize, l: usize, r: usize, x: &[u32; WORDS]) -> Result<()> {
        self.step(format!("rm{b}/{l}/{r}"))?;
        assert_eq!(*x, self.m[b * 36 + l][r]);
        self.m[b * 36 + l][r] = initial();
        self.resets += 1;
        Ok(())
    }
}
fn ledger() -> reuse::Ledger {
    reuse::Ledger::new([1; 32], [2; 32]).unwrap()
}
fn gate_forward(gate: &mut Gate, bank: usize) {
    *gate = Gate {
        forward: bank,
        layer: 0,
        phase: Phase::BetweenForwards,
    };
    gate.begin(bank as u64 + 1, bank as u32).unwrap();
    for layer in 0..36 {
        for (a, b) in [
            (Phase::PrefixReady, Phase::PrefixInFlight),
            (Phase::FirstResidualReady, Phase::FirstResidualInFlight),
            (Phase::MlpReady, Phase::MlpInFlight),
            (Phase::FinalResidualReady, Phase::FinalResidualInFlight),
        ] {
            gate.advance(layer, a, b).unwrap();
            gate.phase = match b {
                Phase::PrefixInFlight => Phase::FirstResidualReady,
                Phase::FirstResidualInFlight => Phase::MlpReady,
                Phase::MlpInFlight => Phase::FinalResidualReady,
                _ => Phase::PrefixReady,
            };
        }
        gate.layer += 1;
    }
    gate.phase = Phase::Tail;
    gate.commit().unwrap();
}
#[test]
fn four_generations_use_banks_zero_one_zero_one_and_exact_full_bank_resets() {
    let mut l = ledger();
    let mut b = Bank::new();
    let mut gate = Gate::new();
    for generation in 1..=4 {
        b.events.clear();
        let bank = l
            .prepare(&mut b, [1; 32], [2; 32], generation, generation as u32 - 1)
            .unwrap();
        assert_eq!(bank, ((generation - 1) % 2) as usize);
        if generation <= 2 {
            assert!(!b.events.iter().any(|s| s.starts_with('r')));
        } else {
            assert_eq!(b.events.iter().position(|s| s.starts_with('r')), Some(145));
        }
        gate_forward(&mut gate, bank);
        b.done(bank);
        l.commit(generation, Ok(())).unwrap();
        assert_eq!(l.completed, generation);
    }
    assert_eq!(b.resets, 288);
    assert!(l.between());
    assert!(l.prepare(&mut b, [1; 32], [2; 32], 5, 4).is_err());
}
fn after_two() -> (reuse::Ledger, Bank) {
    let mut l = ledger();
    let mut b = Bank::new();
    for g in 1..=2 {
        let bank = l
            .prepare(&mut b, [1; 32], [2; 32], g, g as u32 - 1)
            .unwrap();
        b.done(bank);
        l.commit(g, Ok(())).unwrap();
    }
    b.events.clear();
    (l, b)
}
#[test]
fn last_slot_corruption_prevents_first_store_and_wrong_v1_initial_is_rejected() {
    let (mut l, mut b) = after_two();
    b.m[35][1][547] = 63;
    assert!(l.prepare(&mut b, [1; 32], [2; 32], 3, 2).is_err());
    assert_eq!(b.resets, 0);
    let mut l = ledger();
    let mut b = Bank::new();
    b.m[0][0][1] = 1;
    b.m[0][0][2] = 0;
    assert!(l.prepare(&mut b, [1; 32], [2; 32], 1, 0).is_err());
    assert_eq!(b.resets, 0);
}
#[test]
fn every_rearm_snapshot_store_readback_and_fence_error_is_permanently_terminal() {
    let (mut l, mut good) = after_two();
    l.prepare(&mut good, [1; 32], [2; 32], 3, 2).unwrap();
    for fail in 0..good.events.len() {
        let (mut l, mut b) = after_two();
        b.fail = Some(fail);
        assert!(l.prepare(&mut b, [1; 32], [2; 32], 3, 2).is_err());
        assert_eq!(l.completed, 2);
        let before = b.events.len();
        b.fail = None;
        assert!(l.prepare(&mut b, [1; 32], [2; 32], 3, 2).is_err());
        assert_eq!(b.events.len(), before);
        assert!(l.commit(3, Ok(())).is_err());
    }
}
#[test]
fn scope_replay_duplicate_active_and_failed_tail_never_commit() {
    for mutation in 0..5 {
        let mut l = ledger();
        let mut b = Bank::new();
        let (r, m, g, p) = match mutation {
            0 => ([9; 32], [2; 32], 1, 0),
            1 => ([1; 32], [9; 32], 1, 0),
            2 => ([1; 32], [2; 32], 2, 0),
            3 => ([1; 32], [2; 32], 1, 1),
            _ => ([1; 32], [2; 32], 0, 0),
        };
        assert!(l.prepare(&mut b, r, m, g, p).is_err());
        assert!(b.events.is_empty());
    }
    let mut l = ledger();
    let mut b = Bank::new();
    l.prepare(&mut b, [1; 32], [2; 32], 1, 0).unwrap();
    assert!(l.prepare(&mut b, [1; 32], [2; 32], 1, 0).is_err());
    assert_eq!(l.completed, 0);
    let mut l = ledger();
    let mut b = Bank::new();
    l.prepare(&mut b, [1; 32], [2; 32], 1, 0).unwrap();
    assert!(l.commit(1, Err("tail/fence".into())).is_err());
    assert_eq!(l.completed, 0);
    assert!(l.commit(1, Ok(())).is_err());
}
#[test]
fn every_tiles_terminal_word_is_checked_not_just_header_or_page_extent() {
    let good = tiles_done();
    assert!(terminal(&good));
    assert_eq!(initial()[..4], [1, 0, 1, 0]);
    for i in 0..WORDS {
        let mut bad = good;
        bad[i] = if good[i] == 0 { 1 } else { 0 };
        assert!(!terminal(&bad), "word{i}");
    }
}
