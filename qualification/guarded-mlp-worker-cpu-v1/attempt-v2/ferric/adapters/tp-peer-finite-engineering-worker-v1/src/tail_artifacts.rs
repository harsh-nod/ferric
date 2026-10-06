//! Closed retained ordinary tail entries, not fresh compiler or GPU evidence.

use fe2o3_kfd::{
    Gfx950EngineeringPeerGroupV1 as Group, Gfx950EngineeringPeerKernelV1 as Kernel,
    engineering_wire::{BufferAccessV1 as Access, ExplicitArgumentV1, KernelMetadataV1},
};
use sha2::{Digest, Sha256};

type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TailKind {
    Embedding,
    Copy,
    FinalNorm,
    Head,
    Argmax,
}

impl TailKind {
    pub(super) const ALL: [Self; 5] = [
        Self::Embedding,
        Self::Copy,
        Self::FinalNorm,
        Self::Head,
        Self::Argmax,
    ];
    pub(super) const fn index(self) -> usize {
        match self {
            Self::Embedding => 0,
            Self::Copy => 1,
            Self::FinalNorm => 2,
            Self::Head => 3,
            Self::Argmax => 4,
        }
    }
    pub(super) const fn rank(self) -> usize {
        if matches!(self, Self::Copy) { 1 } else { 0 }
    }
    pub(super) const fn explicit_bytes(self) -> u32 {
        match self {
            Self::Embedding => 56,
            Self::Copy | Self::Argmax => 40,
            Self::FinalNorm => 96,
            Self::Head => 72,
        }
    }
    pub(super) const fn grid(self) -> [u32; 3] {
        // Native dispatch grid is thread count, not workgroup count.
        [
            match self {
                Self::Embedding | Self::Copy => 4096,
                Self::FinalNorm | Self::Argmax => 64,
                Self::Head => 9496 * 64,
            },
            1,
            1,
        ]
    }
    const fn image(self) -> usize {
        if matches!(self, Self::Copy) { 1 } else { 0 }
    }
    const fn symbol(self) -> &'static str {
        match self {
            Self::Embedding => "ferric_qwen3_tp_batch_embedding_bf16_v2",
            Self::Copy => "ferric_qwen3_tp_peer_copy_bf16_v4",
            Self::FinalNorm => "qwen3_rmsnorm_v1",
            Self::Head => "ferric_qwen3_tp_mfma_gemm_bf16_v3",
            Self::Argmax => "ferric_qwen3_tp_batch_argmax_bf16_v2",
        }
    }
}

const fn hex(text: &[u8; 64]) -> [u8; 32] {
    const fn digit(byte: u8) -> u8 {
        match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            _ => panic!("hex pin"),
        }
    }
    let mut output = [0; 32];
    let mut i = 0;
    while i < 32 {
        output[i] = digit(text[i * 2]) * 16 + digit(text[i * 2 + 1]);
        i += 1;
    }
    output
}
const IMAGE_BYTES: [usize; 2] = [112_872, 14_624];
const IMAGE_SHA: [[u8; 32]; 2] = [
    hex(b"11f53cfe2d18668f191af9f998f48627211aa09a36d5483e516946032ad54d8e"),
    hex(b"8436c1861dcc14f0346aeecc136e4187c459ac1c0b31778265c88b3d8c63396d"),
];

/// Owns only the exact two retained objects. Source custody remains historical.
pub(super) struct ReviewedTailImages {
    objects: [Vec<u8>; 2],
}
impl ReviewedTailImages {
    pub(super) fn new(v3: Vec<u8>, v18: Vec<u8>) -> Result<Self> {
        let objects = [v3, v18];
        for (index, object) in objects.iter().enumerate() {
            if object.len() != IMAGE_BYTES[index]
                || <[u8; 32]>::from(Sha256::digest(object)) != IMAGE_SHA[index]
            {
                return Err("tail is not the exact retained V3/V18 image".into());
            }
        }
        Ok(Self { objects })
    }
}

pub(super) struct LoadedTailArtifacts {
    kernels: [Kernel; 5],
}
impl LoadedTailArtifacts {
    pub(super) fn kernel(&self, kind: TailKind) -> &Kernel {
        &self.kernels[kind.index()]
    }
}

fn arguments(kind: TailKind) -> Vec<ExplicitArgumentV1> {
    let (slices, scalars): (&[(u32, Access)], usize) = match kind {
        TailKind::Embedding => (
            &[(4, Access::Read), (2, Access::Read), (2, Access::Write)],
            1,
        ),
        TailKind::Copy => (&[(2, Access::Read), (2, Access::Write)], 1),
        TailKind::FinalNorm => (
            &[
                (2, Access::Read),
                (2, Access::Read),
                (2, Access::Read),
                (2, Access::Write),
                (2, Access::Write),
            ],
            4,
        ),
        TailKind::Head => (
            &[(2, Access::Read), (2, Access::Read), (2, Access::Write)],
            5,
        ),
        TailKind::Argmax => (&[(2, Access::Read), (4, Access::Write)], 1),
    };
    let mut result = Vec::with_capacity(2 * slices.len() + scalars);
    for (index, (alignment, access)) in slices.iter().enumerate() {
        result.push(ExplicitArgumentV1 {
            offset: index as u32 * 16,
            bytes: 8,
            global_buffer: true,
            pointee_alignment: Some(*alignment),
            access: Some(*access),
        });
        result.push(ExplicitArgumentV1 {
            offset: index as u32 * 16 + 8,
            bytes: 8,
            global_buffer: false,
            pointee_alignment: None,
            access: None,
        });
    }
    for index in 0..scalars {
        result.push(ExplicitArgumentV1 {
            offset: slices.len() as u32 * 16 + index as u32 * 4,
            bytes: 4,
            global_buffer: false,
            pointee_alignment: None,
            access: None,
        });
    }
    result
}

fn validate_metadata(kind: TailKind, metadata: &KernelMetadataV1) -> Result<()> {
    let expected = arguments(kind);
    if metadata.object_sha256 != IMAGE_SHA[kind.image()]
        || metadata.symbol != kind.symbol()
        || metadata.kernarg_bytes != kind.explicit_bytes() + 256
        || metadata.kernarg_alignment != 8
        || metadata.group_segment_bytes != 0
        || metadata.private_segment_bytes != 0
        || metadata.wavefront_size != 64
        || metadata.implicit_argument_offset != Some(kind.explicit_bytes())
        || metadata.implicit_argument_bytes != 256
        || metadata.explicit_arguments.len() != expected.len()
    {
        return Err(format!("tail {kind:?} object metadata mismatch"));
    }
    for (actual, expected) in metadata.explicit_arguments.iter().zip(&expected) {
        // Exact object pins bind omitted optional pointer attributes.
        if actual.offset != expected.offset
            || actual.bytes != expected.bytes
            || actual.global_buffer != expected.global_buffer
            || actual.pointee_alignment.is_some()
                && actual.pointee_alignment != expected.pointee_alignment
            || actual.access.is_some() && actual.access != expected.access
        {
            return Err(format!("tail {kind:?} argument metadata mismatch"));
        }
    }
    Ok(())
}

trait Loader {
    type Token;
    fn load(&mut self, kind: TailKind, bytes: Vec<u8>) -> Result<(Self::Token, KernelMetadataV1)>;
}
impl Loader for Group {
    type Token = Kernel;
    fn load(&mut self, kind: TailKind, bytes: Vec<u8>) -> Result<(Kernel, KernelMetadataV1)> {
        let token = self.load_kernel(
            kind.rank(),
            bytes,
            IMAGE_SHA[kind.image()],
            kind.symbol().into(),
        )?;
        if token.rank() != kind.rank() {
            return Err("tail native kernel rank mismatch".into());
        }
        let metadata = token.metadata().clone();
        Ok((token, metadata))
    }
}
fn load_all<L: Loader>(loader: &mut L, images: ReviewedTailImages) -> Result<[L::Token; 5]> {
    let mut tokens = Vec::with_capacity(5);
    for kind in TailKind::ALL {
        let (token, metadata) = loader.load(kind, images.objects[kind.image()].clone())?;
        validate_metadata(kind, &metadata)?;
        tokens.push(token);
    }
    tokens.try_into().map_err(|_| "tail kernel roster".into())
}

/// Private owner must make any error terminal, retaining uncertain native state.
pub(super) fn load(group: &mut Group, images: ReviewedTailImages) -> Result<LoadedTailArtifacts> {
    Ok(LoadedTailArtifacts {
        kernels: load_all(group, images)?,
    })
}

#[cfg(test)]
#[path = "tail_artifacts_tests.rs"]
mod tests;
