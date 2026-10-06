//! Closed retained P218 objects, not fresh source-image admission.

use super::{Result, Stage, bindings};
use fe2o3_kfd::{
    Gfx950EngineeringPeerGroupV1 as Group, Gfx950EngineeringPeerKernelV1 as Kernel,
    engineering_wire::KernelMetadataV1,
};
use sha2::{Digest, Sha256};

const NORM_SHA: [u8; 32] = hex(b"c33882db1afcd8eb26ec02bc43ea2144af322d29bf4e0e921e03ea74adc55af6");
const WAVE_SHA: [u8; 32] = hex(b"11f53cfe2d18668f191af9f998f48627211aa09a36d5483e516946032ad54d8e");
const KINDS: [Stage; 4] = [Stage::Norm, Stage::Gate, Stage::Activation, Stage::Down];

const fn hex(text: &[u8; 64]) -> [u8; 32] {
    const fn digit(byte: u8) -> u8 {
        match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            _ => panic!("hex pin"),
        }
    }
    let mut result = [0; 32];
    let mut i = 0;
    while i < 32 {
        result[i] = digit(text[i * 2]) * 16 + digit(text[i * 2 + 1]);
        i += 1;
    }
    result
}

fn symbol(stage: Stage) -> &'static str {
    match stage {
        Stage::Norm => "ferric_qwen3_tp_batch32_wave_rmsnorm_bf16_v15",
        Stage::Gate | Stage::Up => "ferric_qwen3_tp_wave_gemv_bf16_v3",
        Stage::Activation => "ferric_qwen3_tp_batch_swiglu_bf16_f32_v2",
        Stage::Down => "ferric_qwen3_tp_wave_gemv_partial_f32_v3",
    }
}

fn slot(stage: Stage) -> usize {
    match stage {
        Stage::Norm => 0,
        Stage::Gate | Stage::Up => 1,
        Stage::Activation => 2,
        Stage::Down => 3,
    }
}

fn digest(stage: Stage) -> [u8; 32] {
    if stage == Stage::Norm {
        NORM_SHA
    } else {
        WAVE_SHA
    }
}

/// Exact historical machine objects only. The optional metadata attributes do
/// not substitute for the fixed object pins and artifact-specific ISA review.
pub(crate) struct ReviewedImages {
    norm: Vec<u8>,
    wave: Vec<u8>,
}

impl ReviewedImages {
    pub(crate) fn new(norm: Vec<u8>, wave: Vec<u8>) -> Result<Self> {
        for (bytes, size, expected) in [(&norm, 11_432, NORM_SHA), (&wave, 112_872, WAVE_SHA)] {
            if bytes.len() != size || <[u8; 32]>::from(Sha256::digest(bytes)) != expected {
                return Err("queued MLP object differs from retained P218 bytes".into());
            }
        }
        Ok(Self { norm, wave })
    }
}

pub(crate) struct LoadedArtifacts {
    ranks: [[Kernel; 4]; 2],
}

impl LoadedArtifacts {
    pub(super) fn kernel(&self, rank: usize, stage: Stage) -> Result<&Kernel> {
        self.ranks
            .get(rank)
            .map(|kernels| &kernels[slot(stage)])
            .ok_or_else(|| "queued MLP artifact rank outside TP2".into())
    }
}

fn validate_metadata(stage: Stage, metadata: &KernelMetadataV1) -> Result<()> {
    if metadata.object_sha256 != digest(stage)
        || metadata.symbol != symbol(stage)
        || metadata.kernarg_bytes != stage.total() as u32
        || metadata.kernarg_alignment != 8
        || metadata.group_segment_bytes != 0
        || metadata.private_segment_bytes != 0
        || metadata.wavefront_size != 64
        || metadata.implicit_argument_offset != Some(stage.hidden())
        || metadata.implicit_argument_bytes != 256
        || metadata.explicit_arguments.len() != stage.slices() * 2 + stage.scalars().len()
    {
        return Err("queued MLP retained entry ABI/resources mismatch".into());
    }
    let roots = bindings(stage);
    for (index, argument) in metadata.explicit_arguments.iter().enumerate() {
        let pair = index < stage.slices() * 2;
        let pointer = pair && index % 2 == 0;
        let offset = if pair {
            index as u32 * 8
        } else {
            stage.slices() as u32 * 16 + (index - stage.slices() * 2) as u32 * 4
        };
        let alignment = if stage == Stage::Down && index == 4 {
            4
        } else {
            2
        };
        if argument.offset != offset
            || argument.bytes != if pair { 8 } else { 4 }
            || argument.global_buffer != pointer
            || pointer
                && (argument
                    .pointee_alignment
                    .is_some_and(|value| value != alignment)
                    || argument
                        .access
                        .is_some_and(|value| value != roots[index / 2].access))
            || !pointer && (argument.pointee_alignment.is_some() || argument.access.is_some())
        {
            return Err("queued MLP pointer/length/scalar roster mismatch".into());
        }
    }
    Ok(())
}

trait Loader {
    type Token;
    fn load(
        &mut self,
        rank: usize,
        stage: Stage,
        bytes: Vec<u8>,
    ) -> Result<(Self::Token, KernelMetadataV1)>;
}

impl Loader for Group {
    type Token = Kernel;
    fn load(
        &mut self,
        rank: usize,
        stage: Stage,
        bytes: Vec<u8>,
    ) -> Result<(Kernel, KernelMetadataV1)> {
        let kernel = self.load_kernel(rank, bytes, digest(stage), symbol(stage).into())?;
        if kernel.rank() != rank {
            return Err("queued MLP artifact native owner mismatch".into());
        }
        let metadata = kernel.metadata().clone();
        Ok((kernel, metadata))
    }
}

fn load_all<L: Loader>(loader: &mut L, images: ReviewedImages) -> Result<[[L::Token; 4]; 2]> {
    let mut ranks = Vec::with_capacity(2);
    for rank in 0..2 {
        let mut kernels = Vec::with_capacity(4);
        for stage in KINDS {
            let bytes = if stage == Stage::Norm {
                &images.norm
            } else {
                &images.wave
            };
            let (kernel, metadata) = loader.load(rank, stage, bytes.clone())?;
            validate_metadata(stage, &metadata)?;
            kernels.push(kernel);
        }
        ranks.push(
            kernels
                .try_into()
                .map_err(|_| "queued MLP artifact kind count")?,
        );
    }
    ranks
        .try_into()
        .map_err(|_| "queued MLP artifact rank count".into())
}

/// Owner-only setup. Any load/metadata error must make the enclosing owner
/// terminal; this function never returns a partially usable artifact set.
pub(crate) fn load(group: &mut Group, images: ReviewedImages) -> Result<LoadedArtifacts> {
    Ok(LoadedArtifacts {
        ranks: load_all(group, images)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use fe2o3_kfd::engineering_wire::ExplicitArgumentV1;

    fn metadata(stage: Stage, attributes: bool) -> KernelMetadataV1 {
        let roots = bindings(stage);
        KernelMetadataV1 {
            object_sha256: digest(stage),
            symbol: symbol(stage).into(),
            kernarg_bytes: stage.total() as u32,
            kernarg_alignment: 8,
            group_segment_bytes: 0,
            private_segment_bytes: 0,
            wavefront_size: 64,
            implicit_argument_offset: Some(stage.hidden()),
            implicit_argument_bytes: 256,
            explicit_arguments: (0..stage.slices() * 2 + stage.scalars().len())
                .map(|index| {
                    let pair = index < stage.slices() * 2;
                    let pointer = pair && index % 2 == 0;
                    ExplicitArgumentV1 {
                        offset: if pair {
                            index as u32 * 8
                        } else {
                            stage.slices() as u32 * 16 + (index - stage.slices() * 2) as u32 * 4
                        },
                        bytes: if pair { 8 } else { 4 },
                        global_buffer: pointer,
                        pointee_alignment: if attributes && pointer {
                            Some(if stage == Stage::Down && index == 4 {
                                4
                            } else {
                                2
                            })
                        } else {
                            None
                        },
                        access: if attributes && pointer {
                            Some(roots[index / 2].access)
                        } else {
                            None
                        },
                    }
                })
                .collect(),
        }
    }

    #[test]
    fn each_retained_entry_has_exact_scalar_and_pointer_metadata() {
        for stage in Stage::ALL {
            for attributes in [false, true] {
                let good = metadata(stage, attributes);
                validate_metadata(stage, &good).unwrap();
                for mutation in 0..11 {
                    let mut bad = good.clone();
                    match mutation {
                        0 => bad.object_sha256[0] ^= 1,
                        1 => bad.symbol.push('_'),
                        2 => bad.kernarg_bytes += 8,
                        3 => bad.kernarg_alignment = 4,
                        4 => bad.group_segment_bytes = 512,
                        5 => bad.private_segment_bytes = 4,
                        6 => bad.wavefront_size = 32,
                        7 => bad.implicit_argument_offset = Some(0),
                        8 => bad.implicit_argument_bytes -= 1,
                        9 => {
                            bad.explicit_arguments.pop();
                        }
                        _ => bad
                            .explicit_arguments
                            .push(bad.explicit_arguments[0].clone()),
                    }
                    assert!(
                        validate_metadata(stage, &bad).is_err(),
                        "{stage:?} mutation {mutation}"
                    );
                }
                for index in 0..good.explicit_arguments.len() {
                    for mutation in 0..5 {
                        let mut bad = good.clone();
                        let arg = &mut bad.explicit_arguments[index];
                        match mutation {
                            0 => arg.offset += 4,
                            1 => arg.bytes += 4,
                            2 => arg.global_buffer = !arg.global_buffer,
                            3 => arg.pointee_alignment = Some(16),
                            _ => arg.access = Some(super::super::Access::ReadWrite),
                        }
                        assert!(
                            validate_metadata(stage, &bad).is_err(),
                            "{stage:?}/{index}/{mutation}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn exact_object_intake_has_no_alternate_hash_or_size_fallback() {
        assert!(ReviewedImages::new(vec![], vec![]).is_err());
        assert!(ReviewedImages::new(vec![0; 11_432], vec![0; 112_872]).is_err());
        assert_eq!(symbol(Stage::Gate), symbol(Stage::Up));
        assert_eq!(slot(Stage::Gate), slot(Stage::Up));
        assert_ne!(NORM_SHA, WAVE_SHA);
    }

    #[test]
    #[ignore = "requires the two exact retained P218 machine objects; no GPU open"]
    fn actual_retained_norm_and_wave_objects_pass_closed_intake() {
        let norm = std::env::var_os("FERRIC_QUEUED_MLP_NORM_V15_HSACO")
            .expect("mandatory V15 object path");
        let wave =
            std::env::var_os("FERRIC_QUEUED_MLP_WAVE_V3_HSACO").expect("mandatory V3 object path");
        ReviewedImages::new(
            std::fs::read(norm).expect("read norm object"),
            std::fs::read(wave).expect("read wave object"),
        )
        .unwrap();
    }

    struct Fake {
        calls: Vec<(usize, Stage)>,
        fail: Option<usize>,
        bad: Option<usize>,
    }
    impl Loader for Fake {
        type Token = (usize, Stage);
        fn load(
            &mut self,
            rank: usize,
            stage: Stage,
            _: Vec<u8>,
        ) -> Result<(Self::Token, KernelMetadataV1)> {
            let index = self.calls.len();
            self.calls.push((rank, stage));
            if self.fail == Some(index) {
                return Err("injected artifact load failure".into());
            }
            let mut result = metadata(stage, false);
            if self.bad == Some(index) {
                result.private_segment_bytes = 1;
            }
            Ok(((rank, stage), result))
        }
    }

    #[test]
    fn both_ranks_load_all_four_entries_and_never_publish_partial_set() {
        for failure in [None, Some(0), Some(3), Some(4), Some(7)] {
            for bad in [false, true] {
                let mut fake = Fake {
                    calls: vec![],
                    fail: if bad { None } else { failure },
                    bad: if bad { failure } else { None },
                };
                // Private fabricated objects test loader plumbing only, not intake.
                let result = load_all(
                    &mut fake,
                    ReviewedImages {
                        norm: vec![],
                        wave: vec![],
                    },
                );
                match failure {
                    None => {
                        assert_eq!(
                            result.unwrap(),
                            [KINDS.map(|stage| (0, stage)), KINDS.map(|stage| (1, stage))]
                        );
                        assert_eq!(fake.calls.len(), 8);
                    }
                    Some(index) => {
                        assert!(result.is_err());
                        assert_eq!(fake.calls.len(), index + 1);
                    }
                }
            }
        }
    }
}
