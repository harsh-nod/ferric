//! Four completed paired phases over retained, rank-owned layer buffers.

use crate::{
    resident_artifacts::{LoadedResidentArtifacts, ResidentKind},
    state_roster::StateRoster,
};
use fe2o3_kfd::{
    Gfx950EngineeringPeerBufferV1 as Buffer, Gfx950EngineeringPeerDispatchV1 as Dispatch,
    Gfx950EngineeringPeerGroupV1 as Group, engineering_wire::BufferAccessV1 as Access,
};

type Result<T> = std::result::Result<T, String>;

pub(crate) mod capture_v1;
#[allow(unsafe_code)]
pub(crate) mod guarded_mlp_decode_v1;
pub(crate) mod mlp_tiles_v2;
pub(crate) mod prefix_tiles_decode_v6;
pub(crate) mod prefix_tiles_v6;
pub(crate) mod queued_mlp_v1;
pub(crate) mod queued_projection_v1;
pub(crate) mod tiles_decode_v1;
use capture_v1::{Boundary, Collector};

const PREFIX_BYTES: [u64; 14] = [
    8192, 8192, 25_165_824, 512, 512, 580, 16_777_216, 8192, 6144, 4096, 2_359_296, 2_359_296,
    4096, 16_384,
];
const MLP_BYTES: [u64; 10] = [
    8192, 8192, 50_331_648, 50_331_648, 50_331_648, 8192, 12_288, 12_288, 12_288, 16_384,
];

/// Only the private catalog constructs these from retained native tokens.
/// The input hidden buffer becomes the output only in the last residual phase.
pub(super) struct LayerBindings<B = Buffer> {
    pub(super) prefix: [[B; 14]; 2],
    pub(super) mlp: [[B; 10]; 2],
    pub(super) final_hidden: [B; 2],
}

pub(super) trait Allocation: Copy + Eq {
    fn owner(self) -> usize;
    fn bytes(self) -> u64;
}
impl Allocation for Buffer {
    fn owner(self) -> usize {
        self.owner_rank()
    }
    fn bytes(self) -> u64 {
        self.bytes()
    }
}

impl<B: Allocation> LayerBindings<B> {
    pub(super) fn validate(&self) -> Result<()> {
        for rank in 0..2 {
            let prefix = &self.prefix[rank];
            let mlp = &self.mlp[rank];
            for (roots, extents) in [
                (prefix.as_slice(), PREFIX_BYTES.as_slice()),
                (mlp.as_slice(), MLP_BYTES.as_slice()),
            ] {
                for (index, (root, bytes)) in roots.iter().zip(extents).enumerate() {
                    if root.owner() != rank
                        || root.bytes() < *bytes
                        || roots[..index].contains(root)
                    {
                        return Err("resident layer owner/extent/within-phase alias".into());
                    }
                }
            }
            if self.final_hidden[rank] != prefix[0]
                || mlp[5] != prefix[7]
                || mlp[9] != prefix[13]
                || prefix.contains(&mlp[0])
            {
                return Err("resident layer hidden/residual/scratch dataflow mismatch".into());
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(super) struct LayerCompletion {
    pub(super) prefix_states: [[u32; 22]; 2],
    pub(super) mlp_states: [[u32; 11]; 2],
    /// Existing queue host durations, not GPU overlap or per-task timings.
    pub(super) paired_ns: [[u64; 2]; 4],
}

/// Separate opt-in evidence. The eleven-word states belong to the finite
/// dispatch, not to the queued comparison that ran afterward.
#[derive(Debug)]
pub(super) struct ComparedLayerCompletion {
    pub(super) finite: LayerCompletion,
    pub(super) queued: queued_mlp_v1::Comparison,
}

/// Prefix states remain genuine finite states, never queued projection states.
#[derive(Debug)]
pub(super) struct ProjectionComparedLayerCompletion {
    pub(super) finite: LayerCompletion,
    pub(super) queued: queued_projection_v1::Comparison,
}

trait Backend {
    fn validate(&mut self) -> Result<()>;
    fn capture(&mut self, _boundary: Boundary) -> Result<()> {
        Ok(())
    }
    fn prefix(&mut self) -> Result<([[u32; 22]; 2], [u64; 2])>;
    fn compare_projections(
        &mut self,
        _finite_states: [[u32; 22]; 2],
        _finite_prefix_host_ns: [u64; 2],
    ) -> Result<queued_projection_v1::Comparison> {
        Err("queued projection comparison not selected".into())
    }
    fn first_residual(&mut self) -> Result<[u64; 2]>;
    fn mlp(&mut self) -> Result<([[u32; 11]; 2], [u64; 2])>;
    fn compare_mlp(
        &mut self,
        _finite_states: [[u32; 11]; 2],
        _finite_queue_host_ns: [u64; 2],
    ) -> Result<queued_mlp_v1::Comparison> {
        Err("queued MLP comparison not selected".into())
    }
    fn final_residual(&mut self) -> Result<[u64; 2]>;
    fn poison(&mut self);
}

fn coordinate(backend: &mut impl Backend) -> Result<LayerCompletion> {
    let result = (|| {
        backend.validate()?;
        backend.capture(Boundary::BeforePrefix)?;
        let (prefix_states, prefix) = backend.prefix()?;
        backend.capture(Boundary::AfterPrefix)?;
        let first = backend.first_residual()?;
        backend.capture(Boundary::AfterFirstResidual)?;
        let (mlp_states, mlp) = backend.mlp()?;
        backend.capture(Boundary::AfterMlp)?;
        let last = backend.final_residual()?;
        backend.capture(Boundary::AfterFinalResidual)?;
        Ok(LayerCompletion {
            prefix_states,
            mlp_states,
            paired_ns: [prefix, first, mlp, last],
        })
    })();
    if result.is_err() {
        backend.poison();
    }
    result
}

fn coordinate_comparison(
    backend: &mut impl Backend,
    profile: queued_mlp_v1::ComparisonProfile,
    layer: usize,
) -> Result<ComparedLayerCompletion> {
    let result = (|| {
        profile.validate(layer)?;
        backend.validate()?;
        backend.capture(Boundary::BeforePrefix)?;
        let (prefix_states, prefix) = backend.prefix()?;
        backend.capture(Boundary::AfterPrefix)?;
        let first = backend.first_residual()?;
        backend.capture(Boundary::AfterFirstResidual)?;
        let (mlp_states, mlp) = backend.mlp()?;
        backend.capture(Boundary::AfterMlp)?;
        let queued = backend.compare_mlp(mlp_states, mlp)?;
        let last = backend.final_residual()?;
        backend.capture(Boundary::AfterFinalResidual)?;
        Ok(ComparedLayerCompletion {
            finite: LayerCompletion {
                prefix_states,
                mlp_states,
                paired_ns: [prefix, first, mlp, last],
            },
            queued,
        })
    })();
    if result.is_err() {
        backend.poison();
    }
    result
}

fn coordinate_projection_comparison(
    backend: &mut impl Backend,
    profile: queued_projection_v1::ComparisonProfile,
    layer: usize,
) -> Result<ProjectionComparedLayerCompletion> {
    let result = (|| {
        profile.validate(layer)?;
        backend.validate()?;
        backend.capture(Boundary::BeforePrefix)?;
        let (prefix_states, prefix) = backend.prefix()?;
        backend.capture(Boundary::AfterPrefix)?;
        let queued = backend.compare_projections(prefix_states, prefix)?;
        let first = backend.first_residual()?;
        backend.capture(Boundary::AfterFirstResidual)?;
        let (mlp_states, mlp) = backend.mlp()?;
        backend.capture(Boundary::AfterMlp)?;
        let last = backend.final_residual()?;
        backend.capture(Boundary::AfterFinalResidual)?;
        Ok(ProjectionComparedLayerCompletion {
            finite: LayerCompletion {
                prefix_states,
                mlp_states,
                paired_ns: [prefix, first, mlp, last],
            },
            queued,
        })
    })();
    if result.is_err() {
        backend.poison();
    }
    result
}

fn consumer_bytes() -> Vec<u8> {
    let mut bytes = vec![0; 424];
    for slot in [0usize, 1, 8, 9] {
        bytes[slot * 16 + 8..slot * 16 + 16].copy_from_slice(&4096u64.to_le_bytes());
    }
    bytes[160..164].copy_from_slice(&1u32.to_le_bytes());
    bytes[164..168].copy_from_slice(&2u32.to_le_bytes());
    bytes
}

struct Native<'a> {
    group: &'a mut Group,
    states: &'a mut StateRoster,
    artifacts: &'a LoadedResidentArtifacts,
    layer: usize,
    roots: &'a LayerBindings,
    timeout_ms: u32,
    capture: Option<&'a mut Collector>,
    comparison: Option<(
        &'a queued_mlp_v1::LoadedArtifacts,
        queued_mlp_v1::ComparisonProfile,
    )>,
    projection_comparison: Option<(
        &'a queued_projection_v1::LoadedArtifacts,
        queued_projection_v1::ComparisonProfile,
    )>,
}

impl Native<'_> {
    fn residual(&mut self, first: bool) -> Result<[u64; 2]> {
        let mut commands = Vec::with_capacity(2);
        for rank in 0..2 {
            let mut pointers = (0..8)
                .map(|slot| {
                    let peer = if slot < 2 { slot } else { 0 };
                    self.roots.prefix[peer][13].pointer(
                        slot as u32 * 16,
                        0,
                        if slot < 2 { 16_384 } else { 0 },
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
        // SAFETY: exact pinned residual ABI; both original partials completed
        // before this call. Peer access is read-only and outputs are disjoint.
        unsafe { self.group.dispatch_round_unchecked(commands)? }
            .try_into()
            .map_err(|_| "resident residual paired completion count".into())
    }
}

impl Backend for Native<'_> {
    fn validate(&mut self) -> Result<()> {
        if self.layer >= 36 || !(1..=10_000).contains(&self.timeout_ms) {
            return Err("resident layer index/deadline".into());
        }
        self.roots.validate()?;
        for rank in 0..2 {
            for kind in [
                ResidentKind::Prefix,
                ResidentKind::Mlp,
                ResidentKind::Residual,
            ] {
                if self.artifacts.kernel(rank, kind)?.rank() != rank {
                    return Err("resident layer artifact owner mismatch".into());
                }
            }
        }
        Ok(())
    }

    fn capture(&mut self, boundary: Boundary) -> Result<()> {
        match &mut self.capture {
            Some(capture) => capture.observe(boundary, self.group, self.roots),
            None => Ok(()),
        }
    }

    fn prefix(&mut self) -> Result<([[u32; 22]; 2], [u64; 2])> {
        let completion: Result<[u64; 2]> = (|| {
            let states = self.states.take_prefix(self.layer)?;
            let mut commands = Vec::with_capacity(2);
            for rank in 0..2 {
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
                if states[rank].owner_rank() != rank {
                    return Err("resident prefix state owner".into());
                }
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
            // SAFETY: original reviewed two-workgroup V5 image, exact rank-local
            // roots and fresh typed states retained by the exclusive owner.
            unsafe { self.group.dispatch_round_unchecked(commands)? }
                .try_into()
                .map_err(|_| "resident prefix paired completion count".into())
        })();
        let observed =
            self.states
                .observe_prefix(self.group, self.layer, completion.clone().map(|_| ()))?;
        Ok((observed, completion?))
    }

    fn compare_projections(
        &mut self,
        finite_states: [[u32; 22]; 2],
        finite_prefix_host_ns: [u64; 2],
    ) -> Result<queued_projection_v1::Comparison> {
        let (artifacts, profile) = self
            .projection_comparison
            .ok_or("queued projection comparison not selected")?;
        // SAFETY: the distinct coordinator has just acquired this owner's
        // genuine prefix state. No residual, MLP, query or KV write runs here.
        unsafe {
            queued_projection_v1::compare(
                self.group,
                artifacts,
                self.roots,
                self.timeout_ms,
                profile,
                finite_states,
                finite_prefix_host_ns,
            )
        }
    }

    fn first_residual(&mut self) -> Result<[u64; 2]> {
        self.states.begin_first_residual(self.layer)?;
        let completion = self.residual(true);
        self.states
            .finish_first_residual(self.layer, completion.clone().map(|_| ()))?;
        completion
    }

    fn mlp(&mut self) -> Result<([[u32; 11]; 2], [u64; 2])> {
        let completion: Result<[u64; 2]> = (|| {
            let states = self.states.take_mlp(self.layer)?;
            let mut commands = Vec::with_capacity(2);
            for rank in 0..2 {
                let mut pointers = self.roots.mlp[rank]
                    .iter()
                    .zip(MLP_BYTES)
                    .enumerate()
                    .map(|(i, (root, bytes))| {
                        root.pointer(
                            i as u32 * 8,
                            0,
                            bytes,
                            if i < 5 {
                                Access::Read
                            } else {
                                Access::ReadWrite
                            },
                        )
                    })
                    .collect::<Vec<_>>();
                if states[rank].owner_rank() != rank {
                    return Err("resident MLP state owner".into());
                }
                pointers.push(states[rank].pointer_mlp_v1());
                commands.push(Dispatch {
                    kernel: self.artifacts.kernel(rank, ResidentKind::Mlp)?,
                    bytes: vec![0; 344],
                    workgroup: [64, 1, 1],
                    grid: [128, 1, 1],
                    pointers,
                    timeout_ms: self.timeout_ms,
                });
            }
            // SAFETY: exact pinned MLP image and private roots/state; no peer
            // reads occur until both dispatches and semantic states complete.
            unsafe { self.group.dispatch_round_unchecked(commands)? }
                .try_into()
                .map_err(|_| "resident MLP paired completion count".into())
        })();
        let observed =
            self.states
                .observe_mlp(self.group, self.layer, completion.clone().map(|_| ()))?;
        Ok((observed, completion?))
    }

    fn final_residual(&mut self) -> Result<[u64; 2]> {
        self.states.begin_final_residual(self.layer)?;
        let completion = self.residual(false);
        self.states
            .finish_final_residual(self.layer, completion.clone().map(|_| ()))?;
        completion
    }

    fn compare_mlp(
        &mut self,
        finite_states: [[u32; 11]; 2],
        finite_queue_host_ns: [u64; 2],
    ) -> Result<queued_mlp_v1::Comparison> {
        let (artifacts, profile) = self
            .comparison
            .ok_or("queued MLP comparison not selected")?;
        // SAFETY: called by the distinct coordinator only after this Native's
        // actual MLP dispatch and acquired terminal-state validation completed.
        unsafe {
            queued_mlp_v1::compare(
                self.group,
                artifacts,
                self.roots,
                self.timeout_ms,
                profile,
                finite_states,
                finite_queue_host_ns,
            )
        }
    }

    fn poison(&mut self) {
        self.states.poison();
    }
}

/// Execute one layer from the owning catalog; no token/model commit occurs here.
/// # Safety
/// The private caller must bind authenticated model bytes and exact root roles,
/// reviewed image provenance and engineering device/visibility assumptions to
/// this exclusive group. It must have begun the matching roster forward after
/// real embedding/metadata completion, and make every error terminal. These
/// requirements are not satisfied by descriptive IPC IDs or an image hash alone.
pub(super) unsafe fn execute_layer(
    group: &mut Group,
    states: &mut StateRoster,
    artifacts: &LoadedResidentArtifacts,
    layer: usize,
    roots: &LayerBindings,
    timeout_ms: u32,
) -> Result<LayerCompletion> {
    coordinate(&mut Native {
        group,
        states,
        artifacts,
        layer,
        roots,
        timeout_ms,
        capture: None,
        comparison: None,
        projection_comparison: None,
    })
}

/// Same execution and safety contract as execute_layer, with a separate fixed
/// diagnostic sink. The caller must select this only for actual generation1,
/// position0, layer0 after normal forward validation. No wire/default selects it.
pub(super) unsafe fn execute_layer_with_capture(
    group: &mut Group,
    states: &mut StateRoster,
    artifacts: &LoadedResidentArtifacts,
    layer: usize,
    roots: &LayerBindings,
    timeout_ms: u32,
    capture: &mut Collector,
) -> Result<LayerCompletion> {
    if layer != 0 {
        states.poison();
        return Err("diagnostic execution requires layer zero".into());
    }
    coordinate(&mut Native {
        group,
        states,
        artifacts,
        layer,
        roots,
        timeout_ms,
        capture: Some(capture),
        comparison: None,
        projection_comparison: None,
    })
}

/// Explicit diagnostic dual execution over the same layer-zero roots. The
/// ordinary and capture paths above never call the queued kernels.
/// # Safety
/// Same contract as execute_layer. Additionally, queued_artifacts must be the
/// retained P218 objects loaded into this same exclusive group. The caller must
/// select a distinct engineering profile and label both sets of host durations
/// separately; no queued semantic state or whole-model speedup is established.
pub(super) unsafe fn execute_layer_with_mlp_comparison(
    group: &mut Group,
    states: &mut StateRoster,
    artifacts: &LoadedResidentArtifacts,
    queued_artifacts: &queued_mlp_v1::LoadedArtifacts,
    layer: usize,
    roots: &LayerBindings,
    timeout_ms: u32,
    profile: queued_mlp_v1::ComparisonProfile,
) -> Result<ComparedLayerCompletion> {
    coordinate_comparison(
        &mut Native {
            group,
            states,
            artifacts,
            layer,
            roots,
            timeout_ms,
            capture: None,
            comparison: Some((queued_artifacts, profile)),
            projection_comparison: None,
        },
        profile,
        layer,
    )
}

/// Distinct opt-in Q/K/V/O comparison after finite prefix acquire and before
/// first residual. The ordinary, capture and queued-MLP routes stay unchanged.
/// # Safety
/// Same authenticated roots/exclusive owner contract as execute_layer, plus the
/// exact retained V3 projection entries loaded in this group. Whole-forward and
/// healthy-close publication must remain outside this private layer helper.
pub(super) unsafe fn execute_layer_with_projection_comparison(
    group: &mut Group,
    states: &mut StateRoster,
    artifacts: &LoadedResidentArtifacts,
    queued_artifacts: &queued_projection_v1::LoadedArtifacts,
    layer: usize,
    roots: &LayerBindings,
    timeout_ms: u32,
    profile: queued_projection_v1::ComparisonProfile,
) -> Result<ProjectionComparedLayerCompletion> {
    coordinate_projection_comparison(
        &mut Native {
            group,
            states,
            artifacts,
            layer,
            roots,
            timeout_ms,
            capture: None,
            comparison: None,
            projection_comparison: Some((queued_artifacts, profile)),
        },
        profile,
        layer,
    )
}

#[cfg(test)]
#[path = "resident_layer_tests.rs"]
mod tests;

/// Same authenticated model/root lifetime as the V1 layer, plus a separately
/// reviewed V2 image and two fresh typed548 states in the same retained group.
/// # Safety
/// This is one diagnostic dual-execution at layer zero, not V2 model admission.
pub(super) unsafe fn execute_layer_with_tiles_comparison(
    group: &mut Group,
    states: &mut StateRoster,
    artifacts: &LoadedResidentArtifacts,
    tiles: &mut mlp_tiles_v2::LoadedArtifacts,
    layer: usize,
    roots: &LayerBindings,
    timeout_ms: u32,
) -> Result<(LayerCompletion, mlp_tiles_v2::Comparison)> {
    mlp_tiles_v2::execute(
        Native {
            group,
            states,
            artifacts,
            layer,
            roots,
            timeout_ms,
            capture: None,
            comparison: None,
            projection_comparison: None,
        },
        tiles,
    )
}
