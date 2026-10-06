//! Distinct execution roster. The original source Program still describes V1.
//! No V1 MLP allocation, pointer conversion, or eleven-word completion exists here.

use super::{
    Gate, Group, LAYERS, Phase, PrefixState, ReservationIdentity, Result, WorkerKind,
    prefix_terminal,
};
use fe2o3_kfd::Gfx950EngineeringPeerWaveMlpTilesStateV2 as TilesState;

pub(crate) const BANKS: usize = 2;
pub(crate) const WORDS: usize = 548;
pub(crate) const COUNTS: [usize; 2] = [714, 710];

struct Layer<P = PrefixState, M = TilesState> {
    prefix: [P; 2],
    mlp: [M; 2],
}

trait Allocator {
    type Prefix;
    type Tiles;
    fn preflight(&mut self, counts: &[usize]) -> Result<Vec<usize>>;
    fn prefix(&mut self, rank: usize) -> Result<Self::Prefix>;
    fn tiles(&mut self, rank: usize) -> Result<Self::Tiles>;
}
impl Allocator for Group {
    type Prefix = PrefixState;
    type Tiles = TilesState;
    fn preflight(&mut self, counts: &[usize]) -> Result<Vec<usize>> {
        self.preflight_additional_allocations_v1(counts)
    }
    fn prefix(&mut self, rank: usize) -> Result<PrefixState> {
        self.allocate_wave_output_state_v5(rank)
    }
    fn tiles(&mut self, rank: usize) -> Result<TilesState> {
        self.allocate_wave_mlp_tiles_state_v2(rank)
    }
}
fn allocate<A: Allocator>(
    a: &mut A,
    model: [u8; 32],
) -> Result<(Vec<Layer<A::Prefix, A::Tiles>>, Vec<ReservationIdentity>)> {
    if model == [0; 32] {
        return Err("tiles roster empty model".into());
    }
    let _: [usize; 2] = a
        .preflight(&[144, 144])?
        .try_into()
        .map_err(|_| "tiles roster owner count")?;
    let mut states = Vec::with_capacity(BANKS * LAYERS);
    let mut ids = Vec::with_capacity(288);
    for bank in 0..BANKS {
        for layer in 0..LAYERS {
            let prefix = [a.prefix(0)?, a.prefix(1)?];
            let mlp = [a.tiles(0)?, a.tiles(1)?];
            for kind in [WorkerKind::PrefixV5, WorkerKind::MlpTilesV2] {
                for rank in 0..2 {
                    ids.push(ReservationIdentity {
                        reservation_id: ids.len() as u64 + 1,
                        model,
                        forward: bank as u32,
                        layer: layer as u32,
                        rank,
                        kind,
                    });
                }
            }
            states.push(Layer { prefix, mlp });
        }
    }
    Ok((states, ids))
}

pub(crate) struct Pending {
    model: [u8; 32],
    states: Vec<Layer>,
    ids: Vec<ReservationIdentity>,
}
impl Pending {
    pub(crate) fn allocate(group: &mut Group, model: [u8; 32]) -> Result<Self> {
        let (states, ids) = allocate(group, model)?;
        Ok(Self { model, states, ids })
    }
    pub(crate) fn catalog(&self) -> &[ReservationIdentity] {
        &self.ids
    }
    pub(crate) fn seal(self, registration: [u8; 32]) -> Result<Roster> {
        if registration == [0; 32] || self.states.len() != 72 || self.ids.len() != 288 {
            return Err("tiles roster seal identity/cardinality".into());
        }
        Ok(Roster {
            states: self.states,
            gate: Gate::new(),
            ledger: reuse::Ledger::new(registration, self.model)?,
        })
    }
}

pub(crate) fn initial() -> [u32; WORDS] {
    core::array::from_fn(|i| u32::from(i == 0 || i == 2))
}
pub(crate) fn terminal(words: &[u32; WORDS]) -> bool {
    // Same strict decoder used by the independently bounded comparison annex.
    crate::finite_mlp_tiles_comparison_wire_v1::validate_tiles_state(words).is_ok()
}
fn idle(group: &mut Group) -> Result<()> {
    if group.preflight_additional_allocations_v1(&[0, 0])? != COUNTS {
        return Err("tiles decode allocation census changed".into());
    }
    Ok(())
}

pub(crate) struct Roster {
    states: Vec<Layer>,
    gate: Gate,
    ledger: reuse::Ledger,
}
impl Roster {
    pub(crate) fn begin(
        &mut self,
        group: &mut Group,
        registration: [u8; 32],
        model: [u8; 32],
        generation: u64,
        position: u32,
    ) -> Result<()> {
        let result = (|| {
            let boundary = if self.ledger.completed > 0 && self.ledger.completed % 2 == 0 {
                Phase::Exhausted
            } else {
                Phase::BetweenForwards
            };
            if self.gate.phase != boundary {
                return Err("tiles previous forward active".into());
            }
            let bank = self.ledger.prepare(
                &mut reuse::Native {
                    group,
                    states: &mut self.states,
                },
                registration,
                model,
                generation,
                position,
            )?;
            self.gate = Gate {
                forward: bank,
                layer: 0,
                phase: Phase::BetweenForwards,
            };
            self.gate.begin(bank as u64 + 1, bank as u32)
        })();
        if result.is_err() {
            self.poison();
        }
        result
    }
    pub(crate) fn take_prefix(&mut self, layer: usize) -> Result<[&PrefixState; 2]> {
        let slot = self
            .gate
            .advance(layer, Phase::PrefixReady, Phase::PrefixInFlight)?;
        Ok([&self.states[slot].prefix[0], &self.states[slot].prefix[1]])
    }
    pub(crate) fn finish_prefix(
        &mut self,
        group: &mut Group,
        layer: usize,
        result: Result<()>,
    ) -> Result<[[u32; 22]; 2]> {
        self.gate.outcome(result)?;
        let slot = self.gate.expect(layer, Phase::PrefixInFlight)?;
        let observed = (|| {
            let words = [
                group.observe_wave_output_state_v5(&self.states[slot].prefix[0])?,
                group.observe_wave_output_state_v5(&self.states[slot].prefix[1])?,
            ];
            if !words.iter().all(prefix_terminal) {
                return Err("tiles prefix incomplete".into());
            }
            Ok(words)
        })();
        let words = self.gate.outcome(observed)?;
        self.gate.phase = Phase::FirstResidualReady;
        Ok(words)
    }
    pub(crate) fn begin_residual(&mut self, layer: usize, first: bool) -> Result<()> {
        let (from, to) = if first {
            (Phase::FirstResidualReady, Phase::FirstResidualInFlight)
        } else {
            (Phase::FinalResidualReady, Phase::FinalResidualInFlight)
        };
        self.gate.advance(layer, from, to).map(|_| ())
    }
    pub(crate) fn finish_residual(
        &mut self,
        layer: usize,
        first: bool,
        result: Result<()>,
    ) -> Result<()> {
        self.gate.outcome(result)?;
        self.gate.expect(
            layer,
            if first {
                Phase::FirstResidualInFlight
            } else {
                Phase::FinalResidualInFlight
            },
        )?;
        if first {
            self.gate.phase = Phase::MlpReady;
        } else {
            self.gate.layer += 1;
            self.gate.phase = if self.gate.layer == LAYERS {
                Phase::Tail
            } else {
                Phase::PrefixReady
            };
        }
        Ok(())
    }
    pub(crate) fn take_tiles(&mut self, layer: usize) -> Result<[&mut TilesState; 2]> {
        let slot = self
            .gate
            .advance(layer, Phase::MlpReady, Phase::MlpInFlight)?;
        let [left, right] = &mut self.states[slot].mlp;
        Ok([left, right])
    }
    pub(crate) fn finish_tiles(
        &mut self,
        group: &mut Group,
        layer: usize,
        result: Result<[[u32; WORDS]; 2]>,
    ) -> Result<[[u32; WORDS]; 2]> {
        let dispatched = self.gate.outcome(result)?;
        let slot = self.gate.expect(layer, Phase::MlpInFlight)?;
        let observed = (|| {
            let words = [
                group.observe_wave_mlp_tiles_state_v2(&self.states[slot].mlp[0])?,
                group.observe_wave_mlp_tiles_state_v2(&self.states[slot].mlp[1])?,
            ];
            if words != dispatched || !words.iter().all(terminal) {
                return Err("tiles paired completion/terminal state mismatch".into());
            }
            Ok(words)
        })();
        let words = self.gate.outcome(observed)?;
        self.gate.phase = Phase::FinalResidualReady;
        Ok(words)
    }
    pub(crate) fn commit(&mut self, group: &mut Group, generation: u64) -> Result<()> {
        let result = (|| {
            self.gate.commit()?;
            self.ledger.commit(generation, idle(group))
        })();
        if result.is_err() {
            self.poison();
        }
        result
    }
    pub(crate) fn completed(&self) -> u64 {
        self.ledger.completed
    }
    pub(crate) fn between(&self) -> bool {
        self.ledger.between()
    }
    pub(crate) fn poison(&mut self) {
        self.gate.phase = Phase::Terminal;
        self.ledger.poison();
    }
}

mod reuse;
#[cfg(test)]
mod tests;
