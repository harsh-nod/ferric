//! Separate two-root split-attention admission; only generated names cross SDKs.

use super::{EngineeringTpArtifactV1, M1EngineeringAggregateArtifactOpenErrorV1};
use std::path::Path;

include!(concat!(env!("OUT_DIR"), "/v21_compiler_names.rs"));

/// V21 partial/merge roots, independently admitted from historical V14.
pub const ENGINEERING_TP_SPLIT_ATTENTION_EXPORTS_V21: [&str; 2] = [
    "ferric_qwen3_tp_c1_split8_attention_partial_f32_v21",
    "ferric_qwen3_tp_c1_split8_attention_merge_bf16_v21",
];
const CRATE: &str = "ferric_qwen3_tp_c1_split8_attention_kernels_device_v21";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SplitAttentionBindingV21 {
    pub(crate) hsaco: [u8; 32],
    manifest: [u8; 32],
    handoff: [u8; 32],
}

#[cfg(test)]
impl SplitAttentionBindingV21 {
    pub(crate) const fn recording() -> Self {
        Self {
            hsaco: [211; 32],
            manifest: [212; 32],
            handoff: [213; 32],
        }
    }
}

impl EngineeringTpArtifactV1 {
    /// Opens V21 with its own generated c4c roster and independently checked ABI.
    /// # Errors
    /// Rejects canonical identity, source, root, ABI, descriptor, or resource drift.
    pub fn open_split_attention_v21(
        root: &Path,
    ) -> Result<Self, M1EngineeringAggregateArtifactOpenErrorV1> {
        let artifact = Self::open_profile_names(
            root,
            &V21_COMPILER_NAMES,
            CRATE,
            &ENGINEERING_TP_SPLIT_ATTENTION_EXPORTS_V21,
        )?;
        if !artifact
            .inspection
            .hsaco()
            .kernels()
            .iter()
            .all(metadata_matches)
        {
            return Err(M1EngineeringAggregateArtifactOpenErrorV1::HsacoProfile);
        }
        Ok(artifact)
    }

    pub(crate) fn split_attention_binding_v21(&self) -> Option<SplitAttentionBindingV21> {
        (self.source_crate == CRATE).then(|| SplitAttentionBindingV21 {
            hsaco: *self.hsaco_id.as_bytes(),
            manifest: *self.manifest_id.as_bytes(),
            handoff: *self.handoff_id.as_bytes(),
        })
    }
}

fn metadata_matches(kernel: &fe2o3_hsaco::InspectedKernel) -> bool {
    use fe2o3_hsaco::HiddenValueKind;
    let partial = kernel.name() == ENGINEERING_TP_SPLIT_ATTENTION_EXPORTS_V21[0];
    let (implicit, arguments) = if partial { (136, 19) } else { (48, 6) };
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
    ENGINEERING_TP_SPLIT_ATTENTION_EXPORTS_V21.contains(&kernel.name())
        && kernel.kernarg_segment_size() == implicit + 256
        && kernel.kernarg_segment_alignment() == 8
        && kernel.implicit_argument_offset() == Some(implicit)
        && kernel.implicit_argument_size() == 256
        && kernel.wavefront_size() == 64
        && kernel.max_flat_workgroup_size() == 64
        && kernel.required_workgroup_size() == Some([64, 1, 1])
        && kernel.group_segment_fixed_size() == 0
        && kernel.private_segment_fixed_size() == 0
        && kernel.agpr_count() == Some(0)
        && kernel.sgpr_spill_count() == Some(0)
        && kernel.vgpr_spill_count() == Some(0)
        && !kernel.uses_dynamic_stack()
        && kernel.explicit_arguments().len() == arguments
        && kernel
            .explicit_arguments()
            .iter()
            .enumerate()
            .all(|(index, argument)| Argument::from(argument).matches(partial, index))
        && kernel.hidden_arguments().len() == hidden.len()
        && kernel
            .hidden_arguments()
            .iter()
            .zip(hidden)
            .all(|(argument, (offset, size, kind))| {
                (argument.offset(), argument.size(), argument.value_kind())
                    == (implicit + offset, size, kind)
            })
}

#[derive(Clone, Copy)]
struct Argument {
    offset: u64,
    size: u64,
    kind: fe2o3_hsaco::ExplicitValueKind,
    address_space: Option<fe2o3_hsaco::ArgumentAddressSpace>,
    alignment: Option<u64>,
    pointee_alignment: Option<u64>,
    access: Option<fe2o3_hsaco::ArgumentAccess>,
    actual_access: Option<fe2o3_hsaco::ArgumentAccess>,
    value_type: Option<fe2o3_hsaco::ExplicitValueType>,
}

impl From<&fe2o3_hsaco::ExplicitArgument> for Argument {
    fn from(argument: &fe2o3_hsaco::ExplicitArgument) -> Self {
        Self {
            offset: argument.offset(),
            size: argument.size(),
            kind: argument.value_kind(),
            address_space: argument.address_space(),
            alignment: argument.alignment(),
            pointee_alignment: argument.pointee_alignment(),
            access: argument.access(),
            actual_access: argument.actual_access(),
            value_type: argument.value_type(),
        }
    }
}

impl Argument {
    fn matches(self, partial: bool, index: usize) -> bool {
        use fe2o3_hsaco::{
            ArgumentAddressSpace::Global,
            ExplicitValueKind::{ByValue, GlobalBuffer},
        };
        let slices = if partial { 14 } else { 6 };
        if index >= slices + if partial { 5 } else { 0 } {
            return false;
        }
        let pointer = index < slices && index.is_multiple_of(2);
        let (offset, size) = if index < slices {
            (index * 8, 8)
        } else {
            (112 + (index - 14) * 4, 4)
        };
        self.offset == offset as u64
            && self.size == size
            && self.kind == (if pointer { GlobalBuffer } else { ByValue })
            && self.address_space == (if pointer { Some(Global) } else { None })
            && self.alignment.is_none()
            && self.pointee_alignment.is_none()
            && self.access.is_none()
            && self.actual_access.is_none()
            && self.value_type.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_roots_remain_distinct_from_v14_and_only_names_cross_sdks() {
        assert_eq!(V21_COMPILER_NAMES.len(), 2);
        assert!(
            V21_COMPILER_NAMES
                .iter()
                .all(|(logical, _)| !logical.is_empty())
        );
        assert_ne!(V21_COMPILER_NAMES[0].0, V21_COMPILER_NAMES[1].0);
        assert!(super::super::exact_roster(
            V21_COMPILER_NAMES.iter().map(|(_, export)| *export),
            &ENGINEERING_TP_SPLIT_ATTENTION_EXPORTS_V21,
        ));
        assert!(
            ENGINEERING_TP_SPLIT_ATTENTION_EXPORTS_V21
                .iter()
                .all(|root| !super::super::ENGINEERING_TP_QUERY_HOIST_EXPORTS_V14.contains(root))
        );
    }

    #[test]
    fn both_argument_layouts_reject_all_optional_qualifier_and_extent_mutations() {
        use fe2o3_hsaco::{
            ArgumentAccess, ArgumentAddressSpace, ExplicitValueKind, ExplicitValueType,
        };
        for partial in [false, true] {
            let slices = if partial { 14 } else { 6 };
            let count = if partial { 19 } else { 6 };
            for index in 0_usize..count {
                let pointer = index < slices && index.is_multiple_of(2);
                let valid = Argument {
                    offset: (if index < slices {
                        index * 8
                    } else {
                        112 + (index - 14) * 4
                    }) as u64,
                    size: if index < slices { 8 } else { 4 },
                    kind: if pointer {
                        ExplicitValueKind::GlobalBuffer
                    } else {
                        ExplicitValueKind::ByValue
                    },
                    address_space: pointer.then_some(ArgumentAddressSpace::Global),
                    alignment: None,
                    pointee_alignment: None,
                    access: None,
                    actual_access: None,
                    value_type: None,
                };
                assert!(valid.matches(partial, index));
                assert!(!valid.matches(partial, count));
                for mutation in 0..10 {
                    let mut bad = valid;
                    match mutation {
                        0 => bad.offset += 1,
                        1 => bad.size += 1,
                        2 => bad.kind = ExplicitValueKind::DynamicSharedPointer,
                        3 => bad.address_space = Some(ArgumentAddressSpace::Local),
                        4 => {
                            bad.address_space = if pointer {
                                None
                            } else {
                                Some(ArgumentAddressSpace::Global)
                            }
                        }
                        5 => bad.alignment = Some(8),
                        6 => bad.pointee_alignment = Some(4),
                        7 => bad.access = Some(ArgumentAccess::ReadOnly),
                        8 => bad.actual_access = Some(ArgumentAccess::WriteOnly),
                        9 => bad.value_type = Some(ExplicitValueType::F32),
                        _ => unreachable!(),
                    }
                    assert!(
                        !bad.matches(partial, index),
                        "partial={partial} arg={index} mutation={mutation}"
                    );
                }
            }
        }
    }

    #[test]
    #[ignore = "requires the actual emitted V21 image; CPU artifact inspection only"]
    fn emitted_v21_image_has_its_own_abi_and_identity() {
        let path = std::path::PathBuf::from(
            std::env::var_os("FERRIC_TEST_SPLIT_ATTENTION_V21_ARTIFACT")
                .expect("explicit V21 image"),
        );
        let artifact = EngineeringTpArtifactV1::open_split_attention_v21(&path).unwrap();
        let binding = artifact.split_attention_binding_v21().unwrap();
        assert_eq!(binding.hsaco, *artifact.hsaco_id().as_bytes());
        assert_eq!(binding.manifest, *artifact.manifest_id().as_bytes());
        assert_eq!(binding.handoff, *artifact.handoff_id().as_bytes());
        assert!(artifact.query_hoist_binding_v14().is_none());
        assert!(EngineeringTpArtifactV1::open_query_hoist_v14(&path).is_err());
    }
}
