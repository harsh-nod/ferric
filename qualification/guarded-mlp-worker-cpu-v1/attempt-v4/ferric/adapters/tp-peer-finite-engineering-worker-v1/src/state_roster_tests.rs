use super::*;

fn finish_layer(gate: &mut Gate, layer: usize) -> usize {
    let slot = gate
        .advance(layer, Phase::PrefixReady, Phase::PrefixInFlight)
        .unwrap();
    gate.advance(layer, Phase::PrefixInFlight, Phase::FirstResidualReady)
        .unwrap();
    gate.advance(
        layer,
        Phase::FirstResidualReady,
        Phase::FirstResidualInFlight,
    )
    .unwrap();
    gate.advance(layer, Phase::FirstResidualInFlight, Phase::MlpReady)
        .unwrap();
    assert_eq!(
        gate.advance(layer, Phase::MlpReady, Phase::MlpInFlight)
            .unwrap(),
        slot
    );
    gate.advance(layer, Phase::MlpInFlight, Phase::FinalResidualReady)
        .unwrap();
    gate.advance(
        layer,
        Phase::FinalResidualReady,
        Phase::FinalResidualInFlight,
    )
    .unwrap();
    gate.expect(layer, Phase::FinalResidualInFlight).unwrap();
    gate.layer += 1;
    gate.phase = if gate.layer == LAYERS {
        Phase::Tail
    } else {
        Phase::PrefixReady
    };
    slot
}

#[test]
fn two_forwards_consume_seventy_two_disjoint_layer_slots() {
    let mut gate = Gate::new();
    let mut slots = std::collections::BTreeSet::new();
    for forward in 0..FORWARDS {
        gate.begin(forward as u64 + 1, forward as u32).unwrap();
        for layer in 0..LAYERS {
            assert!(slots.insert(finish_layer(&mut gate, layer)));
        }
        gate.commit().unwrap();
    }
    assert_eq!(slots.len(), 72);
    assert_eq!(STATES_PER_RANK, 144);
    assert_eq!(gate.phase, Phase::Exhausted);
    assert!(gate.begin(3, 2).is_err());
    assert_eq!(gate.phase, Phase::Terminal);
}

#[test]
fn wrong_forward_position_generation_or_double_begin_is_terminal() {
    for (generation, position) in [(0, 0), (2, 0), (1, 1), (u64::MAX, u32::MAX)] {
        let mut gate = Gate::new();
        assert!(gate.begin(generation, position).is_err());
        assert!(gate.begin(1, 0).is_err());
    }
    let mut gate = Gate::new();
    gate.begin(1, 0).unwrap();
    assert!(gate.begin(1, 0).is_err());
}

#[test]
fn skipped_layer_and_reused_prefix_slot_are_terminal() {
    let mut gate = Gate::new();
    gate.begin(1, 0).unwrap();
    assert!(
        gate.advance(1, Phase::PrefixReady, Phase::PrefixInFlight)
            .is_err()
    );
    let mut gate = Gate::new();
    gate.begin(1, 0).unwrap();
    gate.advance(0, Phase::PrefixReady, Phase::PrefixInFlight)
        .unwrap();
    assert!(
        gate.advance(0, Phase::PrefixReady, Phase::PrefixInFlight)
            .is_err()
    );
}

#[test]
fn dependent_round_cannot_skip_predecessor_state_or_residual_gate() {
    for phase in [
        Phase::PrefixReady,
        Phase::PrefixInFlight,
        Phase::FirstResidualReady,
        Phase::FirstResidualInFlight,
    ] {
        let mut gate = Gate::new();
        gate.begin(1, 0).unwrap();
        gate.phase = phase;
        assert!(
            gate.advance(0, Phase::MlpReady, Phase::MlpInFlight)
                .is_err()
        );
        assert_eq!(gate.phase, Phase::Terminal);
    }
    let mut gate = Gate::new();
    gate.begin(1, 0).unwrap();
    gate.phase = Phase::MlpInFlight;
    assert!(
        gate.advance(0, Phase::FinalResidualReady, Phase::FinalResidualInFlight)
            .is_err()
    );
}

#[test]
fn token_commit_requires_all_layers_and_tail() {
    for phase in [
        Phase::BetweenForwards,
        Phase::PrefixReady,
        Phase::FinalResidualInFlight,
        Phase::Terminal,
    ] {
        let mut gate = Gate::new();
        gate.layer = LAYERS;
        gate.phase = phase;
        assert!(gate.commit().is_err());
    }
    let mut gate = Gate::new();
    gate.phase = Phase::Tail;
    gate.layer = LAYERS - 1;
    assert!(gate.commit().is_err());
}

#[test]
fn prefix_terminal_checks_every_word_without_shift_by_thirty_two() {
    let mut words = [64; 22];
    words[..6].copy_from_slice(&[1, 0, 0xffff, 0xffff, 0x5555_5555, 0]);
    assert!(prefix_terminal(&words));
    let mut other_owner = words;
    other_owner[4] = 0xaaaa_aaaa;
    assert!(prefix_terminal(&other_owner));
    for index in 0..words.len() {
        let mut bad = words;
        bad[index] = if index == 4 { 0 } else { bad[index] ^ 1 };
        assert!(!prefix_terminal(&bad), "word{index}");
    }
    let mut incomplete_error_zero = words;
    incomplete_error_zero[2] &= !1;
    incomplete_error_zero[3] &= !1;
    assert_eq!(incomplete_error_zero[5], 0);
    assert!(!prefix_terminal(&incomplete_error_zero));
}

#[test]
fn mlp_terminal_requires_all_five_arrivals_and_no_high_owner_bits() {
    let mut words = [64; 11];
    words[..6].copy_from_slice(&[1, 0, 31, 31, 0x155, 0]);
    assert!(mlp_terminal(&words));
    for index in 0..words.len() {
        let mut bad = words;
        bad[index] = if index == 4 { 0 } else { bad[index] ^ 1 };
        assert!(!mlp_terminal(&bad), "word{index}");
    }
    words[4] |= 1 << 10;
    assert!(!mlp_terminal(&words));
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct MockToken {
    group: u64,
    id: usize,
    rank: usize,
    kind: WorkerKind,
}

struct MockBackend {
    group: u64,
    terminal: bool,
    preflights: usize,
    occupied: Vec<usize>,
    fail_preflight: bool,
    fail_allocation: Option<usize>,
    allocated: Vec<MockToken>,
    observations: usize,
    fail_observation: Option<usize>,
    prefix: [[u32; 22]; RANKS],
    mlp: [[u32; 11]; RANKS],
}

impl MockBackend {
    fn new(group: u64) -> Self {
        let mut prefix = [64; 22];
        prefix[..6].copy_from_slice(&[1, 0, 0xffff, 0xffff, 0x5555_5555, 0]);
        let mut mlp = [64; 11];
        mlp[..6].copy_from_slice(&[1, 0, 31, 31, 0x155, 0]);
        Self {
            group,
            terminal: false,
            preflights: 0,
            occupied: vec![40, 52],
            fail_preflight: false,
            fail_allocation: None,
            allocated: Vec::new(),
            observations: 0,
            fail_observation: None,
            prefix: [prefix; RANKS],
            mlp: [mlp; RANKS],
        }
    }
    fn allocate(&mut self, rank: usize, kind: WorkerKind) -> Result<MockToken> {
        if self.terminal {
            return Err("mock resource owner is quarantined".into());
        }
        if self.fail_allocation == Some(self.allocated.len()) {
            self.terminal = true;
            return Err("injected native typed allocation failure".into());
        }
        let token = MockToken {
            group: self.group,
            id: self.allocated.len() + 1,
            rank,
            kind,
        };
        self.allocated.push(token.clone());
        Ok(token)
    }
    fn observe(&mut self, token: &MockToken, kind: WorkerKind) -> Result<()> {
        if self.terminal {
            return Err("mock resource owner is quarantined".into());
        }
        self.observations += 1;
        if token.group != self.group
            || token.kind != kind
            || token.rank >= RANKS
            || self.allocated.get(token.id.saturating_sub(1)) != Some(token)
            || self.fail_observation == Some(self.observations)
        {
            self.terminal = true;
            return Err("foreign token or injected native observation failure".into());
        }
        Ok(())
    }
}

impl StateBackend for MockBackend {
    type Prefix = MockToken;
    type Mlp = MockToken;
    fn preflight(&mut self, additional: &[usize]) -> Result<Vec<usize>> {
        if self.terminal {
            return Err("mock resource owner is quarantined".into());
        }
        self.preflights += 1;
        assert_eq!(additional, &[STATES_PER_RANK; RANKS]);
        if self.fail_preflight {
            self.terminal = true;
            return Err("injected preflight failure".into());
        }
        Ok(self.occupied.clone())
    }
    fn allocate_prefix(&mut self, rank: usize) -> Result<MockToken> {
        self.allocate(rank, WorkerKind::PrefixV5)
    }
    fn allocate_mlp(&mut self, rank: usize) -> Result<MockToken> {
        self.allocate(rank, WorkerKind::MlpV1)
    }
    fn observe_prefix(&mut self, token: &MockToken) -> Result<[u32; 22]> {
        self.observe(token, WorkerKind::PrefixV5)?;
        Ok(self.prefix[token.rank])
    }
    fn observe_mlp(&mut self, token: &MockToken) -> Result<[u32; 11]> {
        self.observe(token, WorkerKind::MlpV1)?;
        Ok(self.mlp[token.rank])
    }
}

#[test]
fn exact_model_roster_preflights_once_and_keeps_two_hundred_eighty_eight_typed_states() {
    let mut backend = MockBackend::new(9);
    let reserved = reserve(&mut backend, [3; 32]).unwrap();
    assert_eq!(backend.preflights, 1);
    assert_eq!(reserved.occupied_before, [40, 52]);
    assert_eq!(reserved.states.len(), FORWARDS * LAYERS);
    assert_eq!(reserved.identities.len(), RANKS * STATES_PER_RANK);
    assert_eq!(backend.allocated.len(), RANKS * STATES_PER_RANK);
    assert_eq!(reserved.identities[0].kind, WorkerKind::PrefixV5);
    assert_eq!(reserved.identities[1].rank, 1);
    assert_eq!(reserved.identities[2].kind, WorkerKind::MlpV1);
    assert_eq!(reserved.identities[144].forward, 1);
    for (slot, state) in reserved.states.iter().enumerate() {
        for rank in 0..RANKS {
            for (kind, token) in [
                (WorkerKind::PrefixV5, &state.prefix[rank]),
                (WorkerKind::MlpV1, &state.mlp[rank]),
            ] {
                assert_eq!((token.group, token.rank, token.kind), (9, rank, kind));
                let identity = reserved
                    .identities
                    .iter()
                    .find(|row| {
                        row.forward as usize == slot / LAYERS
                            && row.layer as usize == slot % LAYERS
                            && row.rank as usize == rank
                            && row.kind == kind
                    })
                    .unwrap();
                assert_eq!(identity.model, [3; 32]);
                assert_eq!(
                    reserved.identities[(identity.reservation_id - 1) as usize],
                    *identity
                );
            }
        }
    }
}

#[test]
fn missing_identity_bad_preflight_and_wrong_owner_roster_allocate_nothing() {
    let mut backend = MockBackend::new(1);
    assert!(reserve(&mut backend, [0; 32]).is_err());
    assert_eq!(backend.preflights, 0);
    for occupied in [vec![], vec![0], vec![0; 3]] {
        let mut backend = MockBackend::new(1);
        backend.occupied = occupied;
        assert!(reserve(&mut backend, [3; 32]).is_err());
        assert!(backend.allocated.is_empty());
    }
    backend.fail_preflight = true;
    assert!(reserve(&mut backend, [3; 32]).is_err());
    assert!(backend.terminal && backend.allocated.is_empty());
}

#[test]
fn every_partial_setup_failure_retains_original_allocations_and_prevents_retry() {
    for failed_index in 0..RANKS * STATES_PER_RANK {
        let mut backend = MockBackend::new(1);
        backend.fail_allocation = Some(failed_index);
        assert!(reserve(&mut backend, [3; 32]).is_err());
        assert!(backend.terminal);
        assert_eq!(backend.allocated.len(), failed_index);
        let retained = backend.allocated.clone();
        assert!(reserve(&mut backend, [3; 32]).is_err());
        assert_eq!(backend.allocated, retained);
        assert_eq!(backend.preflights, 1);
    }
}

#[test]
fn paired_observations_require_both_original_rank_tokens_before_dependents() {
    let mut backend = MockBackend::new(1);
    let reserved = reserve(&mut backend, [3; 32]).unwrap();
    let mut gate = Gate::new();
    gate.begin(1, 0).unwrap();
    gate.advance(0, Phase::PrefixReady, Phase::PrefixInFlight)
        .unwrap();
    assert!(observe_prefix_pair(&mut gate, &mut backend, &reserved.states[0].prefix).is_ok());
    assert_eq!(
        (backend.observations, gate.phase),
        (2, Phase::FirstResidualReady)
    );
    gate.advance(0, Phase::FirstResidualReady, Phase::FirstResidualInFlight)
        .unwrap();
    gate.outcome(Ok(())).unwrap();
    gate.advance(0, Phase::FirstResidualInFlight, Phase::MlpReady)
        .unwrap();
    gate.advance(0, Phase::MlpReady, Phase::MlpInFlight)
        .unwrap();
    assert!(observe_mlp_pair(&mut gate, &mut backend, &reserved.states[0].mlp).is_ok());
    assert_eq!(
        (backend.observations, gate.phase),
        (4, Phase::FinalResidualReady)
    );
}

#[test]
fn foreign_group_and_each_failed_acquire_make_host_gate_terminal_without_reuse() {
    for foreign in [false, true] {
        for failing_read in 1..=2 {
            for prefix in [false, true] {
                let mut original = MockBackend::new(1);
                let reserved = reserve(&mut original, [3; 32]).unwrap();
                let mut backend = if foreign {
                    MockBackend::new(2)
                } else {
                    original
                };
                backend.fail_observation = Some(failing_read);
                let mut gate = Gate::new();
                gate.begin(1, 0).unwrap();
                gate.phase = if prefix {
                    Phase::PrefixInFlight
                } else {
                    Phase::MlpInFlight
                };
                if prefix {
                    assert!(
                        observe_prefix_pair(&mut gate, &mut backend, &reserved.states[0].prefix)
                            .is_err()
                    );
                } else {
                    assert!(
                        observe_mlp_pair(&mut gate, &mut backend, &reserved.states[0].mlp).is_err()
                    );
                }
                assert!(backend.terminal);
                assert_eq!(gate.phase, Phase::Terminal);
                let count = backend.observations;
                if prefix {
                    assert!(
                        observe_prefix_pair(&mut gate, &mut backend, &reserved.states[0].prefix)
                            .is_err()
                    );
                } else {
                    assert!(
                        observe_mlp_pair(&mut gate, &mut backend, &reserved.states[0].mlp).is_err()
                    );
                }
                assert_eq!(backend.observations, count);
                assert!(gate.commit().is_err());
            }
        }
    }
}

#[test]
fn error_zero_incomplete_state_on_either_rank_prevents_residual_and_token_commit() {
    for rank in 0..RANKS {
        for prefix in [false, true] {
            let mut backend = MockBackend::new(1);
            let reserved = reserve(&mut backend, [3; 32]).unwrap();
            let mut gate = Gate::new();
            gate.begin(1, 0).unwrap();
            if prefix {
                backend.prefix[rank][2] &= !1;
                gate.phase = Phase::PrefixInFlight;
                assert!(
                    observe_prefix_pair(&mut gate, &mut backend, &reserved.states[0].prefix)
                        .is_err()
                );
                assert_eq!(backend.prefix[rank][5], 0);
            } else {
                backend.mlp[rank][6] = 63;
                gate.phase = Phase::MlpInFlight;
                assert!(
                    observe_mlp_pair(&mut gate, &mut backend, &reserved.states[0].mlp).is_err()
                );
                assert_eq!(backend.mlp[rank][5], 0);
            }
            assert_eq!(gate.phase, Phase::Terminal);
            assert!(gate.commit().is_err());
            assert!(gate.begin(1, 0).is_err());
            // A semantic failure fences the private host owner. It is not
            // falsely reported as a native allocation/readback failure.
            assert!(!backend.terminal);
            assert_eq!(backend.allocated.len(), RANKS * STATES_PER_RANK);
        }
    }
}

#[test]
fn failed_native_results_at_each_host_phase_are_sticky_without_advancing() {
    for phase in [
        Phase::PrefixInFlight,
        Phase::FirstResidualInFlight,
        Phase::MlpInFlight,
        Phase::FinalResidualInFlight,
        Phase::Tail,
        Phase::Exhausted,
    ] {
        let mut gate = Gate::new();
        gate.phase = phase;
        gate.layer = if matches!(phase, Phase::Tail | Phase::Exhausted) {
            LAYERS
        } else {
            0
        };
        let before = (gate.forward, gate.layer);
        let error: Result<()> =
            Err("injected native dispatch, tail, read, fence or close failure".into());
        assert!(gate.outcome(error).is_err());
        assert_eq!(gate.phase, Phase::Terminal);
        assert_eq!((gate.forward, gate.layer), before);
        assert!(gate.outcome(Ok(())).is_err());
        assert!(gate.commit().is_err());
        assert!(gate.begin(1, 0).is_err());
    }
}
