//! Offline construction and structural inspection of the seven M1 HSACOs.
//!
//! The returned owner keeps the live, non-clone inspected Worker evidence. The
//! persisted objects and manifest cannot recreate that custody after process
//! exit, do not independently approve deployment content, and grant no HSA
//! loading or execution authority.

use std::error::Error;
use std::ffi::OsString;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use fe2o3_hsaco_finalize::{ContentIdentityV1, InertProtectedFirstBuildWorkerV3EvidenceV1};
use ferric_qwen_kernels::{gemm, logits, paged_decode, prefill, rmsnorm, rope_kv, swiglu};
use rustix::fs::{renameat_with, RenameFlags, CWD};
use sha2::{Digest, Sha256};

use super::kernel_artifact_manifest::{
    speculative_assembly_catalog_identity, M1KernelArtifactEntryV1, M1KernelArtifactFamilyV1,
    M1KernelArtifactManifestErrorV1, M1KernelArtifactManifestV1, M1KernelProfileCatalogV1,
    M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1,
};
use super::kernel_artifact_policy::{
    worker_execution_policy_matches_m1_kernel_policy_v1,
    worker_measurement_matches_m1_kernel_policy_v1,
};

/// Stable filename of the canonical manifest inside a published output directory.
pub const M1_KERNEL_ARTIFACT_MANIFEST_FILENAME_V1: &str = "m1-kernel-artifacts.manifest.bin";

const LABEL_DOMAIN: &[u8] = b"ferric.m1.kernel-artifact-builder.inert-label.v1";

/// Linear live custody of all seven structurally inspected M1 kernel artifacts.
///
/// This owner deliberately is not `Clone`. Its persisted manifest remains
/// inert and cannot be converted back into this live Worker-evidence custody.
///
/// ```compile_fail
/// use ferric_build::BuiltAndInspectedM1KernelArtifactsV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<BuiltAndInspectedM1KernelArtifactsV1>();
/// ```
///
/// ```compile_fail
/// use ferric_build::BuiltAndInspectedM1KernelArtifactsV1;
/// fn bypass(owner: BuiltAndInspectedM1KernelArtifactsV1) {
///     let _ = owner.gemm;
/// }
/// ```
///
/// ```compile_fail
/// use ferric_build::{
///     decode_m1_kernel_artifact_manifest_v1, BuiltAndInspectedM1KernelArtifactsV1,
/// };
/// fn reopen(bytes: &[u8]) -> BuiltAndInspectedM1KernelArtifactsV1 {
///     decode_m1_kernel_artifact_manifest_v1(bytes).unwrap()
/// }
/// ```
pub struct BuiltAndInspectedM1KernelArtifactsV1 {
    gemm: gemm::InspectedQwen3GemmKernelV1,
    rmsnorm: rmsnorm::InspectedQwen3RmsNormKernelV1,
    rope_kv: rope_kv::InspectedQwen3RopeKvKernelV1,
    prefill: prefill::InspectedQwen3PrefillKernelV1,
    paged_decode: paged_decode::InspectedQwen3PagedDecodeKernelV1,
    swiglu: swiglu::InspectedQwen3SwiGluKernelV1,
    logits: logits::InspectedQwen3LogitsKernelV1,
    manifest: M1KernelArtifactManifestV1,
    output_directory: PathBuf,
    publication: M1KernelArtifactPublicationStatusV1,
}

/// Move-only protected Worker V3 evidence for every M1 kernel family.
///
/// Each owner must originate from fe2o3's compiler-produced V3 transaction. The
/// builder checks the exact Worker measurement, link options, and execution
/// limits, and the family adapters independently compare each nested compiler
/// handoff with current Ferric source before any HSACO is inspected or
/// published.
pub struct M1KernelWorkerV3EvidenceSetV1 {
    gemm: InertProtectedFirstBuildWorkerV3EvidenceV1,
    rmsnorm: InertProtectedFirstBuildWorkerV3EvidenceV1,
    rope_kv: InertProtectedFirstBuildWorkerV3EvidenceV1,
    prefill: InertProtectedFirstBuildWorkerV3EvidenceV1,
    paged_decode: InertProtectedFirstBuildWorkerV3EvidenceV1,
    swiglu: InertProtectedFirstBuildWorkerV3EvidenceV1,
    logits: InertProtectedFirstBuildWorkerV3EvidenceV1,
}

impl M1KernelWorkerV3EvidenceSetV1 {
    /// Retains one exact protected V3 owner for each named K1-K7 family.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        gemm: InertProtectedFirstBuildWorkerV3EvidenceV1,
        rmsnorm: InertProtectedFirstBuildWorkerV3EvidenceV1,
        rope_kv: InertProtectedFirstBuildWorkerV3EvidenceV1,
        prefill: InertProtectedFirstBuildWorkerV3EvidenceV1,
        paged_decode: InertProtectedFirstBuildWorkerV3EvidenceV1,
        swiglu: InertProtectedFirstBuildWorkerV3EvidenceV1,
        logits: InertProtectedFirstBuildWorkerV3EvidenceV1,
    ) -> Self {
        Self {
            gemm,
            rmsnorm,
            rope_kv,
            prefill,
            paged_decode,
            swiglu,
            logits,
        }
    }
}

impl fmt::Debug for BuiltAndInspectedM1KernelArtifactsV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BuiltAndInspectedM1KernelArtifactsV1")
            .field("manifest", &self.manifest.identity())
            .field("output_directory", &self.output_directory)
            .field("publication", &self.publication)
            .finish_non_exhaustive()
    }
}

impl BuiltAndInspectedM1KernelArtifactsV1 {
    /// K1 inspected artifact owner.
    #[must_use]
    pub const fn gemm(&self) -> &gemm::InspectedQwen3GemmKernelV1 {
        &self.gemm
    }

    /// K2 inspected artifact owner.
    #[must_use]
    pub const fn rmsnorm(&self) -> &rmsnorm::InspectedQwen3RmsNormKernelV1 {
        &self.rmsnorm
    }

    /// K3 inspected artifact owner.
    #[must_use]
    pub const fn rope_kv(&self) -> &rope_kv::InspectedQwen3RopeKvKernelV1 {
        &self.rope_kv
    }

    /// K4 inspected artifact owner.
    #[must_use]
    pub const fn prefill(&self) -> &prefill::InspectedQwen3PrefillKernelV1 {
        &self.prefill
    }

    /// K5 inspected artifact owner.
    #[must_use]
    pub const fn paged_decode(&self) -> &paged_decode::InspectedQwen3PagedDecodeKernelV1 {
        &self.paged_decode
    }

    /// K6 inspected artifact owner.
    #[must_use]
    pub const fn swiglu(&self) -> &swiglu::InspectedQwen3SwiGluKernelV1 {
        &self.swiglu
    }

    /// K7 inspected artifact owner.
    #[must_use]
    pub const fn logits(&self) -> &logits::InspectedQwen3LogitsKernelV1 {
        &self.logits
    }

    /// Strict canonical inert manifest written beside the seven objects.
    #[must_use]
    pub const fn manifest(&self) -> &M1KernelArtifactManifestV1 {
        &self.manifest
    }

    /// Final atomically published output directory.
    #[must_use]
    pub fn output_directory(&self) -> &Path {
        &self.output_directory
    }

    /// Terminal filesystem publication state after the no-clobber rename.
    #[must_use]
    pub const fn publication_status(&self) -> &M1KernelArtifactPublicationStatusV1 {
        &self.publication
    }

    /// Persistence does not preserve the private Worker-evidence owner graph.
    #[must_use]
    pub const fn has_durable_reopen_authority(&self) -> bool {
        false
    }

    /// The self-addressed manifest is not an independent deployment approval.
    #[must_use]
    pub const fn has_independent_deployment_pin(&self) -> bool {
        false
    }

    /// Offline compilation and inspection make no HSA execution claim.
    #[must_use]
    pub const fn proves_hardware_execution(&self) -> bool {
        false
    }
}

/// Terminal state after the final artifact directory becomes visible.
#[derive(Debug)]
pub enum M1KernelArtifactPublicationStatusV1 {
    /// The no-clobber rename and parent-directory durability sync both succeeded.
    ParentDirectorySynced,
    /// The complete final directory is visible, but syncing its parent failed.
    ///
    /// Publication cannot be rolled back without risking removal of a path
    /// already observed by another process, so this is retained as successful
    /// live-owner custody with an explicit durability warning.
    PublishedButParentDirectorySyncFailed {
        /// Parent directory whose sync failed after publication.
        parent_directory: PathBuf,
        /// Exact terminal sync failure.
        source: io::Error,
    },
}

impl M1KernelArtifactPublicationStatusV1 {
    /// Whether the final rename was also persisted by syncing its parent.
    #[must_use]
    pub const fn parent_directory_synced(&self) -> bool {
        matches!(self, Self::ParentDirectorySynced)
    }
}

/// Stable build stage attached to a family-local failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1KernelArtifactBuildStageV1 {
    /// Closed source, catalog, and compiler handoff construction.
    Prepare,
    /// Binding one compiler-produced Worker V3 owner to current Ferric source.
    BindWorkerV3,
    /// HSACO, AMDHSA ABI, resource, and loader inspection.
    Inspect,
    /// Content-addressed object or manifest publication.
    Publish,
}

/// Failure while building and atomically publishing the complete K1-K7 set.
#[derive(Debug)]
pub enum M1KernelArtifactBuildErrorV1 {
    /// The caller supplied an output path with no final component.
    InvalidOutputPath,
    /// The final output already exists and is left unchanged.
    OutputAlreadyExists(PathBuf),
    /// One protected V3 owner carries a Worker measurement outside M1 policy.
    WorkerMeasurementPolicy(M1KernelArtifactFamilyV1),
    /// One protected V3 owner carries link options or execution limits outside M1 policy.
    WorkerExecutionPolicy(M1KernelArtifactFamilyV1),
    /// One family failed in a named closed stage.
    Family {
        /// Exact family whose build failed.
        family: M1KernelArtifactFamilyV1,
        /// Exact stage that rejected the build.
        stage: M1KernelArtifactBuildStageV1,
        /// Underlying typed error retained behind a common boundary.
        source: Box<dyn Error + Send + Sync>,
    },
    /// Canonical manifest construction failed.
    Manifest(M1KernelArtifactManifestErrorV1),
    /// A retained artifact no longer matches the identity being published.
    ArtifactIdentity(M1KernelArtifactFamilyV1),
    /// Two inspected families unexpectedly produced one identical object.
    DuplicateArtifact,
    /// Filesystem staging or publication failed.
    Io {
        /// Operation that failed.
        operation: &'static str,
        /// Path on which the operation failed.
        path: PathBuf,
        /// Exact I/O error.
        source: io::Error,
    },
}

impl fmt::Display for M1KernelArtifactBuildErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "M1 kernel artifact build failed: {self:?}")
    }
}

impl Error for M1KernelArtifactBuildErrorV1 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Family { source, .. } => Some(source.as_ref()),
            Self::Manifest(source) => Some(source),
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<M1KernelArtifactManifestErrorV1> for M1KernelArtifactBuildErrorV1 {
    fn from(source: M1KernelArtifactManifestErrorV1) -> Self {
        Self::Manifest(source)
    }
}

/// Recomputes the seven canonical compiler-input fact sets from current Ferric
/// source without invoking a Worker or publishing any artifact.
///
/// Persisted artifact admission uses the exact module, handoff, symbol, and
/// profile identities as Ferric-owned pins for its ABI join. A manifest-carried
/// digest is inert until it matches the corresponding value returned here.
/// These facts do not authenticate compiler provenance or Worker execution.
///
/// # Errors
///
/// Returns a family-local `Prepare` failure if any canonical Qwen source,
/// profile catalog, symbol manifest, or compiler handoff cannot be rebuilt.
pub fn current_m1_kernel_source_facts_v1() -> Result<
    [M1CurrentKernelSourceFactsV1; M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1],
    M1KernelArtifactBuildErrorV1,
> {
    let gemm_family = M1KernelArtifactFamilyV1::Gemm;
    let gemm_labels = inert_labels(gemm_family);
    let gemm = gemm::prepare_qwen3_gemm_kernel_v1(gemm::Qwen3GemmSourceBindingsV1::new(
        gemm_labels[0],
        gemm_labels[1],
        gemm_labels[2],
        gemm_labels[3],
    ))
    .map_err(|source| family_error(gemm_family, M1KernelArtifactBuildStageV1::Prepare, source))?;

    let rmsnorm_family = M1KernelArtifactFamilyV1::RmsNorm;
    let rmsnorm_labels = inert_labels(rmsnorm_family);
    let rmsnorm =
        rmsnorm::prepare_qwen3_rmsnorm_kernel_v1(rmsnorm::Qwen3RmsNormSourceBindingsV1::new(
            rmsnorm_labels[0],
            rmsnorm_labels[1],
            rmsnorm_labels[2],
            rmsnorm_labels[3],
        ))
        .map_err(|source| {
            family_error(
                rmsnorm_family,
                M1KernelArtifactBuildStageV1::Prepare,
                source,
            )
        })?;

    let rope_kv_family = M1KernelArtifactFamilyV1::RopeKv;
    let rope_kv_labels = inert_labels(rope_kv_family);
    let rope_kv =
        rope_kv::prepare_qwen3_rope_kv_kernel_v1(rope_kv::Qwen3RopeKvSourceBindingsV1::new(
            rope_kv_labels[0],
            rope_kv_labels[1],
            rope_kv_labels[2],
            rope_kv_labels[3],
        ))
        .map_err(|source| {
            family_error(
                rope_kv_family,
                M1KernelArtifactBuildStageV1::Prepare,
                source,
            )
        })?;

    let prefill_family = M1KernelArtifactFamilyV1::Prefill;
    let prefill_labels = inert_labels(prefill_family);
    let prefill =
        prefill::prepare_qwen3_prefill_kernel_v1(prefill::Qwen3PrefillSourceBindingsV1::new(
            prefill_labels[0],
            prefill_labels[1],
            prefill_labels[2],
            prefill_labels[3],
        ))
        .map_err(|source| {
            family_error(
                prefill_family,
                M1KernelArtifactBuildStageV1::Prepare,
                source,
            )
        })?;

    let paged_decode_family = M1KernelArtifactFamilyV1::PagedDecode;
    let paged_decode_labels = inert_labels(paged_decode_family);
    let paged_decode = paged_decode::prepare_qwen3_paged_decode_kernel_v1(
        paged_decode::Qwen3PagedDecodeSourceBindingsV1::new(
            paged_decode_labels[0],
            paged_decode_labels[1],
            paged_decode_labels[2],
            paged_decode_labels[3],
        ),
    )
    .map_err(|source| {
        family_error(
            paged_decode_family,
            M1KernelArtifactBuildStageV1::Prepare,
            source,
        )
    })?;

    let swiglu_family = M1KernelArtifactFamilyV1::SwiGlu;
    let swiglu_labels = inert_labels(swiglu_family);
    let swiglu = swiglu::prepare_qwen3_swiglu_kernel_v1(swiglu::Qwen3SwiGluSourceBindingsV1::new(
        swiglu_labels[0],
        swiglu_labels[1],
        swiglu_labels[2],
        swiglu_labels[3],
    ))
    .map_err(|source| family_error(swiglu_family, M1KernelArtifactBuildStageV1::Prepare, source))?;

    let logits_family = M1KernelArtifactFamilyV1::Logits;
    let logits_labels = inert_labels(logits_family);
    let logits = logits::prepare_qwen3_logits_kernel_v1(logits::Qwen3LogitsSourceBindingsV1::new(
        logits_labels[0],
        logits_labels[1],
        logits_labels[2],
        logits_labels[3],
    ))
    .map_err(|source| family_error(logits_family, M1KernelArtifactBuildStageV1::Prepare, source))?;

    Ok([
        gemm_source_facts(&gemm),
        rmsnorm_source_facts(&rmsnorm),
        rope_kv_source_facts(&rope_kv),
        prefill_source_facts(&prefill),
        paged_decode_source_facts(&paged_decode),
        swiglu_source_facts(&swiglu),
        logits_source_facts(&logits),
    ])
}

/// Structurally inspects and atomically publishes one compiler-produced HSACO
/// for each K1-K7 family.
///
/// `evidence` must contain seven move-only owners produced by fe2o3's protected
/// Worker V3 transaction. `output_directory` must not exist. An atomic
/// no-replace rename happens only after all seven objects and the strict
/// canonical manifest have been synced. If syncing the parent directory then
/// fails, the function returns the live owner successfully with
/// [`M1KernelArtifactPublicationStatusV1::PublishedButParentDirectorySyncFailed`]
/// because the final directory is already visible and cannot be rolled back
/// transactionally.
///
/// # Errors
///
/// Returns [`M1KernelArtifactBuildErrorV1`] if any V3 owner does not carry the
/// exact M1 Worker measurement, link options, and execution limits or bind to
/// current Ferric source, a prepare/inspect stage fails closed, an artifact
/// identity drifts, or filesystem staging fails before publication.
/// A no-replace collision also returns an error while preserving both the
/// existing destination and automatic staging cleanup. No error is returned
/// after the final directory has become visible.
pub fn build_and_publish_m1_kernel_artifacts_v1(
    evidence: M1KernelWorkerV3EvidenceSetV1,
    output_directory: impl AsRef<Path>,
) -> Result<BuiltAndInspectedM1KernelArtifactsV1, M1KernelArtifactBuildErrorV1> {
    let M1KernelWorkerV3EvidenceSetV1 {
        gemm: gemm_evidence,
        rmsnorm: rmsnorm_evidence,
        rope_kv: rope_kv_evidence,
        prefill: prefill_evidence,
        paged_decode: paged_decode_evidence,
        swiglu: swiglu_evidence,
        logits: logits_evidence,
    } = evidence;
    for (family, worker) in [
        (M1KernelArtifactFamilyV1::Gemm, &gemm_evidence),
        (M1KernelArtifactFamilyV1::RmsNorm, &rmsnorm_evidence),
        (M1KernelArtifactFamilyV1::RopeKv, &rope_kv_evidence),
        (M1KernelArtifactFamilyV1::Prefill, &prefill_evidence),
        (
            M1KernelArtifactFamilyV1::PagedDecode,
            &paged_decode_evidence,
        ),
        (M1KernelArtifactFamilyV1::SwiGlu, &swiglu_evidence),
        (M1KernelArtifactFamilyV1::Logits, &logits_evidence),
    ] {
        if !worker_measurement_matches_m1_kernel_policy_v1(worker.worker_measurement()) {
            return Err(M1KernelArtifactBuildErrorV1::WorkerMeasurementPolicy(
                family,
            ));
        }
        if !worker_execution_policy_matches_m1_kernel_policy_v1(
            worker.execution_limits(),
            worker.plan().options(),
        ) {
            return Err(M1KernelArtifactBuildErrorV1::WorkerExecutionPolicy(family));
        }
    }
    let output_directory = output_directory.as_ref();
    let mut staging = StagingDirectory::create(output_directory)?;

    let gemm = build_gemm(gemm_evidence)?;
    let rmsnorm = build_rmsnorm(rmsnorm_evidence)?;
    let rope_kv = build_rope_kv(rope_kv_evidence)?;
    let prefill = build_prefill(prefill_evidence)?;
    let paged_decode = build_paged_decode(paged_decode_evidence)?;
    let swiglu = build_swiglu(swiglu_evidence)?;
    let logits = build_logits(logits_evidence)?;

    let entries = [
        gemm.entry.clone(),
        rmsnorm.entry.clone(),
        rope_kv.entry.clone(),
        prefill.entry.clone(),
        paged_decode.entry.clone(),
        swiglu.entry.clone(),
        logits.entry.clone(),
    ];
    require_distinct_artifacts(&entries)?;
    let manifest = M1KernelArtifactManifestV1::new(entries)?;

    for (family, identity, bytes) in [
        (
            M1KernelArtifactFamilyV1::Gemm,
            gemm.entry.artifact(),
            gemm.owner.exact_worker_output_bytes(),
        ),
        (
            M1KernelArtifactFamilyV1::RmsNorm,
            rmsnorm.entry.artifact(),
            rmsnorm.owner.exact_worker_output_bytes(),
        ),
        (
            M1KernelArtifactFamilyV1::RopeKv,
            rope_kv.entry.artifact(),
            rope_kv.owner.exact_worker_output_bytes(),
        ),
        (
            M1KernelArtifactFamilyV1::Prefill,
            prefill.entry.artifact(),
            prefill.owner.exact_worker_output_bytes(),
        ),
        (
            M1KernelArtifactFamilyV1::PagedDecode,
            paged_decode.entry.artifact(),
            paged_decode.owner.exact_worker_output_bytes(),
        ),
        (
            M1KernelArtifactFamilyV1::SwiGlu,
            swiglu.entry.artifact(),
            swiglu.owner.exact_worker_output_bytes(),
        ),
        (
            M1KernelArtifactFamilyV1::Logits,
            logits.entry.artifact(),
            logits.owner.exact_worker_output_bytes(),
        ),
    ] {
        write_object(staging.path(), family, identity, bytes)?;
    }
    write_new_file(
        &staging.path().join(M1_KERNEL_ARTIFACT_MANIFEST_FILENAME_V1),
        manifest.canonical_bytes(),
    )?;
    sync_directory(&staging.path().join("objects/sha256"))?;
    sync_directory(&staging.path().join("objects"))?;
    sync_directory(staging.path())?;
    let publication = staging.publish(output_directory)?;

    Ok(BuiltAndInspectedM1KernelArtifactsV1 {
        gemm: gemm.owner,
        rmsnorm: rmsnorm.owner,
        rope_kv: rope_kv.owner,
        prefill: prefill.owner,
        paged_decode: paged_decode.owner,
        swiglu: swiglu.owner,
        logits: logits.owner,
        manifest,
        output_directory: output_directory.to_path_buf(),
        publication,
    })
}

struct CompletedFamily<T> {
    owner: T,
    entry: M1KernelArtifactEntryV1,
}

fn gemm_source_facts(prepared: &gemm::PreparedQwen3GemmKernelV1) -> M1CurrentKernelSourceFactsV1 {
    M1CurrentKernelSourceFactsV1 {
        family: M1KernelArtifactFamilyV1::Gemm,
        compiler_module: ContentIdentityV1::calculate(prepared.compiler_handoff().module_bytes()),
        compiler_handoff: handoff_identity(prepared.compiler_handoff_identity()),
        symbol_manifest: symbol_identity(prepared.manifest_identity()),
        profile_catalogs: vec![
            M1KernelProfileCatalogV1::new(
                "gemm",
                gemm::QWEN3_GEMM_PROFILE_COUNT_V1,
                *prepared.catalog().identity().as_bytes(),
            ),
            M1KernelProfileCatalogV1::new(
                "token-embedding",
                gemm::QWEN3_TOKEN_EMBEDDING_PROFILE_COUNT_V1,
                *prepared.token_embedding_catalog().identity().as_bytes(),
            ),
        ],
    }
}

fn rmsnorm_source_facts(
    prepared: &rmsnorm::PreparedQwen3RmsNormKernelV1,
) -> M1CurrentKernelSourceFactsV1 {
    M1CurrentKernelSourceFactsV1 {
        family: M1KernelArtifactFamilyV1::RmsNorm,
        compiler_module: ContentIdentityV1::calculate(prepared.compiler_handoff().module_bytes()),
        compiler_handoff: handoff_identity(prepared.compiler_handoff_identity()),
        symbol_manifest: symbol_identity(prepared.manifest_identity()),
        profile_catalogs: vec![M1KernelProfileCatalogV1::new(
            "rmsnorm",
            rmsnorm::QWEN3_RMSNORM_PROFILE_COUNT_V1,
            *prepared.catalog().identity().as_bytes(),
        )],
    }
}

fn rope_kv_source_facts(
    prepared: &rope_kv::PreparedQwen3RopeKvKernelV1,
) -> M1CurrentKernelSourceFactsV1 {
    M1CurrentKernelSourceFactsV1 {
        family: M1KernelArtifactFamilyV1::RopeKv,
        compiler_module: ContentIdentityV1::calculate(prepared.compiler_handoff().module_bytes()),
        compiler_handoff: handoff_identity(prepared.compiler_handoff_identity()),
        symbol_manifest: symbol_identity(prepared.manifest_identity()),
        profile_catalogs: vec![M1KernelProfileCatalogV1::new(
            "rope-kv",
            rope_kv::QWEN3_ROPE_KV_PROFILE_COUNT_V1,
            *prepared.catalog().identity().as_bytes(),
        )],
    }
}

fn prefill_source_facts(
    prepared: &prefill::PreparedQwen3PrefillKernelV1,
) -> M1CurrentKernelSourceFactsV1 {
    M1CurrentKernelSourceFactsV1 {
        family: M1KernelArtifactFamilyV1::Prefill,
        compiler_module: ContentIdentityV1::calculate(prepared.compiler_handoff().module_bytes()),
        compiler_handoff: handoff_identity(prepared.compiler_handoff_identity()),
        symbol_manifest: symbol_identity(prepared.manifest_identity()),
        profile_catalogs: vec![M1KernelProfileCatalogV1::new(
            "prefill",
            prefill::QWEN3_PREFILL_PROFILE_COUNT_V1,
            *prepared.catalog().identity().as_bytes(),
        )],
    }
}

fn paged_decode_source_facts(
    prepared: &paged_decode::PreparedQwen3PagedDecodeKernelV1,
) -> M1CurrentKernelSourceFactsV1 {
    M1CurrentKernelSourceFactsV1 {
        family: M1KernelArtifactFamilyV1::PagedDecode,
        compiler_module: ContentIdentityV1::calculate(prepared.compiler_handoff().module_bytes()),
        compiler_handoff: handoff_identity(prepared.compiler_handoff_identity()),
        symbol_manifest: symbol_identity(prepared.manifest_identity()),
        profile_catalogs: vec![M1KernelProfileCatalogV1::new(
            "paged-decode",
            paged_decode::QWEN3_PAGED_DECODE_PROFILE_COUNT_V1,
            *prepared.catalog().identity().as_bytes(),
        )],
    }
}

fn swiglu_source_facts(
    prepared: &swiglu::PreparedQwen3SwiGluKernelV1,
) -> M1CurrentKernelSourceFactsV1 {
    M1CurrentKernelSourceFactsV1 {
        family: M1KernelArtifactFamilyV1::SwiGlu,
        compiler_module: ContentIdentityV1::calculate(prepared.compiler_handoff().module_bytes()),
        compiler_handoff: handoff_identity(prepared.compiler_handoff_identity()),
        symbol_manifest: symbol_identity(prepared.manifest_identity()),
        profile_catalogs: vec![M1KernelProfileCatalogV1::new(
            "swiglu",
            swiglu::QWEN3_SWIGLU_PROFILE_COUNT_V1,
            *prepared.catalog().identity().as_bytes(),
        )],
    }
}

fn logits_source_facts(
    prepared: &logits::PreparedQwen3LogitsKernelV1,
) -> M1CurrentKernelSourceFactsV1 {
    M1CurrentKernelSourceFactsV1 {
        family: M1KernelArtifactFamilyV1::Logits,
        compiler_module: ContentIdentityV1::calculate(prepared.compiler_handoff().module_bytes()),
        compiler_handoff: handoff_identity(prepared.compiler_handoff_identity()),
        symbol_manifest: symbol_identity(prepared.manifest_identity()),
        profile_catalogs: vec![
            M1KernelProfileCatalogV1::new(
                "logits",
                logits::QWEN3_LOGITS_PROFILE_COUNT_V1,
                *prepared.catalog().identity().as_bytes(),
            ),
            M1KernelProfileCatalogV1::new(
                "speculative-token-assembly",
                logits::QWEN3_SPECULATIVE_TOKEN_ASSEMBLY_PROFILE_COUNT_V1,
                speculative_assembly_catalog_identity(),
            ),
        ],
    }
}

/// Current canonical source facts for one M1 kernel family.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct M1CurrentKernelSourceFactsV1 {
    family: M1KernelArtifactFamilyV1,
    compiler_module: ContentIdentityV1,
    compiler_handoff: ContentIdentityV1,
    symbol_manifest: ContentIdentityV1,
    profile_catalogs: Vec<M1KernelProfileCatalogV1>,
}

impl M1CurrentKernelSourceFactsV1 {
    /// Stable K1-K7 family whose source was reconstructed.
    #[must_use]
    pub const fn family(&self) -> M1KernelArtifactFamilyV1 {
        self.family
    }

    /// Identity of the exact current LLVM module bytes.
    #[must_use]
    pub const fn compiler_module(&self) -> ContentIdentityV1 {
        self.compiler_module
    }

    /// Identity of the exact current canonical compiler handoff.
    #[must_use]
    pub const fn compiler_handoff(&self) -> ContentIdentityV1 {
        self.compiler_handoff
    }

    /// Identity of the exact current compiler symbol manifest.
    #[must_use]
    pub const fn symbol_manifest(&self) -> ContentIdentityV1 {
        self.symbol_manifest
    }

    /// Complete current finite profile catalogs for this family.
    #[must_use]
    pub fn profile_catalogs(&self) -> &[M1KernelProfileCatalogV1] {
        &self.profile_catalogs
    }
}

fn build_gemm(
    worker: InertProtectedFirstBuildWorkerV3EvidenceV1,
) -> Result<CompletedFamily<gemm::InspectedQwen3GemmKernelV1>, M1KernelArtifactBuildErrorV1> {
    let family = M1KernelArtifactFamilyV1::Gemm;
    let labels = inert_labels(family);
    let prepared = gemm::prepare_qwen3_gemm_kernel_v1(gemm::Qwen3GemmSourceBindingsV1::new(
        labels[0], labels[1], labels[2], labels[3],
    ))
    .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::Prepare, source))?;
    let facts = gemm_source_facts(&prepared);
    let evidence =
        gemm::bind_qwen3_gemm_worker_v3_v1(gemm::lower_qwen3_gemm_kernel_v1(prepared), worker)
            .map_err(|source| {
                family_error(family, M1KernelArtifactBuildStageV1::BindWorkerV3, source)
            })?;
    let owner = gemm::inspect_qwen3_gemm_kernel_v1(evidence)
        .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::Inspect, source))?;
    let load_plan = *owner.loader_plan();
    let artifact = ContentIdentityV1::calculate(owner.exact_worker_output_bytes());
    Ok(completed(family, facts, load_plan, artifact, owner))
}

fn build_rmsnorm(
    worker: InertProtectedFirstBuildWorkerV3EvidenceV1,
) -> Result<CompletedFamily<rmsnorm::InspectedQwen3RmsNormKernelV1>, M1KernelArtifactBuildErrorV1> {
    let family = M1KernelArtifactFamilyV1::RmsNorm;
    let labels = inert_labels(family);
    let prepared = rmsnorm::prepare_qwen3_rmsnorm_kernel_v1(
        rmsnorm::Qwen3RmsNormSourceBindingsV1::new(labels[0], labels[1], labels[2], labels[3]),
    )
    .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::Prepare, source))?;
    let facts = rmsnorm_source_facts(&prepared);
    let evidence = rmsnorm::bind_qwen3_rmsnorm_worker_v3_v1(
        rmsnorm::lower_qwen3_rmsnorm_kernel_v1(prepared),
        worker,
    )
    .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::BindWorkerV3, source))?;
    let owner = rmsnorm::inspect_qwen3_rmsnorm_kernel_v1(evidence)
        .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::Inspect, source))?;
    let load_plan = *owner.loader_plan();
    let artifact = ContentIdentityV1::calculate(owner.exact_worker_output_bytes());
    Ok(completed(family, facts, load_plan, artifact, owner))
}

fn build_rope_kv(
    worker: InertProtectedFirstBuildWorkerV3EvidenceV1,
) -> Result<CompletedFamily<rope_kv::InspectedQwen3RopeKvKernelV1>, M1KernelArtifactBuildErrorV1> {
    let family = M1KernelArtifactFamilyV1::RopeKv;
    let labels = inert_labels(family);
    let prepared = rope_kv::prepare_qwen3_rope_kv_kernel_v1(
        rope_kv::Qwen3RopeKvSourceBindingsV1::new(labels[0], labels[1], labels[2], labels[3]),
    )
    .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::Prepare, source))?;
    let facts = rope_kv_source_facts(&prepared);
    let evidence = rope_kv::bind_qwen3_rope_kv_worker_v3_v1(
        rope_kv::lower_qwen3_rope_kv_kernel_v1(prepared),
        worker,
    )
    .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::BindWorkerV3, source))?;
    let owner = rope_kv::inspect_qwen3_rope_kv_kernel_v1(evidence)
        .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::Inspect, source))?;
    let load_plan = *owner.loader_plan();
    let artifact = ContentIdentityV1::calculate(owner.exact_worker_output_bytes());
    Ok(completed(family, facts, load_plan, artifact, owner))
}

fn build_prefill(
    worker: InertProtectedFirstBuildWorkerV3EvidenceV1,
) -> Result<CompletedFamily<prefill::InspectedQwen3PrefillKernelV1>, M1KernelArtifactBuildErrorV1> {
    let family = M1KernelArtifactFamilyV1::Prefill;
    let labels = inert_labels(family);
    let prepared = prefill::prepare_qwen3_prefill_kernel_v1(
        prefill::Qwen3PrefillSourceBindingsV1::new(labels[0], labels[1], labels[2], labels[3]),
    )
    .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::Prepare, source))?;
    let facts = prefill_source_facts(&prepared);
    let evidence = prefill::bind_qwen3_prefill_worker_v3_v1(
        prefill::lower_qwen3_prefill_kernel_v1(prepared),
        worker,
    )
    .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::BindWorkerV3, source))?;
    let owner = prefill::inspect_qwen3_prefill_kernel_v1(evidence)
        .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::Inspect, source))?;
    let load_plan = *owner.loader_plan();
    let artifact = ContentIdentityV1::calculate(owner.exact_worker_output_bytes());
    Ok(completed(family, facts, load_plan, artifact, owner))
}

fn build_paged_decode(
    worker: InertProtectedFirstBuildWorkerV3EvidenceV1,
) -> Result<
    CompletedFamily<paged_decode::InspectedQwen3PagedDecodeKernelV1>,
    M1KernelArtifactBuildErrorV1,
> {
    let family = M1KernelArtifactFamilyV1::PagedDecode;
    let labels = inert_labels(family);
    let prepared = paged_decode::prepare_qwen3_paged_decode_kernel_v1(
        paged_decode::Qwen3PagedDecodeSourceBindingsV1::new(
            labels[0], labels[1], labels[2], labels[3],
        ),
    )
    .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::Prepare, source))?;
    let facts = paged_decode_source_facts(&prepared);
    let evidence = paged_decode::bind_qwen3_paged_decode_worker_v3_v1(
        paged_decode::lower_qwen3_paged_decode_kernel_v1(prepared),
        worker,
    )
    .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::BindWorkerV3, source))?;
    let owner = paged_decode::inspect_qwen3_paged_decode_kernel_v1(evidence)
        .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::Inspect, source))?;
    let load_plan = *owner.loader_plan();
    let artifact = ContentIdentityV1::calculate(owner.exact_worker_output_bytes());
    Ok(completed(family, facts, load_plan, artifact, owner))
}

fn build_swiglu(
    worker: InertProtectedFirstBuildWorkerV3EvidenceV1,
) -> Result<CompletedFamily<swiglu::InspectedQwen3SwiGluKernelV1>, M1KernelArtifactBuildErrorV1> {
    let family = M1KernelArtifactFamilyV1::SwiGlu;
    let labels = inert_labels(family);
    let prepared = swiglu::prepare_qwen3_swiglu_kernel_v1(
        swiglu::Qwen3SwiGluSourceBindingsV1::new(labels[0], labels[1], labels[2], labels[3]),
    )
    .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::Prepare, source))?;
    let facts = swiglu_source_facts(&prepared);
    let evidence = swiglu::bind_qwen3_swiglu_worker_v3_v1(
        swiglu::lower_qwen3_swiglu_kernel_v1(prepared),
        worker,
    )
    .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::BindWorkerV3, source))?;
    let owner = swiglu::inspect_qwen3_swiglu_kernel_v1(evidence)
        .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::Inspect, source))?;
    let load_plan = *owner.loader_plan();
    let artifact = ContentIdentityV1::calculate(owner.exact_worker_output_bytes());
    Ok(completed(family, facts, load_plan, artifact, owner))
}

fn build_logits(
    worker: InertProtectedFirstBuildWorkerV3EvidenceV1,
) -> Result<CompletedFamily<logits::InspectedQwen3LogitsKernelV1>, M1KernelArtifactBuildErrorV1> {
    let family = M1KernelArtifactFamilyV1::Logits;
    let labels = inert_labels(family);
    let prepared = logits::prepare_qwen3_logits_kernel_v1(
        logits::Qwen3LogitsSourceBindingsV1::new(labels[0], labels[1], labels[2], labels[3]),
    )
    .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::Prepare, source))?;
    let facts = logits_source_facts(&prepared);
    let evidence = logits::bind_qwen3_logits_worker_v3_v1(
        logits::lower_qwen3_logits_kernel_v1(prepared),
        worker,
    )
    .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::BindWorkerV3, source))?;
    let owner = logits::inspect_qwen3_logits_kernel_v1(evidence)
        .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::Inspect, source))?;
    let load_plan = *owner.loader_plan();
    let artifact = ContentIdentityV1::calculate(owner.exact_worker_output_bytes());
    Ok(completed(family, facts, load_plan, artifact, owner))
}

fn completed<T>(
    family: M1KernelArtifactFamilyV1,
    facts: M1CurrentKernelSourceFactsV1,
    load_plan: fe2o3_amdhsa_loader::LoadPlan,
    artifact: ContentIdentityV1,
    owner: T,
) -> CompletedFamily<T> {
    CompletedFamily {
        owner,
        entry: M1KernelArtifactEntryV1::new(
            family,
            artifact,
            facts.compiler_module,
            facts.compiler_handoff,
            facts.symbol_manifest,
            facts.profile_catalogs,
            &load_plan,
        ),
    }
}

fn handoff_identity(
    identity: fe2o3_compiler_ffi::CompilerModuleHandoffIdentityV2,
) -> ContentIdentityV1 {
    ContentIdentityV1::from_parts(*identity.sha256(), identity.byte_len())
}

fn symbol_identity(
    identity: fe2o3_compiler_ffi::CompilerModuleSymbolManifestIdentityV1,
) -> ContentIdentityV1 {
    ContentIdentityV1::from_parts(*identity.sha256(), identity.byte_len())
}

fn inert_labels(family: M1KernelArtifactFamilyV1) -> [[u8; 32]; 4] {
    [0_u8, 1, 2, 3].map(|role| {
        let mut digest = Sha256::new();
        digest.update((LABEL_DOMAIN.len() as u64).to_le_bytes());
        digest.update(LABEL_DOMAIN);
        digest.update([family as u8, role]);
        digest.finalize().into()
    })
}

fn family_error(
    family: M1KernelArtifactFamilyV1,
    stage: M1KernelArtifactBuildStageV1,
    source: impl Error + Send + Sync + 'static,
) -> M1KernelArtifactBuildErrorV1 {
    M1KernelArtifactBuildErrorV1::Family {
        family,
        stage,
        source: Box::new(source),
    }
}

fn require_distinct_artifacts(
    entries: &[M1KernelArtifactEntryV1; M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1],
) -> Result<(), M1KernelArtifactBuildErrorV1> {
    for (index, entry) in entries.iter().enumerate() {
        if entries[..index]
            .iter()
            .any(|prior| prior.artifact() == entry.artifact())
        {
            return Err(M1KernelArtifactBuildErrorV1::DuplicateArtifact);
        }
    }
    Ok(())
}

fn write_object(
    staging: &Path,
    family: M1KernelArtifactFamilyV1,
    identity: ContentIdentityV1,
    bytes: &[u8],
) -> Result<(), M1KernelArtifactBuildErrorV1> {
    if !identity.matches(bytes) {
        return Err(M1KernelArtifactBuildErrorV1::ArtifactIdentity(family));
    }
    let directory = staging.join("objects/sha256");
    if !directory.exists() {
        fs::create_dir_all(&directory).map_err(|source| M1KernelArtifactBuildErrorV1::Io {
            operation: "create object directory",
            path: directory.clone(),
            source,
        })?;
    }
    let path = directory.join(format!("{}.hsaco", hex(identity.sha256())));
    write_new_file(&path, bytes)
}

fn write_new_file(path: &Path, bytes: &[u8]) -> Result<(), M1KernelArtifactBuildErrorV1> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|source| M1KernelArtifactBuildErrorV1::Io {
            operation: "create file",
            path: path.to_path_buf(),
            source,
        })?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|source| M1KernelArtifactBuildErrorV1::Io {
            operation: "write and sync file",
            path: path.to_path_buf(),
            source,
        })
}

fn sync_directory(path: &Path) -> Result<(), M1KernelArtifactBuildErrorV1> {
    sync_directory_raw(path).map_err(|source| M1KernelArtifactBuildErrorV1::Io {
        operation: "sync directory",
        path: path.to_path_buf(),
        source,
    })
}

fn sync_directory_raw(path: &Path) -> io::Result<()> {
    File::open(path).and_then(|directory| directory.sync_all())
}

struct StagingDirectory {
    path: PathBuf,
    armed: bool,
}

impl StagingDirectory {
    fn create(output: &Path) -> Result<Self, M1KernelArtifactBuildErrorV1> {
        if output.exists() {
            return Err(M1KernelArtifactBuildErrorV1::OutputAlreadyExists(
                output.to_path_buf(),
            ));
        }
        let parent = output.parent().unwrap_or_else(|| Path::new("."));
        let name = output
            .file_name()
            .ok_or(M1KernelArtifactBuildErrorV1::InvalidOutputPath)?;
        for nonce in 0..1_024_u16 {
            let mut staging_name = OsString::from(".");
            staging_name.push(name);
            staging_name.push(format!(".staging.{}.{nonce}", std::process::id()));
            let path = parent.join(staging_name);
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self { path, armed: true }),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(source) => {
                    return Err(M1KernelArtifactBuildErrorV1::Io {
                        operation: "create staging directory",
                        path,
                        source,
                    });
                }
            }
        }
        Err(M1KernelArtifactBuildErrorV1::Io {
            operation: "create unique staging directory",
            path: parent.to_path_buf(),
            source: io::Error::new(io::ErrorKind::AlreadyExists, "staging namespace exhausted"),
        })
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn publish(
        &mut self,
        output: &Path,
    ) -> Result<M1KernelArtifactPublicationStatusV1, M1KernelArtifactBuildErrorV1> {
        self.publish_with(output, sync_directory_raw)
    }

    fn publish_with(
        &mut self,
        output: &Path,
        sync_parent: impl FnOnce(&Path) -> io::Result<()>,
    ) -> Result<M1KernelArtifactPublicationStatusV1, M1KernelArtifactBuildErrorV1> {
        match renameat_with(CWD, &self.path, CWD, output, RenameFlags::NOREPLACE) {
            Ok(()) => {}
            Err(source) if source == rustix::io::Errno::EXIST => {
                return Err(M1KernelArtifactBuildErrorV1::OutputAlreadyExists(
                    output.to_path_buf(),
                ));
            }
            Err(source) => {
                return Err(M1KernelArtifactBuildErrorV1::Io {
                    operation: "publish staging directory without replacement",
                    path: output.to_path_buf(),
                    source: source.into(),
                });
            }
        }
        self.armed = false;
        let parent = output.parent().unwrap_or_else(|| Path::new("."));
        Ok(match sync_parent(parent) {
            Ok(()) => M1KernelArtifactPublicationStatusV1::ParentDirectorySynced,
            Err(source) => {
                M1KernelArtifactPublicationStatusV1::PublishedButParentDirectorySyncFailed {
                    parent_directory: parent.to_path_buf(),
                    source,
                }
            }
        })
    }
}

impl Drop for StagingDirectory {
    fn drop(&mut self) {
        if self.armed {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(DIGITS[(byte >> 4) as usize] as char);
        encoded.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    encoded
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(label: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "ferric-m1-artifact-{label}-{}-{nonce}",
                std::process::id()
            ));
            fs::create_dir(&path).expect("create test directory");
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn private_inert_labels_are_deterministic_nonzero_and_globally_distinct() {
        let mut labels = BTreeSet::new();
        for family in M1KernelArtifactFamilyV1::ALL {
            assert_eq!(inert_labels(family), inert_labels(family));
            for label in inert_labels(family) {
                assert_ne!(label, [0; 32]);
                assert!(labels.insert(label));
            }
        }
        assert_eq!(labels.len(), M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1 * 4);
    }

    #[test]
    fn staging_drop_removes_partial_output_and_preserves_final_path() {
        let root = TestDirectory::new("staging-drop");
        let output = root.0.join("final");
        let staging_path = {
            let staging = StagingDirectory::create(&output).expect("create staging");
            let path = staging.path().to_path_buf();
            write_new_file(&path.join("partial"), b"partial").expect("write partial");
            path
        };
        assert!(!staging_path.exists());
        assert!(!output.exists());
    }

    #[test]
    fn existing_output_is_rejected_without_mutation() {
        let root = TestDirectory::new("existing-output");
        let output = root.0.join("final");
        fs::create_dir(&output).unwrap();
        write_new_file(&output.join("sentinel"), b"retained").unwrap();
        assert!(matches!(
            StagingDirectory::create(&output),
            Err(M1KernelArtifactBuildErrorV1::OutputAlreadyExists(path)) if path == output
        ));
        assert_eq!(fs::read(output.join("sentinel")).unwrap(), b"retained");
    }

    #[test]
    fn destination_created_after_staging_is_never_replaced() {
        let root = TestDirectory::new("publication-race");
        let output = root.0.join("final");
        let staging_path;
        {
            let mut staging = StagingDirectory::create(&output).expect("create staging");
            staging_path = staging.path().to_path_buf();
            write_new_file(&staging_path.join("complete"), b"complete").unwrap();

            fs::create_dir(&output).unwrap();
            write_new_file(&output.join("concurrent-owner"), b"retained").unwrap();
            assert!(matches!(
                staging.publish(&output),
                Err(M1KernelArtifactBuildErrorV1::OutputAlreadyExists(path)) if path == output
            ));
            assert!(staging_path.exists());
        }
        assert!(!staging_path.exists());
        assert_eq!(
            fs::read(output.join("concurrent-owner")).unwrap(),
            b"retained"
        );
        assert!(!output.join("complete").exists());
    }

    #[test]
    fn post_rename_sync_failure_is_an_explicit_published_terminal_state() {
        let root = TestDirectory::new("terminal-sync");
        let output = root.0.join("final");
        let mut staging = StagingDirectory::create(&output).expect("create staging");
        let staging_path = staging.path().to_path_buf();
        write_new_file(&staging_path.join("complete"), b"complete").unwrap();
        let status = staging
            .publish_with(&output, |_| {
                Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "injected parent sync failure",
                ))
            })
            .unwrap();
        assert!(matches!(
            status,
            M1KernelArtifactPublicationStatusV1::PublishedButParentDirectorySyncFailed {
                parent_directory,
                source,
            } if parent_directory == root.0 && source.kind() == io::ErrorKind::PermissionDenied
        ));
        assert!(!staging_path.exists());
        assert_eq!(fs::read(output.join("complete")).unwrap(), b"complete");
    }

    #[test]
    fn content_object_rejects_substituted_bytes() {
        let root = TestDirectory::new("object-substitution");
        let identity = ContentIdentityV1::calculate(b"expected");
        assert!(matches!(
            write_object(
                &root.0,
                M1KernelArtifactFamilyV1::Gemm,
                identity,
                b"substituted",
            ),
            Err(M1KernelArtifactBuildErrorV1::ArtifactIdentity(
                M1KernelArtifactFamilyV1::Gemm
            ))
        ));
        assert!(!root.0.join("objects").exists());
    }
}
