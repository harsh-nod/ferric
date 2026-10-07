//! Authenticated identity join for the complete M1 generated plan and artifacts.
//!
//! The public constructor consumes both the published generated-runner
//! declaration and the live seven-family Worker-evidence owner. Persisted
//! manifest bytes, catalog digests, and caller-supplied [`Identity`] values
//! cannot construct this authority.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

use fe2o3_hsaco_finalize::ContentIdentityV1;
use ferric_qwen_kernels::{gemm, logits, paged_decode, prefill, rmsnorm, rope_kv, swiglu};
use ferric_spec::{Identity, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket};

use super::{
    current_m1_kernel_source_facts_v1, decode_m1_kernel_artifact_manifest_v1, hash_field,
    runner::validate_published_qwen3_gfx942_runner_declaration, sha256::Sha256,
    BuiltAndInspectedM1KernelArtifactsV1, GeneratedRunnerError, M1KernelArtifactBuildErrorV1,
    M1KernelArtifactEntryV1, M1KernelArtifactFamilyV1, M1KernelArtifactManifestErrorV1,
    M1KernelArtifactManifestV1, M1KernelArtifactProgramV1, M1KernelProfileCatalogV1,
    PublishedRunnerDeclaration, M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1, M1_PHYSICAL_PROGRAM_COUNT_V1,
};

/// Canonical format version for the final authenticated executable-plan join.
pub const AUTHENTICATED_M1_EXECUTABLE_PLAN_IDENTITY_VERSION_V1: u32 = 1;

const ARTIFACT_CATALOG_DOMAIN: &[u8] = b"ferric.m1.executable-artifact-catalog.v1";
const EXECUTABLE_PLAN_DOMAIN: &[u8] = b"ferric.m1.authenticated-executable-plan-identity.v1";

#[derive(Clone, Debug, Eq, PartialEq)]
struct ProfileIdentityFactV1 {
    name: String,
    profile_count: u32,
    identity: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ProgramIdentityFactV1 {
    kernel_symbol: String,
    descriptor_symbol: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ArtifactFamilyIdentityFactsV1 {
    family: M1KernelArtifactFamilyV1,
    artifact: ContentIdentityV1,
    compiler_module: ContentIdentityV1,
    compiler_handoff: ContentIdentityV1,
    symbol_manifest: ContentIdentityV1,
    profiles: Vec<ProfileIdentityFactV1>,
    programs: Vec<ProgramIdentityFactV1>,
}

/// Move-only authenticated identity custody for one exact M1 executable plan.
///
/// The owner retains the authenticated target/draft bundle, tokenizer/model
/// identities, all 22 generated plans, the exact generated source and
/// declaration, and the live inspected K1-K7 artifact set. It is identity
/// authority only: it grants no allocation, load, queue, launch, completion,
/// proof, hardware, performance, or qualification authority.
///
/// Caller-supplied digest values cannot satisfy the constructor's move-only
/// authority inputs:
///
/// ```compile_fail
/// use ferric_build::{
///     authenticate_m1_executable_plan_identity_v1,
///     AuthenticatedM1ExecutablePlanIdentityV1,
/// };
/// use ferric_spec::Identity;
///
/// fn forge(raw_digest: Identity) -> AuthenticatedM1ExecutablePlanIdentityV1 {
///     authenticate_m1_executable_plan_identity_v1(raw_digest, raw_digest).unwrap()
/// }
/// ```
#[derive(Debug)]
pub struct AuthenticatedM1ExecutablePlanIdentityV1 {
    runner: PublishedRunnerDeclaration,
    artifacts: BuiltAndInspectedM1KernelArtifactsV1,
    artifact_manifest_identity: ContentIdentityV1,
    artifact_catalog_id: Identity,
    executable_plan_id: Identity,
    canonical_bytes: Box<[u8]>,
}

impl AuthenticatedM1ExecutablePlanIdentityV1 {
    /// Retained exact published generated-runner declaration.
    #[must_use]
    pub const fn runner(&self) -> &PublishedRunnerDeclaration {
        &self.runner
    }

    /// Retained live seven-family inspected artifact custody.
    #[must_use]
    pub const fn artifacts(&self) -> &BuiltAndInspectedM1KernelArtifactsV1 {
        &self.artifacts
    }

    /// Exact canonical seven-family artifact-manifest content identity.
    #[must_use]
    pub const fn artifact_manifest_identity(&self) -> ContentIdentityV1 {
        self.artifact_manifest_identity
    }

    /// Domain-separated identity of the exact ordered profile/program catalog.
    #[must_use]
    pub const fn artifact_catalog_id(&self) -> Identity {
        self.artifact_catalog_id
    }

    /// Domain-separated complete executable-plan identity.
    #[must_use]
    pub const fn executable_plan_id(&self) -> Identity {
        self.executable_plan_id
    }

    /// Complete canonical record hashed by [`Self::executable_plan_id`].
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    /// Exact authenticated executable-plan identity format version.
    #[must_use]
    pub const fn version(&self) -> u32 {
        AUTHENTICATED_M1_EXECUTABLE_PLAN_IDENTITY_VERSION_V1
    }

    /// This build-time identity owner does not prove device execution.
    #[must_use]
    pub const fn proves_hardware_execution(&self) -> bool {
        false
    }

    /// This build-time identity owner does not grant HSA loading authority.
    #[must_use]
    pub const fn grants_hsa_load_authority(&self) -> bool {
        false
    }
}

/// Fail-closed final executable-plan identity construction error.
#[derive(Debug)]
pub enum M1ExecutablePlanIdentityErrorV1 {
    /// The retained published generated declaration no longer validates.
    Runner(GeneratedRunnerError),
    /// The retained manifest no longer decodes as its unique canonical form.
    ArtifactManifest(M1KernelArtifactManifestErrorV1),
    /// Current Ferric source facts could not be reconstructed.
    CurrentKernelSource(M1KernelArtifactBuildErrorV1),
    /// A manifest field differs from current Ferric source.
    CurrentKernelSourceDrift(M1KernelArtifactFamilyV1),
    /// The artifact family roster is incomplete or has trailing entries.
    ArtifactFamilyCount {
        /// Required exact count.
        expected: usize,
        /// Observed count.
        actual: usize,
    },
    /// One artifact family is missing, duplicated, or reordered.
    ArtifactFamilyOrder {
        /// Zero-based manifest position.
        index: usize,
        /// Required family at this position.
        expected: M1KernelArtifactFamilyV1,
        /// Observed family.
        actual: M1KernelArtifactFamilyV1,
    },
    /// Two family slots reuse one artifact content identity.
    DuplicateArtifactIdentity(M1KernelArtifactFamilyV1),
    /// A profile catalog is missing, duplicated, reordered, or stale.
    ProfileCatalogDrift(M1KernelArtifactFamilyV1),
    /// A physical program is missing, duplicated, reordered, or stale.
    ProgramRosterDrift(M1KernelArtifactFamilyV1),
    /// The complete physical-program count differs from exactly 12.
    PhysicalProgramCount {
        /// Required exact count.
        expected: usize,
        /// Observed count.
        actual: usize,
    },
    /// Two physical programs reuse a kernel or descriptor symbol.
    DuplicateProgramSymbol(M1KernelArtifactFamilyV1),
    /// A live inspected artifact owner differs from its manifest entry.
    ArtifactOwnerIdentityDrift(M1KernelArtifactFamilyV1),
    /// A live inspected artifact load plan differs from its manifest entry.
    ArtifactOwnerLoadPlanDrift(M1KernelArtifactFamilyV1),
    /// The preliminary caller assertion for the executable catalog differs
    /// from the catalog derived from live artifact custody.
    ExecutableCatalogIdentityDrift,
    /// A retained or recomputed final identity field differs.
    ExecutablePlanIdentityDrift,
}

impl fmt::Display for M1ExecutablePlanIdentityErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "M1 executable-plan identity join failed: {self:?}"
        )
    }
}

impl Error for M1ExecutablePlanIdentityErrorV1 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ArtifactManifest(source) => Some(source),
            Self::CurrentKernelSource(source) => Some(source),
            _ => None,
        }
    }
}

/// Derives the exact ordered artifact/profile/program catalog identity.
///
/// This helper returns data only. A digest obtained here cannot construct
/// [`AuthenticatedM1ExecutablePlanIdentityV1`]; the authority constructor also
/// consumes live inspected Worker custody.
///
/// # Errors
///
/// Returns [`M1ExecutablePlanIdentityErrorV1`] for any noncanonical manifest,
/// current-source mismatch, missing family or program, duplicate, or reordering.
pub fn expected_m1_executable_artifact_catalog_identity_v1(
    manifest: &M1KernelArtifactManifestV1,
) -> Result<Identity, M1ExecutablePlanIdentityErrorV1> {
    let facts = validate_manifest_and_collect_facts(manifest)?;
    let bytes = artifact_catalog_record(manifest.identity(), &facts);
    Ok(identity_record(ARTIFACT_CATALOG_DOMAIN, &bytes))
}

/// Consumes the exact runner declaration and live inspected K1-K7 artifacts.
///
/// No caller-supplied digest or identity is accepted. The preliminary
/// `executable_catalog` assertion retained by the runner must equal the catalog
/// independently derived here from the live artifact owner's canonical
/// manifest. Success retains both move-only owners in one final identity join.
///
/// # Errors
///
/// Returns [`M1ExecutablePlanIdentityErrorV1`] for any bundle, plan, generated
/// declaration, source, manifest, current-kernel-source, live-owner, catalog,
/// family, profile, program, ordering, duplicate, or identity drift.
pub fn authenticate_m1_executable_plan_identity_v1(
    runner: PublishedRunnerDeclaration,
    artifacts: BuiltAndInspectedM1KernelArtifactsV1,
) -> Result<AuthenticatedM1ExecutablePlanIdentityV1, M1ExecutablePlanIdentityErrorV1> {
    let manifest = artifacts.manifest();
    let (facts, artifact_manifest_identity, artifact_catalog_id) =
        validate_runner_manifest_join(&runner, manifest)?;
    validate_live_artifact_owners(&artifacts)?;
    let canonical_bytes = executable_plan_record(
        &runner,
        artifact_manifest_identity,
        artifact_catalog_id,
        &facts,
    );
    let executable_plan_id = identity_record(EXECUTABLE_PLAN_DOMAIN, &canonical_bytes);
    let authority = AuthenticatedM1ExecutablePlanIdentityV1 {
        runner,
        artifacts,
        artifact_manifest_identity,
        artifact_catalog_id,
        executable_plan_id,
        canonical_bytes: canonical_bytes.into_boxed_slice(),
    };
    validate_authenticated_m1_executable_plan_identity_v1(&authority)?;
    Ok(authority)
}

/// Revalidates every retained field of a final executable-plan identity join.
///
/// # Errors
///
/// Returns [`M1ExecutablePlanIdentityErrorV1`] for any retained authority or
/// canonical identity drift.
pub fn validate_authenticated_m1_executable_plan_identity_v1(
    authority: &AuthenticatedM1ExecutablePlanIdentityV1,
) -> Result<(), M1ExecutablePlanIdentityErrorV1> {
    let (facts, manifest_identity, catalog_id) =
        validate_runner_manifest_join(&authority.runner, authority.artifacts.manifest())?;
    validate_live_artifact_owners(&authority.artifacts)?;
    let canonical_bytes =
        executable_plan_record(&authority.runner, manifest_identity, catalog_id, &facts);
    if authority.version() != AUTHENTICATED_M1_EXECUTABLE_PLAN_IDENTITY_VERSION_V1
        || authority.artifact_manifest_identity != manifest_identity
        || authority.artifact_catalog_id != catalog_id
        || authority.canonical_bytes.as_ref() != canonical_bytes
        || authority.executable_plan_id != identity_record(EXECUTABLE_PLAN_DOMAIN, &canonical_bytes)
    {
        return Err(M1ExecutablePlanIdentityErrorV1::ExecutablePlanIdentityDrift);
    }
    Ok(())
}

fn validate_runner_manifest_join(
    runner: &PublishedRunnerDeclaration,
    manifest: &M1KernelArtifactManifestV1,
) -> Result<
    (
        Vec<ArtifactFamilyIdentityFactsV1>,
        ContentIdentityV1,
        Identity,
    ),
    M1ExecutablePlanIdentityErrorV1,
> {
    validate_published_qwen3_gfx942_runner_declaration(runner)
        .map_err(M1ExecutablePlanIdentityErrorV1::Runner)?;
    let facts = validate_manifest_and_collect_facts(manifest)?;
    let manifest_identity = manifest.identity();
    let catalog_bytes = artifact_catalog_record(manifest_identity, &facts);
    let catalog_id = identity_record(ARTIFACT_CATALOG_DOMAIN, &catalog_bytes);
    if runner.declaration().closure().external().executable_catalog != catalog_id {
        return Err(M1ExecutablePlanIdentityErrorV1::ExecutableCatalogIdentityDrift);
    }
    Ok((facts, manifest_identity, catalog_id))
}

fn validate_manifest_and_collect_facts(
    manifest: &M1KernelArtifactManifestV1,
) -> Result<Vec<ArtifactFamilyIdentityFactsV1>, M1ExecutablePlanIdentityErrorV1> {
    let decoded = decode_m1_kernel_artifact_manifest_v1(manifest.canonical_bytes())
        .map_err(M1ExecutablePlanIdentityErrorV1::ArtifactManifest)?;
    if decoded != *manifest || !manifest.identity().matches(manifest.canonical_bytes()) {
        return Err(M1ExecutablePlanIdentityErrorV1::ExecutablePlanIdentityDrift);
    }
    let facts = manifest
        .entries()
        .iter()
        .map(family_facts)
        .collect::<Vec<_>>();
    validate_artifact_facts(&facts)?;
    validate_current_source(&facts)?;
    Ok(facts)
}

fn family_facts(entry: &M1KernelArtifactEntryV1) -> ArtifactFamilyIdentityFactsV1 {
    ArtifactFamilyIdentityFactsV1 {
        family: entry.family(),
        artifact: entry.artifact(),
        compiler_module: entry.compiler_module(),
        compiler_handoff: entry.compiler_handoff(),
        symbol_manifest: entry.symbol_manifest(),
        profiles: entry.profile_catalogs().iter().map(profile_fact).collect(),
        programs: entry.programs().iter().map(program_fact).collect(),
    }
}

fn profile_fact(profile: &M1KernelProfileCatalogV1) -> ProfileIdentityFactV1 {
    ProfileIdentityFactV1 {
        name: profile.name().to_owned(),
        profile_count: profile.profile_count(),
        identity: *profile.identity(),
    }
}

fn program_fact(program: &M1KernelArtifactProgramV1) -> ProgramIdentityFactV1 {
    ProgramIdentityFactV1 {
        kernel_symbol: program.kernel_symbol().to_owned(),
        descriptor_symbol: program.descriptor_symbol().to_owned(),
    }
}

fn validate_artifact_facts(
    facts: &[ArtifactFamilyIdentityFactsV1],
) -> Result<(), M1ExecutablePlanIdentityErrorV1> {
    if facts.len() != M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1 {
        return Err(M1ExecutablePlanIdentityErrorV1::ArtifactFamilyCount {
            expected: M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1,
            actual: facts.len(),
        });
    }
    let mut artifacts = BTreeSet::new();
    let mut profile_ids = BTreeSet::new();
    let mut kernel_symbols = BTreeSet::new();
    let mut descriptor_symbols = BTreeSet::new();
    let mut program_count = 0;
    for (index, (fact, expected_family)) in
        facts.iter().zip(M1KernelArtifactFamilyV1::ALL).enumerate()
    {
        if fact.family != expected_family {
            return Err(M1ExecutablePlanIdentityErrorV1::ArtifactFamilyOrder {
                index,
                expected: expected_family,
                actual: fact.family,
            });
        }
        if !artifacts.insert((fact.artifact.sha256().to_owned(), fact.artifact.byte_len())) {
            return Err(M1ExecutablePlanIdentityErrorV1::DuplicateArtifactIdentity(
                fact.family,
            ));
        }
        for identity in [
            fact.artifact,
            fact.compiler_module,
            fact.compiler_handoff,
            fact.symbol_manifest,
        ] {
            if identity.byte_len() == 0 || identity.sha256() == &[0; 32] {
                return Err(M1ExecutablePlanIdentityErrorV1::CurrentKernelSourceDrift(
                    fact.family,
                ));
            }
        }
        if fact.profiles.is_empty()
            || fact.profiles.len() != expected_profiles(fact.family).len()
            || fact
                .profiles
                .iter()
                .zip(expected_profiles(fact.family))
                .any(|(actual, expected)| {
                    actual.name != expected.0
                        || usize::try_from(actual.profile_count).ok() != Some(expected.1)
                })
            || fact.profiles.iter().any(|profile| {
                profile.name.is_empty()
                    || profile.profile_count == 0
                    || profile.identity == [0; 32]
                    || !profile_ids.insert(profile.identity)
            })
        {
            return Err(M1ExecutablePlanIdentityErrorV1::ProfileCatalogDrift(
                fact.family,
            ));
        }
        let expected_programs = expected_programs(fact.family);
        if fact.programs.len() != expected_programs.len()
            || fact
                .programs
                .iter()
                .zip(expected_programs)
                .any(|(actual, expected)| {
                    actual.kernel_symbol != expected.0 || actual.descriptor_symbol != expected.1
                })
        {
            return Err(M1ExecutablePlanIdentityErrorV1::ProgramRosterDrift(
                fact.family,
            ));
        }
        for program in &fact.programs {
            if !kernel_symbols.insert(program.kernel_symbol.as_str())
                || !descriptor_symbols.insert(program.descriptor_symbol.as_str())
            {
                return Err(M1ExecutablePlanIdentityErrorV1::DuplicateProgramSymbol(
                    fact.family,
                ));
            }
        }
        program_count += fact.programs.len();
    }
    if program_count != M1_PHYSICAL_PROGRAM_COUNT_V1 {
        return Err(M1ExecutablePlanIdentityErrorV1::PhysicalProgramCount {
            expected: M1_PHYSICAL_PROGRAM_COUNT_V1,
            actual: program_count,
        });
    }
    Ok(())
}

fn validate_current_source(
    facts: &[ArtifactFamilyIdentityFactsV1],
) -> Result<(), M1ExecutablePlanIdentityErrorV1> {
    let current = current_m1_kernel_source_facts_v1()
        .map_err(M1ExecutablePlanIdentityErrorV1::CurrentKernelSource)?;
    for ((fact, source), expected_family) in facts
        .iter()
        .zip(&current)
        .zip(M1KernelArtifactFamilyV1::ALL)
    {
        let source_profiles = source
            .profile_catalogs()
            .iter()
            .map(profile_fact)
            .collect::<Vec<_>>();
        if fact.family != expected_family
            || source.family() != expected_family
            || fact.compiler_module != source.compiler_module()
            || fact.compiler_handoff != source.compiler_handoff()
            || fact.symbol_manifest != source.symbol_manifest()
            || fact.profiles != source_profiles
        {
            return Err(M1ExecutablePlanIdentityErrorV1::CurrentKernelSourceDrift(
                expected_family,
            ));
        }
    }
    Ok(())
}

fn validate_live_artifact_owners(
    artifacts: &BuiltAndInspectedM1KernelArtifactsV1,
) -> Result<(), M1ExecutablePlanIdentityErrorV1> {
    let identities = [
        ContentIdentityV1::calculate(artifacts.gemm().exact_worker_output_bytes()),
        ContentIdentityV1::calculate(artifacts.rmsnorm().exact_worker_output_bytes()),
        ContentIdentityV1::calculate(artifacts.rope_kv().exact_worker_output_bytes()),
        ContentIdentityV1::calculate(artifacts.prefill().exact_worker_output_bytes()),
        ContentIdentityV1::calculate(artifacts.paged_decode().exact_worker_output_bytes()),
        ContentIdentityV1::calculate(artifacts.swiglu().exact_worker_output_bytes()),
        ContentIdentityV1::calculate(artifacts.logits().exact_worker_output_bytes()),
    ];
    let load_plans = [
        artifacts.gemm().loader_plan(),
        artifacts.rmsnorm().loader_plan(),
        artifacts.rope_kv().loader_plan(),
        artifacts.prefill().loader_plan(),
        artifacts.paged_decode().loader_plan(),
        artifacts.swiglu().loader_plan(),
        artifacts.logits().loader_plan(),
    ];
    for (((entry, identity), load_plan), family) in artifacts
        .manifest()
        .entries()
        .iter()
        .zip(identities)
        .zip(load_plans)
        .zip(M1KernelArtifactFamilyV1::ALL)
    {
        if entry.family() != family || entry.artifact() != identity {
            return Err(M1ExecutablePlanIdentityErrorV1::ArtifactOwnerIdentityDrift(
                family,
            ));
        }
        if !entry.matches_validated_load_plan(load_plan) {
            return Err(M1ExecutablePlanIdentityErrorV1::ArtifactOwnerLoadPlanDrift(
                family,
            ));
        }
    }
    Ok(())
}

fn artifact_catalog_record(
    manifest_identity: ContentIdentityV1,
    facts: &[ArtifactFamilyIdentityFactsV1],
) -> Vec<u8> {
    let mut record = Vec::with_capacity(2_048);
    record.extend_from_slice(&AUTHENTICATED_M1_EXECUTABLE_PLAN_IDENTITY_VERSION_V1.to_le_bytes());
    push_content_identity(&mut record, manifest_identity);
    push_u64(&mut record, facts.len());
    for fact in facts {
        record.push(fact.family as u8);
        for identity in [
            fact.artifact,
            fact.compiler_module,
            fact.compiler_handoff,
            fact.symbol_manifest,
        ] {
            push_content_identity(&mut record, identity);
        }
        push_u64(&mut record, fact.profiles.len());
        for profile in &fact.profiles {
            push_bytes(&mut record, profile.name.as_bytes());
            record.extend_from_slice(&profile.profile_count.to_le_bytes());
            record.extend_from_slice(&profile.identity);
        }
        push_u64(&mut record, fact.programs.len());
        for program in &fact.programs {
            push_bytes(&mut record, program.kernel_symbol.as_bytes());
            push_bytes(&mut record, program.descriptor_symbol.as_bytes());
        }
    }
    record
}

fn executable_plan_record(
    runner: &PublishedRunnerDeclaration,
    manifest_identity: ContentIdentityV1,
    artifact_catalog_id: Identity,
    facts: &[ArtifactFamilyIdentityFactsV1],
) -> Vec<u8> {
    let declaration = runner.declaration();
    let closure = declaration.closure();
    let catalog = closure.catalog();
    let deployment = catalog.deployment();
    let external = closure.external();
    let mut record = Vec::with_capacity(4_096);
    record.extend_from_slice(&AUTHENTICATED_M1_EXECUTABLE_PLAN_IDENTITY_VERSION_V1.to_le_bytes());
    for identity in [
        runner.admission_record_id(),
        runner.bundle_id(),
        deployment.target_model.config.model_id,
        deployment.target_model.config.config_id,
        deployment.target_model.tokenizer.tokenizer_id,
        deployment.target_model.tokenizer.vocabulary_id,
        deployment.target_model.weights.weights_id,
        deployment.draft_model.config.model_id,
        deployment.draft_model.config.config_id,
        deployment.draft_model.tokenizer.tokenizer_id,
        deployment.draft_model.tokenizer.vocabulary_id,
        deployment.draft_model.weights.weights_id,
        runner.target_prepacked_id(),
        runner.draft_prepacked_id(),
        runner.plan_catalog_id(),
    ] {
        push_identity(&mut record, identity);
    }
    push_u64(&mut record, runner.plans().len());
    for plan in runner.plans() {
        record.extend_from_slice(&plan.plan_index.to_le_bytes());
        push_identity(&mut record, plan.plan_id);
        encode_selection(
            &mut record,
            plan.selection.role,
            plan.selection.mode,
            plan.selection.bucket,
        );
        record.extend_from_slice(&plan.operation_start.to_le_bytes());
        record.extend_from_slice(&plan.operation_count.to_le_bytes());
    }
    for identity in [
        runner.kernel_catalog_id(),
        runner.closure_id(),
        runner.source_id(),
        runner.declaration_id(),
        external.ferric_source,
        external.fe2o3_source,
        external.compiler,
        external.compiler_configuration,
        external.target_contract,
        external.kernel_catalog,
        external.kernel_proof_set,
        external.kernel_abi_catalog,
        external.executable_catalog,
        external.runtime_contract,
        external.runtime_abi,
        external.generated_runner,
        external.validator_registry,
        external.qualification_protocol,
        external.tcb_report,
    ] {
        push_identity(&mut record, identity);
    }
    push_content_identity(&mut record, manifest_identity);
    push_identity(&mut record, artifact_catalog_id);
    let catalog_record = artifact_catalog_record(manifest_identity, facts);
    push_bytes(&mut record, &catalog_record);
    record
}

fn expected_programs(family: M1KernelArtifactFamilyV1) -> &'static [(&'static str, &'static str)] {
    match family {
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
    }
}

fn expected_profiles(family: M1KernelArtifactFamilyV1) -> &'static [(&'static str, usize)] {
    match family {
        M1KernelArtifactFamilyV1::Gemm => &[
            ("gemm", gemm::QWEN3_GEMM_PROFILE_COUNT_V1),
            (
                "token-embedding",
                gemm::QWEN3_TOKEN_EMBEDDING_PROFILE_COUNT_V1,
            ),
        ],
        M1KernelArtifactFamilyV1::RmsNorm => {
            &[("rmsnorm", rmsnorm::QWEN3_RMSNORM_PROFILE_COUNT_V1)]
        }
        M1KernelArtifactFamilyV1::RopeKv => &[("rope-kv", rope_kv::QWEN3_ROPE_KV_PROFILE_COUNT_V1)],
        M1KernelArtifactFamilyV1::Prefill => {
            &[("prefill", prefill::QWEN3_PREFILL_PROFILE_COUNT_V1)]
        }
        M1KernelArtifactFamilyV1::PagedDecode => &[(
            "paged-decode",
            paged_decode::QWEN3_PAGED_DECODE_PROFILE_COUNT_V1,
        )],
        M1KernelArtifactFamilyV1::SwiGlu => &[("swiglu", swiglu::QWEN3_SWIGLU_PROFILE_COUNT_V1)],
        M1KernelArtifactFamilyV1::Logits => &[
            ("logits", logits::QWEN3_LOGITS_PROFILE_COUNT_V1),
            (
                "speculative-token-assembly",
                logits::QWEN3_SPECULATIVE_TOKEN_ASSEMBLY_PROFILE_COUNT_V1,
            ),
        ],
    }
}

fn encode_selection(
    record: &mut Vec<u8>,
    role: Qwen3ModelRole,
    mode: Qwen3ExecutionMode,
    bucket: Qwen3PlanBucket,
) {
    record.push(match role {
        Qwen3ModelRole::Target8B => 1,
        Qwen3ModelRole::Draft06B => 2,
    });
    record.push(match mode {
        Qwen3ExecutionMode::Prefill => 1,
        Qwen3ExecutionMode::Decode => 2,
        Qwen3ExecutionMode::Speculative => 3,
    });
    record.push(match bucket {
        Qwen3PlanBucket::PrefillS1T128 => 1,
        Qwen3PlanBucket::PrefillS8T128 => 2,
        Qwen3PlanBucket::PrefillS1T512 => 3,
        Qwen3PlanBucket::PrefillS1T2048 => 4,
        Qwen3PlanBucket::DecodeS1C8192 => 5,
        Qwen3PlanBucket::DecodeS8C8192 => 6,
        Qwen3PlanBucket::DecodeS32C8192 => 7,
        Qwen3PlanBucket::SpeculativeS1K4C8192 => 8,
        Qwen3PlanBucket::SpeculativeS8K4C8192 => 9,
        Qwen3PlanBucket::SpeculativeS1K8C8192 => 10,
        Qwen3PlanBucket::SpeculativeS1K16C8192 => 11,
    });
}

fn push_identity(record: &mut Vec<u8>, identity: Identity) {
    record.extend_from_slice(identity.as_bytes());
}

fn push_content_identity(record: &mut Vec<u8>, identity: ContentIdentityV1) {
    record.extend_from_slice(identity.sha256());
    record.extend_from_slice(&identity.byte_len().to_le_bytes());
}

fn push_u64(record: &mut Vec<u8>, value: usize) {
    record.extend_from_slice(
        &u64::try_from(value)
            .expect("bounded M1 catalog count fits u64")
            .to_le_bytes(),
    );
}

fn push_bytes(record: &mut Vec<u8>, bytes: &[u8]) {
    push_u64(record, bytes.len());
    record.extend_from_slice(bytes);
}

fn identity_record(domain: &[u8], bytes: &[u8]) -> Identity {
    let mut hasher = Sha256::new();
    hash_field(&mut hasher, domain);
    hash_field(&mut hasher, bytes);
    Identity::new(hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn publication(executable_catalog: Identity) -> PublishedRunnerDeclaration {
        let closure = crate::runner::qwen3_runner_closure_test_fixture_with_executable_catalog(
            executable_catalog,
        );
        let declaration = crate::generate_qwen3_gfx942_runner_declaration(closure)
            .expect("exact generated declaration");
        crate::publish_qwen3_gfx942_runner_declaration(declaration)
            .expect("exact published declaration")
    }

    fn facts() -> Vec<ArtifactFamilyIdentityFactsV1> {
        let manifest =
            crate::kernel_artifact_manifest::m1_kernel_artifact_manifest_unit_fixture_v1();
        manifest.entries().iter().map(family_facts).collect()
    }

    #[test]
    fn exact_artifact_catalog_is_complete_deterministic_and_domain_separated() {
        let manifest =
            crate::kernel_artifact_manifest::m1_kernel_artifact_manifest_unit_fixture_v1();
        let first = expected_m1_executable_artifact_catalog_identity_v1(&manifest).unwrap();
        let second = expected_m1_executable_artifact_catalog_identity_v1(&manifest).unwrap();
        assert_eq!(first, second);
        assert!(first.is_present());
        assert_ne!(first.as_bytes(), manifest.identity().sha256());
        assert_eq!(manifest.entries().len(), M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1);
        assert_eq!(
            manifest
                .entries()
                .iter()
                .map(|entry| entry.programs().len())
                .sum::<usize>(),
            M1_PHYSICAL_PROGRAM_COUNT_V1
        );
    }

    #[test]
    fn published_runner_joins_only_to_the_exact_derived_artifact_catalog() {
        let manifest =
            crate::kernel_artifact_manifest::m1_kernel_artifact_manifest_unit_fixture_v1();
        let catalog_id = expected_m1_executable_artifact_catalog_identity_v1(&manifest).unwrap();
        let runner = publication(catalog_id);
        let (facts, manifest_id, joined_catalog_id) =
            validate_runner_manifest_join(&runner, &manifest).unwrap();
        assert_eq!(facts.len(), M1_KERNEL_ARTIFACT_FAMILY_COUNT_V1);
        assert_eq!(manifest_id, manifest.identity());
        assert_eq!(joined_catalog_id, catalog_id);

        let record = executable_plan_record(&runner, manifest_id, catalog_id, &facts);
        assert!(!record.is_empty());
        assert!(identity_record(EXECUTABLE_PLAN_DOMAIN, &record).is_present());
    }

    #[test]
    fn caller_asserted_catalog_digest_cannot_substitute_for_artifact_derived_identity() {
        let manifest =
            crate::kernel_artifact_manifest::m1_kernel_artifact_manifest_unit_fixture_v1();
        let runner = publication(Identity::new([0xa5; 32]));
        assert!(matches!(
            validate_runner_manifest_join(&runner, &manifest),
            Err(M1ExecutablePlanIdentityErrorV1::ExecutableCatalogIdentityDrift)
        ));
    }

    #[test]
    fn missing_duplicate_and_reordered_family_inputs_fail_closed() {
        let exact = facts();

        let mut missing = exact.clone();
        missing.pop();
        assert!(matches!(
            validate_artifact_facts(&missing),
            Err(M1ExecutablePlanIdentityErrorV1::ArtifactFamilyCount { .. })
        ));

        let mut reordered = exact.clone();
        reordered.swap(0, 1);
        assert!(matches!(
            validate_artifact_facts(&reordered),
            Err(M1ExecutablePlanIdentityErrorV1::ArtifactFamilyOrder { index: 0, .. })
        ));

        let mut duplicate = exact;
        duplicate[1].artifact = duplicate[0].artifact;
        assert_eq!(
            validate_artifact_facts(&duplicate).unwrap_err().to_string(),
            M1ExecutablePlanIdentityErrorV1::DuplicateArtifactIdentity(
                M1KernelArtifactFamilyV1::RmsNorm
            )
            .to_string()
        );
    }

    #[test]
    fn missing_duplicate_reordered_and_drifted_program_inputs_fail_closed() {
        let mut missing = facts();
        missing[0].programs.pop();
        assert!(matches!(
            validate_artifact_facts(&missing),
            Err(M1ExecutablePlanIdentityErrorV1::ProgramRosterDrift(
                M1KernelArtifactFamilyV1::Gemm
            ))
        ));

        let mut reordered = facts();
        reordered[0].programs.swap(0, 1);
        assert!(matches!(
            validate_artifact_facts(&reordered),
            Err(M1ExecutablePlanIdentityErrorV1::ProgramRosterDrift(
                M1KernelArtifactFamilyV1::Gemm
            ))
        ));

        let mut duplicate = facts();
        duplicate[0].programs[1] = duplicate[0].programs[0].clone();
        assert!(matches!(
            validate_artifact_facts(&duplicate),
            Err(M1ExecutablePlanIdentityErrorV1::ProgramRosterDrift(
                M1KernelArtifactFamilyV1::Gemm
            ))
        ));

        let mut drifted = facts();
        drifted[6].programs[2].kernel_symbol.push('x');
        assert!(matches!(
            validate_artifact_facts(&drifted),
            Err(M1ExecutablePlanIdentityErrorV1::ProgramRosterDrift(
                M1KernelArtifactFamilyV1::Logits
            ))
        ));
    }

    #[test]
    fn missing_duplicate_reordered_and_drifted_profile_inputs_fail_closed() {
        let mut missing = facts();
        missing[0].profiles.pop();
        assert!(matches!(
            validate_artifact_facts(&missing),
            Err(M1ExecutablePlanIdentityErrorV1::ProfileCatalogDrift(_))
        ));

        let mut duplicate = facts();
        duplicate[1].profiles[0].identity = duplicate[0].profiles[0].identity;
        assert!(matches!(
            validate_artifact_facts(&duplicate),
            Err(M1ExecutablePlanIdentityErrorV1::ProfileCatalogDrift(_))
        ));

        let mut reordered = facts();
        reordered[0].profiles.swap(0, 1);
        assert!(matches!(
            validate_artifact_facts(&reordered),
            Err(M1ExecutablePlanIdentityErrorV1::ProfileCatalogDrift(
                M1KernelArtifactFamilyV1::Gemm
            ))
        ));

        let mut drifted = facts();
        drifted[6].profiles[0].identity[0] ^= 1;
        assert!(matches!(
            validate_current_source(&drifted),
            Err(M1ExecutablePlanIdentityErrorV1::CurrentKernelSourceDrift(
                M1KernelArtifactFamilyV1::Logits
            ))
        ));
    }

    #[test]
    fn raw_identity_values_cannot_construct_final_authority() {
        assert!(!std::mem::needs_drop::<Identity>());
        assert!(std::mem::needs_drop::<
            AuthenticatedM1ExecutablePlanIdentityV1,
        >());
        assert!(std::mem::needs_drop::<PublishedRunnerDeclaration>());
        assert!(std::mem::needs_drop::<BuiltAndInspectedM1KernelArtifactsV1>());
    }
}
