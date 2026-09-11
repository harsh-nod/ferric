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

    fn open_profile(
        root: &Path,
        expected: &[CompilerGeneratedKernelExpectationRosterEntryV1],
        source_crate: &str,
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
