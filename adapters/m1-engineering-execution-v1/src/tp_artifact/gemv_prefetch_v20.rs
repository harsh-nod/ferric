//! Exact V20 artifact admission; only generated names cross from the c4c build unit.

use super::{EngineeringTpArtifactV1, M1EngineeringAggregateArtifactOpenErrorV1};
use std::path::Path;

include!(concat!(env!("OUT_DIR"), "/v20_compiler_names.rs"));

/// Two independent V20 decoder projection roots; no legacy image substitution.
pub const ENGINEERING_TP_GEMV_PREFETCH_EXPORTS_V20: [&str; 2] = [
    "ferric_qwen3_tp_wave_gemv_prefetch4_bf16_v20",
    "ferric_qwen3_tp_wave_gemv_prefetch4_partial_f32_v20",
];
const CRATE: &str = "ferric_qwen3_tp_gemv_prefetch_kernels_device_v20";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct GemvPrefetchBindingV20 {
    pub(crate) hsaco: [u8; 32],
    manifest: [u8; 32],
    handoff: [u8; 32],
}

#[cfg(test)]
impl GemvPrefetchBindingV20 {
    pub(crate) const fn recording() -> Self {
        Self { hsaco: [201; 32], manifest: [202; 32], handoff: [203; 32] }
    }
}

impl EngineeringTpArtifactV1 {
    /// Opens the separately pinned V20 image using its own generated roster.
    /// # Errors
    /// Rejects file, source, descriptor, root, ABI or resource disagreement.
    pub fn open_gemv_prefetch_v20(root: &Path) -> Result<Self, M1EngineeringAggregateArtifactOpenErrorV1> {
        let artifact = Self::open_profile_names(root, &V20_COMPILER_NAMES, CRATE, &ENGINEERING_TP_GEMV_PREFETCH_EXPORTS_V20)?;
        if !artifact.inspection.hsaco().kernels().iter().all(metadata_matches) {
            return Err(M1EngineeringAggregateArtifactOpenErrorV1::HsacoProfile);
        }
        Ok(artifact)
    }

    pub(crate) fn gemv_prefetch_binding_v20(&self) -> Option<GemvPrefetchBindingV20> {
        (self.source_crate == CRATE).then(|| GemvPrefetchBindingV20 {
            hsaco: *self.hsaco_id.as_bytes(), manifest: *self.manifest_id.as_bytes(), handoff: *self.handoff_id.as_bytes(),
        })
    }
}

fn metadata_matches(kernel: &fe2o3_hsaco::InspectedKernel) -> bool {
    use fe2o3_hsaco::HiddenValueKind;
    let hidden = [
        (72, 4, HiddenValueKind::BlockCountX), (76, 4, HiddenValueKind::BlockCountY), (80, 4, HiddenValueKind::BlockCountZ),
        (84, 2, HiddenValueKind::GroupSizeX), (86, 2, HiddenValueKind::GroupSizeY), (88, 2, HiddenValueKind::GroupSizeZ),
        (90, 2, HiddenValueKind::RemainderX), (92, 2, HiddenValueKind::RemainderY), (94, 2, HiddenValueKind::RemainderZ),
        (112, 8, HiddenValueKind::GlobalOffsetX), (120, 8, HiddenValueKind::GlobalOffsetY), (128, 8, HiddenValueKind::GlobalOffsetZ),
        (136, 2, HiddenValueKind::GridDimensions),
    ];
    ENGINEERING_TP_GEMV_PREFETCH_EXPORTS_V20.contains(&kernel.name())
        && kernel.kernarg_segment_size() == 328 && kernel.kernarg_segment_alignment() == 8
        && kernel.implicit_argument_offset() == Some(72) && kernel.implicit_argument_size() == 256
        && kernel.wavefront_size() == 64 && kernel.max_flat_workgroup_size() == 64
        && kernel.required_workgroup_size() == Some([64, 1, 1])
        && kernel.group_segment_fixed_size() == 0 && kernel.private_segment_fixed_size() == 0
        && kernel.agpr_count() == Some(0) && kernel.sgpr_spill_count() == Some(0) && kernel.vgpr_spill_count() == Some(0)
        && !kernel.uses_dynamic_stack() && kernel.explicit_arguments().len() == 11
        && kernel.explicit_arguments().iter().enumerate().all(|(index, arg)| Argument::from(arg).matches(index))
        && kernel.hidden_arguments().len() == hidden.len()
        && kernel.hidden_arguments().iter().zip(hidden).all(|(arg, expected)| (arg.offset(), arg.size(), arg.value_kind()) == expected)
}

#[derive(Clone, Copy)]
struct Argument {
    offset: u64, size: u64, kind: fe2o3_hsaco::ExplicitValueKind,
    address_space: Option<fe2o3_hsaco::ArgumentAddressSpace>, alignment: Option<u64>, pointee_alignment: Option<u64>,
    access: Option<fe2o3_hsaco::ArgumentAccess>, actual_access: Option<fe2o3_hsaco::ArgumentAccess>,
    value_type: Option<fe2o3_hsaco::ExplicitValueType>,
}

impl From<&fe2o3_hsaco::ExplicitArgument> for Argument {
    fn from(arg: &fe2o3_hsaco::ExplicitArgument) -> Self {
        Self { offset: arg.offset(), size: arg.size(), kind: arg.value_kind(), address_space: arg.address_space(),
            alignment: arg.alignment(), pointee_alignment: arg.pointee_alignment(), access: arg.access(),
            actual_access: arg.actual_access(), value_type: arg.value_type() }
    }
}

impl Argument {
    fn matches(self, index: usize) -> bool {
        use fe2o3_hsaco::{ArgumentAddressSpace::Global, ExplicitValueKind::{ByValue, GlobalBuffer}};
        if index >= 11 { return false; }
        let pointer = index < 6 && index.is_multiple_of(2);
        let (offset, size) = if index < 6 { (index * 8, 8) } else { (48 + (index - 6) * 4, 4) };
        self.offset == offset as u64 && self.size == size
            && self.kind == (if pointer { GlobalBuffer } else { ByValue })
            && self.address_space == (if pointer { Some(Global) } else { None })
            && self.alignment.is_none() && self.pointee_alignment.is_none()
            && self.access.is_none() && self.actual_access.is_none() && self.value_type.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_c4c_roster_is_exact_distinct_and_only_metadata_crosses_sdks() {
        assert_eq!(V20_COMPILER_NAMES.len(), 2);
        assert!(V20_COMPILER_NAMES.iter().all(|(logical, export)| !logical.is_empty() && !export.is_empty()));
        assert_ne!(V20_COMPILER_NAMES[0].0, V20_COMPILER_NAMES[1].0);
        assert_ne!(V20_COMPILER_NAMES[0].1, V20_COMPILER_NAMES[1].1);
        assert!(super::super::exact_roster(V20_COMPILER_NAMES.iter().map(|entry| entry.1), &ENGINEERING_TP_GEMV_PREFETCH_EXPORTS_V20));
        assert!(ENGINEERING_TP_GEMV_PREFETCH_EXPORTS_V20.iter().all(|root| !super::super::ENGINEERING_TP_BATCH32_EXPORTS_V5.contains(root)));
    }

    #[test]
    fn argument_profile_rejects_every_layout_and_optional_qualifier_mutation() {
        use fe2o3_hsaco::{ArgumentAccess, ArgumentAddressSpace, ExplicitValueKind, ExplicitValueType};
        for index in 0_usize..11 {
            let pointer = index < 6 && index.is_multiple_of(2);
            let valid = Argument { offset: (if index < 6 { index * 8 } else { 48 + (index - 6) * 4 }) as u64,
                size: if index < 6 { 8 } else { 4 }, kind: if pointer { ExplicitValueKind::GlobalBuffer } else { ExplicitValueKind::ByValue },
                address_space: pointer.then_some(ArgumentAddressSpace::Global), alignment: None, pointee_alignment: None,
                access: None, actual_access: None, value_type: None };
            assert!(valid.matches(index));
            assert!(!valid.matches(11));
            for mutation in 0..10 {
                let mut bad = valid;
                match mutation {
                    0 => bad.offset += 1, 1 => bad.size += 1, 2 => bad.kind = ExplicitValueKind::DynamicSharedPointer,
                    3 => bad.address_space = Some(ArgumentAddressSpace::Local),
                    4 => bad.address_space = if pointer { None } else { Some(ArgumentAddressSpace::Global) },
                    5 => bad.alignment = Some(8), 6 => bad.pointee_alignment = Some(2),
                    7 => bad.access = Some(ArgumentAccess::ReadOnly), 8 => bad.actual_access = Some(ArgumentAccess::WriteOnly),
                    9 => bad.value_type = Some(ExplicitValueType::F32), _ => unreachable!(),
                }
                assert!(!bad.matches(index), "argument {index}, mutation {mutation}");
            }
        }
    }

    #[test]
    #[ignore = "requires the actual emitted V20 image; host admission only"]
    fn emitted_image_has_its_own_abi_and_identity() {
        let path = std::path::PathBuf::from(std::env::var_os("FERRIC_TEST_GEMV_PREFETCH_V20_ARTIFACT").expect("explicit V20 image"));
        let artifact = EngineeringTpArtifactV1::open_gemv_prefetch_v20(&path).unwrap();
        let binding = artifact.gemv_prefetch_binding_v20().unwrap();
        assert_eq!(binding.hsaco, *artifact.hsaco_id().as_bytes());
        assert_eq!(binding.manifest, *artifact.manifest_id().as_bytes());
        assert_eq!(binding.handoff, *artifact.handoff_id().as_bytes());
        assert!(artifact.query_hoist_binding_v14().is_none());
        assert!(artifact.c1_kv_copy_binding_v19().is_none());
        assert!(EngineeringTpArtifactV1::open_query_hoist_v14(&path).is_err());
    }
}
