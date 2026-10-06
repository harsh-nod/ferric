//! Explicit actual candidate image custody, not proof of its arithmetic.
use super::Result;
use crate::finite_setup_wire_v1::Part;
use fe2o3_kfd::{
    Gfx950EngineeringPeerGroupV1 as Group, Gfx950EngineeringPeerKernelV1 as Kernel,
    engineering_wire::{BufferAccessV1 as Access, KernelMetadataV1},
};
use sha2::{Digest, Sha256};

pub(crate) const SYMBOL: &str = "ferric_qwen3_tp_peer_tp2_projection_residual_bf16_v1";
pub(crate) struct Image {
    bytes: Vec<u8>,
    sha256: [u8; 32],
}
impl Image {
    pub(crate) fn new(bytes: Vec<u8>, pin: &Part) -> Result<Self> {
        if bytes.is_empty()
            || bytes.len() > 32 << 20
            || bytes.len() != pin.bytes as usize
            || pin.sha256 == [0; 32]
            || <[u8; 32]>::from(Sha256::digest(&bytes)) != pin.sha256
        {
            return Err("projection-residual actual image extent/digest".into());
        }
        Ok(Self {
            bytes,
            sha256: pin.sha256,
        })
    }
    pub(crate) fn sha256(&self) -> [u8; 32] {
        self.sha256
    }
}
pub(crate) struct Loaded {
    pub(crate) kernels: [Kernel; 2],
    pub(crate) sha256: [u8; 32],
}

fn validate(m: &KernelMetadataV1, sha: [u8; 32]) -> Result<()> {
    if m.object_sha256 != sha
        || m.symbol != SYMBOL
        || m.kernarg_bytes != 424
        || m.kernarg_alignment != 8
        || m.wavefront_size != 64
        || m.private_segment_bytes != 0
        || m.group_segment_bytes != 0
        || m.implicit_argument_offset != Some(168)
        || m.implicit_argument_bytes != 256
        || m.explicit_arguments.len() != 22
    {
        return Err("projection-residual distinct168/22 metadata".into());
    }
    for (i, a) in m.explicit_arguments.iter().enumerate() {
        let pointer = i < 20 && i % 2 == 0;
        let offset = if i < 20 {
            i as u32 * 8
        } else {
            160 + (i as u32 - 20) * 4
        };
        let width = if i < 20 { 8 } else { 4 };
        let alignment = if i < 16 { 4 } else { 2 };
        let access = if i == 18 { Access::Write } else { Access::Read };
        if a.offset != offset
            || a.bytes != width
            || a.global_buffer != pointer
            || a.pointee_alignment
                .is_some_and(|v| !pointer || v != alignment)
            || a.access.is_some_and(|v| !pointer || v != access)
        {
            return Err("projection-residual pointer/scalar role".into());
        }
    }
    Ok(())
}

trait Loader {
    type Token;
    fn counts(&mut self) -> Result<Vec<usize>>;
    fn load(
        &mut self,
        rank: usize,
        image: &Image,
    ) -> Result<(Self::Token, usize, KernelMetadataV1)>;
}
impl Loader for Group {
    type Token = Kernel;
    fn counts(&mut self) -> Result<Vec<usize>> {
        self.preflight_additional_allocations_v1(&[0, 0])
    }
    fn load(&mut self, rank: usize, image: &Image) -> Result<(Kernel, usize, KernelMetadataV1)> {
        let kernel = self.load_kernel(rank, image.bytes.clone(), image.sha256, SYMBOL.into())?;
        let owner = kernel.rank();
        let metadata = kernel.metadata().clone();
        Ok((kernel, owner, metadata))
    }
}
fn load_pair<L: Loader>(loader: &mut L, image: &Image) -> Result<[L::Token; 2]> {
    if loader.counts()? != [714, 710] {
        return Err("candidate requires sealed original allocation census".into());
    }
    let mut tokens = Vec::with_capacity(2);
    for rank in 0..2 {
        let (token, owner, metadata) = loader.load(rank, image)?;
        if owner != rank {
            return Err("candidate residual rank ownership".into());
        }
        validate(&metadata, image.sha256)?;
        tokens.push(token);
    }
    if loader.counts()? != [714, 710] {
        return Err("candidate image changed allocation census".into());
    }
    tokens
        .try_into()
        .map_err(|_| "candidate residual pair count".into())
}
/// The consuming owner makes every failed load/metadata/census check terminal.
pub(crate) fn load(group: &mut Group, image: Image) -> Result<Loaded> {
    Ok(Loaded {
        kernels: load_pair(group, &image)?,
        sha256: image.sha256,
    })
}

#[cfg(test)]
mod tests;
