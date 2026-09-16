use fe2o3_kfd::engineering_wire::{BufferAccessV1, KernelMetadataV1};
use serde::{Deserialize, Serialize};

use crate::{artifact, kproj_artifact, Result};

pub const SYMBOL: &str = "ferric_gfx950_qwen3_knorm_v1";
pub const WORKGROUP: [u16; 3] = [128, 1, 1];
pub const GRID: [u32; 3] = [512, 1, 1];
pub const LENGTHS: [u64; 4] = [1024, 128, 1024, 1024];
pub const BYTES: [usize; 6] = [2048, 2_097_152, 4096, 256, 2048, 2048];
pub const GUARD: usize = 64;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Stage {
    source_sha256: String,
    object_sha256: String,
    pub metadata: KernelMetadataV1,
    observed_launch_metadata: artifact::ObservedLaunchMetadata,
    observed_argument_qualifiers: Vec<artifact::ObservedArgumentQualifiers>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Graph {
    basis: String,
    allocation_bytes: [usize; 6],
    producer_buffers: [usize; 3],
    consumer_buffers: [usize; 4],
    producer_grid: [u32; 3],
    consumer_grid: [u32; 3],
    workgroup: [u16; 3],
    consumer_lengths: [u64; 4],
    completion_ordered: bool,
    intermediate_reupload: bool,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Record {
    schema: String,
    pub producer: Stage,
    pub consumer: Stage,
    graph: Graph,
}

fn stage(object: &[u8], source: &[u8], observed: artifact::Inspection) -> Stage {
    Stage {
        source_sha256: artifact::hex(&artifact::digest(source)),
        object_sha256: artifact::hex(&artifact::digest(object)),
        metadata: observed.metadata,
        observed_launch_metadata: observed.launch,
        observed_argument_qualifiers: observed.qualifiers,
    }
}

pub fn validate_consumer(metadata: &KernelMetadataV1) -> Result<()> {
    if metadata.symbol != SYMBOL
        || metadata.kernarg_alignment != 8
        || metadata.group_segment_bytes != 0
        || metadata.private_segment_bytes != 0
        || metadata.wavefront_size != 64
        || metadata.explicit_arguments.len() != 8
        || !matches!(
            (
                metadata.kernarg_bytes,
                metadata.implicit_argument_offset,
                metadata.implicit_argument_bytes
            ),
            (64, None, 0) | (320, Some(64), 256)
        )
    {
        return Err("Qwen3 K norm resources or argument layout mismatch".into());
    }
    for (index, argument) in metadata.explicit_arguments.iter().enumerate() {
        let pointer = index.is_multiple_of(2);
        let access = if index < 4 {
            BufferAccessV1::Read
        } else {
            BufferAccessV1::Write
        };
        let alignment = if index == 0 { 4 } else { 2 };
        if argument.offset != u32::try_from(index * 8).expect("eight arguments")
            || argument.bytes != 8
            || argument.global_buffer != pointer
            || (pointer
                && argument
                    .pointee_alignment
                    .is_some_and(|value| value != alignment))
            || (pointer && argument.access.is_some_and(|value| value != access))
            || (!pointer && (argument.pointee_alignment.is_some() || argument.access.is_some()))
        {
            return Err("Qwen3 K norm explicit argument mismatch".into());
        }
    }
    Ok(())
}

pub fn validate_qualifier(
    index: usize,
    qualifier: &artifact::ObservedArgumentQualifiers,
) -> Result<()> {
    let pointer = index.is_multiple_of(2);
    let write = index >= 4;
    if index >= 8
        || qualifier.is_volatile == Some(true)
        || qualifier.is_pipe == Some(true)
        || (pointer
            && qualifier.actual_access.is_some_and(|value| {
                value
                    != if write {
                        BufferAccessV1::Write
                    } else {
                        BufferAccessV1::Read
                    }
            }))
        || (pointer && qualifier.is_const.is_some_and(|value| value == write))
        || (pointer && qualifier.is_restrict.is_some_and(|value| value != write))
        || (!pointer
            && (qualifier.actual_access.is_some()
                || qualifier.is_const == Some(true)
                || qualifier.is_restrict == Some(true)))
    {
        return Err("Qwen3 K norm argument qualifier mismatch".into());
    }
    Ok(())
}

pub fn validate_launch(launch: &artifact::ObservedLaunchMetadata) -> Result<()> {
    if launch.required_workgroup_size != Some(WORKGROUP.map(u32::from))
        || launch.max_flat_workgroup_size != 128
        || launch
            .max_workgroups
            .into_iter()
            .zip([4, 1, 1])
            .any(|(observed, expected)| observed.is_some_and(|value| value != expected))
        || launch.cluster_dims.is_some()
    {
        return Err("Qwen3 K norm workgroup contract mismatch".into());
    }
    Ok(())
}

pub fn inspect(
    producer: &[u8],
    producer_source: &[u8],
    consumer: &[u8],
    consumer_source: &[u8],
) -> Result<Record> {
    let producer_observed = kproj_artifact::inspect(producer, kproj_artifact::Variant::Wave64)?;
    let consumer_observed = crate::object::inspect(consumer, SYMBOL)?;
    validate_consumer(&consumer_observed.metadata)?;
    validate_launch(&consumer_observed.launch)?;
    if consumer_observed.qualifiers.len() != 8 {
        return Err("Qwen3 K norm qualifier count mismatch".into());
    }
    for (index, qualifier) in consumer_observed.qualifiers.iter().enumerate() {
        validate_qualifier(index, qualifier)?;
    }
    Ok(Record {
        schema: "ferric-qwen3-knorm-chain-artifact-v1".into(),
        producer: stage(producer, producer_source, producer_observed),
        consumer: stage(consumer, consumer_source, consumer_observed),
        graph: Graph {
            basis: "fixed engineering graph; not source-to-object or runtime publication authority"
                .into(),
            allocation_bytes: BYTES,
            producer_buffers: [0, 1, 2],
            consumer_buffers: [2, 3, 4, 5],
            producer_grid: kproj_artifact::WAVE64_GRID,
            consumer_grid: GRID,
            workgroup: WORKGROUP,
            consumer_lengths: LENGTHS,
            completion_ordered: true,
            intermediate_reupload: false,
        },
    })
}

pub fn kernarg(metadata: &KernelMetadataV1) -> Result<Vec<u8>> {
    validate_consumer(metadata)?;
    let mut bytes = vec![0; usize::try_from(metadata.kernarg_bytes).expect("bounded ABI")];
    for (index, length) in LENGTHS.into_iter().enumerate() {
        let offset = index * 16 + 8;
        bytes[offset..offset + 8].copy_from_slice(&length.to_le_bytes());
    }
    Ok(bytes)
}
