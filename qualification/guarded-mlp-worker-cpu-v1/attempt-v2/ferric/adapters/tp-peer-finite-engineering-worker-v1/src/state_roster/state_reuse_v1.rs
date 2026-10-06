//! Private engineering state reuse. Device epoch1 is not a host generation.
//! Requires a separately admitted long-forward profile; P223 never selects this.

use super::{
    Gate, Group, LAYERS, LayerStates, Phase, RANKS, Result, StateRoster, mlp_terminal,
    prefix_terminal,
};

pub(crate) const MAX_FORWARDS: u64 = 2303;
const BANKS: usize = 2;
const EXACT_ALLOCATIONS: [usize; 2] = [714, 710];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReusePhase {
    Between,
    Rearming,
    Active(u64),
    Exhausted,
    Terminal,
}

struct Ledger {
    registration: [u8; 32],
    model: [u8; 32],
    completed: u64,
    bank_generation: [u64; BANKS],
    phase: ReusePhase,
}

trait ReuseBackend {
    fn fence(&mut self) -> Result<()>;
    fn prefix(&mut self, bank: usize, layer: usize, rank: usize) -> Result<[u32; 22]>;
    fn mlp(&mut self, bank: usize, layer: usize, rank: usize) -> Result<[u32; 11]>;
    fn rearm_prefix(
        &mut self,
        bank: usize,
        layer: usize,
        rank: usize,
        expected: &[u32; 22],
    ) -> Result<()>;
    fn rearm_mlp(
        &mut self,
        bank: usize,
        layer: usize,
        rank: usize,
        expected: &[u32; 11],
    ) -> Result<()>;
}

fn initial<const N: usize>() -> [u32; N] {
    core::array::from_fn(|index| u32::from(index < 2))
}

impl Ledger {
    fn new(registration: [u8; 32], model: [u8; 32]) -> Result<Self> {
        if registration == [0; 32] || model == [0; 32] {
            return Err("empty reuse identity".into());
        }
        Ok(Self {
            registration,
            model,
            completed: 0,
            bank_generation: [0; BANKS],
            phase: ReusePhase::Between,
        })
    }

    fn poison<T>(&mut self, error: impl Into<String>) -> Result<T> {
        self.phase = ReusePhase::Terminal;
        Err(error.into())
    }

    fn prepare(
        &mut self,
        backend: &mut impl ReuseBackend,
        registration: [u8; 32],
        model: [u8; 32],
        generation: u64,
        position: u32,
    ) -> Result<usize> {
        let result: Result<usize> = (|| {
            if self.phase != ReusePhase::Between
                || registration != self.registration
                || model != self.model
                || self.completed >= MAX_FORWARDS
                || self.completed.checked_add(1) != Some(generation)
                || u64::from(position) != self.completed
            {
                return Err("reuse identity, generation, position or phase".into());
            }
            let bank = ((generation - 1) % BANKS as u64) as usize;
            let prior = self.bank_generation[bank];
            if prior != generation.checked_sub(BANKS as u64).unwrap_or(0) {
                return Err("reuse bank generation mismatch".into());
            }
            self.phase = ReusePhase::Rearming;
            backend.fence()?;
            // Validate the complete bank before the first write. No reset after
            // a stale, missing, erroneous or in-flight completion is permitted.
            let mut snapshots = [([[0; 22]; RANKS], [[0; 11]; RANKS]); LAYERS];
            for layer in 0..LAYERS {
                let p = [
                    backend.prefix(bank, layer, 0)?,
                    backend.prefix(bank, layer, 1)?,
                ];
                let m = [backend.mlp(bank, layer, 0)?, backend.mlp(bank, layer, 1)?];
                if if prior == 0 {
                    p.iter().any(|v| *v != initial()) || m.iter().any(|v| *v != initial())
                } else {
                    !p.iter().all(prefix_terminal) || !m.iter().all(mlp_terminal)
                } {
                    return Err("reuse bank is not exactly fresh/terminal".into());
                }
                snapshots[layer] = (p, m);
            }
            if prior != 0 {
                for (layer, (p, m)) in snapshots.iter().enumerate() {
                    for rank in 0..RANKS {
                        backend.rearm_prefix(bank, layer, rank, &p[rank])?;
                        backend.rearm_mlp(bank, layer, rank, &m[rank])?;
                    }
                }
            }
            for layer in 0..LAYERS {
                for rank in 0..RANKS {
                    if backend.prefix(bank, layer, rank)? != initial()
                        || backend.mlp(bank, layer, rank)? != initial()
                    {
                        return Err("reuse initial readback mismatch".into());
                    }
                }
            }
            backend.fence()?;
            self.bank_generation[bank] = generation;
            self.phase = ReusePhase::Active(generation);
            Ok(bank)
        })();
        match result {
            Ok(bank) => Ok(bank),
            Err(error) => self.poison(error),
        }
    }

    fn require_active(&mut self, generation: u64) -> Result<()> {
        if self.phase != ReusePhase::Active(generation) {
            return self.poison("stale or non-active reuse generation");
        }
        Ok(())
    }

    fn commit(&mut self, generation: u64, completed_tail_and_fence: Result<()>) -> Result<()> {
        self.require_active(generation)?;
        if let Err(error) = completed_tail_and_fence {
            return self.poison(error);
        }
        if self.completed.checked_add(1) != Some(generation) || generation > MAX_FORWARDS {
            return self.poison("reuse generation rollover/exhaustion");
        }
        self.completed = generation;
        self.phase = if generation == MAX_FORWARDS {
            ReusePhase::Exhausted
        } else {
            ReusePhase::Between
        };
        Ok(())
    }
}

struct NativeBank<'a> {
    group: &'a mut Group,
    states: &'a mut [LayerStates],
}

fn idle(group: &mut Group) -> Result<()> {
    if group.preflight_additional_allocations_v1(&[0, 0])? != EXACT_ALLOCATIONS {
        return Err("reuse allocation census changed".into());
    }
    Ok(())
}

impl ReuseBackend for NativeBank<'_> {
    fn fence(&mut self) -> Result<()> {
        idle(self.group)
    }
    fn prefix(&mut self, bank: usize, layer: usize, rank: usize) -> Result<[u32; 22]> {
        self.group
            .observe_wave_output_state_v5(&self.states[bank * LAYERS + layer].prefix[rank])
    }
    fn mlp(&mut self, bank: usize, layer: usize, rank: usize) -> Result<[u32; 11]> {
        self.group
            .observe_wave_mlp_state_v1(&self.states[bank * LAYERS + layer].mlp[rank])
    }
    fn rearm_prefix(
        &mut self,
        bank: usize,
        layer: usize,
        rank: usize,
        expected: &[u32; 22],
    ) -> Result<()> {
        // SAFETY: the private ledger has committed the previous use, fenced all
        // queues and acquired every bank state. No old command may be retained.
        unsafe {
            self.group.rearm_wave_output_state_for_reuse_v1(
                &mut self.states[bank * LAYERS + layer].prefix[rank],
                expected,
            )
        }
    }
    fn rearm_mlp(
        &mut self,
        bank: usize,
        layer: usize,
        rank: usize,
        expected: &[u32; 11],
    ) -> Result<()> {
        // SAFETY: same closed idle/generation boundary as the prefix rearm.
        unsafe {
            self.group.rearm_wave_mlp_state_for_reuse_v1(
                &mut self.states[bank * LAYERS + layer].mlp[rank],
                expected,
            )
        }
    }
}

/// A distinct private owner; does not change the original two-forward API.
/// Both physical banks and all typed allocations remain retained until close.
pub(crate) struct ReusableStateRosterV1 {
    inner: StateRoster,
    ledger: Ledger,
}

fn physical_boundary(completed: u64) -> Phase {
    if completed > 0 && completed % 2 == 0 {
        Phase::Exhausted
    } else {
        Phase::BetweenForwards
    }
}

fn begin_physical_bank(gate: &mut Gate, bank: usize) -> Result<()> {
    if bank >= BANKS {
        return gate.reject("physical reuse bank bound");
    }
    *gate = Gate {
        forward: bank,
        layer: 0,
        phase: Phase::BetweenForwards,
    };
    gate.begin(bank as u64 + 1, bank as u32)
}

impl ReusableStateRosterV1 {
    /// Only a separately validated long-forward registration may choose this.
    /// The input must be the exact fresh sealed two-bank allocation roster, not
    /// an owner that has already dispatched or a token reconstructed from bytes.
    pub(crate) fn from_fresh(inner: StateRoster) -> Result<Self> {
        if inner.gate.phase != Phase::BetweenForwards
            || inner.gate.forward != 0
            || inner.gate.layer != 0
            || inner.states.len() != BANKS * LAYERS
            || inner.identities.len() != 288
        {
            return Err("reuse requires a fresh exact two-bank owner".into());
        }
        let ledger = Ledger::new(inner.registration, inner.model)?;
        Ok(Self { inner, ledger })
    }

    pub(crate) fn begin_forward(
        &mut self,
        group: &mut Group,
        registration: [u8; 32],
        model: [u8; 32],
        generation: u64,
        position: u32,
    ) -> Result<()> {
        let result: Result<()> = (|| {
            if self.inner.gate.phase != physical_boundary(self.ledger.completed) {
                return Err("reuse inner roster still active/terminal".into());
            }
            let bank = self.ledger.prepare(
                &mut NativeBank {
                    group,
                    states: &mut self.inner.states,
                },
                registration,
                model,
                generation,
                position,
            )?;
            // The unchanged inner gate indexes physical slots. Host generation
            // identity remains exclusively in this new ledger, never in epoch1.
            begin_physical_bank(&mut self.inner.gate, bank)
        })();
        if result.is_err() {
            self.poison();
        }
        result
    }

    /// Borrow only for the existing closed resident_layer::execute_layer call.
    /// Neither the owner nor its saved pointer records may escape that call.
    pub(crate) fn active_states(&mut self, generation: u64) -> Result<&mut StateRoster> {
        if let Err(error) = self.ledger.require_active(generation) {
            self.inner.poison();
            return Err(error);
        }
        Ok(&mut self.inner)
    }

    pub(crate) fn commit_forward(
        &mut self,
        group: &mut Group,
        generation: u64,
        validated_tail: Result<()>,
    ) -> Result<()> {
        let result: Result<()> = (|| {
            self.ledger.require_active(generation)?;
            self.inner.commit_forward(validated_tail)?;
            let fence = idle(group);
            self.ledger.commit(generation, fence)
        })();
        if result.is_err() {
            self.poison();
        }
        result
    }

    pub(crate) fn exhausted(&self) -> bool {
        self.ledger.phase == ReusePhase::Exhausted
    }
    pub(crate) fn poison(&mut self) {
        self.ledger.phase = ReusePhase::Terminal;
        self.inner.poison();
    }
}

#[cfg(test)]
mod tests;
