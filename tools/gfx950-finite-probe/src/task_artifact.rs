use fe2o3_kfd::engineering_wire::{BufferAccessV1, KernelMetadataV1};
use serde::{Deserialize, Serialize};

use crate::artifact::{self, ObservedArgumentQualifiers, ObservedLaunchMetadata};
use crate::Result;

pub const SYMBOL: &str = "ferric_gfx950_task_graph_v1";
pub const WORKGROUP: [u16; 3] = [128, 1, 1];
pub const GRID: [u32; 3] = [256, 1, 1];
pub const INPUT_WORDS: usize = 896;
pub const BUFFER_COUNT: usize = 15;
pub const STATE_WORDS: usize = 13;
pub const GUARD: usize = 64;
pub const EPOCH_COUNT: usize = 5;
pub const EDGES: [[u32; 2]; 8] = [
    [0, 1],
    [0, 2],
    [1, 3],
    [2, 3],
    [3, 4],
    [3, 5],
    [4, 6],
    [5, 6],
];
pub const ROLES: [&str; BUFFER_COUNT] = [
    "inputs",
    "expected_epoch",
    "epoch",
    "ready",
    "done",
    "claimed",
    "owners",
    "errors",
    "payload0",
    "payload1",
    "payload2",
    "payload3",
    "payload4",
    "payload5",
    "payload6",
];

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeclaredAbi {
    basis: String,
    workgroup: [u16; 3],
    grid_work_items: [u32; 3],
    shared_bytes: u32,
    explicit_kernarg_bytes: u32,
    buffer_roles: Vec<String>,
    buffer_words: [u32; BUFFER_COUNT],
    distinct_guarded_allocations: bool,
    dependency_edges: [[u32; 2]; 8],
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

pub fn buffer_bytes(index: usize) -> usize {
    if index == 0 {
        INPUT_WORDS * 4
    } else {
        4
    }
}

pub fn buffer_access(index: usize) -> BufferAccessV1 {
    if index < 2 {
        BufferAccessV1::Read
    } else {
        BufferAccessV1::ReadWrite
    }
}

pub fn pointer_offset(index: usize) -> u32 {
    u32::try_from(if index < 2 {
        index * 16
    } else {
        (index + 2) * 8
    })
    .expect("fixed fifteen-buffer ABI")
}

pub fn validate_metadata(metadata: &KernelMetadataV1) -> Result<()> {
    if metadata.symbol != SYMBOL
        || metadata.kernarg_alignment != 8
        || metadata.group_segment_bytes != 1024
        || metadata.private_segment_bytes != 0
        || metadata.wavefront_size != 64
        || metadata.explicit_arguments.len() != 17
        || !matches!(
            (
                metadata.kernarg_bytes,
                metadata.implicit_argument_offset,
                metadata.implicit_argument_bytes
            ),
            (136, None, 0) | (392, Some(136), 256)
        )
    {
        return Err("task graph fixed resources or kernarg layout mismatch".into());
    }
    for (index, argument) in metadata.explicit_arguments.iter().enumerate() {
        let pointer = index != 1 && index != 3;
        let expected = if index < 4 {
            BufferAccessV1::Read
        } else {
            BufferAccessV1::ReadWrite
        };
        if argument.offset != u32::try_from(index * 8).expect("seventeen arguments")
            || argument.bytes != 8
            || argument.global_buffer != pointer
            || (pointer && argument.pointee_alignment.is_some_and(|value| value != 4))
            || (pointer && argument.access.is_some_and(|value| value != expected))
            || (!pointer && (argument.pointee_alignment.is_some() || argument.access.is_some()))
        {
            return Err("task graph explicit argument mismatch".into());
        }
    }
    Ok(())
}

pub fn validate_qualifier(index: usize, qualifier: &ObservedArgumentQualifiers) -> Result<()> {
    let pointer = index != 1 && index != 3;
    let read = index < 4;
    if qualifier.is_volatile == Some(true)
        || qualifier.is_pipe == Some(true)
        || qualifier.is_restrict == Some(true)
        || (pointer && qualifier.is_const.is_some_and(|value| value != read))
        || (pointer
            && read
            && qualifier
                .actual_access
                .is_some_and(|value| value != BufferAccessV1::Read))
        || (!pointer && (qualifier.actual_access.is_some() || qualifier.is_const == Some(true)))
    {
        return Err("task graph optional qualifiers contradict the fixed shared-atomic ABI".into());
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
    for (index, qualifier) in observed.qualifiers.iter().enumerate() {
        validate_qualifier(index, qualifier)?;
    }
    let mut words = [1; BUFFER_COUNT];
    words[0] = u32::try_from(INPUT_WORDS).expect("bounded inputs");
    Ok(Record {
        schema: "ferric-task-graph-artifact-v1".into(),
        source_sha256: artifact::hex(&artifact::digest(source)),
        object_sha256: artifact::hex(&artifact::digest(object)),
        metadata: observed.metadata,
        observed_launch_metadata: observed.launch,
        observed_argument_qualifiers: observed.qualifiers,
        source_declared_abi: DeclaredAbi {
            basis:
                "fixed engineering fixture declaration; not ELF-derived source-to-object authority"
                    .into(),
            workgroup: WORKGROUP,
            grid_work_items: GRID,
            shared_bytes: 1024,
            explicit_kernarg_bytes: 136,
            buffer_roles: ROLES.map(String::from).into(),
            buffer_words: words,
            distinct_guarded_allocations: true,
            dependency_edges: EDGES,
        },
    })
}

pub fn kernarg(metadata: &KernelMetadataV1) -> Result<Vec<u8>> {
    validate_metadata(metadata)?;
    let mut bytes = vec![0; usize::try_from(metadata.kernarg_bytes).expect("bounded ABI")];
    bytes[8..16].copy_from_slice(&896_u64.to_le_bytes());
    bytes[24..32].copy_from_slice(&1_u64.to_le_bytes());
    Ok(bytes)
}

pub fn validate_inputs(bytes: &[u8]) -> Result<()> {
    if bytes.len() != INPUT_WORDS * 4
        || bytes
            .chunks_exact(4)
            .any(|word| u32::from_le_bytes(word.try_into().expect("four bytes")) > 1024)
    {
        return Err(
            "task graph inputs require exactly896 little-endian u32 values in0..1024".into(),
        );
    }
    Ok(())
}
