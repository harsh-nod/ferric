//! Separate exact-image admission for the split-K4 gate/up experiment.

use super::{EngineeringTpArtifactV1, M1EngineeringAggregateArtifactOpenErrorV1};
use std::path::Path;

/// Ordered partial and merge exports; not a numerical or performance proof.
pub const ENGINEERING_TP_SPLITK_GATE_UP_EXPORTS_R1: [&str; 2] = [
    "ferric_qwen3_c1_gate_up_splitk4_mfma_partial_f32_r1",
    "ferric_qwen3_c1_gate_up_splitk4_merge_bf16_r1",
];

/// Exact separately retained code object and its compiler evidence identities.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EngineeringTpSplitKGateUpImageIdsR1 {
    /// Code object SHA-256.
    pub hsaco: [u8; 32],
    /// Observation manifest SHA-256.
    pub manifest: [u8; 32],
    /// Compiler handoff SHA-256.
    pub handoff: [u8; 32],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SplitKGateUpBindingR1(EngineeringTpSplitKGateUpImageIdsR1);

impl SplitKGateUpBindingR1 {
    pub(crate) const fn hsaco(self) -> [u8; 32] {
        self.0.hsaco
    }

    #[cfg(test)]
    pub(crate) const fn recording() -> Self {
        Self(EngineeringTpSplitKGateUpImageIdsR1 {
            hsaco: [231; 32],
            manifest: [232; 32],
            handoff: [233; 32],
        })
    }
}

/// Private binding is minted only by the exact artifact opener.
pub struct EngineeringTpSplitKGateUpArtifactR1 {
    artifact: EngineeringTpArtifactV1,
    binding: SplitKGateUpBindingR1,
}

impl EngineeringTpSplitKGateUpArtifactR1 {
    /// Opens the closed two-root ABI/resource profile observed in the component image.
    /// # Errors
    /// Rejects missing pins or mismatched roster, target, physical ABI or resources.
    pub fn open(
        root: &Path,
        compiler_names: &[(&str, &str); 2],
        expected: EngineeringTpSplitKGateUpImageIdsR1,
    ) -> Result<Self, M1EngineeringAggregateArtifactOpenErrorV1> {
        use M1EngineeringAggregateArtifactOpenErrorV1 as Error;
        if compiler_names
            .iter()
            .any(|(logical, export)| logical.is_empty() || export.is_empty())
            || compiler_names[0].0 == compiler_names[1].0
            || compiler_names
                .iter()
                .map(|entry| entry.1)
                .ne(ENGINEERING_TP_SPLITK_GATE_UP_EXPORTS_R1)
            || [expected.hsaco, expected.manifest, expected.handoff].contains(&[0; 32])
        {
            return Err(Error::CurrentFerricDescriptorRoster);
        }
        let artifact = EngineeringTpArtifactV1::open_profile_names(
            root,
            compiler_names,
            "ferric_qwen3_tp_c1_splitk_gate_up_kernels_device_v1",
            &ENGINEERING_TP_SPLITK_GATE_UP_EXPORTS_R1,
        )?;
        let actual = EngineeringTpSplitKGateUpImageIdsR1 {
            hsaco: *artifact.hsaco_id.as_bytes(),
            manifest: *artifact.manifest_id.as_bytes(),
            handoff: *artifact.handoff_id.as_bytes(),
        };
        if actual != expected {
            return Err(Error::ManifestPolicy {
                field: "splitk_gate_up_r1.expected_image_identities",
            });
        }
        if !artifact.inspection.hsaco().kernels().iter().all(|kernel| {
            let (slices, scalars, implicit, agprs, sgprs, vgprs) = match kernel.name() {
                "ferric_qwen3_c1_gate_up_splitk4_mfma_partial_f32_r1" => (3, 5, 72, 4, 36, 28),
                "ferric_qwen3_c1_gate_up_splitk4_merge_bf16_r1" => (2, 0, 32, 0, 25, 9),
                _ => return false,
            };
            super::packed_bf16_r2::metadata_matches_layout_with_agprs(
                kernel, slices, scalars, implicit, agprs,
            ) && kernel.sgpr_count() == sgprs
                && kernel.vgpr_count() == vgprs
        }) {
            return Err(Error::HsacoProfile);
        }
        Ok(Self {
            artifact,
            binding: SplitKGateUpBindingR1(actual),
        })
    }

    /// Exact admitted bytes for the existing worker image loader.
    #[must_use]
    pub const fn artifact(&self) -> &EngineeringTpArtifactV1 {
        &self.artifact
    }

    pub(crate) const fn binding(&self) -> SplitKGateUpBindingR1 {
        self.binding
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splitk_image_rejects_wrong_root_order_and_placeholder_pins_before_open() {
        let names = ENGINEERING_TP_SPLITK_GATE_UP_EXPORTS_R1.map(|name| (name, name));
        for mutation in 0..7 {
            let mut changed = names;
            let mut ids = SplitKGateUpBindingR1::recording().0;
            match mutation {
                0 => ids.hsaco = [0; 32],
                1 => ids.manifest = [0; 32],
                2 => ids.handoff = [0; 32],
                3 => changed.swap(0, 1),
                4 => changed[1].0 = changed[0].0,
                5 => changed[0].1 = super::super::ENGINEERING_TP_PACKED_DOWN_EXPORTS_R1[0],
                _ => changed[0].0 = "",
            }
            assert!(matches!(
                EngineeringTpSplitKGateUpArtifactR1::open(Path::new("/unused"), &changed, ids,),
                Err(M1EngineeringAggregateArtifactOpenErrorV1::CurrentFerricDescriptorRoster)
            ));
        }
    }

    #[test]
    #[ignore = "requires exact retained split-K4 gate/up image; CPU admission only"]
    fn splitk_actual_component_image_admits_exact_physical_abi_and_resources() {
        use sha2::{Digest, Sha256};
        let root = std::path::PathBuf::from(
            std::env::var_os("FERRIC_TEST_SPLITK_GATE_UP_R1_ARTIFACT")
                .expect("explicit retained image required"),
        );
        let hash = |hex: &str| -> [u8; 32] {
            std::array::from_fn(|index| {
                u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).unwrap()
            })
        };
        let ids = EngineeringTpSplitKGateUpImageIdsR1 {
            hsaco: hash("d28610d291eeec0589afbf269e26d21b7111c96f08106e1f661d6a66f024bf03"),
            manifest: hash("a7417322dafe8e0af927a6457f0dea2c324e94ec2692329f7d76a215ed3273bb"),
            handoff: hash("ea8672a0acfcfef4606c6597f6b7f3af0fbbfcd9634d8feffe5d29d5f11b39b9"),
        };
        let image = root.join("observation.hsaco");
        assert_eq!(std::fs::symlink_metadata(&image).unwrap().len(), 13_136);
        let bytes = std::fs::read(&image).unwrap();
        assert_eq!(Sha256::digest(&bytes).as_slice(), ids.hsaco);
        let inspection = fe2o3_hsaco_finalize::inspect_finalized(&bytes).unwrap();
        let names = ENGINEERING_TP_SPLITK_GATE_UP_EXPORTS_R1.map(|export| {
            let descriptor = inspection
                .descriptor_table()
                .kernels()
                .iter()
                .find(|entry| entry.entry_name().as_str() == export)
                .unwrap();
            (descriptor.logical_name().as_str(), export)
        });
        let admitted = EngineeringTpSplitKGateUpArtifactR1::open(&root, &names, ids).unwrap();
        println!(
            "native gate/up compiler roster {}",
            serde_json::json!(names.map(|(logical_name, export_name)|
            serde_json::json!({"logical_name":logical_name,"export_name":export_name})))
        );
        assert_eq!(admitted.binding().hsaco(), ids.hsaco);
        for kernel in admitted.artifact().inspection().hsaco().kernels() {
            if kernel.name() == ENGINEERING_TP_SPLITK_GATE_UP_EXPORTS_R1[0] {
                assert!(!super::super::packed_bf16_r2::metadata_matches_layout(
                    kernel, 3, 5, 72
                ));
                assert!(
                    super::super::packed_bf16_r2::metadata_matches_layout_with_agprs(
                        kernel, 3, 5, 72, 4
                    )
                );
            } else {
                assert!(super::super::packed_bf16_r2::metadata_matches_layout(
                    kernel, 2, 0, 32
                ));
            }
            println!(
                "split-K actual ABI {} kernarg={} explicit={} agpr={:?}",
                kernel.name(),
                kernel.kernarg_segment_size(),
                kernel.explicit_arguments().len(),
                kernel.agpr_count()
            );
        }
        for mutation in 0..3 {
            let mut changed = ids;
            match mutation {
                0 => changed.hsaco[0] ^= 1,
                1 => changed.manifest[0] ^= 1,
                _ => changed.handoff[0] ^= 1,
            }
            assert!(EngineeringTpSplitKGateUpArtifactR1::open(&root, &names, changed).is_err());
        }
        assert!(EngineeringTpArtifactV1::open_gemv_prefetch_v20(&root).is_err());
    }
}
