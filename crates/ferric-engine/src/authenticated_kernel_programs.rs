//! Ferric-owned intake for the exact aggregate M1 Worker V3 roster.
//!
//! Every marker and the roster type come from the same selected compiler unit.
//! Nothing in this module constructs authentication authority from persisted
//! bytes or reopens a raw HSACO file.

use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

use fe2o3_amd_target::AmdTargetId;
use fe2o3_host::{
    AuthenticatedWorkerV3ProgramLookupErrorV1, AuthenticatedWorkerV3ProgramSetAdmissionErrorV1,
    AuthenticatedWorkerV3ProgramSetV1, AuthenticatedWorkerV3RosterV1,
    CompilerGeneratedKernelExpectationRosterV1, CompilerGeneratedKernelExpectationV1,
    RecoveredWorkerV3AdmissionErrorV1,
};
use ferric_build::M1KernelArtifactFamilyV1;
use ferric_kernels::KernelFamily;
use ferric_qwen3_all_kernels_device_v1::{
    gemm::{
        ferric_qwen3_gemm_mfma_bf16_f32_bf16_v1_gpu::Marker as GemmMfmaMarkerV1,
        ferric_qwen3_gemm_reference_bf16_f32_bf16_v1_gpu::Marker as GemmReferenceMarkerV1,
        ferric_qwen3_gemm_vector_a4_bf16_f32_bf16_v1_gpu::Marker as GemmVectorizedMarkerV1,
        ferric_qwen3_token_embedding_bf16_copy_v1_gpu::Marker as TokenEmbeddingMarkerV1,
    },
    logits::{
        ferric_qwen3_compact_completion_v1_gpu::Marker as LogitsCompactMarkerV1,
        ferric_qwen3_lowest_id_argmax_bf16_v1_gpu::Marker as LogitsArgmaxMarkerV1,
        ferric_qwen3_speculative_token_assembly_v1_gpu::Marker as SpeculativeAssemblyMarkerV1,
    },
    paged_decode::qwen3_paged_gqa_decode_bf16_f32_v1_gpu::Marker as PagedDecodeMarkerV1,
    prefill::qwen3_gqa_prefill_causal_bf16_f32_v1_gpu::Marker as PrefillMarkerV1,
    rmsnorm::qwen3_rmsnorm_v1_gpu::Marker as RmsNormMarkerV1,
    rope_kv::{
        qwen3_paged_kv_write_v1_gpu::Marker as PagedKvWriteMarkerV1,
        qwen3_rope_v1_gpu::Marker as RopeMarkerV1,
    },
    swiglu::qwen3_swiglu_bf16_f32_v1_gpu::Marker as SwiGluMarkerV1,
    M1AllKernelsMfmaWorkerV3RosterV1, M1AllKernelsWorkerV3RosterV1,
};
use ferric_spec::Identity;
use sha2::{Digest, Sha256};

use crate::{
    DeclaredKernelFamilyArtifact, M1PhysicalProgramStrategyV1, M1PhysicalProgramV1,
    M1_MFMA_PHYSICAL_PROGRAM_COUNT_V1, M1_PHYSICAL_PROGRAM_COUNT_V1,
};

const M1_AUTHENTICATED_PROGRAM_CATALOG_DOMAIN_V2: &[u8] =
    b"ferric.m1.authenticated-worker-v3-program-catalog.v2";
const M1_AUTHENTICATED_PROGRAM_MAP_DOMAIN_V2: &[u8] =
    b"ferric.m1.authenticated-worker-v3-program-map.v2";
const M1_AGGREGATE_SERVICE_PROGRAM_INDICES_V1: [usize; M1_PHYSICAL_PROGRAM_COUNT_V1] =
    [0, 9, 4, 11, 2, 7, 6, 5, 1, 10, 8, 3];
const M1_MFMA_AGGREGATE_SERVICE_PROGRAM_INDICES_V1: [usize; M1_MFMA_PHYSICAL_PROGRAM_COUNT_V1] =
    [0, 9, 4, 11, 2, 7, 6, 5, 1, 10, 8, 3, 12];

/// Exact production target admitted by the M1 physical runner.
pub const M1_AUTHENTICATED_PROGRAM_TARGET_V1: &str = "gfx942:xnack-";
/// Exact number of independently authenticated artifact rosters.
pub const M1_AUTHENTICATED_ROSTER_COUNT_V1: usize = 1;

/// The one move-only authenticated aggregate roster owner before set composition.
pub type M1AuthenticatedWorkerV3RosterV1 =
    AuthenticatedWorkerV3RosterV1<M1AllKernelsWorkerV3RosterV1>;

/// Separate authenticated custody for the attributed thirteen-root MFMA roster.
pub type M1AuthenticatedMfmaWorkerV3RosterV1 =
    AuthenticatedWorkerV3RosterV1<M1AllKernelsMfmaWorkerV3RosterV1>;

/// Owners retained when exact M1 program-set intake rejects.
#[must_use = "rejected authenticated owners must remain classified"]
pub struct M1AuthenticatedWorkerV3ProgramSetResidueV1<R = M1AllKernelsWorkerV3RosterV1> {
    /// Erased set containing the aggregate roster after successful composition.
    pub programs: Option<AuthenticatedWorkerV3ProgramSetV1>,
    /// The uncomposed or rejected aggregate roster owner.
    pub roster: Option<AuthenticatedWorkerV3RosterV1<R>>,
}

impl<R> fmt::Debug for M1AuthenticatedWorkerV3ProgramSetResidueV1<R> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedWorkerV3ProgramSetResidueV1")
            .field("programs", &self.programs)
            .field("has_roster", &self.roster.is_some())
            .finish()
    }
}

/// Exact intake phase that rejected authenticated aggregate custody.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedProgramSetIntakePhaseV1 {
    SourceFacts,
    Preflight,
    Compose,
    Aggregate,
}

/// Why the authenticated aggregate roster did not become one exact 12-program set.
#[derive(Debug)]
#[non_exhaustive]
pub enum M1AuthenticatedProgramSetIntakeErrorV1 {
    CurrentPublication(Box<RecoveredWorkerV3AdmissionErrorV1>),
    SourceIdentity {
        axis: &'static str,
    },
    Target {
        expected: AmdTargetId,
        actual: AmdTargetId,
    },
    EntryCount {
        expected: usize,
        actual: usize,
    },
    MarkerSymbol {
        ordinal: usize,
        logical: &'static str,
        export: &'static str,
    },
    MarkerIdentity {
        ordinal: usize,
    },
    VerificationEntry {
        ordinal: usize,
    },
    VerificationAuthority,
    EmptyFinalizedArtifact,
    DuplicateKernelBinding,
    ProgramSet(Box<AuthenticatedWorkerV3ProgramSetAdmissionErrorV1>),
    AggregateCount {
        expected_rosters: usize,
        actual_rosters: usize,
        expected_programs: usize,
        actual_programs: usize,
    },
    ProgramIndex {
        program: M1PhysicalProgramV1,
        expected_service_index: usize,
        actual: Option<usize>,
    },
}

impl fmt::Display for M1AuthenticatedProgramSetIntakeErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "authenticated M1 aggregate Worker V3 program-set intake failed: {self:?}"
        )
    }
}

impl Error for M1AuthenticatedProgramSetIntakeErrorV1 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CurrentPublication(source) => Some(source),
            Self::ProgramSet(source) => Some(source),
            _ => None,
        }
    }
}

/// Intake failure retaining every aggregate owner available at the rejection.
#[must_use = "intake failure retains authenticated roster custody"]
pub struct M1AuthenticatedProgramSetIntakeFailureV1<R = M1AllKernelsWorkerV3RosterV1> {
    phase: M1AuthenticatedProgramSetIntakePhaseV1,
    error: Box<M1AuthenticatedProgramSetIntakeErrorV1>,
    residue: Box<M1AuthenticatedWorkerV3ProgramSetResidueV1<R>>,
}

impl<R> M1AuthenticatedProgramSetIntakeFailureV1<R> {
    /// Returns the exact rejection phase.
    #[must_use]
    pub const fn phase(&self) -> M1AuthenticatedProgramSetIntakePhaseV1 {
        self.phase
    }

    /// Returns the exact rejection diagnostic.
    #[must_use]
    pub const fn error(&self) -> &M1AuthenticatedProgramSetIntakeErrorV1 {
        &self.error
    }

    /// Returns the exact diagnostic and every retained owner.
    pub fn into_parts(
        self,
    ) -> (
        M1AuthenticatedProgramSetIntakePhaseV1,
        M1AuthenticatedProgramSetIntakeErrorV1,
        M1AuthenticatedWorkerV3ProgramSetResidueV1<R>,
    ) {
        (self.phase, *self.error, *self.residue)
    }
}

impl<R> fmt::Debug for M1AuthenticatedProgramSetIntakeFailureV1<R> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedProgramSetIntakeFailureV1")
            .field("phase", &self.phase)
            .field("error", &self.error)
            .field("residue", &self.residue)
            .finish()
    }
}

/// Ferric-qualified custody of one current aggregate roster and exactly 12 programs.
#[must_use = "authenticated M1 program custody must remain retained"]
pub struct M1AuthenticatedWorkerV3ProgramSetV1 {
    programs: AuthenticatedWorkerV3ProgramSetV1,
    family_artifacts: Box<[DeclaredKernelFamilyArtifact]>,
    catalog_id: Identity,
    strategy: M1PhysicalProgramStrategyV1,
    service_program_indices: [usize; M1_MFMA_PHYSICAL_PROGRAM_COUNT_V1],
}

/// Ferric-only identity and role map retained while fe2o3 owns executable custody.
#[must_use = "authenticated program identity witness must remain joined to executable custody"]
#[derive(Debug)]
pub(crate) struct M1AuthenticatedProgramCatalogWitnessV1 {
    family_artifacts: Box<[DeclaredKernelFamilyArtifact]>,
    catalog_id: Identity,
    strategy: M1PhysicalProgramStrategyV1,
    service_program_indices: [usize; M1_MFMA_PHYSICAL_PROGRAM_COUNT_V1],
}

impl M1AuthenticatedProgramCatalogWitnessV1 {
    pub(crate) const fn program_strategy(&self) -> M1PhysicalProgramStrategyV1 {
        self.strategy
    }

    pub(crate) const fn catalog_id(&self) -> Identity {
        self.catalog_id
    }

    pub(crate) fn family_artifacts(&self) -> &[DeclaredKernelFamilyArtifact] {
        &self.family_artifacts
    }

    pub(crate) const fn service_program_index(&self, program: M1PhysicalProgramV1) -> usize {
        match self.try_service_program_index(program) {
            Some(index) => index,
            None => panic!("program is absent from the authenticated strategy"),
        }
    }

    pub(crate) const fn try_service_program_index(
        &self,
        program: M1PhysicalProgramV1,
    ) -> Option<usize> {
        checked_service_program_index(self.strategy, &self.service_program_indices, program)
    }
}

impl fmt::Debug for M1AuthenticatedWorkerV3ProgramSetV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedWorkerV3ProgramSetV1")
            .field("roster_count", &self.programs.roster_count())
            .field("program_count", &self.programs.program_count())
            .field("target", &self.programs.target())
            .field("catalog_id", &self.catalog_id)
            .field("service_program_indices", &self.service_program_indices)
            .finish_non_exhaustive()
    }
}

impl M1AuthenticatedWorkerV3ProgramSetV1 {
    pub(crate) fn into_queue_parts(
        self,
    ) -> (
        AuthenticatedWorkerV3ProgramSetV1,
        M1AuthenticatedProgramCatalogWitnessV1,
    ) {
        (
            self.programs,
            M1AuthenticatedProgramCatalogWitnessV1 {
                family_artifacts: self.family_artifacts,
                catalog_id: self.catalog_id,
                strategy: self.strategy,
                service_program_indices: self.service_program_indices,
            },
        )
    }

    pub(crate) fn from_queue_parts(
        programs: AuthenticatedWorkerV3ProgramSetV1,
        witness: M1AuthenticatedProgramCatalogWitnessV1,
    ) -> Self {
        Self {
            programs,
            family_artifacts: witness.family_artifacts,
            catalog_id: witness.catalog_id,
            strategy: witness.strategy,
            service_program_indices: witness.service_program_indices,
        }
    }

    /// Returns the exact one-roster count.
    #[must_use]
    pub fn roster_count(&self) -> usize {
        self.programs.roster_count()
    }

    /// Returns the exact flattened program count.
    #[must_use]
    pub fn program_count(&self) -> usize {
        self.programs.program_count()
    }

    /// Returns the exact common target.
    #[must_use]
    pub const fn target(&self) -> AmdTargetId {
        self.programs.target()
    }

    /// Returns the Ferric-domain-separated current program catalog identity.
    #[must_use]
    pub const fn catalog_id(&self) -> Identity {
        self.catalog_id
    }

    /// Exact strategy fixed by typed roster admission, not a caller preference.
    #[must_use]
    pub const fn program_strategy(&self) -> M1PhysicalProgramStrategyV1 {
        self.strategy
    }

    /// Resolves one stable Ferric program role to its aggregate service index.
    ///
    /// # Panics
    /// Panics if the requested program is absent from the admitted strategy.
    /// Use [`Self::try_service_program_index`] for a role not already roster-bound.
    #[must_use]
    pub const fn service_program_index(&self, program: M1PhysicalProgramV1) -> usize {
        match self.try_service_program_index(program) {
            Some(index) => index,
            None => panic!("program is absent from the authenticated strategy"),
        }
    }

    /// Resolves a role only if it belongs to the exact authenticated strategy.
    #[must_use]
    pub const fn try_service_program_index(&self, program: M1PhysicalProgramV1) -> Option<usize> {
        checked_service_program_index(self.strategy, &self.service_program_indices, program)
    }

    /// Resolves a compiler-generated marker in the retained program set.
    ///
    /// # Errors
    ///
    /// Returns an authenticated-program lookup error when the marker is absent or mismatched.
    pub fn program_index<K: CompilerGeneratedKernelExpectationV1>(
        &self,
    ) -> Result<usize, AuthenticatedWorkerV3ProgramLookupErrorV1> {
        self.programs.program_index::<K>()
    }

    pub(crate) fn family_artifacts(&self) -> &[DeclaredKernelFamilyArtifact] {
        &self.family_artifacts
    }
}

/// Typed fail-closed result for a legacy raw-artifact production entry point.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct M1AuthenticatedRosterAcquisitionRequiredV1 {
    artifact_root: PathBuf,
}

impl M1AuthenticatedRosterAcquisitionRequiredV1 {
    /// Returns the legacy path that cannot establish authenticated custody.
    #[must_use]
    pub fn artifact_root(&self) -> &Path {
        &self.artifact_root
    }
}

impl fmt::Display for M1AuthenticatedRosterAcquisitionRequiredV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "authenticated Worker V3 roster acquisition is required; legacy KERNEL-ARTIFACTS path {} cannot establish current roster custody",
            self.artifact_root.display()
        )
    }
}

impl Error for M1AuthenticatedRosterAcquisitionRequiredV1 {}

/// Rejects a path-only production request before raw artifact reopening.
///
/// # Errors
///
/// Always returns a typed rejection naming the path that lacks authenticated roster custody.
pub fn require_m1_authenticated_roster_acquisition_v1(
    artifact_root: &Path,
) -> Result<(), M1AuthenticatedRosterAcquisitionRequiredV1> {
    Err(M1AuthenticatedRosterAcquisitionRequiredV1 {
        artifact_root: artifact_root.to_path_buf(),
    })
}

/// Admits one authenticated aggregate roster into the exact 12-program catalog.
///
/// The move-only roster is produced only by the protected verifier adapter. Its
/// retained receipt and current-publication revalidation are the selection
/// authority; this boundary accepts no caller-supplied hashes or currentness
/// assertion.
///
/// # Errors
///
/// Returns the exact rejected intake phase, diagnostic, and retained aggregate owner.
pub fn admit_m1_authenticated_worker_v3_programs_v1(
    roster: M1AuthenticatedWorkerV3RosterV1,
) -> Result<M1AuthenticatedWorkerV3ProgramSetV1, M1AuthenticatedProgramSetIntakeFailureV1> {
    admit_authenticated_programs_with_strategy(roster, M1PhysicalProgramStrategyV1::LegacyScalar12)
}

/// Admits only the separate authenticated thirteen-root MFMA roster.
///
/// # Errors
/// Requires the same current-publication, protected verification, exact source,
/// target and service-index checks as legacy admission, retaining all failures.
pub fn admit_m1_authenticated_mfma_worker_v3_programs_v1(
    roster: M1AuthenticatedMfmaWorkerV3RosterV1,
) -> Result<
    M1AuthenticatedWorkerV3ProgramSetV1,
    M1AuthenticatedProgramSetIntakeFailureV1<M1AllKernelsMfmaWorkerV3RosterV1>,
> {
    admit_authenticated_programs_with_strategy(
        roster,
        M1PhysicalProgramStrategyV1::AttributedMfma13,
    )
}

fn admit_authenticated_programs_with_strategy<R: CompilerGeneratedKernelExpectationRosterV1>(
    roster: AuthenticatedWorkerV3RosterV1<R>,
    strategy: M1PhysicalProgramStrategyV1,
) -> Result<M1AuthenticatedWorkerV3ProgramSetV1, M1AuthenticatedProgramSetIntakeFailureV1<R>> {
    if let Err(error) = validate_roster(&roster, strategy) {
        return Err(intake_failure(
            M1AuthenticatedProgramSetIntakePhaseV1::Preflight,
            error,
            M1AuthenticatedWorkerV3ProgramSetResidueV1 {
                programs: None,
                roster: Some(roster),
            },
        ));
    }

    let roster_catalog_id = authenticated_catalog_id(&roster, strategy);
    let family_artifacts = authenticated_family_artifacts(&roster);
    let programs = match AuthenticatedWorkerV3ProgramSetV1::from_roster(roster) {
        Ok(programs) => programs,
        Err(failure) => {
            let (error, roster) = failure.into_parts();
            return Err(intake_failure(
                M1AuthenticatedProgramSetIntakePhaseV1::Compose,
                M1AuthenticatedProgramSetIntakeErrorV1::ProgramSet(Box::new(error)),
                M1AuthenticatedWorkerV3ProgramSetResidueV1 {
                    programs: None,
                    roster: Some(roster),
                },
            ));
        }
    };

    if programs.roster_count() != M1_AUTHENTICATED_ROSTER_COUNT_V1
        || programs.program_count() != strategy.program_count()
    {
        let error = M1AuthenticatedProgramSetIntakeErrorV1::AggregateCount {
            expected_rosters: M1_AUTHENTICATED_ROSTER_COUNT_V1,
            actual_rosters: programs.roster_count(),
            expected_programs: strategy.program_count(),
            actual_programs: programs.program_count(),
        };
        return Err(intake_failure(
            M1AuthenticatedProgramSetIntakePhaseV1::Aggregate,
            error,
            M1AuthenticatedWorkerV3ProgramSetResidueV1 {
                programs: Some(programs),
                roster: None,
            },
        ));
    }

    let service_program_indices = match authenticated_program_indices(&programs, strategy) {
        Ok(indices) => indices,
        Err(error) => {
            return Err(intake_failure(
                M1AuthenticatedProgramSetIntakePhaseV1::Aggregate,
                error,
                M1AuthenticatedWorkerV3ProgramSetResidueV1 {
                    programs: Some(programs),
                    roster: None,
                },
            ));
        }
    };
    let catalog_id = authenticated_catalog_id_with_program_map(
        roster_catalog_id,
        &service_program_indices,
        strategy,
    );
    Ok(M1AuthenticatedWorkerV3ProgramSetV1 {
        programs,
        family_artifacts,
        catalog_id,
        strategy,
        service_program_indices,
    })
}

fn intake_failure<R>(
    phase: M1AuthenticatedProgramSetIntakePhaseV1,
    error: M1AuthenticatedProgramSetIntakeErrorV1,
    residue: M1AuthenticatedWorkerV3ProgramSetResidueV1<R>,
) -> M1AuthenticatedProgramSetIntakeFailureV1<R> {
    M1AuthenticatedProgramSetIntakeFailureV1 {
        phase,
        error: Box::new(error),
        residue: Box::new(residue),
    }
}

fn validate_roster<R: CompilerGeneratedKernelExpectationRosterV1>(
    roster: &AuthenticatedWorkerV3RosterV1<R>,
    strategy: M1PhysicalProgramStrategyV1,
) -> Result<(), M1AuthenticatedProgramSetIntakeErrorV1> {
    let expected_target =
        AmdTargetId::parse(M1_AUTHENTICATED_PROGRAM_TARGET_V1).expect("fixed target is canonical");
    roster.revalidate_currentness().map_err(|source| {
        M1AuthenticatedProgramSetIntakeErrorV1::CurrentPublication(Box::new(source))
    })?;

    if roster.target() != expected_target {
        return Err(M1AuthenticatedProgramSetIntakeErrorV1::Target {
            expected: expected_target,
            actual: roster.target(),
        });
    }
    if roster.entry_count() != strategy.program_count()
        || R::ENTRIES.len() != strategy.program_count()
    {
        return Err(M1AuthenticatedProgramSetIntakeErrorV1::EntryCount {
            expected: strategy.program_count(),
            actual: roster.entry_count(),
        });
    }

    let verification = roster.verification();
    if !roster.authenticates_verification_authority()
        || !verification.retains_current_compiler_and_signed_verus_evidence()
        || verification.validated_compiler_proof_inputs().is_none()
        || verification.validated_compiler_target_lineage().is_none()
    {
        return Err(M1AuthenticatedProgramSetIntakeErrorV1::VerificationAuthority);
    }
    if verification.finalized_hsaco_length() == 0
        || verification.finalized_hsaco_sha256() == [0; 32]
    {
        return Err(M1AuthenticatedProgramSetIntakeErrorV1::EmptyFinalizedArtifact);
    }
    let mut bindings = Vec::with_capacity(strategy.program_count());
    for (ordinal, entry) in R::ENTRIES.iter().enumerate() {
        if entry.logical_name() != entry.export_name()
            || !strategy
                .program_roster()
                .iter()
                .any(|program| program.kernel_symbol() == entry.export_name())
        {
            return Err(M1AuthenticatedProgramSetIntakeErrorV1::MarkerSymbol {
                ordinal,
                logical: entry.logical_name(),
                export: entry.export_name(),
            });
        }
        if entry.kernel_binding_id() == [0; 32]
            || entry.generated_host_contract_identity() == [0; 32]
        {
            return Err(M1AuthenticatedProgramSetIntakeErrorV1::MarkerIdentity { ordinal });
        }
        if bindings.contains(&entry.kernel_binding_id()) {
            return Err(M1AuthenticatedProgramSetIntakeErrorV1::DuplicateKernelBinding);
        }
        bindings.push(entry.kernel_binding_id());

        let Some(evidence) = verification.entries().get(ordinal) else {
            return Err(M1AuthenticatedProgramSetIntakeErrorV1::VerificationEntry { ordinal });
        };
        if evidence.marker_binding_identity() != entry.kernel_binding_id()
            || evidence.generated_host_contract_identity()
                != entry.generated_host_contract_identity()
        {
            return Err(M1AuthenticatedProgramSetIntakeErrorV1::VerificationEntry { ordinal });
        }
    }
    Ok(())
}

const fn checked_service_program_index(
    strategy: M1PhysicalProgramStrategyV1,
    indices: &[usize; M1_MFMA_PHYSICAL_PROGRAM_COUNT_V1],
    program: M1PhysicalProgramV1,
) -> Option<usize> {
    let ordinal = program.program_index();
    if ordinal >= strategy.program_count() || indices[ordinal] >= strategy.program_count() {
        None
    } else {
        Some(indices[ordinal])
    }
}

fn authenticated_program_indices(
    programs: &AuthenticatedWorkerV3ProgramSetV1,
    strategy: M1PhysicalProgramStrategyV1,
) -> Result<[usize; M1_MFMA_PHYSICAL_PROGRAM_COUNT_V1], M1AuthenticatedProgramSetIntakeErrorV1> {
    let actual = [
        programs.program_index::<GemmReferenceMarkerV1>().ok(),
        programs.program_index::<GemmVectorizedMarkerV1>().ok(),
        programs.program_index::<TokenEmbeddingMarkerV1>().ok(),
        programs.program_index::<RmsNormMarkerV1>().ok(),
        programs.program_index::<RopeMarkerV1>().ok(),
        programs.program_index::<PagedKvWriteMarkerV1>().ok(),
        programs.program_index::<PrefillMarkerV1>().ok(),
        programs.program_index::<PagedDecodeMarkerV1>().ok(),
        programs.program_index::<SwiGluMarkerV1>().ok(),
        programs.program_index::<LogitsArgmaxMarkerV1>().ok(),
        programs.program_index::<LogitsCompactMarkerV1>().ok(),
        programs.program_index::<SpeculativeAssemblyMarkerV1>().ok(),
        programs.program_index::<GemmMfmaMarkerV1>().ok(),
    ];
    let expected: &[usize] = match strategy {
        M1PhysicalProgramStrategyV1::LegacyScalar12 => &M1_AGGREGATE_SERVICE_PROGRAM_INDICES_V1,
        M1PhysicalProgramStrategyV1::AttributedMfma13 => {
            &M1_MFMA_AGGREGATE_SERVICE_PROGRAM_INDICES_V1
        }
    };
    let mut indices = [usize::MAX; M1_MFMA_PHYSICAL_PROGRAM_COUNT_V1];
    for ((program, actual), expected_service_index) in strategy
        .program_roster()
        .iter()
        .copied()
        .zip(actual)
        .zip(expected.iter().copied())
    {
        if actual != Some(expected_service_index) {
            return Err(M1AuthenticatedProgramSetIntakeErrorV1::ProgramIndex {
                program,
                expected_service_index,
                actual,
            });
        }
        indices[program.program_index()] = expected_service_index;
    }
    Ok(indices)
}

fn authenticated_catalog_id_with_program_map(
    roster_catalog_id: Identity,
    service_program_indices: &[usize; M1_MFMA_PHYSICAL_PROGRAM_COUNT_V1],
    strategy: M1PhysicalProgramStrategyV1,
) -> Identity {
    let mut digest = Sha256::new();
    digest.update(M1_AUTHENTICATED_PROGRAM_MAP_DOMAIN_V2);
    digest.update(roster_catalog_id.as_bytes());
    if strategy == M1PhysicalProgramStrategyV1::AttributedMfma13 {
        digest.update(b"ferric.m1.attributed-mfma-program-strategy.v1");
    }
    for (program, service_index) in strategy
        .program_roster()
        .iter()
        .copied()
        .zip(service_program_indices)
    {
        digest.update([program as u8]);
        digest.update((*service_index as u64).to_le_bytes());
    }
    Identity::new(digest.finalize().into())
}

fn authenticated_family_artifacts<R: CompilerGeneratedKernelExpectationRosterV1>(
    roster: &AuthenticatedWorkerV3RosterV1<R>,
) -> Box<[DeclaredKernelFamilyArtifact]> {
    let compiler_handoff = Identity::new(*roster.compiler_handoff_identity().sha256());
    let finalized = Identity::new(roster.verification().finalized_hsaco_sha256());
    let symbol_manifest = Identity::new(*roster.compiler_symbol_manifest_identity().sha256());
    M1KernelArtifactFamilyV1::ALL
        .into_iter()
        .map(|family| {
            DeclaredKernelFamilyArtifact::new(
                kernel_family(family),
                compiler_handoff,
                finalized,
                symbol_manifest,
            )
        })
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

fn authenticated_catalog_id<R: CompilerGeneratedKernelExpectationRosterV1>(
    roster: &AuthenticatedWorkerV3RosterV1<R>,
    strategy: M1PhysicalProgramStrategyV1,
) -> Identity {
    let verification = roster.verification();
    let compiler_module = roster.compiler_module_identity();
    let compiler_handoff = roster.compiler_handoff_identity();
    let symbol_manifest = roster.compiler_symbol_manifest_identity();
    let mut digest = Sha256::new();
    digest.update(M1_AUTHENTICATED_PROGRAM_CATALOG_DOMAIN_V2);
    digest.update((M1_AUTHENTICATED_ROSTER_COUNT_V1 as u64).to_le_bytes());
    digest.update((strategy.program_count() as u64).to_le_bytes());
    if strategy == M1PhysicalProgramStrategyV1::AttributedMfma13 {
        digest.update(b"ferric.m1.attributed-mfma-program-strategy.v1");
    }
    digest.update(compiler_module.sha256());
    digest.update(compiler_module.byte_len().to_le_bytes());
    digest.update(compiler_handoff.sha256());
    digest.update(compiler_handoff.byte_len().to_le_bytes());
    digest.update(symbol_manifest.sha256());
    digest.update(symbol_manifest.byte_len().to_le_bytes());
    digest.update(verification.lineage_identity().as_bytes());
    digest.update(verification.roster_identity().as_bytes());
    digest.update(verification.finalized_hsaco_sha256());
    digest.update(verification.finalized_hsaco_length().to_le_bytes());
    for entry in verification.entries() {
        digest.update(entry.marker_binding_identity());
        digest.update(entry.generated_host_contract_identity());
    }
    for family in M1KernelArtifactFamilyV1::ALL {
        digest.update([family as u8]);
    }
    Identity::new(digest.finalize().into())
}

const fn kernel_family(family: M1KernelArtifactFamilyV1) -> KernelFamily {
    match family {
        M1KernelArtifactFamilyV1::Gemm => KernelFamily::K1GemmGemv,
        M1KernelArtifactFamilyV1::RmsNorm => KernelFamily::K2RmsNormResidual,
        M1KernelArtifactFamilyV1::RopeKv => KernelFamily::K3RopePagedKv,
        M1KernelArtifactFamilyV1::Prefill => KernelFamily::K4GqaPrefill,
        M1KernelArtifactFamilyV1::PagedDecode => KernelFamily::K5PagedGqaDecode,
        M1KernelArtifactFamilyV1::SwiGlu => KernelFamily::K6SwiGlu,
        M1KernelArtifactFamilyV1::Logits => KernelFamily::K7LogitsCompact,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn service_index<K: CompilerGeneratedKernelExpectationV1>() -> usize {
        M1AllKernelsWorkerV3RosterV1::ENTRIES
            .iter()
            .position(|entry| entry.kernel_binding_id() == K::KERNEL_BINDING_ID_V1)
            .expect("aggregate marker must occur exactly once")
    }

    #[test]
    fn aggregate_roster_is_canonical_and_covers_twelve_exact_markers() {
        let roster = M1AllKernelsWorkerV3RosterV1::ENTRIES;
        assert_eq!(roster.len(), M1_PHYSICAL_PROGRAM_COUNT_V1);
        assert!(
            roster
                .windows(2)
                .all(|entries| entries[0].kernel_binding_id() < entries[1].kernel_binding_id()),
            "aggregate Worker V3 roster must follow canonical descriptor-table order"
        );
        let mut symbols = roster
            .iter()
            .map(fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1::export_name)
            .collect::<Vec<_>>();
        let mut expected = M1PhysicalProgramV1::ALL
            .into_iter()
            .map(M1PhysicalProgramV1::kernel_symbol)
            .collect::<Vec<_>>();
        symbols.sort_unstable();
        expected.sort_unstable();
        assert_eq!(symbols, expected);
    }

    #[test]
    fn physical_roles_map_to_exact_aggregate_service_indices() {
        let actual = [
            service_index::<GemmReferenceMarkerV1>(),
            service_index::<GemmVectorizedMarkerV1>(),
            service_index::<TokenEmbeddingMarkerV1>(),
            service_index::<RmsNormMarkerV1>(),
            service_index::<RopeMarkerV1>(),
            service_index::<PagedKvWriteMarkerV1>(),
            service_index::<PrefillMarkerV1>(),
            service_index::<PagedDecodeMarkerV1>(),
            service_index::<SwiGluMarkerV1>(),
            service_index::<LogitsArgmaxMarkerV1>(),
            service_index::<LogitsCompactMarkerV1>(),
            service_index::<SpeculativeAssemblyMarkerV1>(),
        ];
        assert_eq!(actual, [0, 9, 4, 11, 2, 7, 6, 5, 1, 10, 8, 3]);
        assert_eq!(actual, M1_AGGREGATE_SERVICE_PROGRAM_INDICES_V1);
    }

    #[test]
    fn mfma_roster_has_separate_exact_markers_and_service_order() {
        let entries = M1AllKernelsMfmaWorkerV3RosterV1::ENTRIES;
        assert_eq!(entries.len(), M1_MFMA_PHYSICAL_PROGRAM_COUNT_V1);
        assert!(entries
            .windows(2)
            .all(|pair| pair[0].kernel_binding_id() < pair[1].kernel_binding_id()));
        for (program, &index) in M1PhysicalProgramV1::MFMA_ALL
            .iter()
            .zip(&M1_MFMA_AGGREGATE_SERVICE_PROGRAM_INDICES_V1)
        {
            assert_eq!(entries[index].export_name(), program.kernel_symbol());
            assert_eq!(entries[index].logical_name(), program.kernel_symbol());
            assert_ne!(entries[index].generated_host_contract_identity(), [0; 32]);
        }
        assert_eq!(
            entries[12].kernel_binding_id(),
            GemmMfmaMarkerV1::KERNEL_BINDING_ID_V1
        );
        assert!(!M1AllKernelsWorkerV3RosterV1::ENTRIES
            .iter()
            .any(|entry| entry.kernel_binding_id() == GemmMfmaMarkerV1::KERNEL_BINDING_ID_V1));
    }

    #[test]
    fn legacy_program_map_hash_bytes_ignore_unused_capacity_and_mfma_is_separate() {
        let roster_id = Identity::new([41; 32]);
        let mut indices = [usize::MAX; M1_MFMA_PHYSICAL_PROGRAM_COUNT_V1];
        indices[..12].copy_from_slice(&M1_AGGREGATE_SERVICE_PROGRAM_INDICES_V1);
        let legacy = authenticated_catalog_id_with_program_map(
            roster_id,
            &indices,
            M1PhysicalProgramStrategyV1::LegacyScalar12,
        );
        let mut previous = Sha256::new();
        previous.update(M1_AUTHENTICATED_PROGRAM_MAP_DOMAIN_V2);
        previous.update(roster_id.as_bytes());
        for (program, index) in M1PhysicalProgramV1::ALL
            .into_iter()
            .zip(M1_AGGREGATE_SERVICE_PROGRAM_INDICES_V1)
        {
            previous.update([program as u8]);
            previous.update((index as u64).to_le_bytes());
        }
        assert_eq!(legacy, Identity::new(previous.finalize().into()));
        indices[12] = 12;
        assert_eq!(
            legacy,
            authenticated_catalog_id_with_program_map(
                roster_id,
                &indices,
                M1PhysicalProgramStrategyV1::LegacyScalar12
            )
        );
        assert_ne!(
            legacy,
            authenticated_catalog_id_with_program_map(
                roster_id,
                &indices,
                M1PhysicalProgramStrategyV1::AttributedMfma13
            )
        );
    }

    #[test]
    fn checked_service_lookup_never_exposes_absent_or_out_of_range_indices() {
        use M1PhysicalProgramStrategyV1::{AttributedMfma13, LegacyScalar12};
        let mut indices = M1_MFMA_AGGREGATE_SERVICE_PROGRAM_INDICES_V1;
        for program in M1PhysicalProgramV1::ALL {
            assert_eq!(
                checked_service_program_index(LegacyScalar12, &indices, program),
                Some(indices[program.program_index()])
            );
        }
        assert_eq!(
            checked_service_program_index(LegacyScalar12, &indices, M1PhysicalProgramV1::GemmMfma),
            None
        );
        assert_eq!(
            checked_service_program_index(
                AttributedMfma13,
                &indices,
                M1PhysicalProgramV1::GemmMfma
            ),
            Some(12)
        );
        indices[0] = usize::MAX;
        assert_eq!(
            checked_service_program_index(
                LegacyScalar12,
                &indices,
                M1PhysicalProgramV1::GemmReference
            ),
            None
        );
        assert_eq!(
            checked_service_program_index(
                AttributedMfma13,
                &indices,
                M1PhysicalProgramV1::GemmReference
            ),
            None
        );
    }

    #[test]
    fn aggregate_k3_markers_retain_exact_generated_contracts() {
        let entries = M1AllKernelsWorkerV3RosterV1::ENTRIES;
        for (ordinal, binding, contract) in [
            (
                7,
                PagedKvWriteMarkerV1::KERNEL_BINDING_ID_V1,
                PagedKvWriteMarkerV1::PROFILE.generated_host_contract_identity(),
            ),
            (
                2,
                RopeMarkerV1::KERNEL_BINDING_ID_V1,
                RopeMarkerV1::PROFILE.generated_host_contract_identity(),
            ),
        ] {
            assert_eq!(entries[ordinal].kernel_binding_id(), binding);
            assert_eq!(
                entries[ordinal].generated_host_contract_identity(),
                contract
            );
            assert_ne!(binding, [0; 32]);
        }
    }

    #[test]
    fn path_only_acquisition_is_typed_and_fail_closed() {
        let path = Path::new("/kernel-artifacts");
        let error = require_m1_authenticated_roster_acquisition_v1(path)
            .expect_err("a path cannot authenticate Worker V3 custody");
        assert_eq!(error.artifact_root(), path);
        assert!(error.to_string().contains("roster acquisition is required"));
    }

    #[test]
    fn aggregate_admission_has_one_authenticated_roster_authority() {
        assert_eq!(M1_AUTHENTICATED_ROSTER_COUNT_V1, 1);
    }
}
