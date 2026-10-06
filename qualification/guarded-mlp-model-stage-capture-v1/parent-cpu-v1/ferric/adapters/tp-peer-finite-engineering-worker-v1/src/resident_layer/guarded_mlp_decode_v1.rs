//! Prefix284 followed by one retained R1/MLP/validator/R2 segment per layer.
use super::mlp_tiles_v2::artifacts::LoadedKernels;
use super::prefix_tiles_v6::{artifacts::Loaded as PrefixKernels, projection_residual};
use super::{Allocation, Buffer, Group, LayerBindings, Result};
use crate::state_roster::guarded_mlp_decode_v1::Roster;
use fe2o3_kfd::{
    Gfx950EngineeringPeerGuardedMlpInputsV1 as Inputs,
    Gfx950EngineeringPeerGuardedMlpObservationV1 as Observation,
    Gfx950EngineeringPeerGuardedMlpRankInputsV1 as RankInputs,
    Gfx950EngineeringPeerKernelV1 as Kernel,
    Gfx950EngineeringPeerWaveQkvAttentionOutputTilesDispatchV6 as PrefixDispatch,
    engineering_wire::KernelMetadataV1,
};
use sha2::{Digest, Sha256};

pub(crate) use crate::finite_guarded_mlp_decode_wire_v1::GUARDED_IMAGE;
const GUARD: &str = "ferric_qwen3_mlp_state_guard_v2";
const R2: &str = "ferric_qwen3_tp2_guarded_projection_residual_bf16_v2";
pub(crate) struct Image {
    bytes: Vec<u8>,
}
impl Image {
    pub(crate) fn new(bytes: Vec<u8>, pin: &crate::finite_setup_wire_v1::Part) -> Result<Self> {
        if bytes.is_empty()
            || bytes.len() > 32 << 20
            || bytes.len() != pin.bytes as usize
            || pin.sha256 != GUARDED_IMAGE
            || <[u8; 32]>::from(Sha256::digest(&bytes)) != GUARDED_IMAGE
        {
            return Err("guarded decode exact reviewed image bytes".into());
        }
        Ok(Self { bytes })
    }
}
pub(crate) struct Images {
    pub(crate) projection: projection_residual::Image,
    pub(crate) guarded: Image,
}
pub(crate) struct Loaded {
    pub(crate) projection: projection_residual::Loaded,
    validator: [Kernel; 2],
    residual: [Kernel; 2],
}
fn metadata(m: &KernelMetadataV1, validator: bool) -> Result<()> {
    let slices = if validator { 1 } else { 6 };
    let explicit = if validator { 24 } else { 104 };
    if m.object_sha256 != GUARDED_IMAGE
        || m.symbol != if validator { GUARD } else { R2 }
        || m.kernarg_bytes != explicit + 256
        || m.kernarg_alignment != 8
        || m.wavefront_size != 64
        || m.private_segment_bytes != 0
        || m.group_segment_bytes != 0
        || m.implicit_argument_offset != Some(explicit)
        || m.implicit_argument_bytes != 256
        || m.explicit_arguments.len() != slices * 2 + 2
    {
        return Err("guarded decode validator/R2 exact ABI".into());
    }
    for (i, arg) in m.explicit_arguments.iter().enumerate() {
        let (offset, bytes, pointer) = if i < slices * 2 {
            ((i * 8) as u32, 8, i % 2 == 0)
        } else {
            ((slices * 16 + (i - slices * 2) * 4) as u32, 4, false)
        };
        if (arg.offset, arg.bytes, arg.global_buffer) != (offset, bytes, pointer)
            || arg.access.is_some()
            || arg.pointee_alignment.is_some()
        {
            return Err("guarded decode physical argument roster".into());
        }
    }
    Ok(())
}
pub(crate) fn load(group: &mut Group, images: Images) -> Result<Loaded> {
    let projection = projection_residual::load(group, images.projection)?;
    let mut load_pair = |symbol: &str, validator: bool| -> Result<[Kernel; 2]> {
        let mut kernels = Vec::with_capacity(2);
        for rank in 0..2 {
            let k = group.load_kernel(
                rank,
                images.guarded.bytes.clone(),
                GUARDED_IMAGE,
                symbol.into(),
            )?;
            if k.rank() != rank {
                return Err("guarded decode kernel rank".into());
            }
            metadata(k.metadata(), validator)?;
            kernels.push(k);
        }
        kernels
            .try_into()
            .map_err(|_| "guarded decode image pair count".into())
    };
    let validator = load_pair(GUARD, true)?;
    let residual = load_pair(R2, false)?;
    if group.preflight_additional_allocations_v1(&[0, 0])? != [714, 710] {
        return Err("guarded decode image loading changed census".into());
    }
    Ok(Loaded {
        projection,
        validator,
        residual,
    })
}
pub(crate) fn selected_mlp<B: Allocation>(
    roots: &LayerBindings<B>,
    down: [B; 2],
) -> Result<[[B; 10]; 2]> {
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
                .any(|v| *v == down[rank])
        {
            return Err("guarded decode distinct Down scratch".into());
        }
    }
    let mut selected = roots.mlp;
    for rank in 0..2 {
        selected[rank][9] = down[rank];
    }
    Ok(selected)
}
pub(crate) fn inputs<'a>(
    images: &'a Loaded,
    mlp: &'a LoadedKernels,
    roots: &LayerBindings,
    down: [Buffer; 2],
) -> Result<Inputs<'a>> {
    let selected = selected_mlp(roots, down)?;
    let ranks = core::array::from_fn(|rank| RankInputs {
        kernels: [
            &images.projection.kernels[rank],
            &mlp.kernels[rank],
            &images.validator[rank],
            &images.residual[rank],
        ],
        mlp_roots: selected[rank],
        residual_input: roots.prefix[rank][0],
        output: roots.final_hidden[rank],
    });
    Ok(Inputs {
        ranks,
        partials: [roots.prefix[0][13], roots.prefix[1][13]],
        projection_sha256: images.projection.sha256,
        mlp_sha256: mlp.sha256,
    })
}
pub(crate) struct Completion {
    pub(crate) prefix_states: [[u32; 284]; 2],
    pub(crate) prefix_ns: [u64; 2],
    pub(crate) guarded: Observation,
}
trait Backend {
    fn validate(&mut self) -> Result<()>;
    fn capture(&mut self, _boundary: super::capture_v1::Boundary) -> Result<()> {
        Ok(())
    }
    fn prefix(&mut self) -> Result<([[u32; 284]; 2], [u64; 2])>;
    fn guarded(&mut self) -> Result<Observation>;
    fn poison(&mut self);
}
fn coordinate(b: &mut impl Backend) -> Result<Completion> {
    let result = (|| {
        use super::capture_v1::Boundary;
        b.validate()?;
        b.capture(Boundary::BeforePrefix)?;
        let (prefix_states, prefix_ns) = b.prefix()?;
        b.capture(Boundary::AfterPrefix)?;
        let guarded = b.guarded()?;
        // These retained outputs survive the complete paired segment. Sampling
        // here does not insert a host wait or packet between R1, MLP, and R2.
        b.capture(Boundary::AfterFirstResidual)?;
        b.capture(Boundary::AfterMlp)?;
        b.capture(Boundary::AfterFinalResidual)?;
        // R2 is already the final packet in this retained paired segment.
        Ok(Completion {
            prefix_states,
            prefix_ns,
            guarded,
        })
    })();
    if result.is_err() {
        b.poison();
    }
    result
}
struct Native<'a> {
    group: &'a mut Group,
    roster: &'a mut Roster,
    prefix: &'a PrefixKernels,
    mlp: &'a LoadedKernels,
    images: &'a Loaded,
    roots: &'a LayerBindings,
    down: [Buffer; 2],
    layer: usize,
    timeout_ms: u32,
    capture: Option<&'a mut super::capture_v1::Collector>,
}
impl Backend for Native<'_> {
    fn validate(&mut self) -> Result<()> {
        if self.layer >= 36 || !(1..=10000).contains(&self.timeout_ms) {
            return Err("guarded decode layer/deadline".into());
        }
        let _ = inputs(self.images, self.mlp, self.roots, self.down)?;
        for rank in 0..2 {
            if self.prefix.kernels[rank].rank() != rank {
                return Err("guarded prefix kernel rank".into());
            }
        }
        Ok(())
    }
    fn prefix(&mut self) -> Result<([[u32; 284]; 2], [u64; 2])> {
        let completion = (|| {
            let [left, right] = self.roster.take_prefix(self.layer)?;
            let commands = [
                PrefixDispatch {
                    kernel: &self.prefix.kernels[0],
                    object_sha256: self.prefix.sha256,
                    roots: self.roots.prefix[0],
                    state: left,
                    timeout_ms: self.timeout_ms,
                },
                PrefixDispatch {
                    kernel: &self.prefix.kernels[1],
                    object_sha256: self.prefix.sha256,
                    roots: self.roots.prefix[1],
                    state: right,
                    timeout_ms: self.timeout_ms,
                },
            ];
            // SAFETY: sealed original model roles and exact typed284 ownership.
            unsafe {
                self.group
                    .dispatch_wave_qkv_attention_output_tiles_round_v6(commands)
            }
        })();
        let words = self.roster.finish_prefix(
            self.group,
            self.layer,
            completion
                .as_ref()
                .map(|v| v.final_states)
                .map_err(Clone::clone),
        )?;
        Ok((words, completion?.dispatch_elapsed_ns))
    }
    fn guarded(&mut self) -> Result<Observation> {
        self.roster.dispatch(
            self.group,
            self.layer,
            inputs(self.images, self.mlp, self.roots, self.down)?,
            self.timeout_ms,
        )
    }
    fn capture(&mut self, boundary: super::capture_v1::Boundary) -> Result<()> {
        if let Some(capture) = self.capture.as_mut() {
            capture.observe_guarded(boundary, self.group, self.roots, self.down)?;
        }
        Ok(())
    }
    fn poison(&mut self) {
        self.roster.poison();
    }
}
/// # Safety
/// Exact authenticated images/model roles, selected mixed-bank generation and
/// full Prefix retirement are obligations of the private retained model owner.
pub(crate) unsafe fn execute(
    group: &mut Group,
    roster: &mut Roster,
    prefix: &PrefixKernels,
    mlp: &LoadedKernels,
    images: &Loaded,
    roots: &LayerBindings,
    down: [Buffer; 2],
    layer: usize,
    timeout_ms: u32,
) -> Result<Completion> {
    coordinate(&mut Native {
        group,
        roster,
        prefix,
        mlp,
        images,
        roots,
        down,
        layer,
        timeout_ms,
        capture: None,
    })
}

/// Identical dispatch route with bounded, opt-in readback of retained layer0 stages.
/// # Safety
/// The same authenticated images, role bindings, and retirement requirements as
/// `execute` apply. The collector additionally enforces generation1/position0/layer0.
pub(crate) unsafe fn execute_with_capture(
    group: &mut Group,
    roster: &mut Roster,
    prefix: &PrefixKernels,
    mlp: &LoadedKernels,
    images: &Loaded,
    roots: &LayerBindings,
    down: [Buffer; 2],
    layer: usize,
    timeout_ms: u32,
    capture: &mut super::capture_v1::Collector,
) -> Result<Completion> {
    if layer != 0 {
        roster.poison();
        return Err("guarded capture requires layer zero".into());
    }
    coordinate(&mut Native {
        group, roster, prefix, mlp, images, roots, down, layer, timeout_ms,
        capture: Some(capture),
    })
}

#[cfg(test)]
mod tests;
