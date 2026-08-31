//! Ferric-owned intake for the exact authenticated M1 Worker V3 roster set.
//!
//! Six integrated families use their compiler-generated marker types directly.
//! K3 remains generic until its generated device crate is integrated; callers
//! must supply the authenticated roster and both generated marker types.
//! Nothing in this module constructs authentication authority from persisted
//! bytes or reopens a raw HSACO file.

use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

use fe2o3_amd_target::AmdTargetId;
use fe2o3_host::{
    AuthenticatedServiceCurrentnessFailureV1, AuthenticatedServiceQueueBindFailureV1,
    AuthenticatedServiceQueueCreateFailureV1, AuthenticatedServiceQueueRolloverFailureV1,
    AuthenticatedServiceQueueRolloverSuccessV1, AuthenticatedServiceQueueSessionV1,
    AuthenticatedServiceQueueUnboundSessionV1, AuthenticatedServiceRecycledQueueSessionV1,
    AuthenticatedWorkerV3ProgramLookupErrorV1, AuthenticatedWorkerV3ProgramSetAdmissionErrorV1,
    AuthenticatedWorkerV3ProgramSetV1, AuthenticatedWorkerV3RosterV1,
    CompilerGeneratedKernelExpectationRosterV1, CompilerGeneratedKernelExpectationV1,
    RecoveredWorkerV3AdmissionErrorV1,
};
use fe2o3_service_host::{ServiceAllocationSessionV1, ServiceFixedDispatchPacketV1};
use ferric_build::{
    current_m1_kernel_source_facts_v1, M1CurrentKernelSourceFactsV1, M1KernelArtifactBuildErrorV1,
    M1KernelArtifactFamilyV1,
};
use ferric_kernels::KernelFamily;
use ferric_spec::Identity;
use sha2::{Digest, Sha256};

use crate::{DeclaredKernelFamilyArtifact, M1PhysicalProgramV1, M1_PHYSICAL_PROGRAM_COUNT_V1};

const M1_AUTHENTICATED_PROGRAM_CATALOG_DOMAIN_V1: &[u8] =
    b"ferric.m1.authenticated-worker-v3-program-catalog.v1";
/// Exact production target admitted by the M1 physical runner.
pub const M1_AUTHENTICATED_PROGRAM_TARGET_V1: &str = "gfx942:xnack-";
/// Exact number of independently authenticated artifact rosters.
pub const M1_AUTHENTICATED_ROSTER_COUNT_V1: usize = 7;

type GemmReferenceMarkerV1 =
    ferric_qwen3_gemm_device_v1::ferric_qwen3_gemm_reference_bf16_f32_bf16_v1_gpu::Marker;
type GemmVectorizedMarkerV1 =
    ferric_qwen3_gemm_device_v1::ferric_qwen3_gemm_vector_a4_bf16_f32_bf16_v1_gpu::Marker;
type TokenEmbeddingMarkerV1 =
    ferric_qwen3_gemm_device_v1::ferric_qwen3_token_embedding_bf16_copy_v1_gpu::Marker;
type RmsNormMarkerV1 = ferric_qwen3_rmsnorm_device_v1::qwen3_rmsnorm_v1_gpu::Marker;
type PrefillMarkerV1 =
    ferric_qwen3_prefill_device_v1::qwen3_gqa_prefill_causal_bf16_f32_v1_gpu::Marker;
type PagedDecodeMarkerV1 =
    ferric_qwen3_paged_decode_device_v1::qwen3_paged_gqa_decode_bf16_f32_v1_gpu::Marker;
type SwiGluMarkerV1 = ferric_qwen3_swiglu_device_v1::qwen3_swiglu_bf16_f32_v1_gpu::Marker;
type LogitsArgmaxMarkerV1 =
    ferric_qwen3_logits_device_v1::ferric_qwen3_lowest_id_argmax_bf16_v1_gpu::Marker;
type LogitsCompactMarkerV1 =
    ferric_qwen3_logits_device_v1::ferric_qwen3_compact_completion_v1_gpu::Marker;
type SpeculativeAssemblyMarkerV1 =
    ferric_qwen3_logits_device_v1::ferric_qwen3_speculative_token_assembly_v1_gpu::Marker;

fe2o3_host::compiler_generated_kernel_expectation_roster_v1! {
    /// Exact compiler-generated K1 marker roster.
    pub struct M1GemmWorkerV3RosterV1 = [
        GemmReferenceMarkerV1,
        GemmVectorizedMarkerV1,
        TokenEmbeddingMarkerV1,
    ];
}

fe2o3_host::compiler_generated_kernel_expectation_roster_v1! {
    /// Exact compiler-generated K2 marker roster.
    pub struct M1RmsNormWorkerV3RosterV1 = [RmsNormMarkerV1];
}

fe2o3_host::compiler_generated_kernel_expectation_roster_v1! {
    /// Exact compiler-generated K4 marker roster.
    pub struct M1PrefillWorkerV3RosterV1 = [PrefillMarkerV1];
}

fe2o3_host::compiler_generated_kernel_expectation_roster_v1! {
    /// Exact compiler-generated K5 marker roster.
    pub struct M1PagedDecodeWorkerV3RosterV1 = [PagedDecodeMarkerV1];
}

fe2o3_host::compiler_generated_kernel_expectation_roster_v1! {
    /// Exact compiler-generated K6 marker roster.
    pub struct M1SwiGluWorkerV3RosterV1 = [SwiGluMarkerV1];
}

fe2o3_host::compiler_generated_kernel_expectation_roster_v1! {
    /// Exact compiler-generated K7 marker roster.
    pub struct M1LogitsWorkerV3RosterV1 = [
        LogitsArgmaxMarkerV1,
        LogitsCompactMarkerV1,
        SpeculativeAssemblyMarkerV1,
    ];
}

/// Move-only authenticated roster owners before heterogeneous set composition.
#[must_use = "authenticated roster custody must be admitted or explicitly released"]
pub struct M1AuthenticatedWorkerV3RostersV1<K3R> {
    gemm: AuthenticatedWorkerV3RosterV1<M1GemmWorkerV3RosterV1>,
    rmsnorm: AuthenticatedWorkerV3RosterV1<M1RmsNormWorkerV3RosterV1>,
    rope_kv: AuthenticatedWorkerV3RosterV1<K3R>,
    prefill: AuthenticatedWorkerV3RosterV1<M1PrefillWorkerV3RosterV1>,
    paged_decode: AuthenticatedWorkerV3RosterV1<M1PagedDecodeWorkerV3RosterV1>,
    swiglu: AuthenticatedWorkerV3RosterV1<M1SwiGluWorkerV3RosterV1>,
    logits: AuthenticatedWorkerV3RosterV1<M1LogitsWorkerV3RosterV1>,
}

impl<K3R> fmt::Debug for M1AuthenticatedWorkerV3RostersV1<K3R> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedWorkerV3RostersV1")
            .field("gemm", &self.gemm)
            .field("rmsnorm", &self.rmsnorm)
            .field("rope_kv", &self.rope_kv)
            .field("prefill", &self.prefill)
            .field("paged_decode", &self.paged_decode)
            .field("swiglu", &self.swiglu)
            .field("logits", &self.logits)
            .finish()
    }
}

impl<K3R: CompilerGeneratedKernelExpectationRosterV1> M1AuthenticatedWorkerV3RostersV1<K3R> {
    /// Groups seven already-authenticated roster owners without creating authority.
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        gemm: AuthenticatedWorkerV3RosterV1<M1GemmWorkerV3RosterV1>,
        rmsnorm: AuthenticatedWorkerV3RosterV1<M1RmsNormWorkerV3RosterV1>,
        rope_kv: AuthenticatedWorkerV3RosterV1<K3R>,
        prefill: AuthenticatedWorkerV3RosterV1<M1PrefillWorkerV3RosterV1>,
        paged_decode: AuthenticatedWorkerV3RosterV1<M1PagedDecodeWorkerV3RosterV1>,
        swiglu: AuthenticatedWorkerV3RosterV1<M1SwiGluWorkerV3RosterV1>,
        logits: AuthenticatedWorkerV3RosterV1<M1LogitsWorkerV3RosterV1>,
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

    fn into_residue(self) -> M1AuthenticatedWorkerV3ProgramSetResidueV1<K3R> {
        M1AuthenticatedWorkerV3ProgramSetResidueV1 {
            programs: None,
            gemm: Some(self.gemm),
            rmsnorm: Some(self.rmsnorm),
            rope_kv: Some(self.rope_kv),
            prefill: Some(self.prefill),
            paged_decode: Some(self.paged_decode),
            swiglu: Some(self.swiglu),
            logits: Some(self.logits),
        }
    }
}

/// Owners retained when exact M1 program-set intake rejects.
#[must_use = "rejected authenticated owners must remain classified"]
pub struct M1AuthenticatedWorkerV3ProgramSetResidueV1<K3R> {
    /// Erased set containing every roster composed before the rejection.
    pub programs: Option<AuthenticatedWorkerV3ProgramSetV1>,
    /// Uncomposed or rejected K1 owner.
    pub gemm: Option<AuthenticatedWorkerV3RosterV1<M1GemmWorkerV3RosterV1>>,
    /// Uncomposed or rejected K2 owner.
    pub rmsnorm: Option<AuthenticatedWorkerV3RosterV1<M1RmsNormWorkerV3RosterV1>>,
    /// Uncomposed or rejected caller-supplied K3 owner.
    pub rope_kv: Option<AuthenticatedWorkerV3RosterV1<K3R>>,
    /// Uncomposed or rejected K4 owner.
    pub prefill: Option<AuthenticatedWorkerV3RosterV1<M1PrefillWorkerV3RosterV1>>,
    /// Uncomposed or rejected K5 owner.
    pub paged_decode: Option<AuthenticatedWorkerV3RosterV1<M1PagedDecodeWorkerV3RosterV1>>,
    /// Uncomposed or rejected K6 owner.
    pub swiglu: Option<AuthenticatedWorkerV3RosterV1<M1SwiGluWorkerV3RosterV1>>,
    /// Uncomposed or rejected K7 owner.
    pub logits: Option<AuthenticatedWorkerV3RosterV1<M1LogitsWorkerV3RosterV1>>,
}

impl<K3R> fmt::Debug for M1AuthenticatedWorkerV3ProgramSetResidueV1<K3R> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedWorkerV3ProgramSetResidueV1")
            .field("programs", &self.programs)
            .field("has_gemm", &self.gemm.is_some())
            .field("has_rmsnorm", &self.rmsnorm.is_some())
            .field("has_rope_kv", &self.rope_kv.is_some())
            .field("has_prefill", &self.prefill.is_some())
            .field("has_paged_decode", &self.paged_decode.is_some())
            .field("has_swiglu", &self.swiglu.is_some())
            .field("has_logits", &self.logits.is_some())
            .finish()
    }
}

/// Exact intake phase that rejected authenticated roster custody.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedProgramSetIntakePhaseV1 {
    SourceFacts,
    Preflight(M1KernelArtifactFamilyV1),
    Compose(M1KernelArtifactFamilyV1),
    Aggregate,
}

/// Why seven authenticated M1 rosters did not become one exact 12-program set.
#[derive(Debug)]
#[non_exhaustive]
pub enum M1AuthenticatedProgramSetIntakeErrorV1 {
    SourceFacts(Box<M1KernelArtifactBuildErrorV1>),
    CurrentPublication {
        family: M1KernelArtifactFamilyV1,
        source: Box<RecoveredWorkerV3AdmissionErrorV1>,
    },
    Target {
        family: M1KernelArtifactFamilyV1,
        expected: AmdTargetId,
        actual: AmdTargetId,
    },
    EntryCount {
        family: M1KernelArtifactFamilyV1,
        expected: usize,
        actual: usize,
    },
    MarkerSymbol {
        family: M1KernelArtifactFamilyV1,
        ordinal: usize,
        expected: &'static str,
        logical: &'static str,
        export: &'static str,
    },
    MarkerIdentity {
        family: M1KernelArtifactFamilyV1,
        ordinal: usize,
    },
    VerificationEntry {
        family: M1KernelArtifactFamilyV1,
        ordinal: usize,
    },
    VerificationAuthority(M1KernelArtifactFamilyV1),
    EmptyFinalizedArtifact(M1KernelArtifactFamilyV1),
    DuplicateKernelBinding,
    K3MarkerType {
        ordinal: usize,
        expected: &'static str,
        logical: &'static str,
        export: &'static str,
    },
    SourceFamilyOrder {
        expected: M1KernelArtifactFamilyV1,
        actual: M1KernelArtifactFamilyV1,
    },
    ProgramSet {
        family: M1KernelArtifactFamilyV1,
        source: Box<AuthenticatedWorkerV3ProgramSetAdmissionErrorV1>,
    },
    AggregateCount {
        expected_rosters: usize,
        actual_rosters: usize,
        expected_programs: usize,
        actual_programs: usize,
    },
    ProgramIndex {
        program: M1PhysicalProgramV1,
        expected: usize,
        actual: Option<usize>,
    },
}

impl fmt::Display for M1AuthenticatedProgramSetIntakeErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "authenticated M1 Worker V3 program-set intake failed: {self:?}"
        )
    }
}

impl Error for M1AuthenticatedProgramSetIntakeErrorV1 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::SourceFacts(source) => Some(source),
            Self::CurrentPublication { source, .. } => Some(source),
            Self::ProgramSet { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Intake failure retaining every roster owner available at the rejection.
#[must_use = "intake failure retains authenticated roster custody"]
pub struct M1AuthenticatedProgramSetIntakeFailureV1<K3R> {
    phase: M1AuthenticatedProgramSetIntakePhaseV1,
    error: Box<M1AuthenticatedProgramSetIntakeErrorV1>,
    residue: Box<M1AuthenticatedWorkerV3ProgramSetResidueV1<K3R>>,
}

impl<K3R> M1AuthenticatedProgramSetIntakeFailureV1<K3R> {
    /// Returns the exact rejection phase.
    pub const fn phase(&self) -> M1AuthenticatedProgramSetIntakePhaseV1 {
        self.phase
    }

    /// Returns the exact rejection diagnostic.
    pub const fn error(&self) -> &M1AuthenticatedProgramSetIntakeErrorV1 {
        &self.error
    }

    /// Returns the exact diagnostic and every retained owner.
    pub fn into_parts(
        self,
    ) -> (
        M1AuthenticatedProgramSetIntakePhaseV1,
        M1AuthenticatedProgramSetIntakeErrorV1,
        M1AuthenticatedWorkerV3ProgramSetResidueV1<K3R>,
    ) {
        (self.phase, *self.error, *self.residue)
    }
}

impl<K3R> fmt::Debug for M1AuthenticatedProgramSetIntakeFailureV1<K3R> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedProgramSetIntakeFailureV1")
            .field("phase", &self.phase)
            .field("error", &self.error)
            .field("residue", &self.residue)
            .finish()
    }
}

/// Ferric-qualified custody of seven current rosters and exactly 12 programs.
#[must_use = "authenticated M1 program custody must remain retained"]
pub struct M1AuthenticatedWorkerV3ProgramSetV1 {
    programs: AuthenticatedWorkerV3ProgramSetV1,
    family_artifacts: Box<[DeclaredKernelFamilyArtifact]>,
    catalog_id: Identity,
}

impl fmt::Debug for M1AuthenticatedWorkerV3ProgramSetV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M1AuthenticatedWorkerV3ProgramSetV1")
            .field("roster_count", &self.programs.roster_count())
            .field("program_count", &self.programs.program_count())
            .field("target", &self.programs.target())
            .field("catalog_id", &self.catalog_id)
            .finish_non_exhaustive()
    }
}

impl M1AuthenticatedWorkerV3ProgramSetV1 {
    /// Returns the exact seven-roster count.
    pub fn roster_count(&self) -> usize {
        self.programs.roster_count()
    }

    /// Returns the exact flattened program count.
    pub fn program_count(&self) -> usize {
        self.programs.program_count()
    }

    /// Returns the exact common target.
    pub const fn target(&self) -> AmdTargetId {
        self.programs.target()
    }

    /// Returns the Ferric-domain-separated current program catalog identity.
    pub const fn catalog_id(&self) -> Identity {
        self.catalog_id
    }

    /// Resolves a compiler-generated marker in the retained program set.
    pub fn program_index<K: CompilerGeneratedKernelExpectationV1>(
        &self,
    ) -> Result<usize, AuthenticatedWorkerV3ProgramLookupErrorV1> {
        self.programs.program_index::<K>()
    }

    pub(crate) fn family_artifacts(&self) -> &[DeclaredKernelFamilyArtifact] {
        &self.family_artifacts
    }

    pub(crate) fn into_programs(self) -> AuthenticatedWorkerV3ProgramSetV1 {
        self.programs
    }

    /// Creates a queue only from this authenticated set and addressless packets.
    ///
    /// The generic fe2o3 failure retains the program set, allocation owner, and
    /// packets according to its exact rejection phase.
    pub fn create_service_queue<const N: usize>(
        self,
        allocations: ServiceAllocationSessionV1,
        ring_bytes: u32,
        packets: [ServiceFixedDispatchPacketV1; N],
    ) -> Result<AuthenticatedServiceQueueSessionV1<N>, AuthenticatedServiceQueueCreateFailureV1<N>>
    {
        AuthenticatedServiceQueueSessionV1::create(self.programs, allocations, ring_bytes, packets)
    }
}

/// Revalidates and reuses the same attached authenticated set.
pub fn reuse_m1_authenticated_service_queue_v1<const N: usize>(
    recycled: AuthenticatedServiceRecycledQueueSessionV1<N>,
) -> Result<
    AuthenticatedServiceQueueSessionV1<N>,
    AuthenticatedServiceCurrentnessFailureV1<AuthenticatedServiceRecycledQueueSessionV1<N>>,
> {
    recycled.reuse()
}

/// Binds a fresh move-only Ferric-qualified replacement set.
pub fn bind_m1_authenticated_service_queue_v1<const N: usize>(
    queue: AuthenticatedServiceQueueUnboundSessionV1,
    replacement: M1AuthenticatedWorkerV3ProgramSetV1,
    packets: [ServiceFixedDispatchPacketV1; N],
) -> Result<AuthenticatedServiceQueueSessionV1<N>, AuthenticatedServiceQueueBindFailureV1<N>> {
    queue.bind(replacement.into_programs(), packets)
}

/// Rolls over only with a fresh move-only Ferric-qualified replacement set.
pub fn rollover_m1_authenticated_service_queue_v1<const N: usize>(
    queue: AuthenticatedServiceQueueUnboundSessionV1,
    ring_bytes: u32,
    replacement: M1AuthenticatedWorkerV3ProgramSetV1,
    packets: [ServiceFixedDispatchPacketV1; N],
) -> Result<
    AuthenticatedServiceQueueRolloverSuccessV1<N>,
    AuthenticatedServiceQueueRolloverFailureV1<N>,
> {
    queue.rollover(ring_bytes, replacement.into_programs(), packets)
}

/// Typed fail-closed result for a legacy raw-artifact production entry point.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct M1AuthenticatedRosterAcquisitionRequiredV1 {
    artifact_root: PathBuf,
}

impl M1AuthenticatedRosterAcquisitionRequiredV1 {
    /// Returns the legacy path that cannot establish authenticated custody.
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
pub fn require_m1_authenticated_roster_acquisition_v1(
    artifact_root: &Path,
) -> Result<(), M1AuthenticatedRosterAcquisitionRequiredV1> {
    Err(M1AuthenticatedRosterAcquisitionRequiredV1 {
        artifact_root: artifact_root.to_path_buf(),
    })
}

/// Composes seven current authenticated rosters in canonical K1-K7 order.
///
/// K3 marker types must be the two real compiler-generated markers from the
/// caller's K3 roster. This API intentionally cannot be concretely instantiated
/// by Ferric until that generated crate is integrated.
pub fn admit_m1_authenticated_worker_v3_programs_v1<K3R, K3Rope, K3PagedKv>(
    rosters: M1AuthenticatedWorkerV3RostersV1<K3R>,
) -> Result<M1AuthenticatedWorkerV3ProgramSetV1, M1AuthenticatedProgramSetIntakeFailureV1<K3R>>
where
    K3R: CompilerGeneratedKernelExpectationRosterV1,
    K3Rope: CompilerGeneratedKernelExpectationV1,
    K3PagedKv: CompilerGeneratedKernelExpectationV1,
{
    let facts = match current_m1_kernel_source_facts_v1() {
        Ok(facts) => facts,
        Err(error) => {
            return Err(intake_failure(
                M1AuthenticatedProgramSetIntakePhaseV1::SourceFacts,
                M1AuthenticatedProgramSetIntakeErrorV1::SourceFacts(Box::new(error)),
                rosters.into_residue(),
            ));
        }
    };
    if let Err(error) = validate_source_family_order(&facts) {
        return Err(intake_failure(
            M1AuthenticatedProgramSetIntakePhaseV1::SourceFacts,
            error,
            rosters.into_residue(),
        ));
    }
    if let Err((family, error)) = validate_rosters::<K3R, K3Rope, K3PagedKv>(&rosters) {
        return Err(intake_failure(
            M1AuthenticatedProgramSetIntakePhaseV1::Preflight(family),
            error,
            rosters.into_residue(),
        ));
    }
    let catalog_id = authenticated_catalog_id(&facts, &rosters);
    let family_artifacts = authenticated_family_artifacts(&facts, &rosters);

    let M1AuthenticatedWorkerV3RostersV1 {
        gemm,
        rmsnorm,
        rope_kv,
        prefill,
        paged_decode,
        swiglu,
        logits,
    } = rosters;
    let mut residue = M1AuthenticatedWorkerV3ProgramSetResidueV1 {
        programs: None,
        gemm: None,
        rmsnorm: Some(rmsnorm),
        rope_kv: Some(rope_kv),
        prefill: Some(prefill),
        paged_decode: Some(paged_decode),
        swiglu: Some(swiglu),
        logits: Some(logits),
    };
    let mut programs = match AuthenticatedWorkerV3ProgramSetV1::from_roster(gemm) {
        Ok(programs) => programs,
        Err(failure) => {
            let (error, gemm) = failure.into_parts();
            residue.gemm = Some(gemm);
            return Err(intake_failure(
                M1AuthenticatedProgramSetIntakePhaseV1::Compose(M1KernelArtifactFamilyV1::Gemm),
                M1AuthenticatedProgramSetIntakeErrorV1::ProgramSet {
                    family: M1KernelArtifactFamilyV1::Gemm,
                    source: Box::new(error),
                },
                residue,
            ));
        }
    };

    macro_rules! append_roster {
        ($field:ident, $family:expr) => {{
            let roster = residue
                .$field
                .take()
                .expect("canonical intake retains each uncomposed roster");
            programs = match programs.append_roster(roster) {
                Ok(programs) => programs,
                Err(failure) => {
                    let (error, retained, roster) = failure.into_parts();
                    residue.programs = Some(retained);
                    residue.$field = Some(roster);
                    return Err(intake_failure(
                        M1AuthenticatedProgramSetIntakePhaseV1::Compose($family),
                        M1AuthenticatedProgramSetIntakeErrorV1::ProgramSet {
                            family: $family,
                            source: Box::new(error),
                        },
                        residue,
                    ));
                }
            };
        }};
    }

    append_roster!(rmsnorm, M1KernelArtifactFamilyV1::RmsNorm);
    append_roster!(rope_kv, M1KernelArtifactFamilyV1::RopeKv);
    append_roster!(prefill, M1KernelArtifactFamilyV1::Prefill);
    append_roster!(paged_decode, M1KernelArtifactFamilyV1::PagedDecode);
    append_roster!(swiglu, M1KernelArtifactFamilyV1::SwiGlu);
    append_roster!(logits, M1KernelArtifactFamilyV1::Logits);

    if programs.roster_count() != M1_AUTHENTICATED_ROSTER_COUNT_V1
        || programs.program_count() != M1_PHYSICAL_PROGRAM_COUNT_V1
    {
        let error = M1AuthenticatedProgramSetIntakeErrorV1::AggregateCount {
            expected_rosters: M1_AUTHENTICATED_ROSTER_COUNT_V1,
            actual_rosters: programs.roster_count(),
            expected_programs: M1_PHYSICAL_PROGRAM_COUNT_V1,
            actual_programs: programs.program_count(),
        };
        residue.programs = Some(programs);
        return Err(intake_failure(
            M1AuthenticatedProgramSetIntakePhaseV1::Aggregate,
            error,
            residue,
        ));
    }
    if let Err(error) = validate_program_indices::<K3Rope, K3PagedKv>(&programs) {
        residue.programs = Some(programs);
        return Err(intake_failure(
            M1AuthenticatedProgramSetIntakePhaseV1::Aggregate,
            error,
            residue,
        ));
    }
    Ok(M1AuthenticatedWorkerV3ProgramSetV1 {
        programs,
        family_artifacts,
        catalog_id,
    })
}

fn intake_failure<K3R>(
    phase: M1AuthenticatedProgramSetIntakePhaseV1,
    error: M1AuthenticatedProgramSetIntakeErrorV1,
    residue: M1AuthenticatedWorkerV3ProgramSetResidueV1<K3R>,
) -> M1AuthenticatedProgramSetIntakeFailureV1<K3R> {
    M1AuthenticatedProgramSetIntakeFailureV1 {
        phase,
        error: Box::new(error),
        residue: Box::new(residue),
    }
}

fn validate_source_family_order(
    facts: &[M1CurrentKernelSourceFactsV1; M1_AUTHENTICATED_ROSTER_COUNT_V1],
) -> Result<(), M1AuthenticatedProgramSetIntakeErrorV1> {
    for (fact, expected) in facts.iter().zip(M1KernelArtifactFamilyV1::ALL) {
        if fact.family() != expected {
            return Err(M1AuthenticatedProgramSetIntakeErrorV1::SourceFamilyOrder {
                expected,
                actual: fact.family(),
            });
        }
    }
    Ok(())
}

fn validate_rosters<K3R, K3Rope, K3PagedKv>(
    rosters: &M1AuthenticatedWorkerV3RostersV1<K3R>,
) -> Result<
    (),
    (
        M1KernelArtifactFamilyV1,
        M1AuthenticatedProgramSetIntakeErrorV1,
    ),
>
where
    K3R: CompilerGeneratedKernelExpectationRosterV1,
    K3Rope: CompilerGeneratedKernelExpectationV1,
    K3PagedKv: CompilerGeneratedKernelExpectationV1,
{
    let checks = [
        validate_roster(
            M1KernelArtifactFamilyV1::Gemm,
            &rosters.gemm,
            &M1PhysicalProgramV1::ALL[0..3],
        ),
        validate_roster(
            M1KernelArtifactFamilyV1::RmsNorm,
            &rosters.rmsnorm,
            &M1PhysicalProgramV1::ALL[3..4],
        ),
        validate_roster(
            M1KernelArtifactFamilyV1::RopeKv,
            &rosters.rope_kv,
            &M1PhysicalProgramV1::ALL[4..6],
        ),
        validate_roster(
            M1KernelArtifactFamilyV1::Prefill,
            &rosters.prefill,
            &M1PhysicalProgramV1::ALL[6..7],
        ),
        validate_roster(
            M1KernelArtifactFamilyV1::PagedDecode,
            &rosters.paged_decode,
            &M1PhysicalProgramV1::ALL[7..8],
        ),
        validate_roster(
            M1KernelArtifactFamilyV1::SwiGlu,
            &rosters.swiglu,
            &M1PhysicalProgramV1::ALL[8..9],
        ),
        validate_roster(
            M1KernelArtifactFamilyV1::Logits,
            &rosters.logits,
            &M1PhysicalProgramV1::ALL[9..12],
        ),
    ];
    for check in checks {
        check?;
    }
    validate_k3_marker_types::<K3R, K3Rope, K3PagedKv>()
        .map_err(|error| (M1KernelArtifactFamilyV1::RopeKv, error))?;

    let mut bindings = Vec::with_capacity(M1_PHYSICAL_PROGRAM_COUNT_V1);
    append_bindings::<M1GemmWorkerV3RosterV1>(&mut bindings, M1KernelArtifactFamilyV1::Gemm)?;
    append_bindings::<M1RmsNormWorkerV3RosterV1>(&mut bindings, M1KernelArtifactFamilyV1::RmsNorm)?;
    append_bindings::<K3R>(&mut bindings, M1KernelArtifactFamilyV1::RopeKv)?;
    append_bindings::<M1PrefillWorkerV3RosterV1>(&mut bindings, M1KernelArtifactFamilyV1::Prefill)?;
    append_bindings::<M1PagedDecodeWorkerV3RosterV1>(
        &mut bindings,
        M1KernelArtifactFamilyV1::PagedDecode,
    )?;
    append_bindings::<M1SwiGluWorkerV3RosterV1>(&mut bindings, M1KernelArtifactFamilyV1::SwiGlu)?;
    append_bindings::<M1LogitsWorkerV3RosterV1>(&mut bindings, M1KernelArtifactFamilyV1::Logits)?;
    Ok(())
}

fn validate_roster<R: CompilerGeneratedKernelExpectationRosterV1>(
    family: M1KernelArtifactFamilyV1,
    roster: &AuthenticatedWorkerV3RosterV1<R>,
    programs: &[M1PhysicalProgramV1],
) -> Result<
    (),
    (
        M1KernelArtifactFamilyV1,
        M1AuthenticatedProgramSetIntakeErrorV1,
    ),
> {
    let expected_target =
        AmdTargetId::parse(M1_AUTHENTICATED_PROGRAM_TARGET_V1).expect("fixed target is canonical");
    roster.revalidate_currentness().map_err(|source| {
        (
            family,
            M1AuthenticatedProgramSetIntakeErrorV1::CurrentPublication {
                family,
                source: Box::new(source),
            },
        )
    })?;
    if roster.target() != expected_target {
        return Err((
            family,
            M1AuthenticatedProgramSetIntakeErrorV1::Target {
                family,
                expected: expected_target,
                actual: roster.target(),
            },
        ));
    }
    if roster.entry_count() != programs.len() || R::ENTRIES.len() != programs.len() {
        return Err((
            family,
            M1AuthenticatedProgramSetIntakeErrorV1::EntryCount {
                family,
                expected: programs.len(),
                actual: roster.entry_count(),
            },
        ));
    }
    let verification = roster.verification();
    if !roster.authenticates_verification_authority()
        || !verification.retains_current_compiler_and_signed_verus_evidence()
        || verification.validated_compiler_proof_inputs().is_none()
        || verification.validated_compiler_target_lineage().is_none()
    {
        return Err((
            family,
            M1AuthenticatedProgramSetIntakeErrorV1::VerificationAuthority(family),
        ));
    }
    if verification.finalized_hsaco_length() == 0
        || verification.finalized_hsaco_sha256() == [0; 32]
    {
        return Err((
            family,
            M1AuthenticatedProgramSetIntakeErrorV1::EmptyFinalizedArtifact(family),
        ));
    }
    for (ordinal, (entry, program)) in R::ENTRIES.iter().zip(programs).enumerate() {
        let expected = program.kernel_symbol();
        if entry.logical_name() != expected || entry.export_name() != expected {
            return Err((
                family,
                M1AuthenticatedProgramSetIntakeErrorV1::MarkerSymbol {
                    family,
                    ordinal,
                    expected,
                    logical: entry.logical_name(),
                    export: entry.export_name(),
                },
            ));
        }
        if entry.kernel_binding_id() == [0; 32]
            || entry.generated_host_contract_identity() == [0; 32]
        {
            return Err((
                family,
                M1AuthenticatedProgramSetIntakeErrorV1::MarkerIdentity { family, ordinal },
            ));
        }
        let Some(evidence) = verification.entries().get(ordinal) else {
            return Err((
                family,
                M1AuthenticatedProgramSetIntakeErrorV1::VerificationEntry { family, ordinal },
            ));
        };
        if evidence.marker_binding_identity() != entry.kernel_binding_id()
            || evidence.generated_host_contract_identity()
                != entry.generated_host_contract_identity()
        {
            return Err((
                family,
                M1AuthenticatedProgramSetIntakeErrorV1::VerificationEntry { family, ordinal },
            ));
        }
    }
    Ok(())
}

fn validate_k3_marker_types<K3R, K3Rope, K3PagedKv>(
) -> Result<(), M1AuthenticatedProgramSetIntakeErrorV1>
where
    K3R: CompilerGeneratedKernelExpectationRosterV1,
    K3Rope: CompilerGeneratedKernelExpectationV1,
    K3PagedKv: CompilerGeneratedKernelExpectationV1,
{
    let expected = [
        M1PhysicalProgramV1::Rope.kernel_symbol(),
        M1PhysicalProgramV1::PagedKvWrite.kernel_symbol(),
    ];
    let logical = [K3Rope::LOGICAL_NAME, K3PagedKv::LOGICAL_NAME];
    let export = [K3Rope::EXPORT_NAME, K3PagedKv::EXPORT_NAME];
    let bindings = [
        K3Rope::KERNEL_BINDING_ID_V1,
        K3PagedKv::KERNEL_BINDING_ID_V1,
    ];
    let contracts = [
        K3Rope::PROFILE.generated_host_contract_identity(),
        K3PagedKv::PROFILE.generated_host_contract_identity(),
    ];
    for ordinal in 0..2 {
        if logical[ordinal] != expected[ordinal] || export[ordinal] != expected[ordinal] {
            return Err(M1AuthenticatedProgramSetIntakeErrorV1::K3MarkerType {
                ordinal,
                expected: expected[ordinal],
                logical: logical[ordinal],
                export: export[ordinal],
            });
        }
        let entry = &K3R::ENTRIES[ordinal];
        if entry.logical_name() != logical[ordinal]
            || entry.export_name() != export[ordinal]
            || entry.kernel_binding_id() != bindings[ordinal]
            || entry.generated_host_contract_identity() != contracts[ordinal]
        {
            return Err(M1AuthenticatedProgramSetIntakeErrorV1::K3MarkerType {
                ordinal,
                expected: expected[ordinal],
                logical: entry.logical_name(),
                export: entry.export_name(),
            });
        }
    }
    Ok(())
}

fn append_bindings<R: CompilerGeneratedKernelExpectationRosterV1>(
    bindings: &mut Vec<[u8; 32]>,
    family: M1KernelArtifactFamilyV1,
) -> Result<
    (),
    (
        M1KernelArtifactFamilyV1,
        M1AuthenticatedProgramSetIntakeErrorV1,
    ),
> {
    for entry in R::ENTRIES {
        let binding = entry.kernel_binding_id();
        if bindings.contains(&binding) {
            return Err((
                family,
                M1AuthenticatedProgramSetIntakeErrorV1::DuplicateKernelBinding,
            ));
        }
        bindings.push(binding);
    }
    Ok(())
}

fn validate_program_indices<K3Rope, K3PagedKv>(
    programs: &AuthenticatedWorkerV3ProgramSetV1,
) -> Result<(), M1AuthenticatedProgramSetIntakeErrorV1>
where
    K3Rope: CompilerGeneratedKernelExpectationV1,
    K3PagedKv: CompilerGeneratedKernelExpectationV1,
{
    let actual = [
        programs.program_index::<GemmReferenceMarkerV1>().ok(),
        programs.program_index::<GemmVectorizedMarkerV1>().ok(),
        programs.program_index::<TokenEmbeddingMarkerV1>().ok(),
        programs.program_index::<RmsNormMarkerV1>().ok(),
        programs.program_index::<K3Rope>().ok(),
        programs.program_index::<K3PagedKv>().ok(),
        programs.program_index::<PrefillMarkerV1>().ok(),
        programs.program_index::<PagedDecodeMarkerV1>().ok(),
        programs.program_index::<SwiGluMarkerV1>().ok(),
        programs.program_index::<LogitsArgmaxMarkerV1>().ok(),
        programs.program_index::<LogitsCompactMarkerV1>().ok(),
        programs.program_index::<SpeculativeAssemblyMarkerV1>().ok(),
    ];
    for (program, actual) in M1PhysicalProgramV1::ALL.into_iter().zip(actual) {
        if actual != Some(program.program_index()) {
            return Err(M1AuthenticatedProgramSetIntakeErrorV1::ProgramIndex {
                program,
                expected: program.program_index(),
                actual,
            });
        }
    }
    Ok(())
}

fn authenticated_family_artifacts<K3R>(
    facts: &[M1CurrentKernelSourceFactsV1; M1_AUTHENTICATED_ROSTER_COUNT_V1],
    rosters: &M1AuthenticatedWorkerV3RostersV1<K3R>,
) -> Box<[DeclaredKernelFamilyArtifact]>
where
    K3R: CompilerGeneratedKernelExpectationRosterV1,
{
    let finalized = [
        rosters.gemm.verification().finalized_hsaco_sha256(),
        rosters.rmsnorm.verification().finalized_hsaco_sha256(),
        rosters.rope_kv.verification().finalized_hsaco_sha256(),
        rosters.prefill.verification().finalized_hsaco_sha256(),
        rosters.paged_decode.verification().finalized_hsaco_sha256(),
        rosters.swiglu.verification().finalized_hsaco_sha256(),
        rosters.logits.verification().finalized_hsaco_sha256(),
    ];
    facts
        .iter()
        .zip(finalized)
        .map(|(fact, artifact)| {
            DeclaredKernelFamilyArtifact::new(
                kernel_family(fact.family()),
                Identity::new(*fact.compiler_handoff().sha256()),
                Identity::new(artifact),
                Identity::new(*fact.symbol_manifest().sha256()),
            )
        })
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

fn authenticated_catalog_id<K3R>(
    facts: &[M1CurrentKernelSourceFactsV1; M1_AUTHENTICATED_ROSTER_COUNT_V1],
    rosters: &M1AuthenticatedWorkerV3RostersV1<K3R>,
) -> Identity
where
    K3R: CompilerGeneratedKernelExpectationRosterV1,
{
    let verifications = [
        rosters.gemm.verification(),
        rosters.rmsnorm.verification(),
        rosters.rope_kv.verification(),
        rosters.prefill.verification(),
        rosters.paged_decode.verification(),
        rosters.swiglu.verification(),
        rosters.logits.verification(),
    ];
    let mut digest = Sha256::new();
    digest.update(M1_AUTHENTICATED_PROGRAM_CATALOG_DOMAIN_V1);
    digest.update((M1_AUTHENTICATED_ROSTER_COUNT_V1 as u64).to_le_bytes());
    digest.update((M1_PHYSICAL_PROGRAM_COUNT_V1 as u64).to_le_bytes());
    for (fact, verification) in facts.iter().zip(verifications) {
        digest.update([fact.family() as u8]);
        digest.update(fact.compiler_handoff().sha256());
        digest.update(fact.compiler_handoff().byte_len().to_le_bytes());
        digest.update(fact.symbol_manifest().sha256());
        digest.update(fact.symbol_manifest().byte_len().to_le_bytes());
        digest.update(verification.lineage_identity().as_bytes());
        digest.update(verification.roster_identity().as_bytes());
        digest.update(verification.finalized_hsaco_sha256());
        digest.update(verification.finalized_hsaco_length().to_le_bytes());
        for entry in verification.entries() {
            digest.update(entry.marker_binding_identity());
            digest.update(entry.generated_host_contract_identity());
        }
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

    #[test]
    fn integrated_rosters_cover_ten_exact_compiler_generated_markers() {
        let rosters: [&[fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1]; 6] = [
            M1GemmWorkerV3RosterV1::ENTRIES,
            M1RmsNormWorkerV3RosterV1::ENTRIES,
            M1PrefillWorkerV3RosterV1::ENTRIES,
            M1PagedDecodeWorkerV3RosterV1::ENTRIES,
            M1SwiGluWorkerV3RosterV1::ENTRIES,
            M1LogitsWorkerV3RosterV1::ENTRIES,
        ];
        assert_eq!(rosters.iter().map(|roster| roster.len()).sum::<usize>(), 10);
        let symbols = rosters
            .into_iter()
            .flat_map(|roster| roster.iter().map(|entry| entry.export_name()))
            .collect::<Vec<_>>();
        let expected = M1PhysicalProgramV1::ALL
            .into_iter()
            .filter(|program| {
                !matches!(
                    program,
                    M1PhysicalProgramV1::Rope | M1PhysicalProgramV1::PagedKvWrite
                )
            })
            .map(M1PhysicalProgramV1::kernel_symbol)
            .collect::<Vec<_>>();
        assert_eq!(symbols, expected);
    }

    #[test]
    fn canonical_contract_reserves_exact_k3_slots() {
        assert_eq!(M1PhysicalProgramV1::Rope.program_index(), 4);
        assert_eq!(M1PhysicalProgramV1::PagedKvWrite.program_index(), 5);
        assert_eq!(M1PhysicalProgramV1::Rope.kernel_symbol(), "qwen3_rope_v1");
        assert_eq!(
            M1PhysicalProgramV1::PagedKvWrite.kernel_symbol(),
            "qwen3_paged_kv_write_v1"
        );
    }

    #[test]
    fn path_only_acquisition_is_typed_and_fail_closed() {
        let path = Path::new("/kernel-artifacts");
        let error = require_m1_authenticated_roster_acquisition_v1(path)
            .expect_err("a path cannot authenticate Worker V3 custody");
        assert_eq!(error.artifact_root(), path);
        assert!(error.to_string().contains("roster acquisition is required"));
    }
}
