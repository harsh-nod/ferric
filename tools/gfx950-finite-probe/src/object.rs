use fe2o3_amdhsa_loader::AdmittedProfile;
use fe2o3_hsaco::{ArgumentAccess, ExplicitValueKind};
use fe2o3_kfd::engineering_wire::{BufferAccessV1, ExplicitArgumentV1, KernelMetadataV1};

use crate::artifact::{digest, Inspection, ObservedArgumentQualifiers, ObservedLaunchMetadata};
use crate::Result;

fn narrow(value: u64) -> Result<u32> {
    u32::try_from(value).map_err(|_| "kernel metadata integer exceeds supported width".into())
}

fn access(value: ArgumentAccess) -> BufferAccessV1 {
    match value {
        ArgumentAccess::ReadOnly => BufferAccessV1::Read,
        ArgumentAccess::WriteOnly => BufferAccessV1::Write,
        ArgumentAccess::ReadWrite => BufferAccessV1::ReadWrite,
    }
}

/// Observation only: each fixed fixture must separately validate its exact ABI.
pub fn inspect(object: &[u8], symbol: &str) -> Result<Inspection> {
    let closure = fe2o3_amdhsa_loader::validate(object, AdmittedProfile::Gfx950XnackOffCov6)
        .map_err(|_| "offline gfx950 code object admission failed")?
        .bind_kernel(symbol)
        .map_err(|_| "fixed fixture symbol is absent or invalid")?;
    let resources = closure.resources();
    let launch = ObservedLaunchMetadata {
        required_workgroup_size: resources.required_workgroup_size(),
        max_flat_workgroup_size: resources.max_flat_workgroup_size(),
        max_workgroups: resources.max_workgroups(),
        cluster_dims: resources.cluster_dims(),
    };
    let kernel = closure.selected_kernel();
    if !kernel.arguments_were_emitted() {
        return Err("explicit kernel argument metadata is required".into());
    }
    let qualifiers = kernel
        .explicit_arguments()
        .iter()
        .map(|argument| ObservedArgumentQualifiers {
            actual_access: argument.actual_access().map(access),
            is_const: argument.is_const(),
            is_restrict: argument.is_restrict(),
            is_volatile: argument.is_volatile(),
            is_pipe: argument.is_pipe(),
        })
        .collect();
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
                access: argument.access().map(access),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(Inspection {
        metadata: KernelMetadataV1 {
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
        },
        launch,
        qualifiers,
    })
}
