//! Ordinary admission of the exact two-root split-context attention sidecar.
use super::{EngineeringTpArtifactV1, M1EngineeringAggregateArtifactOpenErrorV1};
use fe2o3_hsaco::{ExplicitArgument, HiddenValueKind, InspectedKernel};
use std::path::Path;

/// Exact partial then merge roots; neither accepts the legacy attention ABI.
pub const ENGINEERING_TP_GRAPH_SPLIT_ATTENTION_ROOTS_V1: [&str; 2] = [
    "ferric_qwen3_tp_split_context_partial_bf16_v1",
    "ferric_qwen3_tp_split_context_merge_bf16_v1",
];
const CRATE: &str = "ferric_qwen3_tp_split_context_attention_device_v1";
const IMAGE: [u8; 32] = [
    0x01, 0x10, 0xd1, 0x4a, 0xdc, 0x42, 0x4e, 0xa3, 0xff, 0x43, 0x9c, 0x6d, 0xee, 0x86, 0x62, 0xa0,
    0xaf, 0x69, 0xbb, 0x73, 0x8c, 0xdf, 0x05, 0xca, 0x98, 0xaa, 0x5d, 0x2f, 0x49, 0x36, 0xf7, 0x66,
];

impl EngineeringTpArtifactV1 {
    /// Uses the immutable device source roster and ordinary artifact loader.
    /// # Errors
    /// Rejects any changed source, image, ABI or observed resource contract.
    pub fn open_graph_split_attention_v1(
        root: &Path,
    ) -> Result<Self, M1EngineeringAggregateArtifactOpenErrorV1> {
        let artifact = Self::open_profile(
            root,
            &ferric_qwen3_tp_split_context_attention_device_v1::compiler_expectation_roster_split_context_v1(),
            CRATE,
            &ENGINEERING_TP_GRAPH_SPLIT_ATTENTION_ROOTS_V1,
        )?;
        if !artifact.is_graph_split_attention_v1() {
            return Err(M1EngineeringAggregateArtifactOpenErrorV1::HsacoProfile);
        }
        Ok(artifact)
    }

    /// Rechecks both admitted descriptors before allocating scratch or registering.
    #[must_use]
    pub fn is_graph_split_attention_v1(&self) -> bool {
        let kernels = self.inspection.hsaco().kernels();
        self.hsaco_id().as_bytes() == &IMAGE
            && kernels.len() == 2
            && ENGINEERING_TP_GRAPH_SPLIT_ATTENTION_ROOTS_V1
                .iter()
                .enumerate()
                .all(|(index, root)| {
                    kernels
                        .iter()
                        .find(|kernel| kernel.name() == *root)
                        .is_some_and(|kernel| metadata_matches(kernel, index))
                })
    }
}

fn metadata_matches(kernel: &InspectedKernel, index: usize) -> bool {
    let implicit = if index == 0 { 120 } else { 64 };
    let hidden = hidden_layout(implicit);
    kernel.name() == ENGINEERING_TP_GRAPH_SPLIT_ATTENTION_ROOTS_V1[index]
        && kernel.kernarg_segment_size() == implicit + 256
        && kernel.kernarg_segment_alignment() == 8
        && kernel.implicit_argument_offset() == Some(implicit)
        && kernel.implicit_argument_size() == 256
        && kernel.wavefront_size() == 64
        && kernel.max_flat_workgroup_size() == 64
        && kernel.required_workgroup_size() == Some([64, 1, 1])
        && kernel.group_segment_fixed_size() == 0
        && kernel.private_segment_fixed_size() == 0
        && kernel.sgpr_count() == if index == 0 { 78 } else { 60 }
        && kernel.vgpr_count() == if index == 0 { 38 } else { 23 }
        && kernel.agpr_count() == Some(0)
        && kernel.sgpr_spill_count() == Some(0)
        && kernel.vgpr_spill_count() == Some(0)
        && !kernel.uses_dynamic_stack()
        && kernel.explicit_arguments().len() == if index == 0 { 17 } else { 10 }
        && kernel
            .explicit_arguments()
            .iter()
            .enumerate()
            .all(|(argument_index, argument)| {
                argument_matches(argument, argument_index, index == 0)
            })
        && kernel.hidden_arguments().len() == hidden.len()
        && kernel
            .hidden_arguments()
            .iter()
            .zip(hidden)
            .all(|(argument, wanted)| {
                (argument.offset(), argument.size(), argument.value_kind()) == wanted
            })
}

fn argument_matches(argument: &ExplicitArgument, index: usize, partial: bool) -> bool {
    use fe2o3_hsaco::{
        ArgumentAddressSpace::Global,
        ExplicitValueKind::{ByValue, GlobalBuffer},
    };
    let pairs = if partial { 12 } else { 6 };
    let pointer = index < pairs && index.is_multiple_of(2);
    let offset = if index < pairs {
        index * 8
    } else {
        pairs * 8 + (index - pairs) * 4
    };
    u64::try_from(offset).ok() == Some(argument.offset())
        && argument.size() == if index < pairs { 8 } else { 4 }
        && argument.value_kind() == if pointer { GlobalBuffer } else { ByValue }
        && argument.address_space() == if pointer { Some(Global) } else { None }
        && argument.alignment().is_none()
        && argument.pointee_alignment().is_none()
        && argument.access().is_none()
        && argument.actual_access().is_none()
        && argument.value_type().is_none()
}

fn hidden_layout(base: u64) -> [(u64, u64, HiddenValueKind); 13] {
    use HiddenValueKind::{
        BlockCountX, BlockCountY, BlockCountZ, GlobalOffsetX, GlobalOffsetY, GlobalOffsetZ,
        GridDimensions, GroupSizeX, GroupSizeY, GroupSizeZ, RemainderX, RemainderY, RemainderZ,
    };
    [
        (base, 4, BlockCountX),
        (base + 4, 4, BlockCountY),
        (base + 8, 4, BlockCountZ),
        (base + 12, 2, GroupSizeX),
        (base + 14, 2, GroupSizeY),
        (base + 16, 2, GroupSizeZ),
        (base + 18, 2, RemainderX),
        (base + 20, 2, RemainderY),
        (base + 22, 2, RemainderZ),
        (base + 40, 8, GlobalOffsetX),
        (base + 48, 8, GlobalOffsetY),
        (base + 56, 8, GlobalOffsetZ),
        (base + 64, 2, GridDimensions),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_missing_artifact_rejects_before_allocation_or_registration() {
        assert!(
            EngineeringTpArtifactV1::open_graph_split_attention_v1(Path::new(
                "/ferric-missing-split-attention-artifact"
            ))
            .is_err()
        );
    }

    #[test]
    #[ignore = "CPU-only ordinary admission requires pinned actual artifact paths"]
    fn split_actual_artifact_admits_both_exact_roots_and_rejects_old_images() {
        let path = std::env::var_os("FERRIC_PREPARED_SPLIT_ATTENTION_ARTIFACT").unwrap();
        let image =
            EngineeringTpArtifactV1::open_graph_split_attention_v1(Path::new(&path)).unwrap();
        assert!(image.is_graph_split_attention_v1());
        assert_eq!(image.hsaco_id().as_bytes(), &IMAGE);
        assert_eq!(image.inspection().hsaco().kernels().len(), 2);
        for name in [
            "FERRIC_PREPARED_MAIN_ARTIFACT",
            "FERRIC_PREPARED_PEER_ARTIFACT",
            "FERRIC_PREPARED_V22_ARTIFACT",
            "FERRIC_PREPARED_V15_ARTIFACT",
        ] {
            let path = std::env::var_os(name).unwrap();
            assert!(
                EngineeringTpArtifactV1::open_graph_split_attention_v1(Path::new(&path)).is_err()
            );
        }
    }
}
