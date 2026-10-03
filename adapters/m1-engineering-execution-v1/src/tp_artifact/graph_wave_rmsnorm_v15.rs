//! Exact V15 sidecar admission for the graph-only matched-residency profiles.
use super::{EngineeringTpArtifactV1, M1EngineeringAggregateArtifactOpenErrorV1};
use std::path::Path;

const IMAGE: [u8; 32] = [
    195, 56, 130, 219, 26, 252, 216, 235, 38, 236, 2, 188, 67, 234, 33, 68, 175, 50, 45, 41, 191,
    78, 14, 146, 30, 3, 234, 116, 173, 197, 90, 246,
];

impl EngineeringTpArtifactV1 {
    /// Uses the existing ordinary V15 source-roster and complete ABI admission.
    /// # Errors
    /// Rejects another image or resources, even if its symbol and ABI are compatible.
    pub fn open_graph_wave_rmsnorm_v15(
        root: &Path,
    ) -> Result<Self, M1EngineeringAggregateArtifactOpenErrorV1> {
        let artifact = Self::open_wave_rmsnorm_v15(root)?;
        if !artifact.is_graph_wave_rmsnorm_v15() {
            return Err(M1EngineeringAggregateArtifactOpenErrorV1::HsacoProfile);
        }
        Ok(artifact)
    }

    /// Checks the already admitted exact sidecar before graph registration.
    #[must_use]
    pub fn is_graph_wave_rmsnorm_v15(&self) -> bool {
        let kernels = self.inspection.hsaco().kernels();
        self.wave_rmsnorm_binding_v15().is_some()
            && self.hsaco_id().as_bytes() == &IMAGE
            && kernels.len() == 1
            && kernels[0].name() == super::ENGINEERING_TP_WAVE_RMSNORM_EXPORTS_V15[0]
            && kernels[0].sgpr_count() == 74
            && kernels[0].vgpr_count() == 24
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_v15_missing_artifact_is_rejected_before_registration() {
        assert!(
            EngineeringTpArtifactV1::open_graph_wave_rmsnorm_v15(Path::new(
                "/ferric-missing-graph-v15-artifact"
            ))
            .is_err()
        );
    }

    #[test]
    #[ignore = "CPU-only ordinary image admission requires pinned artifact paths"]
    fn graph_v15_admits_actual_image_and_rejects_other_profiles() {
        let path = std::env::var_os("FERRIC_PREPARED_V15_ARTIFACT").unwrap();
        let image = EngineeringTpArtifactV1::open_graph_wave_rmsnorm_v15(Path::new(&path)).unwrap();
        assert!(image.is_graph_wave_rmsnorm_v15());
        assert_eq!(image.hsaco_id().as_bytes(), &IMAGE);
        for name in [
            "FERRIC_PREPARED_MAIN_ARTIFACT",
            "FERRIC_PREPARED_PEER_ARTIFACT",
            "FERRIC_PREPARED_V22_ARTIFACT",
        ] {
            let path = std::env::var_os(name).unwrap();
            assert!(
                EngineeringTpArtifactV1::open_graph_wave_rmsnorm_v15(Path::new(&path)).is_err()
            );
        }
    }
}
