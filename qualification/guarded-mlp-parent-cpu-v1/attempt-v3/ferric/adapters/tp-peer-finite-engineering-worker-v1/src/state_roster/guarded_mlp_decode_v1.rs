//! Two banks of genuine Prefix284 and opaque combined2208 paired owners.
use super::{Group, LAYERS, ReservationIdentity, Result, WorkerKind};
use fe2o3_kfd::{
    Gfx950EngineeringPeerGuardedMlpBankEntryV1 as BankEntry,
    Gfx950EngineeringPeerGuardedMlpInputsV1 as Inputs,
    Gfx950EngineeringPeerGuardedMlpObservationV1 as Observation,
    Gfx950EngineeringPeerRetainedGuardedMlpPairV1 as Pair,
    Gfx950EngineeringPeerUnboundGuardedMlpPairV1 as Unbound,
    Gfx950EngineeringPeerWaveQkvAttentionOutputTilesStateV6 as Prefix,
};

const BANKS: usize = 2;
const SLOTS: usize = BANKS * LAYERS;
pub(crate) const STATE_COUNTS: [usize; 2] = [714, 710];
pub(crate) const BOUND_COUNTS: [usize; 2] = [715, 711];
pub(crate) const MAX_COUNTS: [usize; 2] = [859, 855];
const SETUP_TIMEOUT_MS: u32 = 10_000;

struct Slot<P, C> {
    prefixes: [P; 2],
    pair: C,
}
trait Allocator {
    type Prefix;
    type Storage;
    fn counts(&mut self, extra: &[usize]) -> Result<Vec<usize>>;
    fn prefix(&mut self, rank: usize) -> Result<Self::Prefix>;
    fn storage(&mut self) -> Result<Self::Storage>;
}
impl Allocator for Group {
    type Prefix = Prefix;
    type Storage = Unbound;
    fn counts(&mut self, extra: &[usize]) -> Result<Vec<usize>> {
        self.preflight_additional_allocations_v1(extra)
    }
    fn prefix(&mut self, rank: usize) -> Result<Prefix> {
        self.allocate_wave_qkv_attention_output_tiles_state_v6(rank)
    }
    fn storage(&mut self) -> Result<Unbound> {
        self.allocate_guarded_mlp_pair_storage_v1(SETUP_TIMEOUT_MS)
    }
}
fn allocate<A: Allocator>(a: &mut A, model: [u8; 32]) -> Result<Vec<Slot<A::Prefix, A::Storage>>> {
    if model == [0; 32] || a.counts(&[144, 144])? != [570, 566] {
        return Err("guarded decode initial model/allocation census".into());
    }
    let mut slots = Vec::with_capacity(SLOTS);
    for _ in 0..SLOTS {
        slots.push(Slot {
            prefixes: [a.prefix(0)?, a.prefix(1)?],
            pair: a.storage()?,
        });
    }
    if a.counts(&[0, 0])? != STATE_COUNTS {
        return Err("guarded decode combined owner census".into());
    }
    Ok(slots)
}
fn identities(model: [u8; 32]) -> Vec<ReservationIdentity> {
    let mut ids = Vec::with_capacity(SLOTS * 4);
    for bank in 0..BANKS {
        for layer in 0..LAYERS {
            for kind in [WorkerKind::PrefixTilesV6, WorkerKind::GuardedMlpCombinedV1] {
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
        }
    }
    ids
}
fn consume_slots<P, U, B, I>(
    slots: Vec<Slot<P, U>>,
    inputs: &[I],
    mut bind: impl FnMut(U, &I) -> Result<B>,
) -> Result<Vec<Slot<P, B>>> {
    if slots.len() != SLOTS || inputs.len() != SLOTS {
        return Err("guarded decode requires all 72 pair bindings".into());
    }
    let mut bound = Vec::with_capacity(SLOTS);
    for (slot, input) in slots.into_iter().zip(inputs) {
        bound.push(Slot {
            prefixes: slot.prefixes,
            pair: bind(slot.pair, input)?,
        });
    }
    Ok(bound)
}
pub(crate) struct Pending {
    model: [u8; 32],
    slots: Vec<Slot<Prefix, Unbound>>,
    ids: Vec<ReservationIdentity>,
}
impl Pending {
    pub(crate) fn allocate(group: &mut Group, model: [u8; 32]) -> Result<Self> {
        Ok(Self {
            model,
            slots: allocate(group, model)?,
            ids: identities(model),
        })
    }
    pub(crate) fn catalog(&self) -> &[ReservationIdentity] {
        &self.ids
    }
    /// # Safety
    /// All inputs name the authenticated, retained model/image roles. No pair
    /// is published until all 72 consuming bindings have succeeded.
    pub(crate) unsafe fn seal(
        self,
        group: &mut Group,
        registration: [u8; 32],
        inputs: &[Inputs<'_>],
        timeout_ms: u32,
    ) -> Result<Roster> {
        let ledger = Ledger::new(registration, self.model)?;
        if group.preflight_additional_allocations_v1(&[0, 0])? != BOUND_COUNTS {
            return Err("guarded decode distinct Down allocation census".into());
        }
        let slots = consume_slots(self.slots, inputs, |storage, input| unsafe {
            group.bind_guarded_mlp_pair_exact_own_residual_unchecked_v1(storage, input, timeout_ms)
        })?;
        if group.preflight_additional_allocations_v1(&[0, 0])? != BOUND_COUNTS {
            return Err("guarded decode binding changed allocation census".into());
        }
        Ok(Roster { slots, ledger })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Between,
    Prefix,
    PrefixInFlight,
    Guarded,
    GuardedInFlight,
    Tail,
    Exhausted,
    Terminal,
}
struct Ledger {
    registration: [u8; 32],
    model: [u8; 32],
    completed: u64,
    bank: usize,
    generation: u64,
    layer: usize,
    dispatches: usize,
    phase: Phase,
}
impl Ledger {
    fn new(registration: [u8; 32], model: [u8; 32]) -> Result<Self> {
        if registration == [0; 32] || model == [0; 32] {
            return Err("guarded decode registration/model".into());
        }
        Ok(Self {
            registration,
            model,
            completed: 0,
            bank: 0,
            generation: 0,
            layer: 0,
            dispatches: 0,
            phase: Phase::Between,
        })
    }
    fn counts(&self) -> [usize; 2] {
        BOUND_COUNTS.map(|n| n + self.dispatches)
    }
    fn begin(
        &mut self,
        registration: [u8; 32],
        model: [u8; 32],
        forward: u64,
        position: u32,
        mut bank_action: impl FnMut(usize, bool) -> Result<u64>,
    ) -> Result<()> {
        let result = (|| {
            if self.phase != Phase::Between
                || self.completed >= 4
                || registration != self.registration
                || model != self.model
                || forward != self.completed + 1
                || u64::from(position) != self.completed
            {
                return Err("guarded decode forward scope/order".into());
            }
            let bank = (self.completed % 2) as usize;
            let expected = self.completed / 2 + 1;
            let actual = bank_action(bank, self.completed >= 2)?;
            if actual != expected {
                return Err("guarded decode bank-local generation".into());
            }
            self.bank = bank;
            self.generation = actual;
            self.layer = 0;
            self.phase = Phase::Prefix;
            Ok(())
        })();
        self.checked(result)
    }
    fn expect(&mut self, layer: usize, phase: Phase) -> Result<usize> {
        let result = if layer < LAYERS && self.layer == layer && self.phase == phase {
            Ok(self.bank * LAYERS + layer)
        } else {
            Err("guarded decode layer/phase".into())
        };
        self.checked(result)
    }
    fn checked<T>(&mut self, result: Result<T>) -> Result<T> {
        if result.is_err() {
            self.phase = Phase::Terminal;
        }
        result
    }
    fn guarded_complete(&mut self, layer: usize) -> Result<()> {
        self.expect(layer, Phase::GuardedInFlight)?;
        self.dispatches += 1;
        self.layer += 1;
        self.phase = if self.layer == LAYERS {
            Phase::Tail
        } else {
            Phase::Prefix
        };
        Ok(())
    }
    fn commit(&mut self, forward: u64, fence: Result<()>) -> Result<()> {
        let result = (|| {
            if self.phase != Phase::Tail || forward != self.completed + 1 {
                return Err("guarded decode incomplete forward".into());
            }
            fence?;
            self.completed += 1;
            self.phase = if self.completed == 4 {
                Phase::Exhausted
            } else {
                Phase::Between
            };
            Ok(())
        })();
        self.checked(result)
    }
}
pub(crate) struct Roster {
    slots: Vec<Slot<Prefix, Pair>>,
    ledger: Ledger,
}
impl Roster {
    pub(crate) fn counts(&self) -> [usize; 2] {
        self.ledger.counts()
    }
    pub(crate) fn completed(&self) -> u64 {
        self.ledger.completed
    }
    pub(crate) fn between(&self) -> bool {
        matches!(self.ledger.phase, Phase::Between | Phase::Exhausted)
    }
    pub(crate) fn poison(&mut self) {
        self.ledger.phase = Phase::Terminal;
    }
    pub(crate) fn begin(
        &mut self,
        group: &mut Group,
        registration: [u8; 32],
        model: [u8; 32],
        forward: u64,
        position: u32,
        timeout_ms: u32,
    ) -> Result<()> {
        let slots = &mut self.slots;
        self.ledger
            .begin(registration, model, forward, position, |bank, rearm| {
                let mut entries = Vec::with_capacity(LAYERS);
                for slot in &mut slots[bank * LAYERS..(bank + 1) * LAYERS] {
                    let [left, right] = &mut slot.prefixes;
                    entries.push(BankEntry {
                        prefixes: [left, right],
                        pair: &mut slot.pair,
                    });
                }
                if rearm {
                    // SAFETY: all36 layers and tail of this bank were committed;
                    // runtime rechecks the entire mixed bank before its first store.
                    unsafe { group.rearm_guarded_mlp_bank_unchecked_v1(&mut entries, timeout_ms) }
                } else {
                    group.validate_guarded_mlp_initial_bank_v1(&mut entries, timeout_ms)
                }
            })
    }
    pub(crate) fn take_prefix(&mut self, layer: usize) -> Result<[&mut Prefix; 2]> {
        let index = self.ledger.expect(layer, Phase::Prefix)?;
        self.ledger.phase = Phase::PrefixInFlight;
        let [left, right] = &mut self.slots[index].prefixes;
        Ok([left, right])
    }
    pub(crate) fn finish_prefix(
        &mut self,
        group: &mut Group,
        layer: usize,
        result: Result<[[u32; 284]; 2]>,
    ) -> Result<[[u32; 284]; 2]> {
        let result = (|| {
            let words = result?;
            let index = self.ledger.expect(layer, Phase::PrefixInFlight)?;
            let actual = [
                group.observe_wave_qkv_attention_output_tiles_state_v6(
                    &self.slots[index].prefixes[0],
                )?,
                group.observe_wave_qkv_attention_output_tiles_state_v6(
                    &self.slots[index].prefixes[1],
                )?,
            ];
            if words != actual || !words.iter().all(super::prefix_tiles_v6::terminal) {
                return Err("guarded decode Prefix284 completion".into());
            }
            self.ledger.phase = Phase::Guarded;
            Ok(words)
        })();
        self.ledger.checked(result)
    }
    pub(crate) fn dispatch(
        &mut self,
        group: &mut Group,
        layer: usize,
        inputs: Inputs<'_>,
        timeout_ms: u32,
    ) -> Result<Observation> {
        let result = (|| {
            let index = self.ledger.expect(layer, Phase::Guarded)?;
            let before = self.counts();
            if group.preflight_additional_allocations_v1(&[1, 1])? != before {
                return Err("guarded decode pre-dispatch arena census".into());
            }
            self.ledger.phase = Phase::GuardedInFlight;
            let observation = group.dispatch_guarded_mlp_pair_v1(
                &mut self.slots[index].pair,
                inputs,
                timeout_ms,
            )?;
            if observation.guards != [[self.ledger.generation as u32, 0, 1, 0]; 2]
                || !observation
                    .prefixes
                    .iter()
                    .all(super::tiles_decode_v1::terminal)
                || group.preflight_additional_allocations_v1(&[0, 0])? != before.map(|v| v + 1)
            {
                return Err("guarded decode completion/generation/arena census".into());
            }
            self.ledger.guarded_complete(layer)?;
            Ok(observation)
        })();
        self.ledger.checked(result)
    }
    pub(crate) fn commit(&mut self, group: &mut Group, forward: u64) -> Result<()> {
        let fence = group
            .preflight_additional_allocations_v1(&[0, 0])
            .and_then(|v| {
                if v == self.counts() {
                    Ok(())
                } else {
                    Err("guarded decode final allocation census".into())
                }
            });
        self.ledger.commit(forward, fence)
    }
}

#[cfg(test)]
mod tests;
