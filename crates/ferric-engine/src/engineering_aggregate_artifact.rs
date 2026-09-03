//! Non-authoritative admission of one fe2o3 engineering aggregate observation.
//!
//! This feature-gated path deliberately stops at an inert, lexically borrowed
//! structural program catalog. It cannot construct authenticated Worker V3
//! custody, select a current publication, load an executable, or publish a
//! queue. The private manifest decoder mirrors the current canonical
//! `EngineeringHsacoObservationV1` JSON schema. Any schema or canonical field
//! order change fails closed until Ferric audits and updates this decoder.

use std::error::Error;
use std::ffi::OsStr;
use std::fmt;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use fe2o3_amdhsa_loader::{AdmittedProfile, LoadPlan, PlanError};
use fe2o3_host::CompilerGeneratedKernelExpectationRosterV1;
use ferric_qwen3_all_kernels_device_v1::M1AllKernelsWorkerV3RosterV1;
use ferric_spec::Identity;
use rustix::fd::OwnedFd;
use rustix::fs::{fstat, openat2, Dir, FileType, Mode, OFlags, ResolveFlags, CWD};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::physical_program_catalog::{
    bind_content_bound_m1_program_catalog_from_engineering_aggregate_v1,
    ContentBoundM1ProgramCatalogV1, M1PhysicalProgramCatalogErrorV1,
    M1PhysicalProgramSourceContractV1, M1_PHYSICAL_PROGRAM_COUNT_V1,
};

/// Exact manifest schema emitted by `cargo fe2o3 engineering hsaco`.
///
/// The private decoder is frozen against fe2o3 commit
/// `5099cf38c7bee0aa513a8cf9d5ce4efb56a0ffa8` and rejects canonical-shape drift.
pub const M1_ENGINEERING_AGGREGATE_OBSERVATION_SCHEMA_V1: &str = "EngineeringHsacoObservationV1";
/// Exact manifest filename in one fe2o3 engineering observation directory.
pub const M1_ENGINEERING_AGGREGATE_MANIFEST_FILENAME_V1: &str = "observation.json";
/// Exact HSACO filename in one fe2o3 engineering observation directory.
pub const M1_ENGINEERING_AGGREGATE_ARTIFACT_FILENAME_V1: &str = "observation.hsaco";

const ENGINEERING_NAMESPACE_V1: &str = "fe2o3-engineering-v1";
const ENGINEERING_AUTHORITY_V1: &str = "none";
const AGGREGATE_CRATE_NAME_V1: &str = "ferric_qwen3_all_kernels_device_v1";
const GFX942_XNACK_MINUS: &str = "gfx942:xnack-";
const CONTENT_ID_DOMAIN_V1: &[u8] = b"FE2O3/ENGINEERING-HSACO-OBSERVATION-CONTENT/V1\0";
const MAX_MANIFEST_BYTES_V1: usize = 1024 * 1024;
const MAX_HSACO_BYTES_V1: usize = 64 * 1024 * 1024;
const MAX_HANDOFF_BYTES_V1: u64 = 64 * 1024 * 1024;
const MAX_TOOL_BYTES_V1: u64 = 1024 * 1024 * 1024;
const MAX_WORKER_BYTES_V1: u64 = 512 * 1024 * 1024;
const MAX_WORKER_REQUEST_BYTES_V1: u64 = 64 * 1024 * 1024 + 256 * 1024;
const MAX_WORKER_RESPONSE_BYTES_V1: u64 = 64 * 1024 * 1024 + 16 * 1024 + 2 * 1024 * 1024;
const MAX_PROVIDER_BYTES_V1: u64 = 64 * 1024 * 1024;
const MAX_EXTERNAL_PROVIDERS_V1: usize = 127;
const MAX_CARGO_GIT_SOURCES_V1: usize = 64;
const MAX_CARGO_GIT_SOURCE_URL_BYTES_V1: usize = 1024;
const MAX_TOOLCHAIN_ID_BYTES_V1: usize = 160;

/// Canonical file or directory involved in engineering observation admission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1EngineeringAggregateArtifactFileV1 {
    /// Caller-supplied observation content directory.
    RootDirectory,
    /// Canonical `observation.json`.
    Manifest,
    /// Canonical `observation.hsaco`.
    Hsaco,
}

/// Failure while admitting a non-authoritative aggregate engineering observation.
#[derive(Debug)]
#[non_exhaustive]
pub enum M1EngineeringAggregateArtifactOpenErrorV1 {
    /// The host kernel cannot enforce the required no-follow resolution policy.
    StrictNoFollowUnavailable(io::Error),
    /// A descriptor-relative filesystem operation failed.
    Io {
        /// Subject of the operation.
        file: M1EngineeringAggregateArtifactFileV1,
        /// Operating-system failure.
        source: io::Error,
    },
    /// The content directory does not contain exactly the two canonical files.
    DirectoryRoster,
    /// An expected regular file had another filesystem type.
    NotRegularFile(M1EngineeringAggregateArtifactFileV1),
    /// A file violated its exact bounded size.
    InvalidSize(M1EngineeringAggregateArtifactFileV1),
    /// A bounded read buffer could not be reserved.
    ReadBufferAllocation(M1EngineeringAggregateArtifactFileV1),
    /// Descriptor metadata changed during a bounded read.
    ChangedWhileReading(M1EngineeringAggregateArtifactFileV1),
    /// The manifest is not valid JSON for the exact current schema.
    ManifestJson(String),
    /// The manifest is not the exact compact canonical encoding with one newline.
    NonCanonicalManifest,
    /// One fixed or bounded manifest field violates the engineering contract.
    ManifestPolicy {
        /// Stable field path naming the rejected policy axis.
        field: &'static str,
    },
    /// The artifact bytes differ from the exact manifest identity.
    HsacoIdentity,
    /// The directory basename does not bind the exact canonical manifest and HSACO bytes.
    ContentDirectoryIdentity,
    /// Independent finalized-descriptor inspection rejected the HSACO.
    FinalizedHsaco(Box<fe2o3_hsaco_finalize::FinalizationError>),
    /// The finalized HSACO target or code-object version is not exact.
    HsacoProfile,
    /// The manifest digest differs from the independently verified canonical digest.
    CanonicalDescriptorDigest,
    /// Manifest and AMDHSA metadata do not name the same exact ordered kernels.
    MetadataKernelRoster,
    /// The canonical descriptor table differs from current Ferric aggregate markers.
    CurrentFerricDescriptorRoster,
    /// The allocation-free generic loader rejected the exact HSACO.
    Loader(PlanError),
    /// Current Ferric physical symbols or dispatch ABIs do not close over the aggregate.
    ProgramCatalog(Box<M1PhysicalProgramCatalogErrorV1>),
}

impl fmt::Display for M1EngineeringAggregateArtifactOpenErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "non-authoritative M1 engineering aggregate rejected: {self:?}"
        )
    }
}

impl Error for M1EngineeringAggregateArtifactOpenErrorV1 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::StrictNoFollowUnavailable(source) | Self::Io { source, .. } => Some(source),
            Self::FinalizedHsaco(source) => Some(source.as_ref()),
            Self::ProgramCatalog(source) => Some(source.as_ref()),
            _ => None,
        }
    }
}

/// Move-only custody of one structurally admitted engineering aggregate.
///
/// The private bytes and load plan can only be lent to a callback as an inert
/// [`ContentBoundM1ProgramCatalogV1`]. No conversion to an authenticated
/// Worker V3 owner exists.
///
/// ```compile_fail
/// use ferric_engine::{
///     M1AuthenticatedWorkerV3ProgramSetV1, M1EngineeringAggregateArtifactV1,
/// };
/// fn cannot_authenticate(
///     value: M1EngineeringAggregateArtifactV1,
/// ) -> M1AuthenticatedWorkerV3ProgramSetV1 {
///     value.into()
/// }
/// ```
///
/// ```compile_fail
/// use ferric_engine::{M1EngineeringAggregateArtifactV1, M1AuthenticatedPhysicalRunnerV1};
/// fn cannot_become_runner(
///     value: M1EngineeringAggregateArtifactV1,
/// ) -> M1AuthenticatedPhysicalRunnerV1 {
///     value.into()
/// }
/// ```
///
/// ```compile_fail
/// use ferric_engine::M1EngineeringAggregateArtifactV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<M1EngineeringAggregateArtifactV1>();
/// ```
#[must_use = "engineering aggregate byte custody must remain scoped"]
pub struct M1EngineeringAggregateArtifactV1 {
    manifest_id: Identity,
    hsaco_id: Identity,
    compiler_handoff_id: Identity,
    compiler_handoff_len: u64,
    canonical_descriptor_id: Identity,
    bytes: Box<[u8]>,
    plan: LoadPlan,
    source: M1PhysicalProgramSourceContractV1,
    program_catalog_id: Identity,
}

impl fmt::Debug for M1EngineeringAggregateArtifactV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1EngineeringAggregateArtifactV1")
            .field("authority", &ENGINEERING_AUTHORITY_V1)
            .field("manifest_id", &self.manifest_id)
            .field("hsaco_id", &self.hsaco_id)
            .field("program_catalog_id", &self.program_catalog_id)
            .finish_non_exhaustive()
    }
}

impl M1EngineeringAggregateArtifactV1 {
    /// SHA-256 identity of the exact canonical observation manifest bytes.
    #[must_use]
    pub const fn manifest_id(&self) -> Identity {
        self.manifest_id
    }

    /// SHA-256 identity of the exact finalized aggregate HSACO bytes.
    #[must_use]
    pub const fn hsaco_id(&self) -> Identity {
        self.hsaco_id
    }

    /// SHA-256 identity of the compiler handoff observed by fe2o3.
    #[must_use]
    pub const fn compiler_handoff_id(&self) -> Identity {
        self.compiler_handoff_id
    }

    /// Exact byte length of the compiler handoff observed by fe2o3.
    #[must_use]
    pub const fn compiler_handoff_len(&self) -> u64 {
        self.compiler_handoff_len
    }

    /// Independently verified canonical whole-HSACO descriptor digest.
    #[must_use]
    pub const fn canonical_descriptor_id(&self) -> Identity {
        self.canonical_descriptor_id
    }

    /// Ferric-domain-separated identity of all twelve current structural programs.
    #[must_use]
    pub const fn program_catalog_id(&self) -> Identity {
        self.program_catalog_id
    }

    /// Revalidates and lends the exact structural program catalog for one lexical use.
    ///
    /// The callback result cannot retain a borrowed envelope. This function
    /// grants no executable allocation, load, queue, publication, or launch
    /// authority.
    ///
    /// # Errors
    ///
    /// Returns a current physical symbol or dispatch-ABI closure failure before
    /// invoking the callback.
    pub fn with_structural_program_catalog_v1<R>(
        &self,
        use_catalog: impl for<'catalog> FnOnce(ContentBoundM1ProgramCatalogV1<'catalog>) -> R,
    ) -> Result<R, M1PhysicalProgramCatalogErrorV1> {
        let catalog = bind_content_bound_m1_program_catalog_from_engineering_aggregate_v1(
            &self.bytes,
            self.plan,
            self.source,
        )?;
        Ok(use_catalog(catalog))
    }

    /// The observation carries the literal fe2o3 authority value `none`.
    #[must_use]
    pub const fn authority(&self) -> &'static str {
        ENGINEERING_AUTHORITY_V1
    }

    /// Engineering observation does not authenticate compiler process origin.
    #[must_use]
    pub const fn authenticates_compiler_origin(&self) -> bool {
        false
    }

    /// Engineering observation grants no Worker V3 publication authority.
    #[must_use]
    pub const fn grants_publication_authority(&self) -> bool {
        false
    }

    /// Engineering observation grants no executable-load authority.
    #[must_use]
    pub const fn grants_load_authority(&self) -> bool {
        false
    }

    /// Engineering observation grants no queue-launch authority.
    #[must_use]
    pub const fn grants_launch_authority(&self) -> bool {
        false
    }
}

/// Strictly reopens one fe2o3 engineering observation content directory.
///
/// The directory must contain exactly `observation.json` and
/// `observation.hsaco`. Admission verifies canonical schema bytes, all fixed
/// non-authority fields, content-directory identity, artifact identity,
/// finalized descriptor digest, gfx942:xnack-/COV6, exact metadata and current
/// Ferric descriptor rosters, the generic load plan, and all current Ferric
/// dispatch ABIs.
///
/// # Errors
///
/// Returns the exact filesystem, schema, identity, finalized-HSACO, loader, or
/// structural-catalog rejection. No partial artifact owner is returned.
pub fn reopen_m1_engineering_aggregate_artifact_v1(
    root: impl AsRef<Path>,
) -> Result<M1EngineeringAggregateArtifactV1, M1EngineeringAggregateArtifactOpenErrorV1> {
    let root_path = root.as_ref();
    if root_path
        .parent()
        .and_then(Path::file_name)
        .and_then(OsStr::to_str)
        != Some(ENGINEERING_NAMESPACE_V1)
    {
        return Err(M1EngineeringAggregateArtifactOpenErrorV1::ContentDirectoryIdentity);
    }
    let root = openat2(
        CWD,
        root_path,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )
    .map_err(|source| io_error(M1EngineeringAggregateArtifactFileV1::RootDirectory, source))?;
    require_exact_directory_roster(&root)?;

    let manifest_bytes = read_regular_file(
        &root,
        M1_ENGINEERING_AGGREGATE_MANIFEST_FILENAME_V1,
        M1EngineeringAggregateArtifactFileV1::Manifest,
        ReadBound::Maximum(MAX_MANIFEST_BYTES_V1),
    )?;
    let (manifest, facts) = decode_manifest(&manifest_bytes)?;
    let hsaco_len = usize::try_from(facts.hsaco.byte_len)
        .ok()
        .filter(|length| *length <= MAX_HSACO_BYTES_V1)
        .ok_or(M1EngineeringAggregateArtifactOpenErrorV1::InvalidSize(
            M1EngineeringAggregateArtifactFileV1::Hsaco,
        ))?;
    let hsaco_bytes = read_regular_file(
        &root,
        M1_ENGINEERING_AGGREGATE_ARTIFACT_FILENAME_V1,
        M1EngineeringAggregateArtifactFileV1::Hsaco,
        ReadBound::Exact(hsaco_len),
    )?;
    require_exact_directory_roster(&root)?;

    if digest(&hsaco_bytes) != facts.hsaco.sha256 {
        return Err(M1EngineeringAggregateArtifactOpenErrorV1::HsacoIdentity);
    }
    let expected_content_id = observation_content_id(&manifest_bytes, &hsaco_bytes);
    if root_path.file_name().and_then(OsStr::to_str) != Some(expected_content_id.as_str()) {
        return Err(M1EngineeringAggregateArtifactOpenErrorV1::ContentDirectoryIdentity);
    }

    let inspection = fe2o3_hsaco_finalize::inspect_finalized(&hsaco_bytes).map_err(|error| {
        M1EngineeringAggregateArtifactOpenErrorV1::FinalizedHsaco(Box::new(error))
    })?;
    validate_inspection(&manifest, &facts, &inspection)?;

    let envelope = fe2o3_amdhsa_loader::validate(&hsaco_bytes, AdmittedProfile::Gfx942XnackOffCov6)
        .map_err(M1EngineeringAggregateArtifactOpenErrorV1::Loader)?;
    let plan = *envelope.plan();
    drop(envelope);

    let source = M1PhysicalProgramSourceContractV1::new(
        facts.compiler_handoff.sha256,
        facts.compiler_handoff.byte_len,
    );
    let catalog = bind_content_bound_m1_program_catalog_from_engineering_aggregate_v1(
        &hsaco_bytes,
        plan,
        source,
    )
    .map_err(|error| M1EngineeringAggregateArtifactOpenErrorV1::ProgramCatalog(Box::new(error)))?;
    let program_catalog_id = catalog.catalog_id();
    drop(catalog);

    Ok(M1EngineeringAggregateArtifactV1 {
        manifest_id: Identity::new(digest(&manifest_bytes)),
        hsaco_id: Identity::new(facts.hsaco.sha256),
        compiler_handoff_id: Identity::new(facts.compiler_handoff.sha256),
        compiler_handoff_len: facts.compiler_handoff.byte_len,
        canonical_descriptor_id: Identity::new(facts.canonical_descriptor_sha256),
        bytes: hsaco_bytes.into_boxed_slice(),
        plan,
        source,
        program_catalog_id,
    })
}

#[derive(Clone, Copy)]
struct DecodedIdentityV1 {
    sha256: [u8; 32],
    byte_len: u64,
}

struct ValidatedManifestFactsV1 {
    compiler_handoff: DecodedIdentityV1,
    hsaco: DecodedIdentityV1,
    canonical_descriptor_sha256: [u8; 32],
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct EngineeringManifestV1 {
    schema: String,
    namespace: String,
    authority: String,
    artifact: String,
    crate_name: String,
    target: String,
    code_object_version: u8,
    compiler_handoff: ManifestIdentityV1,
    tools: ManifestToolsV1,
    providers: Vec<ManifestProviderV1>,
    options: ManifestOptionsV1,
    execution: ManifestExecutionV1,
    hsaco: ManifestHsacoV1,
    grants: ManifestGrantsV1,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ManifestIdentityV1 {
    sha256: String,
    byte_len: u64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ManifestToolsV1 {
    cargo: ManifestIdentityV1,
    rustc: ManifestIdentityV1,
    rustc_lib_tree_sha256: String,
    host_linker: ManifestIdentityV1,
    host_lld: ManifestIdentityV1,
    host_lld_proxy: ManifestIdentityV1,
    cargo_vendor: Option<ManifestCargoVendorV1>,
    extractor: ManifestIdentityV1,
    extractor_backend: ManifestIdentityV1,
    worker: ManifestWorkerV1,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ManifestCargoVendorV1 {
    tree_sha256: String,
    git_sources: Vec<ManifestCargoGitSourceV1>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ManifestCargoGitSourceV1 {
    url: String,
    rev: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ManifestWorkerV1 {
    executable: ManifestIdentityV1,
    worker_build_identity: String,
    llvm_build_identity: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ManifestProviderV1 {
    kind: String,
    identity: ManifestIdentityV1,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ManifestOptionsV1 {
    optimization: String,
    strip_debug: bool,
    verify_each: bool,
    timeout_seconds: u64,
    maximum_output_bytes: u64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ManifestExecutionV1 {
    bootstrap_request: ManifestIdentityV1,
    bootstrap_response: ManifestIdentityV1,
    replay_request: ManifestIdentityV1,
    replay_response: ManifestIdentityV1,
    exact_output_replay: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ManifestHsacoV1 {
    identity: ManifestIdentityV1,
    canonical_descriptor_sha256: String,
    kernel_names: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ManifestGrantsV1 {
    publication: bool,
    load: bool,
    launch: bool,
}

fn decode_manifest(
    bytes: &[u8],
) -> Result<
    (EngineeringManifestV1, ValidatedManifestFactsV1),
    M1EngineeringAggregateArtifactOpenErrorV1,
> {
    if bytes.is_empty() || bytes.len() > MAX_MANIFEST_BYTES_V1 {
        return Err(M1EngineeringAggregateArtifactOpenErrorV1::InvalidSize(
            M1EngineeringAggregateArtifactFileV1::Manifest,
        ));
    }
    let manifest: EngineeringManifestV1 = serde_json::from_slice(bytes).map_err(|error| {
        M1EngineeringAggregateArtifactOpenErrorV1::ManifestJson(error.to_string())
    })?;
    let mut canonical = serde_json::to_vec(&manifest).map_err(|error| {
        M1EngineeringAggregateArtifactOpenErrorV1::ManifestJson(error.to_string())
    })?;
    canonical.push(b'\n');
    if canonical != bytes {
        return Err(M1EngineeringAggregateArtifactOpenErrorV1::NonCanonicalManifest);
    }
    let facts = validate_manifest(&manifest)?;
    Ok((manifest, facts))
}

fn validate_manifest(
    manifest: &EngineeringManifestV1,
) -> Result<ValidatedManifestFactsV1, M1EngineeringAggregateArtifactOpenErrorV1> {
    require(
        manifest.schema == M1_ENGINEERING_AGGREGATE_OBSERVATION_SCHEMA_V1,
        "schema",
    )?;
    require(manifest.namespace == ENGINEERING_NAMESPACE_V1, "namespace")?;
    require(manifest.authority == ENGINEERING_AUTHORITY_V1, "authority")?;
    require(
        manifest.artifact == M1_ENGINEERING_AGGREGATE_ARTIFACT_FILENAME_V1,
        "artifact",
    )?;
    require(manifest.crate_name == AGGREGATE_CRATE_NAME_V1, "crate_name")?;
    require(manifest.target == GFX942_XNACK_MINUS, "target")?;
    require(manifest.code_object_version == 6, "code_object_version")?;

    let compiler_handoff = decode_identity(
        &manifest.compiler_handoff,
        MAX_HANDOFF_BYTES_V1,
        "compiler_handoff",
    )?;
    validate_tool_identities(&manifest.tools)?;
    validate_providers(&manifest.providers)?;
    validate_options(&manifest.options)?;
    validate_execution(&manifest.execution)?;
    require(
        !manifest.grants.publication && !manifest.grants.load && !manifest.grants.launch,
        "grants",
    )?;

    let hsaco = decode_identity(
        &manifest.hsaco.identity,
        MAX_HSACO_BYTES_V1 as u64,
        "hsaco.identity",
    )?;
    require(
        manifest.options.maximum_output_bytes >= hsaco.byte_len,
        "options.maximum_output_bytes",
    )?;
    let canonical_descriptor_sha256 = decode_sha256(
        &manifest.hsaco.canonical_descriptor_sha256,
        "hsaco.canonical_descriptor_sha256",
    )?;
    validate_manifest_kernel_roster(&manifest.hsaco.kernel_names)?;

    Ok(ValidatedManifestFactsV1 {
        compiler_handoff,
        hsaco,
        canonical_descriptor_sha256,
    })
}

fn validate_tool_identities(
    tools: &ManifestToolsV1,
) -> Result<(), M1EngineeringAggregateArtifactOpenErrorV1> {
    decode_identity(&tools.cargo, MAX_TOOL_BYTES_V1, "tools.cargo")?;
    decode_identity(&tools.rustc, MAX_TOOL_BYTES_V1, "tools.rustc")?;
    decode_sha256(&tools.rustc_lib_tree_sha256, "tools.rustc_lib_tree_sha256")?;
    decode_identity(&tools.host_linker, MAX_TOOL_BYTES_V1, "tools.host_linker")?;
    decode_identity(&tools.host_lld, MAX_TOOL_BYTES_V1, "tools.host_lld")?;
    decode_identity(
        &tools.host_lld_proxy,
        MAX_TOOL_BYTES_V1,
        "tools.host_lld_proxy",
    )?;
    if let Some(vendor) = &tools.cargo_vendor {
        validate_cargo_vendor(vendor)?;
    }
    decode_identity(&tools.extractor, MAX_TOOL_BYTES_V1, "tools.extractor")?;
    decode_identity(
        &tools.extractor_backend,
        MAX_TOOL_BYTES_V1,
        "tools.extractor_backend",
    )?;
    decode_identity(
        &tools.worker.executable,
        MAX_WORKER_BYTES_V1,
        "tools.worker.executable",
    )?;
    validate_toolchain_id(
        &tools.worker.worker_build_identity,
        "tools.worker.worker_build_identity",
    )?;
    validate_toolchain_id(
        &tools.worker.llvm_build_identity,
        "tools.worker.llvm_build_identity",
    )
}

fn validate_cargo_vendor(
    vendor: &ManifestCargoVendorV1,
) -> Result<(), M1EngineeringAggregateArtifactOpenErrorV1> {
    decode_sha256(&vendor.tree_sha256, "tools.cargo_vendor.tree_sha256")?;
    require(
        vendor.git_sources.len() <= MAX_CARGO_GIT_SOURCES_V1,
        "tools.cargo_vendor.git_sources",
    )?;
    for source in &vendor.git_sources {
        require(
            source.url.starts_with("https://")
                && source.url.len() <= MAX_CARGO_GIT_SOURCE_URL_BYTES_V1
                && !source.url.bytes().any(|byte| {
                    byte.is_ascii_control() || matches!(byte, b'"' | b'\\' | b'?' | b'#' | b'@')
                }),
            "tools.cargo_vendor.git_sources.url",
        )?;
        require(
            source.rev.len() == 40
                && source
                    .rev
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
            "tools.cargo_vendor.git_sources.rev",
        )?;
    }
    require(
        vendor
            .git_sources
            .windows(2)
            .all(|pair| (&pair[0].url, &pair[0].rev) < (&pair[1].url, &pair[1].rev)),
        "tools.cargo_vendor.git_sources",
    )
}

fn validate_providers(
    providers: &[ManifestProviderV1],
) -> Result<(), M1EngineeringAggregateArtifactOpenErrorV1> {
    require(providers.len() <= MAX_EXTERNAL_PROVIDERS_V1, "providers")?;
    let mut previous: Option<([u8; 32], u64, u8)> = None;
    let mut total = 0_u64;
    for provider in providers {
        let kind = match provider.kind.as_str() {
            "llvm-bitcode" => 1,
            "amdgpu-relocatable" => 2,
            "llvm-ir" => 3,
            _ => return policy("providers.kind"),
        };
        let identity = decode_identity(
            &provider.identity,
            MAX_PROVIDER_BYTES_V1,
            "providers.identity",
        )?;
        total = total
            .checked_add(identity.byte_len)
            .ok_or_else(|| policy_error("providers.identity.byte_len"))?;
        require(
            total <= MAX_PROVIDER_BYTES_V1,
            "providers.identity.byte_len",
        )?;
        let current = (identity.sha256, identity.byte_len, kind);
        if previous
            .is_some_and(|prior| (prior.0, prior.1) == (current.0, current.1) || prior >= current)
        {
            return policy("providers");
        }
        previous = Some(current);
    }
    Ok(())
}

fn validate_options(
    options: &ManifestOptionsV1,
) -> Result<(), M1EngineeringAggregateArtifactOpenErrorV1> {
    require(options.optimization == "O2", "options.optimization")?;
    require(options.strip_debug, "options.strip_debug")?;
    require(options.verify_each, "options.verify_each")?;
    require(
        (1..=600).contains(&options.timeout_seconds),
        "options.timeout_seconds",
    )?;
    require(
        (1..=MAX_HSACO_BYTES_V1 as u64).contains(&options.maximum_output_bytes),
        "options.maximum_output_bytes",
    )
}

fn validate_execution(
    execution: &ManifestExecutionV1,
) -> Result<(), M1EngineeringAggregateArtifactOpenErrorV1> {
    decode_identity(
        &execution.bootstrap_request,
        MAX_WORKER_REQUEST_BYTES_V1,
        "execution.bootstrap_request",
    )?;
    decode_identity(
        &execution.bootstrap_response,
        MAX_WORKER_RESPONSE_BYTES_V1,
        "execution.bootstrap_response",
    )?;
    decode_identity(
        &execution.replay_request,
        MAX_WORKER_REQUEST_BYTES_V1,
        "execution.replay_request",
    )?;
    decode_identity(
        &execution.replay_response,
        MAX_WORKER_RESPONSE_BYTES_V1,
        "execution.replay_response",
    )?;
    require(
        execution.exact_output_replay,
        "execution.exact_output_replay",
    )
}

fn validate_manifest_kernel_roster(
    kernel_names: &[String],
) -> Result<(), M1EngineeringAggregateArtifactOpenErrorV1> {
    let expected = M1AllKernelsWorkerV3RosterV1::ENTRIES;
    if kernel_names.len() != M1_PHYSICAL_PROGRAM_COUNT_V1 || expected.len() != kernel_names.len() {
        return policy("hsaco.kernel_names");
    }
    let mut actual = kernel_names.iter().map(String::as_str).collect::<Vec<_>>();
    actual.sort_unstable();
    if actual.windows(2).any(|pair| pair[0] == pair[1]) {
        return policy("hsaco.kernel_names");
    }
    let mut expected = expected
        .iter()
        .map(|entry| entry.export_name())
        .collect::<Vec<_>>();
    expected.sort_unstable();
    require(actual == expected, "hsaco.kernel_names")
}

fn validate_inspection(
    manifest: &EngineeringManifestV1,
    facts: &ValidatedManifestFactsV1,
    inspection: &fe2o3_hsaco_finalize::FinalizedDescriptorInspection,
) -> Result<(), M1EngineeringAggregateArtifactOpenErrorV1> {
    if inspection.hsaco().target().to_string() != GFX942_XNACK_MINUS
        || inspection.hsaco().code_object_version().number() != 6
        || inspection.descriptor_table().device_target().to_string() != GFX942_XNACK_MINUS
        || inspection.descriptor_table().code_object_version().number() != 6
    {
        return Err(M1EngineeringAggregateArtifactOpenErrorV1::HsacoProfile);
    }
    if inspection.digest().as_bytes() != &facts.canonical_descriptor_sha256 {
        return Err(M1EngineeringAggregateArtifactOpenErrorV1::CanonicalDescriptorDigest);
    }
    if !inspection
        .hsaco()
        .kernels()
        .iter()
        .map(|kernel| kernel.name())
        .eq(manifest.hsaco.kernel_names.iter().map(String::as_str))
    {
        return Err(M1EngineeringAggregateArtifactOpenErrorV1::MetadataKernelRoster);
    }

    let descriptors = inspection.descriptor_table().kernels();
    let expected = M1AllKernelsWorkerV3RosterV1::ENTRIES;
    if descriptors.len() != M1_PHYSICAL_PROGRAM_COUNT_V1
        || descriptors.len() != expected.len()
        || descriptors
            .iter()
            .zip(expected)
            .any(|(descriptor, expected)| {
                *descriptor.kernel_id().as_bytes() != expected.kernel_binding_id()
                    || descriptor.logical_name().as_str() != expected.logical_name()
                    || descriptor.entry_name().as_str() != expected.export_name()
                    || !descriptor_symbol_matches(
                        descriptor.descriptor_symbol().as_str(),
                        expected.export_name(),
                    )
            })
    {
        return Err(M1EngineeringAggregateArtifactOpenErrorV1::CurrentFerricDescriptorRoster);
    }
    Ok(())
}

fn descriptor_symbol_matches(actual: &str, entry: &str) -> bool {
    actual.len() == entry.len() + ".kd".len() && actual.strip_suffix(".kd") == Some(entry)
}

fn decode_identity(
    identity: &ManifestIdentityV1,
    maximum: u64,
    field: &'static str,
) -> Result<DecodedIdentityV1, M1EngineeringAggregateArtifactOpenErrorV1> {
    require((1..=maximum).contains(&identity.byte_len), field)?;
    Ok(DecodedIdentityV1 {
        sha256: decode_sha256(&identity.sha256, field)?,
        byte_len: identity.byte_len,
    })
}

fn decode_sha256(
    value: &str,
    field: &'static str,
) -> Result<[u8; 32], M1EngineeringAggregateArtifactOpenErrorV1> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return policy(field);
    }
    let mut digest = [0_u8; 32];
    for (index, output) in digest.iter_mut().enumerate() {
        *output = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|_| policy_error(field))?;
    }
    Ok(digest)
}

fn validate_toolchain_id(
    value: &str,
    field: &'static str,
) -> Result<(), M1EngineeringAggregateArtifactOpenErrorV1> {
    require(
        !value.is_empty()
            && value.len() <= MAX_TOOLCHAIN_ID_BYTES_V1
            && value.is_ascii()
            && !value.bytes().any(|byte| byte.is_ascii_control()),
        field,
    )
}

fn require(
    condition: bool,
    field: &'static str,
) -> Result<(), M1EngineeringAggregateArtifactOpenErrorV1> {
    if condition {
        Ok(())
    } else {
        policy(field)
    }
}

fn policy<T>(field: &'static str) -> Result<T, M1EngineeringAggregateArtifactOpenErrorV1> {
    Err(policy_error(field))
}

const fn policy_error(field: &'static str) -> M1EngineeringAggregateArtifactOpenErrorV1 {
    M1EngineeringAggregateArtifactOpenErrorV1::ManifestPolicy { field }
}

#[derive(Clone, Copy)]
enum ReadBound {
    Maximum(usize),
    Exact(usize),
}

impl ReadBound {
    const fn limit(self) -> usize {
        match self {
            Self::Maximum(limit) | Self::Exact(limit) => limit,
        }
    }

    const fn accepts(self, length: usize) -> bool {
        match self {
            Self::Maximum(limit) => length != 0 && length <= limit,
            Self::Exact(expected) => length == expected,
        }
    }
}

fn require_exact_directory_roster(
    root: &OwnedFd,
) -> Result<(), M1EngineeringAggregateArtifactOpenErrorV1> {
    let scan = openat2(
        root,
        ".",
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )
    .map_err(|source| io_error(M1EngineeringAggregateArtifactFileV1::RootDirectory, source))?;
    let mut entries = Dir::read_from(&scan)
        .map_err(|source| io_error(M1EngineeringAggregateArtifactFileV1::RootDirectory, source))?;
    let mut manifest = false;
    let mut hsaco = false;
    for entry in &mut entries {
        let entry = entry.map_err(|source| {
            io_error(M1EngineeringAggregateArtifactFileV1::RootDirectory, source)
        })?;
        match entry.file_name().to_bytes() {
            b"." | b".." => {}
            name if name == M1_ENGINEERING_AGGREGATE_MANIFEST_FILENAME_V1.as_bytes() => {
                if manifest {
                    return Err(M1EngineeringAggregateArtifactOpenErrorV1::DirectoryRoster);
                }
                manifest = true;
            }
            name if name == M1_ENGINEERING_AGGREGATE_ARTIFACT_FILENAME_V1.as_bytes() => {
                if hsaco {
                    return Err(M1EngineeringAggregateArtifactOpenErrorV1::DirectoryRoster);
                }
                hsaco = true;
            }
            _ => return Err(M1EngineeringAggregateArtifactOpenErrorV1::DirectoryRoster),
        }
    }
    if manifest && hsaco {
        Ok(())
    } else {
        Err(M1EngineeringAggregateArtifactOpenErrorV1::DirectoryRoster)
    }
}

fn read_regular_file(
    root: &OwnedFd,
    name: &str,
    subject: M1EngineeringAggregateArtifactFileV1,
    bound: ReadBound,
) -> Result<Vec<u8>, M1EngineeringAggregateArtifactOpenErrorV1> {
    let descriptor = openat2(
        root,
        name,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )
    .map_err(|source| io_error(subject, source))?;
    let initial = fstat(&descriptor).map_err(|source| io_error(subject, source))?;
    if FileType::from_raw_mode(initial.st_mode) != FileType::RegularFile {
        return Err(M1EngineeringAggregateArtifactOpenErrorV1::NotRegularFile(
            subject,
        ));
    }
    let initial_len = usize::try_from(initial.st_size)
        .ok()
        .filter(|length| bound.accepts(*length))
        .ok_or(M1EngineeringAggregateArtifactOpenErrorV1::InvalidSize(
            subject,
        ))?;
    let mut file = File::from(descriptor);
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(initial_len.saturating_add(1))
        .map_err(|_| M1EngineeringAggregateArtifactOpenErrorV1::ReadBufferAllocation(subject))?;
    Read::by_ref(&mut file)
        .take(bound.limit().saturating_add(1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|source| M1EngineeringAggregateArtifactOpenErrorV1::Io {
            file: subject,
            source,
        })?;
    let final_stat = fstat(&file).map_err(|source| io_error(subject, source))?;
    if !same_file_snapshot(&initial, &final_stat) {
        return Err(M1EngineeringAggregateArtifactOpenErrorV1::ChangedWhileReading(subject));
    }
    if bytes.len() != initial_len || !bound.accepts(bytes.len()) {
        return Err(M1EngineeringAggregateArtifactOpenErrorV1::InvalidSize(
            subject,
        ));
    }
    Ok(bytes)
}

fn same_file_snapshot(initial: &rustix::fs::Stat, final_stat: &rustix::fs::Stat) -> bool {
    initial.st_dev == final_stat.st_dev
        && initial.st_ino == final_stat.st_ino
        && initial.st_mode == final_stat.st_mode
        && initial.st_nlink == final_stat.st_nlink
        && initial.st_size == final_stat.st_size
        && initial.st_mtime == final_stat.st_mtime
        && initial.st_mtime_nsec == final_stat.st_mtime_nsec
        && initial.st_ctime == final_stat.st_ctime
        && initial.st_ctime_nsec == final_stat.st_ctime_nsec
}

fn io_error(
    file: M1EngineeringAggregateArtifactFileV1,
    source: rustix::io::Errno,
) -> M1EngineeringAggregateArtifactOpenErrorV1 {
    if matches!(source, rustix::io::Errno::NOSYS | rustix::io::Errno::INVAL) {
        return M1EngineeringAggregateArtifactOpenErrorV1::StrictNoFollowUnavailable(
            io::Error::from(source),
        );
    }
    M1EngineeringAggregateArtifactOpenErrorV1::Io {
        file,
        source: io::Error::from(source),
    }
}

fn observation_content_id(manifest: &[u8], hsaco: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(CONTENT_ID_DOMAIN_V1);
    hasher.update((manifest.len() as u64).to_le_bytes());
    hasher.update(manifest);
    hasher.update((hsaco.len() as u64).to_le_bytes());
    hasher.update(hsaco);
    hex(&hasher.finalize())
}

fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(DIGITS[usize::from(byte >> 4)]));
        output.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    output
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::symlink;
    use std::path::PathBuf;
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
                "ferric-engineering-aggregate-{label}-{}-{nonce}",
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
    fn current_canonical_manifest_schema_decodes_exactly() {
        let manifest = manifest(b"not-a-hsaco");
        let bytes = encode(&manifest);
        let (decoded, facts) = decode_manifest(&bytes).expect("canonical manifest");
        assert_eq!(
            decoded.schema,
            M1_ENGINEERING_AGGREGATE_OBSERVATION_SCHEMA_V1
        );
        assert_eq!(facts.hsaco.sha256, digest(b"not-a-hsaco"));
        assert_eq!(facts.compiler_handoff.byte_len, 1234);

        let mut without_vendor = manifest(b"not-a-hsaco");
        without_vendor.tools.cargo_vendor = None;
        decode_manifest(&encode(&without_vendor)).expect("canonical null cargo_vendor");
    }

    #[test]
    fn unknown_reordered_noncanonical_and_non_ascii_fields_fail_closed() {
        let manifest = manifest(b"not-a-hsaco");
        let canonical = encode(&manifest);
        let mut value: serde_json::Value = serde_json::from_slice(&canonical).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("unexpected".to_owned(), serde_json::Value::Bool(true));
        assert!(matches!(
            decode_manifest(&serde_json::to_vec(&value).unwrap()),
            Err(M1EngineeringAggregateArtifactOpenErrorV1::ManifestJson(_))
        ));

        let mut reordered =
            serde_json::to_vec(&serde_json::from_slice::<serde_json::Value>(&canonical).unwrap())
                .unwrap();
        reordered.push(b'\n');
        assert_ne!(reordered, canonical);
        assert!(matches!(
            decode_manifest(&reordered),
            Err(M1EngineeringAggregateArtifactOpenErrorV1::NonCanonicalManifest)
        ));

        let pretty = serde_json::to_vec_pretty(&manifest).unwrap();
        assert!(matches!(
            decode_manifest(&pretty),
            Err(M1EngineeringAggregateArtifactOpenErrorV1::NonCanonicalManifest)
        ));

        let mut non_ascii = manifest;
        non_ascii.tools.worker.worker_build_identity = "worker-\u{2603}".to_owned();
        assert!(matches!(
            decode_manifest(&encode(&non_ascii)),
            Err(M1EngineeringAggregateArtifactOpenErrorV1::ManifestPolicy {
                field: "tools.worker.worker_build_identity"
            })
        ));

        let nested_unknown = manifest(b"not-a-hsaco");
        let mut nested_value = serde_json::to_value(&nested_unknown).unwrap();
        nested_value["tools"]["cargo_vendor"]
            .as_object_mut()
            .unwrap()
            .insert(
                "path".to_owned(),
                serde_json::Value::String("/tmp".to_owned()),
            );
        assert!(matches!(
            decode_manifest(&serde_json::to_vec(&nested_value).unwrap()),
            Err(M1EngineeringAggregateArtifactOpenErrorV1::ManifestJson(_))
        ));

        let mut missing_vendor = serde_json::to_value(manifest(b"not-a-hsaco")).unwrap();
        missing_vendor["tools"]
            .as_object_mut()
            .unwrap()
            .remove("cargo_vendor");
        let mut missing_vendor = serde_json::to_vec(&missing_vendor).unwrap();
        missing_vendor.push(b'\n');
        assert!(matches!(
            decode_manifest(&missing_vendor),
            Err(M1EngineeringAggregateArtifactOpenErrorV1::NonCanonicalManifest)
        ));
    }

    #[test]
    fn every_authority_profile_and_replay_claim_is_fail_closed() {
        for mutation in 0..14 {
            let mut manifest = manifest(b"not-a-hsaco");
            match mutation {
                0 => manifest.schema.push('2'),
                1 => manifest.namespace.push('2'),
                2 => manifest.authority = "production".to_owned(),
                3 => manifest.artifact = "other.hsaco".to_owned(),
                4 => manifest.crate_name = "other_device".to_owned(),
                5 => manifest.target = "gfx942:xnack+".to_owned(),
                6 => manifest.code_object_version = 5,
                7 => manifest.options.optimization = "O3".to_owned(),
                8 => manifest.options.strip_debug = false,
                9 => manifest.options.verify_each = false,
                10 => manifest.execution.exact_output_replay = false,
                11 => manifest.grants.publication = true,
                12 => manifest.grants.load = true,
                13 => manifest.grants.launch = true,
                _ => unreachable!(),
            }
            assert!(matches!(
                decode_manifest(&encode(&manifest)),
                Err(M1EngineeringAggregateArtifactOpenErrorV1::ManifestPolicy { .. })
            ));
        }
    }

    #[test]
    fn identity_bounds_and_lowercase_digests_are_exact() {
        let mut uppercase = manifest(b"not-a-hsaco");
        uppercase.hsaco.identity.sha256.replace_range(..1, "A");
        assert!(matches!(
            decode_manifest(&encode(&uppercase)),
            Err(M1EngineeringAggregateArtifactOpenErrorV1::ManifestPolicy {
                field: "hsaco.identity"
            })
        ));

        let mut zero_length = manifest(b"not-a-hsaco");
        zero_length.compiler_handoff.byte_len = 0;
        assert!(matches!(
            decode_manifest(&encode(&zero_length)),
            Err(M1EngineeringAggregateArtifactOpenErrorV1::ManifestPolicy {
                field: "compiler_handoff"
            })
        ));

        let mut undersized_bound = manifest(b"not-a-hsaco");
        undersized_bound.options.maximum_output_bytes = 1;
        assert!(matches!(
            decode_manifest(&encode(&undersized_bound)),
            Err(M1EngineeringAggregateArtifactOpenErrorV1::ManifestPolicy {
                field: "options.maximum_output_bytes"
            })
        ));

        for select in 0..7 {
            let mut oversized_tool = manifest(b"not-a-hsaco");
            let selected = match select {
                0 => &mut oversized_tool.tools.cargo,
                1 => &mut oversized_tool.tools.rustc,
                2 => &mut oversized_tool.tools.host_linker,
                3 => &mut oversized_tool.tools.host_lld,
                4 => &mut oversized_tool.tools.host_lld_proxy,
                5 => &mut oversized_tool.tools.extractor,
                6 => &mut oversized_tool.tools.extractor_backend,
                _ => unreachable!(),
            };
            selected.byte_len = MAX_TOOL_BYTES_V1 + 1;
            assert!(matches!(
                decode_manifest(&encode(&oversized_tool)),
                Err(M1EngineeringAggregateArtifactOpenErrorV1::ManifestPolicy { .. })
            ));
        }

        let mut oversized_worker = manifest(b"not-a-hsaco");
        oversized_worker.tools.worker.executable.byte_len = MAX_WORKER_BYTES_V1 + 1;
        assert!(matches!(
            decode_manifest(&encode(&oversized_worker)),
            Err(M1EngineeringAggregateArtifactOpenErrorV1::ManifestPolicy {
                field: "tools.worker.executable"
            })
        ));

        for select in 0..4 {
            let mut oversized_protocol = manifest(b"not-a-hsaco");
            let (selected, maximum) = match select {
                0 => (
                    &mut oversized_protocol.execution.bootstrap_request,
                    MAX_WORKER_REQUEST_BYTES_V1,
                ),
                1 => (
                    &mut oversized_protocol.execution.bootstrap_response,
                    MAX_WORKER_RESPONSE_BYTES_V1,
                ),
                2 => (
                    &mut oversized_protocol.execution.replay_request,
                    MAX_WORKER_REQUEST_BYTES_V1,
                ),
                3 => (
                    &mut oversized_protocol.execution.replay_response,
                    MAX_WORKER_RESPONSE_BYTES_V1,
                ),
                _ => unreachable!(),
            };
            selected.byte_len = maximum + 1;
            assert!(matches!(
                decode_manifest(&encode(&oversized_protocol)),
                Err(M1EngineeringAggregateArtifactOpenErrorV1::ManifestPolicy { .. })
            ));
        }
    }

    #[test]
    fn cargo_vendor_digest_source_bounds_and_canonical_order_are_exact() {
        let mut invalid_tree = manifest(b"not-a-hsaco");
        invalid_tree
            .tools
            .cargo_vendor
            .as_mut()
            .unwrap()
            .tree_sha256
            .replace_range(..1, "A");
        assert!(matches!(
            decode_manifest(&encode(&invalid_tree)),
            Err(M1EngineeringAggregateArtifactOpenErrorV1::ManifestPolicy {
                field: "tools.cargo_vendor.tree_sha256"
            })
        ));

        let mut invalid_url = manifest(b"not-a-hsaco");
        invalid_url.tools.cargo_vendor.as_mut().unwrap().git_sources[0].url =
            "ssh://github.com/example/alpha.git".to_owned();
        assert!(matches!(
            decode_manifest(&encode(&invalid_url)),
            Err(M1EngineeringAggregateArtifactOpenErrorV1::ManifestPolicy {
                field: "tools.cargo_vendor.git_sources.url"
            })
        ));

        let mut invalid_rev = manifest(b"not-a-hsaco");
        invalid_rev.tools.cargo_vendor.as_mut().unwrap().git_sources[0].rev = "A".repeat(40);
        assert!(matches!(
            decode_manifest(&encode(&invalid_rev)),
            Err(M1EngineeringAggregateArtifactOpenErrorV1::ManifestPolicy {
                field: "tools.cargo_vendor.git_sources.rev"
            })
        ));

        for duplicate in [false, true] {
            let mut unordered = manifest(b"not-a-hsaco");
            let sources = &mut unordered.tools.cargo_vendor.as_mut().unwrap().git_sources;
            if duplicate {
                let url = sources[0].url.clone();
                let rev = sources[0].rev.clone();
                sources[1].url = url;
                sources[1].rev = rev;
            } else {
                sources.swap(0, 1);
            }
            assert!(matches!(
                decode_manifest(&encode(&unordered)),
                Err(M1EngineeringAggregateArtifactOpenErrorV1::ManifestPolicy {
                    field: "tools.cargo_vendor.git_sources"
                })
            ));
        }

        let mut too_many = manifest(b"not-a-hsaco");
        too_many.tools.cargo_vendor.as_mut().unwrap().git_sources = (0..=MAX_CARGO_GIT_SOURCES_V1)
            .map(|index| ManifestCargoGitSourceV1 {
                url: format!("https://example.com/{index:03}"),
                rev: format!("{index:040x}"),
            })
            .collect();
        assert!(matches!(
            decode_manifest(&encode(&too_many)),
            Err(M1EngineeringAggregateArtifactOpenErrorV1::ManifestPolicy {
                field: "tools.cargo_vendor.git_sources"
            })
        ));
    }

    #[test]
    fn missing_extra_duplicate_and_substituted_kernel_names_fail_closed() {
        for mutation in 0..4 {
            let mut manifest = manifest(b"not-a-hsaco");
            match mutation {
                0 => {
                    manifest.hsaco.kernel_names.pop();
                }
                1 => manifest.hsaco.kernel_names.push("extra".to_owned()),
                2 => manifest.hsaco.kernel_names[1] = manifest.hsaco.kernel_names[0].clone(),
                3 => manifest.hsaco.kernel_names[0] = "substituted".to_owned(),
                _ => unreachable!(),
            }
            assert!(matches!(
                decode_manifest(&encode(&manifest)),
                Err(M1EngineeringAggregateArtifactOpenErrorV1::ManifestPolicy {
                    field: "hsaco.kernel_names"
                })
            ));
        }
    }

    #[test]
    fn provider_kinds_duplicates_order_and_total_are_exact() {
        let mut unknown = manifest(b"not-a-hsaco");
        unknown.providers[0].kind = "native-object".to_owned();
        assert!(matches!(
            decode_manifest(&encode(&unknown)),
            Err(M1EngineeringAggregateArtifactOpenErrorV1::ManifestPolicy {
                field: "providers.kind"
            })
        ));

        let mut duplicate = manifest(b"not-a-hsaco");
        duplicate.providers.push(ManifestProviderV1 {
            kind: "llvm-ir".to_owned(),
            identity: identity(0x20, 200),
        });
        assert!(matches!(
            decode_manifest(&encode(&duplicate)),
            Err(M1EngineeringAggregateArtifactOpenErrorV1::ManifestPolicy { field: "providers" })
        ));

        let mut reversed = manifest(b"not-a-hsaco");
        reversed.providers.insert(
            0,
            ManifestProviderV1 {
                kind: "llvm-bitcode".to_owned(),
                identity: identity(0x30, 100),
            },
        );
        assert!(matches!(
            decode_manifest(&encode(&reversed)),
            Err(M1EngineeringAggregateArtifactOpenErrorV1::ManifestPolicy { field: "providers" })
        ));

        let mut too_many = manifest(b"not-a-hsaco");
        too_many.providers = (0..=MAX_EXTERNAL_PROVIDERS_V1)
            .map(|index| ManifestProviderV1 {
                kind: "llvm-bitcode".to_owned(),
                identity: ManifestIdentityV1 {
                    sha256: format!("{index:064x}"),
                    byte_len: 1,
                },
            })
            .collect();
        assert!(matches!(
            decode_manifest(&encode(&too_many)),
            Err(M1EngineeringAggregateArtifactOpenErrorV1::ManifestPolicy { field: "providers" })
        ));
    }

    #[test]
    fn artifact_digest_mismatch_precedes_structural_inspection() {
        let outer = TestDirectory::new("digest");
        let declared = b"declared";
        let actual = b"alteredd";
        let manifest = encode(&manifest(declared));
        let namespace = outer.0.join(ENGINEERING_NAMESPACE_V1);
        fs::create_dir(&namespace).unwrap();
        let root = namespace.join(observation_content_id(&manifest, actual));
        fs::create_dir(&root).unwrap();
        fs::write(
            root.join(M1_ENGINEERING_AGGREGATE_MANIFEST_FILENAME_V1),
            manifest,
        )
        .unwrap();
        fs::write(
            root.join(M1_ENGINEERING_AGGREGATE_ARTIFACT_FILENAME_V1),
            actual,
        )
        .unwrap();
        assert!(matches!(
            reopen_m1_engineering_aggregate_artifact_v1(&root),
            Err(M1EngineeringAggregateArtifactOpenErrorV1::HsacoIdentity)
        ));
    }

    #[test]
    fn exact_directory_roster_and_no_follow_policy_reject_substitution() {
        let outer = TestDirectory::new("roster");
        let bytes = b"not-a-hsaco";
        let manifest = encode(&manifest(bytes));
        let namespace = outer.0.join(ENGINEERING_NAMESPACE_V1);
        fs::create_dir(&namespace).unwrap();
        let root = namespace.join(observation_content_id(&manifest, bytes));
        fs::create_dir(&root).unwrap();
        fs::write(
            root.join(M1_ENGINEERING_AGGREGATE_MANIFEST_FILENAME_V1),
            &manifest,
        )
        .unwrap();
        fs::write(
            root.join(M1_ENGINEERING_AGGREGATE_ARTIFACT_FILENAME_V1),
            bytes,
        )
        .unwrap();
        fs::write(root.join("extra"), b"extra").unwrap();
        assert!(matches!(
            reopen_m1_engineering_aggregate_artifact_v1(&root),
            Err(M1EngineeringAggregateArtifactOpenErrorV1::DirectoryRoster)
        ));

        fs::remove_file(root.join("extra")).unwrap();
        let manifest_copy = outer.0.join("manifest-copy");
        fs::write(&manifest_copy, &manifest).unwrap();
        fs::remove_file(root.join(M1_ENGINEERING_AGGREGATE_MANIFEST_FILENAME_V1)).unwrap();
        symlink(
            &manifest_copy,
            root.join(M1_ENGINEERING_AGGREGATE_MANIFEST_FILENAME_V1),
        )
        .unwrap();
        assert!(matches!(
            reopen_m1_engineering_aggregate_artifact_v1(&root),
            Err(M1EngineeringAggregateArtifactOpenErrorV1::Io {
                file: M1EngineeringAggregateArtifactFileV1::Manifest,
                ..
            })
        ));
    }

    #[test]
    fn content_directory_identity_binds_manifest_and_hsaco() {
        assert_eq!(
            observation_content_id(b"manifest", b"hsaco"),
            observation_content_id(b"manifest", b"hsaco")
        );
        assert_ne!(
            observation_content_id(b"manifest", b"hsaco"),
            observation_content_id(b"manifest-2", b"hsaco")
        );
        assert_ne!(
            observation_content_id(b"manifest", b"hsaco"),
            observation_content_id(b"manifest", b"hsaco-2")
        );
    }

    #[test]
    #[ignore = "requires one real qwen3-all-kernels fe2o3 engineering observation"]
    fn configured_real_engineering_observation_admits_lexical_catalog_only() {
        let root = std::env::var_os("FERRIC_M1_ENGINEERING_AGGREGATE_OBSERVATION_DIRECTORY")
            .expect("set FERRIC_M1_ENGINEERING_AGGREGATE_OBSERVATION_DIRECTORY");
        let artifact = reopen_m1_engineering_aggregate_artifact_v1(root).unwrap();
        artifact
            .with_structural_program_catalog_v1(|catalog| {
                assert_eq!(catalog.program_count(), M1_PHYSICAL_PROGRAM_COUNT_V1);
                assert_eq!(catalog.catalog_id(), artifact.program_catalog_id());
                assert!(!catalog.has_independent_deployment_pin());
                assert!(!catalog.proves_hardware_execution());
            })
            .unwrap();
        assert_eq!(artifact.authority(), "none");
        assert!(!artifact.authenticates_compiler_origin());
        assert!(!artifact.grants_publication_authority());
        assert!(!artifact.grants_load_authority());
        assert!(!artifact.grants_launch_authority());
    }

    fn manifest(hsaco: &[u8]) -> EngineeringManifestV1 {
        EngineeringManifestV1 {
            schema: M1_ENGINEERING_AGGREGATE_OBSERVATION_SCHEMA_V1.to_owned(),
            namespace: ENGINEERING_NAMESPACE_V1.to_owned(),
            authority: ENGINEERING_AUTHORITY_V1.to_owned(),
            artifact: M1_ENGINEERING_AGGREGATE_ARTIFACT_FILENAME_V1.to_owned(),
            crate_name: AGGREGATE_CRATE_NAME_V1.to_owned(),
            target: GFX942_XNACK_MINUS.to_owned(),
            code_object_version: 6,
            compiler_handoff: identity(0x10, 1234),
            tools: ManifestToolsV1 {
                cargo: identity(0x11, 101),
                rustc: identity(0x12, 102),
                rustc_lib_tree_sha256: hex(&[0x13; 32]),
                host_linker: identity(0x14, 104),
                host_lld: identity(0x15, 105),
                host_lld_proxy: identity(0x16, 106),
                cargo_vendor: Some(ManifestCargoVendorV1 {
                    tree_sha256: hex(&[0x17; 32]),
                    git_sources: vec![
                        ManifestCargoGitSourceV1 {
                            url: "https://github.com/example/alpha.git".to_owned(),
                            rev: "1".repeat(40),
                        },
                        ManifestCargoGitSourceV1 {
                            url: "https://github.com/example/beta.git".to_owned(),
                            rev: "2".repeat(40),
                        },
                    ],
                }),
                extractor: identity(0x18, 108),
                extractor_backend: identity(0x19, 109),
                worker: ManifestWorkerV1 {
                    executable: identity(0x1a, 110),
                    worker_build_identity: "worker-v1".to_owned(),
                    llvm_build_identity: "llvm-v1".to_owned(),
                },
            },
            providers: vec![ManifestProviderV1 {
                kind: "llvm-bitcode".to_owned(),
                identity: identity(0x20, 200),
            }],
            options: ManifestOptionsV1 {
                optimization: "O2".to_owned(),
                strip_debug: true,
                verify_each: true,
                timeout_seconds: 600,
                maximum_output_bytes: MAX_HSACO_BYTES_V1 as u64,
            },
            execution: ManifestExecutionV1 {
                bootstrap_request: identity(0x31, 301),
                bootstrap_response: identity(0x32, 302),
                replay_request: identity(0x33, 303),
                replay_response: identity(0x34, 304),
                exact_output_replay: true,
            },
            hsaco: ManifestHsacoV1 {
                identity: ManifestIdentityV1 {
                    sha256: hex(&digest(hsaco)),
                    byte_len: hsaco.len() as u64,
                },
                canonical_descriptor_sha256: hex(&[0x40; 32]),
                kernel_names: M1AllKernelsWorkerV3RosterV1::ENTRIES
                    .iter()
                    .map(|entry| entry.export_name().to_owned())
                    .collect(),
            },
            grants: ManifestGrantsV1 {
                publication: false,
                load: false,
                launch: false,
            },
        }
    }

    fn identity(byte: u8, byte_len: u64) -> ManifestIdentityV1 {
        ManifestIdentityV1 {
            sha256: hex(&[byte; 32]),
            byte_len,
        }
    }

    fn encode(manifest: &EngineeringManifestV1) -> Vec<u8> {
        let mut bytes = serde_json::to_vec(manifest).unwrap();
        bytes.push(b'\n');
        bytes
    }
}
