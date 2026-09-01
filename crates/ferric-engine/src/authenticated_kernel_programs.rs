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
    M1AllKernelsWorkerV3RosterV1,
};
use ferric_spec::Identity;
use sha2::{Digest, Sha256};

use crate::{DeclaredKernelFamilyArtifact, M1PhysicalProgramV1, M1_PHYSICAL_PROGRAM_COUNT_V1};

const M1_AUTHENTICATED_PROGRAM_CATALOG_DOMAIN_V2: &[u8] =
    b"ferric.m1.authenticated-worker-v3-program-catalog.v2";
const M1_AUTHENTICATED_PROGRAM_MAP_DOMAIN_V2: &[u8] =
    b"ferric.m1.authenticated-worker-v3-program-map.v2";
const M1_AGGREGATE_SERVICE_PROGRAM_INDICES_V1: [usize; M1_PHYSICAL_PROGRAM_COUNT_V1] =
    [7, 6, 8, 11, 10, 3, 1, 4, 0, 2, 9, 5];

/// Exact production target admitted by the M1 physical runner.
pub const M1_AUTHENTICATED_PROGRAM_TARGET_V1: &str = "gfx942:xnack-";
/// Exact number of independently authenticated artifact rosters.
pub const M1_AUTHENTICATED_ROSTER_COUNT_V1: usize = 1;

/// The one move-only authenticated aggregate roster owner before set composition.
pub type M1AuthenticatedWorkerV3RosterV1 =
    AuthenticatedWorkerV3RosterV1<M1AllKernelsWorkerV3RosterV1>;

/// Owners retained when exact M1 program-set intake rejects.
#[must_use = "rejected authenticated owners must remain classified"]
pub struct M1AuthenticatedWorkerV3ProgramSetResidueV1 {
    /// Erased set containing the aggregate roster after successful composition.
    pub programs: Option<AuthenticatedWorkerV3ProgramSetV1>,
    /// The uncomposed or rejected aggregate roster owner.
    pub roster: Option<M1AuthenticatedWorkerV3RosterV1>,
}

impl fmt::Debug for M1AuthenticatedWorkerV3ProgramSetResidueV1 {
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
    /// No checked-in producer fact can yet identify the current aggregate compiler output.
    MissingAggregateSourcePin,
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
pub struct M1AuthenticatedProgramSetIntakeFailureV1 {
    phase: M1AuthenticatedProgramSetIntakePhaseV1,
    error: Box<M1AuthenticatedProgramSetIntakeErrorV1>,
    residue: Box<M1AuthenticatedWorkerV3ProgramSetResidueV1>,
}

impl M1AuthenticatedProgramSetIntakeFailureV1 {
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
        M1AuthenticatedWorkerV3ProgramSetResidueV1,
    ) {
        (self.phase, *self.error, *self.residue)
    }
}

impl fmt::Debug for M1AuthenticatedProgramSetIntakeFailureV1 {
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
    service_program_indices: [usize; M1_PHYSICAL_PROGRAM_COUNT_V1],
}

/// Ferric-only identity and role map retained while fe2o3 owns executable custody.
#[must_use = "authenticated program identity witness must remain joined to executable custody"]
#[derive(Debug)]
pub(crate) struct M1AuthenticatedProgramCatalogWitnessV1 {
    family_artifacts: Box<[DeclaredKernelFamilyArtifact]>,
    catalog_id: Identity,
    service_program_indices: [usize; M1_PHYSICAL_PROGRAM_COUNT_V1],
}

impl M1AuthenticatedProgramCatalogWitnessV1 {
    pub(crate) const fn catalog_id(&self) -> Identity {
        self.catalog_id
    }

    pub(crate) fn family_artifacts(&self) -> &[DeclaredKernelFamilyArtifact] {
        &self.family_artifacts
    }

    pub(crate) const fn service_program_index(&self, program: M1PhysicalProgramV1) -> usize {
        self.service_program_indices[program.program_index()]
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

    /// Resolves one stable Ferric program role to its aggregate service index.
    #[must_use]
    pub const fn service_program_index(&self, program: M1PhysicalProgramV1) -> usize {
        self.service_program_indices[program.program_index()]
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

/// Producer-owned selection coordinates for the single aggregate compiler output.
///
/// A selection candidate is not a current selection. Keeping this unavailable type and its
/// current value private prevents callers from substituting identities while the protected
/// publication and verifier-authority work remains incomplete.
struct M1AggregatePublicationSelectionV1 {
    compiler_module_sha256: [u8; 32],
    compiler_module_length: u64,
    compiler_handoff_sha256: [u8; 32],
    compiler_handoff_length: u64,
    symbol_manifest_sha256: [u8; 32],
    symbol_manifest_length: u64,
    finalized_artifact_sha256: [u8; 32],
    finalized_artifact_length: u64,
}

const M1_CURRENT_AGGREGATE_PUBLICATION_SELECTION_V1: Option<M1AggregatePublicationSelectionV1> =
    None;

/// Admits one authenticated aggregate roster into the exact 12-program catalog.
///
/// A produced selection candidate is not current authority. Until a separate review installs
/// one private current selection, this function rejects with `MissingAggregateSourcePin` and
/// returns the authenticated roster in its residue.
///
/// # Errors
///
/// Returns the exact rejected intake phase, diagnostic, and retained aggregate owner.
pub fn admit_m1_authenticated_worker_v3_programs_v1(
    roster: M1AuthenticatedWorkerV3RosterV1,
) -> Result<M1AuthenticatedWorkerV3ProgramSetV1, M1AuthenticatedProgramSetIntakeFailureV1> {
    let Some(selection) = M1_CURRENT_AGGREGATE_PUBLICATION_SELECTION_V1 else {
        return Err(intake_failure(
            M1AuthenticatedProgramSetIntakePhaseV1::SourceFacts,
            M1AuthenticatedProgramSetIntakeErrorV1::MissingAggregateSourcePin,
            M1AuthenticatedWorkerV3ProgramSetResidueV1 {
                programs: None,
                roster: Some(roster),
            },
        ));
    };

    if let Err(error) = validate_roster(&selection, &roster) {
        return Err(intake_failure(
            M1AuthenticatedProgramSetIntakePhaseV1::Preflight,
            error,
            M1AuthenticatedWorkerV3ProgramSetResidueV1 {
                programs: None,
                roster: Some(roster),
            },
        ));
    }

    let roster_catalog_id = authenticated_catalog_id(&selection, &roster);
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
        || programs.program_count() != M1_PHYSICAL_PROGRAM_COUNT_V1
    {
        let error = M1AuthenticatedProgramSetIntakeErrorV1::AggregateCount {
            expected_rosters: M1_AUTHENTICATED_ROSTER_COUNT_V1,
            actual_rosters: programs.roster_count(),
            expected_programs: M1_PHYSICAL_PROGRAM_COUNT_V1,
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

    let service_program_indices = match authenticated_program_indices(&programs) {
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
    let catalog_id =
        authenticated_catalog_id_with_program_map(roster_catalog_id, &service_program_indices);
    Ok(M1AuthenticatedWorkerV3ProgramSetV1 {
        programs,
        family_artifacts,
        catalog_id,
        service_program_indices,
    })
}

fn intake_failure(
    phase: M1AuthenticatedProgramSetIntakePhaseV1,
    error: M1AuthenticatedProgramSetIntakeErrorV1,
    residue: M1AuthenticatedWorkerV3ProgramSetResidueV1,
) -> M1AuthenticatedProgramSetIntakeFailureV1 {
    M1AuthenticatedProgramSetIntakeFailureV1 {
        phase,
        error: Box::new(error),
        residue: Box::new(residue),
    }
}

fn validate_roster(
    selection: &M1AggregatePublicationSelectionV1,
    roster: &M1AuthenticatedWorkerV3RosterV1,
) -> Result<(), M1AuthenticatedProgramSetIntakeErrorV1> {
    let expected_target =
        AmdTargetId::parse(M1_AUTHENTICATED_PROGRAM_TARGET_V1).expect("fixed target is canonical");
    roster.revalidate_currentness().map_err(|source| {
        M1AuthenticatedProgramSetIntakeErrorV1::CurrentPublication(Box::new(source))
    })?;

    let compiler_module = roster.compiler_module_identity();
    if compiler_module.sha256() != &selection.compiler_module_sha256
        || compiler_module.byte_len() != selection.compiler_module_length
    {
        return Err(M1AuthenticatedProgramSetIntakeErrorV1::SourceIdentity {
            axis: "compiler module",
        });
    }
    let compiler_handoff = roster.compiler_handoff_identity();
    if compiler_handoff.sha256() != &selection.compiler_handoff_sha256
        || compiler_handoff.byte_len() != selection.compiler_handoff_length
    {
        return Err(M1AuthenticatedProgramSetIntakeErrorV1::SourceIdentity {
            axis: "compiler handoff",
        });
    }
    let symbol_manifest = roster.compiler_symbol_manifest_identity();
    if symbol_manifest.sha256() != &selection.symbol_manifest_sha256
        || symbol_manifest.byte_len() != selection.symbol_manifest_length
    {
        return Err(M1AuthenticatedProgramSetIntakeErrorV1::SourceIdentity {
            axis: "symbol manifest",
        });
    }
    if roster.target() != expected_target {
        return Err(M1AuthenticatedProgramSetIntakeErrorV1::Target {
            expected: expected_target,
            actual: roster.target(),
        });
    }
    if roster.entry_count() != M1_PHYSICAL_PROGRAM_COUNT_V1
        || M1AllKernelsWorkerV3RosterV1::ENTRIES.len() != M1_PHYSICAL_PROGRAM_COUNT_V1
    {
        return Err(M1AuthenticatedProgramSetIntakeErrorV1::EntryCount {
            expected: M1_PHYSICAL_PROGRAM_COUNT_V1,
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
    if verification.finalized_hsaco_sha256() != selection.finalized_artifact_sha256
        || verification.finalized_hsaco_length() != selection.finalized_artifact_length
    {
        return Err(M1AuthenticatedProgramSetIntakeErrorV1::SourceIdentity {
            axis: "finalized artifact",
        });
    }

    let mut bindings = Vec::with_capacity(M1_PHYSICAL_PROGRAM_COUNT_V1);
    for (ordinal, entry) in M1AllKernelsWorkerV3RosterV1::ENTRIES.iter().enumerate() {
        if entry.logical_name() != entry.export_name()
            || !M1PhysicalProgramV1::ALL
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

fn authenticated_program_indices(
    programs: &AuthenticatedWorkerV3ProgramSetV1,
) -> Result<[usize; M1_PHYSICAL_PROGRAM_COUNT_V1], M1AuthenticatedProgramSetIntakeErrorV1> {
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
    ];
    let mut indices = [0; M1_PHYSICAL_PROGRAM_COUNT_V1];
    for ((program, actual), expected_service_index) in M1PhysicalProgramV1::ALL
        .into_iter()
        .zip(actual)
        .zip(M1_AGGREGATE_SERVICE_PROGRAM_INDICES_V1)
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
    service_program_indices: &[usize; M1_PHYSICAL_PROGRAM_COUNT_V1],
) -> Identity {
    let mut digest = Sha256::new();
    digest.update(M1_AUTHENTICATED_PROGRAM_MAP_DOMAIN_V2);
    digest.update(roster_catalog_id.as_bytes());
    for (program, service_index) in M1PhysicalProgramV1::ALL
        .into_iter()
        .zip(service_program_indices)
    {
        digest.update([program as u8]);
        digest.update((*service_index as u64).to_le_bytes());
    }
    Identity::new(digest.finalize().into())
}

fn authenticated_family_artifacts(
    roster: &M1AuthenticatedWorkerV3RosterV1,
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

fn authenticated_catalog_id(
    selection: &M1AggregatePublicationSelectionV1,
    roster: &M1AuthenticatedWorkerV3RosterV1,
) -> Identity {
    let verification = roster.verification();
    let mut digest = Sha256::new();
    digest.update(M1_AUTHENTICATED_PROGRAM_CATALOG_DOMAIN_V2);
    digest.update((M1_AUTHENTICATED_ROSTER_COUNT_V1 as u64).to_le_bytes());
    digest.update((M1_PHYSICAL_PROGRAM_COUNT_V1 as u64).to_le_bytes());
    digest.update(selection.compiler_module_sha256);
    digest.update(selection.compiler_module_length.to_le_bytes());
    digest.update(selection.compiler_handoff_sha256);
    digest.update(selection.compiler_handoff_length.to_le_bytes());
    digest.update(selection.symbol_manifest_sha256);
    digest.update(selection.symbol_manifest_length.to_le_bytes());
    digest.update(selection.finalized_artifact_sha256);
    digest.update(selection.finalized_artifact_length.to_le_bytes());
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
        assert_eq!(actual, [7, 6, 8, 11, 10, 3, 1, 4, 0, 2, 9, 5]);
        assert_eq!(actual, M1_AGGREGATE_SERVICE_PROGRAM_INDICES_V1);
    }

    #[test]
    fn aggregate_k3_markers_retain_exact_generated_contracts() {
        let entries = M1AllKernelsWorkerV3RosterV1::ENTRIES;
        for (ordinal, binding, contract) in [
            (
                3,
                PagedKvWriteMarkerV1::KERNEL_BINDING_ID_V1,
                PagedKvWriteMarkerV1::PROFILE.generated_host_contract_identity(),
            ),
            (
                10,
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
    fn aggregate_publication_selection_is_explicitly_unavailable() {
        assert!(M1_CURRENT_AGGREGATE_PUBLICATION_SELECTION_V1.is_none());
        assert_eq!(M1_AUTHENTICATED_ROSTER_COUNT_V1, 1);
    }
}
