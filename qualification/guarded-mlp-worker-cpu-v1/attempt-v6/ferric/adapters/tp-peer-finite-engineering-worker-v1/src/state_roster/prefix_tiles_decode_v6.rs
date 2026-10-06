//! Distinct four-forward, all-layer Prefix284 + MLP548 state ownership.
use super::{Gate, Group, LAYERS, Phase, ReservationIdentity, Result, WorkerKind};
use fe2o3_kfd::{
    Gfx950EngineeringPeerWaveMlpTilesStateV2 as Mlp,
    Gfx950EngineeringPeerWaveQkvAttentionOutputTilesStateV6 as Prefix,
};

pub(crate) const COUNTS: [usize; 2] = [714, 710];
pub(crate) const ORDERED_COUNTS: [usize; 2] = [715, 711];
const BANKS: usize = 2;
const FORWARDS: u64 = 4;
struct Layer<P = Prefix, M = Mlp> {
    prefix: [P; 2],
    mlp: [M; 2],
}
trait Allocator {
    type Prefix;
    type Mlp;
    fn preflight(&mut self, counts: &[usize]) -> Result<Vec<usize>>;
    fn prefix(&mut self, rank: usize) -> Result<Self::Prefix>;
    fn mlp(&mut self, rank: usize) -> Result<Self::Mlp>;
}
impl Allocator for Group {
    type Prefix = Prefix;
    type Mlp = Mlp;
    fn preflight(&mut self, counts: &[usize]) -> Result<Vec<usize>> {
        self.preflight_additional_allocations_v1(counts)
    }
    fn prefix(&mut self, rank: usize) -> Result<Prefix> {
        self.allocate_wave_qkv_attention_output_tiles_state_v6(rank)
    }
    fn mlp(&mut self, rank: usize) -> Result<Mlp> {
        self.allocate_wave_mlp_tiles_state_v2(rank)
    }
}
fn allocate<A: Allocator>(
    a: &mut A,
    model: [u8; 32],
) -> Result<(Vec<Layer<A::Prefix, A::Mlp>>, Vec<ReservationIdentity>)> {
    if model == [0; 32] || a.preflight(&[144, 144])? != [570, 566] {
        return Err("prefix decode source allocation census/model".into());
    }
    let mut states = Vec::with_capacity(BANKS * LAYERS);
    let mut ids = Vec::with_capacity(288);
    for bank in 0..BANKS {
        for layer in 0..LAYERS {
            let prefix = [a.prefix(0)?, a.prefix(1)?];
            let mlp = [a.mlp(0)?, a.mlp(1)?];
            for kind in [WorkerKind::PrefixTilesV6, WorkerKind::MlpTilesV2] {
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
    if a.preflight(&[0, 0])? != COUNTS {
        return Err("prefix decode final allocation census".into());
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
        if self.states.len() != BANKS * LAYERS || self.ids.len() != 288 {
            return Err("prefix decode sealed state cardinality".into());
        }
        Ok(Roster {
            states: self.states,
            expected_counts: COUNTS,
            gate: Gate::new(),
            ledger: reuse::Ledger::new(registration, self.model)?,
        })
    }
}
fn prefix_initial() -> [u32; 284] {
    core::array::from_fn(|i| u32::from(i == 0 || i == 2))
}
fn mlp_initial() -> [u32; 548] {
    core::array::from_fn(|i| u32::from(i == 0 || i == 2))
}
fn prefix_terminal(words: &[u32; 284]) -> bool {
    super::prefix_tiles_v6::terminal(words)
}
fn mlp_terminal(words: &[u32; 548]) -> bool {
    super::tiles_decode_v1::terminal(words)
}
fn idle(group: &mut Group, counts: [usize; 2]) -> Result<()> {
    if group.preflight_additional_allocations_v1(&[0, 0])? != counts {
        return Err("prefix decode allocation census changed".into());
    }
    Ok(())
}
pub(crate) struct Roster {
    expected_counts: [usize; 2],
    states: Vec<Layer>,
    gate: Gate,
    ledger: reuse::Ledger,
}
impl Roster {
    pub(crate) fn bind_ordered_buffers(&mut self, group: &mut Group) -> Result<()> {
        let result = (|| {
            if self.expected_counts != COUNTS
                || self.gate.phase != Phase::BetweenForwards
                || self.ledger.completed != 0
                || !self.ledger.between()
            {
                return Err("ordered scratch requires pristine roster".into());
            }
            idle(group, ORDERED_COUNTS)?;
            self.expected_counts = ORDERED_COUNTS;
            Ok(())
        })();
        self.outcome(result)
    }
    pub(crate) fn counts(&self) -> [usize; 2] {
        self.expected_counts
    }
    pub(crate) fn take_residual_mlp(&mut self, layer: usize) -> Result<[&mut Mlp; 2]> {
        let result = if self.expected_counts == ORDERED_COUNTS {
            self.gate
                .advance(layer, Phase::FirstResidualReady, Phase::ResidualMlpInFlight)
        } else {
            Err("compound route requires distinct Down scratch".into())
        };
        let slot = self.outcome(result)?;
        let [left, right] = &mut self.states[slot].mlp;
        Ok([left, right])
    }
    pub(crate) fn finish_residual_mlp(
        &mut self,
        group: &mut Group,
        layer: usize,
        result: Result<[[u32; 548]; 2]>,
    ) -> Result<[[u32; 548]; 2]> {
        let result = (|| {
            let dispatched = result?;
            let slot = self.gate.expect(layer, Phase::ResidualMlpInFlight)?;
            let words = [
                group.observe_wave_mlp_tiles_state_v2(&self.states[slot].mlp[0])?,
                group.observe_wave_mlp_tiles_state_v2(&self.states[slot].mlp[1])?,
            ];
            if words != dispatched || !words.iter().all(mlp_terminal) {
                return Err("ordered paired548 terminal mismatch".into());
            }
            self.gate.phase = Phase::FinalResidualReady;
            Ok(words)
        })();
        self.outcome(result)
    }
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
                return Err("prefix decode previous layer/forward active".into());
            }
            let bank = self.ledger.prepare(
                &mut reuse::Native {
                    group,
                    counts: self.expected_counts,
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
        self.outcome(result)
    }
    fn outcome<T>(&mut self, value: Result<T>) -> Result<T> {
        if value.is_err() {
            self.poison();
        }
        value
    }
    pub(crate) fn take_prefix(&mut self, layer: usize) -> Result<[&mut Prefix; 2]> {
        let result = self
            .gate
            .advance(layer, Phase::PrefixReady, Phase::PrefixInFlight);
        let slot = self.outcome(result)?;
        let [left, right] = &mut self.states[slot].prefix;
        Ok([left, right])
    }
    pub(crate) fn finish_prefix(
        &mut self,
        group: &mut Group,
        layer: usize,
        result: Result<[[u32; 284]; 2]>,
    ) -> Result<[[u32; 284]; 2]> {
        let result = (|| {
            let dispatched = result?;
            let slot = self.gate.expect(layer, Phase::PrefixInFlight)?;
            let words = [
                group.observe_wave_qkv_attention_output_tiles_state_v6(
                    &self.states[slot].prefix[0],
                )?,
                group.observe_wave_qkv_attention_output_tiles_state_v6(
                    &self.states[slot].prefix[1],
                )?,
            ];
            if words != dispatched || !words.iter().all(prefix_terminal) {
                return Err("prefix decode paired284 terminal mismatch".into());
            }
            self.gate.phase = Phase::FirstResidualReady;
            Ok(words)
        })();
        self.outcome(result)
    }
    pub(crate) fn begin_residual(&mut self, layer: usize, first: bool) -> Result<()> {
        let (from, to) = if first {
            (Phase::FirstResidualReady, Phase::FirstResidualInFlight)
        } else {
            (Phase::FinalResidualReady, Phase::FinalResidualInFlight)
        };
        let result = self.gate.advance(layer, from, to).map(|_| ());
        self.outcome(result)
    }
    pub(crate) fn finish_residual(
        &mut self,
        layer: usize,
        first: bool,
        result: Result<()>,
    ) -> Result<()> {
        let result = (|| {
            result?;
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
        })();
        self.outcome(result)
    }
    pub(crate) fn take_mlp(&mut self, layer: usize) -> Result<[&mut Mlp; 2]> {
        let result = self
            .gate
            .advance(layer, Phase::MlpReady, Phase::MlpInFlight);
        let slot = self.outcome(result)?;
        let [left, right] = &mut self.states[slot].mlp;
        Ok([left, right])
    }
    pub(crate) fn finish_mlp(
        &mut self,
        group: &mut Group,
        layer: usize,
        result: Result<[[u32; 548]; 2]>,
    ) -> Result<[[u32; 548]; 2]> {
        let result = (|| {
            let dispatched = result?;
            let slot = self.gate.expect(layer, Phase::MlpInFlight)?;
            let words = [
                group.observe_wave_mlp_tiles_state_v2(&self.states[slot].mlp[0])?,
                group.observe_wave_mlp_tiles_state_v2(&self.states[slot].mlp[1])?,
            ];
            if words != dispatched || !words.iter().all(mlp_terminal) {
                return Err("prefix decode paired548 terminal mismatch".into());
            }
            self.gate.phase = Phase::FinalResidualReady;
            Ok(words)
        })();
        self.outcome(result)
    }
    pub(crate) fn commit(&mut self, group: &mut Group, generation: u64) -> Result<()> {
        let result = (|| {
            self.gate.commit()?;
            self.ledger
                .commit(generation, idle(group, self.expected_counts))
        })();
        self.outcome(result)
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

#[cfg(test)]
mod ordered_tests {
    use super::*;
    #[test]
    fn ordered_phase_cannot_skip_prefix_or_run_separate_mlp() {
        let mut gate = Gate::new();
        assert!(
            gate.advance(0, Phase::FirstResidualReady, Phase::ResidualMlpInFlight)
                .is_err()
        );
        assert_eq!(gate.phase, Phase::Terminal);
        let mut gate = Gate::new();
        gate.begin(1, 0).unwrap();
        gate.advance(0, Phase::PrefixReady, Phase::PrefixInFlight)
            .unwrap();
        gate.phase = Phase::FirstResidualReady;
        gate.advance(0, Phase::FirstResidualReady, Phase::ResidualMlpInFlight)
            .unwrap();
        assert!(
            gate.advance(0, Phase::MlpReady, Phase::MlpInFlight)
                .is_err()
        );
        assert!(
            gate.advance(0, Phase::FinalResidualReady, Phase::FinalResidualInFlight)
                .is_err()
        );
    }
    #[test]
    fn ordered_phase_requires_compound_terminal_before_final_and_next_layer() {
        let mut gate = Gate::new();
        gate.begin(1, 0).unwrap();
        gate.phase = Phase::ResidualMlpInFlight;
        assert!(gate.commit().is_err());
        assert_eq!(gate.phase, Phase::Terminal);
        let mut gate = Gate::new();
        gate.begin(1, 0).unwrap();
        gate.phase = Phase::FinalResidualReady;
        gate.advance(0, Phase::FinalResidualReady, Phase::FinalResidualInFlight)
            .unwrap();
        assert!(
            gate.advance(1, Phase::PrefixReady, Phase::PrefixInFlight)
                .is_err()
        );
        assert_ne!(COUNTS, ORDERED_COUNTS);
        assert_eq!(ORDERED_COUNTS, [COUNTS[0] + 1, COUNTS[1] + 1]);
    }
}
