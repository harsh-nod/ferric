//! Strict, non-authoritative admission for independently closed TP kernel images.
//!
//! This does not convert an old M1 program catalog, rewrite a target label, or
//! authenticate machine-code origin. It retains the exact observed bytes for
//! the explicitly opted-in isolated engineering worker.

use super::{
    CWD, ENGINEERING_NAMESPACE_V1, EngineeringManifestV1,
    M1_ENGINEERING_AGGREGATE_ARTIFACT_FILENAME_V1, M1_ENGINEERING_AGGREGATE_MANIFEST_FILENAME_V1,
    M1EngineeringAggregateArtifactFileV1, M1EngineeringAggregateArtifactOpenErrorV1,
    MAX_HSACO_BYTES_V1, MAX_MANIFEST_BYTES_V1, Mode, OFlags, ReadBound, ResolveFlags,
    descriptor_symbol_matches, digest, io_error, observation_content_id, openat2,
    read_regular_file, require_exact_directory_roster, validate_manifest_profile,
};
use fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1;
use fe2o3_hsaco_finalize::FinalizedDescriptorInspection;
use ferric_spec::Identity;
use std::ffi::OsStr;
use std::path::Path;
use std::sync::Arc;

/// Exact target of the initial MI350 tensor-parallel engineering path.
pub const ENGINEERING_TP_TARGET_V1: &str = "gfx950:xnack-";
/// The separate compilation unit, never the protected M1 aggregate.
pub const ENGINEERING_TP_CRATE_V1: &str = "ferric_qwen3_tp_kernels_device_v1";

/// Closed export roster, including unchanged imported helper roots.
pub const ENGINEERING_TP_EXPORTS_V1: [&str; 13] = [
    "ferric_qwen3_gemm_reference_bf16_f32_bf16_v1",
    "ferric_qwen3_gemm_vector_a4_bf16_f32_bf16_v1",
    "ferric_qwen3_token_embedding_bf16_copy_v1",
    "qwen3_rmsnorm_v1",
    "ferric_qwen3_lowest_id_argmax_bf16_v1",
    "ferric_qwen3_speculative_token_assembly_v1",
    "ferric_qwen3_compact_completion_v1",
    "ferric_qwen3_tp_gemv_bf16_f32_bf16_v1",
    "ferric_qwen3_tp_gemv_partial_bf16_f32_v1",
    "ferric_qwen3_tp_swiglu_bf16_f32_v1",
    "ferric_qwen3_tp_rope_v1",
    "ferric_qwen3_tp_kv_append_v1",
    "ferric_qwen3_tp_gqa_decode_bf16_f32_v1",
];

/// Independent compilation unit for bounded multi-row, paged TP execution.
pub const ENGINEERING_TP_BATCH_CRATE_V2: &str = "ferric_qwen3_tp_batch_kernels_device_v2";
/// Closed additive roster; old single-sequence artifacts are not reinterpreted.
pub const ENGINEERING_TP_BATCH_EXPORTS_V2: [&str; 9] = [
    "qwen3_rmsnorm_v1",
    "ferric_qwen3_tp_batch_embedding_bf16_v2",
    "ferric_qwen3_tp_batch_gemm_bf16_f32_bf16_v2",
    "ferric_qwen3_tp_batch_gemm_partial_bf16_f32_v2",
    "ferric_qwen3_tp_batch_swiglu_bf16_f32_v2",
    "ferric_qwen3_tp_batch_rope_v2",
    "ferric_qwen3_tp_batch_paged_kv_append_v2",
    "ferric_qwen3_tp_batch_paged_gqa_bf16_f32_v2",
    "ferric_qwen3_tp_batch_argmax_bf16_v2",
];

/// Closed additional roots in the full v3 profile; the wave profile omits MFMA only.
pub const ENGINEERING_TP_PERFORMANCE_EXPORTS_V3: [&str; 6] = [
    "ferric_qwen3_tp_wave_gemv_bf16_v3",
    "ferric_qwen3_tp_wave_gemv_partial_f32_v3",
    "ferric_qwen3_tp_mfma_gemm_bf16_v3",
    "ferric_qwen3_tp_mfma_gemm_partial_f32_v3",
    "ferric_qwen3_tp_wave_paged_gqa_bf16_v3",
    "ferric_qwen3_tp_batch_residual_bf16_v3",
];

/// Independent two-root image for ordered peer reduction and embedding copy.
pub const ENGINEERING_TP_PEER_EXPORTS_V4: [&str; 2] = [
    "ferric_qwen3_tp_peer_ordered_residual_bf16_v4",
    "ferric_qwen3_tp_peer_copy_bf16_v4",
];

/// Exact separate image required for peer arithmetic with the 32-row profile.
pub const ENGINEERING_TP_PEER32_EXPORTS_V6: [&str; 2] = [
    "ferric_qwen3_tp_batch32_peer_ordered_residual_bf16_v6",
    "ferric_qwen3_tp_batch32_peer_copy_bf16_v6",
];

/// Closed additional TP1 scalar/MFMA FP32 head and lowest-ID FP32 argmax image.
pub const ENGINEERING_TP_FP32_HEAD_EXPORTS_V7: [&str; 3] = [
    "ferric_qwen3_tp_head_bf16_f32_v7",
    "ferric_qwen3_tp_mfma_head_f32_v7",
    "ferric_qwen3_tp_argmax_f32_v7",
];

/// Closed additional TP1 FP32 head image for genuine 32-row execution.
pub const ENGINEERING_TP_FP32_HEAD32_EXPORTS_V8: [&str; 3] = [
    "ferric_qwen3_tp_batch32_head_bf16_f32_v8",
    "ferric_qwen3_tp_batch32_mfma_head_f32_v8",
    "ferric_qwen3_tp_batch32_argmax_f32_v8",
];

/// Independent Wave64 FP32 argmax image; the v8 projection remains required.
#[cfg(feature = "tp-batch-engineering")]
pub const ENGINEERING_TP_FP32_ARGMAX32_EXPORTS_V11: [&str; 1] =
    ["ferric_qwen3_tp_batch32_wave_argmax_f32_v11"];
#[cfg(feature = "tp-batch-engineering")]
const FP32_ARGMAX32_CRATE_V11: &str = "ferric_qwen3_tp_fp32_argmax_kernels_device_v11";

/// Private image identity minted only by exact one-root v11 admission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg(feature = "tp-batch-engineering")]
pub(crate) struct Fp32ArgmaxBindingV11 {
    pub(crate) hsaco: [u8; 32],
    manifest: [u8; 32],
    handoff: [u8; 32],
}

#[cfg(all(test, feature = "tp-batch-engineering"))]
impl Fp32ArgmaxBindingV11 {
    pub(crate) const fn recording() -> Self {
        Self {
            hsaco: [113; 32],
            manifest: [114; 32],
            handoff: [115; 32],
        }
    }
}

/// Separate query-hoisted GQA image; no existing v5 export is replaced at admission.
#[cfg(feature = "tp-batch-engineering")]
pub const ENGINEERING_TP_QUERY_HOIST_EXPORTS_V14: [&str; 1] =
    ["ferric_qwen3_tp_batch32_wave_paged_gqa_query_hoist_bf16_v14"];
#[cfg(feature = "tp-batch-engineering")]
const QUERY_HOIST_CRATE_V14: &str = "ferric_qwen3_tp_wave_query_hoist_kernels_device_v14";

/// Private identity minted only by the separate one-root v14 profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg(feature = "tp-batch-engineering")]
pub(crate) struct QueryHoistBindingV14 {
    pub(crate) hsaco: [u8; 32],
    manifest: [u8; 32],
    handoff: [u8; 32],
}

#[cfg(all(test, feature = "tp-batch-engineering"))]
impl QueryHoistBindingV14 {
    pub(crate) const fn recording() -> Self {
        Self {
            hsaco: [141; 32],
            manifest: [142; 32],
            handoff: [143; 32],
        }
    }
}

/// Closed TP1 image extending physical KV storage without extending logical context.
pub const ENGINEERING_TP_LARGE_KV_EXPORTS_V9: [&str; 2] = [
    "ferric_qwen3_tp_batch32_large_kv_append_v9",
    "ferric_qwen3_tp_batch32_large_kv_paged_gqa_bf16_f32_v9",
];
#[cfg(feature = "tp-batch-engineering")]
const LARGE_KV_CRATE_V9: &str = "ferric_qwen3_tp_large_kv_kernels_device_v9";

/// Independent, closed `Draft06B` roster; no target image can substitute.
#[cfg(feature = "tp-batch-engineering")]
pub const ENGINEERING_DRAFT_BATCH32_EXPORTS_V10: [&str; 14] = [
    "ferric_qwen3_draft_batch32_rmsnorm_v10",
    "ferric_qwen3_draft_batch32_embedding_bf16_v10",
    "ferric_qwen3_draft_batch32_gemm_bf16_f32_bf16_v10",
    "ferric_qwen3_draft_batch32_mfma_gemm_bf16_v10",
    "ferric_qwen3_draft_batch32_gemm_partial_bf16_f32_v10",
    "ferric_qwen3_draft_batch32_mfma_gemm_partial_f32_v10",
    "ferric_qwen3_draft_batch32_swiglu_bf16_f32_v10",
    "ferric_qwen3_draft_batch32_rope_v10",
    "ferric_qwen3_draft_batch32_paged_kv_append_v10",
    "ferric_qwen3_draft_batch32_paged_gqa_bf16_f32_v10",
    "ferric_qwen3_draft_batch32_residual_bf16_v10",
    "ferric_qwen3_draft_batch32_head_bf16_f32_v10",
    "ferric_qwen3_draft_batch32_mfma_head_f32_v10",
    "ferric_qwen3_draft_batch32_argmax_f32_v10",
];
#[cfg(feature = "tp-batch-engineering")]
const DRAFT_BATCH32_CRATE_V10: &str = "ferric_qwen3_draft_batch32_kernels_device_v10";

/// Structural identity minted only by the exact v10 admission path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg(feature = "tp-batch-engineering")]
pub(crate) struct DraftBindingV10 {
    pub(crate) hsaco: [u8; 32],
    manifest: [u8; 32],
    handoff: [u8; 32],
}

#[cfg(all(test, feature = "tp-batch-engineering"))]
impl DraftBindingV10 {
    pub(crate) const fn recording() -> Self {
        Self {
            hsaco: [110; 32],
            manifest: [111; 32],
            handoff: [112; 32],
        }
    }
}

/// Structural image binding minted only by exact v9 artifact admission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg(feature = "tp-batch-engineering")]
pub(crate) struct LargeKvBindingV9 {
    pub(crate) hsaco: [u8; 32],
    manifest: [u8; 32],
    handoff: [u8; 32],
}

#[cfg(all(test, feature = "tp-batch-engineering"))]
impl LargeKvBindingV9 {
    pub(crate) const fn recording() -> Self {
        Self {
            hsaco: [19; 32],
            manifest: [20; 32],
            handoff: [21; 32],
        }
    }
}

/// Closed full v5 image; the wave-only form omits exactly the two MFMA roots.
pub const ENGINEERING_TP_BATCH32_EXPORTS_V5: [&str; 15] = [
    "qwen3_rmsnorm_v1",
    "ferric_qwen3_tp_batch32_embedding_bf16_v5",
    "ferric_qwen3_tp_batch32_gemm_bf16_f32_bf16_v5",
    "ferric_qwen3_tp_batch32_gemm_partial_bf16_f32_v5",
    "ferric_qwen3_tp_batch32_swiglu_bf16_f32_v5",
    "ferric_qwen3_tp_batch32_rope_v5",
    "ferric_qwen3_tp_batch32_paged_kv_append_v5",
    "ferric_qwen3_tp_batch32_paged_gqa_bf16_f32_v5",
    "ferric_qwen3_tp_batch32_argmax_bf16_v5",
    "ferric_qwen3_tp_batch32_residual_bf16_v5",
    "ferric_qwen3_tp_batch32_wave_gemv_bf16_v5",
    "ferric_qwen3_tp_batch32_wave_gemv_partial_f32_v5",
    "ferric_qwen3_tp_batch32_wave_paged_gqa_bf16_v5",
    "ferric_qwen3_tp_batch32_mfma_gemm_bf16_v5",
    "ferric_qwen3_tp_batch32_mfma_gemm_partial_f32_v5",
];

/// Exact content-addressed engineering bytes and independently inspected metadata.
pub struct EngineeringTpArtifactV1 {
    #[cfg(feature = "tp-batch-engineering")]
    source_crate: &'static str,
    bytes: Arc<[u8]>,
    inspection: FinalizedDescriptorInspection,
    manifest_id: Identity,
    hsaco_id: Identity,
    handoff_id: Identity,
}

impl EngineeringTpArtifactV1 {
    /// Reopens an exact two-file engineering observation and its source roster.
    ///
    /// `expected` must be the compiler-generated roster from the current TP
    /// source crate. The fixed thirteen export names are checked independently.
    /// This is structural source agreement, not compiler-origin authentication.
    ///
    /// # Errors
    ///
    /// Rejects path, file, schema, identity, target, descriptor or roster drift.
    pub fn open(
        root: &Path,
        expected: &[CompilerGeneratedKernelExpectationRosterEntryV1],
    ) -> Result<Self, M1EngineeringAggregateArtifactOpenErrorV1> {
        Self::open_profile(
            root,
            expected,
            ENGINEERING_TP_CRATE_V1,
            &ENGINEERING_TP_EXPORTS_V1,
        )
    }

    /// Opens only the separate nine-root multi-row and paged engineering image.
    ///
    /// # Errors
    /// Rejects any target, source roster, identity, argument, or file drift.
    #[cfg(feature = "tp-batch-engineering")]
    pub fn open_batch(
        root: &Path,
        expected: &[CompilerGeneratedKernelExpectationRosterEntryV1],
    ) -> Result<Self, M1EngineeringAggregateArtifactOpenErrorV1> {
        Self::open_profile(
            root,
            expected,
            ENGINEERING_TP_BATCH_CRATE_V2,
            &ENGINEERING_TP_BATCH_EXPORTS_V2,
        )
    }

    /// Opens one exact v3 profile: thirteen wave roots or fifteen including MFMA.
    /// # Errors
    /// Rejects any missing/additional root, target, descriptor, source or identity drift.
    #[cfg(feature = "tp-batch-engineering")]
    pub fn open_performance(
        root: &Path,
        expected: &[CompilerGeneratedKernelExpectationRosterEntryV1],
        mfma: bool,
    ) -> Result<Self, M1EngineeringAggregateArtifactOpenErrorV1> {
        let mut exports = ENGINEERING_TP_BATCH_EXPORTS_V2.to_vec();
        exports.extend(
            ENGINEERING_TP_PERFORMANCE_EXPORTS_V3
                .into_iter()
                .filter(|name| mfma || !name.contains("_mfma_")),
        );
        Self::open_profile(
            root,
            expected,
            "ferric_qwen3_tp_perf_kernels_device_v3",
            &exports,
        )
    }

    /// Opens the exact separate peer image, never a replacement for the base image.
    /// # Errors
    /// Rejects any missing/additional root, target, descriptor, source or identity drift.
    #[cfg(feature = "tp-batch-engineering")]
    pub fn open_peer(
        root: &Path,
        expected: &[CompilerGeneratedKernelExpectationRosterEntryV1],
    ) -> Result<Self, M1EngineeringAggregateArtifactOpenErrorV1> {
        Self::open_profile(
            root,
            expected,
            "ferric_qwen3_tp_peer_kernels_device_v4",
            &ENGINEERING_TP_PEER_EXPORTS_V4,
        )
    }

    /// Opens one exact independent 32-row profile; old images cannot substitute.
    /// # Errors
    /// Rejects any missing/additional root, target, descriptor, source or identity drift.
    #[cfg(feature = "tp-batch-engineering")]
    pub fn open_batch32(
        root: &Path,
        expected: &[CompilerGeneratedKernelExpectationRosterEntryV1],
        mfma: bool,
    ) -> Result<Self, M1EngineeringAggregateArtifactOpenErrorV1> {
        let exports = ENGINEERING_TP_BATCH32_EXPORTS_V5
            .into_iter()
            .filter(|name| mfma || !name.contains("_mfma_"))
            .collect::<Vec<_>>();
        Self::open_profile(
            root,
            expected,
            "ferric_qwen3_tp_batch32_kernels_device_v5",
            &exports,
        )
    }

    /// Opens only the separate 32-row peer image, never the earlier v4 image.
    /// # Errors
    /// Rejects any missing/additional root, target, descriptor, source or identity drift.
    #[cfg(feature = "tp-batch-engineering")]
    pub fn open_peer32(
        root: &Path,
        expected: &[CompilerGeneratedKernelExpectationRosterEntryV1],
    ) -> Result<Self, M1EngineeringAggregateArtifactOpenErrorV1> {
        Self::open_profile(
            root,
            expected,
            "ferric_qwen3_tp_peer32_kernels_device_v6",
            &ENGINEERING_TP_PEER32_EXPORTS_V6,
        )
    }

    /// Opens only the separate v7 head image; the base BF16 image remains required.
    /// # Errors
    /// Rejects target, exact roster, descriptor, source, file, or identity drift.
    #[cfg(feature = "tp-batch-engineering")]
    pub fn open_fp32_head(
        root: &Path,
        expected: &[CompilerGeneratedKernelExpectationRosterEntryV1],
    ) -> Result<Self, M1EngineeringAggregateArtifactOpenErrorV1> {
        Self::open_profile(
            root,
            expected,
            "ferric_qwen3_tp_fp32_head_kernels_device_v7",
            &ENGINEERING_TP_FP32_HEAD_EXPORTS_V7,
        )
    }

    /// Opens only the separate v8 head32 image; neither v7 nor the base image is reinterpreted.
    /// # Errors
    /// Rejects target, exact roster, descriptor, source, file, or identity drift.
    #[cfg(feature = "tp-batch-engineering")]
    pub fn open_fp32_head32(
        root: &Path,
        expected: &[CompilerGeneratedKernelExpectationRosterEntryV1],
    ) -> Result<Self, M1EngineeringAggregateArtifactOpenErrorV1> {
        Self::open_profile(
            root,
            expected,
            "ferric_qwen3_tp_fp32_head32_kernels_device_v8",
            &ENGINEERING_TP_FP32_HEAD32_EXPORTS_V8,
        )
    }

    /// Opens only the separate v11 argmax image against its own compiled roster.
    /// This grants no authentication authority and never replaces the v8 head image.
    /// # Errors
    /// Rejects target, exact roster, descriptor, source, file or identity drift.
    #[cfg(feature = "tp-batch-engineering")]
    pub fn open_fp32_argmax32_v11(
        root: &Path,
    ) -> Result<Self, M1EngineeringAggregateArtifactOpenErrorV1> {
        Self::open_profile(
            root,
            &ferric_qwen3_tp_fp32_argmax_kernels_device_v11::compiler_expectation_roster_v11(),
            FP32_ARGMAX32_CRATE_V11,
            &ENGINEERING_TP_FP32_ARGMAX32_EXPORTS_V11,
        )
    }

    #[cfg(feature = "tp-batch-engineering")]
    pub(crate) fn fp32_argmax_binding_v11(&self) -> Option<Fp32ArgmaxBindingV11> {
        (self.source_crate == FP32_ARGMAX32_CRATE_V11).then(|| Fp32ArgmaxBindingV11 {
            hsaco: *self.hsaco_id.as_bytes(),
            manifest: *self.manifest_id.as_bytes(),
            handoff: *self.handoff_id.as_bytes(),
        })
    }

    /// Admits only the separate query-hoisted Wave64 attention image and its exact ABI.
    /// This retains observation-only handoff provenance, not numerical or model proof.
    /// # Errors
    /// Rejects noncanonical identity, wrong roots, changed argument geometry or resource use.
    #[cfg(feature = "tp-batch-engineering")]
    pub fn open_query_hoist_v14(
        root: &Path,
    ) -> Result<Self, M1EngineeringAggregateArtifactOpenErrorV1> {
        let artifact = Self::open_profile(
            root,
            &ferric_qwen3_tp_wave_query_hoist_kernels_device_v14::compiler_expectation_roster_v14(),
            QUERY_HOIST_CRATE_V14,
            &ENGINEERING_TP_QUERY_HOIST_EXPORTS_V14,
        )?;
        if !query_hoist_metadata_matches_v14(&artifact.inspection.hsaco().kernels()[0]) {
            return Err(M1EngineeringAggregateArtifactOpenErrorV1::HsacoProfile);
        }
        Ok(artifact)
    }

    #[cfg(feature = "tp-batch-engineering")]
    pub(crate) fn query_hoist_binding_v14(&self) -> Option<QueryHoistBindingV14> {
        (self.source_crate == QUERY_HOIST_CRATE_V14).then(|| QueryHoistBindingV14 {
            hsaco: *self.hsaco_id.as_bytes(),
            manifest: *self.manifest_id.as_bytes(),
            handoff: *self.handoff_id.as_bytes(),
        })
    }

    /// Opens the separately compiled two-root TP1 large-physical-KV image.
    /// # Errors
    /// Rejects any source, target, descriptor, roster, file or identity drift.
    #[cfg(feature = "tp-batch-engineering")]
    pub fn open_large_kv32(
        root: &Path,
        expected: &[CompilerGeneratedKernelExpectationRosterEntryV1],
    ) -> Result<Self, M1EngineeringAggregateArtifactOpenErrorV1> {
        Self::open_profile(
            root,
            expected,
            LARGE_KV_CRATE_V9,
            &ENGINEERING_TP_LARGE_KV_EXPORTS_V9,
        )
    }

    #[cfg(feature = "tp-batch-engineering")]
    pub(crate) fn large_kv_binding(&self) -> Option<LargeKvBindingV9> {
        (self.source_crate == LARGE_KV_CRATE_V9).then(|| LargeKvBindingV9 {
            hsaco: *self.hsaco_id.as_bytes(),
            manifest: *self.manifest_id.as_bytes(),
            handoff: *self.handoff_id.as_bytes(),
        })
    }

    /// Opens the complete independent draft32 image against its own compiled roster.
    /// # Errors
    /// Rejects every target, descriptor, root, file or source-identity mismatch.
    #[cfg(feature = "tp-batch-engineering")]
    pub fn open_draft32(root: &Path) -> Result<Self, M1EngineeringAggregateArtifactOpenErrorV1> {
        Self::open_profile(
            root,
            &ferric_qwen3_draft_batch32_kernels_device_v10::compiler_expectation_roster_v10(),
            DRAFT_BATCH32_CRATE_V10,
            &ENGINEERING_DRAFT_BATCH32_EXPORTS_V10,
        )
    }

    #[cfg(feature = "tp-batch-engineering")]
    pub(crate) fn draft_binding(&self) -> Option<DraftBindingV10> {
        (self.source_crate == DRAFT_BATCH32_CRATE_V10).then(|| DraftBindingV10 {
            hsaco: *self.hsaco_id.as_bytes(),
            manifest: *self.manifest_id.as_bytes(),
            handoff: *self.handoff_id.as_bytes(),
        })
    }

    fn open_profile(
        root: &Path,
        expected: &[CompilerGeneratedKernelExpectationRosterEntryV1],
        source_crate: &'static str,
        exports: &[&str],
    ) -> Result<Self, M1EngineeringAggregateArtifactOpenErrorV1> {
        use M1EngineeringAggregateArtifactOpenErrorV1 as Error;
        if root
            .parent()
            .and_then(Path::file_name)
            .and_then(OsStr::to_str)
            != Some(ENGINEERING_NAMESPACE_V1)
        {
            return Err(Error::ContentDirectoryIdentity);
        }
        let directory = openat2(
            CWD,
            root,
            OFlags::RDONLY
                | OFlags::DIRECTORY
                | OFlags::NOFOLLOW
                | OFlags::NONBLOCK
                | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )
        .map_err(|error| io_error(M1EngineeringAggregateArtifactFileV1::RootDirectory, error))?;
        require_exact_directory_roster(&directory)?;
        let raw_manifest = read_regular_file(
            &directory,
            M1_ENGINEERING_AGGREGATE_MANIFEST_FILENAME_V1,
            M1EngineeringAggregateArtifactFileV1::Manifest,
            ReadBound::Maximum(MAX_MANIFEST_BYTES_V1),
        )?;
        let manifest: EngineeringManifestV1 = serde_json::from_slice(&raw_manifest)
            .map_err(|error| Error::ManifestJson(error.to_string()))?;
        let mut canonical = serde_json::to_vec(&manifest)
            .map_err(|error| Error::ManifestJson(error.to_string()))?;
        canonical.push(b'\n');
        if canonical != raw_manifest {
            return Err(Error::NonCanonicalManifest);
        }
        let facts = validate_manifest_profile(&manifest, source_crate, ENGINEERING_TP_TARGET_V1)?;
        if !exact_roster(
            manifest.hsaco.kernel_names.iter().map(String::as_str),
            exports,
        ) || !exact_roster(
            expected
                .iter()
                .map(CompilerGeneratedKernelExpectationRosterEntryV1::export_name),
            exports,
        ) {
            return Err(Error::MetadataKernelRoster);
        }
        let hsaco_len = usize::try_from(facts.hsaco.byte_len)
            .ok()
            .filter(|length| *length <= MAX_HSACO_BYTES_V1)
            .ok_or(Error::InvalidSize(
                M1EngineeringAggregateArtifactFileV1::Hsaco,
            ))?;
        let bytes = read_regular_file(
            &directory,
            M1_ENGINEERING_AGGREGATE_ARTIFACT_FILENAME_V1,
            M1EngineeringAggregateArtifactFileV1::Hsaco,
            ReadBound::Exact(hsaco_len),
        )?;
        require_exact_directory_roster(&directory)?;
        if digest(&bytes) != facts.hsaco.sha256 {
            return Err(Error::HsacoIdentity);
        }
        let content_id = observation_content_id(&raw_manifest, &bytes);
        if root.file_name().and_then(OsStr::to_str) != Some(content_id.as_str()) {
            return Err(Error::ContentDirectoryIdentity);
        }
        let inspection = fe2o3_hsaco_finalize::inspect_finalized(&bytes)
            .map_err(|error| Error::FinalizedHsaco(Box::new(error)))?;
        if inspection.hsaco().target().to_string() != ENGINEERING_TP_TARGET_V1
            || inspection.descriptor_table().device_target().to_string() != ENGINEERING_TP_TARGET_V1
            || inspection.hsaco().code_object_version().number() != 6
            || inspection.descriptor_table().code_object_version().number() != 6
        {
            return Err(Error::HsacoProfile);
        }
        if inspection.digest().as_bytes() != &facts.canonical_descriptor_sha256 {
            return Err(Error::CanonicalDescriptorDigest);
        }
        if inspection.hsaco().kernels().len() != manifest.hsaco.kernel_names.len()
            || inspection
                .hsaco()
                .kernels()
                .iter()
                .zip(&manifest.hsaco.kernel_names)
                .any(|(kernel, name)| kernel.name() != name)
        {
            return Err(Error::MetadataKernelRoster);
        }
        let descriptors = inspection.descriptor_table().kernels();
        if descriptors.len() != expected.len()
            || descriptors.iter().enumerate().any(|(index, left)| {
                descriptors
                    .iter()
                    .skip(index + 1)
                    .any(|right| left.logical_name() == right.logical_name())
            })
            || expected.iter().enumerate().any(|(index, left)| {
                expected
                    .iter()
                    .skip(index + 1)
                    .any(|right| left.logical_name() == right.logical_name())
            })
            || descriptors.iter().any(|descriptor| {
                !expected.iter().any(|entry| {
                    descriptor.logical_name().as_str() == entry.logical_name()
                        && descriptor.entry_name().as_str() == entry.export_name()
                        && descriptor_symbol_matches(
                            descriptor.descriptor_symbol().as_str(),
                            entry.export_name(),
                        )
                })
            })
        {
            return Err(Error::CurrentFerricDescriptorRoster);
        }
        Ok(Self {
            #[cfg(feature = "tp-batch-engineering")]
            source_crate,
            bytes: bytes.into(),
            inspection,
            manifest_id: Identity::new(digest(&raw_manifest)),
            hsaco_id: Identity::new(facts.hsaco.sha256),
            handoff_id: Identity::new(facts.compiler_handoff.sha256),
        })
    }

    /// Borrows the exact retained HSACO image for isolated worker loading.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Borrows independently inspected argument layouts and target metadata.
    #[must_use]
    pub const fn inspection(&self) -> &FinalizedDescriptorInspection {
        &self.inspection
    }

    /// Returns the exact canonical manifest identity.
    #[must_use]
    pub const fn manifest_id(&self) -> Identity {
        self.manifest_id
    }

    /// Returns the exact HSACO byte identity.
    #[must_use]
    pub const fn hsaco_id(&self) -> Identity {
        self.hsaco_id
    }

    /// Returns the handoff identity observed by the compiler, not an attestation.
    #[must_use]
    pub const fn handoff_id(&self) -> Identity {
        self.handoff_id
    }
}

#[cfg(feature = "tp-batch-engineering")]
fn query_hoist_metadata_matches_v14(kernel: &fe2o3_hsaco::InspectedKernel) -> bool {
    use fe2o3_hsaco::HiddenValueKind;
    let hidden = [
        (120, 4, HiddenValueKind::BlockCountX),
        (124, 4, HiddenValueKind::BlockCountY),
        (128, 4, HiddenValueKind::BlockCountZ),
        (132, 2, HiddenValueKind::GroupSizeX),
        (134, 2, HiddenValueKind::GroupSizeY),
        (136, 2, HiddenValueKind::GroupSizeZ),
        (138, 2, HiddenValueKind::RemainderX),
        (140, 2, HiddenValueKind::RemainderY),
        (142, 2, HiddenValueKind::RemainderZ),
        (160, 8, HiddenValueKind::GlobalOffsetX),
        (168, 8, HiddenValueKind::GlobalOffsetY),
        (176, 8, HiddenValueKind::GlobalOffsetZ),
        (184, 2, HiddenValueKind::GridDimensions),
    ];
    kernel.name() == ENGINEERING_TP_QUERY_HOIST_EXPORTS_V14[0]
        && kernel.kernarg_segment_size() == 376
        && kernel.kernarg_segment_alignment() == 8
        && kernel.implicit_argument_offset() == Some(120)
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
        && kernel.explicit_arguments().len() == 17
        && kernel
            .explicit_arguments()
            .iter()
            .enumerate()
            .all(|(index, arg)| QueryHoistArgumentV14::from(arg).matches(index))
        && kernel.hidden_arguments().len() == hidden.len()
        && kernel
            .hidden_arguments()
            .iter()
            .zip(hidden)
            .all(|(arg, expected)| (arg.offset(), arg.size(), arg.value_kind()) == expected)
}

/// Parsed fields used by this new exact ABI profile and its synthetic negative tests.
#[derive(Clone, Copy)]
#[cfg(feature = "tp-batch-engineering")]
struct QueryHoistArgumentV14 {
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

#[cfg(feature = "tp-batch-engineering")]
impl From<&fe2o3_hsaco::ExplicitArgument> for QueryHoistArgumentV14 {
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

#[cfg(feature = "tp-batch-engineering")]
impl QueryHoistArgumentV14 {
    fn matches(self, index: usize) -> bool {
        use fe2o3_hsaco::{
            ArgumentAddressSpace::Global,
            ExplicitValueKind::{ByValue, GlobalBuffer},
        };
        if index >= 17 {
            return false;
        }
        let pointer = index < 12 && index.is_multiple_of(2);
        let (offset, size) = if index < 12 {
            (index * 8, 8)
        } else {
            (96 + (index - 12) * 4, 4)
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
fn exact_exports<'a>(actual: impl Iterator<Item = &'a str>) -> bool {
    exact_roster(actual, &ENGINEERING_TP_EXPORTS_V1)
}

fn exact_roster<'a>(actual: impl Iterator<Item = &'a str>, expected: &[&str]) -> bool {
    let mut actual = actual.collect::<Vec<_>>();
    let mut expected = expected.to_vec();
    actual.sort_unstable();
    expected.sort_unstable();
    actual == expected
}

#[cfg(test)]
mod tests {
    use super::{ENGINEERING_TP_EXPORTS_V1, exact_exports};

    #[test]
    #[cfg(feature = "tp-batch-engineering")]
    fn query_hoist_v14_roster_is_exact_and_disjoint_from_all_resident_images() {
        use super::{ENGINEERING_TP_QUERY_HOIST_EXPORTS_V14 as ROOTS, exact_roster};
        assert!(exact_roster(
            ferric_qwen3_tp_wave_query_hoist_kernels_device_v14::compiler_expectation_roster_v14()
                .iter()
                .map(fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1::export_name),
            &ROOTS,
        ));
        assert_eq!(
            ROOTS,
            ferric_qwen3_tp_wave_query_hoist_kernels_device_v14::ROOTS_V14
        );
        assert!(!exact_roster(std::iter::empty(), &ROOTS));
        assert!(!exact_roster([ROOTS[0], ROOTS[0]].into_iter(), &ROOTS));
        for root in super::ENGINEERING_TP_BATCH32_EXPORTS_V5
            .into_iter()
            .chain(super::ENGINEERING_TP_FP32_HEAD32_EXPORTS_V8)
            .chain(super::ENGINEERING_TP_LARGE_KV_EXPORTS_V9)
            .chain(super::ENGINEERING_DRAFT_BATCH32_EXPORTS_V10)
            .chain(super::ENGINEERING_TP_FP32_ARGMAX32_EXPORTS_V11)
        {
            assert!(!ROOTS.contains(&root));
            assert!(!exact_roster([root].into_iter(), &ROOTS));
        }
    }

    #[test]
    #[cfg(feature = "tp-batch-engineering")]
    fn query_hoist_v14_argument_profile_rejects_geometry_and_qualifier_drift() {
        use fe2o3_hsaco::{
            ArgumentAccess, ArgumentAddressSpace, ExplicitValueKind, ExplicitValueType,
        };
        for index in 0_usize..17 {
            let pointer = index < 12 && index.is_multiple_of(2);
            let valid = super::QueryHoistArgumentV14 {
                offset: if index < 12 {
                    index * 8
                } else {
                    96 + (index - 12) * 4
                } as u64,
                size: if index < 12 { 8 } else { 4 },
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
            assert!(!valid.matches(17));
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
    #[ignore = "requires the separately emitted canonical v14 image; host admission only"]
    #[cfg(feature = "tp-batch-engineering")]
    fn query_hoist_v14_image_admission_binds_exact_abi_and_its_own_identity() {
        let root = std::path::PathBuf::from(
            std::env::var_os("FERRIC_TEST_QUERY_HOIST_V14_ARTIFACT").expect("explicit v14 image"),
        );
        let artifact = super::EngineeringTpArtifactV1::open_query_hoist_v14(&root).unwrap();
        let binding = artifact.query_hoist_binding_v14().unwrap();
        assert_eq!(binding.hsaco, *artifact.hsaco_id().as_bytes());
        assert_eq!(binding.manifest, *artifact.manifest_id().as_bytes());
        assert_eq!(binding.handoff, *artifact.handoff_id().as_bytes());
        assert!(super::query_hoist_metadata_matches_v14(
            &artifact.inspection().hsaco().kernels()[0]
        ));
        assert!(artifact.fp32_argmax_binding_v11().is_none());
        assert!(artifact.draft_binding().is_none());
        assert!(artifact.large_kv_binding().is_none());
        assert!(super::EngineeringTpArtifactV1::open_fp32_argmax32_v11(&root).is_err());
        assert!(
            super::EngineeringTpArtifactV1::open_batch32(
                &root,
                &ferric_qwen3_tp_batch32_kernels_device_v5::compiler_expectation_roster_v5(),
                true,
            )
            .is_err()
        );
    }

    #[test]
    #[cfg(feature = "tp-batch-engineering")]
    fn argmax_v11_roster_is_single_root_and_disjoint_from_unchanged_head() {
        use super::{ENGINEERING_TP_FP32_ARGMAX32_EXPORTS_V11 as ROOTS, exact_roster};
        assert!(exact_roster(
            ferric_qwen3_tp_fp32_argmax_kernels_device_v11::compiler_expectation_roster_v11()
                .iter()
                .map(fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1::export_name),
            &ROOTS
        ));
        assert!(!exact_roster([ROOTS[0], ROOTS[0]].into_iter(), &ROOTS));
        for root in super::ENGINEERING_TP_BATCH32_EXPORTS_V5
            .into_iter()
            .chain(super::ENGINEERING_TP_FP32_HEAD32_EXPORTS_V8)
            .chain(super::ENGINEERING_DRAFT_BATCH32_EXPORTS_V10)
        {
            assert!(!ROOTS.contains(&root));
        }
    }

    #[test]
    #[ignore = "requires a separately emitted canonical v11 image; host admission only"]
    #[cfg(feature = "tp-batch-engineering")]
    fn argmax_v11_image_admission_binds_its_own_exact_roster() {
        let root = std::path::PathBuf::from(
            std::env::var_os("FERRIC_TEST_ARGMAX_V11_ARTIFACT").expect("explicit v11 image"),
        );
        let artifact = super::EngineeringTpArtifactV1::open_fp32_argmax32_v11(&root).unwrap();
        let binding = artifact.fp32_argmax_binding_v11().unwrap();
        assert_eq!(binding.hsaco, *artifact.hsaco_id().as_bytes());
        assert!(artifact.draft_binding().is_none());
        assert!(artifact.large_kv_binding().is_none());
        assert!(
            super::EngineeringTpArtifactV1::open_fp32_head32(
                &root,
                &ferric_qwen3_tp_fp32_head32_kernels_device_v8::compiler_expectation_roster_v8()
            )
            .is_err()
        );
        assert!(
            super::EngineeringTpArtifactV1::open_batch32(
                &root,
                &ferric_qwen3_tp_batch32_kernels_device_v5::compiler_expectation_roster_v5(),
                true
            )
            .is_err()
        );
    }

    #[test]
    #[cfg(feature = "tp-batch-engineering")]
    fn draft32_roster_is_exact_closed14_and_disjoint_from_target_images() {
        use super::{ENGINEERING_DRAFT_BATCH32_EXPORTS_V10 as DRAFT, exact_roster};
        assert!(exact_roster(
            ferric_qwen3_draft_batch32_kernels_device_v10::compiler_expectation_roster_v10()
                .iter()
                .map(fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1::export_name),
            &DRAFT,
        ));
        assert!(exact_roster(
            DRAFT.into_iter(),
            &ferric_qwen3_draft_batch32_kernels_device_v10::contract::ROOTS_V10
        ));
        assert!(!exact_roster(DRAFT.into_iter().take(13), &DRAFT));
        assert!(!exact_roster([DRAFT[0]; 14].into_iter(), &DRAFT));
        for root in super::ENGINEERING_TP_BATCH32_EXPORTS_V5
            .into_iter()
            .chain(super::ENGINEERING_TP_FP32_HEAD32_EXPORTS_V8)
            .chain(super::ENGINEERING_TP_LARGE_KV_EXPORTS_V9)
        {
            assert!(!DRAFT.contains(&root));
        }
    }

    #[test]
    #[ignore = "requires an independently emitted canonical v10 image; host admission only"]
    #[cfg(feature = "tp-batch-engineering")]
    fn draft32_image_bound_admission_uses_its_compiler_roster() {
        let root = std::path::PathBuf::from(
            std::env::var_os("FERRIC_TEST_DRAFT32_ARTIFACT").expect("explicit v10 image"),
        );
        let artifact = super::EngineeringTpArtifactV1::open_draft32(&root).unwrap();
        let binding = artifact.draft_binding().unwrap();
        assert_eq!(binding.hsaco, *artifact.hsaco_id().as_bytes());
        assert_eq!(binding.manifest, *artifact.manifest_id().as_bytes());
        assert_eq!(binding.handoff, *artifact.handoff_id().as_bytes());
        assert!(artifact.large_kv_binding().is_none());
        assert!(
            super::EngineeringTpArtifactV1::open_batch32(
                &root,
                &ferric_qwen3_tp_batch32_kernels_device_v5::compiler_expectation_roster_v5(),
                true
            )
            .is_err()
        );
    }

    #[test]
    fn fp32_head_roster_is_closed_and_disjoint_from_every_base_profile() {
        use super::{ENGINEERING_TP_FP32_HEAD_EXPORTS_V7 as HEAD, exact_roster};
        assert!(exact_roster(HEAD.into_iter().rev(), &HEAD));
        assert!(!exact_roster(HEAD.into_iter().take(2), &HEAD));
        assert!(!exact_roster([HEAD[0]; 3].into_iter(), &HEAD));
        for base in super::ENGINEERING_TP_BATCH_EXPORTS_V2
            .into_iter()
            .chain(super::ENGINEERING_TP_PERFORMANCE_EXPORTS_V3)
            .chain(super::ENGINEERING_TP_BATCH32_EXPORTS_V5)
        {
            assert!(!HEAD.contains(&base));
        }
        #[cfg(feature = "tp-batch-engineering")]
        assert!(exact_roster(
            ferric_qwen3_tp_fp32_head_kernels_device_v7::compiler_expectation_roster_v7()
                .iter()
                .map(fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1::export_name),
            &HEAD,
        ));
    }

    #[test]
    fn fp32_head32_roster_is_closed_and_cannot_substitute_for_v7() {
        use super::{ENGINEERING_TP_FP32_HEAD32_EXPORTS_V8 as HEAD, exact_roster};
        assert!(exact_roster(HEAD.into_iter().rev(), &HEAD));
        assert!(!exact_roster(HEAD.into_iter().take(2), &HEAD));
        assert!(!exact_roster([HEAD[0]; 3].into_iter(), &HEAD));
        for base in super::ENGINEERING_TP_BATCH_EXPORTS_V2
            .into_iter()
            .chain(super::ENGINEERING_TP_PERFORMANCE_EXPORTS_V3)
            .chain(super::ENGINEERING_TP_BATCH32_EXPORTS_V5)
            .chain(super::ENGINEERING_TP_FP32_HEAD_EXPORTS_V7)
        {
            assert!(!HEAD.contains(&base));
        }
        #[cfg(feature = "tp-batch-engineering")]
        assert!(exact_roster(
            ferric_qwen3_tp_fp32_head32_kernels_device_v8::compiler_expectation_roster_v8()
                .iter()
                .map(fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1::export_name),
            &HEAD,
        ));
    }

    #[test]
    fn exact_tp_roster_accepts_permutation_but_not_duplicates_or_substitution() {
        assert!(exact_exports(ENGINEERING_TP_EXPORTS_V1.into_iter()));
        assert!(exact_exports(ENGINEERING_TP_EXPORTS_V1.into_iter().rev()));
        let mut names = ENGINEERING_TP_EXPORTS_V1;
        names[0] = names[1];
        assert!(!exact_exports(names.into_iter()));
        assert!(!exact_exports(
            ENGINEERING_TP_EXPORTS_V1[..12].iter().copied()
        ));
        names[0] = "qwen3_rope_v1";
        assert!(!exact_exports(names.into_iter()));
    }

    #[test]
    fn batch_roster_is_closed_and_distinct_from_single_sequence() {
        use super::{ENGINEERING_TP_BATCH_EXPORTS_V2, exact_roster};
        assert!(exact_roster(
            ENGINEERING_TP_BATCH_EXPORTS_V2.into_iter().rev(),
            &ENGINEERING_TP_BATCH_EXPORTS_V2
        ));
        assert!(!exact_roster(
            ENGINEERING_TP_EXPORTS_V1.into_iter(),
            &ENGINEERING_TP_BATCH_EXPORTS_V2
        ));
        assert!(!exact_exports(ENGINEERING_TP_BATCH_EXPORTS_V2.into_iter()));
        let mut duplicate = ENGINEERING_TP_BATCH_EXPORTS_V2;
        duplicate[1] = duplicate[0];
        assert!(!exact_roster(
            duplicate.into_iter(),
            &ENGINEERING_TP_BATCH_EXPORTS_V2
        ));
    }

    #[test]
    fn peer_roster_cannot_substitute_or_extend_the_base_image() {
        use super::{
            ENGINEERING_TP_BATCH_EXPORTS_V2, ENGINEERING_TP_PEER_EXPORTS_V4, exact_roster,
        };
        assert!(exact_roster(
            ENGINEERING_TP_PEER_EXPORTS_V4.into_iter().rev(),
            &ENGINEERING_TP_PEER_EXPORTS_V4
        ));
        assert!(!exact_roster(
            ENGINEERING_TP_BATCH_EXPORTS_V2.into_iter(),
            &ENGINEERING_TP_PEER_EXPORTS_V4
        ));
        assert!(!exact_roster(
            ENGINEERING_TP_PEER_EXPORTS_V4.into_iter(),
            &ENGINEERING_TP_BATCH_EXPORTS_V2
        ));
        assert!(!exact_roster(
            ENGINEERING_TP_PEER_EXPORTS_V4.into_iter().take(1),
            &ENGINEERING_TP_PEER_EXPORTS_V4
        ));
        assert!(!exact_roster(
            [ENGINEERING_TP_PEER_EXPORTS_V4[0]; 2].into_iter(),
            &ENGINEERING_TP_PEER_EXPORTS_V4
        ));
    }
}
