//! Canonical inert manifest for one complete M1 K1-K7 artifact build.
//!
//! The manifest is content-addressed build evidence. It is not an independent
//! deployment approval and cannot recreate the live inspected kernel owners.

use std::collections::BTreeSet;
use std::fmt;

use fe2o3_amdhsa_loader::{
    LoadPlan, SegmentPermissions, LOAD_ALIGNMENT, LOAD_SEGMENT_COUNT, MAX_IMAGE_SPAN_BYTES,
    MAX_INPUT_BYTES, MAX_METADATA_BYTES,
};
use fe2o3_hsaco_finalize::ContentIdentityV1;
use ferric_qwen_kernels::{
    gemm, logits, paged_decode, prefill, rmsnorm, rope_kv, swiglu, QWEN3_GFX942_OCML_IMPORT_V1,
    QWEN3_GFX942_OCML_PROVIDER_FILES_V1, QWEN3_GFX942_OCML_PROVIDER_IDENTITY_V1,
};
use sha2::{Digest, Sha256};

use super::kernel_artifact_policy::{
    M1_KERNEL_WORKER_BUILD_IDENTITY_V1, M1_KERNEL_WORKER_EXECUTABLE_BYTES_V1,
    M1_KERNEL_WORKER_EXECUTABLE_SHA256_V1, M1_KERNEL_WORKER_LLVM_BUILD_IDENTITY_V1,
};

const MANIFEST_MAGIC: &[u8] = b"FERRIC-M1-KERNEL-ARTIFACTS-V1\0";
/// Maximum encoded size admitted for the canonical K1-K7 artifact manifest.
pub const M1_KERNEL_ARTIFACT_MANIFEST_MAX_BYTES_V1: usize = 64 * 1024;
const TARGET: &str = "gfx942:xnack-";
const CODE_OBJECT_VERSION: u8 = 6;
const LINK_OPTIONS: [(&str, &str); 4] = [
    ("code-object-version", "6"),
    ("opt-level", "2"),
    ("strip-debug", "true"),
    ("verify-each", "true"),
];

const ASSEMBLY_CATALOG_DOMAIN: &[u8] =
    b"ferric.m1.kernel-artifact-builder.speculative-assembly-catalog.v1";

/// Canonical manifest format version.
pub const M1_KERNEL_ARTIFACT_MANIFEST_VERSION_V1: u32 = 1;
/// Exact K1-K7 artifact family count.
pub const M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1: usize = 7;
/// Exact selected physical-program count across the seven artifacts.
pub const M1_PHYSICAL_PROGRAM_COUNT_V1: usize = 12;

/// Stable K1-K7 artifact family ordinal.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum M1KernelArtifactFamilyV1 {
    /// K1 GEMM/GEMV and token embedding.
    Gemm = 1,
    /// K2 RMSNorm/residual.
    RmsNorm = 2,
    /// K3 rotary position and paged KV write.
    RopeKv = 3,
    /// K4 causal paged prefill attention.
    Prefill = 4,
    /// K5 paged grouped-query decode.
    PagedDecode = 5,
    /// K6 `SwiGLU` activation.
    SwiGlu = 6,
    /// K7 logits and compact completion.
    Logits = 7,
}

impl M1KernelArtifactFamilyV1 {
    /// Complete canonical family order.
    pub const ALL: [Self; M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1] = [
        Self::Gemm,
        Self::RmsNorm,
        Self::RopeKv,
        Self::Prefill,
        Self::PagedDecode,
        Self::SwiGlu,
        Self::Logits,
    ];

    /// Stable short family name used in diagnostics and transaction labels.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Gemm => "k1-gemm",
            Self::RmsNorm => "k2-rmsnorm",
            Self::RopeKv => "k3-rope-kv",
            Self::Prefill => "k4-prefill",
            Self::PagedDecode => "k5-paged-decode",
            Self::SwiGlu => "k6-swiglu",
            Self::Logits => "k7-logits",
        }
    }

    pub(crate) const fn uses_ocml(self) -> bool {
        matches!(self, Self::Prefill | Self::PagedDecode | Self::SwiGlu)
    }

    fn decode(value: u8) -> Result<Self, M1KernelArtifactManifestErrorV1> {
        Self::ALL
            .into_iter()
            .find(|family| *family as u8 == value)
            .ok_or(M1KernelArtifactManifestErrorV1::Invalid("family ordinal"))
    }
}

/// One complete finite runtime-profile catalog carried by a family artifact.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct M1KernelProfileCatalogV1 {
    name: String,
    profile_count: u32,
    identity: [u8; 32],
}

impl M1KernelProfileCatalogV1 {
    pub(crate) fn new(name: &str, profile_count: usize, identity: [u8; 32]) -> Self {
        Self {
            name: name.to_owned(),
            profile_count: u32::try_from(profile_count).expect("finite profile count fits u32"),
            identity,
        }
    }

    /// Stable catalog role within its artifact family.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Complete profile count, never one selected runtime profile.
    #[must_use]
    pub const fn profile_count(&self) -> u32 {
        self.profile_count
    }

    /// Domain-separated catalog identity.
    #[must_use]
    pub const fn identity(&self) -> &[u8; 32] {
        &self.identity
    }
}

/// One ordered kernel/descriptor pair retained by a family artifact.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct M1KernelArtifactProgramV1 {
    kernel_symbol: String,
    descriptor_symbol: String,
}

impl M1KernelArtifactProgramV1 {
    fn new(kernel_symbol: &str, descriptor_symbol: &str) -> Self {
        Self {
            kernel_symbol: kernel_symbol.to_owned(),
            descriptor_symbol: descriptor_symbol.to_owned(),
        }
    }

    /// Exact AMDHSA kernel symbol.
    #[must_use]
    pub fn kernel_symbol(&self) -> &str {
        &self.kernel_symbol
    }

    /// Exact AMDHSA descriptor symbol.
    #[must_use]
    pub fn descriptor_symbol(&self) -> &str {
        &self.descriptor_symbol
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct LoadSegmentRecordV1 {
    file_offset: u64,
    file_size: u64,
    virtual_address: u64,
    memory_size: u64,
    mapping_address: u64,
    mapping_size: u64,
    permissions: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LoadPlanRecordV1 {
    input_len: u64,
    image_start: u64,
    image_end: u64,
    metadata_offset: u64,
    metadata_len: u64,
    segments: Vec<LoadSegmentRecordV1>,
}

impl LoadPlanRecordV1 {
    fn from_plan(plan: &LoadPlan) -> Self {
        Self {
            input_len: plan.input_len(),
            image_start: plan.image_start(),
            image_end: plan.image_end(),
            metadata_offset: plan.metadata_note().file_offset(),
            metadata_len: plan.metadata_note().byte_len(),
            segments: plan
                .segments()
                .iter()
                .map(|segment| LoadSegmentRecordV1 {
                    file_offset: segment.file_offset(),
                    file_size: segment.file_size(),
                    virtual_address: segment.virtual_address(),
                    memory_size: segment.memory_size(),
                    mapping_address: segment.mapping_address(),
                    mapping_size: segment.mapping_size(),
                    permissions: match segment.permissions() {
                        SegmentPermissions::ReadOnly => 1,
                        SegmentPermissions::ReadExecute => 2,
                        SegmentPermissions::ReadWrite => 3,
                    },
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DeviceLibraryProviderRecordV1 {
    identity: String,
    import_symbol: String,
    files: Vec<(String, [u8; 32])>,
}

impl DeviceLibraryProviderRecordV1 {
    fn ocml() -> Self {
        Self {
            identity: QWEN3_GFX942_OCML_PROVIDER_IDENTITY_V1.to_owned(),
            import_symbol: QWEN3_GFX942_OCML_IMPORT_V1.to_owned(),
            files: QWEN3_GFX942_OCML_PROVIDER_FILES_V1
                .into_iter()
                .map(|(name, digest)| (name.to_owned(), digest))
                .collect(),
        }
    }
}

/// Canonical facts for one complete family HSACO.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct M1KernelArtifactEntryV1 {
    family: M1KernelArtifactFamilyV1,
    artifact: ContentIdentityV1,
    compiler_module: ContentIdentityV1,
    compiler_handoff: ContentIdentityV1,
    symbol_manifest: ContentIdentityV1,
    profile_catalogs: Vec<M1KernelProfileCatalogV1>,
    programs: Vec<M1KernelArtifactProgramV1>,
    provider: Option<DeviceLibraryProviderRecordV1>,
    load_plan: LoadPlanRecordV1,
}

impl M1KernelArtifactEntryV1 {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        family: M1KernelArtifactFamilyV1,
        artifact: ContentIdentityV1,
        compiler_module: ContentIdentityV1,
        compiler_handoff: ContentIdentityV1,
        symbol_manifest: ContentIdentityV1,
        profile_catalogs: Vec<M1KernelProfileCatalogV1>,
        load_plan: &LoadPlan,
    ) -> Self {
        Self {
            family,
            artifact,
            compiler_module,
            compiler_handoff,
            symbol_manifest,
            profile_catalogs,
            programs: expected_programs(family),
            provider: family.uses_ocml().then(DeviceLibraryProviderRecordV1::ocml),
            load_plan: LoadPlanRecordV1::from_plan(load_plan),
        }
    }

    /// Stable K1-K7 family.
    #[must_use]
    pub const fn family(&self) -> M1KernelArtifactFamilyV1 {
        self.family
    }

    /// SHA-256 and exact byte length of the family HSACO.
    #[must_use]
    pub const fn artifact(&self) -> ContentIdentityV1 {
        self.artifact
    }

    /// Identity of the exact LLVM module submitted to the measured Worker.
    #[must_use]
    pub const fn compiler_module(&self) -> ContentIdentityV1 {
        self.compiler_module
    }

    /// Identity of the exact compiler handoff consumed by the build attempt.
    #[must_use]
    pub const fn compiler_handoff(&self) -> ContentIdentityV1 {
        self.compiler_handoff
    }

    /// Identity of the exact K1-K7 symbol manifest checked during inspection.
    #[must_use]
    pub const fn symbol_manifest(&self) -> ContentIdentityV1 {
        self.symbol_manifest
    }

    /// Complete finite profile catalogs contained by this one artifact.
    #[must_use]
    pub fn profile_catalogs(&self) -> &[M1KernelProfileCatalogV1] {
        &self.profile_catalogs
    }

    /// Ordered kernel/descriptor roster contained by this one artifact.
    #[must_use]
    pub fn programs(&self) -> &[M1KernelArtifactProgramV1] {
        &self.programs
    }

    /// Whether the measured Worker supplied the exact reviewed OCML closure.
    #[must_use]
    pub const fn uses_ocml_provider(&self) -> bool {
        self.provider.is_some()
    }

    /// Relative content-addressed object path derived solely from the digest.
    #[must_use]
    pub fn object_path(&self) -> String {
        format!("objects/sha256/{}.hsaco", hex(self.artifact.sha256()))
    }

    /// Whether fresh generic-loader validation reproduced the exact persisted plan.
    ///
    /// This comparison grants no compiler, artifact, load, or execution custody.
    #[must_use]
    pub fn matches_validated_load_plan(&self, plan: &LoadPlan) -> bool {
        self.load_plan == LoadPlanRecordV1::from_plan(plan)
    }
}

/// Strict canonical manifest for the complete seven-artifact build.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct M1KernelArtifactManifestV1 {
    entries: [M1KernelArtifactEntryV1; M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1],
    canonical_bytes: Box<[u8]>,
    identity: ContentIdentityV1,
}

impl M1KernelArtifactManifestV1 {
    pub(crate) fn new(
        entries: [M1KernelArtifactEntryV1; M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1],
    ) -> Result<Self, M1KernelArtifactManifestErrorV1> {
        validate_entries(&entries)?;
        let canonical_bytes = encode_manifest(&entries)?;
        let identity = ContentIdentityV1::calculate(&canonical_bytes);
        Ok(Self {
            entries,
            canonical_bytes: canonical_bytes.into_boxed_slice(),
            identity,
        })
    }

    /// Complete family entries in fixed K1-K7 order.
    #[must_use]
    pub const fn entries(&self) -> &[M1KernelArtifactEntryV1; M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1] {
        &self.entries
    }

    /// Exact canonical manifest bytes.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    /// SHA-256 and length of the complete canonical manifest.
    #[must_use]
    pub const fn identity(&self) -> ContentIdentityV1 {
        self.identity
    }

    /// Self-content-addressing is integrity, not independent deployment approval.
    #[must_use]
    pub const fn has_independent_deployment_pin(&self) -> bool {
        false
    }

    /// A persisted manifest cannot recreate live inspected Worker evidence.
    #[must_use]
    pub const fn grants_persisted_reopen_authority(&self) -> bool {
        false
    }

    /// Offline structural inspection makes no HSA load or execution claim.
    #[must_use]
    pub const fn proves_hardware_execution(&self) -> bool {
        false
    }
}

/// Failure while decoding or constructing a canonical K1-K7 manifest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum M1KernelArtifactManifestErrorV1 {
    /// Record exceeds its fixed maximum.
    TooLarge,
    /// Record ended before a declared field was complete.
    Truncated,
    /// Text is not canonical printable ASCII.
    InvalidText,
    /// A fixed roster or structural relation is invalid.
    Invalid(&'static str),
    /// Ferric could not reconstruct one of its own canonical finite catalogs.
    CanonicalCatalog(M1KernelCanonicalCatalogErrorV1),
    /// Bytes decode but are not the unique canonical encoding.
    NonCanonical,
}

impl fmt::Display for M1KernelArtifactManifestErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "M1 kernel artifact manifest rejected: {self:?}")
    }
}

impl std::error::Error for M1KernelArtifactManifestErrorV1 {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CanonicalCatalog(source) => Some(source),
            _ => None,
        }
    }
}

/// Exact canonical finite-catalog constructor that unexpectedly failed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1KernelCanonicalCatalogErrorV1 {
    /// K1 GEMM/GEMV catalog derivation failed.
    Gemm(gemm::Qwen3GemmCatalogErrorV1),
    /// K1 token-embedding catalog derivation failed.
    TokenEmbedding(gemm::Qwen3TokenEmbeddingCatalogErrorV1),
    /// K2 `RMSNorm` catalog derivation failed.
    RmsNorm(rmsnorm::Qwen3RmsNormCatalogErrorV1),
    /// K3 rotary/KV catalog derivation failed.
    RopeKv(rope_kv::Qwen3RopeKvCatalogErrorV1),
    /// K4 prefill catalog derivation failed.
    Prefill(prefill::Qwen3PrefillCatalogErrorV1),
    /// K5 paged-decode catalog derivation failed.
    PagedDecode(paged_decode::Qwen3PagedDecodeCatalogErrorV1),
    /// K6 `SwiGLU` catalog derivation failed.
    SwiGlu(swiglu::Qwen3SwiGluCatalogErrorV1),
    /// K7 logits catalog derivation failed.
    Logits(logits::Qwen3LogitsCatalogErrorV1),
}

impl fmt::Display for M1KernelCanonicalCatalogErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "canonical M1 kernel catalog failed: {self:?}")
    }
}

impl std::error::Error for M1KernelCanonicalCatalogErrorV1 {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(match self {
            Self::Gemm(source) => source,
            Self::TokenEmbedding(source) => source,
            Self::RmsNorm(source) => source,
            Self::RopeKv(source) => source,
            Self::Prefill(source) => source,
            Self::PagedDecode(source) => source,
            Self::SwiGlu(source) => source,
            Self::Logits(source) => source,
        })
    }
}

/// Constructs a canonical manifest around caller-supplied structural test objects.
///
/// This helper exists only under the `test-fixtures` feature and grants no
/// compiler-build, Worker, deployment, load, or execution custody.
#[cfg(feature = "test-fixtures")]
#[doc(hidden)]
pub fn m1_kernel_artifact_manifest_test_fixture_v1(
    objects: [&[u8]; M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1],
    plans: &[LoadPlan; M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1],
) -> Result<M1KernelArtifactManifestV1, M1KernelArtifactManifestErrorV1> {
    let entries = std::array::from_fn(|index| {
        let seed = u8::try_from(index + 1).expect("seven fixture families fit u8");
        M1KernelArtifactEntryV1::new(
            M1KernelArtifactFamilyV1::ALL[index],
            ContentIdentityV1::calculate(objects[index]),
            ContentIdentityV1::from_parts([seed; 32], 1),
            ContentIdentityV1::from_parts([seed + 16; 32], 1),
            ContentIdentityV1::from_parts([seed + 32; 32], 1),
            canonical_profile_catalogs(M1KernelArtifactFamilyV1::ALL[index])
                .expect("checked-in finite test catalogs remain constructible"),
            &plans[index],
        )
    });
    M1KernelArtifactManifestV1::new(entries)
}

/// Constructs a structural test manifest carrying caller-supplied current source facts.
///
/// This helper exists only under the `test-fixtures` feature. The resulting
/// manifest remains inert: persisted admission must independently reproduce
/// the supplied facts before using them.
#[cfg(feature = "test-fixtures")]
#[doc(hidden)]
pub fn m1_kernel_artifact_manifest_with_source_facts_test_fixture_v1(
    objects: [&[u8]; M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1],
    plans: &[LoadPlan; M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1],
    source_facts: &[super::kernel_artifacts::M1CurrentKernelSourceFactsV1;
         M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1],
) -> Result<M1KernelArtifactManifestV1, M1KernelArtifactManifestErrorV1> {
    for (facts, expected_family) in source_facts.iter().zip(M1KernelArtifactFamilyV1::ALL) {
        if facts.family() != expected_family {
            return Err(M1KernelArtifactManifestErrorV1::Invalid(
                "test source-fact order",
            ));
        }
    }
    let entries = std::array::from_fn(|index| {
        let facts = &source_facts[index];
        M1KernelArtifactEntryV1::new(
            M1KernelArtifactFamilyV1::ALL[index],
            ContentIdentityV1::calculate(objects[index]),
            facts.compiler_module(),
            facts.compiler_handoff(),
            facts.symbol_manifest(),
            facts.profile_catalogs().to_vec(),
            &plans[index],
        )
    });
    M1KernelArtifactManifestV1::new(entries)
}

impl From<M1KernelCanonicalCatalogErrorV1> for M1KernelArtifactManifestErrorV1 {
    fn from(source: M1KernelCanonicalCatalogErrorV1) -> Self {
        Self::CanonicalCatalog(source)
    }
}

/// Decodes and re-encodes one strict canonical inert manifest.
///
/// # Errors
///
/// Returns [`M1KernelArtifactManifestErrorV1`] for an oversized, truncated,
/// non-canonical, or structurally substituted record.
pub fn decode_m1_kernel_artifact_manifest_v1(
    bytes: &[u8],
) -> Result<M1KernelArtifactManifestV1, M1KernelArtifactManifestErrorV1> {
    if bytes.len() > M1_KERNEL_ARTIFACT_MANIFEST_MAX_BYTES_V1 {
        return Err(M1KernelArtifactManifestErrorV1::TooLarge);
    }
    let mut decoder = Decoder::new(bytes);
    decoder.expect(MANIFEST_MAGIC)?;
    if decoder.u32()? != M1_KERNEL_ARTIFACT_MANIFEST_VERSION_V1 {
        return Err(M1KernelArtifactManifestErrorV1::Invalid("version"));
    }
    if decoder.text()? != TARGET || decoder.u8()? != CODE_OBJECT_VERSION {
        return Err(M1KernelArtifactManifestErrorV1::Invalid("target"));
    }
    if decoder.u8()? as usize != M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1
        || decoder.u8()? as usize != M1_PHYSICAL_PROGRAM_COUNT_V1
    {
        return Err(M1KernelArtifactManifestErrorV1::Invalid("cardinality"));
    }
    let executable = decoder.content_identity()?;
    if executable
        != ContentIdentityV1::from_parts(
            M1_KERNEL_WORKER_EXECUTABLE_SHA256_V1,
            M1_KERNEL_WORKER_EXECUTABLE_BYTES_V1,
        )
        || decoder.text()? != M1_KERNEL_WORKER_BUILD_IDENTITY_V1
        || decoder.text()? != M1_KERNEL_WORKER_LLVM_BUILD_IDENTITY_V1
    {
        return Err(M1KernelArtifactManifestErrorV1::Invalid("worker policy"));
    }
    if decoder.u8()? as usize != LINK_OPTIONS.len() {
        return Err(M1KernelArtifactManifestErrorV1::Invalid("link options"));
    }
    for expected in LINK_OPTIONS {
        if decoder.text()? != expected.0 || decoder.text()? != expected.1 {
            return Err(M1KernelArtifactManifestErrorV1::Invalid("link option"));
        }
    }
    if decoder.u8()? as usize != M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1 {
        return Err(M1KernelArtifactManifestErrorV1::Invalid("family count"));
    }
    let mut entries = Vec::with_capacity(M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1);
    for _ in 0..M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1 {
        entries.push(decode_entry(&mut decoder)?);
    }
    decoder.finish()?;
    let manifest = M1KernelArtifactManifestV1::new(
        entries
            .try_into()
            .map_err(|_| M1KernelArtifactManifestErrorV1::Invalid("family count"))?,
    )?;
    if manifest.canonical_bytes() != bytes {
        return Err(M1KernelArtifactManifestErrorV1::NonCanonical);
    }
    Ok(manifest)
}

fn validate_entries(
    entries: &[M1KernelArtifactEntryV1; M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1],
) -> Result<(), M1KernelArtifactManifestErrorV1> {
    let mut artifacts = BTreeSet::new();
    let mut program_count = 0;
    for (entry, expected_family) in entries.iter().zip(M1KernelArtifactFamilyV1::ALL) {
        if entry.family != expected_family {
            return Err(M1KernelArtifactManifestErrorV1::Invalid("family order"));
        }
        for identity in [
            entry.artifact,
            entry.compiler_module,
            entry.compiler_handoff,
            entry.symbol_manifest,
        ] {
            if identity.byte_len() == 0 || identity.sha256() == &[0; 32] {
                return Err(M1KernelArtifactManifestErrorV1::Invalid("content identity"));
            }
        }
        if !artifacts.insert((
            entry.artifact.sha256().to_owned(),
            entry.artifact.byte_len(),
        )) {
            return Err(M1KernelArtifactManifestErrorV1::Invalid(
                "duplicate artifact",
            ));
        }
        validate_load_plan(entry)?;
        validate_profiles(entry)?;
        if entry.programs != expected_programs(entry.family) {
            return Err(M1KernelArtifactManifestErrorV1::Invalid("program roster"));
        }
        if entry.provider
            != entry
                .family
                .uses_ocml()
                .then(DeviceLibraryProviderRecordV1::ocml)
        {
            return Err(M1KernelArtifactManifestErrorV1::Invalid("provider policy"));
        }
        program_count += entry.programs.len();
    }
    if program_count != M1_PHYSICAL_PROGRAM_COUNT_V1 {
        return Err(M1KernelArtifactManifestErrorV1::Invalid("program count"));
    }
    Ok(())
}

fn validate_load_plan(
    entry: &M1KernelArtifactEntryV1,
) -> Result<(), M1KernelArtifactManifestErrorV1> {
    let plan = &entry.load_plan;
    let metadata_end = plan
        .metadata_offset
        .checked_add(plan.metadata_len)
        .ok_or(M1KernelArtifactManifestErrorV1::Invalid("load plan"))?;
    let image_span = plan
        .image_end
        .checked_sub(plan.image_start)
        .ok_or(M1KernelArtifactManifestErrorV1::Invalid("load plan"))?;
    if plan.input_len != entry.artifact.byte_len()
        || plan.input_len == 0
        || plan.input_len > MAX_INPUT_BYTES as u64
        || plan.metadata_len == 0
        || plan.metadata_len > MAX_METADATA_BYTES
        || metadata_end > plan.input_len
        || !plan.image_start.is_multiple_of(LOAD_ALIGNMENT)
        || !plan.image_end.is_multiple_of(LOAD_ALIGNMENT)
        || image_span == 0
        || image_span > MAX_IMAGE_SPAN_BYTES
        || plan.segments.len() != LOAD_SEGMENT_COUNT
    {
        return Err(M1KernelArtifactManifestErrorV1::Invalid("load plan"));
    }

    let mut permissions = [0_u8; 4];
    let mut previous_virtual_end = None;
    let mut previous_mapping_end = None;
    for segment in &plan.segments {
        let file_end = segment
            .file_offset
            .checked_add(segment.file_size)
            .ok_or(M1KernelArtifactManifestErrorV1::Invalid("load plan"))?;
        let virtual_end = segment
            .virtual_address
            .checked_add(segment.memory_size)
            .ok_or(M1KernelArtifactManifestErrorV1::Invalid("load plan"))?;
        let mapping_end = segment
            .mapping_address
            .checked_add(segment.mapping_size)
            .ok_or(M1KernelArtifactManifestErrorV1::Invalid("load plan"))?;
        let mapping_prefix = segment
            .virtual_address
            .checked_sub(segment.mapping_address)
            .ok_or(M1KernelArtifactManifestErrorV1::Invalid("load plan"))?;
        let required_mapping = mapping_prefix
            .checked_add(segment.memory_size)
            .and_then(|size| size.checked_add(LOAD_ALIGNMENT - 1))
            .map(|size| size / LOAD_ALIGNMENT * LOAD_ALIGNMENT)
            .ok_or(M1KernelArtifactManifestErrorV1::Invalid("load plan"))?;
        if segment.file_size == 0
            || segment.memory_size < segment.file_size
            || file_end > plan.input_len
            || segment.mapping_address % LOAD_ALIGNMENT != 0
            || mapping_prefix >= LOAD_ALIGNMENT
            || segment.mapping_size != required_mapping
            || segment.mapping_size == 0
            || previous_virtual_end.is_some_and(|end| end > segment.virtual_address)
            || previous_mapping_end.is_some_and(|end| end > segment.mapping_address)
        {
            return Err(M1KernelArtifactManifestErrorV1::Invalid("load plan"));
        }
        let slot = permissions
            .get_mut(segment.permissions as usize)
            .ok_or(M1KernelArtifactManifestErrorV1::Invalid("load plan"))?;
        *slot += 1;
        previous_virtual_end = Some(virtual_end);
        previous_mapping_end = Some(mapping_end);
    }
    for (index, first) in plan.segments.iter().enumerate() {
        let first_end = first.file_offset + first.file_size;
        for second in &plan.segments[index + 1..] {
            let second_end = second.file_offset + second.file_size;
            if first.file_offset < second_end && second.file_offset < first_end {
                return Err(M1KernelArtifactManifestErrorV1::Invalid("load plan"));
            }
        }
    }
    if permissions[1..] != [1, 1, 1]
        || plan.image_start != plan.segments[0].mapping_address
        || plan.image_end
            != plan.segments[LOAD_SEGMENT_COUNT - 1]
                .mapping_address
                .checked_add(plan.segments[LOAD_SEGMENT_COUNT - 1].mapping_size)
                .ok_or(M1KernelArtifactManifestErrorV1::Invalid("load plan"))?
    {
        return Err(M1KernelArtifactManifestErrorV1::Invalid("load plan"));
    }
    Ok(())
}

fn validate_profiles(
    entry: &M1KernelArtifactEntryV1,
) -> Result<(), M1KernelArtifactManifestErrorV1> {
    let expected = canonical_profile_catalogs(entry.family)?;
    if entry.profile_catalogs.len() != expected.len() {
        return Err(M1KernelArtifactManifestErrorV1::Invalid(
            "profile catalog count",
        ));
    }
    if entry.profile_catalogs != expected {
        return Err(M1KernelArtifactManifestErrorV1::Invalid("profile catalog"));
    }
    Ok(())
}

fn canonical_profile_catalogs(
    family: M1KernelArtifactFamilyV1,
) -> Result<Vec<M1KernelProfileCatalogV1>, M1KernelArtifactManifestErrorV1> {
    Ok(match family {
        M1KernelArtifactFamilyV1::Gemm => {
            let catalog = gemm::Qwen3GemmProfileCatalogV1::canonical()
                .map_err(M1KernelCanonicalCatalogErrorV1::Gemm)?;
            let embedding = gemm::Qwen3TokenEmbeddingProfileCatalogV1::canonical()
                .map_err(M1KernelCanonicalCatalogErrorV1::TokenEmbedding)?;
            vec![
                M1KernelProfileCatalogV1::new(
                    "gemm",
                    gemm::QWEN3_GEMM_PROFILE_COUNT_V1,
                    *catalog.identity().as_bytes(),
                ),
                M1KernelProfileCatalogV1::new(
                    "token-embedding",
                    gemm::QWEN3_TOKEN_EMBEDDING_PROFILE_COUNT_V1,
                    *embedding.identity().as_bytes(),
                ),
            ]
        }
        M1KernelArtifactFamilyV1::RmsNorm => {
            let catalog = rmsnorm::Qwen3RmsNormProfileCatalogV1::canonical()
                .map_err(M1KernelCanonicalCatalogErrorV1::RmsNorm)?;
            vec![M1KernelProfileCatalogV1::new(
                "rmsnorm",
                rmsnorm::QWEN3_RMSNORM_PROFILE_COUNT_V1,
                *catalog.identity().as_bytes(),
            )]
        }
        M1KernelArtifactFamilyV1::RopeKv => {
            let catalog = rope_kv::Qwen3RopeKvProfileCatalogV1::canonical()
                .map_err(M1KernelCanonicalCatalogErrorV1::RopeKv)?;
            vec![M1KernelProfileCatalogV1::new(
                "rope-kv",
                rope_kv::QWEN3_ROPE_KV_PROFILE_COUNT_V1,
                *catalog.identity().as_bytes(),
            )]
        }
        M1KernelArtifactFamilyV1::Prefill => {
            let catalog = prefill::Qwen3PrefillProfileCatalogV1::canonical()
                .map_err(M1KernelCanonicalCatalogErrorV1::Prefill)?;
            vec![M1KernelProfileCatalogV1::new(
                "prefill",
                prefill::QWEN3_PREFILL_PROFILE_COUNT_V1,
                *catalog.identity().as_bytes(),
            )]
        }
        M1KernelArtifactFamilyV1::PagedDecode => {
            let catalog = paged_decode::Qwen3PagedDecodeProfileCatalogV1::canonical()
                .map_err(M1KernelCanonicalCatalogErrorV1::PagedDecode)?;
            vec![M1KernelProfileCatalogV1::new(
                "paged-decode",
                paged_decode::QWEN3_PAGED_DECODE_PROFILE_COUNT_V1,
                *catalog.identity().as_bytes(),
            )]
        }
        M1KernelArtifactFamilyV1::SwiGlu => {
            let catalog = swiglu::Qwen3SwiGluProfileCatalogV1::canonical()
                .map_err(M1KernelCanonicalCatalogErrorV1::SwiGlu)?;
            vec![M1KernelProfileCatalogV1::new(
                "swiglu",
                swiglu::QWEN3_SWIGLU_PROFILE_COUNT_V1,
                *catalog.identity().as_bytes(),
            )]
        }
        M1KernelArtifactFamilyV1::Logits => {
            let catalog = logits::Qwen3LogitsProfileCatalogV1::canonical()
                .map_err(M1KernelCanonicalCatalogErrorV1::Logits)?;
            vec![
                M1KernelProfileCatalogV1::new(
                    "logits",
                    logits::QWEN3_LOGITS_PROFILE_COUNT_V1,
                    *catalog.identity().as_bytes(),
                ),
                M1KernelProfileCatalogV1::new(
                    "speculative-token-assembly",
                    logits::QWEN3_SPECULATIVE_TOKEN_ASSEMBLY_PROFILE_COUNT_V1,
                    speculative_assembly_catalog_identity(),
                ),
            ]
        }
    })
}

pub(crate) fn speculative_assembly_catalog_identity() -> [u8; 32] {
    let profiles = logits::qwen3_speculative_token_assembly_profiles_v1();
    let mut digest = Sha256::new();
    digest.update((ASSEMBLY_CATALOG_DOMAIN.len() as u64).to_le_bytes());
    digest.update(ASSEMBLY_CATALOG_DOMAIN);
    digest.update((profiles.len() as u64).to_le_bytes());
    for profile in profiles {
        digest.update(profile.identity().as_bytes());
    }
    digest.finalize().into()
}

fn expected_programs(family: M1KernelArtifactFamilyV1) -> Vec<M1KernelArtifactProgramV1> {
    let pairs: &[(&str, &str)] = match family {
        M1KernelArtifactFamilyV1::Gemm => &[
            (
                gemm::QWEN3_GEMM_REFERENCE_KERNEL_SYMBOL_V1,
                gemm::QWEN3_GEMM_REFERENCE_DESCRIPTOR_SYMBOL_V1,
            ),
            (
                gemm::QWEN3_GEMM_VECTORIZED_KERNEL_SYMBOL_V1,
                gemm::QWEN3_GEMM_VECTORIZED_DESCRIPTOR_SYMBOL_V1,
            ),
            (
                gemm::QWEN3_TOKEN_EMBEDDING_KERNEL_SYMBOL_V1,
                gemm::QWEN3_TOKEN_EMBEDDING_DESCRIPTOR_SYMBOL_V1,
            ),
        ],
        M1KernelArtifactFamilyV1::RmsNorm => &[(
            rmsnorm::QWEN3_RMSNORM_KERNEL_SYMBOL_V1,
            rmsnorm::QWEN3_RMSNORM_KERNEL_DESCRIPTOR_SYMBOL_V1,
        )],
        M1KernelArtifactFamilyV1::RopeKv => &[
            (
                rope_kv::QWEN3_ROPE_KERNEL_SYMBOL_V1,
                rope_kv::QWEN3_ROPE_KERNEL_DESCRIPTOR_SYMBOL_V1,
            ),
            (
                rope_kv::QWEN3_PAGED_KV_WRITE_KERNEL_SYMBOL_V1,
                rope_kv::QWEN3_PAGED_KV_WRITE_KERNEL_DESCRIPTOR_SYMBOL_V1,
            ),
        ],
        M1KernelArtifactFamilyV1::Prefill => &[(
            prefill::QWEN3_PREFILL_KERNEL_SYMBOL_V1,
            prefill::QWEN3_PREFILL_KERNEL_DESCRIPTOR_SYMBOL_V1,
        )],
        M1KernelArtifactFamilyV1::PagedDecode => &[(
            paged_decode::QWEN3_PAGED_DECODE_KERNEL_SYMBOL_V1,
            paged_decode::QWEN3_PAGED_DECODE_KERNEL_DESCRIPTOR_SYMBOL_V1,
        )],
        M1KernelArtifactFamilyV1::SwiGlu => &[(
            swiglu::QWEN3_SWIGLU_KERNEL_SYMBOL_V1,
            swiglu::QWEN3_SWIGLU_KERNEL_DESCRIPTOR_SYMBOL_V1,
        )],
        M1KernelArtifactFamilyV1::Logits => &[
            (
                logits::QWEN3_LOGITS_ARGMAX_KERNEL_SYMBOL_V1,
                logits::QWEN3_LOGITS_ARGMAX_DESCRIPTOR_SYMBOL_V1,
            ),
            (
                logits::QWEN3_LOGITS_COMPACT_KERNEL_SYMBOL_V1,
                logits::QWEN3_LOGITS_COMPACT_DESCRIPTOR_SYMBOL_V1,
            ),
            (
                logits::QWEN3_SPECULATIVE_TOKEN_ASSEMBLY_KERNEL_SYMBOL_V1,
                logits::QWEN3_SPECULATIVE_TOKEN_ASSEMBLY_DESCRIPTOR_SYMBOL_V1,
            ),
        ],
    };
    pairs
        .iter()
        .map(|(kernel, descriptor)| M1KernelArtifactProgramV1::new(kernel, descriptor))
        .collect()
}

fn encode_manifest(
    entries: &[M1KernelArtifactEntryV1; M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1],
) -> Result<Vec<u8>, M1KernelArtifactManifestErrorV1> {
    let mut bytes = Vec::with_capacity(8 * 1024);
    bytes.extend_from_slice(MANIFEST_MAGIC);
    push_u32(&mut bytes, M1_KERNEL_ARTIFACT_MANIFEST_VERSION_V1);
    push_text(&mut bytes, TARGET)?;
    bytes.push(CODE_OBJECT_VERSION);
    bytes.push(exact_u8(M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1));
    bytes.push(exact_u8(M1_PHYSICAL_PROGRAM_COUNT_V1));
    push_content(
        &mut bytes,
        ContentIdentityV1::from_parts(
            M1_KERNEL_WORKER_EXECUTABLE_SHA256_V1,
            M1_KERNEL_WORKER_EXECUTABLE_BYTES_V1,
        ),
    );
    push_text(&mut bytes, M1_KERNEL_WORKER_BUILD_IDENTITY_V1)?;
    push_text(&mut bytes, M1_KERNEL_WORKER_LLVM_BUILD_IDENTITY_V1)?;
    bytes.push(exact_u8(LINK_OPTIONS.len()));
    for (name, value) in LINK_OPTIONS {
        push_text(&mut bytes, name)?;
        push_text(&mut bytes, value)?;
    }
    bytes.push(exact_u8(entries.len()));
    for entry in entries {
        encode_entry(&mut bytes, entry)?;
    }
    if bytes.len() > M1_KERNEL_ARTIFACT_MANIFEST_MAX_BYTES_V1 {
        return Err(M1KernelArtifactManifestErrorV1::TooLarge);
    }
    Ok(bytes)
}

fn encode_entry(
    bytes: &mut Vec<u8>,
    entry: &M1KernelArtifactEntryV1,
) -> Result<(), M1KernelArtifactManifestErrorV1> {
    bytes.push(entry.family as u8);
    push_text(bytes, entry.family.name())?;
    for identity in [
        entry.artifact,
        entry.compiler_module,
        entry.compiler_handoff,
        entry.symbol_manifest,
    ] {
        push_content(bytes, identity);
    }
    bytes.push(exact_u8(entry.profile_catalogs.len()));
    for catalog in &entry.profile_catalogs {
        push_text(bytes, &catalog.name)?;
        push_u32(bytes, catalog.profile_count);
        bytes.extend_from_slice(&catalog.identity);
    }
    bytes.push(exact_u8(entry.programs.len()));
    for program in &entry.programs {
        push_text(bytes, &program.kernel_symbol)?;
        push_text(bytes, &program.descriptor_symbol)?;
    }
    match &entry.provider {
        None => bytes.push(0),
        Some(provider) => {
            bytes.push(1);
            push_text(bytes, &provider.identity)?;
            push_text(bytes, &provider.import_symbol)?;
            bytes.push(exact_u8(provider.files.len()));
            for (name, digest) in &provider.files {
                push_text(bytes, name)?;
                bytes.extend_from_slice(digest);
            }
        }
    }
    let plan = &entry.load_plan;
    for value in [
        plan.input_len,
        plan.image_start,
        plan.image_end,
        plan.metadata_offset,
        plan.metadata_len,
    ] {
        push_u64(bytes, value);
    }
    bytes.push(exact_u8(plan.segments.len()));
    for segment in &plan.segments {
        for value in [
            segment.file_offset,
            segment.file_size,
            segment.virtual_address,
            segment.memory_size,
            segment.mapping_address,
            segment.mapping_size,
        ] {
            push_u64(bytes, value);
        }
        bytes.push(segment.permissions);
    }
    Ok(())
}

fn decode_entry(
    decoder: &mut Decoder<'_>,
) -> Result<M1KernelArtifactEntryV1, M1KernelArtifactManifestErrorV1> {
    let family = M1KernelArtifactFamilyV1::decode(decoder.u8()?)?;
    if decoder.text()? != family.name() {
        return Err(M1KernelArtifactManifestErrorV1::Invalid("family name"));
    }
    let artifact = decoder.content_identity()?;
    let compiler_module = decoder.content_identity()?;
    let compiler_handoff = decoder.content_identity()?;
    let symbol_manifest = decoder.content_identity()?;
    let profile_count = decoder.u8()? as usize;
    if profile_count > 2 {
        return Err(M1KernelArtifactManifestErrorV1::Invalid(
            "profile catalog count",
        ));
    }
    let mut profile_catalogs = Vec::with_capacity(profile_count);
    for _ in 0..profile_count {
        profile_catalogs.push(M1KernelProfileCatalogV1 {
            name: decoder.text()?.to_owned(),
            profile_count: decoder.u32()?,
            identity: decoder.fixed::<32>()?,
        });
    }
    let program_count = decoder.u8()? as usize;
    if program_count > 3 {
        return Err(M1KernelArtifactManifestErrorV1::Invalid("program count"));
    }
    let mut programs = Vec::with_capacity(program_count);
    for _ in 0..program_count {
        programs.push(M1KernelArtifactProgramV1 {
            kernel_symbol: decoder.text()?.to_owned(),
            descriptor_symbol: decoder.text()?.to_owned(),
        });
    }
    let provider = match decoder.u8()? {
        0 => None,
        1 => {
            let identity = decoder.text()?.to_owned();
            let import_symbol = decoder.text()?.to_owned();
            let file_count = decoder.u8()? as usize;
            if file_count > QWEN3_GFX942_OCML_PROVIDER_FILES_V1.len() {
                return Err(M1KernelArtifactManifestErrorV1::Invalid("provider files"));
            }
            let mut files = Vec::with_capacity(file_count);
            for _ in 0..file_count {
                files.push((decoder.text()?.to_owned(), decoder.fixed::<32>()?));
            }
            Some(DeviceLibraryProviderRecordV1 {
                identity,
                import_symbol,
                files,
            })
        }
        _ => return Err(M1KernelArtifactManifestErrorV1::Invalid("provider tag")),
    };
    let input_len = decoder.u64()?;
    let image_start = decoder.u64()?;
    let image_end = decoder.u64()?;
    let metadata_offset = decoder.u64()?;
    let metadata_len = decoder.u64()?;
    let segment_count = decoder.u8()? as usize;
    if segment_count != LOAD_SEGMENT_COUNT {
        return Err(M1KernelArtifactManifestErrorV1::Invalid("load segments"));
    }
    let mut segments = Vec::with_capacity(segment_count);
    for _ in 0..segment_count {
        let segment = LoadSegmentRecordV1 {
            file_offset: decoder.u64()?,
            file_size: decoder.u64()?,
            virtual_address: decoder.u64()?,
            memory_size: decoder.u64()?,
            mapping_address: decoder.u64()?,
            mapping_size: decoder.u64()?,
            permissions: decoder.u8()?,
        };
        if !(1..=3).contains(&segment.permissions) {
            return Err(M1KernelArtifactManifestErrorV1::Invalid(
                "segment permissions",
            ));
        }
        segments.push(segment);
    }
    Ok(M1KernelArtifactEntryV1 {
        family,
        artifact,
        compiler_module,
        compiler_handoff,
        symbol_manifest,
        profile_catalogs,
        programs,
        provider,
        load_plan: LoadPlanRecordV1 {
            input_len,
            image_start,
            image_end,
            metadata_offset,
            metadata_len,
            segments,
        },
    })
}

fn push_content(bytes: &mut Vec<u8>, identity: ContentIdentityV1) {
    bytes.extend_from_slice(identity.sha256());
    push_u64(bytes, identity.byte_len());
}

fn push_text(bytes: &mut Vec<u8>, value: &str) -> Result<(), M1KernelArtifactManifestErrorV1> {
    if value.is_empty()
        || value.len() > u16::MAX as usize
        || !value.is_ascii()
        || value.bytes().any(|byte| byte.is_ascii_control())
    {
        return Err(M1KernelArtifactManifestErrorV1::InvalidText);
    }
    let length =
        u16::try_from(value.len()).map_err(|_| M1KernelArtifactManifestErrorV1::InvalidText)?;
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(value.as_bytes());
    Ok(())
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn exact_u8(value: usize) -> u8 {
    u8::try_from(value).expect("validated manifest cardinality fits u8")
}

struct Decoder<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Decoder<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], M1KernelArtifactManifestErrorV1> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or(M1KernelArtifactManifestErrorV1::Truncated)?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(M1KernelArtifactManifestErrorV1::Truncated)?;
        self.offset = end;
        Ok(value)
    }

    fn expect(&mut self, expected: &[u8]) -> Result<(), M1KernelArtifactManifestErrorV1> {
        if self.take(expected.len())? != expected {
            return Err(M1KernelArtifactManifestErrorV1::Invalid("magic"));
        }
        Ok(())
    }

    fn fixed<const N: usize>(&mut self) -> Result<[u8; N], M1KernelArtifactManifestErrorV1> {
        self.take(N)?
            .try_into()
            .map_err(|_| M1KernelArtifactManifestErrorV1::Truncated)
    }

    fn u8(&mut self) -> Result<u8, M1KernelArtifactManifestErrorV1> {
        Ok(self.fixed::<1>()?[0])
    }

    fn u16(&mut self) -> Result<u16, M1KernelArtifactManifestErrorV1> {
        Ok(u16::from_le_bytes(self.fixed()?))
    }

    fn u32(&mut self) -> Result<u32, M1KernelArtifactManifestErrorV1> {
        Ok(u32::from_le_bytes(self.fixed()?))
    }

    fn u64(&mut self) -> Result<u64, M1KernelArtifactManifestErrorV1> {
        Ok(u64::from_le_bytes(self.fixed()?))
    }

    fn text(&mut self) -> Result<&'a str, M1KernelArtifactManifestErrorV1> {
        let length = self.u16()? as usize;
        let bytes = self.take(length)?;
        if bytes.is_empty() || !bytes.is_ascii() || bytes.iter().any(u8::is_ascii_control) {
            return Err(M1KernelArtifactManifestErrorV1::InvalidText);
        }
        std::str::from_utf8(bytes).map_err(|_| M1KernelArtifactManifestErrorV1::InvalidText)
    }

    fn content_identity(&mut self) -> Result<ContentIdentityV1, M1KernelArtifactManifestErrorV1> {
        let sha256 = self.fixed()?;
        let byte_len = self.u64()?;
        Ok(ContentIdentityV1::from_parts(sha256, byte_len))
    }

    fn finish(self) -> Result<(), M1KernelArtifactManifestErrorV1> {
        if self.offset != self.bytes.len() {
            return Err(M1KernelArtifactManifestErrorV1::NonCanonical);
        }
        Ok(())
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
pub(crate) fn m1_kernel_artifact_manifest_unit_fixture_v1() -> M1KernelArtifactManifestV1 {
    let source_facts = super::current_m1_kernel_source_facts_v1()
        .expect("current M1 source facts remain constructible");
    let entries = std::array::from_fn(|index| {
        let seed = u8::try_from(index + 1).expect("seven fixture families fit u8");
        let artifact_len = 4_096 + u64::from(seed);
        let family = M1KernelArtifactFamilyV1::ALL[index];
        let source = &source_facts[index];
        M1KernelArtifactEntryV1 {
            family,
            artifact: ContentIdentityV1::from_parts([seed; 32], artifact_len),
            compiler_module: source.compiler_module(),
            compiler_handoff: source.compiler_handoff(),
            symbol_manifest: source.symbol_manifest(),
            profile_catalogs: source.profile_catalogs().to_vec(),
            programs: expected_programs(family),
            provider: family.uses_ocml().then(DeviceLibraryProviderRecordV1::ocml),
            load_plan: LoadPlanRecordV1 {
                input_len: artifact_len,
                image_start: 0x1_000,
                image_end: 0x6_000,
                metadata_offset: 64,
                metadata_len: 64,
                segments: vec![
                    LoadSegmentRecordV1 {
                        file_offset: 0,
                        file_size: 256,
                        virtual_address: 0x1_000,
                        memory_size: 256,
                        mapping_address: 0x1_000,
                        mapping_size: 0x1_000,
                        permissions: 1,
                    },
                    LoadSegmentRecordV1 {
                        file_offset: 512,
                        file_size: 512,
                        virtual_address: 0x3_000,
                        memory_size: 512,
                        mapping_address: 0x3_000,
                        mapping_size: 0x1_000,
                        permissions: 2,
                    },
                    LoadSegmentRecordV1 {
                        file_offset: 1_024,
                        file_size: 512,
                        virtual_address: 0x5_000,
                        memory_size: 1_024,
                        mapping_address: 0x5_000,
                        mapping_size: 0x1_000,
                        permissions: 3,
                    },
                ],
            },
        }
    });
    M1KernelArtifactManifestV1::new(entries).expect("exact current-source unit manifest")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_identity(seed: u8, byte_len: u64) -> ContentIdentityV1 {
        ContentIdentityV1::from_parts([seed; 32], byte_len)
    }

    fn fixture_profiles(family: M1KernelArtifactFamilyV1) -> Vec<M1KernelProfileCatalogV1> {
        canonical_profile_catalogs(family).unwrap()
    }

    fn fixture_entry(family: M1KernelArtifactFamilyV1, index: usize) -> M1KernelArtifactEntryV1 {
        let seed = u8::try_from(index + 1).unwrap();
        let artifact_len = 4_096 + u64::from(seed);
        M1KernelArtifactEntryV1 {
            family,
            artifact: fixture_identity(seed, artifact_len),
            compiler_module: fixture_identity(seed + 16, 256),
            compiler_handoff: fixture_identity(seed + 32, 512),
            symbol_manifest: fixture_identity(seed + 48, 128),
            profile_catalogs: fixture_profiles(family),
            programs: expected_programs(family),
            provider: family.uses_ocml().then(DeviceLibraryProviderRecordV1::ocml),
            load_plan: LoadPlanRecordV1 {
                input_len: artifact_len,
                image_start: 0x1_000,
                image_end: 0x6_000,
                metadata_offset: 64,
                metadata_len: 64,
                segments: vec![
                    LoadSegmentRecordV1 {
                        file_offset: 0,
                        file_size: 256,
                        virtual_address: 0x1_000,
                        memory_size: 256,
                        mapping_address: 0x1_000,
                        mapping_size: 0x1_000,
                        permissions: 1,
                    },
                    LoadSegmentRecordV1 {
                        file_offset: 512,
                        file_size: 512,
                        virtual_address: 0x3_000,
                        memory_size: 512,
                        mapping_address: 0x3_000,
                        mapping_size: 0x1_000,
                        permissions: 2,
                    },
                    LoadSegmentRecordV1 {
                        file_offset: 1_024,
                        file_size: 512,
                        virtual_address: 0x5_000,
                        memory_size: 1_024,
                        mapping_address: 0x5_000,
                        mapping_size: 0x1_000,
                        permissions: 3,
                    },
                ],
            },
        }
    }

    fn fixture_entries() -> [M1KernelArtifactEntryV1; M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1] {
        std::array::from_fn(|index| fixture_entry(M1KernelArtifactFamilyV1::ALL[index], index))
    }

    fn fixture_manifest() -> M1KernelArtifactManifestV1 {
        M1KernelArtifactManifestV1::new(fixture_entries()).unwrap()
    }

    #[test]
    fn reviewed_ocml_closure_is_exact() {
        assert_eq!(QWEN3_GFX942_OCML_PROVIDER_FILES_V1.len(), 4);
        assert_eq!(
            hex(&QWEN3_GFX942_OCML_PROVIDER_FILES_V1[0].1),
            "cfe97fe9ee29379f522e5f20ae55aae1cdb96eb41d6aa250ea11c4941c54e019"
        );
        assert!(M1KernelArtifactFamilyV1::Prefill.uses_ocml());
        assert!(M1KernelArtifactFamilyV1::PagedDecode.uses_ocml());
        assert!(M1KernelArtifactFamilyV1::SwiGlu.uses_ocml());
        assert!(!M1KernelArtifactFamilyV1::Gemm.uses_ocml());
    }

    #[test]
    fn family_order_and_program_roster_are_complete() {
        assert_eq!(M1KernelArtifactFamilyV1::ALL.len(), 7);
        let count: usize = M1KernelArtifactFamilyV1::ALL
            .into_iter()
            .map(|family| expected_programs(family).len())
            .sum();
        assert_eq!(count, M1_PHYSICAL_PROGRAM_COUNT_V1);
        assert_eq!(expected_programs(M1KernelArtifactFamilyV1::Gemm).len(), 3);
        assert_eq!(expected_programs(M1KernelArtifactFamilyV1::Logits).len(), 3);
    }

    #[test]
    fn canonical_manifest_round_trips_and_is_self_addressed() {
        let manifest = fixture_manifest();
        let decoded = decode_m1_kernel_artifact_manifest_v1(manifest.canonical_bytes()).unwrap();
        assert_eq!(decoded, manifest);
        assert!(manifest.identity().matches(manifest.canonical_bytes()));
        assert!(!manifest.has_independent_deployment_pin());
        assert!(!manifest.grants_persisted_reopen_authority());
        assert!(!manifest.proves_hardware_execution());
        for entry in manifest.entries() {
            let path = entry.object_path();
            assert!(path.starts_with("objects/sha256/"));
            assert_eq!(&path[path.len() - ".hsaco".len()..], ".hsaco");
            assert_eq!(path.len(), "objects/sha256/".len() + 64 + ".hsaco".len());
            assert!(path.bytes().all(|byte| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b'/' | b'.' | b'-')
            }));
        }
    }

    #[test]
    fn every_truncation_and_trailing_data_fail_closed() {
        let manifest = fixture_manifest();
        for length in 0..manifest.canonical_bytes().len() {
            assert!(
                decode_m1_kernel_artifact_manifest_v1(&manifest.canonical_bytes()[..length])
                    .is_err()
            );
        }
        let mut trailing = manifest.canonical_bytes().to_vec();
        trailing.push(0);
        assert_eq!(
            decode_m1_kernel_artifact_manifest_v1(&trailing),
            Err(M1KernelArtifactManifestErrorV1::NonCanonical)
        );
        assert_eq!(
            decode_m1_kernel_artifact_manifest_v1(&vec![
                0;
                M1_KERNEL_ARTIFACT_MANIFEST_MAX_BYTES_V1
                    + 1
            ]),
            Err(M1KernelArtifactManifestErrorV1::TooLarge)
        );
    }

    #[test]
    fn header_cardinality_target_and_worker_substitutions_fail_closed() {
        let manifest = fixture_manifest();
        let cardinality_offset = MANIFEST_MAGIC.len() + 4 + 2 + TARGET.len() + 1;
        let worker_digest_offset = cardinality_offset + 2;
        for offset in [
            0,
            MANIFEST_MAGIC.len() + 4 + 2,
            cardinality_offset,
            cardinality_offset + 1,
            worker_digest_offset,
        ] {
            let mut substituted = manifest.canonical_bytes().to_vec();
            substituted[offset] ^= 1;
            assert!(decode_m1_kernel_artifact_manifest_v1(&substituted).is_err());
        }
    }

    #[test]
    fn order_count_profile_provider_and_identity_substitutions_fail_closed() {
        let mut entries = fixture_entries();
        entries.swap(0, 1);
        assert_eq!(
            M1KernelArtifactManifestV1::new(entries).unwrap_err(),
            M1KernelArtifactManifestErrorV1::Invalid("family order")
        );

        let mut entries = fixture_entries();
        entries[1].artifact = entries[0].artifact;
        assert_eq!(
            M1KernelArtifactManifestV1::new(entries).unwrap_err(),
            M1KernelArtifactManifestErrorV1::Invalid("duplicate artifact")
        );

        let mut entries = fixture_entries();
        entries[0].profile_catalogs.pop();
        assert_eq!(
            M1KernelArtifactManifestV1::new(entries).unwrap_err(),
            M1KernelArtifactManifestErrorV1::Invalid("profile catalog count")
        );

        let mut entries = fixture_entries();
        entries[3].provider = None;
        assert_eq!(
            M1KernelArtifactManifestV1::new(entries).unwrap_err(),
            M1KernelArtifactManifestErrorV1::Invalid("provider policy")
        );

        let mut entries = fixture_entries();
        entries[6].programs.pop();
        assert_eq!(
            M1KernelArtifactManifestV1::new(entries).unwrap_err(),
            M1KernelArtifactManifestErrorV1::Invalid("program roster")
        );
    }

    #[test]
    fn every_nonzero_catalog_identity_substitution_fails_decode() {
        let mut substitutions = 0;
        for family_index in 0..M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1 {
            let catalog_count = fixture_entries()[family_index].profile_catalogs.len();
            for catalog_index in 0..catalog_count {
                let mut entries = fixture_entries();
                let identity = &mut entries[family_index].profile_catalogs[catalog_index].identity;
                identity[0] ^= 0x80;
                assert_ne!(*identity, [0; 32]);
                let substituted = encode_manifest(&entries).unwrap();
                assert_eq!(
                    decode_m1_kernel_artifact_manifest_v1(&substituted),
                    Err(M1KernelArtifactManifestErrorV1::Invalid(
                        "profile catalog"
                    )),
                    "accepted catalog identity substitution for family {family_index}, catalog {catalog_index}"
                );
                substitutions += 1;
            }
        }
        assert_eq!(substitutions, 9);
    }

    #[test]
    fn corrupted_load_plan_relations_fail_closed() {
        let corruptions: [fn(&mut M1KernelArtifactEntryV1); 6] = [
            |entry: &mut M1KernelArtifactEntryV1| entry.load_plan.input_len += 1,
            |entry: &mut M1KernelArtifactEntryV1| entry.load_plan.metadata_len = u64::MAX,
            |entry: &mut M1KernelArtifactEntryV1| entry.load_plan.image_start += 1,
            |entry: &mut M1KernelArtifactEntryV1| entry.load_plan.segments[0].mapping_size = 1,
            |entry: &mut M1KernelArtifactEntryV1| entry.load_plan.segments[1].permissions = 1,
            |entry: &mut M1KernelArtifactEntryV1| entry.load_plan.segments[1].file_offset = 128,
        ];
        for corrupt in corruptions {
            let mut entries = fixture_entries();
            corrupt(&mut entries[0]);
            assert_eq!(
                M1KernelArtifactManifestV1::new(entries).unwrap_err(),
                M1KernelArtifactManifestErrorV1::Invalid("load plan")
            );
        }
    }
}
