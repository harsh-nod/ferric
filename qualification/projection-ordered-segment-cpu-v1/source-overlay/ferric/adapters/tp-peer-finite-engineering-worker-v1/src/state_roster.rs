//! Private worker-side state ownership for the fixed 8B/TP2/two-forward branch.
//! Not a production-admission type and not a replacement for prepared Machine.
//!
//! Integration prerequisite: the generic KFD allocation-count preflight.
//! Registration/dataflow validation belongs to the existing worker;
//! this module must remain private and must not receive arbitrary callbacks.

use fe2o3_kfd::{
    Gfx950EngineeringPeerGroupV1 as Group, Gfx950EngineeringPeerWaveMlpStateV1 as MlpState,
    Gfx950EngineeringPeerWaveOutputStateV5 as PrefixState,
};

type Result<T> = core::result::Result<T, String>;
const FORWARDS: usize = 2;
const LAYERS: usize = 36;
const RANKS: usize = 2;
const STATES_PER_RANK: usize = FORWARDS * LAYERS * 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum WorkerKind {
    PrefixV5,
    PrefixTilesV6,
    MlpV1,
    MlpTilesV2,
}

#[allow(unsafe_code)]
pub(crate) mod prefix_tiles_decode_v6;
pub(crate) mod prefix_tiles_v6;

/// Inert catalog identity. It contains neither a GPU address nor a buffer handle.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ReservationIdentity {
    pub(super) reservation_id: u64,
    pub(super) model: [u8; 32],
    pub(super) forward: u32,
    pub(super) layer: u32,
    pub(super) rank: u32,
    pub(super) kind: WorkerKind,
}

struct LayerStates<P = PrefixState, M = MlpState> {
    prefix: [P; RANKS],
    mlp: [M; RANKS],
}

// Private static test seam. Wire input cannot provide an implementation,
// callback, pointer, state token, or successful completion through this trait.
trait StateBackend {
    type Prefix;
    type Mlp;
    fn preflight(&mut self, additional: &[usize]) -> Result<Vec<usize>>;
    fn allocate_prefix(&mut self, rank: usize) -> Result<Self::Prefix>;
    fn allocate_mlp(&mut self, rank: usize) -> Result<Self::Mlp>;
    fn observe_prefix(&mut self, state: &Self::Prefix) -> Result<[u32; 22]>;
    fn observe_mlp(&mut self, state: &Self::Mlp) -> Result<[u32; 11]>;
}

impl StateBackend for Group {
    type Prefix = PrefixState;
    type Mlp = MlpState;
    fn preflight(&mut self, additional: &[usize]) -> Result<Vec<usize>> {
        self.preflight_additional_allocations_v1(additional)
    }
    fn allocate_prefix(&mut self, rank: usize) -> Result<PrefixState> {
        self.allocate_wave_output_state_v5(rank)
    }
    fn allocate_mlp(&mut self, rank: usize) -> Result<MlpState> {
        self.allocate_wave_mlp_state_v1(rank)
    }
    fn observe_prefix(&mut self, state: &PrefixState) -> Result<[u32; 22]> {
        self.observe_wave_output_state_v5(state)
    }
    fn observe_mlp(&mut self, state: &MlpState) -> Result<[u32; 11]> {
        self.observe_wave_mlp_state_v1(state)
    }
}

struct Reserved<P, M> {
    states: Vec<LayerStates<P, M>>,
    identities: Vec<ReservationIdentity>,
    occupied_before: [usize; RANKS],
}

fn reserve<B: StateBackend>(
    backend: &mut B,
    model: [u8; 32],
) -> Result<Reserved<B::Prefix, B::Mlp>> {
    if model == [0; 32] {
        return Err("missing exact model identity".into());
    }
    // Counts remain model-specific only here. The KFD observation reserves
    // nothing; actual typed allocation checks and failures remain authoritative.
    let occupied_before: [usize; RANKS] = backend
        .preflight(&[STATES_PER_RANK; RANKS])?
        .try_into()
        .map_err(|_| "allocation preflight returned wrong owner roster")?;
    let mut states = Vec::with_capacity(FORWARDS * LAYERS);
    let mut identities = Vec::with_capacity(RANKS * STATES_PER_RANK);
    for forward in 0..FORWARDS {
        for layer in 0..LAYERS {
            let prefix = [backend.allocate_prefix(0)?, backend.allocate_prefix(1)?];
            let mlp = [backend.allocate_mlp(0)?, backend.allocate_mlp(1)?];
            for kind in [WorkerKind::PrefixV5, WorkerKind::MlpV1] {
                for rank in 0..RANKS {
                    let reservation_id = u64::try_from(identities.len() + 1)
                        .map_err(|_| "reservation identity overflow")?;
                    identities.push(ReservationIdentity {
                        reservation_id,
                        model,
                        forward: forward as u32,
                        layer: layer as u32,
                        rank: rank as u32,
                        kind,
                    });
                }
            }
            states.push(LayerStates { prefix, mlp });
        }
    }
    Ok(Reserved {
        states,
        identities,
        occupied_before,
    })
}

/// This object is created during setup, before registration is sealed.
/// Allocation errors leave the Group quarantined by its existing API. The
/// enclosing Native must become Terminal even when a non-native check fails.
pub(super) struct PendingStates {
    model: [u8; 32],
    states: Vec<LayerStates>,
    identities: Vec<ReservationIdentity>,
    occupied_before: [usize; RANKS],
}

impl PendingStates {
    pub(super) fn allocate(group: &mut Group, model: [u8; 32]) -> Result<Self> {
        // Actual current Context/group counts, not a guessed model baseline.
        // No setup allocation/load/dispatch may interleave with this exclusive
        // owner borrow and the immediately following fixed reservation loop.
        let Reserved {
            states,
            identities,
            occupied_before,
        } = reserve(group, model)?;
        Ok(Self {
            model,
            states,
            identities,
            occupied_before,
        })
    }

    pub(super) fn catalog(&self) -> &[ReservationIdentity] {
        &self.identities
    }

    /// Call ONLY after exact finite-profile/dataflow/catalog validation, while
    /// Machine is Executing. A digest by itself is not registration authority.
    /// Its private scope intentionally prevents an external authority factory.
    pub(super) fn seal(self, registration: [u8; 32]) -> Result<StateRoster> {
        if registration == [0; 32]
            || self.states.len() != FORWARDS * LAYERS
            || self.identities.len() != RANKS * STATES_PER_RANK
        {
            return Err("invalid sealed state roster".into());
        }
        Ok(StateRoster {
            registration,
            model: self.model,
            states: self.states,
            identities: self.identities,
            occupied_before: self.occupied_before,
            gate: Gate::new(),
        })
    }
}

/// Retains every actual typed token until the original Group closes. No Clone,
/// ordinary-buffer conversion, reset, state upload, release, or public group().
pub(super) struct StateRoster {
    registration: [u8; 32],
    model: [u8; 32],
    states: Vec<LayerStates>,
    identities: Vec<ReservationIdentity>,
    occupied_before: [usize; RANKS],
    gate: Gate,
}

impl StateRoster {
    pub(super) fn begin_forward(
        &mut self,
        registration: [u8; 32],
        model: [u8; 32],
        generation: u64,
        position: u32,
    ) -> Result<()> {
        if self.registration != registration || self.model != model {
            return self.gate.reject("foreign registration/model");
        }
        self.gate.begin(generation, position)
    }

    /// Obtain state pointers only from this roster, never from wire pointer IDs.
    /// The native call site builds the exact validated rank-local root lists.
    pub(super) fn take_prefix(&mut self, layer: usize) -> Result<[&PrefixState; RANKS]> {
        let slot = self
            .gate
            .advance(layer, Phase::PrefixReady, Phase::PrefixInFlight)?;
        Ok([&self.states[slot].prefix[0], &self.states[slot].prefix[1]])
    }

    /// Must follow successful actual paired dispatch/queue completion. These
    /// acquire observations add semantic graph completion, which error0 alone
    /// does not imply. Returned snapshots are engineering observations only.
    pub(super) fn observe_prefix(
        &mut self,
        group: &mut Group,
        layer: usize,
        completion: Result<()>,
    ) -> Result<[[u32; 22]; RANKS]> {
        self.gate.outcome(completion)?;
        let slot = self.gate.expect(layer, Phase::PrefixInFlight)?;
        observe_prefix_pair(&mut self.gate, group, &self.states[slot].prefix)
    }

    pub(super) fn begin_first_residual(&mut self, layer: usize) -> Result<()> {
        self.gate
            .advance(
                layer,
                Phase::FirstResidualReady,
                Phase::FirstResidualInFlight,
            )
            .map(|_| ())
    }

    /// Private native call site invokes this only after the original resident
    /// first-residual paired dispatch has returned both acquired completions.
    pub(super) fn finish_first_residual(
        &mut self,
        layer: usize,
        completion: Result<()>,
    ) -> Result<()> {
        self.gate.outcome(completion)?;
        self.gate
            .advance(layer, Phase::FirstResidualInFlight, Phase::MlpReady)
            .map(|_| ())
    }

    pub(super) fn take_mlp(&mut self, layer: usize) -> Result<[&MlpState; RANKS]> {
        let slot = self
            .gate
            .advance(layer, Phase::MlpReady, Phase::MlpInFlight)?;
        Ok([&self.states[slot].mlp[0], &self.states[slot].mlp[1]])
    }

    pub(super) fn observe_mlp(
        &mut self,
        group: &mut Group,
        layer: usize,
        completion: Result<()>,
    ) -> Result<[[u32; 11]; RANKS]> {
        self.gate.outcome(completion)?;
        let slot = self.gate.expect(layer, Phase::MlpInFlight)?;
        observe_mlp_pair(&mut self.gate, group, &self.states[slot].mlp)
    }

    pub(super) fn begin_final_residual(&mut self, layer: usize) -> Result<()> {
        self.gate
            .advance(
                layer,
                Phase::FinalResidualReady,
                Phase::FinalResidualInFlight,
            )
            .map(|_| ())
    }

    pub(super) fn finish_final_residual(
        &mut self,
        layer: usize,
        completion: Result<()>,
    ) -> Result<()> {
        self.gate.outcome(completion)?;
        self.gate.expect(layer, Phase::FinalResidualInFlight)?;
        self.gate.layer += 1;
        self.gate.phase = if self.gate.layer == LAYERS {
            Phase::Tail
        } else {
            Phase::PrefixReady
        };
        Ok(())
    }

    /// Only after the exact existing finalnorm/head/argmax tail, bounded token
    /// check, actual receipt verification, and idle/currentness fence succeeded.
    /// The outer prepared Machine commits its cursor/pages/token afterward.
    pub(super) fn commit_forward(&mut self, validated_tail: Result<()>) -> Result<()> {
        self.gate.outcome(validated_tail)?;
        self.gate.commit()
    }

    pub(super) fn exhausted(&self) -> bool {
        self.gate.phase == Phase::Exhausted
    }

    pub(super) fn poison(&mut self) {
        self.gate.phase = Phase::Terminal;
    }
}

fn observe_prefix_pair<B: StateBackend>(
    gate: &mut Gate,
    backend: &mut B,
    states: &[B::Prefix; RANKS],
) -> Result<[[u32; 22]; RANKS]> {
    gate.expect(gate.layer, Phase::PrefixInFlight)?;
    let result = (|| {
        let words = [
            backend.observe_prefix(&states[0])?,
            backend.observe_prefix(&states[1])?,
        ];
        if !words.iter().all(prefix_terminal) {
            return Err("incomplete or erroneous prefix state".into());
        }
        Ok(words)
    })();
    let words = gate.outcome(result)?;
    gate.phase = Phase::FirstResidualReady;
    Ok(words)
}

fn observe_mlp_pair<B: StateBackend>(
    gate: &mut Gate,
    backend: &mut B,
    states: &[B::Mlp; RANKS],
) -> Result<[[u32; 11]; RANKS]> {
    gate.expect(gate.layer, Phase::MlpInFlight)?;
    let result = (|| {
        let words = [
            backend.observe_mlp(&states[0])?,
            backend.observe_mlp(&states[1])?,
        ];
        if !words.iter().all(mlp_terminal) {
            return Err("incomplete or erroneous MLP state".into());
        }
        Ok(words)
    })();
    let words = gate.outcome(result)?;
    gate.phase = Phase::FinalResidualReady;
    Ok(words)
}

fn prefix_terminal(words: &[u32; 22]) -> bool {
    words[..4] == [1, 0, 0xffff, 0xffff]
        && words[5] == 0
        && words[6..] == [64; 16]
        && (0..16).all(|task| matches!((words[4] >> (task * 2)) & 3, 1 | 2))
}

fn mlp_terminal(words: &[u32; 11]) -> bool {
    words[..4] == [1, 0, 31, 31]
        && words[5] == 0
        && words[6..] == [64; 5]
        && words[4] & !0x3ff == 0
        && (0..5).all(|task| matches!((words[4] >> (task * 2)) & 3, 1 | 2))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Phase {
    BetweenForwards,
    PrefixReady,
    PrefixInFlight,
    FirstResidualReady,
    FirstResidualInFlight,
    MlpReady,
    MlpInFlight,
    ResidualMlpInFlight,
    FinalResidualReady,
    FinalResidualInFlight,
    Tail,
    Exhausted,
    Terminal,
}

struct Gate {
    forward: usize,
    layer: usize,
    phase: Phase,
}

impl Gate {
    fn new() -> Self {
        Self {
            forward: 0,
            layer: 0,
            phase: Phase::BetweenForwards,
        }
    }

    fn reject<T>(&mut self, error: &str) -> Result<T> {
        self.phase = Phase::Terminal;
        Err(error.into())
    }

    fn outcome<T>(&mut self, result: Result<T>) -> Result<T> {
        if self.phase == Phase::Terminal {
            return Err("finite native owner is terminal".into());
        }
        match result {
            Ok(value) => Ok(value),
            Err(error) => {
                self.phase = Phase::Terminal;
                Err(error)
            }
        }
    }

    fn begin(&mut self, generation: u64, position: u32) -> Result<()> {
        if self.phase != Phase::BetweenForwards
            || self.forward >= FORWARDS
            || generation != self.forward as u64 + 1
            || position != self.forward as u32
        {
            return self.reject("finite forward identity/order/exhaustion");
        }
        self.layer = 0;
        self.phase = Phase::PrefixReady;
        Ok(())
    }

    fn expect(&mut self, layer: usize, phase: Phase) -> Result<usize> {
        if self.phase != phase || self.layer != layer || layer >= LAYERS || self.forward >= FORWARDS
        {
            return self.reject("finite layer/phase order or state reuse");
        }
        Ok(self.forward * LAYERS + layer)
    }

    fn advance(&mut self, layer: usize, from: Phase, to: Phase) -> Result<usize> {
        let slot = self.expect(layer, from)?;
        self.phase = to;
        Ok(slot)
    }

    fn commit(&mut self) -> Result<()> {
        if self.phase != Phase::Tail || self.layer != LAYERS {
            return self.reject("token commit before complete layer/tail boundary");
        }
        self.forward += 1;
        self.phase = if self.forward == FORWARDS {
            Phase::Exhausted
        } else {
            Phase::BetweenForwards
        };
        Ok(())
    }
}

#[cfg(test)]
#[path = "state_roster_tests.rs"]
mod tests;

// Separate opt-in owner; the existing two-forward roster is unchanged.
#[allow(unsafe_code)]
pub(crate) mod state_reuse_v1;

#[allow(unsafe_code)]
pub(crate) mod tiles_decode_v1;
