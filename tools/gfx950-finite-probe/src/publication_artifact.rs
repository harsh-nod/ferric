use fe2o3_kfd::engineering_wire::{BufferAccessV1, KernelMetadataV1};
use serde::{Deserialize, Serialize};

use crate::artifact::{self, ObservedArgumentQualifiers, ObservedLaunchMetadata};
use crate::Result;

pub const SYMBOL: &str = "ferric_gfx950_static_publication_v1";
pub const WORKGROUP: [u16; 3] = [128, 1, 1];
pub const GRID: [u32; 3] = [256, 1, 1];
pub const BYTES: [usize; 5] = [512, 512, 512, 1024, 1024];
pub const GUARD: usize = 64;
pub const REPETITIONS: u32 = 8;
pub const ACCESS: [BufferAccessV1; 5] = [
    BufferAccessV1::ReadWrite,
    BufferAccessV1::ReadWrite,
    BufferAccessV1::ReadWrite,
    BufferAccessV1::Write,
    BufferAccessV1::Write,
];

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Pattern {
    Zero,
    Request,
    Ready,
    Max,
    Mixed,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Shape {
    Valid,
    PayloadShort,
    FlagsShort,
    PayloadEmpty,
    FlagsEmpty,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub schema: String,
    pub pattern: Pattern,
    pub repetition: u32,
    pub shape: Shape,
}

impl Case {
    pub fn validate(&self) -> Result<()> {
        if self.schema != "ferric-static-publication-case-v1"
            || self.repetition >= REPETITIONS
            || (self.shape != Shape::Valid
                && (self.pattern != Pattern::Ready || self.repetition != 0))
        {
            return Err("publication case is outside the frozen matrix".into());
        }
        Ok(())
    }

    pub fn lengths(&self) -> [u64; 5] {
        let mut lengths = [128, 128, 128, 256, 256];
        match self.shape {
            Shape::Valid => {}
            Shape::PayloadShort => lengths[0] = 127,
            Shape::FlagsShort => lengths[1] = 127,
            Shape::PayloadEmpty => lengths[0] = 0,
            Shape::FlagsEmpty => lengths[1] = 0,
        }
        lengths
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub schema: String,
    pub source_sha256: String,
    pub object_sha256: String,
    pub metadata: KernelMetadataV1,
    observed_launch_metadata: ObservedLaunchMetadata,
    observed_argument_qualifiers: Vec<ObservedArgumentQualifiers>,
    engineering_contract: String,
    workgroup: [u16; 3],
    grid: [u32; 3],
    allocation_bytes: [usize; 5],
    pointer_access: [BufferAccessV1; 5],
}

pub fn validate_metadata(metadata: &KernelMetadataV1) -> Result<()> {
    if metadata.symbol != SYMBOL
        || metadata.kernarg_alignment != 8
        || metadata.group_segment_bytes != 0
        || metadata.private_segment_bytes != 0
        || metadata.wavefront_size != 64
        || metadata.explicit_arguments.len() != 10
        || !matches!(
            (
                metadata.kernarg_bytes,
                metadata.implicit_argument_offset,
                metadata.implicit_argument_bytes
            ),
            (80, None, 0) | (336, Some(80), 256)
        )
    {
        return Err("publication fixed metadata mismatch".into());
    }
    for (index, argument) in metadata.explicit_arguments.iter().enumerate() {
        let pointer = index.is_multiple_of(2);
        if argument.offset != u32::try_from(index * 8).expect("ten arguments")
            || argument.bytes != 8
            || argument.global_buffer != pointer
            || (pointer && argument.pointee_alignment.is_some_and(|value| value != 4))
            || (pointer
                && argument
                    .access
                    .is_some_and(|value| value != ACCESS[index / 2]))
            || (!pointer && (argument.pointee_alignment.is_some() || argument.access.is_some()))
        {
            return Err("publication explicit argument mismatch".into());
        }
    }
    Ok(())
}

pub fn validate_qualifier(index: usize, value: &ObservedArgumentQualifiers) -> Result<()> {
    if index >= 10 {
        return Err("publication qualifier count mismatch".into());
    }
    let pointer = index.is_multiple_of(2);
    if value.is_volatile == Some(true)
        || value.is_pipe == Some(true)
        || value.is_const == Some(true)
        || (pointer
            && value.actual_access.is_some_and(|access| {
                access != ACCESS[index / 2] && !(index == 4 && access == BufferAccessV1::Read)
            }))
        || (pointer
            && value
                .is_restrict
                .is_some_and(|restricted| restricted != (index != 2)))
        || (!pointer && (value.actual_access.is_some() || value.is_restrict == Some(true)))
    {
        return Err("publication qualifiers contradict owning/shared-atomic ABI".into());
    }
    Ok(())
}

pub fn inspect(object: &[u8], source: &[u8]) -> Result<Record> {
    let observed = crate::object::inspect(object, SYMBOL)?;
    artifact::validate_launch(
        observed.launch.required_workgroup_size,
        observed.launch.max_flat_workgroup_size,
        observed.launch.max_workgroups,
        observed.launch.cluster_dims,
    )?;
    validate_metadata(&observed.metadata)?;
    if observed.qualifiers.len() != 10 {
        return Err("publication qualifier count mismatch".into());
    }
    for (index, qualifier) in observed.qualifiers.iter().enumerate() {
        validate_qualifier(index, qualifier)?;
    }
    Ok(Record {
        schema: "ferric-static-publication-artifact-v1".into(),
        source_sha256: artifact::hex(&artifact::digest(source)),
        object_sha256: artifact::hex(&artifact::digest(object)),
        metadata: observed.metadata, observed_launch_metadata: observed.launch,
        observed_argument_qualifiers: observed.qualifiers,
        engineering_contract: "fixed source fixture, not source-to-object or runtime authority; PUBLIC allocation does not authenticate System atomic eligibility".into(),
        workgroup: WORKGROUP, grid: GRID, allocation_bytes: BYTES, pointer_access: ACCESS,
    })
}

pub fn kernarg(metadata: &KernelMetadataV1, case: &Case) -> Result<Vec<u8>> {
    validate_metadata(metadata)?;
    case.validate()?;
    let mut bytes = vec![0; usize::try_from(metadata.kernarg_bytes).expect("bounded ABI")];
    for (index, length) in case.lengths().into_iter().enumerate() {
        bytes[index * 16 + 8..index * 16 + 16].copy_from_slice(&length.to_le_bytes());
    }
    Ok(bytes)
}

pub fn validate_inputs(case: &Case, inputs: &[u8], flags: &[u8]) -> Result<()> {
    case.validate()?;
    if inputs.len() != BYTES[2] || flags.len() != BYTES[1] {
        return Err("publication inputs require 128 f32 and 128 u32 words".into());
    }
    for bytes in inputs.chunks_exact(4) {
        let value = f32::from_le_bytes(bytes.try_into().expect("four bytes"));
        if !value.is_finite() || value == 0.0 {
            return Err("publication tagged input domain is finite and nonzero".into());
        }
    }
    for (cell, bytes) in flags.chunks_exact(4).enumerate() {
        let word = u32::from_le_bytes(bytes.try_into().expect("four bytes"));
        let expected = match case.pattern {
            Pattern::Zero => 0,
            Pattern::Request => 1,
            Pattern::Ready => 2,
            Pattern::Max => u32::MAX,
            Pattern::Mixed => {
                [0, 1, 2, u32::MAX, 0xa5a5_5a5a, 3][(cell + case.repetition as usize) % 6]
            }
        };
        if word != expected {
            return Err("initial flags differ from declared case".into());
        }
    }
    Ok(())
}
