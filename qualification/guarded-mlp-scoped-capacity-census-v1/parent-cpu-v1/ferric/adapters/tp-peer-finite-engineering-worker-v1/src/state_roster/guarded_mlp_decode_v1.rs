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
pub(crate) const REUSE_MAX_COUNTS: [usize; 2] = [787, 783];
const SETUP_TIMEOUT_MS: u32 = 10_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ArenaMode {
    Fresh,
    ReuseRetired,
}

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
        unsafe { self.seal_with_mode(group, registration, inputs, timeout_ms, ArenaMode::Fresh) }
    }
    /// # Safety
    /// The seal contract and runtime's private reusable-arena obligations apply.
    pub(crate) unsafe fn seal_reusable(
        self,
        group: &mut Group,
        registration: [u8; 32],
        inputs: &[Inputs<'_>],
        timeout_ms: u32,
    ) -> Result<Roster> {
        unsafe {
            self.seal_with_mode(
                group,
                registration,
                inputs,
                timeout_ms,
                ArenaMode::ReuseRetired,
            )
        }
    }
    unsafe fn seal_with_mode(
        self,
        group: &mut Group,
        registration: [u8; 32],
        inputs: &[Inputs<'_>],
        timeout_ms: u32,
        mode: ArenaMode,
    ) -> Result<Roster> {
        let ledger = Ledger::with_mode(registration, self.model, mode)?;
        if group.preflight_additional_allocations_v1(&[0, 0])? != BOUND_COUNTS {
            return Err("guarded decode distinct Down allocation census".into());
        }
        let slots = consume_slots(self.slots, inputs, |storage, input| unsafe {
            match mode {
                ArenaMode::Fresh => group.bind_guarded_mlp_pair_exact_own_residual_unchecked_v1(
                    storage, input, timeout_ms,
                ),
                ArenaMode::ReuseRetired => group
                    .bind_guarded_mlp_pair_exact_own_residual_reusable_unchecked_v1(
                        storage, input, timeout_ms,
                    ),
            }
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
    ScopedLayerInFlight,
    Tail,
    Exhausted,
    Terminal,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Extent {
    Four,
    Readiness40,
    Full2303,
}
impl Extent {
    fn forwards(self) -> u64 {
        match self {
            Self::Four => 4,
            Self::Readiness40 => 40,
            Self::Full2303 => 2303,
        }
    }
}
struct Ledger {
    registration: [u8; 32],
    model: [u8; 32],
    completed: u64,
    bank: usize,
    generation: u64,
    layer: usize,
    dispatches: usize,
    mode: ArenaMode,
    arena_allocations: usize,
    last_generation: [u64; SLOTS],
    phase: Phase,
    extent: Extent,
    paired_terminal: bool,
    terminal_dispatches: [u32; 4],
}
impl Ledger {
    fn new(registration: [u8; 32], model: [u8; 32]) -> Result<Self> {
        Self::with_mode(registration, model, ArenaMode::Fresh)
    }
    fn with_mode(registration: [u8; 32], model: [u8; 32], mode: ArenaMode) -> Result<Self> {
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
            mode,
            arena_allocations: 0,
            last_generation: [0; SLOTS],
            phase: Phase::Between,
            extent: Extent::Four,
            paired_terminal: false,
            terminal_dispatches: [0; 4],
        })
    }
    fn select_paired_terminal(&mut self) -> Result<()> {
        let result = if !self.paired_terminal
            && self.extent == Extent::Four
            && self.mode == ArenaMode::ReuseRetired
            && self.phase == Phase::Between
            && self.completed == 0
            && self.bank == 0
            && self.generation == 0
            && self.layer == 0
            && self.dispatches == 0
            && self.arena_allocations == 0
            && self.last_generation == [0; SLOTS]
            && self.terminal_dispatches == [0; 4]
        {
            self.paired_terminal = true;
            Ok(())
        } else {
            Err("paired terminal requires pristine reusable AR4 custody".into())
        };
        self.checked(result)
    }
    fn paired_terminal_for(&self, index: usize) -> Result<bool> {
        let additional = self.arena_additional(index)?;
        if !self.paired_terminal {
            return Ok(false);
        }
        if self.extent != Extent::Four
            || self.mode != ArenaMode::ReuseRetired
            || self.completed >= 4
            || self.bank != self.completed as usize % 2
            || self.generation != self.completed / 2 + 1
        {
            return Err("paired terminal scope/bank custody".into());
        }
        match (self.generation, self.last_generation[index], additional) {
            (1, 0, 1) => Ok(false),
            (2, 1, 0) => Ok(true),
            _ => Err("paired terminal requires first use or actual retired bank generation".into()),
        }
    }
    fn paired_terminal_dispatches(&self) -> Result<Option<[u32; 4]>> {
        if !self.paired_terminal {
            return Ok(None);
        }
        if self.mode != ArenaMode::ReuseRetired
            || self.extent != Extent::Four
            || self.phase != Phase::Exhausted
            || self.completed != 4
            || self.dispatches != 144
            || self.arena_allocations != 72
            || self.last_generation != [2; SLOTS]
            || self.terminal_dispatches != [0, 0, 36, 36]
        {
            return Err("paired terminal incomplete warm-only Close census".into());
        }
        Ok(Some(self.terminal_dispatches))
    }
    fn select_readiness40(&mut self) -> Result<()> {
        let result = if !self.paired_terminal
            && self.extent == Extent::Four
            && self.mode == ArenaMode::ReuseRetired
            && self.phase == Phase::Between
            && self.completed == 0
            && self.bank == 0
            && self.generation == 0
            && self.layer == 0
            && self.dispatches == 0
            && self.arena_allocations == 0
            && self.last_generation == [0; SLOTS]
        {
            self.extent = Extent::Readiness40;
            Ok(())
        } else {
            Err("guarded readiness requires pristine reusable four-scope custody".into())
        };
        self.checked(result)
    }
    fn select_full2303(&mut self) -> Result<()> {
        let result = if !self.paired_terminal
            && self.extent == Extent::Four
            && self.mode == ArenaMode::ReuseRetired
            && self.phase == Phase::Between
            && self.completed == 0
            && self.bank == 0
            && self.generation == 0
            && self.layer == 0
            && self.dispatches == 0
            && self.arena_allocations == 0
            && self.last_generation == [0; SLOTS]
        {
            self.extent = Extent::Full2303;
            Ok(())
        } else {
            Err("guarded full2303 requires pristine reusable four-scope custody".into())
        };
        self.checked(result)
    }
    fn counts(&self) -> [usize; 2] {
        BOUND_COUNTS.map(|n| n + self.arena_allocations)
    }
    fn arena_additional(&self, index: usize) -> Result<usize> {
        if index >= SLOTS
            || index != self.bank * LAYERS + self.layer
            || self.last_generation[index].checked_add(1) != Some(self.generation)
        {
            return Err("guarded decode per-pair arena generation custody".into());
        }
        Ok(usize::from(
            self.mode == ArenaMode::Fresh || self.last_generation[index] == 0,
        ))
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
                || self.completed >= self.extent.forwards()
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
        let index = self.expect(layer, Phase::GuardedInFlight)?;
        let additional = self.arena_additional(index);
        let additional = self.checked(additional)?;
        let paired = self.paired_terminal_for(index);
        let paired = self.checked(paired)?;
        if paired {
            self.terminal_dispatches[self.completed as usize] += 1;
        }
        self.arena_allocations += additional;
        self.last_generation[index] = self.generation;
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
            self.phase = if self.completed == self.extent.forwards() {
                Phase::Exhausted
            } else {
                Phase::Between
            };
            Ok(())
        })();
        self.checked(result)
    }
}
fn dispatch_slot<P, C, O>(
    ledger: &mut Ledger,
    slots: &mut [Slot<P, C>],
    layer: usize,
    dispatch: impl FnOnce(&mut C, usize, [usize; 2], u64) -> Result<O>,
) -> Result<O> {
    dispatch_selected_slot(
        ledger,
        slots,
        layer,
        |pair, extra, before, generation, _| dispatch(pair, extra, before, generation),
    )
}
fn dispatch_selected_slot<P, C, O>(
    ledger: &mut Ledger,
    slots: &mut [Slot<P, C>],
    layer: usize,
    dispatch: impl FnOnce(&mut C, usize, [usize; 2], u64, bool) -> Result<O>,
) -> Result<O> {
    let result = (|| {
        if slots.len() != SLOTS {
            return Err("guarded decode retained slot census".into());
        }
        let index = ledger.expect(layer, Phase::Guarded)?;
        let additional = ledger.arena_additional(index)?;
        let paired_terminal = ledger.paired_terminal_for(index)?;
        let before = ledger.counts();
        ledger.phase = Phase::GuardedInFlight;
        let observation = dispatch(
            &mut slots[index].pair,
            additional,
            before,
            ledger.generation,
            paired_terminal,
        )?;
        ledger.guarded_complete(layer)?;
        Ok(observation)
    })();
    ledger.checked(result)
}
pub(crate) struct Roster {
    slots: Vec<Slot<Prefix, Pair>>,
    ledger: Ledger,
}
impl Roster {
    pub(crate) fn select_paired_terminal(&mut self) -> Result<()> {
        if self.slots.len() != SLOTS {
            self.poison();
            return Err("paired terminal requires all 72 genuine pairs".into());
        }
        self.ledger.select_paired_terminal()
    }
    pub(crate) fn paired_terminal_dispatches(&self) -> Result<Option<[u32; 4]>> {
        self.ledger.paired_terminal_dispatches()
    }
    pub(crate) fn select_readiness40(&mut self) -> Result<()> {
        if self.slots.len() != SLOTS {
            self.poison();
            return Err("guarded readiness requires all 72 genuine pairs".into());
        }
        self.ledger.select_readiness40()
    }
    pub(crate) fn readiness40(&self) -> bool {
        self.ledger.extent == Extent::Readiness40
    }
    pub(crate) fn select_full2303(&mut self) -> Result<()> {
        if self.slots.len() != SLOTS {
            self.poison();
            return Err("guarded full2303 requires all 72 genuine pairs".into());
        }
        self.ledger.select_full2303()
    }
    pub(crate) fn full2303(&self) -> bool {
        self.ledger.extent == Extent::Full2303
    }
    pub(crate) fn arena_mode(&self) -> ArenaMode {
        self.ledger.mode
    }
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
        dispatch_selected_slot(
            &mut self.ledger,
            &mut self.slots,
            layer,
            |pair, additional, before, generation, paired_terminal| {
                if group.preflight_additional_allocations_v1(&[additional, additional])? != before {
                    return Err("guarded decode pre-dispatch arena census".into());
                }
                let observation = if paired_terminal {
                    group.dispatch_guarded_mlp_pair_paired_terminal_v1(pair, inputs, timeout_ms)?
                } else {
                    group.dispatch_guarded_mlp_pair_v1(pair, inputs, timeout_ms)?
                };
                if observation.guards != [[generation as u32, 0, 1, 0]; 2]
                    || !observation
                        .prefixes
                        .iter()
                        .all(super::tiles_decode_v1::terminal)
                    || group.preflight_additional_allocations_v1(&[0, 0])?
                        != before.map(|v| v + additional)
                {
                    return Err("guarded decode completion/generation/arena census".into());
                }
                Ok(observation)
            },
        )
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

mod scoped_currentness_v1;
pub(crate) use scoped_currentness_v1::WarmCompletion;

mod bank_scoped_currentness_v1;
pub(crate) use bank_scoped_currentness_v1::BankCompletion;

#[path = "guarded_mlp_decode_v1/scoped_census_v1.rs"]
mod scoped_census_v1;
pub(crate) use scoped_census_v1::WarmCensusCompletion;
