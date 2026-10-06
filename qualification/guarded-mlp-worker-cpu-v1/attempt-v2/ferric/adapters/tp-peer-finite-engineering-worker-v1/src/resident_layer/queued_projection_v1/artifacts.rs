//! Exact retained V3 GEMVs only; unrelated entries cannot be selected.

use super::{Result, Stage, bindings};
use fe2o3_kfd::{
    Gfx950EngineeringPeerGroupV1 as Group, Gfx950EngineeringPeerKernelV1 as Kernel,
    engineering_wire::KernelMetadataV1,
};
use sha2::{Digest, Sha256};

const WAVE_SHA: [u8; 32] = [
    0x11, 0xf5, 0x3c, 0xfe, 0x2d, 0x18, 0x66, 0x8f, 0x19, 0x1a, 0xf9, 0xf9, 0x98, 0xf4, 0x86, 0x27,
    0x21, 0x1a, 0xa0, 0x9a, 0x36, 0xd5, 0x48, 0x3e, 0x51, 0x69, 0x46, 0x03, 0x2a, 0xd5, 0x4d, 0x8e,
];
const KINDS: [Stage; 2] = [Stage::Query, Stage::Output];
fn slot(stage: Stage) -> usize {
    usize::from(stage == Stage::Output)
}
fn symbol(stage: Stage) -> &'static str {
    if stage == Stage::Output {
        "ferric_qwen3_tp_wave_gemv_partial_f32_v3"
    } else {
        "ferric_qwen3_tp_wave_gemv_bf16_v3"
    }
}

pub(crate) struct ReviewedImage {
    wave: Vec<u8>,
}
impl ReviewedImage {
    pub(crate) fn new(wave: Vec<u8>) -> Result<Self> {
        if wave.len() != 112872 || <[u8; 32]>::from(Sha256::digest(&wave)) != WAVE_SHA {
            return Err("queued projection object differs from exact retained V3 image".into());
        }
        Ok(Self { wave })
    }
}
pub(crate) struct LoadedArtifacts {
    ranks: [[Kernel; 2]; 2],
}
impl LoadedArtifacts {
    pub(super) fn kernel(&self, rank: usize, stage: Stage) -> Result<&Kernel> {
        self.ranks
            .get(rank)
            .map(|row| &row[slot(stage)])
            .ok_or_else(|| "queued projection artifact rank".into())
    }
}

fn validate_metadata(stage: Stage, value: &KernelMetadataV1) -> Result<()> {
    if value.object_sha256 != WAVE_SHA
        || value.symbol != symbol(stage)
        || value.kernarg_bytes != 328
        || value.kernarg_alignment != 8
        || value.group_segment_bytes != 0
        || value.private_segment_bytes != 0
        || value.wavefront_size != 64
        || value.implicit_argument_offset != Some(72)
        || value.implicit_argument_bytes != 256
        || value.explicit_arguments.len() != 11
    {
        return Err("queued projection exact retained metadata/resources".into());
    }
    let roots = bindings(stage);
    for (index, arg) in value.explicit_arguments.iter().enumerate() {
        let pair = index < 6;
        let pointer = pair && index % 2 == 0;
        let offset = if pair {
            index as u32 * 8
        } else {
            48 + (index as u32 - 6) * 4
        };
        let alignment = if stage == Stage::Output && index == 4 {
            4
        } else {
            2
        };
        if arg.offset != offset
            || arg.bytes != if pair { 8 } else { 4 }
            || arg.global_buffer != pointer
            || pointer
                && (arg.pointee_alignment.is_some_and(|v| v != alignment)
                    || arg.access.is_some_and(|v| v != roots[index / 2].access))
            || !pointer && (arg.pointee_alignment.is_some() || arg.access.is_some())
        {
            return Err("queued projection pointer/length/scalar roster".into());
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
        let kernel = self.load_kernel(rank, bytes, WAVE_SHA, symbol(stage).into())?;
        if kernel.rank() != rank {
            return Err("queued projection loaded owner mismatch".into());
        }
        let metadata = kernel.metadata().clone();
        Ok((kernel, metadata))
    }
}
fn load_all<L: Loader>(loader: &mut L, image: ReviewedImage) -> Result<[[L::Token; 2]; 2]> {
    let mut ranks = Vec::with_capacity(2);
    for rank in 0..2 {
        let mut kernels = Vec::with_capacity(2);
        for stage in KINDS {
            let (kernel, metadata) = loader.load(rank, stage, image.wave.clone())?;
            validate_metadata(stage, &metadata)?;
            kernels.push(kernel);
        }
        ranks.push(
            kernels
                .try_into()
                .map_err(|_| "queued projection artifact stage census")?,
        );
    }
    ranks
        .try_into()
        .map_err(|_| "queued projection artifact rank census".into())
}
/// Only the private owner loads artifacts. Any failure makes that owner
/// terminal; no partial artifact set is returned. This pin is historical machine
/// identity, not a fresh compiler proof or production device admission.
pub(crate) fn load(group: &mut Group, image: ReviewedImage) -> Result<LoadedArtifacts> {
    Ok(LoadedArtifacts {
        ranks: load_all(group, image)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use fe2o3_kfd::engineering_wire::{BufferAccessV1, ExplicitArgumentV1};
    fn metadata(stage: Stage, attributes: bool) -> KernelMetadataV1 {
        KernelMetadataV1 {
            symbol: symbol(stage).into(),
            object_sha256: WAVE_SHA,
            kernarg_bytes: 328,
            kernarg_alignment: 8,
            group_segment_bytes: 0,
            private_segment_bytes: 0,
            wavefront_size: 64,
            implicit_argument_offset: Some(72),
            implicit_argument_bytes: 256,
            explicit_arguments: (0..11)
                .map(|index| {
                    let pair = index < 6;
                    let pointer = pair && index % 2 == 0;
                    ExplicitArgumentV1 {
                        offset: if pair {
                            index * 8
                        } else {
                            48 + (index - 6) * 4
                        },
                        bytes: if pair { 8 } else { 4 },
                        global_buffer: pointer,
                        pointee_alignment: if attributes && pointer {
                            Some(if stage == Stage::Output && index == 4 {
                                4
                            } else {
                                2
                            })
                        } else {
                            None
                        },
                        access: if attributes && pointer {
                            Some(bindings(stage)[index as usize / 2].access)
                        } else {
                            None
                        },
                    }
                })
                .collect(),
        }
    }
    #[test]
    fn closed_metadata_rejects_resource_pointer_and_scalar_drift() {
        for stage in Stage::ALL {
            for attributes in [false, true] {
                let good = metadata(stage, attributes);
                validate_metadata(stage, &good).unwrap();
                for change in 0..11 {
                    let mut bad = good.clone();
                    match change {
                        0 => bad.object_sha256[0] ^= 1,
                        1 => bad.symbol.push('_'),
                        2 => bad.kernarg_bytes += 4,
                        3 => bad.kernarg_alignment = 4,
                        4 => bad.group_segment_bytes = 4,
                        5 => bad.private_segment_bytes = 4,
                        6 => bad.wavefront_size = 32,
                        7 => bad.implicit_argument_offset = Some(68),
                        8 => bad.implicit_argument_bytes = 0,
                        9 => {
                            bad.explicit_arguments.pop();
                        }
                        _ => bad
                            .explicit_arguments
                            .push(bad.explicit_arguments[0].clone()),
                    }
                    assert!(validate_metadata(stage, &bad).is_err());
                }
                for index in 0..11 {
                    for change in 0..5 {
                        let mut bad = good.clone();
                        let arg = &mut bad.explicit_arguments[index];
                        match change {
                            0 => arg.offset += 4,
                            1 => arg.bytes += 4,
                            2 => arg.global_buffer = !arg.global_buffer,
                            3 => arg.pointee_alignment = Some(16),
                            _ => arg.access = Some(BufferAccessV1::ReadWrite),
                        }
                        assert!(validate_metadata(stage, &bad).is_err());
                    }
                }
            }
        }
    }
    #[test]
    fn intake_and_selected_entries_have_no_fallback() {
        assert!(ReviewedImage::new(vec![]).is_err());
        assert!(ReviewedImage::new(vec![0; 112872]).is_err());
        assert_eq!(symbol(Stage::Query), symbol(Stage::Key));
        assert_eq!(symbol(Stage::Key), symbol(Stage::Value));
        assert_ne!(symbol(Stage::Query), symbol(Stage::Output));
        assert_eq!(Stage::ALL.map(slot), [0, 0, 0, 1]);
    }
    struct Fake {
        calls: Vec<(usize, Stage)>,
        fail: Option<usize>,
        bad: bool,
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
            if self.fail == Some(index) && !self.bad {
                return Err("injected load".into());
            }
            let mut value = metadata(stage, false);
            if self.fail == Some(index) {
                value.private_segment_bytes = 1;
            }
            Ok(((rank, stage), value))
        }
    }
    #[test]
    fn four_actual_loads_are_all_or_no_published_artifacts() {
        for fail in [None, Some(0), Some(1), Some(2), Some(3)] {
            for bad in [false, true] {
                let mut fake = Fake {
                    calls: vec![],
                    fail,
                    bad,
                };
                // Private inert fixture exercises loader control, never image intake.
                let result = load_all(&mut fake, ReviewedImage { wave: vec![0] });
                assert_eq!(result.is_err(), fail.is_some());
                assert_eq!(fake.calls.len(), fail.map_or(4, |i| i + 1));
                if let Ok(value) = result {
                    assert_eq!(
                        value,
                        [
                            [(0, Stage::Query), (0, Stage::Output)],
                            [(1, Stage::Query), (1, Stage::Output)]
                        ]
                    );
                }
            }
        }
    }
    #[test]
    #[ignore = "requires exact retained V3 bytes; never opens a GPU"]
    fn actual_retained_wave_object_passes_closed_intake() {
        let path = std::env::var_os("FERRIC_QUEUED_PROJECTION_WAVE_V3_HSACO")
            .expect("mandatory retained V3 path");
        ReviewedImage::new(std::fs::read(path).expect("retained V3 read")).unwrap();
    }
}
