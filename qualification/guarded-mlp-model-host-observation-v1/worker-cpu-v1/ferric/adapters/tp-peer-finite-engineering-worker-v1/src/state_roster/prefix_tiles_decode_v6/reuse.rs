use super::{
    BANKS, FORWARDS, Group, LAYERS, Layer, Result, idle, mlp_initial, mlp_terminal, prefix_initial,
    prefix_terminal,
};
use fe2o3_kfd::{
    Gfx950EngineeringPeerStateBankEntryV1 as BankEntry,
    Gfx950EngineeringPeerStateBankSnapshotV1 as BankValue,
};

pub(super) struct Snapshot {
    pub(super) prefix: [[u32; 284]; 2],
    pub(super) mlp: [[u32; 548]; 2],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Phase {
    Between,
    Rearming,
    Active(u64),
    Exhausted,
    Terminal,
}
pub(super) struct Ledger {
    registration: [u8; 32],
    model: [u8; 32],
    pub(super) completed: u64,
    banks: [u64; BANKS],
    phase: Phase,
}
pub(super) trait Backend {
    fn fence(&mut self) -> Result<()>;
    fn snapshot_bank(&mut self, bank: usize) -> Result<Vec<Snapshot>>;
    fn reset_prefix(
        &mut self,
        bank: usize,
        layer: usize,
        rank: usize,
        expected: &[u32; 284],
    ) -> Result<()>;
    fn reset_mlp(
        &mut self,
        bank: usize,
        layer: usize,
        rank: usize,
        expected: &[u32; 548],
    ) -> Result<()>;
}
impl Ledger {
    pub(super) fn new(registration: [u8; 32], model: [u8; 32]) -> Result<Self> {
        if registration == [0; 32] || model == [0; 32] {
            return Err("prefix decode reuse identity".into());
        }
        Ok(Self {
            registration,
            model,
            completed: 0,
            banks: [0; BANKS],
            phase: Phase::Between,
        })
    }
    pub(super) fn between(&self) -> bool {
        matches!(self.phase, Phase::Between | Phase::Exhausted)
    }
    pub(super) fn poison(&mut self) {
        self.phase = Phase::Terminal;
    }
    pub(super) fn prepare(
        &mut self,
        b: &mut impl Backend,
        registration: [u8; 32],
        model: [u8; 32],
        generation: u64,
        position: u32,
    ) -> Result<usize> {
        let result = (|| {
            if self.phase != Phase::Between
                || registration != self.registration
                || model != self.model
                || self.completed >= FORWARDS
                || generation != self.completed + 1
                || u64::from(position) != self.completed
            {
                return Err("prefix decode reuse scope/order/exhaustion".into());
            }
            let bank = (self.completed % 2) as usize;
            let prior = generation.checked_sub(2).unwrap_or(0);
            if self.banks[bank] != prior {
                return Err("prefix decode bank generation".into());
            }
            self.phase = Phase::Rearming;
            b.fence()?;
            let snapshots = b.snapshot_bank(bank)?;
            if snapshots.len() != LAYERS {
                return Err("prefix decode incomplete bank snapshot".into());
            }
            for Snapshot { prefix: p, mlp: m } in &snapshots {
                if if prior == 0 {
                    p.iter().any(|x| *x != prefix_initial())
                        || m.iter().any(|x| *x != mlp_initial())
                } else {
                    !p.iter().all(prefix_terminal) || !m.iter().all(mlp_terminal)
                } {
                    return Err("prefix decode bank not completely fresh/terminal".into());
                }
            }
            // Acquire and validate all 144 states before the first rearm store.
            if prior != 0 {
                for (layer, Snapshot { prefix: p, mlp: m }) in snapshots.iter().enumerate() {
                    for rank in 0..2 {
                        b.reset_prefix(bank, layer, rank, &p[rank])?;
                        b.reset_mlp(bank, layer, rank, &m[rank])?;
                    }
                }
            }
            drop(snapshots);
            let readback = b.snapshot_bank(bank)?;
            if readback.len() != LAYERS {
                return Err("prefix decode incomplete bank readback".into());
            }
            for Snapshot { prefix: p, mlp: m } in &readback {
                if p.iter().any(|x| *x != prefix_initial()) || m.iter().any(|x| *x != mlp_initial())
                {
                    return Err("prefix decode rearm readback".into());
                }
            }
            b.fence()?;
            self.banks[bank] = generation;
            self.phase = Phase::Active(generation);
            Ok(bank)
        })();
        if result.is_err() {
            self.poison();
        }
        result
    }
    pub(super) fn commit(&mut self, generation: u64, tail_and_fence: Result<()>) -> Result<()> {
        let result = (|| {
            if self.phase != Phase::Active(generation) || generation != self.completed + 1 {
                return Err("prefix decode commit not active".into());
            }
            tail_and_fence?;
            self.completed = generation;
            self.phase = if generation == FORWARDS {
                Phase::Exhausted
            } else {
                Phase::Between
            };
            Ok(())
        })();
        if result.is_err() {
            self.poison();
        }
        result
    }
}
pub(super) struct Native<'a> {
    pub(super) counts: [usize; 2],
    pub(super) group: &'a mut Group,
    pub(super) states: &'a mut [Layer],
}

fn gather_bank<'a, P, M, T>(
    states: &'a [Layer<P, M>],
    bank: usize,
    mut prefix: impl FnMut(usize, &'a P) -> Result<T>,
    mut mlp: impl FnMut(usize, &'a M) -> Result<T>,
) -> Result<Vec<T>> {
    if bank >= BANKS || states.len() != BANKS * LAYERS {
        return Err("prefix decode bank snapshot bounds".into());
    }
    let mut entries = Vec::with_capacity(LAYERS * 4);
    for layer in &states[bank * LAYERS..(bank + 1) * LAYERS] {
        entries.push(prefix(0, &layer.prefix[0])?);
        entries.push(prefix(1, &layer.prefix[1])?);
        entries.push(mlp(0, &layer.mlp[0])?);
        entries.push(mlp(1, &layer.mlp[1])?);
    }
    Ok(entries)
}

fn decode_bank(values: Vec<BankValue>) -> Result<Vec<Snapshot>> {
    if values.len() != LAYERS * 4 {
        return Err("prefix decode typed bank snapshot cardinality".into());
    }
    let mut values = values.into_iter();
    let mut snapshots = Vec::with_capacity(LAYERS);
    for _ in 0..LAYERS {
        match (values.next(), values.next(), values.next(), values.next()) {
            (
                Some(BankValue::Prefix(left_p)),
                Some(BankValue::Prefix(right_p)),
                Some(BankValue::Mlp(left_m)),
                Some(BankValue::Mlp(right_m)),
            ) => snapshots.push(Snapshot {
                prefix: [left_p, right_p],
                mlp: [left_m, right_m],
            }),
            _ => return Err("prefix decode typed bank snapshot order".into()),
        }
    }
    Ok(snapshots)
}

impl Backend for Native<'_> {
    fn fence(&mut self) -> Result<()> {
        idle(self.group, self.counts)
    }
    fn snapshot_bank(&mut self, bank: usize) -> Result<Vec<Snapshot>> {
        let entries = gather_bank(
            self.states,
            bank,
            |rank, state| {
                if state.owner_rank() != rank {
                    return Err("prefix decode bank prefix rank order".into());
                }
                Ok(BankEntry::Prefix(state))
            },
            |rank, state| {
                if state.owner_rank() != rank {
                    return Err("prefix decode bank MLP rank order".into());
                }
                Ok(BankEntry::Mlp(state))
            },
        )?;
        // The runtime holds the exclusive group borrow through both fresh
        // fences and all Acquire loads; only the complete snapshot escapes.
        decode_bank(self.group.observe_state_bank_v1(&entries)?)
    }
    fn reset_prefix(&mut self, b: usize, l: usize, r: usize, expected: &[u32; 284]) -> Result<()> {
        // SAFETY: this private ledger has committed the prior generation, retired
        // its commands, fenced both participants and acquired the complete bank.
        unsafe {
            self.group.rearm_wave_qkv_attention_output_tiles_state_v6(
                &mut self.states[b * LAYERS + l].prefix[r],
                expected,
            )
        }
    }
    fn reset_mlp(&mut self, b: usize, l: usize, r: usize, expected: &[u32; 548]) -> Result<()> {
        // SAFETY: same whole-bank retirement boundary; distinct typed548 atomics.
        unsafe {
            self.group
                .rearm_wave_mlp_tiles_state_v2(&mut self.states[b * LAYERS + l].mlp[r], expected)
        }
    }
}

#[cfg(test)]
#[path = "bank_batch_tests.rs"]
mod bank_batch_tests;
