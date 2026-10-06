//! Two rank-local ordered packets; distinct Down outputs preserve both O inputs.
use super::*;
use fe2o3_kfd::{
    Gfx950EngineeringPeerBufferV1 as Buffer,
    Gfx950EngineeringPeerProjectionResidualMlpTilesDispatchV1 as Compound,
};

pub(crate) fn validate_down<B: super::super::Allocation>(
    roots: &LayerBindings<B>,
    down: [B; 2],
) -> Result<()> {
    roots.validate()?;
    for rank in 0..2 {
        if down[rank].owner() != rank
            || down[rank].bytes() != 16384
            || down[0] == down[1]
            || roots
                .prefix
                .iter()
                .flatten()
                .chain(roots.mlp.iter().flatten())
                .chain(roots.final_hidden.iter())
                .any(|b| *b == down[rank])
        {
            return Err("ordered Down scratch owner/extent/alias".into());
        }
    }
    Ok(())
}
trait ScratchAllocator {
    type Buffer: super::super::Allocation;
    fn preflight(&mut self, extra: &[usize]) -> Result<Vec<usize>>;
    fn allocate(&mut self, rank: usize) -> Result<Self::Buffer>;
}
impl ScratchAllocator for Group {
    type Buffer = Buffer;
    fn preflight(&mut self, extra: &[usize]) -> Result<Vec<usize>> {
        self.preflight_additional_allocations_v1(extra)
    }
    fn allocate(&mut self, rank: usize) -> Result<Buffer> {
        Group::allocate(self, rank, &[1 - rank], 16384)
    }
}
fn allocate<A: ScratchAllocator>(
    group: &mut A,
    layers: &[LayerBindings<A::Buffer>],
) -> Result<[A::Buffer; 2]> {
    if layers.len() != 36 || group.preflight(&[1, 1])? != [714, 710] {
        return Err("ordered auxiliary scratch preflight".into());
    }
    for roots in layers {
        roots.validate()?;
    }
    let down = [group.allocate(0)?, group.allocate(1)?];
    for roots in layers {
        validate_down(roots, down)?;
    }
    if group.preflight(&[0, 0])? != [715, 711] {
        return Err("ordered auxiliary scratch census".into());
    }
    Ok(down)
}
pub(crate) fn allocate_down(group: &mut Group, layers: &[LayerBindings]) -> Result<[Buffer; 2]> {
    allocate(group, layers)
}
fn selected_mlp<B: super::super::Allocation>(
    roots: &LayerBindings<B>,
    down: [B; 2],
) -> Result<[[B; 10]; 2]> {
    validate_down(roots, down)?;
    let mut selected = roots.mlp;
    for rank in 0..2 {
        selected[rank][9] = down[rank];
    }
    Ok(selected)
}
trait CompoundBackend: Backend {
    fn compound(&mut self) -> Result<([[u32; 548]; 2], u64)>;
}
fn coordinate_ordered(b: &mut impl CompoundBackend) -> Result<Completion> {
    let result = (|| {
        b.validate()?;
        let (prefix_states, prefix_ns) = b.prefix()?;
        let (mlp_states, segment_host_ns) = b.compound()?;
        let final_residual_ns = b.residual(false)?;
        Ok(Completion {
            prefix_states,
            mlp_states,
            timing: Timing::Ordered {
                prefix_ns,
                segment_host_ns,
                final_residual_ns,
            },
        })
    })();
    if result.is_err() {
        b.poison();
    }
    result
}
struct Ordered<'a>(Native<'a>);
impl Backend for Ordered<'_> {
    fn validate(&mut self) -> Result<()> {
        self.0.validate()?;
        if self.0.recording.is_some() || self.0.projection.is_none() {
            return Err("ordered requires separate projection and no raw recording".into());
        }
        validate_down(
            self.0.roots,
            self.0.down.ok_or("ordered Down scratch absent")?,
        )
    }
    fn prefix(&mut self) -> Result<([[u32; 284]; 2], [u64; 2])> {
        self.0.prefix()
    }
    fn residual(&mut self, first: bool) -> Result<[u64; 2]> {
        if first {
            return Err("ordered first residual must be compound".into());
        }
        self.0.residual(false)
    }
    fn mlp(&mut self) -> Result<([[u32; 548]; 2], [u64; 2])> {
        Err("ordered MLP must be compound".into())
    }
    fn poison(&mut self) {
        self.0.poison();
    }
}
impl CompoundBackend for Ordered<'_> {
    fn compound(&mut self) -> Result<([[u32; 548]; 2], u64)> {
        let native = &mut self.0;
        let result = (|| {
            let projection = native.projection.ok_or("ordered projection absent")?;
            let down = native.down.ok_or("ordered Down absent")?;
            let roots = selected_mlp(native.roots, down)?;
            let partials = [native.roots.prefix[0][13], native.roots.prefix[1][13]];
            let [left, right] = native.states.take_residual_mlp(native.layer)?;
            let commands = [
                Compound {
                    projection_kernel: &projection.kernels[0],
                    projection_object_sha256: projection.sha256,
                    partials,
                    residual_input: native.roots.prefix[0][0],
                    mlp: MlpDispatch {
                        kernel: &native.mlp.kernels[0],
                        object_sha256: native.mlp.sha256,
                        roots: roots[0],
                        state: left,
                        timeout_ms: native.timeout_ms,
                    },
                },
                Compound {
                    projection_kernel: &projection.kernels[1],
                    projection_object_sha256: projection.sha256,
                    partials,
                    residual_input: native.roots.prefix[1][0],
                    mlp: MlpDispatch {
                        kernel: &native.mlp.kernels[1],
                        object_sha256: native.mlp.sha256,
                        roots: roots[1],
                        state: right,
                        timeout_ms: native.timeout_ms,
                    },
                },
            ];
            // SAFETY: both Prefix284 producers retired before this call. Both O
            // partials remain read-only throughout the compound; Down uses the
            // private retained pair. Runtime preflights all four packets and
            // returns only after both queues, all signals and terminal states.
            unsafe {
                native
                    .group
                    .dispatch_projection_residual_mlp_tiles_round_unchecked_v1(commands)
            }
        })();
        let words = native.states.finish_residual_mlp(
            native.group,
            native.layer,
            result
                .as_ref()
                .map(|v| v.final_states)
                .map_err(Clone::clone),
        )?;
        Ok((words, result?.segment_host_ns))
    }
}
/// # Safety
/// Exact reviewed images, sealed model roots and private setup-owned Down buffers.
pub(crate) unsafe fn execute(
    group: &mut Group,
    states: &mut Roster,
    original: &LoadedResidentArtifacts,
    prefix: &LoadedPrefix,
    mlp: &LoadedKernels,
    roots: &LayerBindings,
    layer: usize,
    timeout_ms: u32,
    projection: &LoadedProjection,
    down: [Buffer; 2],
) -> Result<Completion> {
    coordinate_ordered(&mut Ordered(Native {
        group,
        states,
        original,
        projection: Some(projection),
        down: Some(down),
        prefix,
        mlp,
        roots,
        layer,
        timeout_ms,
        recording: None,
    }))
}
#[cfg(test)]
mod tests;
