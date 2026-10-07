//! Separate exact-image admission for the unpaired split-K down experiment.

use super::{EngineeringTpArtifactV1, M1EngineeringAggregateArtifactOpenErrorV1};
use std::path::Path;

/// Ordered partial and merge exports; not a numerical or performance proof.
pub const ENGINEERING_TP_SPLITK_DOWN_EXPORTS_R1: [&str; 2] = [
    "ferric_qwen3_c1_down_splitk8_mfma_partial_f32_r1",
    "ferric_qwen3_c1_down_splitk8_merge_f32_r1",
];

/// Exact separately retained code object and its compiler evidence identities.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EngineeringTpSplitKDownImageIdsR1 {
    /// Code object SHA-256.
    pub hsaco: [u8; 32],
    /// Observation manifest SHA-256.
    pub manifest: [u8; 32],
    /// Compiler handoff SHA-256.
    pub handoff: [u8; 32],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SplitKDownBindingR1(EngineeringTpSplitKDownImageIdsR1);

impl SplitKDownBindingR1 {
    pub(crate) const fn hsaco(self) -> [u8; 32] {
        self.0.hsaco
    }

    #[cfg(test)]
    pub(crate) const fn recording() -> Self {
        Self(EngineeringTpSplitKDownImageIdsR1 {
            hsaco: [231; 32],
            manifest: [232; 32],
            handoff: [233; 32],
        })
    }
}

/// Private binding is minted only by the exact artifact opener.
pub struct EngineeringTpSplitKDownArtifactR1 {
    artifact: EngineeringTpArtifactV1,
    binding: SplitKDownBindingR1,
}

impl EngineeringTpSplitKDownArtifactR1 {
    /// Opens the closed two-root ABI/resource profile observed in the component image.
    /// # Errors
    /// Rejects missing pins or mismatched roster, target, physical ABI or resources.
    pub fn open(
        root: &Path,
        compiler_names: &[(&str, &str); 2],
        expected: EngineeringTpSplitKDownImageIdsR1,
    ) -> Result<Self, M1EngineeringAggregateArtifactOpenErrorV1> {
        use M1EngineeringAggregateArtifactOpenErrorV1 as Error;
        if compiler_names
            .iter()
            .any(|(logical, export)| logical.is_empty() || export.is_empty())
            || compiler_names[0].0 == compiler_names[1].0
            || compiler_names
                .iter()
                .map(|entry| entry.1)
                .ne(ENGINEERING_TP_SPLITK_DOWN_EXPORTS_R1)
            || [expected.hsaco, expected.manifest, expected.handoff].contains(&[0; 32])
        {
            return Err(Error::CurrentFerricDescriptorRoster);
        }
        let artifact = EngineeringTpArtifactV1::open_profile_names(
            root,
            compiler_names,
            "ferric_qwen3_tp_c1_splitk_down_kernels_device_v1",
            &ENGINEERING_TP_SPLITK_DOWN_EXPORTS_R1,
        )?;
        let actual = EngineeringTpSplitKDownImageIdsR1 {
            hsaco: *artifact.hsaco_id.as_bytes(),
            manifest: *artifact.manifest_id.as_bytes(),
            handoff: *artifact.handoff_id.as_bytes(),
        };
        if actual != expected {
            return Err(Error::ManifestPolicy {
                field: "splitk_down_r1.expected_image_identities",
            });
        }
        if !artifact.inspection.hsaco().kernels().iter().all(|kernel| {
            let (slices, scalars, implicit, agprs, sgprs, vgprs) = match kernel.name() {
                "ferric_qwen3_c1_down_splitk8_mfma_partial_f32_r1" => (3, 5, 72, 4, 30, 24),
                "ferric_qwen3_c1_down_splitk8_merge_f32_r1" => (2, 0, 32, 0, 23, 12),
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
            binding: SplitKDownBindingR1(actual),
        })
    }

    /// Exact admitted bytes for the existing worker image loader.
    #[must_use]
    pub const fn artifact(&self) -> &EngineeringTpArtifactV1 {
        &self.artifact
    }

    pub(crate) const fn binding(&self) -> SplitKDownBindingR1 {
        self.binding
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splitk_image_rejects_wrong_root_order_and_placeholder_pins_before_open() {
        let names = ENGINEERING_TP_SPLITK_DOWN_EXPORTS_R1.map(|name| (name, name));
        for mutation in 0..7 {
            let mut changed = names;
            let mut ids = SplitKDownBindingR1::recording().0;
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
                EngineeringTpSplitKDownArtifactR1::open(Path::new("/unused"), &changed, ids,),
                Err(M1EngineeringAggregateArtifactOpenErrorV1::CurrentFerricDescriptorRoster)
            ));
        }
    }

    #[test]
    #[ignore = "requires exact retained unpaired split-K image; CPU admission only"]
    fn splitk_actual_component_image_admits_exact_physical_abi_and_resources() {
        use sha2::{Digest, Sha256};
        let root = std::path::PathBuf::from(
            std::env::var_os("FERRIC_TEST_SPLITK_DOWN_R1_ARTIFACT")
                .expect("explicit retained image required"),
        );
        let hash = |hex: &str| -> [u8; 32] {
            std::array::from_fn(|index| {
                u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).unwrap()
            })
        };
        let ids = EngineeringTpSplitKDownImageIdsR1 {
            hsaco: hash("1b16379c91c945883bfc9aecdbde896853573a92546acd228eba6e232d746cae"),
            manifest: hash("2adf8348e129446ab3b1f80d328eafa6b3bf7eed53561681e0a972d3852fdf54"),
            handoff: hash("cd23060449750a973f05742c0d5a043cb637491743e2ac56b56e847cc2c4dbdf"),
        };
        let image = root.join("observation.hsaco");
        assert_eq!(std::fs::symlink_metadata(&image).unwrap().len(), 12_832);
        let bytes = std::fs::read(&image).unwrap();
        assert_eq!(Sha256::digest(&bytes).as_slice(), ids.hsaco);
        let inspection = fe2o3_hsaco_finalize::inspect_finalized(&bytes).unwrap();
        let names = ENGINEERING_TP_SPLITK_DOWN_EXPORTS_R1.map(|export| {
            let descriptor = inspection
                .descriptor_table()
                .kernels()
                .iter()
                .find(|entry| entry.entry_name().as_str() == export)
                .unwrap();
            (descriptor.logical_name().as_str(), export)
        });
        let admitted = EngineeringTpSplitKDownArtifactR1::open(&root, &names, ids).unwrap();
        println!(
            "native down compiler roster {}",
            serde_json::to_string(&names.map(|(logical, export)| {
                serde_json::json!({"logical_name":logical,"export_name":export})
            }))
            .unwrap()
        );
        assert_eq!(admitted.binding().hsaco(), ids.hsaco);
        for kernel in admitted.artifact().inspection().hsaco().kernels() {
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
            assert!(EngineeringTpSplitKDownArtifactR1::open(&root, &names, changed).is_err());
        }
        assert!(EngineeringTpArtifactV1::open_gemv_prefetch_v20(&root).is_err());
    }
}
