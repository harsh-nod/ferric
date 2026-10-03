//! Exact P219 engineering images. This does not confer production authority.

use fe2o3_kfd::{
    Gfx950EngineeringPeerGroupV1 as Group, Gfx950EngineeringPeerKernelV1 as Kernel,
    engineering_wire::{BufferAccessV1 as Access, ExplicitArgumentV1, KernelMetadataV1},
};
use sha2::{Digest, Sha256};

type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ResidentKind {
    Prefix,
    Mlp,
    Residual,
}

impl ResidentKind {
    const ALL: [Self; 3] = [Self::Prefix, Self::Mlp, Self::Residual];

    const fn index(self) -> usize {
        match self {
            Self::Prefix => 0,
            Self::Mlp => 1,
            Self::Residual => 2,
        }
    }

    const fn spec(self) -> Spec {
        match self {
            Self::Prefix => Spec {
                bytes: 48_584,
                digest: hex(b"4d0fe835eef3b76bc1b60ca560e6a8028f85bb9ce292bac77ccf6c6da65e6285"),
                symbol: "ferric_qwen3_claimed_rmsnorm_qkv_attention_output_bf16_f32_v5",
                explicit_bytes: 120,
                arguments: 15,
                lds: 512,
            },
            Self::Mlp => Spec {
                bytes: 29_656,
                digest: hex(b"decdb15e92318fb289306e98f53010b125d81d8898baf191be1fb9641597cb21"),
                symbol: "ferric_qwen3_claimed_mlp_bf16_f32_v1",
                explicit_bytes: 88,
                arguments: 11,
                lds: 512,
            },
            Self::Residual => Spec {
                bytes: 14_624,
                digest: hex(b"8436c1861dcc14f0346aeecc136e4187c459ac1c0b31778265c88b3d8c63396d"),
                symbol: "ferric_qwen3_tp_peer_tp2_ordered_residual_bf16_v18",
                explicit_bytes: 168,
                arguments: 22,
                lds: 0,
            },
        }
    }
}

const fn hex(text: &[u8; 64]) -> [u8; 32] {
    const fn digit(value: u8) -> u8 {
        match value {
            b'0'..=b'9' => value - b'0',
            b'a'..=b'f' => value - b'a' + 10,
            _ => panic!("hex pin"),
        }
    }
    let mut output = [0; 32];
    let mut i = 0;
    while i < 32 {
        output[i] = digit(text[2 * i]) * 16 + digit(text[2 * i + 1]);
        i += 1;
    }
    output
}

struct Spec {
    bytes: usize,
    digest: [u8; 32],
    symbol: &'static str,
    explicit_bytes: u32,
    arguments: usize,
    lds: u32,
}

/// Owns bytes only after matching the three previously GPU-tested objects.
/// No user-supplied digest, metadata or alternate-image fallback is accepted.
pub(super) struct ReviewedImages {
    objects: [Vec<u8>; 3],
}

impl ReviewedImages {
    pub(super) fn new(prefix: Vec<u8>, mlp: Vec<u8>, residual: Vec<u8>) -> Result<Self> {
        let objects = [prefix, mlp, residual];
        for (kind, object) in ResidentKind::ALL.into_iter().zip(&objects) {
            let spec = kind.spec();
            if object.len() != spec.bytes || <[u8; 32]>::from(Sha256::digest(object)) != spec.digest
            {
                return Err(format!("resident {kind:?} is not the retained P219 object"));
            }
        }
        Ok(Self { objects })
    }
}

pub(super) struct LoadedResidentArtifacts {
    ranks: [[Kernel; 3]; 2],
}

impl LoadedResidentArtifacts {
    pub(super) fn kernel(&self, rank: usize, kind: ResidentKind) -> Result<&Kernel> {
        self.ranks
            .get(rank)
            .map(|kernels| &kernels[kind.index()])
            .ok_or_else(|| "resident artifact rank outside TP2".into())
    }
}

fn argument(kind: ResidentKind, index: usize) -> ExplicitArgumentV1 {
    let pointer = kind != ResidentKind::Residual || index < 20 && index % 2 == 0;
    let (alignment, access) = match kind {
        ResidentKind::Prefix => (
            if matches!(index, 4 | 5 | 13 | 14) {
                4
            } else {
                2
            },
            if index < 7 {
                Access::Read
            } else {
                Access::ReadWrite
            },
        ),
        ResidentKind::Mlp => (
            if matches!(index, 9 | 10) { 4 } else { 2 },
            if index < 5 {
                Access::Read
            } else {
                Access::ReadWrite
            },
        ),
        ResidentKind::Residual => (
            if index < 16 { 4 } else { 2 },
            if index == 18 {
                Access::Write
            } else {
                Access::Read
            },
        ),
    };
    ExplicitArgumentV1 {
        offset: if kind == ResidentKind::Residual && index >= 20 {
            160 + (index as u32 - 20) * 4
        } else {
            index as u32 * 8
        },
        bytes: if kind == ResidentKind::Residual && index >= 20 {
            4
        } else {
            8
        },
        global_buffer: pointer,
        pointee_alignment: pointer.then_some(alignment),
        access: pointer.then_some(access),
    }
}

fn validate_metadata(kind: ResidentKind, metadata: &KernelMetadataV1) -> Result<()> {
    let spec = kind.spec();
    if metadata.object_sha256 != spec.digest
        || metadata.symbol != spec.symbol
        || metadata.kernarg_bytes != spec.explicit_bytes + 256
        || metadata.kernarg_alignment != 8
        || metadata.group_segment_bytes != spec.lds
        || metadata.private_segment_bytes != 0
        || metadata.wavefront_size != 64
        || metadata.implicit_argument_offset != Some(spec.explicit_bytes)
        || metadata.implicit_argument_bytes != 256
        || metadata.explicit_arguments.len() != spec.arguments
    {
        return Err(format!("resident {kind:?} object metadata mismatch"));
    }
    for (index, actual) in metadata.explicit_arguments.iter().enumerate() {
        let expected = argument(kind, index);
        // Some retained objects omit optional pointer attributes. The exact
        // object pin, not an absent attribute, binds their access contract.
        if actual.offset != expected.offset
            || actual.bytes != expected.bytes
            || actual.global_buffer != expected.global_buffer
            || actual.pointee_alignment.is_some()
                && actual.pointee_alignment != expected.pointee_alignment
            || actual.access.is_some() && actual.access != expected.access
        {
            return Err(format!("resident {kind:?} argument {index} mismatch"));
        }
    }
    Ok(())
}

trait Loader {
    type Token;
    fn load(
        &mut self,
        rank: usize,
        kind: ResidentKind,
        object: Vec<u8>,
    ) -> Result<(Self::Token, KernelMetadataV1)>;
}

impl Loader for Group {
    type Token = Kernel;
    fn load(
        &mut self,
        rank: usize,
        kind: ResidentKind,
        object: Vec<u8>,
    ) -> Result<(Kernel, KernelMetadataV1)> {
        let spec = kind.spec();
        let kernel = self.load_kernel(rank, object, spec.digest, spec.symbol.into())?;
        if kernel.rank() != rank {
            return Err("resident artifact native owner mismatch".into());
        }
        let metadata = kernel.metadata().clone();
        Ok((kernel, metadata))
    }
}

fn load_all<L: Loader>(loader: &mut L, images: ReviewedImages) -> Result<[[L::Token; 3]; 2]> {
    let mut ranks = Vec::with_capacity(2);
    for rank in 0..2 {
        let mut kernels = Vec::with_capacity(3);
        for kind in ResidentKind::ALL {
            let (kernel, metadata) =
                loader.load(rank, kind, images.objects[kind.index()].clone())?;
            validate_metadata(kind, &metadata)?;
            kernels.push(kernel);
        }
        ranks.push(
            kernels
                .try_into()
                .map_err(|_| "resident artifact kind count")?,
        );
    }
    ranks
        .try_into()
        .map_err(|_| "resident artifact rank count".into())
}

/// Called only through the private owner, which must make any error terminal.
/// The group remains borrowed throughout; no group/handle is imported from IPC.
pub(super) fn load(group: &mut Group, images: ReviewedImages) -> Result<LoadedResidentArtifacts> {
    Ok(LoadedResidentArtifacts {
        ranks: load_all(group, images)?,
    })
}

#[cfg(test)]
#[path = "resident_artifacts_tests.rs"]
mod tests;
