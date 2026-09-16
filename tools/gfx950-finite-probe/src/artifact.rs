use fe2o3_amdhsa_loader::AdmittedProfile;
use fe2o3_hsaco::{ArgumentAccess, ExplicitValueKind};
use fe2o3_kfd::engineering_wire::{BufferAccessV1, ExplicitArgumentV1, KernelMetadataV1};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::Result;

pub const SYMBOL: &str = "ferric_gfx950_decoder_layer_f32_v1";
pub const WORKGROUP: [u16; 3] = [128, 1, 1];
// AQL grid dimensions count work-items, not workgroups.
pub const GRID: [u32; 3] = [256, 1, 1];
pub const LENGTHS: [u64; 3] = [3072, 110, 10240];
pub const BYTES: [usize; 3] = [12288, 440, 40960];
pub const GUARD: usize = 64;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeclaredAbi {
    basis: String,
    workgroup: [u16; 3],
    grid_work_items: [u32; 3],
    max_workgroups: [u32; 3],
    slice_lengths: [u64; 3],
    pointer_alignment: u32,
    pointer_access: [BufferAccessV1; 3],
}

pub fn declared_abi() -> DeclaredAbi {
    DeclaredAbi {
        basis:
            "fixed harness and source-declared ABI; not ELF-derived or source-to-object authority"
                .into(),
        workgroup: WORKGROUP,
        grid_work_items: GRID,
        max_workgroups: [2, 1, 1],
        slice_lengths: LENGTHS,
        pointer_alignment: 4,
        pointer_access: [
            BufferAccessV1::Read,
            BufferAccessV1::Read,
            BufferAccessV1::Write,
        ],
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ObservedLaunchMetadata {
    required_workgroup_size: Option<[u32; 3]>,
    max_flat_workgroup_size: u32,
    max_workgroups: [Option<u32>; 3],
    cluster_dims: Option<[u32; 3]>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ObservedArgumentQualifiers {
    pub actual_access: Option<BufferAccessV1>,
    pub is_const: Option<bool>,
    pub is_restrict: Option<bool>,
    pub is_volatile: Option<bool>,
    pub is_pipe: Option<bool>,
}

impl ObservedArgumentQualifiers {
    pub fn validate(&self, index: usize) -> Result<()> {
        let pointer = index.is_multiple_of(2);
        let read = index != 4;
        let access = if read {
            BufferAccessV1::Read
        } else {
            BufferAccessV1::Write
        };
        if self.is_volatile == Some(true)
            || self.is_pipe == Some(true)
            || (pointer && self.actual_access.is_some_and(|value| value != access))
            || (pointer && self.is_const.is_some_and(|value| value != read))
            || (pointer && self.is_restrict.is_some_and(|value| value != (index == 4)))
            || (!pointer
                && (self.actual_access.is_some()
                    || self.is_const == Some(true)
                    || self.is_restrict == Some(true)))
        {
            return Err(
                "finite decoder optional argument qualifier contradicts source-declared ABI".into(),
            );
        }
        Ok(())
    }
}

fn wire_access(access: ArgumentAccess) -> BufferAccessV1 {
    match access {
        ArgumentAccess::ReadOnly => BufferAccessV1::Read,
        ArgumentAccess::WriteOnly => BufferAccessV1::Write,
        ArgumentAccess::ReadWrite => BufferAccessV1::ReadWrite,
    }
}

pub fn validate_launch(
    required: Option<[u32; 3]>,
    max_flat: u32,
    max_workgroups: [Option<u32>; 3],
    cluster_dims: Option<[u32; 3]>,
) -> Result<()> {
    if required != Some(WORKGROUP.map(u32::from))
        || max_flat != 128
        || max_workgroups
            .into_iter()
            .zip([2, 1, 1])
            .any(|(observed, expected)| observed.is_some_and(|value| value != expected))
        || cluster_dims.is_some()
    {
        return Err("finite decoder workgroup contract mismatch".into());
    }
    Ok(())
}

pub fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

pub fn hex(bytes: &[u8; 32]) -> String {
    use std::fmt::Write;
    let mut output = String::with_capacity(64);
    for byte in bytes {
        write!(&mut output, "{byte:02x}").expect("String writes cannot fail");
    }
    output
}

fn narrow(value: u64) -> Result<u32> {
    u32::try_from(value).map_err(|_| "kernel metadata integer exceeds supported width".into())
}

/// Inspect owned bytes without opening any GPU device or minting authority.
pub struct Inspection {
    pub metadata: KernelMetadataV1,
    pub launch: ObservedLaunchMetadata,
    pub qualifiers: Vec<ObservedArgumentQualifiers>,
}

pub fn inspect(object: &[u8]) -> Result<Inspection> {
    let closure = fe2o3_amdhsa_loader::validate(object, AdmittedProfile::Gfx950XnackOffCov6)
        .map_err(|_| "offline gfx950 code object admission failed")?
        .bind_kernel(SYMBOL)
        .map_err(|_| "finite decoder symbol is absent or invalid")?;
    let resources = closure.resources();
    let launch = ObservedLaunchMetadata {
        required_workgroup_size: resources.required_workgroup_size(),
        max_flat_workgroup_size: resources.max_flat_workgroup_size(),
        max_workgroups: resources.max_workgroups(),
        cluster_dims: resources.cluster_dims(),
    };
    validate_launch(
        launch.required_workgroup_size,
        launch.max_flat_workgroup_size,
        launch.max_workgroups,
        launch.cluster_dims,
    )?;
    let kernel = closure.selected_kernel();
    if !kernel.arguments_were_emitted() {
        return Err("explicit kernel argument metadata is required".into());
    }
    let qualifiers = kernel
        .explicit_arguments()
        .iter()
        .enumerate()
        .map(|(index, argument)| {
            let observed = ObservedArgumentQualifiers {
                actual_access: argument.actual_access().map(wire_access),
                is_const: argument.is_const(),
                is_restrict: argument.is_restrict(),
                is_volatile: argument.is_volatile(),
                is_pipe: argument.is_pipe(),
            };
            observed.validate(index)?;
            Ok(observed)
        })
        .collect::<Result<Vec<_>>>()?;
    let explicit_arguments = kernel
        .explicit_arguments()
        .iter()
        .map(|argument| {
            if !matches!(
                argument.value_kind(),
                ExplicitValueKind::ByValue | ExplicitValueKind::GlobalBuffer
            ) {
                return Err("unsupported explicit argument kind".into());
            }
            Ok(ExplicitArgumentV1 {
                offset: narrow(argument.offset())?,
                bytes: narrow(argument.size())?,
                global_buffer: argument.value_kind() == ExplicitValueKind::GlobalBuffer,
                pointee_alignment: argument.pointee_alignment().map(narrow).transpose()?,
                access: argument.access().map(wire_access),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let metadata = KernelMetadataV1 {
        symbol: kernel.name().into(),
        object_sha256: digest(object),
        kernarg_bytes: narrow(kernel.kernarg_segment_size())?,
        kernarg_alignment: narrow(kernel.kernarg_segment_alignment())?,
        group_segment_bytes: narrow(kernel.group_segment_fixed_size())?,
        private_segment_bytes: narrow(kernel.private_segment_fixed_size())?,
        wavefront_size: kernel.wavefront_size(),
        implicit_argument_offset: kernel.implicit_argument_offset().map(narrow).transpose()?,
        implicit_argument_bytes: narrow(kernel.implicit_argument_size())?,
        explicit_arguments,
    };
    validate_abi(&metadata)?;
    Ok(Inspection {
        metadata,
        launch,
        qualifiers,
    })
}

pub fn validate_abi(metadata: &KernelMetadataV1) -> Result<()> {
    if metadata.symbol != SYMBOL
        || metadata.kernarg_alignment != 8
        || metadata.group_segment_bytes != 0
        || metadata.private_segment_bytes != 0
        || metadata.wavefront_size != 64
        || metadata.explicit_arguments.len() != 6
    {
        return Err("finite decoder resource or argument contract mismatch".into());
    }
    // COV6 implicit arguments are initialized by the worker, never by this client.
    if !matches!(
        (
            metadata.kernarg_bytes,
            metadata.implicit_argument_offset,
            metadata.implicit_argument_bytes
        ),
        (48, None, 0) | (304, Some(48), 256)
    ) {
        return Err("unsupported finite decoder implicit argument layout".into());
    }
    for (index, argument) in metadata.explicit_arguments.iter().enumerate() {
        let pointer = index % 2 == 0;
        let access = if index == 4 {
            BufferAccessV1::Write
        } else {
            BufferAccessV1::Read
        };
        if argument.offset != u32::try_from(index * 8).expect("six arguments")
            || argument.bytes != 8
            || argument.global_buffer != pointer
            || (pointer && argument.pointee_alignment.is_some_and(|value| value != 4))
            || (pointer && argument.access.is_some_and(|value| value != access))
            || (!pointer && (argument.pointee_alignment.is_some() || argument.access.is_some()))
        {
            return Err("finite decoder explicit argument ABI mismatch".into());
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
