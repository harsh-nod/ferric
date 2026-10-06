//! Actual supplied object custody; no generated review or historical V1 alias.
use super::Result;
use crate::finite_mlp_tiles_comparison_wire_v1::MAX_TILES_IMAGE_BYTES;
use crate::finite_setup_wire_v1::Part;
use fe2o3_kfd::{
    Gfx950EngineeringPeerGroupV1 as Group, Gfx950EngineeringPeerKernelV1 as Kernel,
    Gfx950EngineeringPeerWaveMlpTilesStateV2 as State,
    engineering_wire::{BufferAccessV1 as Access, KernelMetadataV1},
};
use sha2::{Digest, Sha256};

pub(super) const SYMBOL: &str = "ferric_qwen3_claimed_mlp_tiles_bf16_f32_v2";
pub(crate) struct Image {
    bytes: Vec<u8>,
    sha256: [u8; 32],
}
impl Image {
    pub(crate) fn sha256(&self) -> [u8; 32] {
        self.sha256
    }
    pub(crate) fn new(bytes: Vec<u8>, pin: &Part) -> Result<Self> {
        if bytes.is_empty()
            || bytes.len() > MAX_TILES_IMAGE_BYTES
            || bytes.len() != pin.bytes as usize
            || pin.sha256 == [0; 32]
            || <[u8; 32]>::from(Sha256::digest(&bytes)) != pin.sha256
        {
            return Err("V2 comparison supplied image pin".into());
        }
        Ok(Self {
            bytes,
            sha256: pin.sha256,
        })
    }
}
/// Image-only owner for real V2 substitution; states belong to the bank roster.
pub(crate) struct LoadedKernels {
    pub(crate) kernels: [Kernel; 2],
    pub(crate) sha256: [u8; 32],
}
fn load_decode<L: Loader>(loader: &mut L, image: Image) -> Result<([L::Kernel; 2], [u8; 32])> {
    if loader.counts()? != [570, 566] {
        return Err("tiles decode images require exact uploaded roots before states".into());
    }
    let mut kernels = Vec::with_capacity(2);
    for rank in 0..2 {
        let (kernel, metadata) = loader.kernel(rank, image.bytes.clone(), image.sha256)?;
        validate_metadata(&metadata, image.sha256)?;
        kernels.push(kernel);
    }
    if loader.counts()? != [570, 566] {
        return Err("tiles image load changed source census".into());
    }
    Ok((
        kernels.try_into().map_err(|_| "tiles image owner count")?,
        image.sha256,
    ))
}
pub(crate) fn load_decode_kernels(group: &mut Group, image: Image) -> Result<LoadedKernels> {
    let (kernels, sha256) = load_decode(group, image)?;
    Ok(LoadedKernels { kernels, sha256 })
}
pub(crate) struct LoadedArtifacts {
    pub(super) kernels: [Kernel; 2],
    pub(super) states: [State; 2],
    pub(super) sha256: [u8; 32],
}
fn validate_metadata(value: &KernelMetadataV1, sha: [u8; 32]) -> Result<()> {
    if value.object_sha256 != sha
        || value.symbol != SYMBOL
        || value.kernarg_bytes != 344
        || value.kernarg_alignment != 8
        || value.wavefront_size != 64
        || value.private_segment_bytes != 0
        || value.group_segment_bytes != 512
        || value.implicit_argument_offset != Some(88)
        || value.implicit_argument_bytes != 256
        || value.explicit_arguments.len() != 11
    {
        return Err("V2 comparison exact typed548 ABI/resources".into());
    }
    for (i, arg) in value.explicit_arguments.iter().enumerate() {
        let access = if i < 5 {
            Access::Read
        } else {
            Access::ReadWrite
        };
        if arg.offset != (i * 8) as u32
            || arg.bytes != 8
            || !arg.global_buffer
            || arg
                .pointee_alignment
                .is_some_and(|a| a != if i < 9 { 2 } else { 4 })
            || arg.access.is_some_and(|a| a != access)
        {
            return Err("V2 comparison pointer ABI".into());
        }
    }
    Ok(())
}
trait Loader {
    type Kernel;
    type State;
    fn counts(&mut self) -> Result<Vec<usize>>;
    fn kernel(
        &mut self,
        rank: usize,
        bytes: Vec<u8>,
        sha: [u8; 32],
    ) -> Result<(Self::Kernel, KernelMetadataV1)>;
    fn state(&mut self, rank: usize) -> Result<Self::State>;
}
impl Loader for Group {
    type Kernel = Kernel;
    type State = State;
    fn counts(&mut self) -> Result<Vec<usize>> {
        self.preflight_additional_allocations_v1(&[0, 0])
    }
    fn kernel(
        &mut self,
        rank: usize,
        bytes: Vec<u8>,
        sha: [u8; 32],
    ) -> Result<(Kernel, KernelMetadataV1)> {
        let value = self.load_kernel(rank, bytes, sha, SYMBOL.into())?;
        if value.rank() != rank {
            return Err("V2 comparison kernel rank".into());
        }
        let metadata = value.metadata().clone();
        Ok((value, metadata))
    }
    fn state(&mut self, rank: usize) -> Result<State> {
        self.allocate_wave_mlp_tiles_state_v2(rank)
    }
}
fn load_all<L: Loader>(
    loader: &mut L,
    image: Image,
) -> Result<([L::Kernel; 2], [L::State; 2], [u8; 32])> {
    if loader.counts()? != [714, 710] {
        return Err("V2 comparison needs exact fresh sealed V1 owner".into());
    }
    let mut kernels = Vec::with_capacity(2);
    for rank in 0..2 {
        let (kernel, metadata) = loader.kernel(rank, image.bytes.clone(), image.sha256)?;
        validate_metadata(&metadata, image.sha256)?;
        kernels.push(kernel);
    }
    let states = [loader.state(0)?, loader.state(1)?];
    if loader.counts()? != [715, 711] {
        return Err("V2 comparison typed-state allocation census".into());
    }
    Ok((
        kernels.try_into().map_err(|_| "V2 kernel count")?,
        states,
        image.sha256,
    ))
}
/// Any failure is terminal for the consuming NativeOwner; partial tokens are
/// never returned and Group retains all actual native allocation records.
pub(crate) fn load(group: &mut Group, image: Image) -> Result<LoadedArtifacts> {
    let (kernels, states, sha256) = load_all(group, image)?;
    Ok(LoadedArtifacts {
        kernels,
        states,
        sha256,
    })
}

#[cfg(test)]
mod tests;
