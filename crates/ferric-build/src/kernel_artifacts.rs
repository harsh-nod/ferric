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

use fe2o3_artifact_transaction::{
    begin_build_attempt, consume_compiler_module_handoff_v1, fail_build_attempt,
    publish_compiler_module_handoff_v1, BuildAttempt, BuildInvocation, BuildSession,
    ConsumedCompilerModuleHandoffV1, ProducerIdentity,
};
use fe2o3_hsaco_finalize::{ContentIdentityV1, PinnedWorkerV1, WorkerExecutionLimitsV1};
use ferric_qwen_kernels::{gemm, logits, paged_decode, prefill, rmsnorm, rope_kv, swiglu};
use rustix::fs::{renameat_with, RenameFlags, CWD};
use sha2::{Digest, Sha256};

use super::kernel_artifact_manifest::{
    speculative_assembly_catalog_identity, M1KernelArtifactEntryV1, M1KernelArtifactFamilyV1,
    M1KernelArtifactManifestErrorV1, M1KernelArtifactManifestV1, M1KernelProfileCatalogV1,
    M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1,
};
use super::kernel_artifact_policy::open_m1_kernel_worker_v1;

/// Stable filename of the canonical manifest inside a published output directory.
pub const M1_KERNEL_ARTIFACT_MANIFEST_FILENAME_V1: &str = "m1-kernel-artifacts.manifest.bin";

const LABEL_DOMAIN: &[u8] = b"ferric.m1.kernel-artifact-builder.inert-label.v1";
const INVOCATION_DOMAIN: &[u8] = b"ferric.m1.kernel-artifact-builder.invocation.v1";
const TRANSACTION_SESSION: BuildSession = BuildSession::from_bytes([
    0x66, 0x65, 0x72, 0x72, 0x69, 0x63, 0x2d, 0x6d, 0x31, 0x2d, 0x6b, 0x31, 0x6b, 0x37, 0x01, 0x01,
]);

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
    /// Durable one-shot compiler-handoff transport.
    Handoff,
    /// Measured Worker V2 bootstrap and exact replay.
    Execute,
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
    /// The Ferric-owned Worker measurement rejected the supplied path/image.
    Worker(fe2o3_hsaco_finalize::WorkerExecutionError),
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
            Self::Worker(source) => Some(source),
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

/// Builds, structurally inspects, and atomically publishes exactly one HSACO
/// for each K1-K7 family.
///
/// `worker_path` is the only caller-controlled Worker input. Ferric supplies
/// the exact reviewed executable digest, length, Worker claim, LLVM identity,
/// fixed link options, and measured OCML policy. `output_directory` must not
/// exist. An atomic no-replace rename happens only after all seven objects and
/// the strict canonical manifest have been synced. If syncing the parent
/// directory then fails, the function returns the live owner successfully with
/// [`M1KernelArtifactPublicationStatusV1::PublishedButParentDirectorySyncFailed`]
/// because the final directory is already visible and cannot be rolled back
/// transactionally.
///
/// # Errors
///
/// Returns [`M1KernelArtifactBuildErrorV1`] if the Worker image differs from
/// Ferric's pin, any prepare/transport/execute/inspect stage fails closed, an
/// artifact identity drifts, or filesystem staging fails before publication.
/// A no-replace collision also returns an error while preserving both the
/// existing destination and automatic staging cleanup. No error is returned
/// after the final directory has become visible.
pub fn build_and_publish_m1_kernel_artifacts_v1(
    worker_path: impl AsRef<Path>,
    output_directory: impl AsRef<Path>,
) -> Result<BuiltAndInspectedM1KernelArtifactsV1, M1KernelArtifactBuildErrorV1> {
    let output_directory = output_directory.as_ref();
    let mut staging = StagingDirectory::create(output_directory)?;
    let transaction_directory = staging.path().join("transactions");
    create_directory(&transaction_directory, "create transaction directory")?;
    let worker = open_m1_kernel_worker_v1(worker_path.as_ref())
        .map_err(M1KernelArtifactBuildErrorV1::Worker)?;
    let limits = WorkerExecutionLimitsV1::default();

    let gemm = build_gemm(&transaction_directory, &worker, limits)?;
    let rmsnorm = build_rmsnorm(&transaction_directory, &worker, limits)?;
    let rope_kv = build_rope_kv(&transaction_directory, &worker, limits)?;
    let prefill = build_prefill(&transaction_directory, &worker, limits)?;
    let paged_decode = build_paged_decode(&transaction_directory, &worker, limits)?;
    let swiglu = build_swiglu(&transaction_directory, &worker, limits)?;
    let logits = build_logits(&transaction_directory, &worker, limits)?;

    remove_directory(
        &transaction_directory,
        "remove consumed transaction directory",
    )?;

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

struct PreparedFacts {
    compiler_module: ContentIdentityV1,
    compiler_handoff: ContentIdentityV1,
    symbol_manifest: ContentIdentityV1,
    profile_catalogs: Vec<M1KernelProfileCatalogV1>,
}

fn build_gemm(
    transactions: &Path,
    worker: &PinnedWorkerV1,
    limits: WorkerExecutionLimitsV1,
) -> Result<CompletedFamily<gemm::InspectedQwen3GemmKernelV1>, M1KernelArtifactBuildErrorV1> {
    let family = M1KernelArtifactFamilyV1::Gemm;
    let labels = inert_labels(family);
    let prepared = gemm::prepare_qwen3_gemm_kernel_v1(gemm::Qwen3GemmSourceBindingsV1::new(
        labels[0], labels[1], labels[2], labels[3],
    ))
    .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::Prepare, source))?;
    let facts = PreparedFacts {
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
    };
    let consumed = transport_handoff(
        transactions,
        family,
        prepared.compiler_handoff().canonical_bytes(),
    )?;
    let evidence = gemm::execute_qwen3_gemm_worker_v2_v1(
        gemm::lower_qwen3_gemm_kernel_v1(prepared),
        consumed,
        worker,
        limits,
    )
    .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::Execute, source))?;
    let owner = gemm::inspect_qwen3_gemm_kernel_v1(evidence)
        .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::Inspect, source))?;
    let load_plan = *owner.loader_plan();
    let artifact = ContentIdentityV1::calculate(owner.exact_worker_output_bytes());
    Ok(completed(family, facts, load_plan, artifact, owner))
}

fn build_rmsnorm(
    transactions: &Path,
    worker: &PinnedWorkerV1,
    limits: WorkerExecutionLimitsV1,
) -> Result<CompletedFamily<rmsnorm::InspectedQwen3RmsNormKernelV1>, M1KernelArtifactBuildErrorV1> {
    let family = M1KernelArtifactFamilyV1::RmsNorm;
    let labels = inert_labels(family);
    let prepared = rmsnorm::prepare_qwen3_rmsnorm_kernel_v1(
        rmsnorm::Qwen3RmsNormSourceBindingsV1::new(labels[0], labels[1], labels[2], labels[3]),
    )
    .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::Prepare, source))?;
    let facts = PreparedFacts {
        compiler_module: ContentIdentityV1::calculate(prepared.compiler_handoff().module_bytes()),
        compiler_handoff: handoff_identity(prepared.compiler_handoff_identity()),
        symbol_manifest: symbol_identity(prepared.manifest_identity()),
        profile_catalogs: vec![M1KernelProfileCatalogV1::new(
            "rmsnorm",
            rmsnorm::QWEN3_RMSNORM_PROFILE_COUNT_V1,
            *prepared.catalog().identity().as_bytes(),
        )],
    };
    let consumed = transport_handoff(
        transactions,
        family,
        prepared.compiler_handoff().canonical_bytes(),
    )?;
    let evidence = rmsnorm::execute_qwen3_rmsnorm_worker_v2_v1(
        rmsnorm::lower_qwen3_rmsnorm_kernel_v1(prepared),
        consumed,
        worker,
        limits,
    )
    .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::Execute, source))?;
    let owner = rmsnorm::inspect_qwen3_rmsnorm_kernel_v1(evidence)
        .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::Inspect, source))?;
    let load_plan = *owner.loader_plan();
    let artifact = ContentIdentityV1::calculate(owner.exact_worker_output_bytes());
    Ok(completed(family, facts, load_plan, artifact, owner))
}

fn build_rope_kv(
    transactions: &Path,
    worker: &PinnedWorkerV1,
    limits: WorkerExecutionLimitsV1,
) -> Result<CompletedFamily<rope_kv::InspectedQwen3RopeKvKernelV1>, M1KernelArtifactBuildErrorV1> {
    let family = M1KernelArtifactFamilyV1::RopeKv;
    let labels = inert_labels(family);
    let prepared = rope_kv::prepare_qwen3_rope_kv_kernel_v1(
        rope_kv::Qwen3RopeKvSourceBindingsV1::new(labels[0], labels[1], labels[2], labels[3]),
    )
    .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::Prepare, source))?;
    let facts = PreparedFacts {
        compiler_module: ContentIdentityV1::calculate(prepared.compiler_handoff().module_bytes()),
        compiler_handoff: handoff_identity(prepared.compiler_handoff_identity()),
        symbol_manifest: symbol_identity(prepared.manifest_identity()),
        profile_catalogs: vec![M1KernelProfileCatalogV1::new(
            "rope-kv",
            rope_kv::QWEN3_ROPE_KV_PROFILE_COUNT_V1,
            *prepared.catalog().identity().as_bytes(),
        )],
    };
    let consumed = transport_handoff(
        transactions,
        family,
        prepared.compiler_handoff().canonical_bytes(),
    )?;
    let evidence = rope_kv::execute_qwen3_rope_kv_worker_v2_v1(
        rope_kv::lower_qwen3_rope_kv_kernel_v1(prepared),
        consumed,
        worker,
        limits,
    )
    .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::Execute, source))?;
    let owner = rope_kv::inspect_qwen3_rope_kv_kernel_v1(evidence)
        .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::Inspect, source))?;
    let load_plan = *owner.loader_plan();
    let artifact = ContentIdentityV1::calculate(owner.exact_worker_output_bytes());
    Ok(completed(family, facts, load_plan, artifact, owner))
}

fn build_prefill(
    transactions: &Path,
    worker: &PinnedWorkerV1,
    limits: WorkerExecutionLimitsV1,
) -> Result<CompletedFamily<prefill::InspectedQwen3PrefillKernelV1>, M1KernelArtifactBuildErrorV1> {
    let family = M1KernelArtifactFamilyV1::Prefill;
    let labels = inert_labels(family);
    let prepared = prefill::prepare_qwen3_prefill_kernel_v1(
        prefill::Qwen3PrefillSourceBindingsV1::new(labels[0], labels[1], labels[2], labels[3]),
    )
    .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::Prepare, source))?;
    let facts = PreparedFacts {
        compiler_module: ContentIdentityV1::calculate(prepared.compiler_handoff().module_bytes()),
        compiler_handoff: handoff_identity(prepared.compiler_handoff_identity()),
        symbol_manifest: symbol_identity(prepared.manifest_identity()),
        profile_catalogs: vec![M1KernelProfileCatalogV1::new(
            "prefill",
            prefill::QWEN3_PREFILL_PROFILE_COUNT_V1,
            *prepared.catalog().identity().as_bytes(),
        )],
    };
    let consumed = transport_handoff(
        transactions,
        family,
        prepared.compiler_handoff().canonical_bytes(),
    )?;
    let evidence = prefill::execute_qwen3_prefill_worker_v2_v1(
        prefill::lower_qwen3_prefill_kernel_v1(prepared),
        consumed,
        worker,
        limits,
    )
    .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::Execute, source))?;
    let owner = prefill::inspect_qwen3_prefill_kernel_v1(evidence)
        .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::Inspect, source))?;
    let load_plan = *owner.loader_plan();
    let artifact = ContentIdentityV1::calculate(owner.exact_worker_output_bytes());
    Ok(completed(family, facts, load_plan, artifact, owner))
}

fn build_paged_decode(
    transactions: &Path,
    worker: &PinnedWorkerV1,
    limits: WorkerExecutionLimitsV1,
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
    let facts = PreparedFacts {
        compiler_module: ContentIdentityV1::calculate(prepared.compiler_handoff().module_bytes()),
        compiler_handoff: handoff_identity(prepared.compiler_handoff_identity()),
        symbol_manifest: symbol_identity(prepared.manifest_identity()),
        profile_catalogs: vec![M1KernelProfileCatalogV1::new(
            "paged-decode",
            paged_decode::QWEN3_PAGED_DECODE_PROFILE_COUNT_V1,
            *prepared.catalog().identity().as_bytes(),
        )],
    };
    let consumed = transport_handoff(
        transactions,
        family,
        prepared.compiler_handoff().canonical_bytes(),
    )?;
    let evidence = paged_decode::execute_qwen3_paged_decode_worker_v2_v1(
        paged_decode::lower_qwen3_paged_decode_kernel_v1(prepared),
        consumed,
        worker,
        limits,
    )
    .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::Execute, source))?;
    let owner = paged_decode::inspect_qwen3_paged_decode_kernel_v1(evidence)
        .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::Inspect, source))?;
    let load_plan = *owner.loader_plan();
    let artifact = ContentIdentityV1::calculate(owner.exact_worker_output_bytes());
    Ok(completed(family, facts, load_plan, artifact, owner))
}

fn build_swiglu(
    transactions: &Path,
    worker: &PinnedWorkerV1,
    limits: WorkerExecutionLimitsV1,
) -> Result<CompletedFamily<swiglu::InspectedQwen3SwiGluKernelV1>, M1KernelArtifactBuildErrorV1> {
    let family = M1KernelArtifactFamilyV1::SwiGlu;
    let labels = inert_labels(family);
    let prepared = swiglu::prepare_qwen3_swiglu_kernel_v1(
        swiglu::Qwen3SwiGluSourceBindingsV1::new(labels[0], labels[1], labels[2], labels[3]),
    )
    .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::Prepare, source))?;
    let facts = PreparedFacts {
        compiler_module: ContentIdentityV1::calculate(prepared.compiler_handoff().module_bytes()),
        compiler_handoff: handoff_identity(prepared.compiler_handoff_identity()),
        symbol_manifest: symbol_identity(prepared.manifest_identity()),
        profile_catalogs: vec![M1KernelProfileCatalogV1::new(
            "swiglu",
            swiglu::QWEN3_SWIGLU_PROFILE_COUNT_V1,
            *prepared.catalog().identity().as_bytes(),
        )],
    };
    let consumed = transport_handoff(
        transactions,
        family,
        prepared.compiler_handoff().canonical_bytes(),
    )?;
    let evidence = swiglu::execute_qwen3_swiglu_worker_v2_v1(
        swiglu::lower_qwen3_swiglu_kernel_v1(prepared),
        consumed,
        worker,
        limits,
    )
    .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::Execute, source))?;
    let owner = swiglu::inspect_qwen3_swiglu_kernel_v1(evidence)
        .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::Inspect, source))?;
    let load_plan = *owner.loader_plan();
    let artifact = ContentIdentityV1::calculate(owner.exact_worker_output_bytes());
    Ok(completed(family, facts, load_plan, artifact, owner))
}

fn build_logits(
    transactions: &Path,
    worker: &PinnedWorkerV1,
    limits: WorkerExecutionLimitsV1,
) -> Result<CompletedFamily<logits::InspectedQwen3LogitsKernelV1>, M1KernelArtifactBuildErrorV1> {
    let family = M1KernelArtifactFamilyV1::Logits;
    let labels = inert_labels(family);
    let prepared = logits::prepare_qwen3_logits_kernel_v1(
        logits::Qwen3LogitsSourceBindingsV1::new(labels[0], labels[1], labels[2], labels[3]),
    )
    .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::Prepare, source))?;
    let facts = PreparedFacts {
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
    };
    let consumed = transport_handoff(
        transactions,
        family,
        prepared.compiler_handoff().canonical_bytes(),
    )?;
    let evidence = logits::execute_qwen3_logits_worker_v2_v1(
        logits::lower_qwen3_logits_kernel_v1(prepared),
        consumed,
        worker,
        limits,
    )
    .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::Execute, source))?;
    let owner = logits::inspect_qwen3_logits_kernel_v1(evidence)
        .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::Inspect, source))?;
    let load_plan = *owner.loader_plan();
    let artifact = ContentIdentityV1::calculate(owner.exact_worker_output_bytes());
    Ok(completed(family, facts, load_plan, artifact, owner))
}

fn completed<T>(
    family: M1KernelArtifactFamilyV1,
    facts: PreparedFacts,
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

fn transport_handoff(
    transactions: &Path,
    family: M1KernelArtifactFamilyV1,
    bytes: &[u8],
) -> Result<ConsumedCompilerModuleHandoffV1, M1KernelArtifactBuildErrorV1> {
    let producer = ProducerIdentity::from_codegen(family.name(), None)
        .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::Handoff, source))?;
    let mut digest = Sha256::new();
    digest.update((INVOCATION_DOMAIN.len() as u64).to_le_bytes());
    digest.update(INVOCATION_DOMAIN);
    digest.update([family as u8]);
    digest.update(ContentIdentityV1::calculate(bytes).sha256());
    let invocation = BuildInvocation::from_bytes(digest.finalize().into());
    let attempt = begin_build_attempt(transactions, &producer, invocation, TRANSACTION_SESSION)
        .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::Handoff, source))?;
    let mut active = ActiveHandoffAttempt {
        transactions,
        producer: &producer,
        attempt,
        consumed: false,
    };
    publish_compiler_module_handoff_v1(transactions, &producer, attempt, bytes)
        .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::Handoff, source))?;
    let consumed = consume_compiler_module_handoff_v1(transactions, &producer, attempt)
        .map_err(|source| family_error(family, M1KernelArtifactBuildStageV1::Handoff, source))?;
    active.consumed = true;
    Ok(consumed)
}

struct ActiveHandoffAttempt<'a> {
    transactions: &'a Path,
    producer: &'a ProducerIdentity,
    attempt: BuildAttempt,
    consumed: bool,
}

impl Drop for ActiveHandoffAttempt<'_> {
    fn drop(&mut self) {
        if !self.consumed {
            let _ = fail_build_attempt(self.transactions, self.producer, self.attempt);
        }
    }
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

fn create_directory(
    path: &Path,
    operation: &'static str,
) -> Result<(), M1KernelArtifactBuildErrorV1> {
    fs::create_dir(path).map_err(|source| M1KernelArtifactBuildErrorV1::Io {
        operation,
        path: path.to_path_buf(),
        source,
    })
}

fn remove_directory(
    path: &Path,
    operation: &'static str,
) -> Result<(), M1KernelArtifactBuildErrorV1> {
    fs::remove_dir_all(path).map_err(|source| M1KernelArtifactBuildErrorV1::Io {
        operation,
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

    #[test]
    fn rejected_worker_removes_staging_without_publishing() {
        let root = TestDirectory::new("worker-rejection");
        let output = root.0.join("artifacts");
        let missing_worker = root.0.join("missing-worker");
        assert!(matches!(
            build_and_publish_m1_kernel_artifacts_v1(&missing_worker, &output),
            Err(M1KernelArtifactBuildErrorV1::Worker(_))
        ));
        assert!(!output.exists());
        let remaining: Vec<_> = fs::read_dir(&root.0).unwrap().collect();
        assert!(remaining.is_empty());
    }

    #[test]
    #[ignore = "requires the exact reviewed native Worker and measured ROCm OCML closure"]
    fn configured_worker_builds_complete_live_owner() {
        let worker = std::env::var_os("FERRIC_M1_KERNEL_WORKER")
            .expect("set FERRIC_M1_KERNEL_WORKER to the exact reviewed Worker");
        let root = TestDirectory::new("configured-worker");
        let output = root.0.join("artifacts");
        let built = build_and_publish_m1_kernel_artifacts_v1(worker, &output).unwrap();
        assert_eq!(built.manifest().entries().len(), 7);
        assert!(output
            .join(M1_KERNEL_ARTIFACT_MANIFEST_FILENAME_V1)
            .is_file());
        assert!(!built.has_durable_reopen_authority());
        assert!(!built.has_independent_deployment_pin());
        assert!(!built.proves_hardware_execution());
        assert!(built.publication_status().parent_directory_synced());
    }
}
