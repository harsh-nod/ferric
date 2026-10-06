use super::{BANKS, Group, LAYERS, Layer, Result, WORDS, idle, initial, prefix_terminal, terminal};

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
    fn prefix(&mut self, bank: usize, layer: usize, rank: usize) -> Result<[u32; 22]>;
    fn tiles(&mut self, bank: usize, layer: usize, rank: usize) -> Result<[u32; WORDS]>;
    fn reset_prefix(
        &mut self,
        bank: usize,
        layer: usize,
        rank: usize,
        expected: &[u32; 22],
    ) -> Result<()>;
    fn reset_tiles(
        &mut self,
        bank: usize,
        layer: usize,
        rank: usize,
        expected: &[u32; WORDS],
    ) -> Result<()>;
}
fn prefix_initial() -> [u32; 22] {
    core::array::from_fn(|i| u32::from(i < 2))
}
impl Ledger {
    pub(super) fn new(registration: [u8; 32], model: [u8; 32]) -> Result<Self> {
        if registration == [0; 32] || model == [0; 32] {
            return Err("tiles reuse identity".into());
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
        self.phase == Phase::Between || self.phase == Phase::Exhausted
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
            // This backend is deliberately bounded to four forwards. A long
            // transport is not admitted by merely changing this constant.
            if self.phase != Phase::Between
                || registration != self.registration
                || model != self.model
                || self.completed >= 4
                || generation != self.completed + 1
                || u64::from(position) != self.completed
            {
                return Err("tiles reuse scope/order/exhaustion".into());
            }
            let bank = (self.completed % 2) as usize;
            let prior = generation.checked_sub(2).unwrap_or(0);
            if self.banks[bank] != prior {
                return Err("tiles bank generation".into());
            }
            self.phase = Phase::Rearming;
            b.fence()?;
            let mut snapshots = Vec::with_capacity(LAYERS);
            for layer in 0..LAYERS {
                let p = [b.prefix(bank, layer, 0)?, b.prefix(bank, layer, 1)?];
                let m = [b.tiles(bank, layer, 0)?, b.tiles(bank, layer, 1)?];
                if if prior == 0 {
                    p.iter().any(|x| *x != prefix_initial()) || m.iter().any(|x| *x != initial())
                } else {
                    !p.iter().all(prefix_terminal) || !m.iter().all(terminal)
                } {
                    return Err("tiles bank not completely fresh/terminal".into());
                }
                snapshots.push((p, m));
            }
            // All 144 current snapshots have passed before the first store.
            if prior != 0 {
                for (layer, (p, m)) in snapshots.iter().enumerate() {
                    for rank in 0..2 {
                        b.reset_prefix(bank, layer, rank, &p[rank])?;
                        b.reset_tiles(bank, layer, rank, &m[rank])?;
                    }
                }
            }
            for layer in 0..LAYERS {
                for rank in 0..2 {
                    if b.prefix(bank, layer, rank)? != prefix_initial()
                        || b.tiles(bank, layer, rank)? != initial()
                    {
                        return Err("tiles rearm readback".into());
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
                return Err("tiles commit not active".into());
            }
            tail_and_fence?;
            self.completed = generation;
            self.phase = if generation == 4 {
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
    fn prefix(&mut self, b: usize, l: usize, r: usize) -> Result<[u32; 22]> {
        self.group
            .observe_wave_output_state_v5(&self.states[b * LAYERS + l].prefix[r])
    }
    fn tiles(&mut self, b: usize, l: usize, r: usize) -> Result<[u32; WORDS]> {
        self.group
            .observe_wave_mlp_tiles_state_v2(&self.states[b * LAYERS + l].mlp[r])
    }
    fn reset_prefix(&mut self, b: usize, l: usize, r: usize, expected: &[u32; 22]) -> Result<()> {
        // SAFETY: only the private ledger invokes this after an exact committed
        // generation and whole-bank acquire/idle check. No command escapes run.
        unsafe {
            self.group.rearm_wave_output_state_for_reuse_v1(
                &mut self.states[b * LAYERS + l].prefix[r],
                expected,
            )
        }
    }
    fn reset_tiles(&mut self, b: usize, l: usize, r: usize, expected: &[u32; WORDS]) -> Result<()> {
        // SAFETY: same retired-command boundary, using the distinct typed548
        // rearm API. It performs Release stores into existing AtomicU32 values.
        unsafe {
            self.group
                .rearm_wave_mlp_tiles_state_v2(&mut self.states[b * LAYERS + l].mlp[r], expected)
        }
    }
}
