//! Content-bound physical program selection for the exact M1 Qwen artifacts.
//!
//! The compiler-facing Qwen owners have already checked their Worker lineage,
//! complete HSACO inventory, ABI, resources, and allocation-free load plan.
//! This module revalidates those retained bytes through the generic fe2o3
//! loader and binds the eleven exact entry points needed by packet lowering.
//! It does not independently approve deployment bytes, allocate or load an
//! image, construct kernargs, publish a queue, observe completion, prove
//! refinement, or report hardware or performance evidence.

use std::fmt;

use fe2o3_amdhsa_loader::{
    AdmittedProfile, KernelClosureError, LoadPlan, PlanError, ValidatedKernelEnvelope,
};
use ferric_qwen_kernels::{gemm, logits, paged_decode, prefill, rmsnorm, rope_kv, swiglu};
use ferric_spec::Identity;
use sha2::{Digest, Sha256};

const PROGRAM_CATALOG_IDENTITY_DOMAIN: &[u8] = b"ferric.m1.physical-program-catalog.v1";

/// Exact number of selected entry points across the seven M1 kernel artifacts.
pub const M1_PHYSICAL_PROGRAM_COUNT_V1: usize = 11;

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
    ];

    /// Zero-based index supplied to a fixed service packet.
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
            Self::TokenEmbedding => gemm::QWEN3_TOKEN_EMBEDDING_KERNEL_SYMBOL_V1,
            Self::RmsNorm => rmsnorm::QWEN3_RMSNORM_KERNEL_SYMBOL_V1,
            Self::Rope => rope_kv::QWEN3_ROPE_KERNEL_SYMBOL_V1,
            Self::PagedKvWrite => rope_kv::QWEN3_PAGED_KV_WRITE_KERNEL_SYMBOL_V1,
            Self::GqaPrefill => prefill::QWEN3_PREFILL_KERNEL_SYMBOL_V1,
            Self::PagedGqaDecode => paged_decode::QWEN3_PAGED_DECODE_KERNEL_SYMBOL_V1,
            Self::SwiGlu => swiglu::QWEN3_SWIGLU_KERNEL_SYMBOL_V1,
            Self::LogitsArgmax => logits::QWEN3_LOGITS_ARGMAX_KERNEL_SYMBOL_V1,
            Self::LogitsCompact => logits::QWEN3_LOGITS_COMPACT_KERNEL_SYMBOL_V1,
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
            Self::GemmReference | Self::GemmVectorized | Self::TokenEmbedding => {
                M1PhysicalProgramFamilyV1::Gemm
            }
            Self::RmsNorm => M1PhysicalProgramFamilyV1::RmsNorm,
            Self::Rope | Self::PagedKvWrite => M1PhysicalProgramFamilyV1::RopeKv,
            Self::GqaPrefill => M1PhysicalProgramFamilyV1::Prefill,
            Self::PagedGqaDecode => M1PhysicalProgramFamilyV1::PagedDecode,
            Self::SwiGlu => M1PhysicalProgramFamilyV1::SwiGlu,
            Self::LogitsArgmax | Self::LogitsCompact => M1PhysicalProgramFamilyV1::Logits,
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

    fn bytes_and_plan(self, program: M1PhysicalProgramV1) -> (&'a [u8], LoadPlan) {
        match program.family() {
            M1PhysicalProgramFamilyV1::Gemm => (
                self.gemm.exact_worker_output_bytes(),
                *self.gemm.loader_plan(),
            ),
            M1PhysicalProgramFamilyV1::RmsNorm => (
                self.rmsnorm.exact_worker_output_bytes(),
                *self.rmsnorm.loader_plan(),
            ),
            M1PhysicalProgramFamilyV1::RopeKv => (
                self.rope_kv.exact_worker_output_bytes(),
                *self.rope_kv.loader_plan(),
            ),
            M1PhysicalProgramFamilyV1::Prefill => (
                self.prefill.exact_worker_output_bytes(),
                *self.prefill.loader_plan(),
            ),
            M1PhysicalProgramFamilyV1::PagedDecode => (
                self.paged_decode.exact_worker_output_bytes(),
                *self.paged_decode.loader_plan(),
            ),
            M1PhysicalProgramFamilyV1::SwiGlu => (
                self.swiglu.exact_worker_output_bytes(),
                *self.swiglu.loader_plan(),
            ),
            M1PhysicalProgramFamilyV1::Logits => (
                self.logits.exact_worker_output_bytes(),
                *self.logits.loader_plan(),
            ),
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
}

impl fmt::Display for M1PhysicalProgramCatalogErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "M1 physical program catalog rejected: {self:?}")
    }
}

impl std::error::Error for M1PhysicalProgramCatalogErrorV1 {}

/// Content-bound custody of the eleven exact selected kernel entry points.
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
    programs: [ValidatedKernelEnvelope<'a>; M1_PHYSICAL_PROGRAM_COUNT_V1],
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
        M1_PHYSICAL_PROGRAM_COUNT_V1
    }

    /// Borrows one exact selected-kernel closure by stable program ordinal.
    #[must_use]
    pub fn program(&self, program: M1PhysicalProgramV1) -> &ValidatedKernelEnvelope<'a> {
        &self.programs[program.program_index()]
    }

    /// Consumes the Ferric catalog into fe2o3's expected stable program order.
    #[must_use]
    pub fn into_programs(self) -> Vec<ValidatedKernelEnvelope<'a>> {
        Vec::from(self.programs)
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
    let programs = M1PhysicalProgramV1::ALL.map(|program| bind_program(artifacts, program));
    let programs = collect_programs(programs)?;
    let catalog_id = program_catalog_identity(&programs);
    Ok(ContentBoundM1ProgramCatalogV1 {
        catalog_id,
        programs,
    })
}

fn bind_program(
    artifacts: InspectedM1KernelArtifacts<'_>,
    program: M1PhysicalProgramV1,
) -> Result<ValidatedKernelEnvelope<'_>, M1PhysicalProgramCatalogErrorV1> {
    let (bytes, retained_plan) = artifacts.bytes_and_plan(program);
    let envelope = fe2o3_amdhsa_loader::validate(bytes, AdmittedProfile::Gfx942XnackOffCov6)
        .map_err(|error| M1PhysicalProgramCatalogErrorV1::Loader { program, error })?;
    if envelope.plan() != &retained_plan {
        return Err(M1PhysicalProgramCatalogErrorV1::LoaderPlanDrift(
            program.family(),
        ));
    }
    envelope
        .bind_kernel(program.kernel_symbol())
        .map_err(|error| M1PhysicalProgramCatalogErrorV1::KernelClosure { program, error })
}

fn collect_programs(
    programs: [Result<ValidatedKernelEnvelope<'_>, M1PhysicalProgramCatalogErrorV1>;
        M1_PHYSICAL_PROGRAM_COUNT_V1],
) -> Result<
    [ValidatedKernelEnvelope<'_>; M1_PHYSICAL_PROGRAM_COUNT_V1],
    M1PhysicalProgramCatalogErrorV1,
> {
    let mut validated = Vec::with_capacity(M1_PHYSICAL_PROGRAM_COUNT_V1);
    for program in programs {
        validated.push(program?);
    }
    validated
        .try_into()
        .map_err(|_| unreachable!("the fixed program roster has exact cardinality"))
}

fn program_catalog_identity(
    programs: &[ValidatedKernelEnvelope<'_>; M1_PHYSICAL_PROGRAM_COUNT_V1],
) -> Identity {
    let mut hasher = Sha256::new();
    hasher.update((PROGRAM_CATALOG_IDENTITY_DOMAIN.len() as u64).to_le_bytes());
    hasher.update(PROGRAM_CATALOG_IDENTITY_DOMAIN);
    hasher.update((M1_PHYSICAL_PROGRAM_COUNT_V1 as u64).to_le_bytes());
    for (program, envelope) in M1PhysicalProgramV1::ALL.into_iter().zip(programs) {
        hasher.update([program as u8]);
        hasher.update((program.kernel_symbol().len() as u64).to_le_bytes());
        hasher.update(program.kernel_symbol().as_bytes());
        hasher.update(envelope.identity_inputs().closure_sha256());
    }
    Identity::new(hasher.finalize().into())
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{M1PhysicalProgramV1, M1_PHYSICAL_PROGRAM_COUNT_V1};

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
}
