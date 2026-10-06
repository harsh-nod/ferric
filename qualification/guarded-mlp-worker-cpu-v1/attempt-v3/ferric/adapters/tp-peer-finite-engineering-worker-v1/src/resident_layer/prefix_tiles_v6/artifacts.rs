//! Caller-supplied V6 image custody, distinct from the same-width V5 ABI.
use super::Result;
use crate::finite_setup_wire_v1::Part;
use fe2o3_kfd::{
    Gfx950EngineeringPeerGroupV1 as Group, Gfx950EngineeringPeerKernelV1 as Kernel,
    engineering_wire::{BufferAccessV1 as Access, KernelMetadataV1},
};
use sha2::{Digest, Sha256};

pub(crate) const SYMBOL: &str =
    "ferric_qwen3_claimed_rmsnorm_qkv_attention_output_tiles_bf16_f32_v6";
pub(crate) struct Image {
    bytes: Vec<u8>,
    sha256: [u8; 32],
}
impl Image {
    pub(crate) fn new(bytes: Vec<u8>, pin: &Part) -> Result<Self> {
        if bytes.is_empty()
            || bytes.len() > 64 << 20
            || bytes.len() != pin.bytes as usize
            || pin.sha256 == [0; 32]
            || <[u8; 32]>::from(Sha256::digest(&bytes)) != pin.sha256
        {
            return Err("prefix V6 actual supplied image extent/digest".into());
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
        || m.kernarg_bytes != 376
        || m.kernarg_alignment != 8
        || m.wavefront_size != 64
        || m.private_segment_bytes != 0
        || m.group_segment_bytes != 512
        || m.implicit_argument_offset != Some(120)
        || m.implicit_argument_bytes != 256
        || m.explicit_arguments.len() != 15
    {
        return Err("prefix V6 distinct typed284 image metadata".into());
    }
    for (i, arg) in m.explicit_arguments.iter().enumerate() {
        let alignment = if matches!(i, 4 | 5 | 13 | 14) { 4 } else { 2 };
        let access = if i < 7 {
            Access::Read
        } else {
            Access::ReadWrite
        };
        if arg.offset != i as u32 * 8
            || arg.bytes != 8
            || !arg.global_buffer
            || arg.pointee_alignment.is_some_and(|a| a != alignment)
            || arg.access.is_some_and(|a| a != access)
        {
            return Err("prefix V6 pointer role/offset/alignment".into());
        }
    }
    Ok(())
}
pub(crate) fn load(group: &mut Group, image: Image) -> Result<Loaded> {
    if group.preflight_additional_allocations_v1(&[0, 0])? != [570, 566] {
        return Err("prefix V6 requires uploaded roots before typed states".into());
    }
    let mut kernels = Vec::with_capacity(2);
    for rank in 0..2 {
        let kernel = group.load_kernel(rank, image.bytes.clone(), image.sha256, SYMBOL.into())?;
        if kernel.rank() != rank {
            return Err("prefix V6 image rank".into());
        }
        validate(kernel.metadata(), image.sha256)?;
        kernels.push(kernel);
    }
    if group.preflight_additional_allocations_v1(&[0, 0])? != [570, 566] {
        return Err("prefix V6 image load changed allocation census".into());
    }
    Ok(Loaded {
        kernels: kernels.try_into().map_err(|_| "prefix image count")?,
        sha256: image.sha256,
    })
}

#[cfg(test)]
mod tests;
