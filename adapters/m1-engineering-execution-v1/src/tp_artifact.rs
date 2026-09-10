//! Strict, non-authoritative admission for the separate thirteen-kernel TP image.
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
}
