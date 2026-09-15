//! Content-bound physical program selection for the exact M1 Qwen artifacts.
//!
//! The compiler-facing Qwen owners have already checked their Worker lineage,
//! complete HSACO inventory, ABI, resources, and allocation-free load plan.
//! This module revalidates those retained bytes through the generic fe2o3
//! loader and binds the exact selected scalar or MFMA program roster.
//! It does not independently approve deployment bytes, allocate or load an
//! image, construct kernargs, publish a queue, observe completion, prove
//! refinement, or report hardware or performance evidence.

use std::fmt;

use fe2o3_amdhsa_loader::{
    AdmittedProfile, KernelClosureError, KernelDispatchAbiErrorV1, KernelGlobalBufferAbiV1,
    LoadPlan, PlanError, ValidatedKernelEnvelope,
};
use ferric_qwen_kernels::{gemm, logits, paged_decode, prefill, rmsnorm, rope_kv, swiglu};
use ferric_spec::Identity;
use sha2::{Digest, Sha256};

const PROGRAM_CATALOG_IDENTITY_DOMAIN: &[u8] = b"ferric.m1.physical-program-catalog.v1";
const PROGRAM_SOURCE_CONTRACT_IDENTITY_DOMAIN: &[u8] =
    b"ferric.m1.physical-program-source-contract.v1";

/// Exact number of selected entry points across the seven M1 kernel artifacts.
pub const M1_PHYSICAL_PROGRAM_COUNT_V1: usize = 12;

/// Exact program count for the separately attributed MFMA aggregate.
pub const M1_MFMA_PHYSICAL_PROGRAM_COUNT_V1: usize = 13;

/// Closed program and numerical-profile strategy retained by artifact custody.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1PhysicalProgramStrategyV1 {
    /// The unchanged scalar/vectorized twelve-program catalog.
    LegacyScalar12,
    /// The thirteen-program attributed aggregate with multi-row MFMA GEMM.
    AttributedMfma13,
}

impl M1PhysicalProgramStrategyV1 {
    /// Exact stable ordinal roster required by this strategy.
    #[must_use]
    pub const fn program_roster(self) -> &'static [M1PhysicalProgramV1] {
        match self {
            Self::LegacyScalar12 => &M1PhysicalProgramV1::ALL,
            Self::AttributedMfma13 => &M1PhysicalProgramV1::MFMA_ALL,
        }
    }

    /// Exact selected program count, not an upper bound.
    #[must_use]
    pub const fn program_count(self) -> usize {
        self.program_roster().len()
    }

    /// Returns the profile catalog whose numerical policy belongs to this strategy.
    ///
    /// # Errors
    /// Returns a checked canonical profile construction error.
    pub fn gemm_profiles(
        self,
    ) -> Result<gemm::Qwen3GemmProfileCatalogV1, gemm::Qwen3GemmCatalogErrorV1> {
        match self {
            Self::LegacyScalar12 => gemm::Qwen3GemmProfileCatalogV1::canonical(),
            Self::AttributedMfma13 => gemm::Qwen3GemmProfileCatalogV1::canonical_mfma(),
        }
    }
}

/// Compiler-handoff lineage used to bind a program-specific Ferric ABI roster.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct M1PhysicalProgramSourceContractV1 {
    compiler_handoff_sha256: [u8; 32],
    compiler_handoff_byte_len: u64,
}

impl M1PhysicalProgramSourceContractV1 {
    pub(crate) const fn new(
        compiler_handoff_sha256: [u8; 32],
        compiler_handoff_byte_len: u64,
    ) -> Self {
        Self {
            compiler_handoff_sha256,
            compiler_handoff_byte_len,
        }
    }
}

/// Stable physical-program ordinal used by future fixed packet batches.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum M1PhysicalProgramV1 {
    /// Scalar/reference BF16/FP32 GEMM path.
    GemmReference = 0,
    /// Vectorized BF16/FP32 GEMM/GEMV path.
    GemmVectorized = 1,
    /// Token embedding lookup path.
    TokenEmbedding = 2,
    /// Mode-tagged RMSNorm/residual path.
    RmsNorm = 3,
    /// Rotary-position transform path.
    Rope = 4,
    /// Paged key/value cache write path.
    PagedKvWrite = 5,
    /// Causal paged prefill attention path.
    GqaPrefill = 6,
    /// Paged grouped-query decode attention path.
    PagedGqaDecode = 7,
    /// `SwiGLU` activation path.
    SwiGlu = 8,
    /// Lowest-token-ID argmax path.
    LogitsArgmax = 9,
    /// Target compact-completion path.
    LogitsCompact = 10,
    /// In-batch speculative target-token assembly infrastructure path.
    SpeculativeTokenAssembly = 11,
    /// BF16 K16 MFMA with FP32 accumulation for multi-row GEMM.
    GemmMfma = 12,
}

impl M1PhysicalProgramV1 {
    /// Complete stable program order expected by packet descriptions.
    pub const ALL: [Self; M1_PHYSICAL_PROGRAM_COUNT_V1] = [
        Self::GemmReference,
        Self::GemmVectorized,
        Self::TokenEmbedding,
        Self::RmsNorm,
        Self::Rope,
        Self::PagedKvWrite,
        Self::GqaPrefill,
        Self::PagedGqaDecode,
        Self::SwiGlu,
        Self::LogitsArgmax,
        Self::LogitsCompact,
        Self::SpeculativeTokenAssembly,
    ];

    /// The attributed MFMA roster preserves all legacy ordinals and appends MFMA.
    pub const MFMA_ALL: [Self; M1_MFMA_PHYSICAL_PROGRAM_COUNT_V1] = [
        Self::GemmReference,
        Self::GemmVectorized,
        Self::TokenEmbedding,
        Self::RmsNorm,
        Self::Rope,
        Self::PagedKvWrite,
        Self::GqaPrefill,
        Self::PagedGqaDecode,
        Self::SwiGlu,
        Self::LogitsArgmax,
        Self::LogitsCompact,
        Self::SpeculativeTokenAssembly,
        Self::GemmMfma,
    ];

    /// Stable structural program ordinal.
    ///
    /// Authenticated Worker V3 service indices are resolved independently from canonical
    /// descriptor order.
    #[must_use]
    pub const fn program_index(self) -> usize {
        self as usize
    }

    /// Exact selected metadata kernel name.
    #[must_use]
    pub const fn kernel_symbol(self) -> &'static str {
        match self {
            Self::GemmReference => gemm::QWEN3_GEMM_REFERENCE_KERNEL_SYMBOL_V1,
            Self::GemmVectorized => gemm::QWEN3_GEMM_VECTORIZED_KERNEL_SYMBOL_V1,
            Self::GemmMfma => gemm::QWEN3_GEMM_MFMA_KERNEL_SYMBOL_V1,
            Self::TokenEmbedding => gemm::QWEN3_TOKEN_EMBEDDING_KERNEL_SYMBOL_V1,
            Self::RmsNorm => rmsnorm::QWEN3_RMSNORM_KERNEL_SYMBOL_V1,
            Self::Rope => rope_kv::QWEN3_ROPE_KERNEL_SYMBOL_V1,
            Self::PagedKvWrite => rope_kv::QWEN3_PAGED_KV_WRITE_KERNEL_SYMBOL_V1,
            Self::GqaPrefill => prefill::QWEN3_PREFILL_KERNEL_SYMBOL_V1,
            Self::PagedGqaDecode => paged_decode::QWEN3_PAGED_DECODE_KERNEL_SYMBOL_V1,
            Self::SwiGlu => swiglu::QWEN3_SWIGLU_KERNEL_SYMBOL_V1,
            Self::LogitsArgmax => logits::QWEN3_LOGITS_ARGMAX_KERNEL_SYMBOL_V1,
            Self::LogitsCompact => logits::QWEN3_LOGITS_COMPACT_KERNEL_SYMBOL_V1,
            Self::SpeculativeTokenAssembly => {
                logits::QWEN3_SPECULATIVE_TOKEN_ASSEMBLY_KERNEL_SYMBOL_V1
            }
        }
    }
}

/// The retained Ferric artifact containing one selected entry point.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1PhysicalProgramFamilyV1 {
    /// K1 GEMM/GEMV and embedding artifact.
    Gemm,
    /// K2 RMSNorm/residual artifact.
    RmsNorm,
    /// K3 `RoPE` and paged-KV-write artifact.
    RopeKv,
    /// K4 prefill-attention artifact.
    Prefill,
    /// K5 paged-decode artifact.
    PagedDecode,
    /// K6 `SwiGLU` artifact.
    SwiGlu,
    /// K7 logits/compact-completion artifact.
    Logits,
}

impl M1PhysicalProgramV1 {
    /// Exact Ferric artifact family that must contain this entry point.
    #[must_use]
    pub const fn family(self) -> M1PhysicalProgramFamilyV1 {
        match self {
            Self::GemmReference | Self::GemmVectorized | Self::GemmMfma | Self::TokenEmbedding => {
                M1PhysicalProgramFamilyV1::Gemm
            }
            Self::RmsNorm => M1PhysicalProgramFamilyV1::RmsNorm,
            Self::Rope | Self::PagedKvWrite => M1PhysicalProgramFamilyV1::RopeKv,
            Self::GqaPrefill => M1PhysicalProgramFamilyV1::Prefill,
            Self::PagedGqaDecode => M1PhysicalProgramFamilyV1::PagedDecode,
            Self::SwiGlu => M1PhysicalProgramFamilyV1::SwiGlu,
            Self::LogitsArgmax | Self::LogitsCompact | Self::SpeculativeTokenAssembly => {
                M1PhysicalProgramFamilyV1::Logits
            }
        }
    }
}

/// Borrowed custody of every structurally inspected Ferric M1 kernel artifact.
///
/// Construction groups existing non-clone owners without granting new
/// compiler, artifact, load, allocation, or execution authority.
#[derive(Clone, Copy)]
pub struct InspectedM1KernelArtifacts<'a> {
    gemm: &'a gemm::InspectedQwen3GemmKernelV1,
    rmsnorm: &'a rmsnorm::InspectedQwen3RmsNormKernelV1,
    rope_kv: &'a rope_kv::InspectedQwen3RopeKvKernelV1,
    prefill: &'a prefill::InspectedQwen3PrefillKernelV1,
    paged_decode: &'a paged_decode::InspectedQwen3PagedDecodeKernelV1,
    swiglu: &'a swiglu::InspectedQwen3SwiGluKernelV1,
    logits: &'a logits::InspectedQwen3LogitsKernelV1,
}

impl<'a> InspectedM1KernelArtifacts<'a> {
    /// Groups the seven exact inspected artifact owners.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        gemm: &'a gemm::InspectedQwen3GemmKernelV1,
        rmsnorm: &'a rmsnorm::InspectedQwen3RmsNormKernelV1,
        rope_kv: &'a rope_kv::InspectedQwen3RopeKvKernelV1,
        prefill: &'a prefill::InspectedQwen3PrefillKernelV1,
        paged_decode: &'a paged_decode::InspectedQwen3PagedDecodeKernelV1,
        swiglu: &'a swiglu::InspectedQwen3SwiGluKernelV1,
        logits: &'a logits::InspectedQwen3LogitsKernelV1,
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

    fn bytes_plan_and_source(
        self,
        program: M1PhysicalProgramV1,
    ) -> (&'a [u8], LoadPlan, M1PhysicalProgramSourceContractV1) {
        match program.family() {
            M1PhysicalProgramFamilyV1::Gemm => {
                let identity = self.gemm.compiler_handoff_identity();
                (
                    self.gemm.exact_worker_output_bytes(),
                    *self.gemm.loader_plan(),
                    M1PhysicalProgramSourceContractV1::new(*identity.sha256(), identity.byte_len()),
                )
            }
            M1PhysicalProgramFamilyV1::RmsNorm => {
                let identity = self.rmsnorm.compiler_handoff_identity();
                (
                    self.rmsnorm.exact_worker_output_bytes(),
                    *self.rmsnorm.loader_plan(),
                    M1PhysicalProgramSourceContractV1::new(*identity.sha256(), identity.byte_len()),
                )
            }
            M1PhysicalProgramFamilyV1::RopeKv => {
                let identity = self.rope_kv.compiler_handoff_identity();
                (
                    self.rope_kv.exact_worker_output_bytes(),
                    *self.rope_kv.loader_plan(),
                    M1PhysicalProgramSourceContractV1::new(*identity.sha256(), identity.byte_len()),
                )
            }
            M1PhysicalProgramFamilyV1::Prefill => {
                let identity = self.prefill.compiler_handoff_identity();
                (
                    self.prefill.exact_worker_output_bytes(),
                    *self.prefill.loader_plan(),
                    M1PhysicalProgramSourceContractV1::new(*identity.sha256(), identity.byte_len()),
                )
            }
            M1PhysicalProgramFamilyV1::PagedDecode => {
                let identity = self.paged_decode.compiler_handoff_identity();
                (
                    self.paged_decode.exact_worker_output_bytes(),
                    *self.paged_decode.loader_plan(),
                    M1PhysicalProgramSourceContractV1::new(*identity.sha256(), identity.byte_len()),
                )
            }
            M1PhysicalProgramFamilyV1::SwiGlu => {
                let identity = self.swiglu.compiler_handoff_identity();
                (
                    self.swiglu.exact_worker_output_bytes(),
                    *self.swiglu.loader_plan(),
                    M1PhysicalProgramSourceContractV1::new(*identity.sha256(), identity.byte_len()),
                )
            }
            M1PhysicalProgramFamilyV1::Logits => {
                let identity = self.logits.compiler_handoff_identity();
                (
                    self.logits.exact_worker_output_bytes(),
                    *self.logits.loader_plan(),
                    M1PhysicalProgramSourceContractV1::new(*identity.sha256(), identity.byte_len()),
                )
            }
        }
    }
}

impl fmt::Debug for InspectedM1KernelArtifacts<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InspectedM1KernelArtifacts")
            .field("family_count", &7)
            .finish_non_exhaustive()
    }
}

/// Failure while revalidating and selecting a physical program.
#[derive(Debug)]
pub enum M1PhysicalProgramCatalogErrorV1 {
    /// Whole-object inspection rejected the uniform aggregate before selection.
    AggregateInspection(fe2o3_hsaco::InspectionError),
    /// The actual aggregate inventory differs from the exact selected strategy.
    AggregateRoster(M1PhysicalProgramStrategyV1),
    /// The generic allocation-free COV6 loader rejected retained bytes.
    Loader {
        /// Program whose containing bytes were rejected.
        program: M1PhysicalProgramV1,
        /// Exact generic loader error.
        error: PlanError,
    },
    /// Revalidation did not reproduce the plan retained by Ferric inspection.
    LoaderPlanDrift(M1PhysicalProgramFamilyV1),
    /// Exact semantic kernel selection failed.
    KernelClosure {
        /// Program that could not be selected.
        program: M1PhysicalProgramV1,
        /// Exact generic semantic-closure error.
        error: KernelClosureError,
    },
    /// Ferric's complete source ABI roster did not reconcile with the object.
    DispatchAbi {
        /// Program whose source/physical ABI join failed.
        program: M1PhysicalProgramV1,
        /// Exact generic reconciliation error.
        error: KernelDispatchAbiErrorV1,
    },
}

impl fmt::Display for M1PhysicalProgramCatalogErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "M1 physical program catalog rejected: {self:?}")
    }
}

impl std::error::Error for M1PhysicalProgramCatalogErrorV1 {}

/// Content-bound custody of one exact selected scalar or MFMA program roster.
///
/// The selected closures borrow their exact inspected Worker output bytes. This
/// owner intentionally does not implement `Clone` and must be consumed to move
/// the closures into a service batch.
///
/// ```compile_fail
/// use ferric_engine::ContentBoundM1ProgramCatalogV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<ContentBoundM1ProgramCatalogV1<'static>>();
/// ```
pub struct ContentBoundM1ProgramCatalogV1<'a> {
    catalog_id: Identity,
    strategy: M1PhysicalProgramStrategyV1,
    programs: Box<[ValidatedKernelEnvelope<'a>]>,
}

impl fmt::Debug for ContentBoundM1ProgramCatalogV1<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ContentBoundM1ProgramCatalogV1")
            .field("catalog_id", &self.catalog_id)
            .field("program_count", &self.programs.len())
            .finish_non_exhaustive()
    }
}

impl<'a> ContentBoundM1ProgramCatalogV1<'a> {
    /// Domain-separated identity of every ordered selected-kernel closure.
    #[must_use]
    pub const fn catalog_id(&self) -> Identity {
        self.catalog_id
    }

    /// Exact selected program count.
    #[must_use]
    pub const fn program_count(&self) -> usize {
        self.strategy.program_count()
    }

    /// Strategy derived from the exact admitted artifact roster.
    #[must_use]
    pub const fn program_strategy(&self) -> M1PhysicalProgramStrategyV1 {
        self.strategy
    }

    /// Borrows one exact selected-kernel closure by stable program ordinal.
    ///
    /// # Panics
    /// Panics if the requested program is absent from this catalog's strategy.
    /// Use [`Self::try_program`] when the program is not already roster-bound.
    #[must_use]
    pub fn program(&self, program: M1PhysicalProgramV1) -> &ValidatedKernelEnvelope<'a> {
        &self.programs[program.program_index()]
    }

    /// Looks up a program without assuming that it belongs to this exact roster.
    #[must_use]
    pub fn try_program(
        &self,
        program: M1PhysicalProgramV1,
    ) -> Option<&ValidatedKernelEnvelope<'a>> {
        self.programs.get(program.program_index())
    }

    /// Consumes the Ferric catalog into fe2o3's expected stable program order.
    #[must_use]
    pub fn into_programs(self) -> Vec<ValidatedKernelEnvelope<'a>> {
        self.programs.into_vec()
    }

    /// Worker output inspection is not an independent deployment approval.
    #[must_use]
    pub const fn has_independent_deployment_pin(&self) -> bool {
        false
    }

    /// Selected program closure alone proves no operator or machine refinement.
    #[must_use]
    pub const fn proves_refinement(&self) -> bool {
        false
    }

    /// This pre-load catalog reports no hardware execution or completion.
    #[must_use]
    pub const fn proves_hardware_execution(&self) -> bool {
        false
    }
}

/// Revalidates retained Worker bytes and selects the exact M1 program roster.
///
/// # Errors
///
/// Returns [`M1PhysicalProgramCatalogErrorV1`] if any retained bytes fail the
/// generic COV6 loader, revalidation differs from Ferric's retained plan, or an
/// exact metadata kernel name cannot be closed over the selected object.
pub fn bind_content_bound_m1_program_catalog_v1(
    artifacts: InspectedM1KernelArtifacts<'_>,
) -> Result<ContentBoundM1ProgramCatalogV1<'_>, M1PhysicalProgramCatalogErrorV1> {
    bind_content_bound_catalog_from_source(
        |program| artifacts.bytes_plan_and_source(program),
        M1PhysicalProgramAbiNamesV1::LegacySemantic,
        M1PhysicalProgramStrategyV1::LegacyScalar12,
    )
}

pub(crate) fn bind_content_bound_m1_program_catalog_from_persisted_v1<'bytes>(
    bytes: [&'bytes [u8]; 7],
    plans: &[LoadPlan; 7],
    sources: &[M1PhysicalProgramSourceContractV1; 7],
) -> Result<ContentBoundM1ProgramCatalogV1<'bytes>, M1PhysicalProgramCatalogErrorV1> {
    bind_content_bound_catalog_from_source(
        |program| {
            let family = program.family();
            (
                bytes[family_index(family)],
                plans[family_index(family)],
                sources[family_index(family)],
            )
        },
        M1PhysicalProgramAbiNamesV1::LegacySemantic,
        M1PhysicalProgramStrategyV1::LegacyScalar12,
    )
}

pub(crate) fn bind_content_bound_m1_program_catalog_from_uniform_artifact_v1(
    bytes: &[u8],
    plan: LoadPlan,
    source: M1PhysicalProgramSourceContractV1,
) -> Result<ContentBoundM1ProgramCatalogV1<'_>, M1PhysicalProgramCatalogErrorV1> {
    bind_content_bound_m1_program_catalog_from_uniform_artifact_with_strategy_v1(
        bytes,
        plan,
        source,
        M1PhysicalProgramStrategyV1::LegacyScalar12,
    )
}

pub(crate) fn bind_content_bound_m1_program_catalog_from_uniform_artifact_with_strategy_v1(
    bytes: &[u8],
    plan: LoadPlan,
    source: M1PhysicalProgramSourceContractV1,
    strategy: M1PhysicalProgramStrategyV1,
) -> Result<ContentBoundM1ProgramCatalogV1<'_>, M1PhysicalProgramCatalogErrorV1> {
    let inspection = fe2o3_hsaco::inspect(bytes)
        .map_err(M1PhysicalProgramCatalogErrorV1::AggregateInspection)?;
    if !aggregate_roster_matches(
        strategy,
        inspection.kernels().iter().map(|kernel| kernel.name()),
    ) {
        return Err(M1PhysicalProgramCatalogErrorV1::AggregateRoster(strategy));
    }
    bind_content_bound_catalog_from_source(
        |_| (bytes, plan, source),
        M1PhysicalProgramAbiNamesV1::CompilerAggregatePositional,
        strategy,
    )
}

fn aggregate_roster_matches<'a>(
    strategy: M1PhysicalProgramStrategyV1,
    names: impl Iterator<Item = &'a str>,
) -> bool {
    let mut seen = [false; M1_MFMA_PHYSICAL_PROGRAM_COUNT_V1];
    let mut count = 0;
    for name in names {
        let Some(index) = strategy
            .program_roster()
            .iter()
            .position(|program| program.kernel_symbol() == name)
        else {
            return false;
        };
        if seen[index] {
            return false;
        }
        seen[index] = true;
        count += 1;
    }
    count == strategy.program_count()
}

#[derive(Clone, Copy)]
enum M1PhysicalProgramAbiNamesV1 {
    LegacySemantic,
    CompilerAggregatePositional,
}

fn bind_content_bound_catalog_from_source<'a>(
    mut source: impl FnMut(
        M1PhysicalProgramV1,
    ) -> (&'a [u8], LoadPlan, M1PhysicalProgramSourceContractV1),
    abi_names: M1PhysicalProgramAbiNamesV1,
    strategy: M1PhysicalProgramStrategyV1,
) -> Result<ContentBoundM1ProgramCatalogV1<'a>, M1PhysicalProgramCatalogErrorV1> {
    let programs = strategy.program_roster().iter().copied().map(|program| {
        let (bytes, retained_plan, source_contract) = source(program);
        bind_program(bytes, retained_plan, source_contract, program, abi_names)
    });
    let programs = programs.collect::<Result<Vec<_>, _>>()?.into_boxed_slice();
    let catalog_id = program_catalog_identity(&programs, strategy);
    Ok(ContentBoundM1ProgramCatalogV1 {
        catalog_id,
        strategy,
        programs,
    })
}

fn bind_program(
    bytes: &[u8],
    retained_plan: LoadPlan,
    source_contract: M1PhysicalProgramSourceContractV1,
    program: M1PhysicalProgramV1,
    abi_names: M1PhysicalProgramAbiNamesV1,
) -> Result<ValidatedKernelEnvelope<'_>, M1PhysicalProgramCatalogErrorV1> {
    let envelope = fe2o3_amdhsa_loader::validate(bytes, AdmittedProfile::Gfx942XnackOffCov6)
        .map_err(|error| M1PhysicalProgramCatalogErrorV1::Loader { program, error })?;
    if envelope.plan() != &retained_plan {
        return Err(M1PhysicalProgramCatalogErrorV1::LoaderPlanDrift(
            program.family(),
        ));
    }
    let envelope = envelope
        .bind_kernel(program.kernel_symbol())
        .map_err(|error| M1PhysicalProgramCatalogErrorV1::KernelClosure { program, error })?;
    let compiler_aggregate_abi;
    let dispatch_abi = match abi_names {
        M1PhysicalProgramAbiNamesV1::LegacySemantic => program_dispatch_abi(program),
        M1PhysicalProgramAbiNamesV1::CompilerAggregatePositional => {
            compiler_aggregate_abi = compiler_aggregate_dispatch_abi(program);
            &compiler_aggregate_abi
        }
    };
    envelope
        .reconcile_dispatch_abi(
            program_source_contract_identity(program, source_contract),
            dispatch_abi,
        )
        .map_err(|error| M1PhysicalProgramCatalogErrorV1::DispatchAbi { program, error })
}

const COMPILER_AGGREGATE_GLOBAL_ARGUMENT_NAMES_V1: [&str; 8] = [
    "arg0.data",
    "arg1.data",
    "arg2.data",
    "arg3.data",
    "arg4.data",
    "arg5.data",
    "arg6.data",
    "arg7.data",
];

fn compiler_aggregate_dispatch_abi(
    program: M1PhysicalProgramV1,
) -> Vec<KernelGlobalBufferAbiV1<'static>> {
    program_dispatch_abi(program)
        .iter()
        .enumerate()
        .map(|(position, row)| {
            KernelGlobalBufferAbiV1::new(
                row.explicit_argument_index(),
                COMPILER_AGGREGATE_GLOBAL_ARGUMENT_NAMES_V1
                    .get(position)
                    .copied()
                    .unwrap_or(""),
                row.offset(),
                row.pointee_alignment(),
                row.access(),
            )
        })
        .collect()
}

fn program_dispatch_abi(
    program: M1PhysicalProgramV1,
) -> &'static [KernelGlobalBufferAbiV1<'static>] {
    match program {
        M1PhysicalProgramV1::GemmReference
        | M1PhysicalProgramV1::GemmVectorized
        | M1PhysicalProgramV1::GemmMfma => &gemm::QWEN3_GEMM_GLOBAL_BUFFER_ABI_V1,
        M1PhysicalProgramV1::TokenEmbedding => &gemm::QWEN3_TOKEN_EMBEDDING_GLOBAL_BUFFER_ABI_V1,
        M1PhysicalProgramV1::RmsNorm => &rmsnorm::QWEN3_RMSNORM_GLOBAL_BUFFER_ABI_V1,
        M1PhysicalProgramV1::Rope => &rope_kv::QWEN3_ROPE_GLOBAL_BUFFER_ABI_V1,
        M1PhysicalProgramV1::PagedKvWrite => &rope_kv::QWEN3_PAGED_KV_WRITE_GLOBAL_BUFFER_ABI_V1,
        M1PhysicalProgramV1::GqaPrefill => &prefill::QWEN3_PREFILL_GLOBAL_BUFFER_ABI_V1,
        M1PhysicalProgramV1::PagedGqaDecode => {
            &paged_decode::QWEN3_PAGED_DECODE_GLOBAL_BUFFER_ABI_V1
        }
        M1PhysicalProgramV1::SwiGlu => &swiglu::QWEN3_SWIGLU_GLOBAL_BUFFER_ABI_V1,
        M1PhysicalProgramV1::LogitsArgmax => &logits::QWEN3_LOGITS_ARGMAX_GLOBAL_BUFFER_ABI_V1,
        M1PhysicalProgramV1::LogitsCompact => &logits::QWEN3_LOGITS_COMPACT_GLOBAL_BUFFER_ABI_V1,
        M1PhysicalProgramV1::SpeculativeTokenAssembly => {
            &logits::QWEN3_SPECULATIVE_TOKEN_ASSEMBLY_GLOBAL_BUFFER_ABI_V1
        }
    }
}

fn program_source_contract_identity(
    program: M1PhysicalProgramV1,
    source: M1PhysicalProgramSourceContractV1,
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update((PROGRAM_SOURCE_CONTRACT_IDENTITY_DOMAIN.len() as u64).to_le_bytes());
    hasher.update(PROGRAM_SOURCE_CONTRACT_IDENTITY_DOMAIN);
    hasher.update([program.family() as u8]);
    hasher.update([program as u8]);
    hasher.update((program.kernel_symbol().len() as u64).to_le_bytes());
    hasher.update(program.kernel_symbol().as_bytes());
    hasher.update(source.compiler_handoff_sha256);
    hasher.update(source.compiler_handoff_byte_len.to_le_bytes());
    hasher.finalize().into()
}

const fn family_index(family: M1PhysicalProgramFamilyV1) -> usize {
    match family {
        M1PhysicalProgramFamilyV1::Gemm => 0,
        M1PhysicalProgramFamilyV1::RmsNorm => 1,
        M1PhysicalProgramFamilyV1::RopeKv => 2,
        M1PhysicalProgramFamilyV1::Prefill => 3,
        M1PhysicalProgramFamilyV1::PagedDecode => 4,
        M1PhysicalProgramFamilyV1::SwiGlu => 5,
        M1PhysicalProgramFamilyV1::Logits => 6,
    }
}

fn program_catalog_identity(
    programs: &[ValidatedKernelEnvelope<'_>],
    strategy: M1PhysicalProgramStrategyV1,
) -> Identity {
    let mut hasher = Sha256::new();
    hasher.update((PROGRAM_CATALOG_IDENTITY_DOMAIN.len() as u64).to_le_bytes());
    hasher.update(PROGRAM_CATALOG_IDENTITY_DOMAIN);
    hasher.update((strategy.program_count() as u64).to_le_bytes());
    if strategy == M1PhysicalProgramStrategyV1::AttributedMfma13 {
        hasher.update(b"ferric.m1.attributed-mfma-program-strategy.v1");
    }
    for (program, envelope) in strategy.program_roster().iter().copied().zip(programs) {
        hasher.update([program as u8]);
        hasher.update((program.kernel_symbol().len() as u64).to_le_bytes());
        hasher.update(program.kernel_symbol().as_bytes());
        hasher.update(envelope.identity_inputs().closure_sha256());
        hasher.update(
            envelope
                .dispatch_abi_identity()
                .expect("M1 catalog contains only source-reconciled programs"),
        );
    }
    Identity::new(hasher.finalize().into())
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{
        aggregate_roster_matches, compiler_aggregate_dispatch_abi, program_dispatch_abi,
        program_source_contract_identity, M1PhysicalProgramSourceContractV1,
        M1PhysicalProgramStrategyV1, M1PhysicalProgramV1, M1_PHYSICAL_PROGRAM_COUNT_V1,
    };

    const PHYSICAL_PROGRAM_CATALOG_SOURCE: &str = include_str!("physical_program_catalog.rs");

    fn compiler_aggregate_projection_source(source: &str) -> Option<&str> {
        let start = source.find("const COMPILER_AGGREGATE_GLOBAL_ARGUMENT_NAMES_V1:")?;
        let tail = &source[start..];
        let end = tail.find("\nfn program_dispatch_abi(")?;
        Some(&tail[..end])
    }

    fn source_between<'a>(source: &'a str, start: &str, end: &str) -> Option<&'a str> {
        let start = source.find(start)?;
        let tail = &source[start..];
        let end = tail.find(end)?;
        Some(&tail[..end])
    }

    fn compiler_aggregate_projection_source_policy(source: &str) -> bool {
        let Some(projection) = compiler_aggregate_projection_source(source) else {
            return false;
        };
        let required_in_order = [
            "\"arg0.data\"",
            "\"arg1.data\"",
            "\"arg2.data\"",
            "\"arg3.data\"",
            "\"arg4.data\"",
            "\"arg5.data\"",
            "\"arg6.data\"",
            "\"arg7.data\"",
            ".enumerate()",
            "KernelGlobalBufferAbiV1::new(",
            "row.explicit_argument_index(),",
            "COMPILER_AGGREGATE_GLOBAL_ARGUMENT_NAMES_V1\n                    .get(position)",
            ".unwrap_or(\"\")",
            "row.offset(),",
            "row.pointee_alignment(),",
            "row.access(),",
        ];
        let mut cursor = 0;
        for required in required_in_order {
            if projection.matches(required).count() != 1 {
                return false;
            }
            let Some(offset) = projection[cursor..].find(required) else {
                return false;
            };
            cursor += offset + required.len();
        }
        true
    }

    fn mutate_compiler_aggregate_projection(source: &str, old: &str, new: &str) -> String {
        let projection =
            compiler_aggregate_projection_source(source).expect("compiler aggregate projection");
        assert_eq!(
            projection.matches(old).count(),
            1,
            "mutation anchor must occur exactly once: {old}"
        );
        let changed = projection.replacen(old, new, 1);
        assert_ne!(changed, projection);
        source.replacen(projection, &changed, 1)
    }

    #[test]
    fn stable_program_order_is_complete_unique_and_symbol_bound() {
        assert_eq!(M1PhysicalProgramV1::ALL.len(), M1_PHYSICAL_PROGRAM_COUNT_V1);
        let mut symbols = HashSet::new();
        for (index, program) in M1PhysicalProgramV1::ALL.into_iter().enumerate() {
            assert_eq!(program.program_index(), index);
            assert!(!program.kernel_symbol().is_empty());
            assert!(symbols.insert(program.kernel_symbol()));
        }
    }

    #[test]
    fn scalar_and_mfma_rosters_are_exact_and_preserve_legacy_ordinals() {
        use M1PhysicalProgramStrategyV1::{AttributedMfma13, LegacyScalar12};
        assert_eq!(M1_PHYSICAL_PROGRAM_COUNT_V1, 12);
        assert_eq!(LegacyScalar12.program_roster(), M1PhysicalProgramV1::ALL);
        assert_eq!(AttributedMfma13.program_count(), 13);
        assert_eq!(
            &AttributedMfma13.program_roster()[..12],
            LegacyScalar12.program_roster()
        );
        assert_eq!(
            AttributedMfma13.program_roster()[12],
            M1PhysicalProgramV1::GemmMfma
        );
        for strategy in [LegacyScalar12, AttributedMfma13] {
            let mut names = strategy
                .program_roster()
                .iter()
                .map(|program| program.kernel_symbol())
                .collect::<Vec<_>>();
            assert!(aggregate_roster_matches(strategy, names.iter().copied()));
            names.reverse();
            assert!(aggregate_roster_matches(strategy, names.iter().copied()));
            assert!(!aggregate_roster_matches(
                strategy,
                names[..names.len() - 1].iter().copied()
            ));
            names[0] = names[1];
            assert!(!aggregate_roster_matches(strategy, names.iter().copied()));
            names[0] = "unrecognized_kernel";
            assert!(!aggregate_roster_matches(strategy, names.iter().copied()));
        }
        assert!(!aggregate_roster_matches(
            LegacyScalar12,
            AttributedMfma13
                .program_roster()
                .iter()
                .map(|program| program.kernel_symbol())
        ));
        assert!(!aggregate_roster_matches(
            AttributedMfma13,
            LegacyScalar12
                .program_roster()
                .iter()
                .map(|program| program.kernel_symbol())
        ));
    }

    #[test]
    fn canonical_dispatch_abi_roster_covers_exactly_54_global_arguments() {
        let mut total = 0usize;
        for program in M1PhysicalProgramV1::ALL {
            let roster = program_dispatch_abi(program);
            assert!(!roster.is_empty());
            let mut ordinals = HashSet::new();
            for row in roster {
                assert!(ordinals.insert(row.explicit_argument_index()));
                assert_eq!(row.explicit_argument_index() % 2, 0);
                assert_eq!(
                    row.offset(),
                    (row.explicit_argument_index() as u64 / 2) * 16
                );
                assert!(!row.name().is_empty());
                assert!(!row.name().starts_with("arg"));
                assert!(row.pointee_alignment().is_power_of_two());
            }
            total += roster.len();
        }
        assert_eq!(total, 54);
    }

    #[test]
    fn compiler_aggregate_projects_all_thirteen_programs_to_positional_names_only() {
        let mut total = 0usize;
        for program in M1PhysicalProgramV1::MFMA_ALL {
            let semantic = program_dispatch_abi(program);
            let positional = compiler_aggregate_dispatch_abi(program);
            assert_eq!(positional.len(), semantic.len());
            for (position, (actual, source)) in positional.iter().zip(semantic).enumerate() {
                assert_eq!(actual.name(), format!("arg{position}.data"));
                assert_ne!(actual.name(), source.name());
                assert_eq!(
                    actual.explicit_argument_index(),
                    source.explicit_argument_index()
                );
                assert_eq!(actual.explicit_argument_index(), position * 2);
                assert_eq!(actual.offset(), source.offset());
                assert_eq!(actual.offset(), (position as u64) * 16);
                assert_eq!(actual.pointee_alignment(), source.pointee_alignment());
                assert_eq!(actual.access(), source.access());
            }
            total += positional.len();
        }
        assert_eq!(total, 57);
    }

    #[test]
    fn compiler_aggregate_projection_keeps_name_and_coordinates_bound_together() {
        for program in M1PhysicalProgramV1::ALL {
            for (position, row) in compiler_aggregate_dispatch_abi(program).iter().enumerate() {
                let expected_name = format!("arg{position}.data");
                assert_eq!(row.name(), expected_name);
                assert_ne!(row.name(), format!("arg{}.data", position + 1));
                assert_eq!(row.explicit_argument_index(), position * 2);
                assert_ne!(row.explicit_argument_index(), position * 2 + 1);
                assert_eq!(row.offset(), (position as u64) * 16);
                assert_ne!(row.offset(), (position as u64) * 16 + 8);
            }
        }
    }

    #[test]
    fn only_uniform_compiler_aggregate_binding_uses_positional_names() {
        let legacy = source_between(
            PHYSICAL_PROGRAM_CATALOG_SOURCE,
            "pub fn bind_content_bound_m1_program_catalog_v1(",
            "\npub(crate) fn bind_content_bound_m1_program_catalog_from_persisted_v1",
        )
        .expect("legacy artifact binder");
        let persisted = source_between(
            PHYSICAL_PROGRAM_CATALOG_SOURCE,
            "pub(crate) fn bind_content_bound_m1_program_catalog_from_persisted_v1",
            "\npub(crate) fn bind_content_bound_m1_program_catalog_from_uniform_artifact_v1",
        )
        .expect("persisted artifact binder");
        let aggregate = source_between(
            PHYSICAL_PROGRAM_CATALOG_SOURCE,
            "pub(crate) fn bind_content_bound_m1_program_catalog_from_uniform_artifact_v1",
            "\n#[derive(Clone, Copy)]",
        )
        .expect("uniform compiler aggregate binder");

        for semantic in [legacy, persisted] {
            assert!(semantic.contains("M1PhysicalProgramAbiNamesV1::LegacySemantic"));
            assert!(!semantic.contains("M1PhysicalProgramAbiNamesV1::CompilerAggregatePositional"));
        }
        assert!(aggregate.contains("M1PhysicalProgramAbiNamesV1::CompilerAggregatePositional"));
        assert!(!aggregate.contains("M1PhysicalProgramAbiNamesV1::LegacySemantic"));
    }

    #[test]
    fn compiler_aggregate_projection_source_policy_rejects_name_and_coordinate_drift() {
        assert!(compiler_aggregate_projection_source_policy(
            PHYSICAL_PROGRAM_CATALOG_SOURCE
        ));
        for (case, hostile) in [
            (
                "name drift",
                mutate_compiler_aggregate_projection(
                    PHYSICAL_PROGRAM_CATALOG_SOURCE,
                    "\"arg0.data\"",
                    "\"arg8.data\"",
                ),
            ),
            (
                "argument index drift",
                mutate_compiler_aggregate_projection(
                    PHYSICAL_PROGRAM_CATALOG_SOURCE,
                    "row.explicit_argument_index(),",
                    "row.explicit_argument_index() + 1,",
                ),
            ),
            (
                "offset drift",
                mutate_compiler_aggregate_projection(
                    PHYSICAL_PROGRAM_CATALOG_SOURCE,
                    "row.offset(),",
                    "row.offset() + 8,",
                ),
            ),
        ] {
            assert!(
                !compiler_aggregate_projection_source_policy(&hostile),
                "source policy accepted hostile mutation: {case}"
            );
        }
    }

    #[test]
    fn source_contract_identity_binds_program_handoff_digest_and_length() {
        let program = M1PhysicalProgramV1::TokenEmbedding;
        let baseline = M1PhysicalProgramSourceContractV1::new([0x11; 32], 4096);
        let changed_digest = M1PhysicalProgramSourceContractV1::new([0x12; 32], 4096);
        let changed_length = M1PhysicalProgramSourceContractV1::new([0x11; 32], 4097);
        let identity = program_source_contract_identity(program, baseline);
        assert_ne!(identity, [0; 32]);
        assert_ne!(
            identity,
            program_source_contract_identity(program, changed_digest)
        );
        assert_ne!(
            identity,
            program_source_contract_identity(program, changed_length)
        );
        assert_ne!(
            identity,
            program_source_contract_identity(M1PhysicalProgramV1::GemmReference, baseline)
        );
    }
}
