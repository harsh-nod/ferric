//! Separate pure width4096 RMSNorm admission, without changing resident root inventories.

use super::{EngineeringTpArtifactV1, M1EngineeringAggregateArtifactOpenErrorV1};
use std::path::Path;

/// One separately loaded Wave64 RMSNorm root; never a resident image substitute.
pub const ENGINEERING_TP_WAVE_RMSNORM_EXPORTS_V15: [&str; 1] =
    ["ferric_qwen3_tp_batch32_wave_rmsnorm_bf16_v15"];
const CRATE: &str = "ferric_qwen3_tp_wave_rmsnorm_kernels_device_v15";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct WaveRmsNormBindingV15 {
    pub(crate) hsaco: [u8; 32],
    manifest: [u8; 32],
    handoff: [u8; 32],
}

#[cfg(test)]
impl WaveRmsNormBindingV15 {
    pub(crate) const fn recording() -> Self {
        Self {
            hsaco: [151; 32],
            manifest: [152; 32],
            handoff: [153; 32],
        }
    }
}

impl EngineeringTpArtifactV1 {
    /// Admits the exact separate V15 root and ABI, not numerical or model correctness.
    /// # Errors
    /// Rejects noncanonical identity, wrong roots, argument drift or unsupported resources.
    pub fn open_wave_rmsnorm_v15(
        root: &Path,
    ) -> Result<Self, M1EngineeringAggregateArtifactOpenErrorV1> {
        let artifact = Self::open_profile(
            root,
            &ferric_qwen3_tp_wave_rmsnorm_kernels_device_v15::compiler_expectation_roster_v15(),
            CRATE,
            &ENGINEERING_TP_WAVE_RMSNORM_EXPORTS_V15,
        )?;
        if !metadata_matches(&artifact.inspection.hsaco().kernels()[0]) {
            return Err(M1EngineeringAggregateArtifactOpenErrorV1::HsacoProfile);
        }
        Ok(artifact)
    }

    pub(crate) fn wave_rmsnorm_binding_v15(&self) -> Option<WaveRmsNormBindingV15> {
        (self.source_crate == CRATE).then(|| WaveRmsNormBindingV15 {
            hsaco: *self.hsaco_id.as_bytes(),
            manifest: *self.manifest_id.as_bytes(),
            handoff: *self.handoff_id.as_bytes(),
        })
    }
}

fn metadata_matches(kernel: &fe2o3_hsaco::InspectedKernel) -> bool {
    use fe2o3_hsaco::HiddenValueKind;
    let hidden = [
        (96, 4, HiddenValueKind::BlockCountX),
        (100, 4, HiddenValueKind::BlockCountY),
        (104, 4, HiddenValueKind::BlockCountZ),
        (108, 2, HiddenValueKind::GroupSizeX),
        (110, 2, HiddenValueKind::GroupSizeY),
        (112, 2, HiddenValueKind::GroupSizeZ),
        (114, 2, HiddenValueKind::RemainderX),
        (116, 2, HiddenValueKind::RemainderY),
        (118, 2, HiddenValueKind::RemainderZ),
        (136, 8, HiddenValueKind::GlobalOffsetX),
        (144, 8, HiddenValueKind::GlobalOffsetY),
        (152, 8, HiddenValueKind::GlobalOffsetZ),
        (160, 2, HiddenValueKind::GridDimensions),
    ];
    kernel.name() == ENGINEERING_TP_WAVE_RMSNORM_EXPORTS_V15[0]
        && kernel.kernarg_segment_size() == 352
        && kernel.kernarg_segment_alignment() == 8
        && kernel.implicit_argument_offset() == Some(96)
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
        && kernel.explicit_arguments().len() == 14
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
        if index >= 14 {
            return false;
        }
        let pointer = index < 10 && index.is_multiple_of(2);
        let (offset, size) = if index < 10 {
            (index * 8, 8)
        } else {
            (80 + (index - 10) * 4, 4)
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
    fn wave_rmsnorm_v15_roster_is_exact_and_disjoint() {
        use super::super::exact_roster;
        let roots = ENGINEERING_TP_WAVE_RMSNORM_EXPORTS_V15;
        assert_eq!(
            roots,
            ferric_qwen3_tp_wave_rmsnorm_kernels_device_v15::ROOTS_V15
        );
        assert!(exact_roster(
            ferric_qwen3_tp_wave_rmsnorm_kernels_device_v15::compiler_expectation_roster_v15()
                .iter()
                .map(fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1::export_name),
            &roots,
        ));
        assert!(!exact_roster(std::iter::empty(), &roots));
        assert!(!exact_roster([roots[0], roots[0]].into_iter(), &roots));
        for root in super::super::ENGINEERING_TP_BATCH32_EXPORTS_V5
            .into_iter()
            .chain(super::super::ENGINEERING_TP_FP32_HEAD32_EXPORTS_V8)
            .chain(super::super::ENGINEERING_TP_LARGE_KV_EXPORTS_V9)
            .chain(super::super::ENGINEERING_DRAFT_BATCH32_EXPORTS_V10)
            .chain(super::super::ENGINEERING_TP_FP32_ARGMAX32_EXPORTS_V11)
            .chain(super::super::ENGINEERING_TP_QUERY_HOIST_EXPORTS_V14)
            .chain(["qwen3_rmsnorm_v1"])
        {
            assert!(!roots.contains(&root));
            assert!(!exact_roster([root].into_iter(), &roots));
        }
    }

    #[test]
    fn wave_rmsnorm_v15_argument_profile_rejects_geometry_and_qualifier_drift() {
        use fe2o3_hsaco::{
            ArgumentAccess, ArgumentAddressSpace, ExplicitValueKind, ExplicitValueType,
        };
        for index in 0_usize..14 {
            let pointer = index < 10 && index.is_multiple_of(2);
            let valid = Argument {
                offset: if index < 10 {
                    index * 8
                } else {
                    80 + (index - 10) * 4
                } as u64,
                size: if index < 10 { 8 } else { 4 },
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
            assert!(!valid.matches(14));
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
                        }
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

    #[test]
    #[ignore = "requires the separately emitted canonical v15 image; host admission only"]
    fn wave_rmsnorm_v15_image_admission_binds_exact_abi_and_identity() {
        let root = std::path::PathBuf::from(
            std::env::var_os("FERRIC_TEST_WAVE_RMSNORM_V15_ARTIFACT").expect("explicit v15 image"),
        );
        let artifact = EngineeringTpArtifactV1::open_wave_rmsnorm_v15(&root).unwrap();
        let binding = artifact.wave_rmsnorm_binding_v15().unwrap();
        assert_eq!(binding.hsaco, *artifact.hsaco_id().as_bytes());
        assert_eq!(binding.manifest, *artifact.manifest_id().as_bytes());
        assert_eq!(binding.handoff, *artifact.handoff_id().as_bytes());
        assert!(metadata_matches(
            &artifact.inspection().hsaco().kernels()[0]
        ));
        assert!(artifact.fp32_argmax_binding_v11().is_none());
        assert!(artifact.query_hoist_binding_v14().is_none());
        assert!(artifact.draft_binding().is_none());
        assert!(artifact.large_kv_binding().is_none());
        assert!(EngineeringTpArtifactV1::open_query_hoist_v14(&root).is_err());
        assert!(EngineeringTpArtifactV1::open_fp32_argmax32_v11(&root).is_err());
        assert!(
            EngineeringTpArtifactV1::open_batch32(
                &root,
                &ferric_qwen3_tp_batch32_kernels_device_v5::compiler_expectation_roster_v5(),
                true,
            )
            .is_err()
        );
    }
}
