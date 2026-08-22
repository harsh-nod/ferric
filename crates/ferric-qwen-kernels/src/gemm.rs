//! Exact finite Qwen3 token-embedding and dense GEMM/GEMV profiles owned by Ferric.
//!
//! A, B, and C use BF16 storage. The declared machine source widens operands
//! to FP32, accumulates in ascending K order with separate multiply and add,
//! optionally adds the widened BF16 residual, and narrows C with BF16 RNE.
//! The two finite schedule classes share a 16x16 Wave64 tile. The reference
//! source transfers one A value per K step; the vectorized-A source transfers
//! four adjacent A values and applies their products in ascending order.
//!
//! The catalog, source pin, compiler handoff, Worker transcript inspection,
//! and checked buffer binding do not establish numerical, operator, race,
//! source-to-machine, hardware, completion, performance, allocation, load, or
//! launch refinement. Duplicate graph profiles with identical runtime matrix
//! geometry are deliberately machine-equivalent; their distinct Ferric
//! profile identities remain host-side and are not classified by the source.
//! Token embedding copies one BF16 weight element per output workitem without
//! arithmetic. Its expected weight identity is an opaque bundle-admission
//! label retained at the checked binding boundary, not content-authentication
//! evidence created by this module.

use core::fmt;
use std::fmt::Write as _;

use fe2o3_amdhsa_loader::{AdmittedProfile, LoadPlan, PlanError};
use fe2o3_artifact_transaction::{
    CompilerModuleHandoffIdentityV1, ConsumedCompilerModuleHandoffV1,
};
use fe2o3_compiler_ffi::{
    CodeObjectVersion, CompilerFfiEnvelopeError, CompilerFfiEnvelopeV1,
    CompilerModuleHandoffErrorV2, CompilerModuleHandoffIdentityV2, CompilerModuleHandoffV2,
    CompilerModuleKindV1, CompilerModuleSymbolManifestErrorV1,
    CompilerModuleSymbolManifestIdentityV1, CompilerModuleSymbolManifestV1,
    CompilerModuleSymbolRoleV1, DeviceTargetV1, EXTERNAL_DEVICE_LIBRARY_GFX942_DATA_LAYOUT_V1,
};
use fe2o3_hsaco::{
    inspect_and_bind_kernel_descriptors, ArgumentAccess, ArgumentAddressSpace,
    CodeObjectVersion as InspectedCodeObjectVersion, ExplicitArgument, ExplicitValueKind,
    ExplicitValueType, HiddenArgument, HiddenValueKind, InspectedKernel, KernelBindingError,
    KernelDescriptorBinding, MAX_HSACO_BYTES,
};
use fe2o3_hsaco_finalize::{
    execute_reproducible_first_build_worker_v2, FirstBuildWorkerV2Error,
    InertDecodedWorkerExchangeV2, InertFirstBuildWorkerV2EvidenceV1, LinkOptionV1, PinnedWorkerV1,
    WorkerExecutionLimitsV1, WorkerOutputConstraintsV1, WorkerProtocolError,
};
use sha2::{Digest as _, Sha256};

/// Exact reference-schedule kernel entry.
pub const QWEN3_GEMM_REFERENCE_KERNEL_SYMBOL_V1: &str =
    "ferric_qwen3_gemm_reference_bf16_f32_bf16_v1";
/// Exact reference-schedule AMDHSA descriptor symbol.
pub const QWEN3_GEMM_REFERENCE_DESCRIPTOR_SYMBOL_V1: &str =
    "ferric_qwen3_gemm_reference_bf16_f32_bf16_v1.kd";
/// Exact vectorized-A-schedule kernel entry.
pub const QWEN3_GEMM_VECTORIZED_KERNEL_SYMBOL_V1: &str =
    "ferric_qwen3_gemm_vector_a4_bf16_f32_bf16_v1";
/// Exact vectorized-A-schedule AMDHSA descriptor symbol.
pub const QWEN3_GEMM_VECTORIZED_DESCRIPTOR_SYMBOL_V1: &str =
    "ferric_qwen3_gemm_vector_a4_bf16_f32_bf16_v1.kd";
/// Exact BF16 token-embedding bit-copy kernel entry.
pub const QWEN3_TOKEN_EMBEDDING_KERNEL_SYMBOL_V1: &str =
    "ferric_qwen3_token_embedding_bf16_copy_v1";
/// Exact BF16 token-embedding descriptor symbol.
pub const QWEN3_TOKEN_EMBEDDING_DESCRIPTOR_SYMBOL_V1: &str =
    "ferric_qwen3_token_embedding_bf16_copy_v1.kd";
/// Exact device target required by every profile.
pub const QWEN3_GEMM_TARGET_V1: &str = "gfx942:xnack-";
/// Exact code-object version required by every profile.
pub const QWEN3_GEMM_CODE_OBJECT_VERSION_V1: u8 = 6;
/// Exact Wave64 workgroup used by both schedules.
pub const QWEN3_GEMM_WORKGROUP_V1: [u32; 3] = [64, 1, 1];
/// Exact output tile dimensions.
pub const QWEN3_GEMM_TILE_V1: [u32; 2] = [16, 16];
/// Qwen3 vocabulary size pinned by the Ferric graph envelope.
pub const QWEN3_VOCABULARY_SIZE_V1: u32 = 151_936;
/// Number of target/draft bucket selections.
pub const QWEN3_GEMM_BUCKET_COUNT_V1: usize = 22;
/// Number of dense operations per bucket.
pub const QWEN3_GEMM_OPERATION_COUNT_V1: usize = 8;
/// Total exact finite profiles.
pub const QWEN3_GEMM_PROFILE_COUNT_V1: usize =
    QWEN3_GEMM_BUCKET_COUNT_V1 * QWEN3_GEMM_OPERATION_COUNT_V1;
/// Exact target/draft token-embedding profile count.
pub const QWEN3_TOKEN_EMBEDDING_PROFILE_COUNT_V1: usize = QWEN3_GEMM_BUCKET_COUNT_V1;
/// Exact explicit three-slice plus four-u32 kernarg bytes.
pub const QWEN3_GEMM_EXPLICIT_KERNARG_BYTES_V1: u64 = 64;
/// Exact explicit plus COV6 hidden kernarg bytes.
pub const QWEN3_GEMM_TOTAL_KERNARG_BYTES_V1: u64 = 320;
/// Exact token-embedding explicit kernarg bytes after ABI alignment.
pub const QWEN3_TOKEN_EMBEDDING_EXPLICIT_KERNARG_BYTES_V1: u64 = 64;
/// Exact token-embedding explicit plus COV6 hidden kernarg bytes.
pub const QWEN3_TOKEN_EMBEDDING_TOTAL_KERNARG_BYTES_V1: u64 = 320;
/// Exact kernarg alignment.
pub const QWEN3_GEMM_KERNARG_ALIGNMENT_V1: u64 = 8;
/// Exact byte length of the final canonical direct-LLVM module.
pub const QWEN3_GEMM_LLVM_BYTES_V1: usize = 26_051;
/// SHA-256 of the final canonical direct-LLVM module bytes.
pub const QWEN3_GEMM_LLVM_SHA256_V1: [u8; 32] = [
    0x7d, 0xc4, 0x42, 0xbc, 0xb9, 0xdd, 0x56, 0xf8, 0xab, 0xa5, 0x02, 0x67, 0xe3, 0xc2, 0x14, 0xbc,
    0xd6, 0x51, 0x73, 0x13, 0x05, 0x9e, 0xe3, 0x36, 0x13, 0x9e, 0xc8, 0x71, 0x28, 0x0a, 0xcc, 0xff,
];

const PROFILE_DOMAIN: &[u8] = b"FERRIC/QWEN3/GEMM/PROFILE/V1\0";
const CATALOG_DOMAIN: &[u8] = b"FERRIC/QWEN3/GEMM/CATALOG/V1\0";
const KERNEL_IR_DOMAIN: &[u8] = b"FERRIC/QWEN3/GEMM/KERNEL-IR/V1\0";
const SOURCE_BINDING_DOMAIN: &[u8] = b"FERRIC/QWEN3/GEMM/SOURCE-BINDING/V1\0";
const EMBEDDING_PROFILE_DOMAIN: &[u8] = b"FERRIC/QWEN3/TOKEN-EMBEDDING/PROFILE/V1\0";
const EMBEDDING_CATALOG_DOMAIN: &[u8] = b"FERRIC/QWEN3/TOKEN-EMBEDDING/CATALOG/V1\0";
const EMBEDDING_KERNEL_IR_DOMAIN: &[u8] = b"FERRIC/QWEN3/TOKEN-EMBEDDING/KERNEL-IR/V1\0";

/// Target or speculative-draft Qwen3 model role.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Qwen3GemmModelRoleV1 {
    /// Qwen3-8B target geometry.
    Target8B = 1,
    /// Qwen3-0.6B draft geometry.
    Draft06B = 2,
}

impl Qwen3GemmModelRoleV1 {
    /// Exact hidden width.
    #[must_use]
    pub const fn hidden_size(self) -> u32 {
        match self {
            Self::Target8B => 4_096,
            Self::Draft06B => 1_024,
        }
    }

    /// Exact gated-MLP intermediate width.
    #[must_use]
    pub const fn intermediate_size(self) -> u32 {
        match self {
            Self::Target8B => 12_288,
            Self::Draft06B => 3_072,
        }
    }

    /// Exact flattened query-head width.
    #[must_use]
    pub const fn query_width(self) -> u32 {
        match self {
            Self::Target8B => 32 * 128,
            Self::Draft06B => 16 * 128,
        }
    }

    /// Exact flattened key/value-head width.
    #[must_use]
    pub const fn kv_width(self) -> u32 {
        8 * 128
    }
}

/// One of the eleven exact Ferric M1 bucket shapes.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Qwen3GemmBucketKindV1 {
    /// One sequence with 128 prefill tokens.
    PrefillS1T128 = 1,
    /// Eight sequences with 128 prefill tokens each.
    PrefillS8T128 = 2,
    /// One sequence with 512 prefill tokens.
    PrefillS1T512 = 3,
    /// One sequence with 2,048 prefill tokens.
    PrefillS1T2048 = 4,
    /// One single-token decode sequence.
    DecodeS1C8192 = 5,
    /// Eight single-token decode sequences.
    DecodeS8C8192 = 6,
    /// Thirty-two single-token decode sequences.
    DecodeS32C8192 = 7,
    /// One speculative sequence with K=4.
    SpeculativeS1K4C8192 = 8,
    /// Eight speculative sequences with K=4.
    SpeculativeS8K4C8192 = 9,
    /// One speculative sequence with K=8.
    SpeculativeS1K8C8192 = 10,
    /// One speculative sequence with K=16.
    SpeculativeS1K16C8192 = 11,
}

impl Qwen3GemmBucketKindV1 {
    const fn sequence_and_active_tokens(self, role: Qwen3GemmModelRoleV1) -> [u32; 2] {
        match self {
            Self::PrefillS1T128 => [1, 128],
            Self::PrefillS8T128 => [8, 128],
            Self::PrefillS1T512 => [1, 512],
            Self::PrefillS1T2048 => [1, 2_048],
            Self::DecodeS1C8192 => [1, 1],
            Self::DecodeS8C8192 => [8, 1],
            Self::DecodeS32C8192 => [32, 1],
            Self::SpeculativeS1K4C8192 => match role {
                Qwen3GemmModelRoleV1::Target8B => [1, 5],
                Qwen3GemmModelRoleV1::Draft06B => [1, 4],
            },
            Self::SpeculativeS8K4C8192 => match role {
                Qwen3GemmModelRoleV1::Target8B => [8, 5],
                Qwen3GemmModelRoleV1::Draft06B => [8, 4],
            },
            Self::SpeculativeS1K8C8192 => match role {
                Qwen3GemmModelRoleV1::Target8B => [1, 9],
                Qwen3GemmModelRoleV1::Draft06B => [1, 8],
            },
            Self::SpeculativeS1K16C8192 => match role {
                Qwen3GemmModelRoleV1::Target8B => [1, 17],
                Qwen3GemmModelRoleV1::Draft06B => [1, 16],
            },
        }
    }
}

/// One exact role and mode-bucket selection.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Qwen3GemmBucketV1 {
    role: Qwen3GemmModelRoleV1,
    kind: Qwen3GemmBucketKindV1,
}

impl Qwen3GemmBucketV1 {
    /// Creates one finite role/bucket selection.
    #[must_use]
    pub const fn new(role: Qwen3GemmModelRoleV1, kind: Qwen3GemmBucketKindV1) -> Self {
        Self { role, kind }
    }

    /// Exact model role.
    #[must_use]
    pub const fn role(self) -> Qwen3GemmModelRoleV1 {
        self.role
    }

    /// Exact bucket kind.
    #[must_use]
    pub const fn kind(self) -> Qwen3GemmBucketKindV1 {
        self.kind
    }

    /// Exact `[sequences, active_tokens]` dimensions.
    #[must_use]
    pub const fn sequence_and_active_tokens(self) -> [u32; 2] {
        self.kind.sequence_and_active_tokens(self.role)
    }

    /// Exact flattened dense-projection row count.
    #[must_use]
    pub const fn flattened_rows(self) -> u32 {
        let dimensions = self.sequence_and_active_tokens();
        dimensions[0] * dimensions[1]
    }
}

/// Dense Qwen3 operations compiled by this lane.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Qwen3GemmOperationV1 {
    /// Hidden to all query heads.
    QueryProjection = 1,
    /// Hidden to all key heads.
    KeyProjection = 2,
    /// Hidden to all value heads.
    ValueProjection = 3,
    /// Flattened attention width to hidden width with residual.
    AttentionOutputResidual = 4,
    /// Hidden to gated-MLP gate width.
    GateProjection = 5,
    /// Hidden to gated-MLP up width.
    UpProjection = 6,
    /// Intermediate to hidden width with residual.
    DownResidual = 7,
    /// Hidden to full-vocabulary logits.
    LogitsProjection = 8,
}

impl Qwen3GemmOperationV1 {
    const fn dimensions(self, role: Qwen3GemmModelRoleV1, m: u32) -> [u32; 3] {
        let hidden = role.hidden_size();
        match self {
            Self::QueryProjection => [m, role.query_width(), hidden],
            Self::KeyProjection | Self::ValueProjection => [m, role.kv_width(), hidden],
            Self::AttentionOutputResidual => [m, hidden, role.query_width()],
            Self::GateProjection | Self::UpProjection => [m, role.intermediate_size(), hidden],
            Self::DownResidual => [m, hidden, role.intermediate_size()],
            Self::LogitsProjection => [m, QWEN3_VOCABULARY_SIZE_V1, hidden],
        }
    }

    const fn beta_bits(self) -> u32 {
        match self {
            Self::AttentionOutputResidual | Self::DownResidual => 1.0_f32.to_bits(),
            _ => 0.0_f32.to_bits(),
        }
    }
}

/// Closed Ferric source schedule catalog.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Qwen3GemmScheduleV1 {
    /// Scalar ascending-K reference transfer for M below 16.
    ReferenceWave64V1 = 1,
    /// Four-element contiguous A transfer with ascending scalar products.
    VectorizedA4Wave64V1 = 2,
}

impl Qwen3GemmScheduleV1 {
    /// Exact kernel entry selected by this schedule.
    #[must_use]
    pub const fn kernel_symbol(self) -> &'static str {
        match self {
            Self::ReferenceWave64V1 => QWEN3_GEMM_REFERENCE_KERNEL_SYMBOL_V1,
            Self::VectorizedA4Wave64V1 => QWEN3_GEMM_VECTORIZED_KERNEL_SYMBOL_V1,
        }
    }
}

/// Whether a profile is the M=1 GEMV case or a multi-row GEMM case.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Qwen3GemmExecutionClassV1 {
    /// One flattened row.
    GemvM1,
    /// Two or more flattened rows.
    TiledGemm,
}

/// Exact declared arithmetic/storage policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Qwen3GemmNumericalPolicyV1 {
    /// BF16 inputs/output, ascending separate FP32 mul/add, optional widened
    /// BF16 residual, and BF16 RNE narrowing.
    Bf16StorageAscendingFp32Bf16Rne = 1,
}

/// SHA-256 identity of one exact profile record.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Qwen3GemmProfileIdentityV1([u8; 32]);

impl Qwen3GemmProfileIdentityV1 {
    /// Returns the domain-separated identity bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// One finite checked Qwen3 projection profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Qwen3GemmProfileV1 {
    bucket: Qwen3GemmBucketV1,
    operation: Qwen3GemmOperationV1,
    schedule: Qwen3GemmScheduleV1,
    dimensions: [u32; 3],
    strides: [u32; 3],
    storage_elements: [u64; 3],
    block_counts: [u32; 3],
    aql_grid_workitems: [u32; 3],
    reduction_phases: u32,
    numerical_policy: Qwen3GemmNumericalPolicyV1,
    identity: Qwen3GemmProfileIdentityV1,
}

impl Qwen3GemmProfileV1 {
    fn checked(
        bucket: Qwen3GemmBucketV1,
        operation: Qwen3GemmOperationV1,
    ) -> Result<Self, Qwen3GemmCatalogErrorV1> {
        let [sequences, active_tokens] = bucket.sequence_and_active_tokens();
        let m = sequences
            .checked_mul(active_tokens)
            .ok_or(Qwen3GemmCatalogErrorV1::ExtentOverflow)?;
        let dimensions = operation.dimensions(bucket.role, m);
        let [m, n, k] = dimensions;
        if m == 0 || n == 0 || k == 0 || !k.is_multiple_of(4) {
            return Err(Qwen3GemmCatalogErrorV1::ArithmeticInvariant);
        }
        let strides = [k, n, n];
        let storage_elements = [
            u64::from(m)
                .checked_mul(u64::from(k))
                .ok_or(Qwen3GemmCatalogErrorV1::ExtentOverflow)?,
            u64::from(k)
                .checked_mul(u64::from(n))
                .ok_or(Qwen3GemmCatalogErrorV1::ExtentOverflow)?,
            u64::from(m)
                .checked_mul(u64::from(n))
                .ok_or(Qwen3GemmCatalogErrorV1::ExtentOverflow)?,
        ];
        if storage_elements
            .iter()
            .any(|extent| *extent > i64::MAX as u64)
        {
            return Err(Qwen3GemmCatalogErrorV1::ExtentOverflow);
        }
        let block_x = checked_ceil_div_16(n).ok_or(Qwen3GemmCatalogErrorV1::GridOverflow)?;
        let block_y = checked_ceil_div_16(m).ok_or(Qwen3GemmCatalogErrorV1::GridOverflow)?;
        let grid_x = block_x
            .checked_mul(QWEN3_GEMM_WORKGROUP_V1[0])
            .ok_or(Qwen3GemmCatalogErrorV1::GridOverflow)?;
        let schedule = if m < 16 {
            Qwen3GemmScheduleV1::ReferenceWave64V1
        } else {
            Qwen3GemmScheduleV1::VectorizedA4Wave64V1
        };
        let mut profile = Self {
            bucket,
            operation,
            schedule,
            dimensions,
            strides,
            storage_elements,
            block_counts: [block_x, block_y, 1],
            aql_grid_workitems: [grid_x, block_y, 1],
            reduction_phases: k / 4,
            numerical_policy: Qwen3GemmNumericalPolicyV1::Bf16StorageAscendingFp32Bf16Rne,
            identity: Qwen3GemmProfileIdentityV1([0; 32]),
        };
        profile.identity = Qwen3GemmProfileIdentityV1(hash(PROFILE_DOMAIN, &profile.encode()));
        Ok(profile)
    }

    fn encode(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(128);
        bytes.extend_from_slice(&[
            self.bucket.role as u8,
            self.bucket.kind as u8,
            self.operation as u8,
            self.schedule as u8,
            self.numerical_policy as u8,
        ]);
        for value in self.dimensions.into_iter().chain(self.strides) {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in self.storage_elements {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in self.block_counts.into_iter().chain(self.aql_grid_workitems) {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(&self.reduction_phases.to_le_bytes());
        bytes.extend_from_slice(&1.0_f32.to_bits().to_le_bytes());
        bytes.extend_from_slice(&self.operation.beta_bits().to_le_bytes());
        bytes
    }

    /// Exact role and bucket selection.
    #[must_use]
    pub const fn bucket(self) -> Qwen3GemmBucketV1 {
        self.bucket
    }

    /// Exact graph operation.
    #[must_use]
    pub const fn operation(self) -> Qwen3GemmOperationV1 {
        self.operation
    }

    /// Closed source schedule selected by row count.
    #[must_use]
    pub const fn schedule(self) -> Qwen3GemmScheduleV1 {
        self.schedule
    }

    /// Exact `[M,N,K]` dimensions.
    #[must_use]
    pub const fn dimensions(self) -> [u32; 3] {
        self.dimensions
    }

    /// Exact row-major `[lda,ldb,ldc]` strides in elements.
    #[must_use]
    pub const fn strides(self) -> [u32; 3] {
        self.strides
    }

    /// Exact `[A BF16,B BF16,C BF16]` element extents.
    #[must_use]
    pub const fn storage_elements(self) -> [u64; 3] {
        self.storage_elements
    }

    /// Exact HSA-adapter block counts.
    #[must_use]
    pub const fn hsa_adapter_block_counts(self) -> [u32; 3] {
        self.block_counts
    }

    /// Exact AQL total-workitem grid.
    #[must_use]
    pub const fn aql_grid_workitems(self) -> [u32; 3] {
        self.aql_grid_workitems
    }

    /// Exact number of four-element source reduction phases.
    #[must_use]
    pub const fn reduction_phases(self) -> u32 {
        self.reduction_phases
    }

    /// Exact alpha bits, always FP32 one.
    #[must_use]
    pub const fn alpha_bits(self) -> u32 {
        1.0_f32.to_bits()
    }

    /// Exact beta bits, FP32 one only for residual operations.
    #[must_use]
    pub const fn beta_bits(self) -> u32 {
        self.operation.beta_bits()
    }

    /// Exact declared numerical policy.
    #[must_use]
    pub const fn numerical_policy(self) -> Qwen3GemmNumericalPolicyV1 {
        self.numerical_policy
    }

    /// GEMV only for M=1; tiled GEMM otherwise.
    #[must_use]
    pub const fn execution_class(self) -> Qwen3GemmExecutionClassV1 {
        if self.dimensions[0] == 1 {
            Qwen3GemmExecutionClassV1::GemvM1
        } else {
            Qwen3GemmExecutionClassV1::TiledGemm
        }
    }

    /// Exact domain-separated profile identity.
    #[must_use]
    pub const fn identity(self) -> Qwen3GemmProfileIdentityV1 {
        self.identity
    }

    /// Checked host geometry is not machine or arithmetic-refinement evidence.
    #[must_use]
    pub const fn proves_machine_arithmetic(self) -> bool {
        false
    }
}

/// Failure while deriving the immutable finite catalog.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Qwen3GemmCatalogErrorV1 {
    /// A matrix extent overflowed its checked domain.
    ExtentOverflow,
    /// Workgroup expansion overflowed the AQL grid domain.
    GridOverflow,
    /// A finite shape violated the source's exact arithmetic preconditions.
    ArithmeticInvariant,
}

impl fmt::Display for Qwen3GemmCatalogErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Qwen3 GEMM catalog failed: {self:?}")
    }
}

impl std::error::Error for Qwen3GemmCatalogErrorV1 {}

/// SHA-256 identity of the complete finite profile catalog.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Qwen3GemmProfileCatalogIdentityV1([u8; 32]);

impl Qwen3GemmProfileCatalogIdentityV1 {
    /// Returns the domain-separated identity bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Complete finite target/draft GEMM/GEMV profile catalog.
#[derive(Debug, Eq, PartialEq)]
pub struct Qwen3GemmProfileCatalogV1 {
    profiles: Box<[Qwen3GemmProfileV1]>,
    canonical_bytes: Box<[u8]>,
    identity: Qwen3GemmProfileCatalogIdentityV1,
}

impl Qwen3GemmProfileCatalogV1 {
    /// Constructs all 176 profiles in stable role/bucket/operation order.
    ///
    /// # Errors
    ///
    /// Returns an error if any exact profile geometry or catalog extent is invalid.
    pub fn canonical() -> Result<Self, Qwen3GemmCatalogErrorV1> {
        let mut profiles = Vec::with_capacity(QWEN3_GEMM_PROFILE_COUNT_V1);
        for role in QWEN3_GEMM_ROLES_V1 {
            for kind in QWEN3_GEMM_BUCKET_KINDS_V1 {
                let bucket = Qwen3GemmBucketV1::new(role, kind);
                for operation in QWEN3_GEMM_OPERATIONS_V1 {
                    profiles.push(Qwen3GemmProfileV1::checked(bucket, operation)?);
                }
            }
        }
        let mut canonical_bytes = Vec::with_capacity(profiles.len() * 160);
        let profile_count =
            u32::try_from(profiles.len()).map_err(|_| Qwen3GemmCatalogErrorV1::ExtentOverflow)?;
        canonical_bytes.extend_from_slice(&profile_count.to_le_bytes());
        for profile in &profiles {
            let encoded = profile.encode();
            let encoded_len = u32::try_from(encoded.len())
                .map_err(|_| Qwen3GemmCatalogErrorV1::ExtentOverflow)?;
            canonical_bytes.extend_from_slice(&encoded_len.to_le_bytes());
            canonical_bytes.extend_from_slice(&encoded);
            canonical_bytes.extend_from_slice(profile.identity.as_bytes());
        }
        let identity = Qwen3GemmProfileCatalogIdentityV1(hash(CATALOG_DOMAIN, &canonical_bytes));
        Ok(Self {
            profiles: profiles.into_boxed_slice(),
            canonical_bytes: canonical_bytes.into_boxed_slice(),
            identity,
        })
    }

    /// Exact stable profile roster.
    #[must_use]
    pub fn profiles(&self) -> &[Qwen3GemmProfileV1] {
        &self.profiles
    }

    /// Finds one exact finite profile.
    #[must_use]
    pub fn profile(
        &self,
        bucket: Qwen3GemmBucketV1,
        operation: Qwen3GemmOperationV1,
    ) -> Option<Qwen3GemmProfileV1> {
        self.profiles
            .iter()
            .copied()
            .find(|profile| profile.bucket == bucket && profile.operation == operation)
    }

    /// Exact canonical catalog bytes.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    /// Exact catalog identity.
    #[must_use]
    pub const fn identity(&self) -> Qwen3GemmProfileCatalogIdentityV1 {
        self.identity
    }

    /// This host roster grants no source, artifact, or launch authority.
    #[must_use]
    pub const fn grants_authority(&self) -> bool {
        false
    }
}

const QWEN3_GEMM_ROLES_V1: [Qwen3GemmModelRoleV1; 2] = [
    Qwen3GemmModelRoleV1::Target8B,
    Qwen3GemmModelRoleV1::Draft06B,
];

const QWEN3_GEMM_BUCKET_KINDS_V1: [Qwen3GemmBucketKindV1; 11] = [
    Qwen3GemmBucketKindV1::PrefillS1T128,
    Qwen3GemmBucketKindV1::PrefillS8T128,
    Qwen3GemmBucketKindV1::PrefillS1T512,
    Qwen3GemmBucketKindV1::PrefillS1T2048,
    Qwen3GemmBucketKindV1::DecodeS1C8192,
    Qwen3GemmBucketKindV1::DecodeS8C8192,
    Qwen3GemmBucketKindV1::DecodeS32C8192,
    Qwen3GemmBucketKindV1::SpeculativeS1K4C8192,
    Qwen3GemmBucketKindV1::SpeculativeS8K4C8192,
    Qwen3GemmBucketKindV1::SpeculativeS1K8C8192,
    Qwen3GemmBucketKindV1::SpeculativeS1K16C8192,
];

const QWEN3_GEMM_OPERATIONS_V1: [Qwen3GemmOperationV1; 8] = [
    Qwen3GemmOperationV1::QueryProjection,
    Qwen3GemmOperationV1::KeyProjection,
    Qwen3GemmOperationV1::ValueProjection,
    Qwen3GemmOperationV1::AttentionOutputResidual,
    Qwen3GemmOperationV1::GateProjection,
    Qwen3GemmOperationV1::UpProjection,
    Qwen3GemmOperationV1::DownResidual,
    Qwen3GemmOperationV1::LogitsProjection,
];

/// SHA-256 identity of one exact token-embedding profile.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Qwen3TokenEmbeddingProfileIdentityV1([u8; 32]);

impl Qwen3TokenEmbeddingProfileIdentityV1 {
    /// Returns the domain-separated identity bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// One finite token-ID to BF16 embedding lookup profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Qwen3TokenEmbeddingProfileV1 {
    bucket: Qwen3GemmBucketV1,
    shape: [u32; 3],
    storage_elements: [u64; 3],
    aql_grid_workitems: [u32; 3],
    identity: Qwen3TokenEmbeddingProfileIdentityV1,
}

impl Qwen3TokenEmbeddingProfileV1 {
    fn checked(bucket: Qwen3GemmBucketV1) -> Result<Self, Qwen3TokenEmbeddingCatalogErrorV1> {
        let [sequences, active_tokens] = bucket.sequence_and_active_tokens();
        let rows = sequences
            .checked_mul(active_tokens)
            .ok_or(Qwen3TokenEmbeddingCatalogErrorV1::ExtentOverflow)?;
        let hidden = bucket.role().hidden_size();
        let weight_elements = u64::from(QWEN3_VOCABULARY_SIZE_V1)
            .checked_mul(u64::from(hidden))
            .ok_or(Qwen3TokenEmbeddingCatalogErrorV1::ExtentOverflow)?;
        let output_elements = u64::from(rows)
            .checked_mul(u64::from(hidden))
            .ok_or(Qwen3TokenEmbeddingCatalogErrorV1::ExtentOverflow)?;
        let grid_x = u32::try_from(output_elements)
            .map_err(|_| Qwen3TokenEmbeddingCatalogErrorV1::GridOverflow)?;
        if rows == 0
            || hidden == 0
            || !grid_x.is_multiple_of(QWEN3_GEMM_WORKGROUP_V1[0])
            || [u64::from(rows), weight_elements, output_elements]
                .iter()
                .any(|extent| *extent > i64::MAX as u64 || extent.checked_mul(4).is_none())
        {
            return Err(Qwen3TokenEmbeddingCatalogErrorV1::ArithmeticInvariant);
        }
        let mut profile = Self {
            bucket,
            shape: [sequences, active_tokens, hidden],
            storage_elements: [u64::from(rows), weight_elements, output_elements],
            aql_grid_workitems: [grid_x, 1, 1],
            identity: Qwen3TokenEmbeddingProfileIdentityV1([0; 32]),
        };
        profile.identity =
            Qwen3TokenEmbeddingProfileIdentityV1(hash(EMBEDDING_PROFILE_DOMAIN, &profile.encode()));
        Ok(profile)
    }

    fn encode(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(96);
        bytes.extend_from_slice(&[self.bucket.role() as u8, self.bucket.kind() as u8]);
        bytes.extend_from_slice(&QWEN3_VOCABULARY_SIZE_V1.to_le_bytes());
        for value in self.shape {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in self.storage_elements {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in self.aql_grid_workitems {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    /// Exact role and bucket selection.
    #[must_use]
    pub const fn bucket(self) -> Qwen3GemmBucketV1 {
        self.bucket
    }

    /// Exact `[sequences, active tokens, hidden]` shape.
    #[must_use]
    pub const fn shape(self) -> [u32; 3] {
        self.shape
    }

    /// Exact flattened row count.
    #[must_use]
    pub const fn rows(self) -> u32 {
        self.shape[0] * self.shape[1]
    }

    /// Exact hidden width.
    #[must_use]
    pub const fn hidden_size(self) -> u32 {
        self.shape[2]
    }

    /// Exact `[token IDs, weight BF16, output BF16]` element extents.
    #[must_use]
    pub const fn storage_elements(self) -> [u64; 3] {
        self.storage_elements
    }

    /// Exact AQL total-workitem grid.
    #[must_use]
    pub const fn aql_grid_workitems(self) -> [u32; 3] {
        self.aql_grid_workitems
    }

    /// Exact domain-separated profile identity.
    #[must_use]
    pub const fn identity(self) -> Qwen3TokenEmbeddingProfileIdentityV1 {
        self.identity
    }

    /// Profile structure does not prove weight content or machine execution.
    #[must_use]
    pub const fn authenticates_weight_content(self) -> bool {
        false
    }
}

/// Failure while deriving the finite token-embedding catalog.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Qwen3TokenEmbeddingCatalogErrorV1 {
    /// A storage extent overflowed.
    ExtentOverflow,
    /// The exact total-workitem grid did not fit the admitted domain.
    GridOverflow,
    /// A finite shape violated the exact source preconditions.
    ArithmeticInvariant,
    /// The exact 22-profile roster drifted.
    ProfileSet,
    /// Two host profiles had the same identity.
    DuplicateIdentity,
}

impl fmt::Display for Qwen3TokenEmbeddingCatalogErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Qwen3 token-embedding catalog failed: {self:?}")
    }
}

impl std::error::Error for Qwen3TokenEmbeddingCatalogErrorV1 {}

/// SHA-256 identity of the complete token-embedding catalog.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Qwen3TokenEmbeddingProfileCatalogIdentityV1([u8; 32]);

impl Qwen3TokenEmbeddingProfileCatalogIdentityV1 {
    /// Returns the domain-separated identity bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Complete finite target/draft token-embedding profile catalog.
#[derive(Debug, Eq, PartialEq)]
pub struct Qwen3TokenEmbeddingProfileCatalogV1 {
    profiles: Box<[Qwen3TokenEmbeddingProfileV1]>,
    canonical_bytes: Box<[u8]>,
    identity: Qwen3TokenEmbeddingProfileCatalogIdentityV1,
}

impl Qwen3TokenEmbeddingProfileCatalogV1 {
    /// Constructs all 22 profiles in stable role/bucket order.
    ///
    /// # Errors
    ///
    /// Returns an error if an exact extent, grid, roster, or identity invariant fails.
    pub fn canonical() -> Result<Self, Qwen3TokenEmbeddingCatalogErrorV1> {
        let mut profiles = Vec::with_capacity(QWEN3_TOKEN_EMBEDDING_PROFILE_COUNT_V1);
        for role in QWEN3_GEMM_ROLES_V1 {
            for kind in QWEN3_GEMM_BUCKET_KINDS_V1 {
                profiles.push(Qwen3TokenEmbeddingProfileV1::checked(
                    Qwen3GemmBucketV1::new(role, kind),
                )?);
            }
        }
        if profiles.len() != QWEN3_TOKEN_EMBEDDING_PROFILE_COUNT_V1 {
            return Err(Qwen3TokenEmbeddingCatalogErrorV1::ProfileSet);
        }
        for index in 0..profiles.len() {
            if profiles[index + 1..]
                .iter()
                .any(|profile| profile.identity == profiles[index].identity)
            {
                return Err(Qwen3TokenEmbeddingCatalogErrorV1::DuplicateIdentity);
            }
        }
        let mut canonical_bytes = Vec::with_capacity(profiles.len() * 128);
        canonical_bytes.extend_from_slice(
            &u32::try_from(profiles.len())
                .map_err(|_| Qwen3TokenEmbeddingCatalogErrorV1::ExtentOverflow)?
                .to_le_bytes(),
        );
        for profile in &profiles {
            let encoded = profile.encode();
            canonical_bytes.extend_from_slice(
                &u32::try_from(encoded.len())
                    .map_err(|_| Qwen3TokenEmbeddingCatalogErrorV1::ExtentOverflow)?
                    .to_le_bytes(),
            );
            canonical_bytes.extend_from_slice(&encoded);
            canonical_bytes.extend_from_slice(profile.identity.as_bytes());
        }
        let identity = Qwen3TokenEmbeddingProfileCatalogIdentityV1(hash(
            EMBEDDING_CATALOG_DOMAIN,
            &canonical_bytes,
        ));
        Ok(Self {
            profiles: profiles.into_boxed_slice(),
            canonical_bytes: canonical_bytes.into_boxed_slice(),
            identity,
        })
    }

    /// Exact stable profile roster.
    #[must_use]
    pub fn profiles(&self) -> &[Qwen3TokenEmbeddingProfileV1] {
        &self.profiles
    }

    /// Finds one exact finite profile.
    #[must_use]
    pub fn profile(&self, bucket: Qwen3GemmBucketV1) -> Option<Qwen3TokenEmbeddingProfileV1> {
        self.profiles
            .iter()
            .copied()
            .find(|profile| profile.bucket == bucket)
    }

    /// Exact canonical catalog bytes.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    /// Exact catalog identity.
    #[must_use]
    pub const fn identity(&self) -> Qwen3TokenEmbeddingProfileCatalogIdentityV1 {
        self.identity
    }

    /// This host roster grants no source, content, artifact, or launch authority.
    #[must_use]
    pub const fn grants_authority(&self) -> bool {
        false
    }
}

/// Ferric-admission-supplied expected embedding-weight identity.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Qwen3ExpectedEmbeddingWeightIdentityV1 {
    role: Qwen3GemmModelRoleV1,
    bytes: [u8; 32],
}

impl Qwen3ExpectedEmbeddingWeightIdentityV1 {
    /// Constructs a nonzero opaque bundle-bound identity for one model role.
    ///
    /// # Errors
    ///
    /// Returns an error for the absent all-zero identity.
    pub fn new(
        role: Qwen3GemmModelRoleV1,
        bytes: [u8; 32],
    ) -> Result<Self, Qwen3ExpectedEmbeddingWeightIdentityErrorV1> {
        if bytes == [0; 32] {
            return Err(Qwen3ExpectedEmbeddingWeightIdentityErrorV1::Absent);
        }
        Ok(Self { role, bytes })
    }

    /// Exact role supplied by Ferric admission.
    #[must_use]
    pub const fn role(self) -> Qwen3GemmModelRoleV1 {
        self.role
    }

    /// Opaque expected identity bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.bytes
    }

    /// Retention is not independent content authentication.
    #[must_use]
    pub const fn authenticates_content(self) -> bool {
        false
    }
}

/// Invalid expected embedding-weight identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Qwen3ExpectedEmbeddingWeightIdentityErrorV1 {
    /// The identity was all zero.
    Absent,
}

impl fmt::Display for Qwen3ExpectedEmbeddingWeightIdentityErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "expected embedding-weight identity failed: {self:?}"
        )
    }
}

impl std::error::Error for Qwen3ExpectedEmbeddingWeightIdentityErrorV1 {}

/// One token-embedding ABI region.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Qwen3TokenEmbeddingBufferV1 {
    /// U32 token IDs `[S,A]`.
    TokenIds = 1,
    /// BF16 embedding weight `[151936,H]`.
    Weight = 2,
    /// BF16 output `[S,A,H]`.
    Output = 3,
}

/// Exact token-embedding buffer admission failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Qwen3TokenEmbeddingBufferContractErrorV1 {
    /// The supplied expected weight identity belongs to the other model role.
    WeightRole,
    /// A required address was zero.
    ZeroAddress(Qwen3TokenEmbeddingBufferV1),
    /// A byte span differed from the exact profile extent.
    ByteLength(Qwen3TokenEmbeddingBufferV1),
    /// An address violated its element alignment.
    Alignment(Qwen3TokenEmbeddingBufferV1),
    /// An exclusive end overflowed u64.
    RangeOverflow(Qwen3TokenEmbeddingBufferV1),
    /// Two exact regions overlapped.
    Aliasing,
}

impl fmt::Display for Qwen3TokenEmbeddingBufferContractErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Qwen3 token-embedding buffer contract failed: {self:?}"
        )
    }
}

impl std::error::Error for Qwen3TokenEmbeddingBufferContractErrorV1 {}

/// Exact checked token-ID, expected-weight, and output spans.
#[derive(Debug, Eq, PartialEq)]
pub struct Qwen3TokenEmbeddingBufferContractV1 {
    expected_weight_identity: Qwen3ExpectedEmbeddingWeightIdentityV1,
    addresses: [u64; 3],
    ends: [u64; 3],
    byte_lengths: [u64; 3],
}

impl Qwen3TokenEmbeddingBufferContractV1 {
    fn checked(
        profile: Qwen3TokenEmbeddingProfileV1,
        expected_weight_identity: Qwen3ExpectedEmbeddingWeightIdentityV1,
        addresses: [u64; 3],
        byte_lengths: [u64; 3],
    ) -> Result<Self, Qwen3TokenEmbeddingBufferContractErrorV1> {
        if expected_weight_identity.role() != profile.bucket().role() {
            return Err(Qwen3TokenEmbeddingBufferContractErrorV1::WeightRole);
        }
        let elements = profile.storage_elements();
        let expected = [
            elements[0].checked_mul(4).ok_or(
                Qwen3TokenEmbeddingBufferContractErrorV1::ByteLength(
                    Qwen3TokenEmbeddingBufferV1::TokenIds,
                ),
            )?,
            elements[1].checked_mul(2).ok_or(
                Qwen3TokenEmbeddingBufferContractErrorV1::ByteLength(
                    Qwen3TokenEmbeddingBufferV1::Weight,
                ),
            )?,
            elements[2].checked_mul(2).ok_or(
                Qwen3TokenEmbeddingBufferContractErrorV1::ByteLength(
                    Qwen3TokenEmbeddingBufferV1::Output,
                ),
            )?,
        ];
        let roles = [
            Qwen3TokenEmbeddingBufferV1::TokenIds,
            Qwen3TokenEmbeddingBufferV1::Weight,
            Qwen3TokenEmbeddingBufferV1::Output,
        ];
        let alignments = [4, 2, 2];
        let mut ends = [0; 3];
        for index in 0..3 {
            if addresses[index] == 0 {
                return Err(Qwen3TokenEmbeddingBufferContractErrorV1::ZeroAddress(
                    roles[index],
                ));
            }
            if byte_lengths[index] != expected[index] {
                return Err(Qwen3TokenEmbeddingBufferContractErrorV1::ByteLength(
                    roles[index],
                ));
            }
            if !addresses[index].is_multiple_of(alignments[index]) {
                return Err(Qwen3TokenEmbeddingBufferContractErrorV1::Alignment(
                    roles[index],
                ));
            }
            ends[index] = addresses[index].checked_add(byte_lengths[index]).ok_or(
                Qwen3TokenEmbeddingBufferContractErrorV1::RangeOverflow(roles[index]),
            )?;
        }
        for left in 0..3 {
            for right in left + 1..3 {
                if addresses[left] < ends[right] && addresses[right] < ends[left] {
                    return Err(Qwen3TokenEmbeddingBufferContractErrorV1::Aliasing);
                }
            }
        }
        Ok(Self {
            expected_weight_identity,
            addresses,
            ends,
            byte_lengths,
        })
    }

    /// Opaque expected weight identity retained from Ferric admission.
    #[must_use]
    pub const fn expected_weight_identity(&self) -> Qwen3ExpectedEmbeddingWeightIdentityV1 {
        self.expected_weight_identity
    }

    /// Exact start addresses in token-ID, weight, output order.
    #[must_use]
    pub const fn addresses(&self) -> &[u64; 3] {
        &self.addresses
    }

    /// Exact exclusive ends.
    #[must_use]
    pub const fn ends(&self) -> &[u64; 3] {
        &self.ends
    }

    /// Exact byte lengths.
    #[must_use]
    pub const fn byte_lengths(&self) -> &[u64; 3] {
        &self.byte_lengths
    }

    /// This join does not authenticate content, allocation, or device memory.
    #[must_use]
    pub const fn authenticates_content_or_memory(&self) -> bool {
        false
    }
}

/// Semantic token-embedding ABI role.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Qwen3TokenEmbeddingArgumentRoleV1 {
    /// U32 token IDs.
    TokenIds = 1,
    /// Bundle-bound BF16 embedding weights.
    ExpectedWeight = 2,
    /// BF16 output.
    Output = 3,
}

/// One exact token-embedding pointer-plus-length ABI declaration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Qwen3TokenEmbeddingArgumentV1 {
    /// Semantic buffer role.
    pub role: Qwen3TokenEmbeddingArgumentRoleV1,
    /// Explicit kernarg byte offset.
    pub offset: u32,
    /// Pointer-plus-length record size.
    pub size: u32,
    /// Pointee alignment.
    pub pointee_alignment: u32,
}

/// Ferric-owned semantic sidecar for one token-embedding profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Qwen3TokenEmbeddingKernelIrV1 {
    profile_identity: Qwen3TokenEmbeddingProfileIdentityV1,
    arguments: [Qwen3TokenEmbeddingArgumentV1; 3],
    shape: [u32; 3],
    identity: [u8; 32],
}

impl Qwen3TokenEmbeddingKernelIrV1 {
    /// Profile identity retained by this sidecar.
    #[must_use]
    pub const fn profile_identity(&self) -> Qwen3TokenEmbeddingProfileIdentityV1 {
        self.profile_identity
    }

    /// Exact three-slice ABI.
    #[must_use]
    pub const fn arguments(&self) -> &[Qwen3TokenEmbeddingArgumentV1; 3] {
        &self.arguments
    }

    /// Exact `[sequences, active tokens, hidden]` shape.
    #[must_use]
    pub const fn shape(&self) -> [u32; 3] {
        self.shape
    }

    /// Domain-separated sidecar identity.
    #[must_use]
    pub const fn identity(&self) -> &[u8; 32] {
        &self.identity
    }

    /// This sidecar is not source-to-machine or content refinement.
    #[must_use]
    pub const fn proves_refinement(&self) -> bool {
        false
    }
}

/// Constructs the canonical token-embedding semantic sidecar.
#[must_use]
pub fn qwen3_token_embedding_kernel_ir_v1(
    profile: Qwen3TokenEmbeddingProfileV1,
) -> Qwen3TokenEmbeddingKernelIrV1 {
    let arguments = [
        Qwen3TokenEmbeddingArgumentV1 {
            role: Qwen3TokenEmbeddingArgumentRoleV1::TokenIds,
            offset: 0,
            size: 16,
            pointee_alignment: 4,
        },
        Qwen3TokenEmbeddingArgumentV1 {
            role: Qwen3TokenEmbeddingArgumentRoleV1::ExpectedWeight,
            offset: 16,
            size: 16,
            pointee_alignment: 2,
        },
        Qwen3TokenEmbeddingArgumentV1 {
            role: Qwen3TokenEmbeddingArgumentRoleV1::Output,
            offset: 32,
            size: 16,
            pointee_alignment: 2,
        },
    ];
    let mut encoded = Vec::with_capacity(160);
    encoded.extend_from_slice(b"ferric::qwen3::token_embedding_v1");
    encoded.extend_from_slice(QWEN3_TOKEN_EMBEDDING_KERNEL_SYMBOL_V1.as_bytes());
    encoded.extend_from_slice(profile.identity().as_bytes());
    for value in profile.shape() {
        encoded.extend_from_slice(&value.to_le_bytes());
    }
    for argument in arguments {
        encoded.push(argument.role as u8);
        encoded.extend_from_slice(&argument.offset.to_le_bytes());
        encoded.extend_from_slice(&argument.size.to_le_bytes());
        encoded.extend_from_slice(&argument.pointee_alignment.to_le_bytes());
    }
    Qwen3TokenEmbeddingKernelIrV1 {
        profile_identity: profile.identity(),
        arguments,
        shape: profile.shape(),
        identity: hash(EMBEDDING_KERNEL_IR_DOMAIN, &encoded),
    }
}

/// One of the three exact ABI regions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Qwen3GemmBufferV1 {
    /// Row-major BF16 activation A `[M,K]`.
    A = 1,
    /// Prepared row-major BF16 weight B `[K,N]`.
    B = 2,
    /// Row-major BF16 output/residual C `[M,N]`.
    C = 3,
}

/// Exact numerical buffer admission failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Qwen3GemmBufferContractErrorV1 {
    /// A required address was zero.
    ZeroAddress(Qwen3GemmBufferV1),
    /// A byte span differed from the exact profile extent.
    ByteLength(Qwen3GemmBufferV1),
    /// An address violated BF16 alignment.
    Alignment(Qwen3GemmBufferV1),
    /// An exclusive end overflowed u64.
    RangeOverflow(Qwen3GemmBufferV1),
    /// Two exact numerical regions overlapped.
    Aliasing,
}

impl fmt::Display for Qwen3GemmBufferContractErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Qwen3 GEMM buffer contract failed: {self:?}")
    }
}

impl std::error::Error for Qwen3GemmBufferContractErrorV1 {}

/// Exact checked A/B/C spans.
#[derive(Debug, Eq, PartialEq)]
pub struct Qwen3GemmBufferContractV1 {
    addresses: [u64; 3],
    ends: [u64; 3],
    byte_lengths: [u64; 3],
}

impl Qwen3GemmBufferContractV1 {
    /// Checks exact BF16 lengths, alignment, overflow, and pairwise disjointness.
    ///
    /// # Errors
    ///
    /// Returns an error for a zero, misaligned, overflowing, incorrectly sized,
    /// or overlapping buffer span.
    pub fn checked(
        profile: Qwen3GemmProfileV1,
        addresses: [u64; 3],
        byte_lengths: [u64; 3],
    ) -> Result<Self, Qwen3GemmBufferContractErrorV1> {
        let elements = profile.storage_elements;
        let expected = [
            elements[0]
                .checked_mul(2)
                .ok_or(Qwen3GemmBufferContractErrorV1::ByteLength(
                    Qwen3GemmBufferV1::A,
                ))?,
            elements[1]
                .checked_mul(2)
                .ok_or(Qwen3GemmBufferContractErrorV1::ByteLength(
                    Qwen3GemmBufferV1::B,
                ))?,
            elements[2]
                .checked_mul(2)
                .ok_or(Qwen3GemmBufferContractErrorV1::ByteLength(
                    Qwen3GemmBufferV1::C,
                ))?,
        ];
        let roles = [
            Qwen3GemmBufferV1::A,
            Qwen3GemmBufferV1::B,
            Qwen3GemmBufferV1::C,
        ];
        let mut ends = [0; 3];
        for index in 0..3 {
            if addresses[index] == 0 {
                return Err(Qwen3GemmBufferContractErrorV1::ZeroAddress(roles[index]));
            }
            if byte_lengths[index] != expected[index] {
                return Err(Qwen3GemmBufferContractErrorV1::ByteLength(roles[index]));
            }
            if !addresses[index].is_multiple_of(2) {
                return Err(Qwen3GemmBufferContractErrorV1::Alignment(roles[index]));
            }
            ends[index] = addresses[index]
                .checked_add(byte_lengths[index])
                .ok_or(Qwen3GemmBufferContractErrorV1::RangeOverflow(roles[index]))?;
        }
        for left in 0..3 {
            for right in left + 1..3 {
                if addresses[left] < ends[right] && addresses[right] < ends[left] {
                    return Err(Qwen3GemmBufferContractErrorV1::Aliasing);
                }
            }
        }
        Ok(Self {
            addresses,
            ends,
            byte_lengths,
        })
    }

    /// Exact start addresses.
    #[must_use]
    pub const fn addresses(&self) -> &[u64; 3] {
        &self.addresses
    }

    /// Exact exclusive ends.
    #[must_use]
    pub const fn ends(&self) -> &[u64; 3] {
        &self.ends
    }

    /// Exact byte lengths.
    #[must_use]
    pub const fn byte_lengths(&self) -> &[u64; 3] {
        &self.byte_lengths
    }

    /// Numerical spans do not authenticate allocation or device memory.
    #[must_use]
    pub const fn authenticates_device_memory(&self) -> bool {
        false
    }
}

/// Semantic role of one three-slice ABI argument.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Qwen3GemmArgumentRoleV1 {
    /// BF16 activation input A.
    ActivationA = 1,
    /// Prepared BF16 weight input B.
    PreparedWeightB = 2,
    /// BF16 output and optional residual C.
    OutputResidualC = 3,
}

/// Semantic access of one ABI argument.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Qwen3GemmArgumentAccessV1 {
    /// Input-only region.
    ReadOnly = 1,
    /// Read-write region because residual profiles read C before writing it.
    ReadWrite = 2,
}

/// One exact pointer-plus-length ABI declaration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Qwen3GemmArgumentV1 {
    /// Semantic buffer role.
    pub role: Qwen3GemmArgumentRoleV1,
    /// Semantic access mode.
    pub access: Qwen3GemmArgumentAccessV1,
    /// Explicit kernarg byte offset.
    pub offset: u32,
    /// Pointer-plus-length record size.
    pub size: u32,
    /// BF16 pointee alignment.
    pub pointee_alignment: u32,
}

/// Ferric-owned semantic KIR sidecar for one exact profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Qwen3GemmKernelIrV1 {
    module_id: String,
    kernel_id: String,
    profile_identity: Qwen3GemmProfileIdentityV1,
    arguments: [Qwen3GemmArgumentV1; 3],
    dimensions: [u32; 3],
    schedule: Qwen3GemmScheduleV1,
    numerical_policy: Qwen3GemmNumericalPolicyV1,
    identity: [u8; 32],
}

impl Qwen3GemmKernelIrV1 {
    /// Ferric-owned semantic module identity.
    #[must_use]
    pub fn module_id(&self) -> &str {
        &self.module_id
    }

    /// Exact selected kernel identity.
    #[must_use]
    pub fn kernel_id(&self) -> &str {
        &self.kernel_id
    }

    /// Profile identity retained by this sidecar.
    #[must_use]
    pub const fn profile_identity(&self) -> Qwen3GemmProfileIdentityV1 {
        self.profile_identity
    }

    /// Exact three-slice BF16 ABI.
    #[must_use]
    pub const fn arguments(&self) -> &[Qwen3GemmArgumentV1; 3] {
        &self.arguments
    }

    /// Exact `[M,N,K]` dimensions.
    #[must_use]
    pub const fn dimensions(&self) -> [u32; 3] {
        self.dimensions
    }

    /// Exact source schedule.
    #[must_use]
    pub const fn schedule(&self) -> Qwen3GemmScheduleV1 {
        self.schedule
    }

    /// Exact declared arithmetic/storage policy.
    #[must_use]
    pub const fn numerical_policy(&self) -> Qwen3GemmNumericalPolicyV1 {
        self.numerical_policy
    }

    /// Domain-separated KIR identity.
    #[must_use]
    pub const fn identity(&self) -> &[u8; 32] {
        &self.identity
    }

    /// The semantic sidecar is not a source-to-machine refinement proof.
    #[must_use]
    pub const fn proves_machine_refinement(&self) -> bool {
        false
    }
}

/// Constructs the canonical Ferric semantic KIR sidecar for one profile.
#[must_use]
pub fn qwen3_gemm_kernel_ir_v1(profile: Qwen3GemmProfileV1) -> Qwen3GemmKernelIrV1 {
    let arguments = [
        Qwen3GemmArgumentV1 {
            role: Qwen3GemmArgumentRoleV1::ActivationA,
            access: Qwen3GemmArgumentAccessV1::ReadOnly,
            offset: 0,
            size: 16,
            pointee_alignment: 2,
        },
        Qwen3GemmArgumentV1 {
            role: Qwen3GemmArgumentRoleV1::PreparedWeightB,
            access: Qwen3GemmArgumentAccessV1::ReadOnly,
            offset: 16,
            size: 16,
            pointee_alignment: 2,
        },
        Qwen3GemmArgumentV1 {
            role: Qwen3GemmArgumentRoleV1::OutputResidualC,
            access: Qwen3GemmArgumentAccessV1::ReadWrite,
            offset: 32,
            size: 16,
            pointee_alignment: 2,
        },
    ];
    let mut encoded = Vec::with_capacity(160);
    encoded.extend_from_slice(b"ferric::qwen3::dense_gemm_v1");
    encoded.extend_from_slice(profile.schedule.kernel_symbol().as_bytes());
    encoded.extend_from_slice(profile.identity.as_bytes());
    encoded.push(profile.schedule as u8);
    encoded.push(profile.numerical_policy as u8);
    for dimension in profile.dimensions {
        encoded.extend_from_slice(&dimension.to_le_bytes());
    }
    for argument in arguments {
        encoded.extend_from_slice(&[argument.role as u8, argument.access as u8]);
        encoded.extend_from_slice(&argument.offset.to_le_bytes());
        encoded.extend_from_slice(&argument.size.to_le_bytes());
        encoded.extend_from_slice(&argument.pointee_alignment.to_le_bytes());
    }
    Qwen3GemmKernelIrV1 {
        module_id: "ferric::qwen3::dense_gemm_v1".to_owned(),
        kernel_id: profile.schedule.kernel_symbol().to_owned(),
        profile_identity: profile.identity,
        arguments,
        dimensions: profile.dimensions,
        schedule: profile.schedule,
        numerical_policy: profile.numerical_policy,
        identity: hash(KERNEL_IR_DOMAIN, &encoded),
    }
}

/// Four inert Ferric source-stage labels bound into the compiler owner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Qwen3GemmSourceBindingsV1 {
    source: [u8; 32],
    kernel_ir: [u8; 32],
    schedule: [u8; 32],
    target_plan: [u8; 32],
}

impl Qwen3GemmSourceBindingsV1 {
    /// Constructs inert source, KIR, schedule, and target-plan labels.
    #[must_use]
    pub const fn new(
        source: [u8; 32],
        kernel_ir: [u8; 32],
        schedule: [u8; 32],
        target_plan: [u8; 32],
    ) -> Self {
        Self {
            source,
            kernel_ir,
            schedule,
            target_plan,
        }
    }

    /// These labels do not authenticate source provenance or content.
    #[must_use]
    pub const fn authenticates_provenance(self) -> bool {
        false
    }
}

/// Failure while preparing the complete Ferric GEMM compiler owner.
#[derive(Debug)]
pub enum PrepareQwen3GemmKernelErrorV1 {
    /// A source-stage label was zero or repeated.
    SourceBindings,
    /// The finite profile catalog failed.
    Catalog(Qwen3GemmCatalogErrorV1),
    /// The finite token-embedding catalog failed.
    TokenEmbeddingCatalog(Qwen3TokenEmbeddingCatalogErrorV1),
    /// A semantic KIR sidecar did not retain its exact profile.
    KernelIr,
    /// The complete direct-LLVM source or classifier differed from its pin.
    CompilerModule,
    /// The no-device-FFI envelope failed.
    CompilerEnvelope(CompilerFfiEnvelopeError),
    /// The closed symbol manifest failed.
    SymbolManifest(CompilerModuleSymbolManifestErrorV1),
    /// The compiler handoff rejected the exact source module.
    CompilerHandoff(CompilerModuleHandoffErrorV2),
}

impl fmt::Display for PrepareQwen3GemmKernelErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Qwen3 GEMM preparation failed: {self:?}")
    }
}

impl std::error::Error for PrepareQwen3GemmKernelErrorV1 {}

/// Linear prepared compiler owner awaiting Worker request construction.
pub struct PreparedQwen3GemmKernelV1 {
    catalog: Qwen3GemmProfileCatalogV1,
    token_embedding_catalog: Qwen3TokenEmbeddingProfileCatalogV1,
    source_binding_identity: [u8; 32],
    llvm_sha256: [u8; 32],
    compiler_handoff_identity: CompilerModuleHandoffIdentityV2,
    manifest_identity: CompilerModuleSymbolManifestIdentityV1,
    compiler_handoff: CompilerModuleHandoffV2,
}

impl fmt::Debug for PreparedQwen3GemmKernelV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedQwen3GemmKernelV1")
            .field("catalog", &self.catalog.identity)
            .field(
                "token_embedding_catalog",
                &self.token_embedding_catalog.identity,
            )
            .field("source_binding", &self.source_binding_identity)
            .field("llvm_sha256", &self.llvm_sha256)
            .field("compiler_handoff", &self.compiler_handoff_identity)
            .finish_non_exhaustive()
    }
}

impl PreparedQwen3GemmKernelV1 {
    /// Complete finite profile catalog.
    #[must_use]
    pub const fn catalog(&self) -> &Qwen3GemmProfileCatalogV1 {
        &self.catalog
    }

    /// Complete finite token-embedding profile catalog.
    #[must_use]
    pub const fn token_embedding_catalog(&self) -> &Qwen3TokenEmbeddingProfileCatalogV1 {
        &self.token_embedding_catalog
    }

    /// Ferric-domain identity binding labels, catalog, KIRs, and source bytes.
    #[must_use]
    pub const fn source_binding_identity(&self) -> &[u8; 32] {
        &self.source_binding_identity
    }

    /// Exact final source SHA-256.
    #[must_use]
    pub const fn llvm_sha256(&self) -> &[u8; 32] {
        &self.llvm_sha256
    }

    /// Complete generic compiler-handoff identity.
    #[must_use]
    pub const fn compiler_handoff_identity(&self) -> CompilerModuleHandoffIdentityV2 {
        self.compiler_handoff_identity
    }

    /// Closed three-entry/three-descriptor manifest identity.
    #[must_use]
    pub const fn manifest_identity(&self) -> CompilerModuleSymbolManifestIdentityV1 {
        self.manifest_identity
    }

    /// Exact generic compiler handoff.
    #[must_use]
    pub const fn compiler_handoff(&self) -> &CompilerModuleHandoffV2 {
        &self.compiler_handoff
    }

    /// This lane uses the bounded direct-LLVM route because the reusable typed
    /// general-GEMM contract has FP32 C rather than this module's BF16 C.
    #[must_use]
    pub const fn uses_typed_handoff_v2_source(&self) -> bool {
        false
    }

    /// The machine classifier retains geometry and schedule, not duplicate
    /// graph profile identity where multiple operations have the same shape.
    #[must_use]
    pub const fn classifier_distinguishes_duplicate_profiles(&self) -> bool {
        false
    }

    /// Direct LLVM is structurally pinned but does not authenticate compiler origin.
    #[must_use]
    pub const fn authenticates_compiler_origin(&self) -> bool {
        false
    }

    /// Source structure is not numerical or operator-refinement evidence.
    #[must_use]
    pub const fn proves_operator_or_numerical_refinement(&self) -> bool {
        false
    }

    /// This compiler slice does not close Ferric's generated-plan join.
    #[must_use]
    pub const fn has_ferric_plan_identity_join(&self) -> bool {
        false
    }

    /// This compiler slice does not close the kernel schedule catalog.
    #[must_use]
    pub const fn has_kernel_schedule_catalog_join(&self) -> bool {
        false
    }

    /// Preparation grants no artifact, load, or launch authority.
    #[must_use]
    pub const fn grants_launch_authority(&self) -> bool {
        false
    }
}

/// Constructs the finite catalog, KIR family, pinned LLVM, and compiler handoff.
///
/// # Errors
///
/// Returns an error if source labels, profile construction, KIR construction,
/// the compiler FFI boundary, symbol manifest, or compiler handoff is invalid.
pub fn prepare_qwen3_gemm_kernel_v1(
    bindings: Qwen3GemmSourceBindingsV1,
) -> Result<PreparedQwen3GemmKernelV1, PrepareQwen3GemmKernelErrorV1> {
    validate_source_bindings(bindings)?;
    let catalog =
        Qwen3GemmProfileCatalogV1::canonical().map_err(PrepareQwen3GemmKernelErrorV1::Catalog)?;
    let token_embedding_catalog = Qwen3TokenEmbeddingProfileCatalogV1::canonical()
        .map_err(PrepareQwen3GemmKernelErrorV1::TokenEmbeddingCatalog)?;
    let mut kir_identities = Vec::with_capacity(QWEN3_GEMM_PROFILE_COUNT_V1 * 32);
    for profile in catalog.profiles() {
        let kir = qwen3_gemm_kernel_ir_v1(*profile);
        if kir.profile_identity() != profile.identity()
            || kir.dimensions() != profile.dimensions()
            || kir.schedule() != profile.schedule()
            || kir.arguments()[2].access != Qwen3GemmArgumentAccessV1::ReadWrite
        {
            return Err(PrepareQwen3GemmKernelErrorV1::KernelIr);
        }
        kir_identities.extend_from_slice(kir.identity());
    }
    let mut embedding_kir_identities =
        Vec::with_capacity(QWEN3_TOKEN_EMBEDDING_PROFILE_COUNT_V1 * 32);
    for profile in token_embedding_catalog.profiles() {
        let kir = qwen3_token_embedding_kernel_ir_v1(*profile);
        if kir.profile_identity() != profile.identity()
            || kir.shape() != profile.shape()
            || kir.arguments()[0].role != Qwen3TokenEmbeddingArgumentRoleV1::TokenIds
            || kir.arguments()[1].role != Qwen3TokenEmbeddingArgumentRoleV1::ExpectedWeight
            || kir.arguments()[2].role != Qwen3TokenEmbeddingArgumentRoleV1::Output
        {
            return Err(PrepareQwen3GemmKernelErrorV1::KernelIr);
        }
        embedding_kir_identities.extend_from_slice(kir.identity());
    }
    let llvm = canonical_qwen3_gemm_llvm();
    validate_canonical_llvm(&llvm)?;
    let llvm_sha256: [u8; 32] = Sha256::digest(llvm.as_bytes()).into();
    let mut source_preimage = Vec::with_capacity(
        32 * (7 + QWEN3_GEMM_PROFILE_COUNT_V1 + QWEN3_TOKEN_EMBEDDING_PROFILE_COUNT_V1),
    );
    source_preimage.extend_from_slice(&bindings.source);
    source_preimage.extend_from_slice(&bindings.kernel_ir);
    source_preimage.extend_from_slice(&bindings.schedule);
    source_preimage.extend_from_slice(&bindings.target_plan);
    source_preimage.extend_from_slice(catalog.identity.as_bytes());
    source_preimage.extend_from_slice(&kir_identities);
    source_preimage.extend_from_slice(token_embedding_catalog.identity.as_bytes());
    source_preimage.extend_from_slice(&embedding_kir_identities);
    source_preimage.extend_from_slice(&llvm_sha256);
    let source_binding_identity = hash(SOURCE_BINDING_DOMAIN, &source_preimage);
    let target = exact_target();
    let envelope =
        CompilerFfiEnvelopeV1::for_module_without_device_ffi(target, CodeObjectVersion::V6)
            .map_err(PrepareQwen3GemmKernelErrorV1::CompilerEnvelope)?;
    let manifest = CompilerModuleSymbolManifestV1::new([
        (
            CompilerModuleSymbolRoleV1::KernelEntry,
            QWEN3_GEMM_REFERENCE_KERNEL_SYMBOL_V1,
        ),
        (
            CompilerModuleSymbolRoleV1::KernelEntry,
            QWEN3_GEMM_VECTORIZED_KERNEL_SYMBOL_V1,
        ),
        (
            CompilerModuleSymbolRoleV1::KernelEntry,
            QWEN3_TOKEN_EMBEDDING_KERNEL_SYMBOL_V1,
        ),
        (
            CompilerModuleSymbolRoleV1::KernelDescriptor,
            QWEN3_GEMM_REFERENCE_DESCRIPTOR_SYMBOL_V1,
        ),
        (
            CompilerModuleSymbolRoleV1::KernelDescriptor,
            QWEN3_GEMM_VECTORIZED_DESCRIPTOR_SYMBOL_V1,
        ),
        (
            CompilerModuleSymbolRoleV1::KernelDescriptor,
            QWEN3_TOKEN_EMBEDDING_DESCRIPTOR_SYMBOL_V1,
        ),
    ])
    .map_err(PrepareQwen3GemmKernelErrorV1::SymbolManifest)?;
    let manifest_identity = manifest.identity();
    let compiler_handoff = CompilerModuleHandoffV2::new(
        CompilerModuleKindV1::LlvmTextIr,
        target,
        CodeObjectVersion::V6,
        envelope,
        manifest,
        llvm.as_bytes(),
    )
    .map_err(PrepareQwen3GemmKernelErrorV1::CompilerHandoff)?;
    let compiler_handoff_identity = compiler_handoff.identity();
    Ok(PreparedQwen3GemmKernelV1 {
        catalog,
        token_embedding_catalog,
        source_binding_identity,
        llvm_sha256,
        compiler_handoff_identity,
        manifest_identity,
        compiler_handoff,
    })
}

fn validate_source_bindings(
    bindings: Qwen3GemmSourceBindingsV1,
) -> Result<(), PrepareQwen3GemmKernelErrorV1> {
    let identities = [
        bindings.source,
        bindings.kernel_ir,
        bindings.schedule,
        bindings.target_plan,
    ];
    for (index, identity) in identities.iter().enumerate() {
        if identity == &[0; 32] || identities[index + 1..].contains(identity) {
            return Err(PrepareQwen3GemmKernelErrorV1::SourceBindings);
        }
    }
    Ok(())
}

fn canonical_qwen3_gemm_llvm() -> String {
    let mut output = String::with_capacity(64 * 1024);
    writeln!(output, "target triple = \"amdgcn-amd-amdhsa\"")
        .expect("writing to a String cannot fail");
    writeln!(
        output,
        "target datalayout = \"{EXTERNAL_DEVICE_LIBRARY_GFX942_DATA_LAYOUT_V1}\"\n"
    )
    .expect("writing to a String cannot fail");
    output.push_str(
        r"declare i32 @llvm.amdgcn.workitem.id.x() #1
declare i32 @llvm.amdgcn.workgroup.id.x() #1
declare i32 @llvm.amdgcn.workgroup.id.y() #1
declare void @llvm.trap()

",
    );
    emit_gemm_kernel(
        &mut output,
        QWEN3_GEMM_REFERENCE_KERNEL_SYMBOL_V1,
        Qwen3GemmScheduleV1::ReferenceWave64V1,
    );
    output.push('\n');
    emit_gemm_kernel(
        &mut output,
        QWEN3_GEMM_VECTORIZED_KERNEL_SYMBOL_V1,
        Qwen3GemmScheduleV1::VectorizedA4Wave64V1,
    );
    output.push('\n');
    emit_token_embedding_kernel(&mut output);
    output.push_str(
        r#"
attributes #0 = { nounwind "amdgpu-flat-work-group-size"="64,64" "target-cpu"="gfx942" "target-features"="-wavefrontsize32,+wavefrontsize64,-xnack" "denormal-fp-math-f32"="ieee,ieee" "unsafe-fp-math"="false" "no-infs-fp-math"="false" "no-nans-fp-math"="false" "no-signed-zeros-fp-math"="false" "approx-func-fp-math"="false" "fp-contract"="off" }
attributes #1 = { nounwind readnone speculatable willreturn }

!0 = !{i32 64, i32 1, i32 1}
!1 = !{!"read_only", !"none", !"read_only", !"none", !"read_write", !"none", !"none", !"none", !"none", !"none"}
!2 = !{!"ushort*", !"ulong", !"ushort*", !"ulong", !"ushort*", !"ulong", !"uint", !"uint", !"uint", !"uint"}
!3 = !{!"const restrict", !"", !"const restrict", !"", !"restrict", !"", !"", !"", !"", !""}
!4 = !{!"read_only", !"none", !"read_only", !"none", !"write_only", !"none", !"none", !"none", !"none"}
!5 = !{!"uint*", !"ulong", !"ushort*", !"ulong", !"ushort*", !"ulong", !"uint", !"uint", !"uint"}
!6 = !{!"const restrict", !"", !"const restrict", !"", !"restrict", !"", !"", !"", !""}
"#,
    );
    output
}

fn emit_gemm_kernel(output: &mut String, symbol: &str, schedule: Qwen3GemmScheduleV1) {
    writeln!(
        output,
        "define amdgpu_kernel void @{symbol}(ptr addrspace(1) noalias nocapture readonly align 2 %a.data, i64 %a.len, ptr addrspace(1) noalias nocapture readonly align 2 %b.data, i64 %b.len, ptr addrspace(1) noalias nocapture align 2 %c.data, i64 %c.len, i32 %m, i32 %n, i32 %k, i32 %beta.bits) #0 !reqd_work_group_size !0 !kernel_arg_access_qual !1 !kernel_arg_type !2 !kernel_arg_base_type !2 !kernel_arg_type_qual !3 {{\nentry:"
    )
    .expect("writing to a String cannot fail");
    emit_machine_classifier(output, schedule);
    output.push_str(
        r"  %local.i32 = call i32 @llvm.amdgcn.workitem.id.x()
  %group.x.i32 = call i32 @llvm.amdgcn.workgroup.id.x()
  %group.y.i32 = call i32 @llvm.amdgcn.workgroup.id.y()
  %local = zext i32 %local.i32 to i64
  %local.ok = icmp ult i64 %local, 64
  %entry.ok = and i1 %known.profile, %local.ok
  br i1 %entry.ok, label %shape.selected, label %trap

shape.selected:
  %m64 = zext i32 %m to i64
  %n64 = zext i32 %n to i64
  %k64 = zext i32 %k to i64
  %a.expected = mul nuw i64 %m64, %k64
  %b.expected = mul nuw i64 %k64, %n64
  %c.expected = mul nuw i64 %m64, %n64
  %a.length.ok = icmp eq i64 %a.len, %a.expected
  %b.length.ok = icmp eq i64 %b.len, %b.expected
  %c.length.ok = icmp eq i64 %c.len, %c.expected
  %ab.lengths.ok = and i1 %a.length.ok, %b.length.ok
  %lengths.ok = and i1 %ab.lengths.ok, %c.length.ok
  br i1 %lengths.ok, label %tile.indices, label %trap

tile.indices:
  %group.x = zext i32 %group.x.i32 to i64
  %group.y = zext i32 %group.y.i32 to i64
  %n.plus.15 = add nuw i64 %n64, 15
  %m.plus.15 = add nuw i64 %m64, 15
  %blocks.x = lshr i64 %n.plus.15, 4
  %blocks.y = lshr i64 %m.plus.15, 4
  %group.x.ok = icmp ult i64 %group.x, %blocks.x
  %group.y.ok = icmp ult i64 %group.y, %blocks.y
  %groups.ok = and i1 %group.x.ok, %group.y.ok
  %tile.column.base = shl nuw i64 %group.x, 4
  %lane.column = and i64 %local, 15
  %column = add nuw i64 %tile.column.base, %lane.column
  %tile.row.base = shl nuw i64 %group.y, 4
  %lane.row = lshr i64 %local, 4
  br i1 %groups.ok, label %row.loop, label %trap

row.loop:
  %row.offset = phi i64 [ 0, %tile.indices ], [ %row.offset.next, %row.continue ]
  %row.in.tile = add nuw i64 %lane.row, %row.offset
  %row = add nuw i64 %tile.row.base, %row.in.tile
  %row.active = icmp ult i64 %row, %m64
  %column.active = icmp ult i64 %column, %n64
  %coordinate.active = and i1 %row.active, %column.active
  br i1 %coordinate.active, label %reduce.entry, label %row.continue

reduce.entry:
  br label %reduce.cond

reduce.cond:
",
    );
    match schedule {
        Qwen3GemmScheduleV1::ReferenceWave64V1 => emit_reference_reduction(output),
        Qwen3GemmScheduleV1::VectorizedA4Wave64V1 => emit_vectorized_reduction(output),
    }
    output.push_str(
        r"
reduce.done:
  br i1 %beta.one, label %residual.load, label %narrow

residual.load:
  %c.row.base = mul nuw i64 %row, %n64
  %c.index = add nuw i64 %c.row.base, %column
  %c.read.ptr = getelementptr inbounds i16, ptr addrspace(1) %c.data, i64 %c.index
  %c.residual.bf16 = load i16, ptr addrspace(1) %c.read.ptr, align 2
  %c.residual.wide = zext i16 %c.residual.bf16 to i32
  %c.residual.bits = shl nuw i32 %c.residual.wide, 16
  %c.residual = bitcast i32 %c.residual.bits to float
  %with.residual = fadd float %accumulator, %c.residual
  br label %narrow

narrow:
  %result = phi float [ %accumulator, %reduce.done ], [ %with.residual, %residual.load ]
  %result.bits = bitcast float %result to i32
  %result.lsb.shift = lshr i32 %result.bits, 16
  %result.lsb = and i32 %result.lsb.shift, 1
  %result.bias = add nuw nsw i32 32767, %result.lsb
  %result.rounded = add i32 %result.bits, %result.bias
  %result.bf16.wide = lshr i32 %result.rounded, 16
  %result.bf16 = trunc i32 %result.bf16.wide to i16
  %c.write.row.base = mul nuw i64 %row, %n64
  %c.write.index = add nuw i64 %c.write.row.base, %column
  %c.write.ptr = getelementptr inbounds i16, ptr addrspace(1) %c.data, i64 %c.write.index
  store i16 %result.bf16, ptr addrspace(1) %c.write.ptr, align 2
  br label %row.continue

row.continue:
  %row.offset.next = add nuw i64 %row.offset, 4
  %row.offset.more = icmp ult i64 %row.offset.next, 16
  br i1 %row.offset.more, label %row.loop, label %return

return:
  ret void

trap:
  call void @llvm.trap()
  ret void
}
",
    );
}

fn emit_machine_classifier(output: &mut String, schedule: Qwen3GemmScheduleV1) {
    let (target_rows, draft_rows): (&[u32], &[u32]) = match schedule {
        Qwen3GemmScheduleV1::ReferenceWave64V1 => (&[1, 5, 8, 9], &[1, 4, 8]),
        Qwen3GemmScheduleV1::VectorizedA4Wave64V1 => (
            &[17, 32, 40, 128, 512, 1_024, 2_048],
            &[16, 32, 128, 512, 1_024, 2_048],
        ),
    };
    let target_rows = emit_allowed_rows(output, "target", "%m", target_rows);
    let draft_rows = emit_allowed_rows(output, "draft", "%m", draft_rows);
    output.push_str(
        r"  %beta.zero = icmp eq i32 %beta.bits, 0
  %beta.one = icmp eq i32 %beta.bits, 1065353216
  %n.t.hidden = icmp eq i32 %n, 4096
  %n.t.kv = icmp eq i32 %n, 1024
  %n.t.intermediate = icmp eq i32 %n, 12288
  %n.vocabulary = icmp eq i32 %n, 151936
  %k.t.hidden = icmp eq i32 %k, 4096
  %k.t.intermediate = icmp eq i32 %k, 12288
  %target.q.nk = and i1 %n.t.hidden, %k.t.hidden
  %target.q = and i1 %target.q.nk, %beta.zero
  %target.kv.nk = and i1 %n.t.kv, %k.t.hidden
  %target.kv = and i1 %target.kv.nk, %beta.zero
  %target.o = and i1 %target.q.nk, %beta.one
  %target.mlp.nk = and i1 %n.t.intermediate, %k.t.hidden
  %target.mlp = and i1 %target.mlp.nk, %beta.zero
  %target.down.nk = and i1 %n.t.hidden, %k.t.intermediate
  %target.down = and i1 %target.down.nk, %beta.one
  %target.logits.nk = and i1 %n.vocabulary, %k.t.hidden
  %target.logits = and i1 %target.logits.nk, %beta.zero
  %target.shape.0 = or i1 %target.q, %target.kv
  %target.shape.1 = or i1 %target.o, %target.mlp
  %target.shape.2 = or i1 %target.down, %target.logits
  %target.shape.3 = or i1 %target.shape.0, %target.shape.1
  %target.shape = or i1 %target.shape.3, %target.shape.2
  %n.d.query = icmp eq i32 %n, 2048
  %n.d.hidden = icmp eq i32 %n, 1024
  %n.d.intermediate = icmp eq i32 %n, 3072
  %k.d.hidden = icmp eq i32 %k, 1024
  %k.d.query = icmp eq i32 %k, 2048
  %k.d.intermediate = icmp eq i32 %k, 3072
  %draft.q.nk = and i1 %n.d.query, %k.d.hidden
  %draft.q = and i1 %draft.q.nk, %beta.zero
  %draft.kv.nk = and i1 %n.d.hidden, %k.d.hidden
  %draft.kv = and i1 %draft.kv.nk, %beta.zero
  %draft.o.nk = and i1 %n.d.hidden, %k.d.query
  %draft.o = and i1 %draft.o.nk, %beta.one
  %draft.mlp.nk = and i1 %n.d.intermediate, %k.d.hidden
  %draft.mlp = and i1 %draft.mlp.nk, %beta.zero
  %draft.down.nk = and i1 %n.d.hidden, %k.d.intermediate
  %draft.down = and i1 %draft.down.nk, %beta.one
  %draft.logits.nk = and i1 %n.vocabulary, %k.d.hidden
  %draft.logits = and i1 %draft.logits.nk, %beta.zero
  %draft.shape.0 = or i1 %draft.q, %draft.kv
  %draft.shape.1 = or i1 %draft.o, %draft.mlp
  %draft.shape.2 = or i1 %draft.down, %draft.logits
  %draft.shape.3 = or i1 %draft.shape.0, %draft.shape.1
  %draft.shape = or i1 %draft.shape.3, %draft.shape.2
",
    );
    writeln!(
        output,
        "  %target.profile = and i1 {target_rows}, %target.shape\n  %draft.profile = and i1 {draft_rows}, %draft.shape\n  %known.profile = or i1 %target.profile, %draft.profile"
    )
    .expect("writing to a String cannot fail");
}

fn emit_allowed_rows(output: &mut String, prefix: &str, dimension: &str, rows: &[u32]) -> String {
    for (index, row) in rows.iter().enumerate() {
        writeln!(
            output,
            "  %{prefix}.row.{index} = icmp eq i32 {dimension}, {row}"
        )
        .expect("writing to a String cannot fail");
    }
    let mut current = format!("%{prefix}.row.0");
    for index in 1..rows.len() {
        let next = format!("%{prefix}.row.any.{index}");
        writeln!(output, "  {next} = or i1 {current}, %{prefix}.row.{index}")
            .expect("writing to a String cannot fail");
        current = next;
    }
    current
}

fn emit_token_embedding_kernel(output: &mut String) {
    let symbol = QWEN3_TOKEN_EMBEDDING_KERNEL_SYMBOL_V1;
    writeln!(
        output,
        "define amdgpu_kernel void @{symbol}(ptr addrspace(1) noalias nocapture readonly align 4 %tokens.data, i64 %tokens.len, ptr addrspace(1) noalias nocapture readonly align 2 %weight.data, i64 %weight.len, ptr addrspace(1) noalias nocapture writeonly align 2 %output.data, i64 %output.len, i32 %rows, i32 %hidden, i32 %vocabulary) #0 !reqd_work_group_size !0 !kernel_arg_access_qual !4 !kernel_arg_type !5 !kernel_arg_base_type !5 !kernel_arg_type_qual !6 {{\nentry:"
    )
    .expect("writing to a String cannot fail");
    let target_rows = emit_allowed_rows(
        output,
        "embedding.target",
        "%rows",
        &[1, 5, 8, 9, 17, 32, 40, 128, 512, 1_024, 2_048],
    );
    let draft_rows = emit_allowed_rows(
        output,
        "embedding.draft",
        "%rows",
        &[1, 4, 8, 16, 32, 128, 512, 1_024, 2_048],
    );
    writeln!(
        output,
        "  %embedding.target.hidden = icmp eq i32 %hidden, 4096\n  %embedding.draft.hidden = icmp eq i32 %hidden, 1024\n  %embedding.target.profile = and i1 {target_rows}, %embedding.target.hidden\n  %embedding.draft.profile = and i1 {draft_rows}, %embedding.draft.hidden\n  %embedding.shape = or i1 %embedding.target.profile, %embedding.draft.profile"
    )
    .expect("writing to a String cannot fail");
    output.push_str(
        r"  %embedding.vocabulary.ok = icmp eq i32 %vocabulary, 151936
  %embedding.known.profile = and i1 %embedding.shape, %embedding.vocabulary.ok
  %rows64 = zext i32 %rows to i64
  %hidden64 = zext i32 %hidden to i64
  %vocabulary64 = zext i32 %vocabulary to i64
  %weight.expected = mul nuw i64 %vocabulary64, %hidden64
  %output.expected = mul nuw i64 %rows64, %hidden64
  %tokens.length.ok = icmp eq i64 %tokens.len, %rows64
  %weight.length.ok = icmp eq i64 %weight.len, %weight.expected
  %output.length.ok = icmp eq i64 %output.len, %output.expected
  %input.lengths.ok = and i1 %tokens.length.ok, %weight.length.ok
  %lengths.ok = and i1 %input.lengths.ok, %output.length.ok
  %entry.ok = and i1 %embedding.known.profile, %lengths.ok
  br i1 %entry.ok, label %coordinates, label %trap

coordinates:
  %local.i32 = call i32 @llvm.amdgcn.workitem.id.x()
  %group.i32 = call i32 @llvm.amdgcn.workgroup.id.x()
  %local = zext i32 %local.i32 to i64
  %group = zext i32 %group.i32 to i64
  %group.base = mul nuw i64 %group, 64
  %output.index = add nuw i64 %group.base, %local
  %output.active = icmp ult i64 %output.index, %output.expected
  br i1 %output.active, label %token.load, label %return

token.load:
  %row = udiv i64 %output.index, %hidden64
  %column = urem i64 %output.index, %hidden64
  %token.ptr = getelementptr inbounds i32, ptr addrspace(1) %tokens.data, i64 %row
  %token = load i32, ptr addrspace(1) %token.ptr, align 4
  %token.in.vocabulary = icmp ult i32 %token, %vocabulary
  br i1 %token.in.vocabulary, label %weight.address, label %trap

weight.address:
  %token64 = zext i32 %token to i64
  %weight.row.base = mul nuw i64 %token64, %hidden64
  %weight.index = add nuw i64 %weight.row.base, %column
  %weight.ptr = getelementptr inbounds i16, ptr addrspace(1) %weight.data, i64 %weight.index
  %embedding.bits = load i16, ptr addrspace(1) %weight.ptr, align 2
  %output.ptr = getelementptr inbounds i16, ptr addrspace(1) %output.data, i64 %output.index
  store i16 %embedding.bits, ptr addrspace(1) %output.ptr, align 2
  br label %return

return:
  ret void

trap:
  call void @llvm.trap()
  ret void
}
",
    );
}

fn emit_reference_reduction(output: &mut String) {
    output.push_str(
        r"  %reduction = phi i64 [ 0, %reduce.entry ], [ %reduction.next, %reduce.body ]
  %accumulator = phi float [ 0.000000e+00, %reduce.entry ], [ %accumulator.next, %reduce.body ]
  %reduction.more = icmp ult i64 %reduction, %k64
  br i1 %reduction.more, label %reduce.body, label %reduce.done

reduce.body:
  %a.row.base = mul nuw i64 %row, %k64
  %a.index = add nuw i64 %a.row.base, %reduction
  %b.row.base = mul nuw i64 %reduction, %n64
  %b.index = add nuw i64 %b.row.base, %column
  %a.ptr = getelementptr inbounds i16, ptr addrspace(1) %a.data, i64 %a.index
  %b.ptr = getelementptr inbounds i16, ptr addrspace(1) %b.data, i64 %b.index
  %a.bf16 = load i16, ptr addrspace(1) %a.ptr, align 2
  %b.bf16 = load i16, ptr addrspace(1) %b.ptr, align 2
  %a.wide = zext i16 %a.bf16 to i32
  %b.wide = zext i16 %b.bf16 to i32
  %a.bits = shl nuw i32 %a.wide, 16
  %b.bits = shl nuw i32 %b.wide, 16
  %a.value = bitcast i32 %a.bits to float
  %b.value = bitcast i32 %b.bits to float
  %product = fmul float %a.value, %b.value
  %accumulator.next = fadd float %accumulator, %product
  %reduction.next = add nuw i64 %reduction, 1
  br label %reduce.cond
",
    );
}

fn emit_vectorized_reduction(output: &mut String) {
    output.push_str(
        r"  %reduction = phi i64 [ 0, %reduce.entry ], [ %reduction.next, %reduce.body ]
  %accumulator = phi float [ 0.000000e+00, %reduce.entry ], [ %accumulator.4, %reduce.body ]
  %reduction.more = icmp ult i64 %reduction, %k64
  br i1 %reduction.more, label %reduce.body, label %reduce.done

reduce.body:
  %a.row.base = mul nuw i64 %row, %k64
  %a.index = add nuw i64 %a.row.base, %reduction
  %a.ptr = getelementptr inbounds i16, ptr addrspace(1) %a.data, i64 %a.index
  %a.vector = load <4 x i16>, ptr addrspace(1) %a.ptr, align 2
",
    );
    for index in 0..4 {
        writeln!(
            output,
            "  %a.{index}.bf16 = extractelement <4 x i16> %a.vector, i32 {index}\n  %a.{index}.wide = zext i16 %a.{index}.bf16 to i32\n  %a.{index}.bits = shl nuw i32 %a.{index}.wide, 16\n  %a.{index}.value = bitcast i32 %a.{index}.bits to float\n  %reduction.{index} = add nuw i64 %reduction, {index}\n  %b.{index}.row.base = mul nuw i64 %reduction.{index}, %n64\n  %b.{index}.index = add nuw i64 %b.{index}.row.base, %column\n  %b.{index}.ptr = getelementptr inbounds i16, ptr addrspace(1) %b.data, i64 %b.{index}.index\n  %b.{index}.bf16 = load i16, ptr addrspace(1) %b.{index}.ptr, align 2\n  %b.{index}.wide = zext i16 %b.{index}.bf16 to i32\n  %b.{index}.bits = shl nuw i32 %b.{index}.wide, 16\n  %b.{index}.value = bitcast i32 %b.{index}.bits to float\n  %product.{index} = fmul float %a.{index}.value, %b.{index}.value"
        )
        .expect("writing to a String cannot fail");
        let previous = if index == 0 {
            "%accumulator".to_owned()
        } else {
            format!("%accumulator.{index}")
        };
        writeln!(
            output,
            "  %accumulator.{} = fadd float {previous}, %product.{index}",
            index + 1
        )
        .expect("writing to a String cannot fail");
    }
    output.push_str(
        r"  %reduction.next = add nuw i64 %reduction, 4
  br label %reduce.cond
",
    );
}

fn validate_canonical_llvm(module: &str) -> Result<(), PrepareQwen3GemmKernelErrorV1> {
    let module_sha256: [u8; 32] = Sha256::digest(module.as_bytes()).into();
    let exact = module.len() == QWEN3_GEMM_LLVM_BYTES_V1
        && module_sha256 == QWEN3_GEMM_LLVM_SHA256_V1
        && module.matches("define amdgpu_kernel").count() == 3
        && module
            .matches(QWEN3_GEMM_REFERENCE_KERNEL_SYMBOL_V1)
            .count()
            == 1
        && module
            .matches(QWEN3_GEMM_VECTORIZED_KERNEL_SYMBOL_V1)
            .count()
            == 1
        && module
            .matches(QWEN3_TOKEN_EMBEDDING_KERNEL_SYMBOL_V1)
            .count()
            == 1
        && module.matches("call void @llvm.trap()").count() == 3
        && module.matches("store i16").count() == 3
        && module.matches("load <4 x i16>").count() == 1
        && module.contains("%draft.o.nk = and i1 %n.d.hidden, %k.d.query")
        && module.contains("%k.d.query = icmp eq i32 %k, 2048")
        && module.contains("%n.d.hidden = icmp eq i32 %n, 1024")
        && module.contains("%target.o = and i1 %target.q.nk, %beta.one")
        && module.contains("%a.expected = mul nuw i64 %m64, %k64")
        && module.contains("%b.expected = mul nuw i64 %k64, %n64")
        && module.contains("%c.expected = mul nuw i64 %m64, %n64")
        && module.contains("%result.bf16 = trunc i32 %result.bf16.wide to i16")
        && module.contains("%embedding.vocabulary.ok = icmp eq i32 %vocabulary, 151936")
        && module.contains("%embedding.target.hidden = icmp eq i32 %hidden, 4096")
        && module.contains("%embedding.draft.hidden = icmp eq i32 %hidden, 1024")
        && module.contains("%token.in.vocabulary = icmp ult i32 %token, %vocabulary")
        && module.contains("br i1 %token.in.vocabulary, label %weight.address, label %trap")
        && module.contains("store i16 %embedding.bits")
        && module.contains("\"fp-contract\"=\"off\"")
        && !module.contains("store float")
        && !module.contains("atomic")
        && !module.contains("volatile")
        && !module.contains(" fast ")
        && !module.contains("contract ")
        && !module.contains("reassoc ")
        && !module.contains("llvm.fma")
        && !module.contains("llvm.amdgcn.mfma")
        && !module.contains("comgr")
        && !module.contains("COMGR");
    if !exact {
        return Err(PrepareQwen3GemmKernelErrorV1::CompilerModule);
    }
    Ok(())
}

/// Linear exact compiler handoff awaiting Worker V2 execution.
pub struct InertQwen3GemmWorkerRequestV1 {
    prepared: PreparedQwen3GemmKernelV1,
}

impl fmt::Debug for InertQwen3GemmWorkerRequestV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InertQwen3GemmWorkerRequestV1")
            .field("catalog", &self.prepared.catalog.identity)
            .field(
                "token_embedding_catalog",
                &self.prepared.token_embedding_catalog.identity,
            )
            .field("source_binding", &self.prepared.source_binding_identity)
            .field("handoff", &self.prepared.compiler_handoff_identity)
            .finish_non_exhaustive()
    }
}

impl InertQwen3GemmWorkerRequestV1 {
    /// Complete finite catalog retained by the request owner.
    #[must_use]
    pub const fn catalog(&self) -> &Qwen3GemmProfileCatalogV1 {
        &self.prepared.catalog
    }

    /// Complete finite token-embedding catalog retained by the request.
    #[must_use]
    pub const fn token_embedding_catalog(&self) -> &Qwen3TokenEmbeddingProfileCatalogV1 {
        &self.prepared.token_embedding_catalog
    }

    /// Exact compiler handoff for transaction publication.
    #[must_use]
    pub const fn compiler_handoff(&self) -> &CompilerModuleHandoffV2 {
        &self.prepared.compiler_handoff
    }

    /// Ferric-domain source binding retained by the compiler handoff.
    #[must_use]
    pub const fn source_binding_identity(&self) -> &[u8; 32] {
        &self.prepared.source_binding_identity
    }

    /// A request does not establish Worker execution or artifact existence.
    #[must_use]
    pub const fn authenticates_worker_execution(&self) -> bool {
        false
    }

    /// A compiler request grants no artifact, load, or launch authority.
    #[must_use]
    pub const fn grants_launch_authority(&self) -> bool {
        false
    }
}

/// Consumes a prepared owner into the exact Worker V2 request stage.
#[must_use]
pub const fn lower_qwen3_gemm_kernel_v1(
    prepared: PreparedQwen3GemmKernelV1,
) -> InertQwen3GemmWorkerRequestV1 {
    InertQwen3GemmWorkerRequestV1 { prepared }
}

/// Failure while executing the exact source through Worker V2.
#[derive(Debug)]
pub enum ExecuteQwen3GemmWorkerErrorV1 {
    /// Consumed transaction bytes differed from the prepared handoff.
    HandoffSubstitution,
    /// A fixed link option could not be represented.
    FixedLinkOption,
    /// The fixed HSACO output ceiling could not be represented.
    OutputConstraint(WorkerProtocolError),
    /// Reproducible bootstrap and replay failed.
    FirstBuild(FirstBuildWorkerV2Error),
}

impl fmt::Display for ExecuteQwen3GemmWorkerErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Qwen3 GEMM Worker V2 execution failed: {self:?}")
    }
}

impl std::error::Error for ExecuteQwen3GemmWorkerErrorV1 {}

/// Linear Worker V2 bootstrap/replay evidence awaiting inspection.
pub struct InertQwen3GemmWorkerEvidenceV1 {
    prepared: PreparedQwen3GemmKernelV1,
    transaction_handoff: CompilerModuleHandoffIdentityV1,
    worker: InertFirstBuildWorkerV2EvidenceV1,
}

impl fmt::Debug for InertQwen3GemmWorkerEvidenceV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InertQwen3GemmWorkerEvidenceV1")
            .field("source_binding", &self.prepared.source_binding_identity)
            .field("transaction_handoff", &self.transaction_handoff)
            .field("worker", &self.worker.identity())
            .finish_non_exhaustive()
    }
}

impl InertQwen3GemmWorkerEvidenceV1 {
    /// Worker evidence remains inert until strict structural inspection.
    #[must_use]
    pub const fn grants_artifact_authority(&self) -> bool {
        false
    }

    /// Worker output does not prove the declared numerical policy.
    #[must_use]
    pub const fn proves_numerical_contract(&self) -> bool {
        false
    }

    /// Worker output does not establish operator or race refinement.
    #[must_use]
    pub const fn proves_operator_or_race_refinement(&self) -> bool {
        false
    }
}

/// Executes the exact transaction handoff through Worker V2 bootstrap/replay.
///
/// # Errors
///
/// Returns an error for a substituted handoff, invalid fixed link options or
/// output constraints, or a Worker V2 execution failure.
pub fn execute_qwen3_gemm_worker_v2_v1(
    request: InertQwen3GemmWorkerRequestV1,
    consumed: ConsumedCompilerModuleHandoffV1,
    worker: &PinnedWorkerV1,
    limits: WorkerExecutionLimitsV1,
) -> Result<InertQwen3GemmWorkerEvidenceV1, ExecuteQwen3GemmWorkerErrorV1> {
    let InertQwen3GemmWorkerRequestV1 { prepared } = request;
    if consumed.bytes() != prepared.compiler_handoff.canonical_bytes() {
        return Err(ExecuteQwen3GemmWorkerErrorV1::HandoffSubstitution);
    }
    let transaction_handoff = consumed.identity();
    let worker_evidence = execute_reproducible_first_build_worker_v2(
        consumed,
        worker,
        Vec::new(),
        fixed_link_options()?,
        WorkerOutputConstraintsV1::new(MAX_HSACO_BYTES as u64)
            .map_err(ExecuteQwen3GemmWorkerErrorV1::OutputConstraint)?,
        limits,
    )
    .map_err(ExecuteQwen3GemmWorkerErrorV1::FirstBuild)?;
    Ok(InertQwen3GemmWorkerEvidenceV1 {
        prepared,
        transaction_handoff,
        worker: worker_evidence,
    })
}

/// Exact post-worker structural rejection.
#[derive(Debug)]
pub enum InspectQwen3GemmKernelErrorV1 {
    /// Worker request or response bytes failed canonical decoding.
    Protocol(WorkerProtocolError),
    /// Compiler, transaction, Worker, or output lineage drifted.
    SourceLineage,
    /// AMDHSA metadata or descriptor binding failed.
    Hsaco(KernelBindingError),
    /// Kernel inventory, ABI, or resource facts differed.
    KernelProfile,
    /// Strict allocation-free COV6 loader validation failed.
    Loader(PlanError),
}

impl fmt::Display for InspectQwen3GemmKernelErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Qwen3 GEMM structural inspection failed: {self:?}"
        )
    }
}

impl std::error::Error for InspectQwen3GemmKernelErrorV1 {}

/// Linear Worker output after exact ABI/resource and loader inspection.
pub struct InspectedQwen3GemmKernelV1 {
    catalog: Qwen3GemmProfileCatalogV1,
    token_embedding_catalog: Qwen3TokenEmbeddingProfileCatalogV1,
    source_binding_identity: [u8; 32],
    compiler_handoff_identity: CompilerModuleHandoffIdentityV2,
    transaction_handoff: CompilerModuleHandoffIdentityV1,
    loader_plan: LoadPlan,
    worker: InertFirstBuildWorkerV2EvidenceV1,
}

impl fmt::Debug for InspectedQwen3GemmKernelV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InspectedQwen3GemmKernelV1")
            .field("catalog", &self.catalog.identity)
            .field(
                "token_embedding_catalog",
                &self.token_embedding_catalog.identity,
            )
            .field("source_binding", &self.source_binding_identity)
            .field("compiler_handoff", &self.compiler_handoff_identity)
            .field("transaction_handoff", &self.transaction_handoff)
            .field("worker", &self.worker.identity())
            .finish_non_exhaustive()
    }
}

impl InspectedQwen3GemmKernelV1 {
    /// Complete finite catalog retained by the inspected owner.
    #[must_use]
    pub const fn catalog(&self) -> &Qwen3GemmProfileCatalogV1 {
        &self.catalog
    }

    /// Complete finite token-embedding catalog retained by the inspected owner.
    #[must_use]
    pub const fn token_embedding_catalog(&self) -> &Qwen3TokenEmbeddingProfileCatalogV1 {
        &self.token_embedding_catalog
    }

    /// Exact strict pure-Rust loader plan over the same output bytes.
    #[must_use]
    pub const fn loader_plan(&self) -> &LoadPlan {
        &self.loader_plan
    }

    /// Exact bytes retained by sealed Worker evidence.
    #[must_use]
    pub fn exact_worker_output_bytes(&self) -> &[u8] {
        self.worker.output_bytes()
    }

    /// Observed bytes are not an independently approved deployment pin.
    #[must_use]
    pub const fn has_independent_deployment_pin(&self) -> bool {
        false
    }

    /// Structural inspection does not prove source-to-machine refinement.
    #[must_use]
    pub const fn proves_machine_refinement(&self) -> bool {
        false
    }

    /// Structural inspection does not prove numerical or operator refinement.
    #[must_use]
    pub const fn proves_operator_or_numerical_refinement(&self) -> bool {
        false
    }

    /// Structural inspection does not prove single-writer or race refinement.
    #[must_use]
    pub const fn proves_race_refinement(&self) -> bool {
        false
    }

    /// Structural inspection does not authenticate weight or buffer content.
    #[must_use]
    pub const fn authenticates_content(&self) -> bool {
        false
    }

    /// Structural inspection does not authenticate allocation ownership.
    #[must_use]
    pub const fn authenticates_allocation_ownership(&self) -> bool {
        false
    }

    /// No mutable generation authority is represented by this owner.
    #[must_use]
    pub const fn authenticates_generation(&self) -> bool {
        false
    }

    /// Structural inspection does not prove hardware execution.
    #[must_use]
    pub const fn proves_hardware_execution(&self) -> bool {
        false
    }

    /// No completion observation is represented by this owner.
    #[must_use]
    pub const fn proves_completion(&self) -> bool {
        false
    }

    /// No performance measurement is represented by this owner.
    #[must_use]
    pub const fn proves_performance(&self) -> bool {
        false
    }

    /// Structural inspection grants no load or launch authority.
    #[must_use]
    pub const fn grants_launch_authority(&self) -> bool {
        false
    }

    /// Binds one exact profile to exact disjoint BF16 buffer spans.
    ///
    /// # Errors
    ///
    /// Returns an error if the profile is absent or any buffer span fails the
    /// exact checked contract.
    pub fn bind_checked_profile(
        &self,
        bucket: Qwen3GemmBucketV1,
        operation: Qwen3GemmOperationV1,
        addresses: [u64; 3],
        byte_lengths: [u64; 3],
    ) -> Result<CheckedQwen3GemmLaunchV1, BindQwen3GemmLaunchErrorV1> {
        let profile = self
            .catalog
            .profile(bucket, operation)
            .ok_or(BindQwen3GemmLaunchErrorV1::Profile)?;
        let buffers = Qwen3GemmBufferContractV1::checked(profile, addresses, byte_lengths)
            .map_err(BindQwen3GemmLaunchErrorV1::Buffers)?;
        Ok(CheckedQwen3GemmLaunchV1 { profile, buffers })
    }

    /// Binds one exact token-embedding profile to disjoint checked spans and
    /// an expected bundle-bound weight identity.
    ///
    /// # Errors
    ///
    /// Returns an error if the profile is absent, the expected identity role
    /// differs, or any address, length, alignment, overflow, or alias check fails.
    pub fn bind_checked_token_embedding_profile(
        &self,
        bucket: Qwen3GemmBucketV1,
        expected_weight_identity: Qwen3ExpectedEmbeddingWeightIdentityV1,
        addresses: [u64; 3],
        byte_lengths: [u64; 3],
    ) -> Result<CheckedQwen3TokenEmbeddingLaunchV1, BindQwen3TokenEmbeddingLaunchErrorV1> {
        let profile = self
            .token_embedding_catalog
            .profile(bucket)
            .ok_or(BindQwen3TokenEmbeddingLaunchErrorV1::Profile)?;
        let buffers = Qwen3TokenEmbeddingBufferContractV1::checked(
            profile,
            expected_weight_identity,
            addresses,
            byte_lengths,
        )
        .map_err(BindQwen3TokenEmbeddingLaunchErrorV1::Buffers)?;
        Ok(CheckedQwen3TokenEmbeddingLaunchV1 { profile, buffers })
    }
}

/// Consumes Worker evidence through transcript, HSACO, ABI/resource, and loader checks.
///
/// # Errors
///
/// Returns an error if lineage, output identity, HSACO structure, kernel ABI,
/// resource limits, or the loader profile fails closed.
pub fn inspect_qwen3_gemm_kernel_v1(
    evidence: InertQwen3GemmWorkerEvidenceV1,
) -> Result<InspectedQwen3GemmKernelV1, InspectQwen3GemmKernelErrorV1> {
    let InertQwen3GemmWorkerEvidenceV1 {
        prepared,
        transaction_handoff,
        worker,
    } = evidence;
    validate_worker_lineage(&prepared, transaction_handoff, &worker)?;
    let bytes = worker.output_bytes();
    if !worker.output_identity().matches(bytes) {
        return Err(InspectQwen3GemmKernelErrorV1::SourceLineage);
    }
    let bound =
        inspect_and_bind_kernel_descriptors(bytes).map_err(InspectQwen3GemmKernelErrorV1::Hsaco)?;
    let [reference, vectorized, token_embedding] = bound.inspection().kernels() else {
        return Err(InspectQwen3GemmKernelErrorV1::KernelProfile);
    };
    let [reference_binding, vectorized_binding, token_embedding_binding] = bound.bindings() else {
        return Err(InspectQwen3GemmKernelErrorV1::KernelProfile);
    };
    let exact = bound.inspection().code_object_version() == InspectedCodeObjectVersion::V6
        && bound.inspection().target().to_string() == QWEN3_GEMM_TARGET_V1
        && !bound.inspection().has_printf_metadata()
        && exact_kernel_profile(
            reference,
            reference_binding,
            ExactKernelProfileV1 {
                index: 0,
                symbol: QWEN3_GEMM_REFERENCE_KERNEL_SYMBOL_V1,
                descriptor: QWEN3_GEMM_REFERENCE_DESCRIPTOR_SYMBOL_V1,
                explicit_bytes: QWEN3_GEMM_EXPLICIT_KERNARG_BYTES_V1,
                total_bytes: QWEN3_GEMM_TOTAL_KERNARG_BYTES_V1,
                explicit_arguments: exact_gemm_explicit_arguments,
            },
        )
        && exact_kernel_profile(
            vectorized,
            vectorized_binding,
            ExactKernelProfileV1 {
                index: 1,
                symbol: QWEN3_GEMM_VECTORIZED_KERNEL_SYMBOL_V1,
                descriptor: QWEN3_GEMM_VECTORIZED_DESCRIPTOR_SYMBOL_V1,
                explicit_bytes: QWEN3_GEMM_EXPLICIT_KERNARG_BYTES_V1,
                total_bytes: QWEN3_GEMM_TOTAL_KERNARG_BYTES_V1,
                explicit_arguments: exact_gemm_explicit_arguments,
            },
        )
        && exact_kernel_profile(
            token_embedding,
            token_embedding_binding,
            ExactKernelProfileV1 {
                index: 2,
                symbol: QWEN3_TOKEN_EMBEDDING_KERNEL_SYMBOL_V1,
                descriptor: QWEN3_TOKEN_EMBEDDING_DESCRIPTOR_SYMBOL_V1,
                explicit_bytes: QWEN3_TOKEN_EMBEDDING_EXPLICIT_KERNARG_BYTES_V1,
                total_bytes: QWEN3_TOKEN_EMBEDDING_TOTAL_KERNARG_BYTES_V1,
                explicit_arguments: exact_token_embedding_explicit_arguments,
            },
        );
    if !exact {
        return Err(InspectQwen3GemmKernelErrorV1::KernelProfile);
    }
    let loader = fe2o3_amdhsa_loader::validate(bytes, AdmittedProfile::Gfx942XnackOffCov6)
        .map_err(InspectQwen3GemmKernelErrorV1::Loader)?;
    let loader_plan = *loader.plan();
    Ok(InspectedQwen3GemmKernelV1 {
        catalog: prepared.catalog,
        token_embedding_catalog: prepared.token_embedding_catalog,
        source_binding_identity: prepared.source_binding_identity,
        compiler_handoff_identity: prepared.compiler_handoff_identity,
        transaction_handoff,
        loader_plan,
        worker,
    })
}

fn validate_worker_lineage(
    prepared: &PreparedQwen3GemmKernelV1,
    transaction_handoff: CompilerModuleHandoffIdentityV1,
    worker: &InertFirstBuildWorkerV2EvidenceV1,
) -> Result<(), InspectQwen3GemmKernelErrorV1> {
    let expected_transaction = CompilerModuleHandoffIdentityV1::from_bytes(
        Sha256::digest(prepared.compiler_handoff.canonical_bytes()).into(),
    );
    if transaction_handoff != expected_transaction
        || worker.handoff_identity() != expected_transaction
        || worker.compiler_envelope() != prepared.compiler_handoff.envelope()
        || worker.symbol_manifest() != prepared.compiler_handoff.symbol_manifest()
        || worker.worker_measurement().llvm_build_identity()
            != fe2o3_llvm_worker_handoff::EXACT_LLVM_BUILD_IDENTITY_V1
    {
        return Err(InspectQwen3GemmKernelErrorV1::SourceLineage);
    }
    let bootstrap = InertDecodedWorkerExchangeV2::decode(
        worker.bootstrap_request_bytes(),
        worker.bootstrap().response().canonical_bytes(),
    )
    .map_err(InspectQwen3GemmKernelErrorV1::Protocol)?;
    let replay = InertDecodedWorkerExchangeV2::decode(
        worker.authorized_request_bytes(),
        worker.authorized().response().canonical_bytes(),
    )
    .map_err(InspectQwen3GemmKernelErrorV1::Protocol)?;
    for exchange in [&bootstrap, &replay] {
        let request = exchange.request();
        if request.target() != exact_target()
            || request.code_object_version() != CodeObjectVersion::V6
            || request.compiler_module().bytes() != prepared.compiler_handoff.module_bytes()
            || !request.external_providers().is_empty()
            || !request.import_symbols().is_empty()
            || !request.export_symbols().is_empty()
            || !request.final_symbols().iter().map(String::as_str).eq([
                QWEN3_GEMM_REFERENCE_KERNEL_SYMBOL_V1,
                QWEN3_GEMM_VECTORIZED_KERNEL_SYMBOL_V1,
                QWEN3_TOKEN_EMBEDDING_KERNEL_SYMBOL_V1,
                QWEN3_GEMM_REFERENCE_DESCRIPTOR_SYMBOL_V1,
                QWEN3_GEMM_VECTORIZED_DESCRIPTOR_SYMBOL_V1,
                QWEN3_TOKEN_EMBEDDING_DESCRIPTOR_SYMBOL_V1,
            ])
            || exchange.response().request_identity() != request.identity()
            || exchange.response().device_library_provider().is_some()
        {
            return Err(InspectQwen3GemmKernelErrorV1::SourceLineage);
        }
    }
    Ok(())
}

struct ExactKernelProfileV1 {
    index: usize,
    symbol: &'static str,
    descriptor: &'static str,
    explicit_bytes: u64,
    total_bytes: u64,
    explicit_arguments: fn(&[ExplicitArgument]) -> bool,
}

fn exact_kernel_profile(
    kernel: &InspectedKernel,
    binding: &KernelDescriptorBinding,
    profile: ExactKernelProfileV1,
) -> bool {
    kernel.name() == profile.symbol
        && kernel.symbol() == profile.descriptor
        && kernel.kernarg_segment_size() == profile.total_bytes
        && kernel.kernarg_segment_alignment() == QWEN3_GEMM_KERNARG_ALIGNMENT_V1
        && kernel.implicit_argument_offset() == Some(profile.explicit_bytes)
        && kernel.implicit_argument_size() == 256
        && kernel.required_workgroup_size() == Some(QWEN3_GEMM_WORKGROUP_V1)
        && kernel.max_flat_workgroup_size() == 64
        && kernel.wavefront_size() == 64
        && kernel.group_segment_fixed_size() == 0
        && kernel.private_segment_fixed_size() == 0
        && kernel.sgpr_spill_count().unwrap_or(0) == 0
        && kernel.vgpr_spill_count().unwrap_or(0) == 0
        && !kernel.uses_dynamic_stack()
        && binding.kernel_index() == profile.index
        && binding.descriptor().group_segment_fixed_size() == 0
        && binding.descriptor().private_segment_fixed_size() == 0
        && binding.descriptor().wavefront_size() == 64
        && !binding.descriptor().uses_dynamic_stack()
        && (profile.explicit_arguments)(kernel.explicit_arguments())
        && exact_hidden_arguments(kernel.hidden_arguments(), profile.explicit_bytes)
}

fn exact_gemm_explicit_arguments(arguments: &[ExplicitArgument]) -> bool {
    if arguments.len() != 10 {
        return false;
    }
    for (index, name, access) in [
        (0, "a.data", ArgumentAccess::ReadOnly),
        (2, "b.data", ArgumentAccess::ReadOnly),
        (4, "c.data", ArgumentAccess::ReadWrite),
    ] {
        if !exact_bf16_pointer_argument(&arguments[index], name, (index as u64 / 2) * 16, access) {
            return false;
        }
    }
    for (index, name) in [(1, "a.len"), (3, "b.len"), (5, "c.len")] {
        if !exact_integer_argument(
            &arguments[index],
            name,
            ((index - 1) as u64 / 2) * 16 + 8,
            8,
            is_u64_metadata_carrier,
        ) {
            return false;
        }
    }
    for (index, name, offset) in [
        (6, "m", 48),
        (7, "n", 52),
        (8, "k", 56),
        (9, "beta.bits", 60),
    ] {
        if !exact_integer_argument(&arguments[index], name, offset, 4, is_u32_metadata_carrier) {
            return false;
        }
    }
    true
}

fn exact_token_embedding_explicit_arguments(arguments: &[ExplicitArgument]) -> bool {
    arguments.len() == 9
        && exact_global_pointer_argument(
            &arguments[0],
            "tokens.data",
            0,
            4,
            ArgumentAccess::ReadOnly,
            is_u32_metadata_carrier,
        )
        && exact_global_pointer_argument(
            &arguments[2],
            "weight.data",
            16,
            2,
            ArgumentAccess::ReadOnly,
            is_bf16_metadata_carrier,
        )
        && exact_global_pointer_argument(
            &arguments[4],
            "output.data",
            32,
            2,
            ArgumentAccess::WriteOnly,
            is_bf16_metadata_carrier,
        )
        && [(1, "tokens.len"), (3, "weight.len"), (5, "output.len")]
            .into_iter()
            .all(|(index, name)| {
                exact_integer_argument(
                    &arguments[index],
                    name,
                    ((index - 1) as u64 / 2) * 16 + 8,
                    8,
                    is_u64_metadata_carrier,
                )
            })
        && [(6, "rows", 48), (7, "hidden", 52), (8, "vocabulary", 56)]
            .into_iter()
            .all(|(index, name, offset)| {
                exact_integer_argument(&arguments[index], name, offset, 4, is_u32_metadata_carrier)
            })
}

fn exact_bf16_pointer_argument(
    argument: &ExplicitArgument,
    name: &str,
    offset: u64,
    access: ArgumentAccess,
) -> bool {
    exact_global_pointer_argument(argument, name, offset, 2, access, is_bf16_metadata_carrier)
}

fn exact_global_pointer_argument(
    argument: &ExplicitArgument,
    name: &str,
    offset: u64,
    pointee_alignment: u64,
    access: ArgumentAccess,
    accepted_type: fn(ExplicitValueType) -> bool,
) -> bool {
    argument.name() == Some(name)
        && argument.offset() == offset
        && argument.size() == 8
        && argument.alignment().is_none_or(|actual| actual == 8)
        && argument
            .pointee_alignment()
            .is_none_or(|actual| actual == pointee_alignment)
        && argument.value_kind() == ExplicitValueKind::GlobalBuffer
        && argument.value_type().is_none_or(accepted_type)
        && argument.address_space() == Some(ArgumentAddressSpace::Global)
        && argument.access() == Some(access)
}

fn exact_integer_argument(
    argument: &ExplicitArgument,
    name: &str,
    offset: u64,
    size: u64,
    accepted_type: fn(ExplicitValueType) -> bool,
) -> bool {
    argument.name() == Some(name)
        && argument.offset() == offset
        && argument.size() == size
        && argument.value_kind() == ExplicitValueKind::ByValue
        && argument.value_type().is_none_or(accepted_type)
        && argument.address_space().is_none()
        && argument.access().is_none()
}

const fn is_bf16_metadata_carrier(value_type: ExplicitValueType) -> bool {
    matches!(
        value_type,
        ExplicitValueType::I16 | ExplicitValueType::U16 | ExplicitValueType::F16
    )
}

const fn is_u64_metadata_carrier(value_type: ExplicitValueType) -> bool {
    matches!(value_type, ExplicitValueType::I64 | ExplicitValueType::U64)
}

const fn is_u32_metadata_carrier(value_type: ExplicitValueType) -> bool {
    matches!(value_type, ExplicitValueType::I32 | ExplicitValueType::U32)
}

fn exact_hidden_arguments(arguments: &[HiddenArgument], offset: u64) -> bool {
    const RELATIVE: [(u64, u64, HiddenValueKind); 19] = [
        (0, 4, HiddenValueKind::BlockCountX),
        (4, 4, HiddenValueKind::BlockCountY),
        (8, 4, HiddenValueKind::BlockCountZ),
        (12, 2, HiddenValueKind::GroupSizeX),
        (14, 2, HiddenValueKind::GroupSizeY),
        (16, 2, HiddenValueKind::GroupSizeZ),
        (18, 2, HiddenValueKind::RemainderX),
        (20, 2, HiddenValueKind::RemainderY),
        (22, 2, HiddenValueKind::RemainderZ),
        (40, 8, HiddenValueKind::GlobalOffsetX),
        (48, 8, HiddenValueKind::GlobalOffsetY),
        (56, 8, HiddenValueKind::GlobalOffsetZ),
        (64, 2, HiddenValueKind::GridDimensions),
        (80, 8, HiddenValueKind::HostcallBuffer),
        (88, 8, HiddenValueKind::MultigridSyncArgument),
        (96, 8, HiddenValueKind::HeapV1),
        (104, 8, HiddenValueKind::DefaultQueue),
        (112, 8, HiddenValueKind::CompletionAction),
        (200, 8, HiddenValueKind::QueuePointer),
    ];
    arguments.len() == RELATIVE.len()
        && arguments.iter().zip(RELATIVE).all(|(actual, expected)| {
            actual.offset() == offset + expected.0
                && actual.size() == expected.1
                && actual.value_kind() == expected.2
        })
}

fn fixed_link_options() -> Result<Vec<LinkOptionV1>, ExecuteQwen3GemmWorkerErrorV1> {
    [
        ("code-object-version", "6"),
        ("opt-level", "2"),
        ("strip-debug", "true"),
        ("verify-each", "true"),
    ]
    .into_iter()
    .map(|(name, value)| {
        LinkOptionV1::new(name, value).map_err(|_| ExecuteQwen3GemmWorkerErrorV1::FixedLinkOption)
    })
    .collect()
}

/// Failure while binding an inspected output to a finite runtime profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindQwen3GemmLaunchErrorV1 {
    /// The requested role/bucket/operation tuple was absent.
    Profile,
    /// Numerical buffer address or extent validation failed.
    Buffers(Qwen3GemmBufferContractErrorV1),
}

impl fmt::Display for BindQwen3GemmLaunchErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Qwen3 GEMM launch binding failed: {self:?}")
    }
}

impl std::error::Error for BindQwen3GemmLaunchErrorV1 {}

/// Exact inert runtime binding retained for a future protected launcher.
#[derive(Debug)]
pub struct CheckedQwen3GemmLaunchV1 {
    profile: Qwen3GemmProfileV1,
    buffers: Qwen3GemmBufferContractV1,
}

impl CheckedQwen3GemmLaunchV1 {
    /// Exact finite profile.
    #[must_use]
    pub const fn profile(&self) -> Qwen3GemmProfileV1 {
        self.profile
    }

    /// Exact checked BF16 buffer ranges.
    #[must_use]
    pub const fn buffers(&self) -> &Qwen3GemmBufferContractV1 {
        &self.buffers
    }

    /// Exact kernel entry selected by the profile schedule.
    #[must_use]
    pub const fn kernel_symbol(&self) -> &'static str {
        self.profile.schedule.kernel_symbol()
    }

    /// This binding grants no allocation, load, or launch authority.
    #[must_use]
    pub const fn grants_launch_authority(&self) -> bool {
        false
    }
}

/// Failure while binding an inspected token-embedding profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindQwen3TokenEmbeddingLaunchErrorV1 {
    /// The requested role/bucket tuple was absent.
    Profile,
    /// Expected weight identity or numerical buffer validation failed.
    Buffers(Qwen3TokenEmbeddingBufferContractErrorV1),
}

impl fmt::Display for BindQwen3TokenEmbeddingLaunchErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Qwen3 token-embedding launch binding failed: {self:?}"
        )
    }
}

impl std::error::Error for BindQwen3TokenEmbeddingLaunchErrorV1 {}

/// Exact inert token-embedding binding retained for a future protected launcher.
#[derive(Debug)]
pub struct CheckedQwen3TokenEmbeddingLaunchV1 {
    profile: Qwen3TokenEmbeddingProfileV1,
    buffers: Qwen3TokenEmbeddingBufferContractV1,
}

impl CheckedQwen3TokenEmbeddingLaunchV1 {
    /// Exact finite profile.
    #[must_use]
    pub const fn profile(&self) -> Qwen3TokenEmbeddingProfileV1 {
        self.profile
    }

    /// Exact checked buffer ranges and expected weight identity.
    #[must_use]
    pub const fn buffers(&self) -> &Qwen3TokenEmbeddingBufferContractV1 {
        &self.buffers
    }

    /// Exact token-embedding kernel symbol.
    #[must_use]
    pub const fn kernel_symbol(&self) -> &'static str {
        QWEN3_TOKEN_EMBEDDING_KERNEL_SYMBOL_V1
    }

    /// This binding grants no content, allocation, load, or launch authority.
    #[must_use]
    pub const fn grants_launch_authority(&self) -> bool {
        false
    }
}

fn checked_ceil_div_16(value: u32) -> Option<u32> {
    value.checked_add(15).map(|sum| sum / 16)
}

fn exact_target() -> DeviceTargetV1 {
    DeviceTargetV1::parse(QWEN3_GEMM_TARGET_V1).expect("the fixed Qwen3 GEMM target is canonical")
}

fn hash(domain: &[u8], bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update((bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn bindings(seed: u8) -> Qwen3GemmSourceBindingsV1 {
        Qwen3GemmSourceBindingsV1::new(
            [seed; 32],
            [seed.wrapping_add(1); 32],
            [seed.wrapping_add(2); 32],
            [seed.wrapping_add(3); 32],
        )
    }

    fn profile(
        role: Qwen3GemmModelRoleV1,
        kind: Qwen3GemmBucketKindV1,
        operation: Qwen3GemmOperationV1,
    ) -> Qwen3GemmProfileV1 {
        Qwen3GemmProfileCatalogV1::canonical()
            .unwrap()
            .profile(Qwen3GemmBucketV1::new(role, kind), operation)
            .unwrap()
    }

    fn layout(profile: Qwen3GemmProfileV1) -> ([u64; 3], [u64; 3]) {
        let elements = profile.storage_elements();
        (
            [0x1_0000_0000, 0x10_0000_0000, 0x20_0000_0000],
            [elements[0] * 2, elements[1] * 2, elements[2] * 2],
        )
    }

    #[test]
    fn exact_176_profile_catalog_is_complete_unique_and_deterministic() {
        let first = Qwen3GemmProfileCatalogV1::canonical().unwrap();
        let second = Qwen3GemmProfileCatalogV1::canonical().unwrap();
        assert_eq!(first, second);
        assert_eq!(first.profiles().len(), QWEN3_GEMM_PROFILE_COUNT_V1);
        assert_eq!(QWEN3_GEMM_PROFILE_COUNT_V1, 176);
        let identities = first
            .profiles()
            .iter()
            .map(|profile| *profile.identity().as_bytes())
            .collect::<BTreeSet<_>>();
        assert_eq!(identities.len(), QWEN3_GEMM_PROFILE_COUNT_V1);
        assert!(!first.grants_authority());
    }

    #[test]
    fn all_target_and_draft_bucket_rows_and_schedules_are_exact() {
        let catalog = Qwen3GemmProfileCatalogV1::canonical().unwrap();
        let rows = |role| {
            catalog
                .profiles()
                .iter()
                .filter(|profile| profile.bucket().role() == role)
                .map(|profile| profile.dimensions()[0])
                .collect::<BTreeSet<_>>()
        };
        assert_eq!(
            rows(Qwen3GemmModelRoleV1::Target8B),
            BTreeSet::from([1, 5, 8, 9, 17, 32, 40, 128, 512, 1_024, 2_048])
        );
        assert_eq!(
            rows(Qwen3GemmModelRoleV1::Draft06B),
            BTreeSet::from([1, 4, 8, 16, 32, 128, 512, 1_024, 2_048])
        );
        for profile in catalog.profiles() {
            assert_eq!(
                profile.schedule(),
                if profile.dimensions()[0] < 16 {
                    Qwen3GemmScheduleV1::ReferenceWave64V1
                } else {
                    Qwen3GemmScheduleV1::VectorizedA4Wave64V1
                }
            );
            assert_eq!(
                profile.execution_class(),
                if profile.dimensions()[0] == 1 {
                    Qwen3GemmExecutionClassV1::GemvM1
                } else {
                    Qwen3GemmExecutionClassV1::TiledGemm
                }
            );
        }
    }

    #[test]
    fn every_role_operation_dimension_is_exact_and_draft_o_projection_is_not_hidden_square() {
        let rows = 7;
        let expected = [
            (
                Qwen3GemmOperationV1::QueryProjection,
                [rows, 4_096, 4_096],
                [rows, 2_048, 1_024],
            ),
            (
                Qwen3GemmOperationV1::KeyProjection,
                [rows, 1_024, 4_096],
                [rows, 1_024, 1_024],
            ),
            (
                Qwen3GemmOperationV1::ValueProjection,
                [rows, 1_024, 4_096],
                [rows, 1_024, 1_024],
            ),
            (
                Qwen3GemmOperationV1::AttentionOutputResidual,
                [rows, 4_096, 4_096],
                [rows, 1_024, 2_048],
            ),
            (
                Qwen3GemmOperationV1::GateProjection,
                [rows, 12_288, 4_096],
                [rows, 3_072, 1_024],
            ),
            (
                Qwen3GemmOperationV1::UpProjection,
                [rows, 12_288, 4_096],
                [rows, 3_072, 1_024],
            ),
            (
                Qwen3GemmOperationV1::DownResidual,
                [rows, 4_096, 12_288],
                [rows, 1_024, 3_072],
            ),
            (
                Qwen3GemmOperationV1::LogitsProjection,
                [rows, 151_936, 4_096],
                [rows, 151_936, 1_024],
            ),
        ];
        for (operation, target, draft) in expected {
            assert_eq!(
                operation.dimensions(Qwen3GemmModelRoleV1::Target8B, rows),
                target
            );
            assert_eq!(
                operation.dimensions(Qwen3GemmModelRoleV1::Draft06B, rows),
                draft
            );
        }
        let draft = profile(
            Qwen3GemmModelRoleV1::Draft06B,
            Qwen3GemmBucketKindV1::DecodeS1C8192,
            Qwen3GemmOperationV1::AttentionOutputResidual,
        );
        assert_eq!(draft.dimensions(), [1, 1_024, 2_048]);
        let mut hostile_hidden_square = draft;
        hostile_hidden_square.dimensions[2] = 1_024;
        hostile_hidden_square.strides[0] = 1_024;
        hostile_hidden_square.storage_elements[0] = 1_024;
        hostile_hidden_square.storage_elements[1] = 1_024 * 1_024;
        assert_ne!(hostile_hidden_square, draft);
        assert_ne!(
            qwen3_gemm_kernel_ir_v1(hostile_hidden_square).identity(),
            qwen3_gemm_kernel_ir_v1(draft).identity()
        );
    }

    #[test]
    fn residual_beta_and_bf16_storage_policy_are_exact() {
        let catalog = Qwen3GemmProfileCatalogV1::canonical().unwrap();
        for profile in catalog.profiles() {
            let residual = matches!(
                profile.operation(),
                Qwen3GemmOperationV1::AttentionOutputResidual | Qwen3GemmOperationV1::DownResidual
            );
            assert_eq!(profile.alpha_bits(), 1.0_f32.to_bits());
            assert_eq!(
                profile.beta_bits(),
                if residual { 1.0_f32 } else { 0.0_f32 }.to_bits()
            );
            assert_eq!(
                profile.numerical_policy(),
                Qwen3GemmNumericalPolicyV1::Bf16StorageAscendingFp32Bf16Rne
            );
            let (_, lengths) = layout(*profile);
            assert_eq!(lengths[2], profile.storage_elements()[2] * 2);
        }
    }

    #[test]
    fn exhaustive_admitted_index_grid_and_extent_arithmetic_is_checked() {
        let catalog = Qwen3GemmProfileCatalogV1::canonical().unwrap();
        for profile in catalog.profiles() {
            let [rows, columns, reduction] = profile.dimensions().map(u64::from);
            let [a_elements, b_elements, c_elements] = profile.storage_elements();
            let a_last = (rows - 1)
                .checked_mul(reduction)
                .and_then(|base| base.checked_add(reduction - 1))
                .unwrap();
            let b_last = (reduction - 1)
                .checked_mul(columns)
                .and_then(|base| base.checked_add(columns - 1))
                .unwrap();
            let c_last = (rows - 1)
                .checked_mul(columns)
                .and_then(|base| base.checked_add(columns - 1))
                .unwrap();
            assert!(a_last < a_elements && b_last < b_elements && c_last < c_elements);
            assert!(
                i64::try_from(a_elements).is_ok()
                    && i64::try_from(b_elements).is_ok()
                    && i64::try_from(c_elements).is_ok()
            );
            assert!(a_elements.checked_mul(2).is_some());
            assert!(b_elements.checked_mul(2).is_some());
            assert!(c_elements.checked_mul(2).is_some());
            assert!(profile.dimensions()[2].is_multiple_of(4));
            let blocks = profile.hsa_adapter_block_counts();
            let grid = profile.aql_grid_workitems();
            assert_eq!(blocks[0].checked_mul(64), Some(grid[0]));
            assert_eq!(blocks[1], grid[1]);
            assert_eq!(blocks[2], grid[2]);
            assert_eq!(
                checked_ceil_div_16(profile.dimensions()[1]),
                Some(blocks[0])
            );
            assert_eq!(
                checked_ceil_div_16(profile.dimensions()[0]),
                Some(blocks[1])
            );
            assert!(!profile.proves_machine_arithmetic());
        }
    }

    #[test]
    fn complete_source_pin_and_classifier_reject_hostile_substitutions() {
        let exact = canonical_qwen3_gemm_llvm();
        assert_eq!(exact.len(), QWEN3_GEMM_LLVM_BYTES_V1);
        assert_eq!(
            <[u8; 32]>::from(Sha256::digest(exact.as_bytes())),
            QWEN3_GEMM_LLVM_SHA256_V1
        );
        validate_canonical_llvm(&exact).unwrap();

        let draft_hidden_substitution = exact.replacen(
            "%k.d.query = icmp eq i32 %k, 2048",
            "%k.d.query = icmp eq i32 %k, 1024",
            1,
        );
        assert_ne!(exact, draft_hidden_substitution);
        assert!(validate_canonical_llvm(&draft_hidden_substitution).is_err());

        let (before_vectorized, after_vectorized) = exact
            .rsplit_once("%k.d.query = icmp eq i32 %k, 2048")
            .unwrap();
        let vectorized_draft_hidden_substitution =
            format!("{before_vectorized}%k.d.query = icmp eq i32 %k, 1024{after_vectorized}");
        assert_ne!(exact, vectorized_draft_hidden_substitution);
        assert!(validate_canonical_llvm(&vectorized_draft_hidden_substitution).is_err());

        let draft_classifier_substitution = exact.replacen(
            "%draft.o.nk = and i1 %n.d.hidden, %k.d.query",
            "%draft.o.nk = and i1 %n.d.hidden, %k.d.hidden",
            1,
        );
        assert_ne!(exact, draft_classifier_substitution);
        assert!(validate_canonical_llvm(&draft_classifier_substitution).is_err());

        let residual_substitution = exact.replacen(
            "%draft.o = and i1 %draft.o.nk, %beta.one",
            "%draft.o = and i1 %draft.o.nk, %beta.zero",
            1,
        );
        assert_ne!(exact, residual_substitution);
        assert!(validate_canonical_llvm(&residual_substitution).is_err());

        let vector_transfer_substitution = exact.replacen(
            "load <4 x i16>, ptr addrspace(1) %a.ptr, align 2",
            "load <4 x i16>, ptr addrspace(1) %a.ptr, align 1",
            1,
        );
        assert_ne!(exact, vector_transfer_substitution);
        assert!(validate_canonical_llvm(&vector_transfer_substitution).is_err());

        let store_substitution = exact.replacen("store i16 %result.bf16", "store i16 0", 1);
        assert_ne!(exact, store_substitution);
        assert!(validate_canonical_llvm(&store_substitution).is_err());
    }

    #[test]
    fn source_bindings_fail_closed_and_preparation_retains_nonclaims() {
        assert!(prepare_qwen3_gemm_kernel_v1(bindings(1)).is_ok());
        assert!(prepare_qwen3_gemm_kernel_v1(Qwen3GemmSourceBindingsV1::new(
            [0; 32], [2; 32], [3; 32], [4; 32]
        ))
        .is_err());
        assert!(prepare_qwen3_gemm_kernel_v1(Qwen3GemmSourceBindingsV1::new(
            [1; 32], [1; 32], [3; 32], [4; 32]
        ))
        .is_err());
        let prepared = prepare_qwen3_gemm_kernel_v1(bindings(7)).unwrap();
        assert!(!prepared.uses_typed_handoff_v2_source());
        assert!(!prepared.classifier_distinguishes_duplicate_profiles());
        assert!(!prepared.authenticates_compiler_origin());
        assert!(!prepared.proves_operator_or_numerical_refinement());
        assert!(!prepared.has_ferric_plan_identity_join());
        assert!(!prepared.has_kernel_schedule_catalog_join());
        assert!(!prepared.grants_launch_authority());
        assert!(prepared
            .compiler_handoff()
            .envelope()
            .directional_symbols()
            .imports()
            .next()
            .is_none());
    }

    #[test]
    fn buffer_contract_rejects_all_aliasing_even_for_beta_zero() {
        let profile = profile(
            Qwen3GemmModelRoleV1::Target8B,
            Qwen3GemmBucketKindV1::DecodeS1C8192,
            Qwen3GemmOperationV1::QueryProjection,
        );
        assert_eq!(profile.beta_bits(), 0);
        let (addresses, lengths) = layout(profile);
        let checked = Qwen3GemmBufferContractV1::checked(profile, addresses, lengths).unwrap();
        assert!(!checked.authenticates_device_memory());
        for pair in [(0, 1), (0, 2), (1, 2)] {
            let mut aliased = addresses;
            aliased[pair.1] = aliased[pair.0];
            assert_eq!(
                Qwen3GemmBufferContractV1::checked(profile, aliased, lengths),
                Err(Qwen3GemmBufferContractErrorV1::Aliasing)
            );
        }
        let mut short = lengths;
        short[2] -= 2;
        assert_eq!(
            Qwen3GemmBufferContractV1::checked(profile, addresses, short),
            Err(Qwen3GemmBufferContractErrorV1::ByteLength(
                Qwen3GemmBufferV1::C
            ))
        );
        let mut misaligned = addresses;
        misaligned[1] += 1;
        assert_eq!(
            Qwen3GemmBufferContractV1::checked(profile, misaligned, lengths),
            Err(Qwen3GemmBufferContractErrorV1::Alignment(
                Qwen3GemmBufferV1::B
            ))
        );
        let mut overflowing = addresses;
        overflowing[0] = u64::MAX - lengths[0] + 1;
        assert_eq!(
            Qwen3GemmBufferContractV1::checked(profile, overflowing, lengths),
            Err(Qwen3GemmBufferContractErrorV1::RangeOverflow(
                Qwen3GemmBufferV1::A
            ))
        );
    }

    #[test]
    fn machine_equivalent_duplicate_shapes_keep_distinct_host_profiles() {
        let catalog = Qwen3GemmProfileCatalogV1::canonical().unwrap();
        let bucket = Qwen3GemmBucketV1::new(
            Qwen3GemmModelRoleV1::Draft06B,
            Qwen3GemmBucketKindV1::DecodeS1C8192,
        );
        let key = catalog
            .profile(bucket, Qwen3GemmOperationV1::KeyProjection)
            .unwrap();
        let value = catalog
            .profile(bucket, Qwen3GemmOperationV1::ValueProjection)
            .unwrap();
        assert_eq!(key.dimensions(), value.dimensions());
        assert_eq!(key.beta_bits(), value.beta_bits());
        assert_ne!(key.identity(), value.identity());
        assert_ne!(
            qwen3_gemm_kernel_ir_v1(key).identity(),
            qwen3_gemm_kernel_ir_v1(value).identity()
        );
    }

    #[test]
    fn exact_22_token_embedding_profiles_bind_role_bucket_rows_and_hidden() {
        let first = Qwen3TokenEmbeddingProfileCatalogV1::canonical().unwrap();
        let second = Qwen3TokenEmbeddingProfileCatalogV1::canonical().unwrap();
        assert_eq!(first, second);
        assert_eq!(
            first.profiles().len(),
            QWEN3_TOKEN_EMBEDDING_PROFILE_COUNT_V1
        );
        assert_eq!(QWEN3_TOKEN_EMBEDDING_PROFILE_COUNT_V1, 22);
        let identities = first
            .profiles()
            .iter()
            .map(|profile| *profile.identity().as_bytes())
            .collect::<BTreeSet<_>>();
        assert_eq!(identities.len(), QWEN3_TOKEN_EMBEDDING_PROFILE_COUNT_V1);
        for profile in first.profiles() {
            let [sequences, active, hidden] = profile.shape();
            let rows = sequences.checked_mul(active).unwrap();
            assert_eq!(profile.rows(), rows);
            assert_eq!(hidden, profile.bucket().role().hidden_size());
            assert_eq!(
                profile.storage_elements(),
                [
                    u64::from(rows),
                    u64::from(QWEN3_VOCABULARY_SIZE_V1) * u64::from(hidden),
                    u64::from(rows) * u64::from(hidden),
                ]
            );
            assert_eq!(
                profile.aql_grid_workitems(),
                [rows.checked_mul(hidden).unwrap(), 1, 1]
            );
            assert_eq!(
                qwen3_token_embedding_kernel_ir_v1(*profile).shape(),
                profile.shape()
            );
            assert!(!profile.authenticates_weight_content());
        }
        assert!(!first.grants_authority());
    }

    #[test]
    fn token_embedding_profile_rejects_hostile_vocab_hidden_row_and_profile_substitutions() {
        let catalog = Qwen3TokenEmbeddingProfileCatalogV1::canonical().unwrap();
        let draft = catalog
            .profile(Qwen3GemmBucketV1::new(
                Qwen3GemmModelRoleV1::Draft06B,
                Qwen3GemmBucketKindV1::SpeculativeS1K16C8192,
            ))
            .unwrap();
        assert_eq!(draft.shape(), [1, 16, 1_024]);
        let mut hidden = draft;
        hidden.shape[2] = 4_096;
        assert_ne!(hidden, draft);
        assert_ne!(
            qwen3_token_embedding_kernel_ir_v1(hidden).identity(),
            qwen3_token_embedding_kernel_ir_v1(draft).identity()
        );
        let mut row = draft;
        row.shape[1] = 17;
        assert_ne!(row, draft);
        let target = catalog
            .profile(Qwen3GemmBucketV1::new(
                Qwen3GemmModelRoleV1::Target8B,
                Qwen3GemmBucketKindV1::SpeculativeS1K16C8192,
            ))
            .unwrap();
        assert_eq!(target.shape(), [1, 17, 4_096]);
        assert_ne!(target.identity(), draft.identity());
    }

    #[test]
    fn token_embedding_buffers_join_expected_role_and_reject_lengths_aliases_and_overflow() {
        let profile = Qwen3TokenEmbeddingProfileCatalogV1::canonical()
            .unwrap()
            .profile(Qwen3GemmBucketV1::new(
                Qwen3GemmModelRoleV1::Target8B,
                Qwen3GemmBucketKindV1::DecodeS1C8192,
            ))
            .unwrap();
        let target_identity =
            Qwen3ExpectedEmbeddingWeightIdentityV1::new(Qwen3GemmModelRoleV1::Target8B, [7; 32])
                .unwrap();
        assert!(Qwen3ExpectedEmbeddingWeightIdentityV1::new(
            Qwen3GemmModelRoleV1::Target8B,
            [0; 32]
        )
        .is_err());
        let elements = profile.storage_elements();
        let addresses = [0x1_0000_0000, 0x10_0000_0000, 0x20_0000_0000];
        let lengths = [elements[0] * 4, elements[1] * 2, elements[2] * 2];
        let checked = Qwen3TokenEmbeddingBufferContractV1::checked(
            profile,
            target_identity,
            addresses,
            lengths,
        )
        .unwrap();
        assert_eq!(checked.expected_weight_identity(), target_identity);
        assert!(!checked.authenticates_content_or_memory());
        let draft_identity =
            Qwen3ExpectedEmbeddingWeightIdentityV1::new(Qwen3GemmModelRoleV1::Draft06B, [8; 32])
                .unwrap();
        assert_eq!(
            Qwen3TokenEmbeddingBufferContractV1::checked(
                profile,
                draft_identity,
                addresses,
                lengths,
            ),
            Err(Qwen3TokenEmbeddingBufferContractErrorV1::WeightRole)
        );
        for pair in [(0, 1), (0, 2), (1, 2)] {
            let mut aliased = addresses;
            aliased[pair.1] = aliased[pair.0];
            assert_eq!(
                Qwen3TokenEmbeddingBufferContractV1::checked(
                    profile,
                    target_identity,
                    aliased,
                    lengths,
                ),
                Err(Qwen3TokenEmbeddingBufferContractErrorV1::Aliasing)
            );
        }
        let mut short = lengths;
        short[1] -= 2;
        assert_eq!(
            Qwen3TokenEmbeddingBufferContractV1::checked(
                profile,
                target_identity,
                addresses,
                short,
            ),
            Err(Qwen3TokenEmbeddingBufferContractErrorV1::ByteLength(
                Qwen3TokenEmbeddingBufferV1::Weight
            ))
        );
        let mut overflowing = addresses;
        overflowing[2] = u64::MAX - lengths[2] + 1;
        assert_eq!(
            Qwen3TokenEmbeddingBufferContractV1::checked(
                profile,
                target_identity,
                overflowing,
                lengths,
            ),
            Err(Qwen3TokenEmbeddingBufferContractErrorV1::RangeOverflow(
                Qwen3TokenEmbeddingBufferV1::Output
            ))
        );
    }

    #[test]
    fn token_bound_dominates_weight_address_and_source_substitutions_fail_closed() {
        let exact = canonical_qwen3_gemm_llvm();
        let embedding = exact
            .split_once(QWEN3_TOKEN_EMBEDDING_KERNEL_SYMBOL_V1)
            .unwrap()
            .1;
        let bound = embedding
            .find("%token.in.vocabulary = icmp ult i32 %token, %vocabulary")
            .unwrap();
        let branch = embedding
            .find("br i1 %token.in.vocabulary, label %weight.address, label %trap")
            .unwrap();
        let weight_address = embedding.find("weight.address:").unwrap();
        let weight_gep = embedding
            .find("%weight.ptr = getelementptr inbounds i16")
            .unwrap();
        assert!(bound < branch && branch < weight_address && weight_address < weight_gep);
        assert_eq!(embedding.matches("store i16 %embedding.bits").count(), 1);
        assert!(!embedding.contains("fadd"));
        assert!(!embedding.contains("fmul"));
        for hostile in [
            exact.replacen(
                "%embedding.vocabulary.ok = icmp eq i32 %vocabulary, 151936",
                "%embedding.vocabulary.ok = icmp eq i32 %vocabulary, 151935",
                1,
            ),
            exact.replacen(
                "%embedding.draft.hidden = icmp eq i32 %hidden, 1024",
                "%embedding.draft.hidden = icmp eq i32 %hidden, 4096",
                1,
            ),
            exact.replacen(
                "%embedding.draft.row.8 = icmp eq i32 %rows, 2048",
                "%embedding.draft.row.8 = icmp eq i32 %rows, 2049",
                1,
            ),
            exact.replacen(
                "%token.in.vocabulary = icmp ult i32 %token, %vocabulary",
                "%token.in.vocabulary = icmp ule i32 %token, %vocabulary",
                1,
            ),
            exact.replacen(
                "br i1 %token.in.vocabulary, label %weight.address, label %trap",
                "br i1 true, label %weight.address, label %trap",
                1,
            ),
            exact.replacen("store i16 %embedding.bits", "store i16 0", 1),
        ] {
            assert_ne!(hostile, exact);
            assert!(validate_canonical_llvm(&hostile).is_err());
        }
    }
}
