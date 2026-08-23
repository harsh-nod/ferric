//! Durable, structural reopen of one canonical M1 K1-K7 artifact directory.
//!
//! Admission pins every path through no-follow descriptors, revalidates the
//! canonical manifest, measures the seven exact objects, reproduces their
//! allocation-free load plans, and closes all twelve selected symbols. The
//! resulting owner is fresh structural byte custody. It does not reconstruct
//! the original measured Worker owners, independently authorize deployment,
//! allocate or load an executable, publish a queue, or report hardware or
//! numerical evidence.

use std::fmt;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use std::sync::OnceLock;

use fe2o3_amdhsa_loader::{AdmittedProfile, LoadPlan, PlanError};
use ferric_build::{
    current_m1_kernel_source_facts_v1, decode_m1_kernel_artifact_manifest_v1,
    M1CurrentKernelSourceFactsV1, M1KernelArtifactBuildErrorV1, M1KernelArtifactFamilyV1,
    M1KernelArtifactManifestErrorV1, M1KernelArtifactManifestV1,
    M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1, M1_KERNEL_ARTIFACT_MANIFEST_FILENAME_V1,
    M1_KERNEL_ARTIFACT_MANIFEST_MAX_BYTES_V1,
};
use ferric_spec::Identity;
use rustix::fd::OwnedFd;
use rustix::fs::{fstat, openat2, FileType, Mode, OFlags, ResolveFlags, CWD};
use sha2::{Digest, Sha256};

use crate::physical_program_catalog::{
    bind_content_bound_m1_program_catalog_from_persisted_v1, ContentBoundM1ProgramCatalogV1,
    M1PhysicalProgramCatalogErrorV1, M1PhysicalProgramSourceContractV1,
};

/// Canonical file or directory involved in persisted M1 artifact admission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1PersistedKernelArtifactFileV1 {
    /// Caller-supplied artifact root directory.
    RootDirectory,
    /// Canonical manifest file.
    Manifest,
    /// Fixed `objects` directory.
    ObjectsDirectory,
    /// Fixed `objects/sha256` directory.
    Sha256Directory,
    /// Content-addressed object for one exact family.
    Object(M1KernelArtifactFamilyV1),
}

/// Failure while strictly reopening persisted K1-K7 bytes.
#[derive(Debug)]
pub enum M1PersistedKernelArtifactOpenErrorV1 {
    /// The host kernel cannot enforce the required `openat2` resolution policy.
    StrictNoFollowUnavailable(io::Error),
    /// A no-follow descriptor operation failed.
    Io {
        /// Canonical subject of the operation.
        file: M1PersistedKernelArtifactFileV1,
        /// Operating-system failure.
        source: io::Error,
    },
    /// An expected regular file had another filesystem type.
    NotRegularFile(M1PersistedKernelArtifactFileV1),
    /// A file's descriptor-reported length violated its exact bound.
    InvalidSize(M1PersistedKernelArtifactFileV1),
    /// The host could not reserve the exact bounded read buffer.
    ReadBufferAllocation(M1PersistedKernelArtifactFileV1),
    /// Descriptor metadata changed while the bounded read was in progress.
    ChangedWhileReading(M1PersistedKernelArtifactFileV1),
    /// Canonical manifest decoding or semantic revalidation failed.
    Manifest(M1KernelArtifactManifestErrorV1),
    /// One object's measured SHA-256 did not match its manifest identity.
    ObjectIdentity(M1KernelArtifactFamilyV1),
    /// Generic gfx942:xnack-/COV6 validation rejected one object.
    Loader {
        /// Exact rejected family.
        family: M1KernelArtifactFamilyV1,
        /// Allocation-free loader error.
        source: PlanError,
    },
    /// Fresh loader validation did not reproduce the exact persisted plan.
    LoaderPlanDrift(M1KernelArtifactFamilyV1),
    /// Current canonical Ferric source could not reproduce its compiler-input facts.
    CurrentSourceContract(M1KernelArtifactBuildErrorV1),
    /// Manifest source facts did not match current canonical Ferric source.
    CurrentSourceIdentityDrift(M1KernelArtifactFamilyV1),
    /// Exact twelve-symbol structural closure failed.
    ProgramCatalog(M1PhysicalProgramCatalogErrorV1),
}

impl fmt::Display for M1PersistedKernelArtifactOpenErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "persisted M1 kernel artifacts rejected: {self:?}"
        )
    }
}

impl std::error::Error for M1PersistedKernelArtifactOpenErrorV1 {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::StrictNoFollowUnavailable(source) | Self::Io { source, .. } => Some(source),
            Self::Manifest(source) => Some(source),
            Self::CurrentSourceContract(source) => Some(source),
            Self::ProgramCatalog(source) => Some(source),
            Self::NotRegularFile(_)
            | Self::InvalidSize(_)
            | Self::ReadBufferAllocation(_)
            | Self::ChangedWhileReading(_)
            | Self::ObjectIdentity(_)
            | Self::Loader { .. }
            | Self::LoaderPlanDrift(_)
            | Self::CurrentSourceIdentityDrift(_) => None,
        }
    }
}

/// Fresh non-clone custody of seven persisted, structurally admitted objects.
///
/// The bytes and reproduced load plans are private. A borrowed physical
/// catalog can exist only inside [`Self::with_content_bound_program_catalog_v1`].
/// This avoids a self-referential owner and prevents a catalog from outliving
/// the exact byte custody on which its kernel envelopes depend.
///
/// ```compile_fail
/// use ferric_engine::AdmittedPersistedM1KernelArtifactsV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<AdmittedPersistedM1KernelArtifactsV1>();
/// ```
///
/// ```compile_fail
/// use ferric_engine::{
///     AdmittedPersistedM1KernelArtifactsV1, ContentBoundM1ProgramCatalogV1,
/// };
/// fn escape(
///     owner: &AdmittedPersistedM1KernelArtifactsV1,
/// ) -> ContentBoundM1ProgramCatalogV1<'_> {
///     owner
///         .with_content_bound_program_catalog_v1(|catalog| catalog)
///         .unwrap()
/// }
/// ```
///
/// ```compile_fail
/// use ferric_engine::AdmittedPersistedM1KernelArtifactsV1;
/// fn raw_bytes(owner: &AdmittedPersistedM1KernelArtifactsV1) {
///     let _ = &owner.objects;
/// }
/// ```
pub struct AdmittedPersistedM1KernelArtifactsV1 {
    manifest: M1KernelArtifactManifestV1,
    objects: [Box<[u8]>; M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1],
    plans: [LoadPlan; M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1],
    source_contracts: [M1PhysicalProgramSourceContractV1; M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1],
    program_catalog_id: Identity,
}

impl fmt::Debug for AdmittedPersistedM1KernelArtifactsV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AdmittedPersistedM1KernelArtifactsV1")
            .field("manifest_identity", &self.manifest.identity())
            .field("program_catalog_id", &self.program_catalog_id)
            .field("family_count", &M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1)
            .finish_non_exhaustive()
    }
}

impl AdmittedPersistedM1KernelArtifactsV1 {
    pub(crate) fn content_bound_program_catalog_v1(
        &self,
    ) -> Result<ContentBoundM1ProgramCatalogV1<'_>, M1PhysicalProgramCatalogErrorV1> {
        bind_content_bound_m1_program_catalog_from_persisted_v1(
            object_borrows(&self.objects),
            &self.plans,
            &self.source_contracts,
        )
    }

    /// Revalidated canonical inert manifest retained with the exact bytes.
    #[must_use]
    pub const fn manifest(&self) -> &M1KernelArtifactManifestV1 {
        &self.manifest
    }

    /// Freshly derived identity of the ordered twelve-program structural catalog.
    ///
    /// This observation is content binding, not independent deployment approval.
    #[must_use]
    pub const fn program_catalog_id(&self) -> Identity {
        self.program_catalog_id
    }

    /// Builds a borrowed exact program catalog for one lexical use.
    ///
    /// The callback result cannot borrow the catalog, so no selected-kernel
    /// envelope can escape this owner's byte custody.
    ///
    /// # Errors
    ///
    /// Returns [`M1PhysicalProgramCatalogErrorV1`] if fresh validation no longer
    /// reproduces the retained plan or any exact symbol closure.
    pub fn with_content_bound_program_catalog_v1<R>(
        &self,
        use_catalog: impl for<'catalog> FnOnce(ContentBoundM1ProgramCatalogV1<'catalog>) -> R,
    ) -> Result<R, M1PhysicalProgramCatalogErrorV1> {
        let catalog = self.content_bound_program_catalog_v1()?;
        Ok(use_catalog(catalog))
    }

    /// Persisted admission does not reconstruct measured Worker custody.
    #[must_use]
    pub const fn reconstructs_inspected_worker_custody(&self) -> bool {
        false
    }

    /// Persisted structural admission is not independent deployment approval.
    #[must_use]
    pub const fn has_independent_deployment_pin(&self) -> bool {
        false
    }

    /// This pre-load owner proves no executable allocation, mapping, or loading.
    #[must_use]
    pub const fn proves_hsa_executable_load(&self) -> bool {
        false
    }

    /// This pre-load owner reports no hardware execution or numerical evidence.
    #[must_use]
    pub const fn proves_hardware_execution(&self) -> bool {
        false
    }
}

/// Strictly reopens the canonical manifest and its seven content-addressed objects.
///
/// The caller supplies only an artifact directory. Every child name is fixed or
/// derived from a decoded manifest digest; manifest text is never used for path
/// traversal. All descriptors use `O_NOFOLLOW`, all files are bounded regular
/// files, and descriptor metadata is checked before and after each exact read.
///
/// # Errors
///
/// Returns [`M1PersistedKernelArtifactOpenErrorV1`] for any filesystem type,
/// size, concurrent-change, manifest, identity, target/COV6, load-plan, or
/// twelve-symbol mismatch.
pub fn reopen_persisted_m1_kernel_artifacts_v1(
    root: impl AsRef<Path>,
) -> Result<AdmittedPersistedM1KernelArtifactsV1, M1PersistedKernelArtifactOpenErrorV1> {
    reopen_with_hook(root.as_ref(), |_| {})
}

fn reopen_with_hook(
    root: &Path,
    mut after_open: impl FnMut(M1PersistedKernelArtifactFileV1),
) -> Result<AdmittedPersistedM1KernelArtifactsV1, M1PersistedKernelArtifactOpenErrorV1> {
    let root = openat2(
        CWD,
        root,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )
    .map_err(|source| io_error(M1PersistedKernelArtifactFileV1::RootDirectory, source))?;

    let manifest_bytes = read_regular_file(
        &root,
        M1_KERNEL_ARTIFACT_MANIFEST_FILENAME_V1,
        M1PersistedKernelArtifactFileV1::Manifest,
        ReadBound::Maximum(M1_KERNEL_ARTIFACT_MANIFEST_MAX_BYTES_V1),
        &mut after_open,
    )?;
    let manifest = decode_m1_kernel_artifact_manifest_v1(&manifest_bytes)
        .map_err(M1PersistedKernelArtifactOpenErrorV1::Manifest)?;

    let objects = open_directory_at(
        &root,
        "objects",
        M1PersistedKernelArtifactFileV1::ObjectsDirectory,
    )?;
    let sha256 = open_directory_at(
        &objects,
        "sha256",
        M1PersistedKernelArtifactFileV1::Sha256Directory,
    )?;

    let mut object_bytes = Vec::with_capacity(M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1);
    let mut plans = Vec::with_capacity(M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1);
    for entry in manifest.entries() {
        let family = entry.family();
        let name = object_name(entry.artifact().sha256());
        let exact_len = usize::try_from(entry.artifact().byte_len()).map_err(|_| {
            M1PersistedKernelArtifactOpenErrorV1::InvalidSize(
                M1PersistedKernelArtifactFileV1::Object(family),
            )
        })?;
        let bytes = read_regular_file(
            &sha256,
            &name,
            M1PersistedKernelArtifactFileV1::Object(family),
            ReadBound::Exact(exact_len),
            &mut after_open,
        )?;
        if <[u8; 32]>::from(Sha256::digest(&bytes)) != *entry.artifact().sha256() {
            return Err(M1PersistedKernelArtifactOpenErrorV1::ObjectIdentity(family));
        }
        let envelope =
            fe2o3_amdhsa_loader::validate(&bytes, AdmittedProfile::Gfx942XnackOffCov6).map_err(
                |source| M1PersistedKernelArtifactOpenErrorV1::Loader { family, source },
            )?;
        let plan = *envelope.plan();
        if !entry.matches_validated_load_plan(&plan) {
            return Err(M1PersistedKernelArtifactOpenErrorV1::LoaderPlanDrift(
                family,
            ));
        }
        object_bytes.push(bytes.into_boxed_slice());
        plans.push(plan);
    }

    let objects: [Box<[u8]>; M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1] = object_bytes
        .try_into()
        .unwrap_or_else(|_| unreachable!("canonical manifest has exactly seven families"));
    let plans: [LoadPlan; M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1] = plans
        .try_into()
        .unwrap_or_else(|_| unreachable!("canonical manifest has exactly seven families"));
    let current_source = current_source_facts()?;
    for ((entry, expected), family) in manifest
        .entries()
        .iter()
        .zip(current_source)
        .zip(M1KernelArtifactFamilyV1::ALL)
    {
        if expected.family() != family
            || entry.compiler_module() != expected.compiler_module()
            || entry.compiler_handoff() != expected.compiler_handoff()
            || entry.symbol_manifest() != expected.symbol_manifest()
            || entry.profile_catalogs() != expected.profile_catalogs()
        {
            return Err(M1PersistedKernelArtifactOpenErrorV1::CurrentSourceIdentityDrift(family));
        }
    }
    let source_contracts = std::array::from_fn(|index| {
        let identity = current_source[index].compiler_handoff();
        M1PhysicalProgramSourceContractV1::new(*identity.sha256(), identity.byte_len())
    });
    let catalog = bind_content_bound_m1_program_catalog_from_persisted_v1(
        object_borrows(&objects),
        &plans,
        &source_contracts,
    )
    .map_err(M1PersistedKernelArtifactOpenErrorV1::ProgramCatalog)?;
    let program_catalog_id = catalog.catalog_id();
    drop(catalog);

    Ok(AdmittedPersistedM1KernelArtifactsV1 {
        manifest,
        objects,
        plans,
        source_contracts,
        program_catalog_id,
    })
}

static CURRENT_SOURCE_FACTS: OnceLock<
    [M1CurrentKernelSourceFactsV1; M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1],
> = OnceLock::new();

fn current_source_facts() -> Result<
    &'static [M1CurrentKernelSourceFactsV1; M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1],
    M1PersistedKernelArtifactOpenErrorV1,
> {
    if let Some(facts) = CURRENT_SOURCE_FACTS.get() {
        return Ok(facts);
    }
    let facts = current_m1_kernel_source_facts_v1()
        .map_err(M1PersistedKernelArtifactOpenErrorV1::CurrentSourceContract)?;
    Ok(CURRENT_SOURCE_FACTS.get_or_init(|| facts))
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

fn open_directory_at(
    parent: &OwnedFd,
    name: &str,
    file: M1PersistedKernelArtifactFileV1,
) -> Result<OwnedFd, M1PersistedKernelArtifactOpenErrorV1> {
    openat2(
        parent,
        name,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )
    .map_err(|source| io_error(file, source))
}

fn read_regular_file(
    directory: &OwnedFd,
    name: &str,
    subject: M1PersistedKernelArtifactFileV1,
    bound: ReadBound,
    after_open: &mut impl FnMut(M1PersistedKernelArtifactFileV1),
) -> Result<Vec<u8>, M1PersistedKernelArtifactOpenErrorV1> {
    let descriptor = openat2(
        directory,
        name,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )
    .map_err(|source| io_error(subject, source))?;
    let initial = fstat(&descriptor).map_err(|source| io_error(subject, source))?;
    if FileType::from_raw_mode(initial.st_mode) != FileType::RegularFile {
        return Err(M1PersistedKernelArtifactOpenErrorV1::NotRegularFile(
            subject,
        ));
    }
    let initial_len = usize::try_from(initial.st_size)
        .ok()
        .filter(|length| bound.accepts(*length))
        .ok_or(M1PersistedKernelArtifactOpenErrorV1::InvalidSize(subject))?;

    after_open(subject);
    let mut file = File::from(descriptor);
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(initial_len.saturating_add(1))
        .map_err(|_| M1PersistedKernelArtifactOpenErrorV1::ReadBufferAllocation(subject))?;
    Read::by_ref(&mut file)
        .take(bound.limit().saturating_add(1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|source| M1PersistedKernelArtifactOpenErrorV1::Io {
            file: subject,
            source,
        })?;
    let final_stat = fstat(&file).map_err(|source| io_error(subject, source))?;
    if !same_file_snapshot(&initial, &final_stat) {
        return Err(M1PersistedKernelArtifactOpenErrorV1::ChangedWhileReading(
            subject,
        ));
    }
    if bytes.len() != initial_len || !bound.accepts(bytes.len()) {
        return Err(M1PersistedKernelArtifactOpenErrorV1::InvalidSize(subject));
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
    file: M1PersistedKernelArtifactFileV1,
    source: rustix::io::Errno,
) -> M1PersistedKernelArtifactOpenErrorV1 {
    if matches!(source, rustix::io::Errno::NOSYS | rustix::io::Errno::INVAL) {
        return M1PersistedKernelArtifactOpenErrorV1::StrictNoFollowUnavailable(io::Error::from(
            source,
        ));
    }
    M1PersistedKernelArtifactOpenErrorV1::Io {
        file,
        source: io::Error::from(source),
    }
}

fn object_borrows(
    objects: &[Box<[u8]>; M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1],
) -> [&[u8]; M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1] {
    std::array::from_fn(|index| objects[index].as_ref())
}

fn object_name(digest: &[u8; 32]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut name = String::with_capacity(64 + ".hsaco".len());
    for byte in digest {
        name.push(DIGITS[(byte >> 4) as usize] as char);
        name.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    name.push_str(".hsaco");
    name
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::symlink;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use fe2o3_amdhsa_loader::{AdmittedProfile, LoadPlan};
    use ferric_build::{
        m1_kernel_artifact_manifest_test_fixture_v1,
        m1_kernel_artifact_manifest_with_source_facts_test_fixture_v1, M1KernelArtifactFamilyV1,
        M1KernelArtifactManifestErrorV1, M1KernelArtifactManifestV1,
        M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1, M1_KERNEL_ARTIFACT_MANIFEST_FILENAME_V1,
        M1_KERNEL_ARTIFACT_MANIFEST_MAX_BYTES_V1,
    };

    use super::{
        current_source_facts, reopen_persisted_m1_kernel_artifacts_v1, reopen_with_hook,
        M1PersistedKernelArtifactFileV1, M1PersistedKernelArtifactOpenErrorV1,
    };

    const PHOFF: usize = 64;
    const PHENT: usize = 56;
    const PHNUM: usize = 8;
    const SHOFF: usize = 0x3000;
    const PT_LOAD: u32 = 1;
    const PT_DYNAMIC: u32 = 2;
    const PT_NOTE: u32 = 4;
    const PT_PHDR: u32 = 6;
    const PT_GNU_STACK: u32 = 0x6474_e551;
    const PT_GNU_RELRO: u32 = 0x6474_e552;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(label: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "ferric-m1-reopen-{label}-{}-{nonce}",
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

    struct Fixture {
        directory: TestDirectory,
        root: PathBuf,
        objects: [Vec<u8>; M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1],
        plans: [LoadPlan; M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1],
        manifest: M1KernelArtifactManifestV1,
    }

    impl Fixture {
        fn new(label: &str) -> Self {
            let directory = TestDirectory::new(label);
            let root = directory.0.join("artifacts");
            fs::create_dir_all(root.join("objects/sha256")).unwrap();
            let objects = std::array::from_fn(|index| {
                let mut bytes = loader_fixture();
                bytes[0x1080 + index] = u8::try_from(index + 1).unwrap();
                bytes
            });
            let plans = std::array::from_fn(|index| {
                *fe2o3_amdhsa_loader::validate(&objects[index], AdmittedProfile::Gfx942XnackOffCov6)
                    .unwrap()
                    .plan()
            });
            let manifest = fixture_manifest(&objects, &plans);
            write_fixture(&root, &manifest, &objects);
            Self {
                directory,
                root,
                objects,
                plans,
                manifest,
            }
        }

        fn manifest_path(&self) -> PathBuf {
            self.root.join(M1_KERNEL_ARTIFACT_MANIFEST_FILENAME_V1)
        }

        fn object_path(&self, family: M1KernelArtifactFamilyV1) -> PathBuf {
            let entry = &self.manifest.entries()[family as usize - 1];
            self.root.join(entry.object_path())
        }

        fn new_with_inert_source_facts(label: &str) -> Self {
            let mut fixture = Self::new(label);
            let manifest = inert_fixture_manifest(&fixture.objects, &fixture.plans);
            write_fixture(&fixture.root, &manifest, &fixture.objects);
            fixture.manifest = manifest;
            fixture
        }
    }

    #[test]
    fn intact_loader_valid_fixture_reaches_exact_symbol_closure() {
        let fixture = Fixture::new("symbol-closure");
        assert!(matches!(
            reopen_persisted_m1_kernel_artifacts_v1(&fixture.root),
            Err(M1PersistedKernelArtifactOpenErrorV1::ProgramCatalog(_))
        ));
    }

    #[test]
    fn inert_synthetic_manifest_cannot_mint_current_source_authority() {
        let fixture = Fixture::new_with_inert_source_facts("source-authority");
        assert!(matches!(
            reopen_persisted_m1_kernel_artifacts_v1(&fixture.root),
            Err(
                M1PersistedKernelArtifactOpenErrorV1::CurrentSourceIdentityDrift(
                    M1KernelArtifactFamilyV1::Gemm
                )
            )
        ));
    }

    #[test]
    fn every_source_identity_field_and_family_drift_fails_closed() {
        let fixture = Fixture::new("source-identity-matrix");
        let canonical = fixture.manifest.canonical_bytes();
        for family in M1KernelArtifactFamilyV1::ALL {
            let entry = &fixture.manifest.entries()[family as usize - 1];
            for identity in [
                entry.compiler_module(),
                entry.compiler_handoff(),
                entry.symbol_manifest(),
            ] {
                let mut hostile = canonical.to_vec();
                let matches = hostile
                    .windows(identity.sha256().len())
                    .enumerate()
                    .filter_map(|(offset, bytes)| (bytes == identity.sha256()).then_some(offset))
                    .collect::<Vec<_>>();
                assert_eq!(
                    matches.len(),
                    1,
                    "source identity must be unique in manifest"
                );
                hostile[matches[0]] ^= 0x80;
                fs::write(fixture.manifest_path(), hostile).unwrap();
                assert!(matches!(
                    reopen_persisted_m1_kernel_artifacts_v1(&fixture.root),
                    Err(M1PersistedKernelArtifactOpenErrorV1::CurrentSourceIdentityDrift(
                        actual
                    )) if actual == family
                ));
            }
        }
        fs::write(fixture.manifest_path(), canonical).unwrap();
    }

    #[test]
    fn root_manifest_and_object_symlinks_fail_closed() {
        let fixture = Fixture::new("symlinks");
        let root_link = fixture.directory.0.join("root-link");
        symlink(&fixture.root, &root_link).unwrap();
        assert!(matches!(
            reopen_persisted_m1_kernel_artifacts_v1(&root_link),
            Err(M1PersistedKernelArtifactOpenErrorV1::Io {
                file: M1PersistedKernelArtifactFileV1::RootDirectory,
                ..
            })
        ));

        let intermediate_link = fixture.directory.0.join("intermediate-link");
        symlink(&fixture.directory.0, &intermediate_link).unwrap();
        assert!(matches!(
            reopen_persisted_m1_kernel_artifacts_v1(intermediate_link.join("artifacts")),
            Err(M1PersistedKernelArtifactOpenErrorV1::Io {
                file: M1PersistedKernelArtifactFileV1::RootDirectory,
                ..
            })
        ));

        let manifest_path = fixture.manifest_path();
        let manifest_copy = fixture.directory.0.join("manifest-copy");
        fs::copy(&manifest_path, &manifest_copy).unwrap();
        fs::remove_file(&manifest_path).unwrap();
        symlink(&manifest_copy, &manifest_path).unwrap();
        assert!(matches!(
            reopen_persisted_m1_kernel_artifacts_v1(&fixture.root),
            Err(M1PersistedKernelArtifactOpenErrorV1::Io {
                file: M1PersistedKernelArtifactFileV1::Manifest,
                ..
            })
        ));

        fs::remove_file(&manifest_path).unwrap();
        fs::write(&manifest_path, fixture.manifest.canonical_bytes()).unwrap();
        let object_path = fixture.object_path(M1KernelArtifactFamilyV1::Gemm);
        let object_copy = fixture.directory.0.join("object-copy");
        fs::copy(&object_path, &object_copy).unwrap();
        fs::remove_file(&object_path).unwrap();
        symlink(&object_copy, &object_path).unwrap();
        assert!(matches!(
            reopen_persisted_m1_kernel_artifacts_v1(&fixture.root),
            Err(M1PersistedKernelArtifactOpenErrorV1::Io {
                file: M1PersistedKernelArtifactFileV1::Object(M1KernelArtifactFamilyV1::Gemm),
                ..
            })
        ));

        let fixture = Fixture::new("objects-symlink");
        let objects = fixture.root.join("objects");
        let moved_objects = fixture.root.join("moved-objects");
        fs::rename(&objects, &moved_objects).unwrap();
        symlink(&moved_objects, &objects).unwrap();
        assert!(matches!(
            reopen_persisted_m1_kernel_artifacts_v1(&fixture.root),
            Err(M1PersistedKernelArtifactOpenErrorV1::Io {
                file: M1PersistedKernelArtifactFileV1::ObjectsDirectory,
                ..
            })
        ));

        let fixture = Fixture::new("sha256-symlink");
        let sha256 = fixture.root.join("objects/sha256");
        let moved_sha256 = fixture.root.join("objects/moved-sha256");
        fs::rename(&sha256, &moved_sha256).unwrap();
        symlink(&moved_sha256, &sha256).unwrap();
        assert!(matches!(
            reopen_persisted_m1_kernel_artifacts_v1(&fixture.root),
            Err(M1PersistedKernelArtifactOpenErrorV1::Io {
                file: M1PersistedKernelArtifactFileV1::Sha256Directory,
                ..
            })
        ));
    }

    #[test]
    fn non_directory_components_and_non_regular_files_are_typed() {
        let root_file = TestDirectory::new("root-file");
        let path = root_file.0.join("not-directory");
        fs::write(&path, b"file").unwrap();
        assert!(matches!(
            reopen_persisted_m1_kernel_artifacts_v1(&path),
            Err(M1PersistedKernelArtifactOpenErrorV1::Io {
                file: M1PersistedKernelArtifactFileV1::RootDirectory,
                ..
            })
        ));

        let fixture = Fixture::new("component-types");
        fs::remove_dir_all(fixture.root.join("objects")).unwrap();
        fs::write(fixture.root.join("objects"), b"file").unwrap();
        assert!(matches!(
            reopen_persisted_m1_kernel_artifacts_v1(&fixture.root),
            Err(M1PersistedKernelArtifactOpenErrorV1::Io {
                file: M1PersistedKernelArtifactFileV1::ObjectsDirectory,
                ..
            })
        ));

        let fixture = Fixture::new("sha256-component-type");
        let sha256 = fixture.root.join("objects/sha256");
        fs::remove_dir_all(&sha256).unwrap();
        fs::write(&sha256, b"file").unwrap();
        assert!(matches!(
            reopen_persisted_m1_kernel_artifacts_v1(&fixture.root),
            Err(M1PersistedKernelArtifactOpenErrorV1::Io {
                file: M1PersistedKernelArtifactFileV1::Sha256Directory,
                ..
            })
        ));

        let fixture = Fixture::new("manifest-type");
        fs::remove_file(fixture.manifest_path()).unwrap();
        fs::create_dir(fixture.manifest_path()).unwrap();
        assert!(matches!(
            reopen_persisted_m1_kernel_artifacts_v1(&fixture.root),
            Err(M1PersistedKernelArtifactOpenErrorV1::NotRegularFile(
                M1PersistedKernelArtifactFileV1::Manifest
            ))
        ));

        let fixture = Fixture::new("object-type");
        let object = fixture.object_path(M1KernelArtifactFamilyV1::Gemm);
        fs::remove_file(&object).unwrap();
        fs::create_dir(&object).unwrap();
        assert!(matches!(
            reopen_persisted_m1_kernel_artifacts_v1(&fixture.root),
            Err(M1PersistedKernelArtifactOpenErrorV1::NotRegularFile(
                M1PersistedKernelArtifactFileV1::Object(M1KernelArtifactFamilyV1::Gemm)
            ))
        ));
    }

    #[test]
    fn manifest_truncation_trailing_bytes_and_oversize_fail_closed() {
        let fixture = Fixture::new("manifest-bounds");
        let canonical = fixture.manifest.canonical_bytes();
        fs::write(fixture.manifest_path(), &canonical[..canonical.len() - 1]).unwrap();
        assert!(matches!(
            reopen_persisted_m1_kernel_artifacts_v1(&fixture.root),
            Err(M1PersistedKernelArtifactOpenErrorV1::Manifest(_))
        ));

        let mut trailing = canonical.to_vec();
        trailing.push(0);
        fs::write(fixture.manifest_path(), trailing).unwrap();
        assert!(matches!(
            reopen_persisted_m1_kernel_artifacts_v1(&fixture.root),
            Err(M1PersistedKernelArtifactOpenErrorV1::Manifest(
                M1KernelArtifactManifestErrorV1::NonCanonical
            ))
        ));

        fs::write(
            fixture.manifest_path(),
            vec![0; M1_KERNEL_ARTIFACT_MANIFEST_MAX_BYTES_V1 + 1],
        )
        .unwrap();
        assert!(matches!(
            reopen_persisted_m1_kernel_artifacts_v1(&fixture.root),
            Err(M1PersistedKernelArtifactOpenErrorV1::InvalidSize(
                M1PersistedKernelArtifactFileV1::Manifest
            ))
        ));
    }

    #[test]
    fn object_truncation_and_trailing_bytes_fail_before_hashing() {
        for (label, truncate) in [("object-truncated", true), ("object-trailing", false)] {
            let fixture = Fixture::new(label);
            let path = fixture.object_path(M1KernelArtifactFamilyV1::Gemm);
            let mut bytes = fixture.objects[0].clone();
            if truncate {
                bytes.pop();
            } else {
                bytes.push(0);
            }
            fs::write(path, bytes).unwrap();
            assert!(matches!(
                reopen_persisted_m1_kernel_artifacts_v1(&fixture.root),
                Err(M1PersistedKernelArtifactOpenErrorV1::InvalidSize(
                    M1PersistedKernelArtifactFileV1::Object(M1KernelArtifactFamilyV1::Gemm)
                ))
            ));
        }
    }

    #[test]
    fn equal_length_object_and_manifest_substitution_fail_closed() {
        let fixture = Fixture::new("object-substitution");
        fs::write(
            fixture.object_path(M1KernelArtifactFamilyV1::Gemm),
            &fixture.objects[1],
        )
        .unwrap();
        assert!(matches!(
            reopen_persisted_m1_kernel_artifacts_v1(&fixture.root),
            Err(M1PersistedKernelArtifactOpenErrorV1::ObjectIdentity(
                M1KernelArtifactFamilyV1::Gemm
            ))
        ));

        let fixture = Fixture::new("manifest-substitution");
        let mut substituted_objects = fixture.objects.clone();
        substituted_objects[0][0x108f] ^= 1;
        let substituted_manifest = fixture_manifest(&substituted_objects, &fixture.plans);
        fs::write(
            fixture.manifest_path(),
            substituted_manifest.canonical_bytes(),
        )
        .unwrap();
        assert!(matches!(
            reopen_persisted_m1_kernel_artifacts_v1(&fixture.root),
            Err(M1PersistedKernelArtifactOpenErrorV1::Io {
                file: M1PersistedKernelArtifactFileV1::Object(M1KernelArtifactFamilyV1::Gemm),
                ..
            })
        ));
    }

    #[test]
    fn wrong_target_bytes_fail_generic_loader_validation() {
        let fixture = Fixture::new("wrong-target");
        let mut wrong_target = fixture.objects.clone();
        write_u32(&mut wrong_target[0], 48, 0);
        let manifest = fixture_manifest(&wrong_target, &fixture.plans);
        write_fixture(&fixture.root, &manifest, &wrong_target);
        assert!(matches!(
            reopen_persisted_m1_kernel_artifacts_v1(&fixture.root),
            Err(M1PersistedKernelArtifactOpenErrorV1::Loader {
                family: M1KernelArtifactFamilyV1::Gemm,
                ..
            })
        ));
    }

    #[test]
    fn valid_but_substituted_persisted_load_plan_fails_closed() {
        let fixture = Fixture::new("load-plan-drift");
        let mut alternate = fixture.objects[0].clone();
        let executable_header = PHOFF + 2 * PHENT;
        write_u64(&mut alternate, executable_header + 32, 0x80);
        write_u64(&mut alternate, executable_header + 40, 0x80);
        let alternate_plan =
            *fe2o3_amdhsa_loader::validate(&alternate, AdmittedProfile::Gfx942XnackOffCov6)
                .unwrap()
                .plan();
        assert_ne!(alternate_plan, fixture.plans[0]);
        let mut substituted_plans = fixture.plans;
        substituted_plans[0] = alternate_plan;
        let manifest = fixture_manifest(&fixture.objects, &substituted_plans);
        fs::write(fixture.manifest_path(), manifest.canonical_bytes()).unwrap();
        assert!(matches!(
            reopen_persisted_m1_kernel_artifacts_v1(&fixture.root),
            Err(M1PersistedKernelArtifactOpenErrorV1::LoaderPlanDrift(
                M1KernelArtifactFamilyV1::Gemm
            ))
        ));
    }

    #[test]
    fn renamed_root_after_manifest_open_cannot_redirect_descriptor_walk() {
        let fixture = Fixture::new("root-rename-race");
        let moved = fixture.directory.0.join("moved-artifacts");
        let replacement = fixture.root.clone();
        let mut injected = false;
        let result = reopen_with_hook(&fixture.root, |subject| {
            if !injected && subject == M1PersistedKernelArtifactFileV1::Manifest {
                fs::rename(&replacement, &moved).unwrap();
                fs::create_dir(&replacement).unwrap();
                fs::write(
                    replacement.join(M1_KERNEL_ARTIFACT_MANIFEST_FILENAME_V1),
                    b"substituted",
                )
                .unwrap();
                injected = true;
            }
        });
        assert!(matches!(
            result,
            Err(M1PersistedKernelArtifactOpenErrorV1::ProgramCatalog(_))
        ));
    }

    #[test]
    fn renamed_object_store_after_first_open_cannot_redirect_later_objects() {
        let fixture = Fixture::new("store-rename-race");
        let store = fixture.root.join("objects/sha256");
        let moved = fixture.root.join("objects/moved-sha256");
        let mut injected = false;
        let result = reopen_with_hook(&fixture.root, |subject| {
            if !injected
                && subject
                    == M1PersistedKernelArtifactFileV1::Object(M1KernelArtifactFamilyV1::Gemm)
            {
                fs::rename(&store, &moved).unwrap();
                fs::create_dir(&store).unwrap();
                fs::write(store.join("substituted.hsaco"), b"substituted").unwrap();
                injected = true;
            }
        });
        assert!(matches!(
            result,
            Err(M1PersistedKernelArtifactOpenErrorV1::ProgramCatalog(_))
        ));
    }

    #[test]
    fn in_place_object_mutation_during_read_is_never_admitted() {
        let fixture = Fixture::new("object-mutation-race");
        let path = fixture.object_path(M1KernelArtifactFamilyV1::Gemm);
        let mut substituted = fixture.objects[0].clone();
        substituted[0x108f] ^= 1;
        let mut injected = false;
        let result = reopen_with_hook(&fixture.root, |subject| {
            if !injected
                && subject
                    == M1PersistedKernelArtifactFileV1::Object(M1KernelArtifactFamilyV1::Gemm)
            {
                fs::write(&path, &substituted).unwrap();
                injected = true;
            }
        });
        assert!(matches!(
            result,
            Err(M1PersistedKernelArtifactOpenErrorV1::ChangedWhileReading(
                M1PersistedKernelArtifactFileV1::Object(M1KernelArtifactFamilyV1::Gemm)
            ) | M1PersistedKernelArtifactOpenErrorV1::ObjectIdentity(
                M1KernelArtifactFamilyV1::Gemm
            ))
        ));
    }

    #[test]
    #[ignore = "requires a locally persisted complete M1 K1-K7 artifact directory"]
    fn configured_real_directory_reopens_and_scopes_complete_catalog() {
        let root = std::env::var_os("FERRIC_M1_KERNEL_ARTIFACT_DIRECTORY")
            .expect("set FERRIC_M1_KERNEL_ARTIFACT_DIRECTORY");
        let owner = reopen_persisted_m1_kernel_artifacts_v1(root).unwrap();
        assert_eq!(owner.manifest().entries().len(), 7);
        owner
            .with_content_bound_program_catalog_v1(|catalog| {
                assert_eq!(catalog.catalog_id(), owner.program_catalog_id());
                assert_eq!(catalog.program_count(), 12);
                for program in crate::M1PhysicalProgramV1::ALL {
                    let envelope = catalog.program(program);
                    assert!(envelope.dispatch_abi_identity().is_some());
                    for (index, argument) in envelope
                        .selected_kernel()
                        .explicit_arguments()
                        .iter()
                        .enumerate()
                    {
                        if argument.value_kind() == fe2o3_hsaco::ExplicitValueKind::GlobalBuffer {
                            assert!(envelope.dispatch_pointee_alignment(index).is_some());
                            assert!(envelope.dispatch_actual_access(index).is_some());
                        }
                    }
                }
            })
            .unwrap();
        assert!(!owner.reconstructs_inspected_worker_custody());
        assert!(!owner.has_independent_deployment_pin());
        assert!(!owner.proves_hsa_executable_load());
        assert!(!owner.proves_hardware_execution());
    }

    fn fixture_manifest(
        objects: &[Vec<u8>; M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1],
        plans: &[LoadPlan; M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1],
    ) -> M1KernelArtifactManifestV1 {
        m1_kernel_artifact_manifest_with_source_facts_test_fixture_v1(
            std::array::from_fn(|index| objects[index].as_slice()),
            plans,
            current_source_facts().expect("current source facts remain constructible"),
        )
        .unwrap()
    }

    fn inert_fixture_manifest(
        objects: &[Vec<u8>; M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1],
        plans: &[LoadPlan; M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1],
    ) -> M1KernelArtifactManifestV1 {
        m1_kernel_artifact_manifest_test_fixture_v1(
            std::array::from_fn(|index| objects[index].as_slice()),
            plans,
        )
        .unwrap()
    }

    fn write_fixture(
        root: &Path,
        manifest: &M1KernelArtifactManifestV1,
        objects: &[Vec<u8>; M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1],
    ) {
        fs::create_dir_all(root.join("objects/sha256")).unwrap();
        fs::write(
            root.join(M1_KERNEL_ARTIFACT_MANIFEST_FILENAME_V1),
            manifest.canonical_bytes(),
        )
        .unwrap();
        for (entry, bytes) in manifest.entries().iter().zip(objects) {
            fs::write(root.join(entry.object_path()), bytes).unwrap();
        }
    }

    fn loader_fixture() -> Vec<u8> {
        let mut bytes = vec![0_u8; SHOFF + 64];
        bytes[..16].copy_from_slice(b"\x7fELF\x02\x01\x01\x40\x04\0\0\0\0\0\0\0");
        write_u16(&mut bytes, 16, 3);
        write_u16(&mut bytes, 18, 224);
        write_u32(&mut bytes, 20, 1);
        write_u64(&mut bytes, 24, 0);
        write_u64(&mut bytes, 32, PHOFF as u64);
        write_u64(&mut bytes, 40, SHOFF as u64);
        write_u32(&mut bytes, 48, 0x64c);
        write_u16(&mut bytes, 52, 64);
        write_u16(&mut bytes, 54, u16::try_from(PHENT).unwrap());
        write_u16(&mut bytes, 56, u16::try_from(PHNUM).unwrap());
        write_u16(&mut bytes, 58, 64);
        write_u16(&mut bytes, 60, 1);
        write_u16(&mut bytes, 62, 0);

        phdr(&mut bytes, 0, PT_PHDR, 4, 0x40, 0x40, 0x1c0, 0x1c0, 8);
        phdr(&mut bytes, 1, PT_LOAD, 4, 0, 0, 0x300, 0x300, 0x1000);
        phdr(
            &mut bytes, 2, PT_LOAD, 5, 0x1000, 0x2000, 0x100, 0x100, 0x1000,
        );
        phdr(
            &mut bytes, 3, PT_LOAD, 6, 0x2000, 0x4000, 0x80, 0x1000, 0x1000,
        );
        phdr(&mut bytes, 4, PT_DYNAMIC, 6, 0x2000, 0x4000, 0x70, 0x70, 8);
        phdr(
            &mut bytes,
            5,
            PT_GNU_RELRO,
            4,
            0x2000,
            0x4000,
            0x80,
            0x1000,
            1,
        );
        phdr(&mut bytes, 6, PT_GNU_STACK, 6, 0, 0, 0, 0, 0);
        phdr(&mut bytes, 7, PT_NOTE, 4, 0x200, 0x200, 0x18, 0x18, 4);

        write_u32(&mut bytes, 0x200, 7);
        write_u32(&mut bytes, 0x204, 1);
        write_u32(&mut bytes, 0x208, 32);
        bytes[0x20c..0x213].copy_from_slice(b"AMDGPU\0");
        bytes[0x214] = 0x80;

        for (index, (tag, value)) in [
            (6, 0x220),
            (11, 24),
            (5, 0x240),
            (10, 16),
            (0x6fff_fef5, 0x260),
            (4, 0x280),
            (0, 0),
        ]
        .into_iter()
        .enumerate()
        {
            write_u64(&mut bytes, 0x2000 + index * 16, tag);
            write_u64(&mut bytes, 0x2008 + index * 16, value);
        }
        bytes
    }

    #[allow(clippy::too_many_arguments)]
    fn phdr(
        bytes: &mut [u8],
        index: usize,
        header_type: u32,
        flags: u32,
        offset: u64,
        virtual_address: u64,
        file_size: u64,
        memory_size: u64,
        alignment: u64,
    ) {
        let base = PHOFF + index * PHENT;
        write_u32(bytes, base, header_type);
        write_u32(bytes, base + 4, flags);
        write_u64(bytes, base + 8, offset);
        write_u64(bytes, base + 16, virtual_address);
        write_u64(bytes, base + 24, virtual_address);
        write_u64(bytes, base + 32, file_size);
        write_u64(bytes, base + 40, memory_size);
        write_u64(bytes, base + 48, alignment);
    }

    fn write_u16(bytes: &mut [u8], offset: usize, value: u16) {
        bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u64(bytes: &mut [u8], offset: usize, value: u64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }
}
