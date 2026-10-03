use super::{
    BANKS, FORWARDS, Group, LAYERS, Layer, Result, idle, mlp_initial, mlp_terminal, prefix_initial,
    prefix_terminal,
};

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
    fn prefix(&mut self, bank: usize, layer: usize, rank: usize) -> Result<[u32; 284]>;
    fn mlp(&mut self, bank: usize, layer: usize, rank: usize) -> Result<[u32; 548]>;
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
            let mut snapshots = Vec::with_capacity(LAYERS);
            for layer in 0..LAYERS {
                let p = [b.prefix(bank, layer, 0)?, b.prefix(bank, layer, 1)?];
                let m = [b.mlp(bank, layer, 0)?, b.mlp(bank, layer, 1)?];
                if if prior == 0 {
                    p.iter().any(|x| *x != prefix_initial())
                        || m.iter().any(|x| *x != mlp_initial())
                } else {
                    !p.iter().all(prefix_terminal) || !m.iter().all(mlp_terminal)
                } {
                    return Err("prefix decode bank not completely fresh/terminal".into());
                }
                snapshots.push((p, m));
            }
            // Acquire and validate all 144 states before the first rearm store.
            if prior != 0 {
                for (layer, (p, m)) in snapshots.iter().enumerate() {
                    for rank in 0..2 {
                        b.reset_prefix(bank, layer, rank, &p[rank])?;
                        b.reset_mlp(bank, layer, rank, &m[rank])?;
                    }
                }
            }
            for layer in 0..LAYERS {
                for rank in 0..2 {
                    if b.prefix(bank, layer, rank)? != prefix_initial()
                        || b.mlp(bank, layer, rank)? != mlp_initial()
                    {
                        return Err("prefix decode rearm readback".into());
                    }
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
    pub(super) group: &'a mut Group,
    pub(super) states: &'a mut [Layer],
}
impl Backend for Native<'_> {
    fn fence(&mut self) -> Result<()> {
        idle(self.group)
    }
    fn prefix(&mut self, b: usize, l: usize, r: usize) -> Result<[u32; 284]> {
        self.group.observe_wave_qkv_attention_output_tiles_state_v6(
            &self.states[b * LAYERS + l].prefix[r],
        )
    }
    fn mlp(&mut self, b: usize, l: usize, r: usize) -> Result<[u32; 548]> {
        self.group
            .observe_wave_mlp_tiles_state_v2(&self.states[b * LAYERS + l].mlp[r])
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
