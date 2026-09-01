//! Content-bound physical program selection for the exact M1 Qwen artifacts.
//!
//! The compiler-facing Qwen owners have already checked their Worker lineage,
//! complete HSACO inventory, ABI, resources, and allocation-free load plan.
//! This module revalidates those retained bytes through the generic fe2o3
//! loader and binds the twelve exact entry points needed by packet lowering.
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
            Self::GemmReference | Self::GemmVectorized | Self::TokenEmbedding => {
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

/// Content-bound custody of the twelve exact selected kernel entry points.
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
    bind_content_bound_catalog_from_source(|program| artifacts.bytes_plan_and_source(program))
}

pub(crate) fn bind_content_bound_m1_program_catalog_from_persisted_v1<'bytes>(
    bytes: [&'bytes [u8]; 7],
    plans: &[LoadPlan; 7],
    sources: &[M1PhysicalProgramSourceContractV1; 7],
) -> Result<ContentBoundM1ProgramCatalogV1<'bytes>, M1PhysicalProgramCatalogErrorV1> {
    bind_content_bound_catalog_from_source(|program| {
        let family = program.family();
        (
            bytes[family_index(family)],
            plans[family_index(family)],
            sources[family_index(family)],
        )
    })
}

fn bind_content_bound_catalog_from_source<'a>(
    mut source: impl FnMut(
        M1PhysicalProgramV1,
    ) -> (&'a [u8], LoadPlan, M1PhysicalProgramSourceContractV1),
) -> Result<ContentBoundM1ProgramCatalogV1<'a>, M1PhysicalProgramCatalogErrorV1> {
    let programs = M1PhysicalProgramV1::ALL.map(|program| {
        let (bytes, retained_plan, source_contract) = source(program);
        bind_program(bytes, retained_plan, source_contract, program)
    });
    let programs = collect_programs(programs)?;
    let catalog_id = program_catalog_identity(&programs);
    Ok(ContentBoundM1ProgramCatalogV1 {
        catalog_id,
        programs,
    })
}

fn bind_program(
    bytes: &[u8],
    retained_plan: LoadPlan,
    source_contract: M1PhysicalProgramSourceContractV1,
    program: M1PhysicalProgramV1,
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
    envelope
        .reconcile_dispatch_abi(
            program_source_contract_identity(program, source_contract),
            program_dispatch_abi(program),
        )
        .map_err(|error| M1PhysicalProgramCatalogErrorV1::DispatchAbi { program, error })
}

fn program_dispatch_abi(
    program: M1PhysicalProgramV1,
) -> &'static [KernelGlobalBufferAbiV1<'static>] {
    match program {
        M1PhysicalProgramV1::GemmReference | M1PhysicalProgramV1::GemmVectorized => {
            &gemm::QWEN3_GEMM_GLOBAL_BUFFER_ABI_V1
        }
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
        program_dispatch_abi, program_source_contract_identity, M1PhysicalProgramSourceContractV1,
        M1PhysicalProgramV1, M1_PHYSICAL_PROGRAM_COUNT_V1,
    };

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
                assert!(row.pointee_alignment().is_power_of_two());
            }
            total += roster.len();
        }
        assert_eq!(total, 54);
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
