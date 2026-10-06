//! Actual all-layer V2 substitution. No V1 MLP dispatch or comparison poisoning.
use super::mlp_tiles_v2::artifacts::LoadedKernels;
use super::{
    Access, Dispatch, Group, LayerBindings, LoadedResidentArtifacts, PREFIX_BYTES, ResidentKind,
    Result, consumer_bytes,
};
use crate::state_roster::tiles_decode_v1::{Roster, WORDS};
use fe2o3_kfd::Gfx950EngineeringPeerWaveMlpTilesDispatchV2 as TilesDispatch;

pub(crate) struct Completion {
    pub(crate) prefix_states: [[u32; 22]; 2],
    pub(crate) tiles_states: [[u32; WORDS]; 2],
    pub(crate) paired_ns: [[u64; 2]; 4],
}
trait Backend {
    fn validate(&mut self) -> Result<()>;
    fn prefix(&mut self) -> Result<([[u32; 22]; 2], [u64; 2])>;
    fn residual(&mut self, first: bool) -> Result<[u64; 2]>;
    fn tiles(&mut self) -> Result<([[u32; WORDS]; 2], [u64; 2])>;
    fn poison(&mut self);
}
fn coordinate(b: &mut impl Backend) -> Result<Completion> {
    let result = (|| {
        b.validate()?;
        let (prefix_states, prefix) = b.prefix()?;
        let first = b.residual(true)?;
        let (tiles_states, tiles) = b.tiles()?;
        let last = b.residual(false)?;
        Ok(Completion {
            prefix_states,
            tiles_states,
            paired_ns: [prefix, first, tiles, last],
        })
    })();
    if result.is_err() {
        b.poison();
    }
    result
}
struct Native<'a> {
    group: &'a mut Group,
    states: &'a mut Roster,
    artifacts: &'a LoadedResidentArtifacts,
    tiles: &'a LoadedKernels,
    roots: &'a LayerBindings,
    layer: usize,
    timeout_ms: u32,
}
impl Backend for Native<'_> {
    fn validate(&mut self) -> Result<()> {
        if self.layer >= 36 || !(1..=10_000).contains(&self.timeout_ms) {
            return Err("tiles layer or deadline".into());
        }
        self.roots.validate()?;
        for rank in 0..2 {
            for kind in [ResidentKind::Prefix, ResidentKind::Residual] {
                if self.artifacts.kernel(rank, kind)?.rank() != rank {
                    return Err("tiles prefix/residual owner".into());
                }
            }
            if self.tiles.kernels[rank].rank() != rank {
                return Err("tiles MLP image owner".into());
            }
        }
        Ok(())
    }
    fn prefix(&mut self) -> Result<([[u32; 22]; 2], [u64; 2])> {
        let completion: Result<[u64; 2]> = (|| {
            let states = self.states.take_prefix(self.layer)?;
            let mut commands = Vec::with_capacity(2);
            for rank in 0..2 {
                if states[rank].owner_rank() != rank {
                    return Err("tiles prefix state rank".into());
                }
                let mut pointers = self.roots.prefix[rank]
                    .iter()
                    .zip(PREFIX_BYTES)
                    .enumerate()
                    .map(|(i, (root, bytes))| {
                        root.pointer(
                            i as u32 * 8,
                            0,
                            bytes,
                            if i < 7 {
                                Access::Read
                            } else {
                                Access::ReadWrite
                            },
                        )
                    })
                    .collect::<Vec<_>>();
                pointers.push(states[rank].pointer_v5());
                commands.push(Dispatch {
                    kernel: self.artifacts.kernel(rank, ResidentKind::Prefix)?,
                    bytes: vec![0; 376],
                    workgroup: [64, 1, 1],
                    grid: [128, 1, 1],
                    pointers,
                    timeout_ms: self.timeout_ms,
                });
            }
            // SAFETY: unchanged pinned prefix, catalog root order and typed22
            // state. The roster admits exactly this layer and physical bank.
            unsafe { self.group.dispatch_round_unchecked(commands)? }
                .try_into()
                .map_err(|_| "tiles prefix completion count".into())
        })();
        let words =
            self.states
                .finish_prefix(self.group, self.layer, completion.clone().map(|_| ()))?;
        Ok((words, completion?))
    }
    fn residual(&mut self, first: bool) -> Result<[u64; 2]> {
        self.states.begin_residual(self.layer, first)?;
        let completion: Result<[u64; 2]> = (|| {
            let mut commands = Vec::with_capacity(2);
            for rank in 0..2 {
                let mut pointers = (0..8)
                    .map(|slot| {
                        let peer = if slot < 2 { slot } else { 0 };
                        self.roots.prefix[peer][13].pointer(
                            slot as u32 * 16,
                            0,
                            if slot < 2 { 16384 } else { 0 },
                            Access::Read,
                        )
                    })
                    .collect::<Vec<_>>();
                let (original, output) = if first {
                    (self.roots.prefix[rank][0], self.roots.mlp[rank][0])
                } else {
                    (self.roots.mlp[rank][0], self.roots.final_hidden[rank])
                };
                pointers.push(original.pointer(128, 0, 8192, Access::Read));
                pointers.push(output.pointer(144, 0, 8192, Access::Write));
                commands.push(Dispatch {
                    kernel: self.artifacts.kernel(rank, ResidentKind::Residual)?,
                    bytes: consumer_bytes(),
                    workgroup: [64, 1, 1],
                    grid: [4096, 1, 1],
                    pointers,
                    timeout_ms: self.timeout_ms,
                });
            }
            // SAFETY: unchanged ordered TP2 residual; paired partials completed
            // and acquired before either rank may consume them.
            unsafe { self.group.dispatch_round_unchecked(commands)? }
                .try_into()
                .map_err(|_| "tiles residual completion count".into())
        })();
        self.states
            .finish_residual(self.layer, first, completion.clone().map(|_| ()))?;
        completion
    }
    fn tiles(&mut self) -> Result<([[u32; WORDS]; 2], [u64; 2])> {
        let completion = (|| {
            let [left, right] = self.states.take_tiles(self.layer)?;
            // SAFETY: same authenticated catalog roles; these are the only
            // typed548 states in this layer. The KFD pair consumes both Ready
            // activations before publishing and acquires both terminal states.
            unsafe {
                self.group.dispatch_wave_mlp_tiles_round_v2([
                    TilesDispatch {
                        kernel: &self.tiles.kernels[0],
                        object_sha256: self.tiles.sha256,
                        roots: self.roots.mlp[0],
                        state: left,
                        timeout_ms: self.timeout_ms,
                    },
                    TilesDispatch {
                        kernel: &self.tiles.kernels[1],
                        object_sha256: self.tiles.sha256,
                        roots: self.roots.mlp[1],
                        state: right,
                        timeout_ms: self.timeout_ms,
                    },
                ])
            }
        })();
        let words = self.states.finish_tiles(
            self.group,
            self.layer,
            completion
                .as_ref()
                .map(|r| r.final_states)
                .map_err(Clone::clone),
        )?;
        Ok((words, completion?.dispatch_elapsed_ns))
    }
    fn poison(&mut self) {
        self.states.poison();
    }
}
/// # Safety
/// The exclusive owner binds authenticated weights/catalog roles and separately
/// reviewed V2 image provenance. No command or pointer may escape this call.
pub(crate) unsafe fn execute(
    group: &mut Group,
    states: &mut Roster,
    artifacts: &LoadedResidentArtifacts,
    tiles: &LoadedKernels,
    roots: &LayerBindings,
    layer: usize,
    timeout_ms: u32,
) -> Result<Completion> {
    coordinate(&mut Native {
        group,
        states,
        artifacts,
        tiles,
        roots,
        layer,
        timeout_ms,
    })
}

#[cfg(test)]
mod tests;
