//! Exact, separately supplied packed-down image admission; no default promotion.

use super::{EngineeringTpArtifactV1, M1EngineeringAggregateArtifactOpenErrorV1};
use std::path::Path;

/// The FP32-output down projection and its integer-only activation pack.
pub const ENGINEERING_TP_PACKED_DOWN_EXPORTS_R1: [&str; 2] = [
    "ferric_qwen3_c1_down_wave_gemv_packed_u32_f32_r1",
    "ferric_qwen3_c1_down_activation_pack_u32_r1",
];

/// Observed code-object, manifest and handoff identities, not a correctness proof.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EngineeringTpPackedDownImageIdsR1 {
    /// Exact code-object bytes.
    pub hsaco: [u8; 32],
    /// Exact observation manifest.
    pub manifest: [u8; 32],
    /// Exact compiler handoff.
    pub handoff: [u8; 32],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PackedDownBindingR1(EngineeringTpPackedDownImageIdsR1);

impl PackedDownBindingR1 {
    pub(crate) const fn hsaco(self) -> [u8; 32] {
        self.0.hsaco
    }

    #[cfg(test)]
    pub(crate) const fn recording() -> Self {
        Self(EngineeringTpPackedDownImageIdsR1 {
            hsaco: [241; 32],
            manifest: [242; 32],
            handoff: [243; 32],
        })
    }
}

/// Only the exact content-addressed opener creates a production image binding.
pub struct EngineeringTpPackedDownArtifactR1 {
    artifact: EngineeringTpArtifactV1,
    binding: PackedDownBindingR1,
}

impl EngineeringTpPackedDownArtifactR1 {
    /// Opens the actual qualified logical/export roster with fixed ABI/resources.
    /// # Errors
    /// Rejects identity, roster, target, descriptor, ABI or resource disagreement.
    pub fn open(
        root: &Path,
        compiler_names: &[(&str, &str); 2],
        expected: EngineeringTpPackedDownImageIdsR1,
    ) -> Result<Self, M1EngineeringAggregateArtifactOpenErrorV1> {
        use M1EngineeringAggregateArtifactOpenErrorV1 as Error;
        if compiler_names
            .iter()
            .any(|(logical, export)| logical.is_empty() || export.is_empty())
            || compiler_names[0].0 == compiler_names[1].0
            || compiler_names
                .iter()
                .map(|entry| entry.1)
                .ne(ENGINEERING_TP_PACKED_DOWN_EXPORTS_R1)
            || [expected.hsaco, expected.manifest, expected.handoff].contains(&[0; 32])
        {
            return Err(Error::CurrentFerricDescriptorRoster);
        }
        let artifact = EngineeringTpArtifactV1::open_profile_names(
            root,
            compiler_names,
            "ferric_qwen3_down_f32_packed_u32_proposal",
            &ENGINEERING_TP_PACKED_DOWN_EXPORTS_R1,
        )?;
        let actual = EngineeringTpPackedDownImageIdsR1 {
            hsaco: *artifact.hsaco_id.as_bytes(),
            manifest: *artifact.manifest_id.as_bytes(),
            handoff: *artifact.handoff_id.as_bytes(),
        };
        if actual != expected {
            return Err(Error::ManifestPolicy {
                field: "packed_down_r1.expected_image_identities",
            });
        }
        if !artifact.inspection.hsaco().kernels().iter().all(|kernel| {
            let (slices, scalars, implicit) = match kernel.name() {
                "ferric_qwen3_c1_down_wave_gemv_packed_u32_f32_r1" => (3, 5, 72),
                "ferric_qwen3_c1_down_activation_pack_u32_r1" => (2, 2, 40),
                _ => return false,
            };
            super::packed_bf16_r2::metadata_matches_layout(kernel, slices, scalars, implicit)
        }) {
            return Err(Error::HsacoProfile);
        }
        Ok(Self {
            artifact,
            binding: PackedDownBindingR1(actual),
        })
    }

    /// Exact admitted bytes for the existing worker image loader.
    #[must_use]
    pub const fn artifact(&self) -> &EngineeringTpArtifactV1 {
        &self.artifact
    }

    pub(crate) const fn binding(&self) -> PackedDownBindingR1 {
        self.binding
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packed_down_rejects_placeholder_ids_and_wrong_rosters_before_open() {
        let names = [
            ("down", ENGINEERING_TP_PACKED_DOWN_EXPORTS_R1[0]),
            ("pack", ENGINEERING_TP_PACKED_DOWN_EXPORTS_R1[1]),
        ];
        for mutation in 0..7 {
            let mut bad = names;
            let mut ids = PackedDownBindingR1::recording().0;
            match mutation {
                0 => ids.hsaco = [0; 32],
                1 => ids.manifest = [0; 32],
                2 => ids.handoff = [0; 32],
                3 => bad.swap(0, 1),
                4 => bad[1].0 = bad[0].0,
                5 => bad[0].1 = super::super::ENGINEERING_TP_PACKED_BF16_EXPORTS_R2[0],
                _ => bad[0].0 = "",
            }
            assert!(matches!(
                EngineeringTpPackedDownArtifactR1::open(Path::new("/unused"), &bad, ids),
                Err(M1EngineeringAggregateArtifactOpenErrorV1::CurrentFerricDescriptorRoster)
            ));
        }
    }

    #[test]
    #[ignore = "requires the separately retained 962 packed-down image; host admission only"]
    fn packed_down_actual_qualified_image_binds_exact_abi_and_identities() {
        let root = std::path::PathBuf::from(
            std::env::var_os("FERRIC_TEST_PACKED_DOWN_R1_ARTIFACT")
                .expect("explicit retained packed-down image required"),
        );
        let hash = |hex: &str| -> [u8; 32] {
            std::array::from_fn(|index| {
                u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).unwrap()
            })
        };
        let ids = EngineeringTpPackedDownImageIdsR1 {
            hsaco: hash("e9b5112b7fbf1ea3b374c7964ca8a91ca0593dcfb7ed082c7a067ad584c20192"),
            manifest: hash("53b04e2db09f3ba39504793f1a9cc5d4fe48a4af0afb8f95ae48f599e16ac781"),
            handoff: hash("249262e9bb828a8bc4600d5a6e8920bc67192c712ad73c98d77439da03b1020f"),
        };
        let names = ENGINEERING_TP_PACKED_DOWN_EXPORTS_R1.map(|name| (name, name));
        let artifact = EngineeringTpPackedDownArtifactR1::open(&root, &names, ids).unwrap();
        assert_eq!(artifact.binding().hsaco(), ids.hsaco);
        assert_eq!(*artifact.artifact().manifest_id().as_bytes(), ids.manifest);
        assert_eq!(*artifact.artifact().handoff_id().as_bytes(), ids.handoff);
        for mutation in 0..3 {
            let mut changed = ids;
            match mutation {
                0 => changed.hsaco[0] ^= 1,
                1 => changed.manifest[0] ^= 1,
                _ => changed.handoff[0] ^= 1,
            }
            assert!(EngineeringTpPackedDownArtifactR1::open(&root, &names, changed).is_err());
        }
        assert!(EngineeringTpArtifactV1::open_c1_kv_copy_v19(&root).is_err());
    }
}
