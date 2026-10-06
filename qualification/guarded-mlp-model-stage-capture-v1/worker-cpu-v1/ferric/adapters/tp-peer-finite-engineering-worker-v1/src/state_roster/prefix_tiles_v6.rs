//! One-shot layer0 state ownership. No old typed state or rearm conversion.
use super::{Group, ReservationIdentity, Result, WorkerKind};
use fe2o3_kfd::{
    Gfx950EngineeringPeerWaveMlpTilesStateV2 as Mlp,
    Gfx950EngineeringPeerWaveQkvAttentionOutputTilesStateV6 as Prefix,
};

pub(crate) const COUNTS: [usize; 2] = [714, 710];
struct Layer<P = Prefix, M = Mlp> {
    prefix: [P; 2],
    mlp: [M; 2],
}
trait Allocator {
    type Prefix;
    type Mlp;
    fn preflight(&mut self, n: &[usize]) -> Result<Vec<usize>>;
    fn prefix(&mut self, rank: usize) -> Result<Self::Prefix>;
    fn mlp(&mut self, rank: usize) -> Result<Self::Mlp>;
}
impl Allocator for Group {
    type Prefix = Prefix;
    type Mlp = Mlp;
    fn preflight(&mut self, n: &[usize]) -> Result<Vec<usize>> {
        self.preflight_additional_allocations_v1(n)
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
        return Err("prefix tiles exact model/source allocation census".into());
    }
    let mut states = Vec::with_capacity(72);
    let mut ids = Vec::with_capacity(288);
    for bank in 0..2 {
        for layer in 0..36 {
            let prefix = [a.prefix(0)?, a.prefix(1)?];
            let mlp = [a.mlp(0)?, a.mlp(1)?];
            for kind in [WorkerKind::PrefixTilesV6, WorkerKind::MlpTilesV2] {
                for rank in 0..2 {
                    ids.push(ReservationIdentity {
                        reservation_id: ids.len() as u64 + 1,
                        model,
                        forward: bank,
                        layer,
                        rank,
                        kind,
                    });
                }
            }
            states.push(Layer { prefix, mlp });
        }
    }
    if a.preflight(&[0, 0])? != COUNTS {
        return Err("prefix tiles final state allocation census".into());
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
            return Err("prefix tiles seal scope/cardinality".into());
        }
        Ok(Roster {
            registration,
            model: self.model,
            states: self.states,
            step: Step::Fresh,
        })
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Step {
    Fresh,
    Prefix,
    PrefixPending,
    FirstResidual,
    FirstPending,
    Mlp,
    MlpPending,
    LastResidual,
    LastPending,
    Complete,
    Terminal,
}
impl Step {
    fn advance(&mut self, expected: Self, next: Self) -> Result<()> {
        if *self != expected {
            *self = Self::Terminal;
            return Err("prefix layer phase or replay".into());
        }
        *self = next;
        Ok(())
    }
}
pub(crate) fn terminal(words: &[u32; 284]) -> bool {
    let counts = [1, 48, 1, 16, 64];
    let masks = [u32::MAX, u32::MAX, u32::MAX, u32::MAX, 3];
    words[..4] == [1, 0, 0, 31]
        && words[4..9] == counts
        && words[9..14] == counts
        && words[14..19] == masks
        && words[19..24] == masks
        && words[24..154].iter().all(|v| (1..=64).contains(v))
        && words[154..].iter().all(|v| *v == 64)
}
pub(crate) struct Roster {
    registration: [u8; 32],
    model: [u8; 32],
    states: Vec<Layer>,
    step: Step,
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
            if registration != self.registration
                || model != self.model
                || generation != 1
                || position != 0
                || group.preflight_additional_allocations_v1(&[0, 0])? != COUNTS
            {
                return Err("prefix one-layer scope/generation/census".into());
            }
            self.step.advance(Step::Fresh, Step::Prefix)
        })();
        self.outcome(result)
    }
    fn outcome<T>(&mut self, value: Result<T>) -> Result<T> {
        if value.is_err() {
            self.poison();
        }
        value
    }
    pub(crate) fn take_prefix(&mut self) -> Result<[&mut Prefix; 2]> {
        self.step.advance(Step::Prefix, Step::PrefixPending)?;
        let [left, right] = &mut self.states[0].prefix;
        Ok([left, right])
    }
    pub(crate) fn finish_prefix(
        &mut self,
        group: &mut Group,
        result: Result<[[u32; 284]; 2]>,
    ) -> Result<[[u32; 284]; 2]> {
        let result = (|| {
            let dispatched = result?;
            let actual = [
                group
                    .observe_wave_qkv_attention_output_tiles_state_v6(&self.states[0].prefix[0])?,
                group
                    .observe_wave_qkv_attention_output_tiles_state_v6(&self.states[0].prefix[1])?,
            ];
            if actual != dispatched || !actual.iter().all(terminal) {
                return Err("prefix paired terminal mismatch".into());
            }
            self.step
                .advance(Step::PrefixPending, Step::FirstResidual)?;
            Ok(actual)
        })();
        self.outcome(result)
    }
    pub(crate) fn begin_residual(&mut self, first: bool) -> Result<()> {
        self.step.advance(
            if first {
                Step::FirstResidual
            } else {
                Step::LastResidual
            },
            if first {
                Step::FirstPending
            } else {
                Step::LastPending
            },
        )
    }
    pub(crate) fn finish_residual(&mut self, first: bool, result: Result<()>) -> Result<()> {
        let result = result.and_then(|_| {
            self.step.advance(
                if first {
                    Step::FirstPending
                } else {
                    Step::LastPending
                },
                if first { Step::Mlp } else { Step::Complete },
            )
        });
        self.outcome(result)
    }
    pub(crate) fn take_mlp(&mut self) -> Result<[&mut Mlp; 2]> {
        self.step.advance(Step::Mlp, Step::MlpPending)?;
        let [left, right] = &mut self.states[0].mlp;
        Ok([left, right])
    }
    pub(crate) fn finish_mlp(
        &mut self,
        group: &mut Group,
        result: Result<[[u32; 548]; 2]>,
    ) -> Result<[[u32; 548]; 2]> {
        let result = (|| {
            let dispatched = result?;
            let actual = [
                group.observe_wave_mlp_tiles_state_v2(&self.states[0].mlp[0])?,
                group.observe_wave_mlp_tiles_state_v2(&self.states[0].mlp[1])?,
            ];
            if actual != dispatched || !actual.iter().all(super::tiles_decode_v1::terminal) {
                return Err("prefix layer MLP terminal mismatch".into());
            }
            self.step.advance(Step::MlpPending, Step::LastResidual)?;
            Ok(actual)
        })();
        self.outcome(result)
    }
    pub(crate) fn complete(&self) -> bool {
        self.step == Step::Complete
    }
    pub(crate) fn poison(&mut self) {
        self.step = Step::Terminal;
    }
}

#[cfg(test)]
mod tests;
