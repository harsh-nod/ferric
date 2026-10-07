//! Separately supplied, exact packed-BF16 image admission; no invented producer roster.

use super::{EngineeringTpArtifactV1, M1EngineeringAggregateArtifactOpenErrorV1};
use std::path::Path;

/// The expanded projection and unchanged integer-only activation pack.
pub const ENGINEERING_TP_PACKED_BF16_EXPORTS_R2: [&str; 2] = [
    "ferric_qwen3_c1_wave_gemv_packed_u32_bf16_r2",
    "ferric_qwen3_c1_activation_pack_u32_r1",
];
const CRATE: &str = "ferric_qwen3_gemv_packed_u32_proposal";

/// Actual emitted identities supplied by the separately reviewed qualification.
/// These hashes identify bytes, not a source-to-device correctness certificate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EngineeringTpPackedBf16ImageIdsR2 {
    /// Exact code-object bytes.
    pub hsaco: [u8; 32],
    /// Exact canonical observation manifest.
    pub manifest: [u8; 32],
    /// Exact compiler handoff named by that manifest.
    pub handoff: [u8; 32],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PackedBf16BindingR2(EngineeringTpPackedBf16ImageIdsR2);

impl PackedBf16BindingR2 {
    pub(crate) const fn hsaco(self) -> [u8; 32] {
        self.0.hsaco
    }

    #[cfg(test)]
    pub(crate) const fn recording() -> Self {
        Self(EngineeringTpPackedBf16ImageIdsR2 {
            hsaco: [231; 32],
            manifest: [232; 32],
            handoff: [233; 32],
        })
    }
}

/// Only this exact opener can create a production packed-image binding.
pub struct EngineeringTpPackedBf16ArtifactR2 {
    artifact: EngineeringTpArtifactV1,
    binding: PackedBf16BindingR2,
}

impl EngineeringTpPackedBf16ArtifactR2 {
    /// Reopens the ordinary content-addressed engineering store and checks the
    /// actual compiler-generated logical/export pairs supplied by qualification.
    /// # Errors
    /// Rejects identity, roster, descriptor, ABI, resource or target disagreement.
    pub fn open(
        root: &Path,
        compiler_names: &[(&str, &str); 2],
        expected: EngineeringTpPackedBf16ImageIdsR2,
    ) -> Result<Self, M1EngineeringAggregateArtifactOpenErrorV1> {
        use M1EngineeringAggregateArtifactOpenErrorV1 as Error;
        if compiler_names
            .iter()
            .any(|(logical, export)| logical.is_empty() || export.is_empty())
            || compiler_names[0].0 == compiler_names[1].0
            || compiler_names
                .iter()
                .map(|entry| entry.1)
                .ne(ENGINEERING_TP_PACKED_BF16_EXPORTS_R2)
            || [expected.hsaco, expected.manifest, expected.handoff].contains(&[0; 32])
        {
            return Err(Error::CurrentFerricDescriptorRoster);
        }
        let artifact = EngineeringTpArtifactV1::open_profile_names(
            root,
            compiler_names,
            CRATE,
            &ENGINEERING_TP_PACKED_BF16_EXPORTS_R2,
        )?;
        let actual = EngineeringTpPackedBf16ImageIdsR2 {
            hsaco: *artifact.hsaco_id.as_bytes(),
            manifest: *artifact.manifest_id.as_bytes(),
            handoff: *artifact.handoff_id.as_bytes(),
        };
        if actual != expected {
            return Err(Error::ManifestPolicy {
                field: "packed_r2.expected_image_identities",
            });
        }
        if !artifact
            .inspection
            .hsaco()
            .kernels()
            .iter()
            .all(metadata_matches)
        {
            return Err(Error::HsacoProfile);
        }
        Ok(Self {
            artifact,
            binding: PackedBf16BindingR2(actual),
        })
    }

    /// The exact admitted bytes, for the existing owned-worker loader.
    #[must_use]
    pub const fn artifact(&self) -> &EngineeringTpArtifactV1 {
        &self.artifact
    }

    pub(crate) const fn binding(&self) -> PackedBf16BindingR2 {
        self.binding
    }
}

fn metadata_matches(kernel: &fe2o3_hsaco::InspectedKernel) -> bool {
    match kernel.name() {
        "ferric_qwen3_c1_wave_gemv_packed_u32_bf16_r2" => metadata_matches_layout(kernel, 3, 5, 72),
        "ferric_qwen3_c1_activation_pack_u32_r1" => metadata_matches_layout(kernel, 2, 2, 40),
        _ => false,
    }
}

pub(super) fn metadata_matches_layout(
    kernel: &fe2o3_hsaco::InspectedKernel,
    slices: usize,
    scalars: usize,
    implicit: u64,
) -> bool {
    metadata_matches_layout_with_agprs(kernel, slices, scalars, implicit, 0)
}

pub(super) fn metadata_matches_layout_with_agprs(
    kernel: &fe2o3_hsaco::InspectedKernel,
    slices: usize,
    scalars: usize,
    implicit: u64,
    agprs: u32,
) -> bool {
    use fe2o3_hsaco::{
        ArgumentAddressSpace::Global,
        ExplicitValueKind::{ByValue, GlobalBuffer},
        HiddenValueKind,
    };
    let hidden = [
        (0, 4, HiddenValueKind::BlockCountX),
        (4, 4, HiddenValueKind::BlockCountY),
        (8, 4, HiddenValueKind::BlockCountZ),
        (12, 2, HiddenValueKind::GroupSizeX),
        (14, 2, HiddenValueKind::GroupSizeY),
        (16, 2, HiddenValueKind::GroupSizeZ),
        (18, 2, HiddenValueKind::RemainderX),
        (20, 2, HiddenValueKind::RemainderY),
        (22, 2, HiddenValueKind::RemainderZ),
        (40, 8, HiddenValueKind::GlobalOffsetX),
        (48, 8, HiddenValueKind::GlobalOffsetY),
        (56, 8, HiddenValueKind::GlobalOffsetZ),
        (64, 2, HiddenValueKind::GridDimensions),
    ];
    kernel.kernarg_segment_size() == implicit + 256
        && kernel.kernarg_segment_alignment() == 8
        && kernel.implicit_argument_offset() == Some(implicit)
        && kernel.implicit_argument_size() == 256
        && kernel.wavefront_size() == 64
        && kernel.max_flat_workgroup_size() == 64
        && kernel.required_workgroup_size() == Some([64, 1, 1])
        && kernel.group_segment_fixed_size() == 0
        && kernel.private_segment_fixed_size() == 0
        && kernel.agpr_count() == Some(agprs)
        && kernel.sgpr_spill_count() == Some(0)
        && kernel.vgpr_spill_count() == Some(0)
        && !kernel.uses_dynamic_stack()
        && kernel.explicit_arguments().len() == slices * 2 + scalars
        && kernel
            .explicit_arguments()
            .iter()
            .enumerate()
            .all(|(index, arg)| {
                let slice_part = index < slices * 2;
                let pointer = slice_part && index.is_multiple_of(2);
                let offset = if slice_part {
                    index * 8
                } else {
                    slices * 16 + (index - slices * 2) * 4
                };
                arg.offset() == offset as u64
                    && arg.size() == (if slice_part { 8 } else { 4 })
                    && arg.value_kind() == (if pointer { GlobalBuffer } else { ByValue })
                    && arg.address_space() == (if pointer { Some(Global) } else { None })
                    && arg.alignment().is_none()
                    && arg.pointee_alignment().is_none()
                    && arg.access().is_none()
                    && arg.actual_access().is_none()
                    && arg.value_type().is_none()
            })
        && kernel.hidden_arguments().len() == hidden.len()
        && kernel
            .hidden_arguments()
            .iter()
            .zip(hidden)
            .all(|(arg, (offset, size, kind))| {
                (arg.offset(), arg.size(), arg.value_kind()) == (implicit + offset, size, kind)
            })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packed_r2_rejects_placeholder_ids_and_wrong_rosters_before_opening_files() {
        let names = [
            ("projection", ENGINEERING_TP_PACKED_BF16_EXPORTS_R2[0]),
            ("pack", ENGINEERING_TP_PACKED_BF16_EXPORTS_R2[1]),
        ];
        for mutation in 0..5 {
            let mut bad = names;
            let mut ids = PackedBf16BindingR2::recording().0;
            match mutation {
                0 => ids.hsaco = [0; 32],
                1 => bad.swap(0, 1),
                2 => bad[1].0 = bad[0].0,
                3 => bad[0].1 = "ferric_qwen3_c1_wave_gemv_packed_u32_bf16_r1",
                _ => bad[0].0 = "",
            }
            assert!(matches!(
                EngineeringTpPackedBf16ArtifactR2::open(Path::new("/unused"), &bad, ids),
                Err(M1EngineeringAggregateArtifactOpenErrorV1::CurrentFerricDescriptorRoster)
            ));
        }
    }
}
