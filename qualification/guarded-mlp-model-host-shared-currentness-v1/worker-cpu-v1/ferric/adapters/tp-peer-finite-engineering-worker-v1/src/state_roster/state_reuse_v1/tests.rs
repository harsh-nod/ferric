use super::*;

const REG: [u8; 32] = [1; 32];
const MODEL: [u8; 32] = [2; 32];

fn terminal<const N: usize>(tasks: usize) -> [u32; N] {
    let mut words = [64; N];
    words[..6].copy_from_slice(&[
        1,
        0,
        (1 << tasks) - 1,
        (1 << tasks) - 1,
        (0..tasks).fold(0, |bits, task| bits | (1 << (2 * task))),
        0,
    ]);
    words
}

struct Fake {
    prefix: Vec<[[u32; 22]; RANKS]>,
    mlp: Vec<[[u32; 11]; RANKS]>,
    fences: usize,
    resets: Vec<(usize, usize, usize, bool)>,
    fail_fence: Option<usize>,
    fail_reset: Option<usize>,
    corrupt_reset: bool,
}

impl Default for Fake {
    fn default() -> Self {
        Self {
            prefix: vec![[initial(); RANKS]; BANKS * LAYERS],
            mlp: vec![[initial(); RANKS]; BANKS * LAYERS],
            fences: 0,
            resets: vec![],
            fail_fence: None,
            fail_reset: None,
            corrupt_reset: false,
        }
    }
}

impl Fake {
    fn complete(&mut self, bank: usize) {
        for layer in 0..LAYERS {
            self.prefix[bank * LAYERS + layer] = [terminal(16); RANKS];
            self.mlp[bank * LAYERS + layer] = [terminal(5); RANKS];
        }
    }

    fn reset(&mut self, bank: usize, layer: usize, rank: usize, prefix: bool) -> Result<()> {
        if self.fail_reset == Some(self.resets.len()) {
            return Err("reset failure".into());
        }
        self.resets.push((bank, layer, rank, prefix));
        Ok(())
    }
}

impl ReuseBackend for Fake {
    fn fence(&mut self) -> Result<()> {
        self.fences += 1;
        if self.fail_fence == Some(self.fences) {
            Err("fence failure".into())
        } else {
            Ok(())
        }
    }
    fn prefix(&mut self, bank: usize, layer: usize, rank: usize) -> Result<[u32; 22]> {
        Ok(self.prefix[bank * LAYERS + layer][rank])
    }
    fn mlp(&mut self, bank: usize, layer: usize, rank: usize) -> Result<[u32; 11]> {
        Ok(self.mlp[bank * LAYERS + layer][rank])
    }
    fn rearm_prefix(
        &mut self,
        bank: usize,
        layer: usize,
        rank: usize,
        expected: &[u32; 22],
    ) -> Result<()> {
        assert_eq!(self.prefix[bank * LAYERS + layer][rank], *expected);
        assert!(prefix_terminal(expected));
        self.reset(bank, layer, rank, true)?;
        self.prefix[bank * LAYERS + layer][rank] = initial();
        if self.corrupt_reset {
            self.prefix[bank * LAYERS + layer][rank][5] = 1;
        }
        Ok(())
    }
    fn rearm_mlp(
        &mut self,
        bank: usize,
        layer: usize,
        rank: usize,
        expected: &[u32; 11],
    ) -> Result<()> {
        assert_eq!(self.mlp[bank * LAYERS + layer][rank], *expected);
        assert!(mlp_terminal(expected));
        self.reset(bank, layer, rank, false)?;
        self.mlp[bank * LAYERS + layer][rank] = initial();
        Ok(())
    }
}

fn finish(ledger: &mut Ledger, backend: &mut Fake, generation: u64) {
    backend.complete(((generation - 1) % 2) as usize);
    ledger.commit(generation, Ok(())).unwrap();
}

fn first_two() -> (Ledger, Fake) {
    let mut ledger = Ledger::new(REG, MODEL).unwrap();
    let mut backend = Fake::default();
    for generation in 1..=2 {
        ledger
            .prepare(&mut backend, REG, MODEL, generation, generation as u32 - 1)
            .unwrap();
        finish(&mut ledger, &mut backend, generation);
    }
    (ledger, backend)
}

#[test]
fn first_reuse_is_generation_three_and_only_resets_completed_bank_zero() {
    let (mut ledger, mut backend) = first_two();
    assert!(backend.resets.is_empty());
    let saved_prefix = backend.prefix[LAYERS..].to_vec();
    let saved_mlp = backend.mlp[LAYERS..].to_vec();
    assert_eq!(ledger.prepare(&mut backend, REG, MODEL, 3, 2).unwrap(), 0);
    assert_eq!(backend.resets.len(), LAYERS * RANKS * 2);
    assert!(backend.resets.iter().all(|&(bank, _, _, _)| bank == 0));
    assert_eq!(backend.prefix[LAYERS..], saved_prefix);
    assert_eq!(backend.mlp[LAYERS..], saved_mlp);
    assert_eq!(ledger.bank_generation, [3, 2]);
    assert_eq!(ledger.completed, 2);
}

#[test]
fn stale_generation_model_registration_and_position_fail_before_any_fence() {
    for mutation in 0..5 {
        let (mut ledger, mut backend) = first_two();
        let (mut reg, mut model, mut generation, mut position) = (REG, MODEL, 3, 2);
        match mutation {
            0 => reg[0] ^= 1,
            1 => model[0] ^= 1,
            2 => generation = 2,
            3 => position = 3,
            _ => generation = u64::MAX,
        }
        let before = backend.fences;
        assert!(
            ledger
                .prepare(&mut backend, reg, model, generation, position)
                .is_err()
        );
        assert_eq!(backend.fences, before);
        assert!(backend.resets.is_empty());
        assert_eq!(ledger.phase, ReusePhase::Terminal);
    }
}

#[test]
fn inflight_generation_cannot_be_rearmed_or_reentered() {
    let mut ledger = Ledger::new(REG, MODEL).unwrap();
    let mut backend = Fake::default();
    ledger.prepare(&mut backend, REG, MODEL, 1, 0).unwrap();
    let fences = backend.fences;
    assert!(ledger.prepare(&mut backend, REG, MODEL, 2, 1).is_err());
    assert_eq!(backend.fences, fences);
    assert!(backend.resets.is_empty());
}

#[test]
fn failed_initial_fence_preserves_all_bank_words_and_poison_is_sticky() {
    let (mut ledger, mut backend) = first_two();
    backend.fail_fence = Some(backend.fences + 1);
    let before = (backend.prefix.clone(), backend.mlp.clone());
    assert!(ledger.prepare(&mut backend, REG, MODEL, 3, 2).is_err());
    assert_eq!((backend.prefix.clone(), backend.mlp.clone()), before);
    backend.fail_fence = None;
    assert!(ledger.prepare(&mut backend, REG, MODEL, 3, 2).is_err());
    assert!(backend.resets.is_empty());
}

#[test]
fn last_bad_terminal_state_rejects_before_first_reset() {
    for mutation in 0..5 {
        let (mut ledger, mut backend) = first_two();
        let word = &mut backend.mlp[LAYERS - 1][1];
        match mutation {
            0 => word[0] = 2,
            1 => word[2] = 0,
            2 => word[4] |= 1 << 20,
            3 => word[5] = 1,
            _ => word[10] = 63,
        }
        assert!(ledger.prepare(&mut backend, REG, MODEL, 3, 2).is_err());
        assert!(backend.resets.is_empty());
    }
}

#[test]
fn partial_reset_failure_never_publishes_new_generation_or_retries() {
    let (mut ledger, mut backend) = first_two();
    backend.fail_reset = Some(13);
    assert!(ledger.prepare(&mut backend, REG, MODEL, 3, 2).is_err());
    assert_eq!(backend.resets.len(), 13);
    assert_eq!(ledger.bank_generation, [1, 2]);
    backend.fail_reset = None;
    assert!(ledger.prepare(&mut backend, REG, MODEL, 3, 2).is_err());
    assert_eq!(backend.resets.len(), 13);
}

#[test]
fn reset_readback_and_post_fence_failures_are_terminal() {
    for bad_readback in [false, true] {
        let (mut ledger, mut backend) = first_two();
        if bad_readback {
            backend.corrupt_reset = true;
        } else {
            backend.fail_fence = Some(backend.fences + 2);
        }
        assert!(ledger.prepare(&mut backend, REG, MODEL, 3, 2).is_err());
        assert_eq!(ledger.bank_generation, [1, 2]);
        assert_eq!(ledger.phase, ReusePhase::Terminal);
    }
}

#[test]
fn failed_tail_or_stale_completion_cannot_authorize_reuse() {
    for stale in [false, true] {
        let mut ledger = Ledger::new(REG, MODEL).unwrap();
        let mut backend = Fake::default();
        ledger.prepare(&mut backend, REG, MODEL, 1, 0).unwrap();
        let result = if stale {
            ledger.commit(2, Ok(()))
        } else {
            ledger.commit(1, Err("tail or fence failed".into()))
        };
        assert!(result.is_err());
        assert_eq!(ledger.completed, 0);
        assert!(ledger.prepare(&mut backend, REG, MODEL, 2, 1).is_err());
    }
}

#[test]
fn two_banks_cover_exactly_2303_generations_without_new_allocations() {
    let mut ledger = Ledger::new(REG, MODEL).unwrap();
    let mut backend = Fake::default();
    for generation in 1..=MAX_FORWARDS {
        assert_eq!(
            ledger
                .prepare(&mut backend, REG, MODEL, generation, generation as u32 - 1)
                .unwrap(),
            ((generation - 1) % 2) as usize
        );
        finish(&mut ledger, &mut backend, generation);
    }
    assert_eq!(backend.prefix.len(), 72);
    assert_eq!(backend.mlp.len(), 72);
    assert_eq!(ledger.completed, 2303);
    assert_eq!(ledger.phase, ReusePhase::Exhausted);
    assert_eq!(backend.resets.len(), (2303 - 2) * LAYERS * RANKS * 2);
    assert!(
        ledger
            .prepare(&mut backend, REG, MODEL, 2304, 2303)
            .is_err()
    );
}

#[test]
fn generation_overflow_and_corrupt_bank_history_fail_closed() {
    let (mut ledger, mut backend) = first_two();
    ledger.completed = u64::MAX;
    assert!(ledger.prepare(&mut backend, REG, MODEL, 0, 0).is_err());
    let (mut ledger, mut backend) = first_two();
    ledger.bank_generation[0] = 2;
    assert!(ledger.prepare(&mut backend, REG, MODEL, 3, 2).is_err());
    assert!(backend.resets.is_empty());
}

#[test]
fn unchanged_inner_gate_reopens_physical_banks_across_generations_two_three_four() {
    let mut physical = Gate::new();
    let mut ledger = Ledger::new(REG, MODEL).unwrap();
    let mut backend = Fake::default();
    for generation in 1..=6 {
        assert_eq!(physical.phase, physical_boundary(ledger.completed));
        let bank = ledger
            .prepare(&mut backend, REG, MODEL, generation, generation as u32 - 1)
            .unwrap();
        begin_physical_bank(&mut physical, bank).unwrap();
        assert_eq!(
            physical.expect(0, Phase::PrefixReady).unwrap(),
            bank * LAYERS
        );
        assert_eq!(physical.forward, bank);
        // Simulate the already separately tested complete36-layer/tail path.
        physical.layer = LAYERS;
        physical.phase = Phase::Tail;
        physical.commit().unwrap();
        finish(&mut ledger, &mut backend, generation);
        assert_eq!(physical.phase, physical_boundary(ledger.completed));
    }
    assert!(begin_physical_bank(&mut physical, 2).is_err());
    assert_eq!(physical.phase, Phase::Terminal);
}
