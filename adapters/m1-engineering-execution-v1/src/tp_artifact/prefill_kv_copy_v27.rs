//! Independent, fixed TP1/prefill16 copy image admission; no page ownership grant.

use super::{EngineeringTpArtifactV1, M1EngineeringAggregateArtifactOpenErrorV1};
use std::path::Path;

/// One parallel raw-bit copy root, never a resident append image substitute.
pub const ENGINEERING_TP_PREFILL_KV_COPY_EXPORTS_V27: [&str; 1] =
    ["ferric_qwen3_tp_prefill16_kv_copy_bf16_v27"];
include!(concat!(env!("OUT_DIR"), "/v27_compiler_names.rs"));

const CRATE: &str = "ferric_qwen3_tp_prefill_kv_copy_kernels_device_v27";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PrefillKvCopyBindingV27 {
    pub(crate) hsaco: [u8; 32],
    manifest: [u8; 32],
    handoff: [u8; 32],
}

#[cfg(test)]
impl PrefillKvCopyBindingV27 {
    pub(crate) const fn recording() -> Self {
        Self {
            hsaco: [191; 32],
            manifest: [192; 32],
            handoff: [193; 32],
        }
    }
}

impl EngineeringTpArtifactV1 {
    /// Admits the separate V27 root and ABI, not slot ownership or numerical correctness.
    /// # Errors
    /// Rejects noncanonical identity, unexpected roots, ABI drift or unsupported resources.
    pub fn open_prefill_kv_copy_v27(
        root: &Path,
    ) -> Result<Self, M1EngineeringAggregateArtifactOpenErrorV1> {
        let artifact = Self::open_profile_names(
            root,
            &V27_COMPILER_NAMES,
            CRATE,
            &ENGINEERING_TP_PREFILL_KV_COPY_EXPORTS_V27,
        )?;
        if !metadata_matches(&artifact.inspection.hsaco().kernels()[0]) {
            return Err(M1EngineeringAggregateArtifactOpenErrorV1::HsacoProfile);
        }
        Ok(artifact)
    }

    pub(crate) fn prefill_kv_copy_binding_v27(&self) -> Option<PrefillKvCopyBindingV27> {
        (self.source_crate == CRATE).then(|| PrefillKvCopyBindingV27 {
            hsaco: *self.hsaco_id.as_bytes(),
            manifest: *self.manifest_id.as_bytes(),
            handoff: *self.handoff_id.as_bytes(),
        })
    }
}

fn metadata_matches(kernel: &fe2o3_hsaco::InspectedKernel) -> bool {
    use fe2o3_hsaco::HiddenValueKind;
    let hidden = [
        (80, 4, HiddenValueKind::BlockCountX),
        (84, 4, HiddenValueKind::BlockCountY),
        (88, 4, HiddenValueKind::BlockCountZ),
        (92, 2, HiddenValueKind::GroupSizeX),
        (94, 2, HiddenValueKind::GroupSizeY),
        (96, 2, HiddenValueKind::GroupSizeZ),
        (98, 2, HiddenValueKind::RemainderX),
        (100, 2, HiddenValueKind::RemainderY),
        (102, 2, HiddenValueKind::RemainderZ),
        (120, 8, HiddenValueKind::GlobalOffsetX),
        (128, 8, HiddenValueKind::GlobalOffsetY),
        (136, 8, HiddenValueKind::GlobalOffsetZ),
        (144, 2, HiddenValueKind::GridDimensions),
    ];
    kernel.name() == ENGINEERING_TP_PREFILL_KV_COPY_EXPORTS_V27[0]
        && kernel.kernarg_segment_size() == 336
        && kernel.kernarg_segment_alignment() == 8
        && kernel.implicit_argument_offset() == Some(80)
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
        && kernel.explicit_arguments().len() == 12
        && kernel
            .explicit_arguments()
            .iter()
            .enumerate()
            .all(|(index, arg)| Argument::from(arg).matches(index))
        && kernel.hidden_arguments().len() == hidden.len()
        && kernel
            .hidden_arguments()
            .iter()
            .zip(hidden)
            .all(|(arg, expected)| (arg.offset(), arg.size(), arg.value_kind()) == expected)
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
    fn from(arg: &fe2o3_hsaco::ExplicitArgument) -> Self {
        Self {
            offset: arg.offset(),
            size: arg.size(),
            kind: arg.value_kind(),
            address_space: arg.address_space(),
            alignment: arg.alignment(),
            pointee_alignment: arg.pointee_alignment(),
            access: arg.access(),
            actual_access: arg.actual_access(),
            value_type: arg.value_type(),
        }
    }
}

impl Argument {
    fn matches(self, index: usize) -> bool {
        use fe2o3_hsaco::{
            ArgumentAddressSpace::Global,
            ExplicitValueKind::{ByValue, GlobalBuffer},
        };
        if index >= 12 {
            return false;
        }
        let pointer = index < 8 && index.is_multiple_of(2);
        let (offset, size) = if index < 8 {
            (index * 8, 8)
        } else {
            (64 + (index - 8) * 4, 4)
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
    #[ignore = "requires the actual canonical emitted V27 image; never synthetic authority"]
    fn v27_actual_prefill_image_has_the_separate_twelve_argument_abi() {
        let root = std::path::PathBuf::from(
            std::env::var_os("FERRIC_TEST_PREFILL_COPY_V27_ARTIFACT")
                .expect("explicit actual V27 image"),
        );
        let artifact = EngineeringTpArtifactV1::open_prefill_kv_copy_v27(&root).unwrap();
        assert!(artifact.prefill_kv_copy_binding_v27().is_some());
        assert!(artifact.c1_kv_copy_binding_v19().is_none());
        assert_eq!(artifact.inspection().hsaco().kernels().len(), 1);
        assert!(metadata_matches(
            &artifact.inspection().hsaco().kernels()[0]
        ));
        assert!(EngineeringTpArtifactV1::open_c1_kv_copy_v19(&root).is_err());
        let binding = artifact.prefill_kv_copy_binding_v27().unwrap();
        assert_eq!(binding.hsaco, *artifact.hsaco_id().as_bytes());
        assert_eq!(binding.manifest, *artifact.manifest_id().as_bytes());
        assert_eq!(binding.handoff, *artifact.handoff_id().as_bytes());
        assert!(artifact.query_hoist_binding_v14().is_none());
        assert!(artifact.wave_rmsnorm_binding_v15().is_none());
        assert!(artifact.fp32_argmax_binding_v11().is_none());
        assert!(EngineeringTpArtifactV1::open_wave_rmsnorm_v15(&root).is_err());
        assert!(EngineeringTpArtifactV1::open_query_hoist_v14(&root).is_err());
    }

    #[test]
    fn v27_copy_roster_is_distinct_and_exact() {
        let roots = ENGINEERING_TP_PREFILL_KV_COPY_EXPORTS_V27;
        assert!(super::super::exact_roster(
            V27_COMPILER_NAMES.iter().map(|entry| entry.1),
            &roots
        ));
        assert_eq!(V27_COMPILER_NAMES.len(), 1);
        assert!(!V27_COMPILER_NAMES[0].0.is_empty());
        assert!(!super::super::exact_roster(
            [roots[0], roots[0]].into_iter(),
            &roots
        ));
        for root in super::super::ENGINEERING_TP_SPLIT_ATTENTION_EXPORTS_V21 {
            assert!(!roots.contains(&root));
        }
        for root in super::super::ENGINEERING_TP_BATCH32_EXPORTS_V5 {
            assert!(!roots.contains(&root));
        }
    }

    #[test]
    fn v27_copy_argument_contract_rejects_every_qualifier_and_geometry_drift() {
        use fe2o3_hsaco::{
            ArgumentAccess, ArgumentAddressSpace, ExplicitValueKind, ExplicitValueType,
        };
        for index in 0_usize..12 {
            let pointer = index < 8 && index.is_multiple_of(2);
            let valid = Argument {
                offset: (if index < 8 {
                    index * 8
                } else {
                    64 + (index - 8) * 4
                }) as u64,
                size: if index < 8 { 8 } else { 4 },
                kind: if pointer {
                    ExplicitValueKind::GlobalBuffer
                } else {
                    ExplicitValueKind::ByValue
                },
                address_space: if pointer {
                    Some(ArgumentAddressSpace::Global)
                } else {
                    None
                },
                alignment: None,
                pointee_alignment: None,
                access: None,
                actual_access: None,
                value_type: None,
            };
            assert!(valid.matches(index));
            assert!(!valid.matches(12));
            for mutation in 0..10 {
                let mut changed = valid;
                match mutation {
                    0 => changed.offset += 1,
                    1 => changed.size += 1,
                    2 => changed.kind = ExplicitValueKind::DynamicSharedPointer,
                    3 => changed.address_space = Some(ArgumentAddressSpace::Local),
                    4 => {
                        changed.address_space = if pointer {
                            None
                        } else {
                            Some(ArgumentAddressSpace::Global)
                        };
                    }
                    5 => changed.alignment = Some(8),
                    6 => changed.pointee_alignment = Some(2),
                    7 => changed.access = Some(ArgumentAccess::ReadOnly),
                    8 => changed.actual_access = Some(ArgumentAccess::WriteOnly),
                    9 => changed.value_type = Some(ExplicitValueType::F32),
                    _ => unreachable!(),
                }
                assert!(
                    !changed.matches(index),
                    "argument {index}, mutation {mutation}"
                );
            }
        }
    }
}
