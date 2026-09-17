use fe2o3_kfd::engineering_wire::{BufferAccessV1, KernelMetadataV1};
use serde::{Deserialize, Serialize};

use crate::{artifact, Result};

pub const SYMBOL: &str = "ferric_gfx950_qwen3_kproj_v1";
pub const WORKGROUP: [u16; 3] = [128, 1, 1];
pub const GRID: [u32; 3] = [1024, 1, 1];
pub const WAVE64_SYMBOL: &str = "ferric_gfx950_qwen3_kproj_wave64_v1";
pub const WAVE64_GRID: [u32; 3] = [65_536, 1, 1];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Variant {
    Scalar,
    Wave64,
}

impl Variant {
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Scalar => SYMBOL,
            Self::Wave64 => WAVE64_SYMBOL,
        }
    }

    pub const fn grid(self) -> [u32; 3] {
        match self {
            Self::Scalar => GRID,
            Self::Wave64 => WAVE64_GRID,
        }
    }

    const fn max_workgroups(self) -> [u32; 3] {
        match self {
            Self::Scalar => [8, 1, 1],
            Self::Wave64 => [512, 1, 1],
        }
    }
}

pub fn variant(metadata: &KernelMetadataV1) -> Result<Variant> {
    match metadata.symbol.as_str() {
        SYMBOL => Ok(Variant::Scalar),
        WAVE64_SYMBOL => Ok(Variant::Wave64),
        _ => Err("unknown fixed Qwen3 projection symbol".into()),
    }
}

pub const LENGTHS: [u64; 3] = [1024, 1_048_576, 1024];
pub const BYTES: [usize; 3] = [2048, 2_097_152, 4096];

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeclaredAbi {
    basis: String,
    workgroup: [u16; 3],
    grid_work_items: [u32; 3],
    max_workgroups: [u32; 3],
    slice_lengths: [u64; 3],
    pointer_alignment: [u32; 3],
    pointer_access: [BufferAccessV1; 3],
}

pub fn declared_abi(variant: Variant) -> DeclaredAbi {
    DeclaredAbi {
        basis: "fixed harness and source-declared ABI; not source-to-object authority".into(),
        workgroup: WORKGROUP,
        grid_work_items: variant.grid(),
        max_workgroups: variant.max_workgroups(),
        slice_lengths: LENGTHS,
        pointer_alignment: [2, 2, 4],
        pointer_access: [
            BufferAccessV1::Read,
            BufferAccessV1::Read,
            BufferAccessV1::Write,
        ],
    }
}

pub fn inspect(object: &[u8], variant: Variant) -> Result<artifact::Inspection> {
    let inspected = crate::object::inspect(object, variant.symbol())?;
    validate_launch(&inspected.launch, variant)?;
    for (index, qualifier) in inspected.qualifiers.iter().enumerate() {
        qualifier.validate(index)?;
    }
    validate_abi(&inspected.metadata)?;
    Ok(inspected)
}

fn validate_launch(launch: &artifact::ObservedLaunchMetadata, variant: Variant) -> Result<()> {
    if launch.required_workgroup_size != Some(WORKGROUP.map(u32::from))
        || launch.max_flat_workgroup_size != 128
        || launch
            .max_workgroups
            .into_iter()
            .zip(variant.max_workgroups())
            .any(|(observed, expected)| observed.is_some_and(|value| value != expected))
        || launch.cluster_dims.is_some()
    {
        return Err("Qwen3 key projection workgroup contract mismatch".into());
    }
    Ok(())
}

pub fn validate_abi(metadata: &KernelMetadataV1) -> Result<()> {
    variant(metadata)?;
    if metadata.kernarg_alignment != 8
        || metadata.group_segment_bytes != 0
        || metadata.private_segment_bytes != 0
        || metadata.wavefront_size != 64
        || metadata.explicit_arguments.len() != 6
        || !matches!(
            (
                metadata.kernarg_bytes,
                metadata.implicit_argument_offset,
                metadata.implicit_argument_bytes
            ),
            (48, None, 0) | (304, Some(48), 256)
        )
    {
        return Err("Qwen3 key projection resource or argument contract mismatch".into());
    }
    for (index, argument) in metadata.explicit_arguments.iter().enumerate() {
        let pointer = index.is_multiple_of(2);
        let access = if index == 4 {
            BufferAccessV1::Write
        } else {
            BufferAccessV1::Read
        };
        let alignment = if index == 4 { 4 } else { 2 };
        if argument.offset != u32::try_from(index * 8).expect("six arguments")
            || argument.bytes != 8
            || argument.global_buffer != pointer
            || (pointer
                && argument
                    .pointee_alignment
                    .is_some_and(|value| value != alignment))
            || (pointer && argument.access.is_some_and(|value| value != access))
            || (!pointer && (argument.pointee_alignment.is_some() || argument.access.is_some()))
        {
            return Err("Qwen3 key projection explicit argument ABI mismatch".into());
        }
    }
    Ok(())
}

pub fn kernarg(metadata: &KernelMetadataV1) -> Result<Vec<u8>> {
    validate_abi(metadata)?;
    let mut bytes = vec![0; usize::try_from(metadata.kernarg_bytes).expect("bounded ABI")];
    for (index, length) in LENGTHS.iter().enumerate() {
        let offset = index * 16 + 8;
        bytes[offset..offset + 8].copy_from_slice(&length.to_le_bytes());
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_metadata_keeps_absence_and_rejects_every_contradictory_field() {
        let exact = || artifact::ObservedLaunchMetadata {
            required_workgroup_size: Some([128, 1, 1]),
            max_flat_workgroup_size: 128,
            max_workgroups: [Some(8), Some(1), Some(1)],
            cluster_dims: None,
        };
        validate_launch(&exact(), Variant::Scalar).unwrap();
        let mut absent = exact();
        absent.max_workgroups = [None; 3];
        validate_launch(&absent, Variant::Scalar).unwrap();
        assert_eq!(absent.max_workgroups, [None; 3]);
        for mutation in 0..7 {
            let mut changed = exact();
            match mutation {
                0 => changed.required_workgroup_size = None,
                1 => changed.required_workgroup_size = Some([64, 1, 1]),
                2 => changed.max_flat_workgroup_size = 256,
                3 => changed.max_workgroups[0] = Some(2),
                4 => changed.max_workgroups[1] = Some(2),
                5 => changed.max_workgroups[2] = Some(2),
                _ => changed.cluster_dims = Some([1, 1, 1]),
            }
            assert!(
                validate_launch(&changed, Variant::Scalar).is_err(),
                "mutation {mutation}"
            );
        }
        assert!(validate_launch(&exact(), Variant::Wave64).is_err());
        let mut wave = exact();
        wave.max_workgroups = [Some(512), Some(1), Some(1)];
        validate_launch(&wave, Variant::Wave64).unwrap();
        assert!(validate_launch(&wave, Variant::Scalar).is_err());
        validate_launch(&absent, Variant::Wave64).unwrap();
    }
}
