//! Ordinary admission of the existing V22 image for the separate TP2 graph path.
use super::{EngineeringTpArtifactV1, M1EngineeringAggregateArtifactOpenErrorV1};
use fe2o3_hsaco::{ExplicitArgument, HiddenValueKind, InspectedKernel};
use std::path::Path;

/// The graph sidecar is not the scalar BF16 or FP32 argmax image.
pub const ENGINEERING_TP_GRAPH_ARGMAX_ROOT_V22: &str =
    "ferric_qwen3_tp_single_wave_argmax_bf16_v22";
const CRATE: &str = "ferric_qwen3_tp_bf16_wave_argmax_kernels_device_v22";
const IMAGE: [u8; 32] = [
    202, 88, 100, 42, 86, 234, 74, 82, 41, 114, 149, 35, 4, 209, 62, 141, 191, 223, 97, 250, 133,
    235, 23, 93, 59, 48, 200, 45, 185, 152, 57, 252,
];

impl EngineeringTpArtifactV1 {
    /// Opens the real V22 source roster through the existing ordinary loader.
    /// This does not change any TP1 driver profile or permit a GPU dispatch.
    /// # Errors
    /// Rejects foreign source rosters, image identity, ABI or observed resources.
    pub fn open_graph_bf16_argmax_v22(
        root: &Path,
    ) -> Result<Self, M1EngineeringAggregateArtifactOpenErrorV1> {
        let artifact = Self::open_profile(
            root,
            &ferric_qwen3_tp_bf16_wave_argmax_kernels_device_v22::compiler_expectation_roster_v22(),
            CRATE,
            &[ENGINEERING_TP_GRAPH_ARGMAX_ROOT_V22],
        )?;
        if !artifact.is_graph_bf16_argmax_v22() {
            return Err(M1EngineeringAggregateArtifactOpenErrorV1::HsacoProfile);
        }
        Ok(artifact)
    }

    /// Checks the retained, already admitted sidecar before graph registration.
    #[must_use]
    pub fn is_graph_bf16_argmax_v22(&self) -> bool {
        let kernels = self.inspection.hsaco().kernels();
        self.hsaco_id().as_bytes() == &IMAGE && kernels.len() == 1 && metadata_matches(&kernels[0])
    }
}

fn metadata_matches(kernel: &InspectedKernel) -> bool {
    let hidden = hidden_layout();
    kernel.name() == ENGINEERING_TP_GRAPH_ARGMAX_ROOT_V22
        && kernel.kernarg_segment_size() == 296
        && kernel.kernarg_segment_alignment() == 8
        && kernel.implicit_argument_offset() == Some(40)
        && kernel.implicit_argument_size() == 256
        && kernel.wavefront_size() == 64
        && kernel.max_flat_workgroup_size() == 64
        && kernel.required_workgroup_size() == Some([64, 1, 1])
        && kernel.group_segment_fixed_size() == 0
        && kernel.private_segment_fixed_size() == 0
        && kernel.sgpr_count() == 25
        && kernel.vgpr_count() == 13
        && kernel.agpr_count() == Some(0)
        && kernel.sgpr_spill_count() == Some(0)
        && kernel.vgpr_spill_count() == Some(0)
        && !kernel.uses_dynamic_stack()
        && kernel.explicit_arguments().len() == 5
        && kernel
            .explicit_arguments()
            .iter()
            .enumerate()
            .all(|(index, argument)| argument_matches(argument, index))
        && kernel.hidden_arguments().len() == hidden.len()
        && kernel
            .hidden_arguments()
            .iter()
            .zip(hidden)
            .all(|(argument, wanted)| {
                (argument.offset(), argument.size(), argument.value_kind()) == wanted
            })
}

fn argument_matches(argument: &ExplicitArgument, index: usize) -> bool {
    use fe2o3_hsaco::{
        ArgumentAddressSpace::Global,
        ExplicitValueKind::{ByValue, GlobalBuffer},
    };
    let pointer = matches!(index, 0 | 2);
    index < 5
        && argument.offset() == [0, 8, 16, 24, 32][index]
        && argument.size() == if index < 4 { 8 } else { 4 }
        && argument.value_kind() == if pointer { GlobalBuffer } else { ByValue }
        && argument.address_space() == if pointer { Some(Global) } else { None }
        && argument.alignment().is_none()
        && argument.pointee_alignment().is_none()
        && argument.access().is_none()
        && argument.actual_access().is_none()
        && argument.value_type().is_none()
}

fn hidden_layout() -> [(u64, u64, HiddenValueKind); 13] {
    use HiddenValueKind::{
        BlockCountX, BlockCountY, BlockCountZ, GlobalOffsetX, GlobalOffsetY, GlobalOffsetZ,
        GridDimensions, GroupSizeX, GroupSizeY, GroupSizeZ, RemainderX, RemainderY, RemainderZ,
    };
    [
        (40, 4, BlockCountX),
        (44, 4, BlockCountY),
        (48, 4, BlockCountZ),
        (52, 2, GroupSizeX),
        (54, 2, GroupSizeY),
        (56, 2, GroupSizeZ),
        (58, 2, RemainderX),
        (60, 2, RemainderY),
        (62, 2, RemainderZ),
        (80, 8, GlobalOffsetX),
        (88, 8, GlobalOffsetY),
        (96, 8, GlobalOffsetZ),
        (104, 2, GridDimensions),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_v22_missing_artifact_is_rejected_before_registration() {
        assert!(
            EngineeringTpArtifactV1::open_graph_bf16_argmax_v22(Path::new(
                "/ferric-missing-graph-v22-artifact"
            ),)
            .is_err()
        );
    }

    #[test]
    #[ignore = "CPU-only real ordinary image admission requires pinned artifact paths"]
    fn graph_v22_admits_actual_image_and_rejects_other_profiles() {
        let path = std::env::var_os("FERRIC_PREPARED_V22_ARTIFACT").unwrap();
        let image = EngineeringTpArtifactV1::open_graph_bf16_argmax_v22(Path::new(&path)).unwrap();
        assert!(image.is_graph_bf16_argmax_v22());
        assert_eq!(image.hsaco_id().as_bytes(), &IMAGE);
        assert_eq!(image.inspection().hsaco().kernels().len(), 1);
        for name in [
            "FERRIC_PREPARED_MAIN_ARTIFACT",
            "FERRIC_PREPARED_PEER_ARTIFACT",
        ] {
            let path = std::env::var_os(name).unwrap();
            assert!(EngineeringTpArtifactV1::open_graph_bf16_argmax_v22(Path::new(&path)).is_err());
        }
    }
}
