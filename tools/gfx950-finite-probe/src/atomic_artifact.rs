use fe2o3_kfd::engineering_wire::{BufferAccessV1, KernelMetadataV1};
use serde::{Deserialize, Serialize};

use crate::artifact::{self, ObservedArgumentQualifiers, ObservedLaunchMetadata};
use crate::Result;

pub const SYMBOL: &str = "ferric_gfx950_atomic_channel_v1";
pub const WORKGROUP: [u16; 3] = [128, 1, 1];
pub const GRID: [u32; 3] = [256, 1, 1];
pub const WORDS: usize = 256;
pub const BYTES: usize = WORDS * 4;
pub const GUARD: usize = 64;
pub const ACCESS: [BufferAccessV1; 3] = [
    BufferAccessV1::ReadWrite,
    BufferAccessV1::ReadWrite,
    BufferAccessV1::Write,
];

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct DeclaredAbi {
    basis: String,
    workgroup: [u16; 3],
    grid_work_items: [u32; 3],
    explicit_kernarg_bytes: u32,
    slice_lengths: [u64; 3],
    buffer_roles: [String; 3],
    pointer_access: [BufferAccessV1; 3],
    distinct_guarded_allocations: bool,
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
    source_declared_abi: DeclaredAbi,
}

pub fn validate_metadata(metadata: &KernelMetadataV1) -> Result<()> {
    if metadata.symbol != SYMBOL
        || metadata.kernarg_alignment != 8
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
        return Err("atomic channel fixed resources or kernarg layout mismatch".into());
    }
    for (index, argument) in metadata.explicit_arguments.iter().enumerate() {
        let pointer = index.is_multiple_of(2);
        if argument.offset != u32::try_from(index * 8).expect("six arguments")
            || argument.bytes != 8
            || argument.global_buffer != pointer
            || (pointer && argument.pointee_alignment.is_some_and(|value| value != 4))
            || (pointer
                && argument
                    .access
                    .is_some_and(|value| value != ACCESS[index / 2]))
            || (!pointer && (argument.pointee_alignment.is_some() || argument.access.is_some()))
        {
            return Err("atomic channel explicit argument mismatch".into());
        }
    }
    Ok(())
}

pub fn validate_qualifier(index: usize, qualifier: &ObservedArgumentQualifiers) -> Result<()> {
    if index >= 6 {
        return Err("atomic channel qualifier index exceeds fixed ABI".into());
    }
    let pointer = index.is_multiple_of(2);
    if qualifier.is_volatile == Some(true)
        || qualifier.is_pipe == Some(true)
        || (pointer
            && qualifier.actual_access.is_some_and(|value| {
                value != ACCESS[index / 2] && !(index == 2 && value == BufferAccessV1::Read)
            }))
        || (pointer && qualifier.is_const == Some(true))
        || (pointer
            && qualifier
                .is_restrict
                .is_some_and(|value| value != (index != 0)))
        || (!pointer
            && (qualifier.actual_access.is_some()
                || qualifier.is_const == Some(true)
                || qualifier.is_restrict == Some(true)))
    {
        return Err("atomic channel qualifiers contradict the shared-atomic ABI".into());
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
    if observed.qualifiers.len() != 6 {
        return Err("atomic channel qualifier count mismatch".into());
    }
    for (index, qualifier) in observed.qualifiers.iter().enumerate() {
        validate_qualifier(index, qualifier)?;
    }
    Ok(Record {
        schema: "ferric-atomic-channel-artifact-v1".into(),
        source_sha256: artifact::hex(&artifact::digest(source)),
        object_sha256: artifact::hex(&artifact::digest(object)),
        metadata: observed.metadata,
        observed_launch_metadata: observed.launch,
        observed_argument_qualifiers: observed.qualifiers,
        source_declared_abi: DeclaredAbi {
            basis: "fixed engineering fixture; not ELF-derived source-to-object authority".into(),
            workgroup: WORKGROUP,
            grid_work_items: GRID,
            explicit_kernarg_bytes: 48,
            slice_lengths: [256; 3],
            buffer_roles: [
                "shared_atomic_channels",
                "exclusive_input_read_only_use",
                "disjoint_output",
            ]
            .map(String::from),
            pointer_access: ACCESS,
            distinct_guarded_allocations: true,
        },
    })
}

pub fn kernarg(metadata: &KernelMetadataV1) -> Result<Vec<u8>> {
    validate_metadata(metadata)?;
    let mut bytes = vec![0; usize::try_from(metadata.kernarg_bytes).expect("bounded ABI")];
    for offset in [8, 24, 40] {
        bytes[offset..offset + 8].copy_from_slice(&256_u64.to_le_bytes());
    }
    Ok(bytes)
}

pub fn validate_inputs(bytes: &[u8]) -> Result<()> {
    if bytes.len() != BYTES {
        return Err("atomic channel requires exactly 256 little-endian u32 values".into());
    }
    Ok(())
}
