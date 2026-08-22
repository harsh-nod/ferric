//! Exact finite Qwen3 split-half RoPE and P16 paged-KV-write profiles.
//!
//! The two machine ABIs validate exact finite geometry and element counts.
//! RoPE indexes fixed 8192-by-64 FP32 cosine/sine tables with the supplied
//! absolute position IDs. KV write scatters through one 512-entry P16 page
//! table per sequence into a fixed global 16,384-page cache pool. Table
//! content, ownership, generation, trigonometric content/provenance, and
//! Ferric plan identity remain host-side obligations.

use core::fmt;

use fe2o3_amdhsa_loader::{AdmittedProfile, LoadPlan, PlanError};
use fe2o3_artifact_transaction::{
    CompilerModuleHandoffIdentityV1, ConsumedCompilerModuleHandoffV1,
};
use fe2o3_compiler_ffi::{
    CodeObjectVersion, CompilerFfiEnvelopeError, CompilerFfiEnvelopeV1,
    CompilerModuleHandoffErrorV2, CompilerModuleHandoffIdentityV2, CompilerModuleHandoffV2,
    CompilerModuleKindV1, CompilerModuleSymbolManifestErrorV1,
    CompilerModuleSymbolManifestIdentityV1, CompilerModuleSymbolManifestV1,
    CompilerModuleSymbolRoleV1, DeviceTargetV1,
};
use fe2o3_hsaco::{
    inspect_and_bind_kernel_descriptors, ArgumentAccess, ArgumentAddressSpace,
    CodeObjectVersion as InspectedCodeObjectVersion, ExplicitArgument, ExplicitValueKind,
    ExplicitValueType, HiddenArgument, HiddenValueKind, KernelBindingError, MAX_HSACO_BYTES,
};
use fe2o3_hsaco_finalize::{
    execute_reproducible_first_build_worker_v2, FirstBuildWorkerV2Error,
    InertDecodedWorkerExchangeV2, InertFirstBuildWorkerV2EvidenceV1, LinkOptionV1, PinnedWorkerV1,
    WorkerExecutionLimitsV1, WorkerOutputConstraintsV1, WorkerProtocolError,
};
use fe2o3_llvm_handoff::{
    AddressSpaceV1, AxisV2, BasicBlockV2, BinaryOperationV2, BlockIdV2, CallingConventionV2,
    CastOperationV2, ComparePredicateV2, EvidenceV2, ExecutableModuleV2, FloatBinaryOperationV2,
    FunctionAttributeV1, FunctionAttributeV2, FunctionIdV2, FunctionKindV2, FunctionParameterV2,
    FunctionV2, Gfx942HandoffInputV1, Gfx942HandoffV1, Gfx942HandoffV2, Gfx942TargetPolicyV1,
    HandoffDiagnosticV1, HandoffDiagnosticV2, HandoffIdentityV2, IdentityV1, InstructionKindV2,
    InstructionV2, IntegerBinaryOperationV2, IntrinsicReferenceV2, IntrinsicV2, KernelEntryV1,
    KernelParameterV1, KernelValueTypeV1, ModuleFlagV1, ModuleMetadataV1, ObligationKindV1,
    ObligationV1, OriginKindV1, OriginV1, ParameterAttributeV1, ReturnTypeV2, ScalarConstantV2,
    ScalarTypeV1, StageIdentitiesV1, TerminatorV2, TypedValueV2, ValueIdV2, ValueTypeV2,
    WorkgroupSizeRangeV1,
};
use fe2o3_llvm_text::{serialize_gfx942_handoff_v2, Gfx942LlvmAssemblyV2, SerializeErrorV2};
use fe2o3_llvm_worker_handoff::{
    MeasuredLlvmLldBuildV1, WorkerAdmissionErrorV2, WorkerAdmissionIdentityV2,
    WorkerAdmissionRequestV2,
};
use sha2::{Digest as _, Sha256};

/// Exact `RoPE` kernel entry emitted by the typed graph.
pub const QWEN3_ROPE_KERNEL_SYMBOL_V1: &str = "qwen3_rope_v1";
/// Exact `RoPE` AMDHSA descriptor symbol.
pub const QWEN3_ROPE_KERNEL_DESCRIPTOR_SYMBOL_V1: &str = "qwen3_rope_v1.kd";
/// Exact paged-KV-write kernel entry emitted by the typed graph.
pub const QWEN3_PAGED_KV_WRITE_KERNEL_SYMBOL_V1: &str = "qwen3_paged_kv_write_v1";
/// Exact paged-KV-write AMDHSA descriptor symbol.
pub const QWEN3_PAGED_KV_WRITE_KERNEL_DESCRIPTOR_SYMBOL_V1: &str = "qwen3_paged_kv_write_v1.kd";
/// Exact device target required by this compiler lane.
pub const QWEN3_ROPE_KV_TARGET_V1: &str = "gfx942:xnack-";
/// Exact code-object version required by this compiler lane.
pub const QWEN3_ROPE_KV_CODE_OBJECT_VERSION_V1: u8 = 6;
/// One wave64 workgroup is assigned to each active row.
pub const QWEN3_ROPE_KV_WORKGROUP_V1: [u32; 3] = [64, 1, 1];
/// Exact Qwen3 head dimension.
pub const QWEN3_ROPE_KV_HEAD_DIMENSION_V1: u32 = 128;
/// Exact split-half rotary pair count.
pub const QWEN3_ROPE_PAIR_COUNT_V1: u32 = 64;
/// Exact Qwen3 rotary base declared for the deployment table.
pub const QWEN3_ROPE_THETA_V1: u32 = 1_000_000;
/// Exact maximum logical context.
pub const QWEN3_ROPE_KV_MAX_CONTEXT_TOKENS_V1: u32 = 8_192;
/// Fixed tokens per physical KV page.
pub const QWEN3_KV_PAGE_TOKENS_V1: u32 = 16;
/// Fixed page-table entries per sequence.
pub const QWEN3_KV_PAGE_TABLE_ENTRIES_V1: u32 = 512;
/// Fixed physical page slots in the global Ferric KV cache pool.
pub const QWEN3_KV_PHYSICAL_PAGE_SLOTS_V1: u32 = 16_384;
/// BF16 elements in each fixed global key or value cache.
pub const QWEN3_KV_CACHE_ELEMENTS_V1: u64 = QWEN3_KV_PHYSICAL_PAGE_SLOTS_V1 as u64
    * QWEN3_KV_PAGE_TOKENS_V1 as u64
    * 8
    * QWEN3_ROPE_KV_HEAD_DIMENSION_V1 as u64;
/// Bytes in each fixed global BF16 key or value cache.
pub const QWEN3_KV_CACHE_BYTES_V1: u64 = QWEN3_KV_CACHE_ELEMENTS_V1 * 2;
/// FP32 entries in each fixed deployment trigonometric table.
pub const QWEN3_ROPE_TRIG_TABLE_ELEMENTS_V1: u64 = 8_192 * 64;
/// Number of exact role/bucket/operator profiles.
pub const QWEN3_ROPE_KV_PROFILE_COUNT_V1: usize = 44;
/// `RoPE` explicit kernarg bytes and hidden-argument offset.
pub const QWEN3_ROPE_EXPLICIT_KERNARG_BYTES_V1: u64 = 128;
/// `RoPE` complete explicit plus COV6 hidden kernarg bytes.
pub const QWEN3_ROPE_TOTAL_KERNARG_BYTES_V1: u64 = 128 + 256;
/// KV-write explicit kernarg bytes and hidden-argument offset.
pub const QWEN3_KV_WRITE_EXPLICIT_KERNARG_BYTES_V1: u64 = 112;
/// KV-write complete explicit plus COV6 hidden kernarg bytes.
pub const QWEN3_KV_WRITE_TOTAL_KERNARG_BYTES_V1: u64 = 112 + 256;
/// Exact kernarg alignment required by the closed ABI.
pub const QWEN3_ROPE_KV_KERNARG_ALIGNMENT_V1: u64 = 8;

const CATALOG_DOMAIN: &[u8] = b"FERRIC/QWEN3/ROPE-KV/PROFILE-CATALOG/V1\0";
const PROFILE_DOMAIN: &[u8] = b"FERRIC/QWEN3/ROPE-KV/PROFILE/V1\0";
const SOURCE_DOMAIN: &[u8] = b"FERRIC/QWEN3/ROPE-KV/TYPED-SOURCE/V1\0";
const SEMANTIC_DOMAIN: &[u8] = b"FERRIC/QWEN3/ROPE-KV/SEMANTIC-STAGE/V1\0";
const SCHEDULE_DOMAIN: &[u8] = b"FERRIC/QWEN3/ROPE-KV/SCHEDULE-STAGE/V1\0";
const TARGET_PLAN_DOMAIN: &[u8] = b"FERRIC/QWEN3/ROPE-KV/TARGET-PLAN-STAGE/V1\0";

const REQUIRED_OBLIGATIONS: [ObligationKindV1; 7] = [
    ObligationKindV1::PreserveKernelAbi,
    ObligationKindV1::PreserveAddressSpaces,
    ObligationKindV1::PreserveTargetFeatures,
    ObligationKindV1::PreserveCallingConvention,
    ObligationKindV1::PreserveFunctionAttributes,
    ObligationKindV1::PreserveModuleMetadata,
    ObligationKindV1::MaintainOriginCoverage,
];

/// Target or speculative-draft Qwen3 model role.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Qwen3RopeKvModelRoleV1 {
    /// Pinned Qwen3-8B target geometry.
    Target8B = 1,
    /// Pinned Qwen3-0.6B draft geometry.
    Draft06B = 2,
}

impl Qwen3RopeKvModelRoleV1 {
    /// Exact transformer layer count; a runner selects the layer outside these kernels.
    #[must_use]
    pub const fn layers(self) -> u32 {
        match self {
            Self::Target8B => 36,
            Self::Draft06B => 28,
        }
    }

    /// Exact query-head count.
    #[must_use]
    pub const fn query_heads(self) -> u32 {
        match self {
            Self::Target8B => 32,
            Self::Draft06B => 16,
        }
    }

    /// Exact key/value-head count.
    #[must_use]
    pub const fn key_value_heads(self) -> u32 {
        8
    }
}

/// One exact Ferric M1 mode bucket.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Qwen3RopeKvBucketKindV1 {
    /// One sequence with 128 active prefill tokens.
    PrefillS1T128 = 1,
    /// Eight sequences with 128 active prefill tokens each.
    PrefillS8T128 = 2,
    /// One sequence with 512 active prefill tokens.
    PrefillS1T512 = 3,
    /// One sequence with 2,048 active prefill tokens.
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

impl Qwen3RopeKvBucketKindV1 {
    const fn sequence_and_active_tokens(self, role: Qwen3RopeKvModelRoleV1) -> [u32; 2] {
        match self {
            Self::PrefillS1T128 => [1, 128],
            Self::PrefillS8T128 => [8, 128],
            Self::PrefillS1T512 => [1, 512],
            Self::PrefillS1T2048 => [1, 2_048],
            Self::DecodeS1C8192 => [1, 1],
            Self::DecodeS8C8192 => [8, 1],
            Self::DecodeS32C8192 => [32, 1],
            Self::SpeculativeS1K4C8192 => match role {
                Qwen3RopeKvModelRoleV1::Target8B => [1, 5],
                Qwen3RopeKvModelRoleV1::Draft06B => [1, 4],
            },
            Self::SpeculativeS8K4C8192 => match role {
                Qwen3RopeKvModelRoleV1::Target8B => [8, 5],
                Qwen3RopeKvModelRoleV1::Draft06B => [8, 4],
            },
            Self::SpeculativeS1K8C8192 => match role {
                Qwen3RopeKvModelRoleV1::Target8B => [1, 9],
                Qwen3RopeKvModelRoleV1::Draft06B => [1, 8],
            },
            Self::SpeculativeS1K16C8192 => match role {
                Qwen3RopeKvModelRoleV1::Target8B => [1, 17],
                Qwen3RopeKvModelRoleV1::Draft06B => [1, 16],
            },
        }
    }

    const fn context_tokens(self) -> u32 {
        match self {
            Self::PrefillS1T128 | Self::PrefillS8T128 => 128,
            Self::PrefillS1T512 => 512,
            Self::PrefillS1T2048 => 2_048,
            Self::DecodeS1C8192
            | Self::DecodeS8C8192
            | Self::DecodeS32C8192
            | Self::SpeculativeS1K4C8192
            | Self::SpeculativeS8K4C8192
            | Self::SpeculativeS1K8C8192
            | Self::SpeculativeS1K16C8192 => 8_192,
        }
    }
}

/// One exact role and mode-bucket selection.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Qwen3RopeKvBucketV1 {
    role: Qwen3RopeKvModelRoleV1,
    kind: Qwen3RopeKvBucketKindV1,
}

impl Qwen3RopeKvBucketV1 {
    /// Creates one finite target/draft selection.
    #[must_use]
    pub const fn new(role: Qwen3RopeKvModelRoleV1, kind: Qwen3RopeKvBucketKindV1) -> Self {
        Self { role, kind }
    }

    /// Exact model role.
    #[must_use]
    pub const fn role(self) -> Qwen3RopeKvModelRoleV1 {
        self.role
    }

    /// Exact bucket kind.
    #[must_use]
    pub const fn kind(self) -> Qwen3RopeKvBucketKindV1 {
        self.kind
    }

    /// Exact `[sequences, active_tokens]` dimensions.
    #[must_use]
    pub const fn sequence_and_active_tokens(self) -> [u32; 2] {
        self.kind.sequence_and_active_tokens(self.role)
    }

    /// Exact logical context selected by Ferric.
    #[must_use]
    pub const fn context_tokens(self) -> u32 {
        self.kind.context_tokens()
    }

    /// Number of active rows seen by RoPE/KV.
    #[must_use]
    pub const fn flattened_rows(self) -> u32 {
        let dimensions = self.sequence_and_active_tokens();
        dimensions[0] * dimensions[1]
    }
}

/// SHA-256 identity of one exact profile record.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Qwen3RopeKvProfileIdentityV1([u8; 32]);

impl Qwen3RopeKvProfileIdentityV1 {
    /// Returns the domain-separated identity bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Exact Ferric graph operator selected by the host catalog.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Qwen3RopeKvOperationV1 {
    /// Qwen3 split-half rotary embedding over query and key heads.
    Rope = 1,
    /// Append rotated keys and values through a per-sequence P16 page table.
    PagedKvWrite = 2,
}

/// One finite checked Qwen3 RoPE/KV operation profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Qwen3RopeKvProfileV1 {
    bucket: Qwen3RopeKvBucketV1,
    operation: Qwen3RopeKvOperationV1,
    base_rows: u32,
    query_elements: u64,
    kv_elements: u64,
    block_counts: [u32; 3],
    aql_grid_work_items: [u32; 3],
    identity: Qwen3RopeKvProfileIdentityV1,
}

impl Qwen3RopeKvProfileV1 {
    fn checked(
        bucket: Qwen3RopeKvBucketV1,
        operation: Qwen3RopeKvOperationV1,
    ) -> Result<Self, Qwen3RopeKvCatalogErrorV1> {
        let base_rows = bucket.flattened_rows();
        let [sequences, active_tokens] = bucket.sequence_and_active_tokens();
        if operation == Qwen3RopeKvOperationV1::Rope
            && !rope_machine_geometry_is_known(
                active_tokens,
                sequences,
                bucket.role.query_heads(),
                bucket.context_tokens(),
            )
        {
            return Err(Qwen3RopeKvCatalogErrorV1::OperationGeometry);
        }
        let query_elements = u64::from(base_rows)
            .checked_mul(u64::from(bucket.role.query_heads()))
            .and_then(|value| value.checked_mul(u64::from(QWEN3_ROPE_KV_HEAD_DIMENSION_V1)))
            .ok_or(Qwen3RopeKvCatalogErrorV1::ExtentOverflow)?;
        let kv_elements = u64::from(base_rows)
            .checked_mul(u64::from(bucket.role.key_value_heads()))
            .and_then(|value| value.checked_mul(u64::from(QWEN3_ROPE_KV_HEAD_DIMENSION_V1)))
            .ok_or(Qwen3RopeKvCatalogErrorV1::ExtentOverflow)?;
        let grid_x = active_tokens
            .checked_mul(QWEN3_ROPE_KV_WORKGROUP_V1[0])
            .ok_or(Qwen3RopeKvCatalogErrorV1::GridOverflow)?;
        let mut profile = Self {
            bucket,
            operation,
            base_rows,
            query_elements,
            kv_elements,
            block_counts: [active_tokens, sequences, 1],
            aql_grid_work_items: [grid_x, sequences, 1],
            identity: Qwen3RopeKvProfileIdentityV1([0; 32]),
        };
        profile.identity = Qwen3RopeKvProfileIdentityV1(hash(PROFILE_DOMAIN, &profile.encode()));
        Ok(profile)
    }

    fn encode(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(80);
        bytes.push(self.bucket.role as u8);
        bytes.push(self.bucket.kind as u8);
        bytes.push(self.operation as u8);
        bytes.extend_from_slice(&self.base_rows.to_le_bytes());
        bytes.extend_from_slice(&self.bucket.context_tokens().to_le_bytes());
        bytes.extend_from_slice(&self.bucket.role.query_heads().to_le_bytes());
        bytes.extend_from_slice(&self.query_elements.to_le_bytes());
        bytes.extend_from_slice(&self.kv_elements.to_le_bytes());
        for value in self.block_counts {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in self.aql_grid_work_items {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(&QWEN3_ROPE_THETA_V1.to_le_bytes());
        bytes.extend_from_slice(&QWEN3_KV_PAGE_TOKENS_V1.to_le_bytes());
        bytes.extend_from_slice(&QWEN3_KV_PAGE_TABLE_ENTRIES_V1.to_le_bytes());
        bytes.extend_from_slice(&QWEN3_KV_PHYSICAL_PAGE_SLOTS_V1.to_le_bytes());
        bytes
    }

    /// Exact role and bucket selection.
    #[must_use]
    pub const fn bucket(self) -> Qwen3RopeKvBucketV1 {
        self.bucket
    }

    /// Exact graph or auxiliary operation variant.
    #[must_use]
    pub const fn operation(self) -> Qwen3RopeKvOperationV1 {
        self.operation
    }

    /// Exact flattened sequence-token row count.
    #[must_use]
    pub const fn base_rows(self) -> u32 {
        self.base_rows
    }

    /// Exact query tensor element count.
    #[must_use]
    pub const fn query_elements(self) -> u64 {
        self.query_elements
    }

    /// Exact key or value tensor element count.
    #[must_use]
    pub const fn kv_elements(self) -> u64 {
        self.kv_elements
    }

    /// Exact logical context tokens per sequence.
    #[must_use]
    pub const fn context_tokens(self) -> u32 {
        self.bucket.context_tokens()
    }

    /// Exact role-specific query head count.
    #[must_use]
    pub const fn query_heads(self) -> u32 {
        self.bucket.role.query_heads()
    }

    /// HSA-adapter block counts before workgroup expansion.
    #[must_use]
    pub const fn hsa_adapter_block_counts(self) -> [u32; 3] {
        self.block_counts
    }

    /// Exact AQL total-workitem grid.
    #[must_use]
    pub const fn aql_grid_work_items(self) -> [u32; 3] {
        self.aql_grid_work_items
    }

    /// Exact domain-separated profile identity.
    #[must_use]
    pub const fn identity(self) -> Qwen3RopeKvProfileIdentityV1 {
        self.identity
    }

    /// Geometry and numerical declarations grant no launch authority.
    #[must_use]
    pub const fn grants_launch_authority(self) -> bool {
        false
    }
}

/// Failure while deriving the finite catalog.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Qwen3RopeKvCatalogErrorV1 {
    /// A role/bucket/operator tuple was absent from the shared exact geometry roster.
    OperationGeometry,
    /// A row extent overflowed `u64`.
    ExtentOverflow,
    /// Workgroup expansion overflowed the AQL grid domain.
    GridOverflow,
}

impl fmt::Display for Qwen3RopeKvCatalogErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Qwen3 RoPE/KV catalog failed: {self:?}")
    }
}

impl std::error::Error for Qwen3RopeKvCatalogErrorV1 {}

/// SHA-256 identity of the complete finite catalog.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Qwen3RopeKvProfileCatalogIdentityV1([u8; 32]);

impl Qwen3RopeKvProfileCatalogIdentityV1 {
    /// Returns the exact catalog identity bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Complete finite target/draft RoPE/KV profile catalog.
#[derive(Debug, Eq, PartialEq)]
pub struct Qwen3RopeKvProfileCatalogV1 {
    profiles: Box<[Qwen3RopeKvProfileV1]>,
    canonical_bytes: Box<[u8]>,
    identity: Qwen3RopeKvProfileCatalogIdentityV1,
}

impl Qwen3RopeKvProfileCatalogV1 {
    /// Constructs all 44 profiles in stable role/bucket/operation order.
    ///
    /// # Errors
    ///
    /// Returns an error if a fixed profile violates the closed operation
    /// geometry or if any derived extent cannot be represented exactly.
    pub fn canonical() -> Result<Self, Qwen3RopeKvCatalogErrorV1> {
        let mut profiles = Vec::with_capacity(QWEN3_ROPE_KV_PROFILE_COUNT_V1);
        for role in QWEN3_ROPE_KV_ROLES_V1 {
            for kind in QWEN3_ROPE_KV_BUCKET_KINDS_V1 {
                for operation in QWEN3_ROPE_KV_OPERATIONS_V1 {
                    profiles.push(Qwen3RopeKvProfileV1::checked(
                        Qwen3RopeKvBucketV1::new(role, kind),
                        operation,
                    )?);
                }
            }
        }
        let mut canonical_bytes = Vec::with_capacity(2_048);
        let profile_count =
            u32::try_from(profiles.len()).map_err(|_| Qwen3RopeKvCatalogErrorV1::ExtentOverflow)?;
        canonical_bytes.extend_from_slice(&profile_count.to_le_bytes());
        canonical_bytes.extend_from_slice(QWEN3_ROPE_KV_TARGET_V1.as_bytes());
        canonical_bytes.push(QWEN3_ROPE_KV_CODE_OBJECT_VERSION_V1);
        for profile in &profiles {
            let encoded = profile.encode();
            let encoded_len = u32::try_from(encoded.len())
                .map_err(|_| Qwen3RopeKvCatalogErrorV1::ExtentOverflow)?;
            canonical_bytes.extend_from_slice(&encoded_len.to_le_bytes());
            canonical_bytes.extend_from_slice(&encoded);
            canonical_bytes.extend_from_slice(profile.identity.as_bytes());
        }
        let identity = Qwen3RopeKvProfileCatalogIdentityV1(hash(CATALOG_DOMAIN, &canonical_bytes));
        Ok(Self {
            profiles: profiles.into_boxed_slice(),
            canonical_bytes: canonical_bytes.into_boxed_slice(),
            identity,
        })
    }

    /// Exact stable profile roster.
    #[must_use]
    pub fn profiles(&self) -> &[Qwen3RopeKvProfileV1] {
        &self.profiles
    }

    /// Finds one exact role/bucket/operation profile.
    #[must_use]
    pub fn profile(
        &self,
        bucket: Qwen3RopeKvBucketV1,
        operation: Qwen3RopeKvOperationV1,
    ) -> Option<Qwen3RopeKvProfileV1> {
        self.profiles
            .iter()
            .copied()
            .find(|profile| profile.bucket == bucket && profile.operation == operation)
    }

    /// Canonical bytes retaining every checked shape and grid.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    /// Exact catalog identity.
    #[must_use]
    pub const fn identity(&self) -> Qwen3RopeKvProfileCatalogIdentityV1 {
        self.identity
    }

    /// The catalog is structural and authenticates no source or artifact.
    #[must_use]
    pub const fn grants_authority(&self) -> bool {
        false
    }
}

const QWEN3_ROPE_KV_ROLES_V1: [Qwen3RopeKvModelRoleV1; 2] = [
    Qwen3RopeKvModelRoleV1::Target8B,
    Qwen3RopeKvModelRoleV1::Draft06B,
];

const QWEN3_ROPE_KV_BUCKET_KINDS_V1: [Qwen3RopeKvBucketKindV1; 11] = [
    Qwen3RopeKvBucketKindV1::PrefillS1T128,
    Qwen3RopeKvBucketKindV1::PrefillS8T128,
    Qwen3RopeKvBucketKindV1::PrefillS1T512,
    Qwen3RopeKvBucketKindV1::PrefillS1T2048,
    Qwen3RopeKvBucketKindV1::DecodeS1C8192,
    Qwen3RopeKvBucketKindV1::DecodeS8C8192,
    Qwen3RopeKvBucketKindV1::DecodeS32C8192,
    Qwen3RopeKvBucketKindV1::SpeculativeS1K4C8192,
    Qwen3RopeKvBucketKindV1::SpeculativeS8K4C8192,
    Qwen3RopeKvBucketKindV1::SpeculativeS1K8C8192,
    Qwen3RopeKvBucketKindV1::SpeculativeS1K16C8192,
];

const QWEN3_ROPE_KV_OPERATIONS_V1: [Qwen3RopeKvOperationV1; 2] = [
    Qwen3RopeKvOperationV1::Rope,
    Qwen3RopeKvOperationV1::PagedKvWrite,
];

/// One exact buffer role across the two operator ABIs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Qwen3RopeKvBufferV1 {
    /// BF16 normalized-query input.
    QueryInput = 0,
    /// BF16 normalized-key or rotated-key input.
    KeyInput = 1,
    /// BF16 value input.
    ValueInput = 2,
    /// U32 absolute position IDs.
    PositionIds = 3,
    /// Fixed FP32 cosine deployment table.
    CosTable = 4,
    /// Fixed FP32 sine deployment table.
    SinTable = 5,
    /// U32 per-sequence logical append starts.
    LogicalStarts = 6,
    /// U32 per-sequence physical page indices.
    PageIndices = 7,
    /// BF16 rotated-query output.
    QueryOutput = 8,
    /// BF16 rotated-key output or paged key cache.
    KeyOutputOrCache = 9,
    /// BF16 paged value cache.
    ValueCache = 10,
}

/// Numerical buffer-contract rejection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Qwen3RopeKvBufferContractErrorV1 {
    /// An operation did not use exact zero for an inactive pointer.
    InactiveAddress(Qwen3RopeKvBufferV1),
    /// An active address was zero.
    ZeroAddress(Qwen3RopeKvBufferV1),
    /// The available byte span differed from the exact selected profile.
    ByteLength(Qwen3RopeKvBufferV1),
    /// An address did not meet its element alignment.
    Alignment(Qwen3RopeKvBufferV1),
    /// A half-open range overflowed `u64`.
    RangeOverflow(Qwen3RopeKvBufferV1),
    /// Two declared buffer ranges overlap.
    Aliasing,
}

impl fmt::Display for Qwen3RopeKvBufferContractErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Qwen3 RoPE/KV buffer contract failed: {self:?}")
    }
}

impl std::error::Error for Qwen3RopeKvBufferContractErrorV1 {}

/// Inert checked byte ranges for one exact operator profile.
#[derive(Debug, Eq, PartialEq)]
pub struct Qwen3RopeKvBufferContractV1 {
    operation: Qwen3RopeKvOperationV1,
    addresses: [u64; 11],
    ends: [u64; 11],
    lengths: [u64; 11],
}

impl Qwen3RopeKvBufferContractV1 {
    /// Checks exact spans, element alignment, overflow, and pairwise disjointness.
    ///
    /// # Errors
    ///
    /// Returns an error for a wrong mode-specific length, a nonzero disabled
    /// buffer, misalignment, address-range overflow, or pairwise aliasing.
    pub fn checked(
        profile: Qwen3RopeKvProfileV1,
        addresses: [u64; 11],
        lengths: [u64; 11],
    ) -> Result<Self, Qwen3RopeKvBufferContractErrorV1> {
        let query_bytes = profile.query_elements.checked_mul(2).ok_or(
            Qwen3RopeKvBufferContractErrorV1::ByteLength(Qwen3RopeKvBufferV1::QueryInput),
        )?;
        let kv_bytes = profile.kv_elements.checked_mul(2).ok_or(
            Qwen3RopeKvBufferContractErrorV1::ByteLength(Qwen3RopeKvBufferV1::KeyInput),
        )?;
        let rows_bytes = u64::from(profile.base_rows).checked_mul(4).ok_or(
            Qwen3RopeKvBufferContractErrorV1::ByteLength(Qwen3RopeKvBufferV1::PositionIds),
        )?;
        let sequence_bytes = u64::from(profile.bucket.sequence_and_active_tokens()[0])
            .checked_mul(4)
            .ok_or(Qwen3RopeKvBufferContractErrorV1::ByteLength(
                Qwen3RopeKvBufferV1::LogicalStarts,
            ))?;
        let page_table_bytes = u64::from(profile.bucket.sequence_and_active_tokens()[0])
            .checked_mul(u64::from(QWEN3_KV_PAGE_TABLE_ENTRIES_V1))
            .and_then(|value| value.checked_mul(4))
            .ok_or(Qwen3RopeKvBufferContractErrorV1::ByteLength(
                Qwen3RopeKvBufferV1::PageIndices,
            ))?;
        let cache_bytes = QWEN3_KV_CACHE_BYTES_V1;
        let trig_bytes = QWEN3_ROPE_TRIG_TABLE_ELEMENTS_V1.checked_mul(4).ok_or(
            Qwen3RopeKvBufferContractErrorV1::ByteLength(Qwen3RopeKvBufferV1::CosTable),
        )?;
        let expected = match profile.operation {
            Qwen3RopeKvOperationV1::Rope => [
                query_bytes,
                kv_bytes,
                0,
                rows_bytes,
                trig_bytes,
                trig_bytes,
                0,
                0,
                query_bytes,
                kv_bytes,
                0,
            ],
            Qwen3RopeKvOperationV1::PagedKvWrite => [
                0,
                kv_bytes,
                kv_bytes,
                0,
                0,
                0,
                sequence_bytes,
                page_table_bytes,
                0,
                cache_bytes,
                cache_bytes,
            ],
        };
        let roles = [
            Qwen3RopeKvBufferV1::QueryInput,
            Qwen3RopeKvBufferV1::KeyInput,
            Qwen3RopeKvBufferV1::ValueInput,
            Qwen3RopeKvBufferV1::PositionIds,
            Qwen3RopeKvBufferV1::CosTable,
            Qwen3RopeKvBufferV1::SinTable,
            Qwen3RopeKvBufferV1::LogicalStarts,
            Qwen3RopeKvBufferV1::PageIndices,
            Qwen3RopeKvBufferV1::QueryOutput,
            Qwen3RopeKvBufferV1::KeyOutputOrCache,
            Qwen3RopeKvBufferV1::ValueCache,
        ];
        let alignments = [2, 2, 2, 4, 4, 4, 4, 4, 2, 2, 2];
        let mut ends = [0; 11];
        for index in 0..11 {
            if expected[index] == 0 && addresses[index] != 0 {
                return Err(Qwen3RopeKvBufferContractErrorV1::InactiveAddress(
                    roles[index],
                ));
            }
            if expected[index] != 0 && addresses[index] == 0 {
                return Err(Qwen3RopeKvBufferContractErrorV1::ZeroAddress(roles[index]));
            }
            if lengths[index] != expected[index] {
                return Err(Qwen3RopeKvBufferContractErrorV1::ByteLength(roles[index]));
            }
            if expected[index] != 0 && !addresses[index].is_multiple_of(alignments[index]) {
                return Err(Qwen3RopeKvBufferContractErrorV1::Alignment(roles[index]));
            }
            ends[index] = if expected[index] == 0 {
                0
            } else {
                addresses[index].checked_add(lengths[index]).ok_or(
                    Qwen3RopeKvBufferContractErrorV1::RangeOverflow(roles[index]),
                )?
            };
        }
        for left in 0..11 {
            for right in left + 1..11 {
                if expected[left] != 0
                    && expected[right] != 0
                    && addresses[left] < ends[right]
                    && addresses[right] < ends[left]
                {
                    return Err(Qwen3RopeKvBufferContractErrorV1::Aliasing);
                }
            }
        }
        Ok(Self {
            operation: profile.operation,
            addresses,
            ends,
            lengths,
        })
    }

    /// Exact operator selected by this host binding.
    #[must_use]
    pub const fn operation(&self) -> Qwen3RopeKvOperationV1 {
        self.operation
    }

    /// Exact starts in ABI role order.
    #[must_use]
    pub const fn addresses(&self) -> [u64; 11] {
        self.addresses
    }

    /// Exact exclusive ends in ABI role order.
    #[must_use]
    pub const fn ends(&self) -> [u64; 11] {
        self.ends
    }

    /// Exact byte lengths in ABI role order.
    #[must_use]
    pub const fn byte_lengths(&self) -> [u64; 11] {
        self.lengths
    }

    /// Integer checks do not authenticate mappings, leases, or contents.
    #[must_use]
    pub const fn authenticates_device_memory(&self) -> bool {
        false
    }

    /// A numerical layout grants no launch authority.
    #[must_use]
    pub const fn grants_launch_authority(&self) -> bool {
        false
    }
}

/// Four inert identities labeling compiler stages preceding this bounded graph.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Qwen3RopeKvSourceBindingsV1 {
    source: [u8; 32],
    semantic: [u8; 32],
    schedule: [u8; 32],
    target_plan: [u8; 32],
}

impl Qwen3RopeKvSourceBindingsV1 {
    /// Constructs inert labels. Preparation requires all four to be nonzero and distinct.
    #[must_use]
    pub const fn new(
        source: [u8; 32],
        semantic: [u8; 32],
        schedule: [u8; 32],
        target_plan: [u8; 32],
    ) -> Self {
        Self {
            source,
            semantic,
            schedule,
            target_plan,
        }
    }

    /// Caller labels authenticate no source, producer, compiler, or target provenance.
    #[must_use]
    pub const fn authenticates_provenance(self) -> bool {
        false
    }
}

/// Failure while preparing the typed BF16 RoPE/KV compiler handoff.
#[derive(Debug)]
pub enum PrepareQwen3RopeKvKernelErrorV1 {
    /// A source label was zero or reused for another role.
    SourceBindings,
    /// The finite profile catalog failed closed.
    Catalog(Qwen3RopeKvCatalogErrorV1),
    /// The Handoff V1 policy object rejected a field.
    HandoffV1(HandoffDiagnosticV1),
    /// The executable Handoff V2 graph rejected a field.
    HandoffV2(HandoffDiagnosticV2),
    /// Canonical LLVM serialization failed.
    Serialize(SerializeErrorV2),
    /// The typed graph did not survive exact serializer identity binding.
    SourceIdentity,
    /// Exact LLVM/LLD Worker policy admission failed.
    WorkerAdmission(WorkerAdmissionErrorV2),
    /// The compiler-FFI envelope failed closed.
    CompilerEnvelope(CompilerFfiEnvelopeError),
    /// The closed two-entry/two-descriptor manifest failed closed.
    SymbolManifest(CompilerModuleSymbolManifestErrorV1),
    /// The Handoff V2 compiler module failed closed.
    CompilerHandoff(CompilerModuleHandoffErrorV2),
}

impl fmt::Display for PrepareQwen3RopeKvKernelErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Qwen3 RoPE/KV preparation failed: {self:?}")
    }
}

impl std::error::Error for PrepareQwen3RopeKvKernelErrorV1 {}

/// Linear prepared typed graph and canonical Handoff V2 compiler module.
pub struct PreparedQwen3RopeKvKernelV1 {
    catalog: Qwen3RopeKvProfileCatalogV1,
    source_identity: HandoffIdentityV2,
    worker_admission_identity: WorkerAdmissionIdentityV2,
    assembly: Gfx942LlvmAssemblyV2,
    compiler_handoff_identity: CompilerModuleHandoffIdentityV2,
    manifest_identity: CompilerModuleSymbolManifestIdentityV1,
    compiler_handoff: CompilerModuleHandoffV2,
}

impl fmt::Debug for PreparedQwen3RopeKvKernelV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedQwen3RopeKvKernelV1")
            .field("catalog", &self.catalog.identity)
            .field("source_identity", &self.source_identity)
            .field("worker_admission", &self.worker_admission_identity)
            .field("assembly_sha256", &self.assembly.sha256())
            .field("compiler_handoff", &self.compiler_handoff_identity)
            .finish_non_exhaustive()
    }
}

impl PreparedQwen3RopeKvKernelV1 {
    /// Complete exact profile catalog retained with this graph owner.
    #[must_use]
    pub const fn catalog(&self) -> &Qwen3RopeKvProfileCatalogV1 {
        &self.catalog
    }

    /// Identity of the complete typed Handoff V2 graph.
    #[must_use]
    pub const fn source_identity(&self) -> HandoffIdentityV2 {
        self.source_identity
    }

    /// Identity binding the typed graph to exact LLVM/LLD policy admission.
    #[must_use]
    pub const fn worker_admission_identity(&self) -> WorkerAdmissionIdentityV2 {
        self.worker_admission_identity
    }

    /// SHA-256 of the exact canonical LLVM assembly.
    #[must_use]
    pub const fn assembly_sha256(&self) -> fe2o3_llvm_text::LlvmAssemblySha256V2 {
        self.assembly.sha256()
    }

    /// Exact canonical LLVM assembly byte length.
    #[must_use]
    pub fn assembly_len(&self) -> u64 {
        self.assembly.as_bytes().len() as u64
    }

    /// Complete canonical compiler handoff identity.
    #[must_use]
    pub const fn compiler_handoff_identity(&self) -> CompilerModuleHandoffIdentityV2 {
        self.compiler_handoff_identity
    }

    /// Closed entry/descriptor symbol manifest identity.
    #[must_use]
    pub const fn manifest_identity(&self) -> CompilerModuleSymbolManifestIdentityV1 {
        self.manifest_identity
    }

    /// Borrows the exact Handoff V2 compiler module for attempt-scoped publication.
    #[must_use]
    pub const fn compiler_handoff(&self) -> &CompilerModuleHandoffV2 {
        &self.compiler_handoff
    }

    /// Current Pliron lowering cannot represent scalar BF16 and was not used.
    #[must_use]
    pub const fn uses_pliron_lowering(&self) -> bool {
        false
    }

    /// Typed graph construction is not a source-to-LLVM refinement proof.
    #[must_use]
    pub const fn proves_machine_refinement(&self) -> bool {
        false
    }

    /// Preparation does not establish RoPE/KV numerical correctness.
    #[must_use]
    pub const fn proves_numerical_contract(&self) -> bool {
        false
    }

    /// Typed declarations do not establish `RoPE` operator refinement.
    #[must_use]
    pub const fn proves_operator_refinement(&self) -> bool {
        false
    }

    /// Typed address arithmetic does not establish Ferric physical-KV refinement.
    #[must_use]
    pub const fn proves_kv_refinement(&self) -> bool {
        false
    }

    /// The fixed table extent does not authenticate theta, values, or provenance.
    #[must_use]
    pub const fn authenticates_trig_table(&self) -> bool {
        false
    }

    /// Exact profile selection is not yet joined to Ferric generated-plan identity.
    #[must_use]
    pub const fn has_ferric_plan_identity_join(&self) -> bool {
        false
    }

    /// This compiler slice does not close the separate schedule-catalog path.
    #[must_use]
    pub const fn has_kernel_schedule_catalog_join(&self) -> bool {
        false
    }

    /// Preparation does not establish that Worker V2 executed.
    #[must_use]
    pub const fn authenticates_worker_execution(&self) -> bool {
        false
    }

    /// Preparation grants no artifact, load, or launch authority.
    #[must_use]
    pub const fn grants_launch_authority(&self) -> bool {
        false
    }
}

/// Constructs the exact catalog and two runtime-parameterized typed graphs.
///
/// # Errors
///
/// Returns an error if source labels are zero or repeated, the finite catalog
/// is invalid, typed handoff construction or serialization fails, or the
/// resulting compiler/Worker identities do not bind exactly.
pub fn prepare_qwen3_rope_kv_kernel_v1(
    bindings: Qwen3RopeKvSourceBindingsV1,
) -> Result<PreparedQwen3RopeKvKernelV1, PrepareQwen3RopeKvKernelErrorV1> {
    validate_source_bindings(bindings)?;
    let catalog = Qwen3RopeKvProfileCatalogV1::canonical()
        .map_err(PrepareQwen3RopeKvKernelErrorV1::Catalog)?;
    let handoff = construct_typed_handoff(&catalog, bindings)?;
    let source_identity = handoff.identity();
    let canonical = handoff.encode_canonical();
    let worker_admission = WorkerAdmissionRequestV2::new(
        canonical.as_bytes(),
        *source_identity.as_bytes(),
        MeasuredLlvmLldBuildV1::exact(),
    )
    .admit()
    .map_err(PrepareQwen3RopeKvKernelErrorV1::WorkerAdmission)?;
    if worker_admission.handoff() != &handoff
        || worker_admission.handoff_identity() != source_identity
    {
        return Err(PrepareQwen3RopeKvKernelErrorV1::SourceIdentity);
    }
    let worker_admission_identity = worker_admission.admission_identity();
    let assembly = serialize_gfx942_handoff_v2(worker_admission.handoff())
        .map_err(PrepareQwen3RopeKvKernelErrorV1::Serialize)?;
    if assembly.source_identity() != source_identity || !assembly.has_embedded_source_identity() {
        return Err(PrepareQwen3RopeKvKernelErrorV1::SourceIdentity);
    }
    let target = exact_target();
    let envelope =
        CompilerFfiEnvelopeV1::for_module_without_device_ffi(target, CodeObjectVersion::V6)
            .map_err(PrepareQwen3RopeKvKernelErrorV1::CompilerEnvelope)?;
    let manifest = CompilerModuleSymbolManifestV1::new([
        (
            CompilerModuleSymbolRoleV1::KernelEntry,
            QWEN3_PAGED_KV_WRITE_KERNEL_SYMBOL_V1,
        ),
        (
            CompilerModuleSymbolRoleV1::KernelEntry,
            QWEN3_ROPE_KERNEL_SYMBOL_V1,
        ),
        (
            CompilerModuleSymbolRoleV1::KernelDescriptor,
            QWEN3_PAGED_KV_WRITE_KERNEL_DESCRIPTOR_SYMBOL_V1,
        ),
        (
            CompilerModuleSymbolRoleV1::KernelDescriptor,
            QWEN3_ROPE_KERNEL_DESCRIPTOR_SYMBOL_V1,
        ),
    ])
    .map_err(PrepareQwen3RopeKvKernelErrorV1::SymbolManifest)?;
    let manifest_identity = manifest.identity();
    let compiler_handoff = CompilerModuleHandoffV2::new(
        CompilerModuleKindV1::LlvmTextIr,
        target,
        CodeObjectVersion::V6,
        envelope,
        manifest,
        assembly.as_bytes(),
    )
    .map_err(PrepareQwen3RopeKvKernelErrorV1::CompilerHandoff)?;
    let compiler_handoff_identity = compiler_handoff.identity();
    Ok(PreparedQwen3RopeKvKernelV1 {
        catalog,
        source_identity,
        worker_admission_identity,
        assembly,
        compiler_handoff_identity,
        manifest_identity,
        compiler_handoff,
    })
}

fn validate_source_bindings(
    bindings: Qwen3RopeKvSourceBindingsV1,
) -> Result<(), PrepareQwen3RopeKvKernelErrorV1> {
    let identities = [
        bindings.source,
        bindings.semantic,
        bindings.schedule,
        bindings.target_plan,
    ];
    for (index, identity) in identities.iter().enumerate() {
        if identity == &[0; 32] || identities[index + 1..].contains(identity) {
            return Err(PrepareQwen3RopeKvKernelErrorV1::SourceBindings);
        }
    }
    Ok(())
}

fn construct_typed_handoff(
    catalog: &Qwen3RopeKvProfileCatalogV1,
    bindings: Qwen3RopeKvSourceBindingsV1,
) -> Result<Gfx942HandoffV2, PrepareQwen3RopeKvKernelErrorV1> {
    let source_bytes = bound_stage_identity(SOURCE_DOMAIN, bindings.source, catalog.identity);
    let semantic_bytes = bound_stage_identity(SEMANTIC_DOMAIN, bindings.semantic, catalog.identity);
    let schedule_bytes = bound_stage_identity(SCHEDULE_DOMAIN, bindings.schedule, catalog.identity);
    let target_plan_bytes =
        bound_stage_identity(TARGET_PLAN_DOMAIN, bindings.target_plan, catalog.identity);
    let source =
        IdentityV1::new(source_bytes).map_err(PrepareQwen3RopeKvKernelErrorV1::HandoffV1)?;
    let stages = StageIdentitiesV1::new(semantic_bytes, schedule_bytes, target_plan_bytes)
        .map_err(PrepareQwen3RopeKvKernelErrorV1::HandoffV1)?;
    let origin = OriginV1::new(OriginKindV1::KernelIr, source, None);
    let kernel_attributes = exact_function_attributes();
    let rope_kernel = KernelEntryV1::new(
        QWEN3_ROPE_KERNEL_SYMBOL_V1,
        rope_kernel_parameters_v1().map_err(PrepareQwen3RopeKvKernelErrorV1::HandoffV1)?,
        kernel_attributes.clone(),
        origin.identity(),
    )
    .map_err(PrepareQwen3RopeKvKernelErrorV1::HandoffV1)?;
    let kv_kernel = KernelEntryV1::new(
        QWEN3_PAGED_KV_WRITE_KERNEL_SYMBOL_V1,
        kv_kernel_parameters_v1().map_err(PrepareQwen3RopeKvKernelErrorV1::HandoffV1)?,
        kernel_attributes.clone(),
        origin.identity(),
    )
    .map_err(PrepareQwen3RopeKvKernelErrorV1::HandoffV1)?;
    let obligations = REQUIRED_OBLIGATIONS
        .into_iter()
        .map(|kind| {
            let subject = match kind {
                ObligationKindV1::PreserveKernelAbi | ObligationKindV1::MaintainOriginCoverage => {
                    stages.semantic()
                }
                _ => stages.target_plan(),
            };
            ObligationV1::new(kind, subject, origin.identity())
        })
        .collect();
    let module_metadata = ModuleMetadataV1::new(
        vec![
            ModuleFlagV1::CodeObjectVersion6,
            ModuleFlagV1::PicLevel2,
            ModuleFlagV1::WcharSize4,
        ],
        vec![],
        vec![],
    )
    .map_err(PrepareQwen3RopeKvKernelErrorV1::HandoffV1)?;
    let base = Gfx942HandoffV1::new(Gfx942HandoffInputV1 {
        stage_identities: stages,
        target: Gfx942TargetPolicyV1::canonical(),
        kernels: vec![rope_kernel, kv_kernel],
        module: module_metadata,
        origins: vec![origin],
        obligations,
    })
    .map_err(PrepareQwen3RopeKvKernelErrorV1::HandoffV1)?;
    let evidence = EvidenceV2::new(
        base.origins()[0].identity(),
        base.obligations()
            .iter()
            .map(|obligation| obligation.identity())
            .collect(),
    )
    .map_err(PrepareQwen3RopeKvKernelErrorV1::HandoffV2)?;
    let rope_function = build_rope_kernel_function(kernel_attributes.clone(), evidence.clone())?;
    let kv_function = build_kv_kernel_function(kernel_attributes, evidence.clone())?;
    let intrinsics = [
        IntrinsicV2::AmdGpuWorkitemId(AxisV2::X),
        IntrinsicV2::AmdGpuWorkgroupId(AxisV2::X),
        IntrinsicV2::AmdGpuWorkgroupId(AxisV2::Y),
        IntrinsicV2::Trap,
    ]
    .into_iter()
    .map(|intrinsic| IntrinsicReferenceV2::new(intrinsic, evidence.clone()))
    .collect();
    let module = ExecutableModuleV2::new(
        base.module().flags().to_vec(),
        base.module().named_metadata().to_vec(),
        vec![],
        intrinsics,
        vec![rope_function, kv_function],
    )
    .map_err(PrepareQwen3RopeKvKernelErrorV1::HandoffV2)?;
    Gfx942HandoffV2::new(base, module).map_err(PrepareQwen3RopeKvKernelErrorV1::HandoffV2)
}

fn readonly_attributes(alignment: u16) -> Vec<ParameterAttributeV1> {
    vec![
        ParameterAttributeV1::NoAlias,
        ParameterAttributeV1::NoCapture,
        ParameterAttributeV1::NonNull,
        ParameterAttributeV1::ReadOnly,
        ParameterAttributeV1::Align(alignment),
    ]
}

fn writeonly_attributes(alignment: u16) -> Vec<ParameterAttributeV1> {
    vec![
        ParameterAttributeV1::NoAlias,
        ParameterAttributeV1::NoCapture,
        ParameterAttributeV1::NonNull,
        ParameterAttributeV1::WriteOnly,
        ParameterAttributeV1::Align(alignment),
    ]
}

fn pointer_type(scalar: ScalarTypeV1) -> KernelValueTypeV1 {
    KernelValueTypeV1::Pointer {
        pointee: scalar,
        address_space: AddressSpaceV1::Global,
    }
}

fn rope_kernel_parameters_v1() -> Result<Vec<KernelParameterV1>, HandoffDiagnosticV1> {
    let mut parameters = Vec::with_capacity(18);
    for (name, scalar, write) in [
        ("query_bf16", ScalarTypeV1::Bf16, false),
        ("key_bf16", ScalarTypeV1::Bf16, false),
        ("position_ids", ScalarTypeV1::I32, false),
        ("cos_table_f32", ScalarTypeV1::F32, false),
        ("sin_table_f32", ScalarTypeV1::F32, false),
        ("rotated_query_bf16", ScalarTypeV1::Bf16, true),
        ("rotated_key_bf16", ScalarTypeV1::Bf16, true),
    ] {
        parameters.push(KernelParameterV1::new(
            name,
            pointer_type(scalar),
            if write {
                writeonly_attributes(if scalar == ScalarTypeV1::Bf16 { 2 } else { 4 })
            } else {
                readonly_attributes(if scalar == ScalarTypeV1::Bf16 { 2 } else { 4 })
            },
        )?);
        let elements_name = format!("{name}_elements");
        parameters.push(KernelParameterV1::new(
            &elements_name,
            KernelValueTypeV1::Scalar(ScalarTypeV1::I64),
            vec![],
        )?);
    }
    for name in [
        "active_tokens",
        "sequences",
        "query_heads",
        "context_tokens",
    ] {
        parameters.push(KernelParameterV1::new(
            name,
            KernelValueTypeV1::Scalar(ScalarTypeV1::I32),
            vec![],
        )?);
    }
    Ok(parameters)
}

fn kv_kernel_parameters_v1() -> Result<Vec<KernelParameterV1>, HandoffDiagnosticV1> {
    let mut parameters = Vec::with_capacity(15);
    for (name, scalar, write) in [
        ("rotated_key_bf16", ScalarTypeV1::Bf16, false),
        ("value_bf16", ScalarTypeV1::Bf16, false),
        ("logical_starts", ScalarTypeV1::I32, false),
        ("page_indices", ScalarTypeV1::I32, false),
        ("key_cache_bf16", ScalarTypeV1::Bf16, true),
        ("value_cache_bf16", ScalarTypeV1::Bf16, true),
    ] {
        parameters.push(KernelParameterV1::new(
            name,
            pointer_type(scalar),
            if write {
                writeonly_attributes(if scalar == ScalarTypeV1::Bf16 { 2 } else { 4 })
            } else {
                readonly_attributes(if scalar == ScalarTypeV1::Bf16 { 2 } else { 4 })
            },
        )?);
        let elements_name = format!("{name}_elements");
        parameters.push(KernelParameterV1::new(
            &elements_name,
            KernelValueTypeV1::Scalar(ScalarTypeV1::I64),
            vec![],
        )?);
    }
    for name in ["active_tokens", "sequences", "context_tokens"] {
        parameters.push(KernelParameterV1::new(
            name,
            KernelValueTypeV1::Scalar(ScalarTypeV1::I32),
            vec![],
        )?);
    }
    Ok(parameters)
}

fn exact_function_attributes() -> Vec<FunctionAttributeV1> {
    FunctionAttributeV1::gfx942_kernel_defaults(
        WorkgroupSizeRangeV1::new(64, 64).expect("the fixed wave64 bound is valid"),
    )
}

struct TypedFunctionBuilder {
    evidence: EvidenceV2,
    current_id: BlockIdV2,
    current: Vec<InstructionV2>,
    blocks: Vec<BasicBlockV2>,
    next_block: u32,
    next_value: u32,
}

impl TypedFunctionBuilder {
    fn new(evidence: EvidenceV2, next_value: u32) -> Self {
        Self {
            evidence,
            current_id: BlockIdV2::new(0),
            current: Vec::new(),
            blocks: Vec::new(),
            next_block: 1,
            next_value,
        }
    }

    fn block(&mut self) -> BlockIdV2 {
        let id = BlockIdV2::new(self.next_block);
        self.next_block += 1;
        id
    }

    fn reserve(&mut self) -> ValueIdV2 {
        let id = ValueIdV2::new(self.next_value);
        self.next_value += 1;
        id
    }

    fn instruction(&mut self, value_type: ValueTypeV2, kind: InstructionKindV2) -> ValueIdV2 {
        let id = self.reserve();
        self.instruction_with(id, value_type, kind);
        id
    }

    fn instruction_with(
        &mut self,
        id: ValueIdV2,
        value_type: ValueTypeV2,
        kind: InstructionKindV2,
    ) {
        self.current.push(
            InstructionV2::new(
                Some(TypedValueV2::new(id, value_type)),
                kind,
                self.evidence.clone(),
            )
            .expect("closed RoPE/KV instruction shape is valid"),
        );
    }

    fn void(&mut self, kind: InstructionKindV2) {
        self.current.push(
            InstructionV2::new(None, kind, self.evidence.clone())
                .expect("closed RoPE/KV void instruction shape is valid"),
        );
    }

    fn finish(&mut self, terminator: TerminatorV2) {
        self.blocks.push(BasicBlockV2::new(
            self.current_id,
            core::mem::take(&mut self.current),
            terminator,
        ));
    }

    fn start(&mut self, block: BlockIdV2) {
        self.current_id = block;
    }

    fn constant(&mut self, scalar_type: ScalarTypeV1, bits: u64) -> ValueIdV2 {
        self.instruction(
            ValueTypeV2::Scalar(scalar_type),
            InstructionKindV2::Constant(
                ScalarConstantV2::new(scalar_type, bits)
                    .expect("closed RoPE/KV constants fit their scalar type"),
            ),
        )
    }

    fn integer(
        &mut self,
        operation: IntegerBinaryOperationV2,
        left: ValueIdV2,
        right: ValueIdV2,
        scalar_type: ScalarTypeV1,
    ) -> ValueIdV2 {
        self.instruction(
            ValueTypeV2::Scalar(scalar_type),
            InstructionKindV2::Binary {
                operation: BinaryOperationV2::Integer(operation),
                left,
                right,
            },
        )
    }

    fn float(
        &mut self,
        operation: FloatBinaryOperationV2,
        left: ValueIdV2,
        right: ValueIdV2,
    ) -> ValueIdV2 {
        self.instruction(
            ValueTypeV2::Scalar(ScalarTypeV1::F32),
            InstructionKindV2::Binary {
                operation: BinaryOperationV2::Float(operation),
                left,
                right,
            },
        )
    }

    fn compare(
        &mut self,
        predicate: ComparePredicateV2,
        left: ValueIdV2,
        right: ValueIdV2,
    ) -> ValueIdV2 {
        self.instruction(
            ValueTypeV2::Scalar(ScalarTypeV1::I1),
            InstructionKindV2::Compare {
                predicate,
                left,
                right,
            },
        )
    }

    fn cast(&mut self, operation: CastOperationV2, value: ValueIdV2, to: ValueTypeV2) -> ValueIdV2 {
        self.instruction(
            to,
            InstructionKindV2::Cast {
                operation,
                value,
                to,
            },
        )
    }

    fn and(&mut self, left: ValueIdV2, right: ValueIdV2) -> ValueIdV2 {
        self.integer(IntegerBinaryOperationV2::And, left, right, ScalarTypeV1::I1)
    }

    fn or(&mut self, left: ValueIdV2, right: ValueIdV2) -> ValueIdV2 {
        self.integer(IntegerBinaryOperationV2::Or, left, right, ScalarTypeV1::I1)
    }

    fn bf16_load(&mut self, base: ValueIdV2, index: ValueIdV2) -> ValueIdV2 {
        self.scalar_load(base, index, ScalarTypeV1::Bf16, 2)
    }

    fn scalar_load(
        &mut self,
        base: ValueIdV2,
        index: ValueIdV2,
        scalar_type: ScalarTypeV1,
        alignment: u16,
    ) -> ValueIdV2 {
        let pointer = self.instruction(
            ValueTypeV2::Pointer {
                pointee: scalar_type,
                address_space: AddressSpaceV1::Global,
            },
            InstructionKindV2::GetElementPtr {
                base,
                indices: vec![index],
            },
        );
        self.instruction(
            ValueTypeV2::Scalar(scalar_type),
            InstructionKindV2::Load {
                pointer,
                value_type: scalar_type,
                alignment,
            },
        )
    }

    fn bf16_store(&mut self, base: ValueIdV2, index: ValueIdV2, value: ValueIdV2) {
        self.scalar_store(base, index, value, ScalarTypeV1::Bf16, 2);
    }

    fn scalar_store(
        &mut self,
        base: ValueIdV2,
        index: ValueIdV2,
        value: ValueIdV2,
        scalar_type: ScalarTypeV1,
        alignment: u16,
    ) {
        let pointer = self.instruction(
            ValueTypeV2::Pointer {
                pointee: scalar_type,
                address_space: AddressSpaceV1::Global,
            },
            InstructionKindV2::GetElementPtr {
                base,
                indices: vec![index],
            },
        );
        self.void(InstructionKindV2::Store {
            pointer,
            value,
            value_type: scalar_type,
            alignment,
        });
    }
}

fn parameter(
    id: u32,
    value_type: ValueTypeV2,
    name: &str,
    attributes: Vec<ParameterAttributeV1>,
) -> Result<FunctionParameterV2, PrepareQwen3RopeKvKernelErrorV1> {
    FunctionParameterV2::new(
        TypedValueV2::new(ValueIdV2::new(id), value_type),
        name,
        attributes,
    )
    .map_err(PrepareQwen3RopeKvKernelErrorV1::HandoffV2)
}

fn executable_attributes(attributes: Vec<FunctionAttributeV1>) -> Vec<FunctionAttributeV2> {
    let mut result = attributes
        .into_iter()
        .map(FunctionAttributeV2::from)
        .collect::<Vec<_>>();
    result.push(FunctionAttributeV2::RequiredWorkgroupSize([64, 1, 1]));
    result
}

fn pointer_value_type(scalar: ScalarTypeV1) -> ValueTypeV2 {
    ValueTypeV2::Pointer {
        pointee: scalar,
        address_space: AddressSpaceV1::Global,
    }
}

fn eq_i32(builder: &mut TypedFunctionBuilder, value: ValueIdV2, expected: u32) -> ValueIdV2 {
    let expected = builder.constant(ScalarTypeV1::I32, u64::from(expected));
    builder.compare(ComparePredicateV2::IntegerEqual, value, expected)
}

fn all(builder: &mut TypedFunctionBuilder, conditions: &[ValueIdV2]) -> ValueIdV2 {
    let mut combined = conditions[0];
    for condition in &conditions[1..] {
        combined = builder.and(combined, *condition);
    }
    combined
}

fn any(builder: &mut TypedFunctionBuilder, conditions: &[ValueIdV2]) -> ValueIdV2 {
    let mut combined = conditions[0];
    for condition in &conditions[1..] {
        combined = builder.or(combined, *condition);
    }
    combined
}

fn bucket_case(
    builder: &mut TypedFunctionBuilder,
    active: ValueIdV2,
    sequences: ValueIdV2,
    context: ValueIdV2,
    expected: [u32; 3],
) -> ValueIdV2 {
    let active = eq_i32(builder, active, expected[0]);
    let sequences = eq_i32(builder, sequences, expected[1]);
    let context = eq_i32(builder, context, expected[2]);
    all(builder, &[active, sequences, context])
}

// KV ABI has no role tag or role-varying tensor width. Target/draft tuples with
// the same sequences/active/context values are deliberately machine-equivalent;
// exact role/profile identity remains in the host catalog and checked binding.
fn known_bucket_geometry(
    builder: &mut TypedFunctionBuilder,
    active: ValueIdV2,
    sequences: ValueIdV2,
    context: ValueIdV2,
) -> ValueIdV2 {
    let cases = [
        [128, 1, 128],
        [128, 8, 128],
        [512, 1, 512],
        [2_048, 1, 2_048],
        [1, 1, 8_192],
        [1, 8, 8_192],
        [1, 32, 8_192],
        [4, 1, 8_192],
        [5, 1, 8_192],
        [4, 8, 8_192],
        [5, 8, 8_192],
        [8, 1, 8_192],
        [9, 1, 8_192],
        [16, 1, 8_192],
        [17, 1, 8_192],
    ]
    .map(|expected| bucket_case(builder, active, sequences, context, expected));
    any(builder, &cases)
}

const fn rope_machine_geometry_is_known(
    active: u32,
    sequences: u32,
    query_heads: u32,
    context: u32,
) -> bool {
    const fn contains(roster: &[[u32; 3]], value: [u32; 3]) -> bool {
        let mut index = 0;
        while index < roster.len() {
            if roster[index][0] == value[0]
                && roster[index][1] == value[1]
                && roster[index][2] == value[2]
            {
                return true;
            }
            index += 1;
        }
        false
    }
    let geometry = [active, sequences, context];
    (contains(&ROPE_COMMON_GEOMETRY_V1, geometry) && matches!(query_heads, 16 | 32))
        || (contains(&ROPE_TARGET_SPECULATIVE_GEOMETRY_V1, geometry) && query_heads == 32)
        || (contains(&ROPE_DRAFT_SPECULATIVE_GEOMETRY_V1, geometry) && query_heads == 16)
}

const ROPE_COMMON_GEOMETRY_V1: [[u32; 3]; 7] = [
    [128, 1, 128],
    [128, 8, 128],
    [512, 1, 512],
    [2_048, 1, 2_048],
    [1, 1, 8_192],
    [1, 8, 8_192],
    [1, 32, 8_192],
];

const ROPE_TARGET_SPECULATIVE_GEOMETRY_V1: [[u32; 3]; 4] =
    [[5, 1, 8_192], [5, 8, 8_192], [9, 1, 8_192], [17, 1, 8_192]];

const ROPE_DRAFT_SPECULATIVE_GEOMETRY_V1: [[u32; 3]; 4] =
    [[4, 1, 8_192], [4, 8, 8_192], [8, 1, 8_192], [16, 1, 8_192]];

fn known_rope_geometry(
    builder: &mut TypedFunctionBuilder,
    active: ValueIdV2,
    sequences: ValueIdV2,
    query_heads: ValueIdV2,
    context: ValueIdV2,
) -> ValueIdV2 {
    let common_cases = ROPE_COMMON_GEOMETRY_V1
        .map(|expected| bucket_case(builder, active, sequences, context, expected));
    let common = any(builder, &common_cases);
    let target_heads = eq_i32(builder, query_heads, 32);
    let draft_heads = eq_i32(builder, query_heads, 16);
    let known_heads = builder.or(target_heads, draft_heads);
    let common = builder.and(common, known_heads);
    let target_cases = ROPE_TARGET_SPECULATIVE_GEOMETRY_V1
        .map(|expected| bucket_case(builder, active, sequences, context, expected));
    let target = any(builder, &target_cases);
    let target = builder.and(target, target_heads);
    let draft_cases = ROPE_DRAFT_SPECULATIVE_GEOMETRY_V1
        .map(|expected| bucket_case(builder, active, sequences, context, expected));
    let draft = any(builder, &draft_cases);
    let draft = builder.and(draft, draft_heads);
    let speculative = builder.or(target, draft);
    builder.or(common, speculative)
}

fn trap_unless(builder: &mut TypedFunctionBuilder, valid: ValueIdV2) {
    let zero = builder.constant(ScalarTypeV1::I1, 0);
    let invalid = builder.compare(ComparePredicateV2::IntegerEqual, valid, zero);
    let trap = builder.block();
    let proceed = builder.block();
    builder.finish(TerminatorV2::ConditionalBranch {
        condition: invalid,
        then_block: trap,
        else_block: proceed,
    });
    builder.start(trap);
    builder.void(InstructionKindV2::Call {
        target: fe2o3_llvm_handoff::CallTargetV2::Intrinsic(IntrinsicV2::Trap),
        arguments: vec![],
    });
    builder.finish(TerminatorV2::Unreachable);
    builder.start(proceed);
}

fn global_index(
    builder: &mut TypedFunctionBuilder,
    sequence: ValueIdV2,
    active_tokens: ValueIdV2,
    local_token: ValueIdV2,
) -> ValueIdV2 {
    let sequence_base = builder.integer(
        IntegerBinaryOperationV2::Multiply,
        sequence,
        active_tokens,
        ScalarTypeV1::I64,
    );
    builder.integer(
        IntegerBinaryOperationV2::Add,
        sequence_base,
        local_token,
        ScalarTypeV1::I64,
    )
}

struct RopeValues {
    query: ValueIdV2,
    query_len: ValueIdV2,
    key: ValueIdV2,
    key_len: ValueIdV2,
    positions: ValueIdV2,
    positions_len: ValueIdV2,
    cos: ValueIdV2,
    cos_len: ValueIdV2,
    sin: ValueIdV2,
    sin_len: ValueIdV2,
    query_output: ValueIdV2,
    query_output_len: ValueIdV2,
    key_output: ValueIdV2,
    key_output_len: ValueIdV2,
    active_tokens: ValueIdV2,
    sequences: ValueIdV2,
    query_heads: ValueIdV2,
    context: ValueIdV2,
}

impl RopeValues {
    const fn fixed() -> Self {
        Self {
            query: ValueIdV2::new(1),
            query_len: ValueIdV2::new(2),
            key: ValueIdV2::new(3),
            key_len: ValueIdV2::new(4),
            positions: ValueIdV2::new(5),
            positions_len: ValueIdV2::new(6),
            cos: ValueIdV2::new(7),
            cos_len: ValueIdV2::new(8),
            sin: ValueIdV2::new(9),
            sin_len: ValueIdV2::new(10),
            query_output: ValueIdV2::new(11),
            query_output_len: ValueIdV2::new(12),
            key_output: ValueIdV2::new(13),
            key_output_len: ValueIdV2::new(14),
            active_tokens: ValueIdV2::new(15),
            sequences: ValueIdV2::new(16),
            query_heads: ValueIdV2::new(17),
            context: ValueIdV2::new(18),
        }
    }
}

fn emit_rotary_pair(
    builder: &mut TypedFunctionBuilder,
    input: ValueIdV2,
    output: ValueIdV2,
    first: ValueIdV2,
    second: ValueIdV2,
    cos: ValueIdV2,
    sin: ValueIdV2,
) {
    let f32_type = ValueTypeV2::Scalar(ScalarTypeV1::F32);
    let bf16_type = ValueTypeV2::Scalar(ScalarTypeV1::Bf16);
    let first_bf16 = builder.bf16_load(input, first);
    let second_bf16 = builder.bf16_load(input, second);
    let first_f32 = builder.cast(CastOperationV2::FloatExtend, first_bf16, f32_type);
    let second_f32 = builder.cast(CastOperationV2::FloatExtend, second_bf16, f32_type);
    let first_cos = builder.float(FloatBinaryOperationV2::Multiply, first_f32, cos);
    let second_sin = builder.float(FloatBinaryOperationV2::Multiply, second_f32, sin);
    let rotated_first = builder.float(FloatBinaryOperationV2::Subtract, first_cos, second_sin);
    let second_cos = builder.float(FloatBinaryOperationV2::Multiply, second_f32, cos);
    let first_sin = builder.float(FloatBinaryOperationV2::Multiply, first_f32, sin);
    let rotated_second = builder.float(FloatBinaryOperationV2::Add, second_cos, first_sin);
    let rotated_first = builder.cast(CastOperationV2::FloatTruncate, rotated_first, bf16_type);
    let rotated_second = builder.cast(CastOperationV2::FloatTruncate, rotated_second, bf16_type);
    builder.bf16_store(output, first, rotated_first);
    builder.bf16_store(output, second, rotated_second);
}

fn emit_rotary_head_loop(
    builder: &mut TypedFunctionBuilder,
    input: ValueIdV2,
    output: ValueIdV2,
    token_index: ValueIdV2,
    head_limit: ValueIdV2,
    lane: ValueIdV2,
    trig: [ValueIdV2; 2],
) {
    let i64_type = ValueTypeV2::Scalar(ScalarTypeV1::I64);
    let zero = builder.constant(ScalarTypeV1::I64, 0);
    let one = builder.constant(ScalarTypeV1::I64, 1);
    let width = builder.constant(ScalarTypeV1::I64, 128);
    let half = builder.constant(ScalarTypeV1::I64, 64);
    let initial = builder.current_id;
    let header = builder.block();
    let body = builder.block();
    let backedge = builder.block();
    let complete = builder.block();
    let next_head = builder.reserve();
    builder.finish(TerminatorV2::Branch(header));
    builder.start(header);
    let head = builder.instruction(
        i64_type,
        InstructionKindV2::Phi {
            incoming: vec![(zero, initial), (next_head, backedge)],
        },
    );
    let active = builder.compare(ComparePredicateV2::UnsignedLessThan, head, head_limit);
    builder.finish(TerminatorV2::ConditionalBranch {
        condition: active,
        then_block: body,
        else_block: complete,
    });
    builder.start(body);
    let token_head = builder.integer(
        IntegerBinaryOperationV2::Multiply,
        token_index,
        head_limit,
        ScalarTypeV1::I64,
    );
    let token_head = builder.integer(
        IntegerBinaryOperationV2::Add,
        token_head,
        head,
        ScalarTypeV1::I64,
    );
    let head_base = builder.integer(
        IntegerBinaryOperationV2::Multiply,
        token_head,
        width,
        ScalarTypeV1::I64,
    );
    let first = builder.integer(
        IntegerBinaryOperationV2::Add,
        head_base,
        lane,
        ScalarTypeV1::I64,
    );
    let second = builder.integer(
        IntegerBinaryOperationV2::Add,
        first,
        half,
        ScalarTypeV1::I64,
    );
    emit_rotary_pair(builder, input, output, first, second, trig[0], trig[1]);
    builder.instruction_with(
        next_head,
        i64_type,
        InstructionKindV2::Binary {
            operation: BinaryOperationV2::Integer(IntegerBinaryOperationV2::Add),
            left: head,
            right: one,
        },
    );
    builder.finish(TerminatorV2::Branch(backedge));
    builder.start(backedge);
    builder.finish(TerminatorV2::Branch(header));
    builder.start(complete);
}

fn build_rope_kernel_function(
    attributes: Vec<FunctionAttributeV1>,
    evidence: EvidenceV2,
) -> Result<FunctionV2, PrepareQwen3RopeKvKernelErrorV1> {
    let values = RopeValues::fixed();
    let mut builder = TypedFunctionBuilder::new(evidence.clone(), 19);
    let i32_type = ValueTypeV2::Scalar(ScalarTypeV1::I32);
    let i64_type = ValueTypeV2::Scalar(ScalarTypeV1::I64);
    let known_geometry = known_rope_geometry(
        &mut builder,
        values.active_tokens,
        values.sequences,
        values.query_heads,
        values.context,
    );
    let active64 = builder.cast(CastOperationV2::ZeroExtend, values.active_tokens, i64_type);
    let sequences64 = builder.cast(CastOperationV2::ZeroExtend, values.sequences, i64_type);
    let base_rows = builder.integer(
        IntegerBinaryOperationV2::Multiply,
        active64,
        sequences64,
        ScalarTypeV1::I64,
    );
    let query_heads64 = builder.cast(CastOperationV2::ZeroExtend, values.query_heads, i64_type);
    let query_elements = builder.integer(
        IntegerBinaryOperationV2::Multiply,
        base_rows,
        query_heads64,
        ScalarTypeV1::I64,
    );
    let width = builder.constant(ScalarTypeV1::I64, 128);
    let query_elements = builder.integer(
        IntegerBinaryOperationV2::Multiply,
        query_elements,
        width,
        ScalarTypeV1::I64,
    );
    let kv_width = builder.constant(ScalarTypeV1::I64, 1_024);
    let kv_elements = builder.integer(
        IntegerBinaryOperationV2::Multiply,
        base_rows,
        kv_width,
        ScalarTypeV1::I64,
    );
    let trig_elements = builder.constant(ScalarTypeV1::I64, QWEN3_ROPE_TRIG_TABLE_ELEMENTS_V1);
    let lengths = [
        builder.compare(
            ComparePredicateV2::IntegerEqual,
            values.query_len,
            query_elements,
        ),
        builder.compare(
            ComparePredicateV2::IntegerEqual,
            values.key_len,
            kv_elements,
        ),
        builder.compare(
            ComparePredicateV2::IntegerEqual,
            values.positions_len,
            base_rows,
        ),
        builder.compare(
            ComparePredicateV2::IntegerEqual,
            values.cos_len,
            trig_elements,
        ),
        builder.compare(
            ComparePredicateV2::IntegerEqual,
            values.sin_len,
            trig_elements,
        ),
        builder.compare(
            ComparePredicateV2::IntegerEqual,
            values.query_output_len,
            query_elements,
        ),
        builder.compare(
            ComparePredicateV2::IntegerEqual,
            values.key_output_len,
            kv_elements,
        ),
    ];
    let lengths = all(&mut builder, &lengths);
    let geometry = builder.and(known_geometry, lengths);
    trap_unless(&mut builder, geometry);

    let lane = builder.instruction(
        i32_type,
        InstructionKindV2::Call {
            target: fe2o3_llvm_handoff::CallTargetV2::Intrinsic(IntrinsicV2::AmdGpuWorkitemId(
                AxisV2::X,
            )),
            arguments: vec![],
        },
    );
    let local_token = builder.instruction(
        i32_type,
        InstructionKindV2::Call {
            target: fe2o3_llvm_handoff::CallTargetV2::Intrinsic(IntrinsicV2::AmdGpuWorkgroupId(
                AxisV2::X,
            )),
            arguments: vec![],
        },
    );
    let sequence = builder.instruction(
        i32_type,
        InstructionKindV2::Call {
            target: fe2o3_llvm_handoff::CallTargetV2::Intrinsic(IntrinsicV2::AmdGpuWorkgroupId(
                AxisV2::Y,
            )),
            arguments: vec![],
        },
    );
    let token_valid = builder.compare(
        ComparePredicateV2::UnsignedLessThan,
        local_token,
        values.active_tokens,
    );
    let sequence_valid = builder.compare(
        ComparePredicateV2::UnsignedLessThan,
        sequence,
        values.sequences,
    );
    let active = builder.and(token_valid, sequence_valid);
    let compute = builder.block();
    let complete = builder.block();
    builder.finish(TerminatorV2::ConditionalBranch {
        condition: active,
        then_block: compute,
        else_block: complete,
    });
    builder.start(complete);
    builder.finish(TerminatorV2::Return(None));
    builder.start(compute);
    let lane64 = builder.cast(CastOperationV2::ZeroExtend, lane, i64_type);
    let token64 = builder.cast(CastOperationV2::ZeroExtend, local_token, i64_type);
    let workgroup_y64 = builder.cast(CastOperationV2::ZeroExtend, sequence, i64_type);
    let token_index = global_index(&mut builder, workgroup_y64, active64, token64);
    let position = builder.scalar_load(values.positions, token_index, ScalarTypeV1::I32, 4);
    let below_context = builder.compare(
        ComparePredicateV2::UnsignedLessThan,
        position,
        values.context,
    );
    let max_context = builder.constant(ScalarTypeV1::I32, 8_192);
    let below_max = builder.compare(ComparePredicateV2::UnsignedLessThan, position, max_context);
    let position_valid = builder.and(below_context, below_max);
    trap_unless(&mut builder, position_valid);
    let position64 = builder.cast(CastOperationV2::ZeroExtend, position, i64_type);
    let pairs = builder.constant(ScalarTypeV1::I64, 64);
    let trig_index = builder.integer(
        IntegerBinaryOperationV2::Multiply,
        position64,
        pairs,
        ScalarTypeV1::I64,
    );
    let trig_index = builder.integer(
        IntegerBinaryOperationV2::Add,
        trig_index,
        lane64,
        ScalarTypeV1::I64,
    );
    let cos = builder.scalar_load(values.cos, trig_index, ScalarTypeV1::F32, 4);
    let sin = builder.scalar_load(values.sin, trig_index, ScalarTypeV1::F32, 4);
    emit_rotary_head_loop(
        &mut builder,
        values.query,
        values.query_output,
        token_index,
        query_heads64,
        lane64,
        [cos, sin],
    );
    let kv_heads = builder.constant(ScalarTypeV1::I64, 8);
    emit_rotary_head_loop(
        &mut builder,
        values.key,
        values.key_output,
        token_index,
        kv_heads,
        lane64,
        [cos, sin],
    );
    builder.finish(TerminatorV2::Return(None));

    let parameters = vec![
        parameter(
            1,
            pointer_value_type(ScalarTypeV1::Bf16),
            "query_bf16",
            readonly_attributes(2),
        )?,
        parameter(2, i64_type, "query_bf16_elements", vec![])?,
        parameter(
            3,
            pointer_value_type(ScalarTypeV1::Bf16),
            "key_bf16",
            readonly_attributes(2),
        )?,
        parameter(4, i64_type, "key_bf16_elements", vec![])?,
        parameter(
            5,
            pointer_value_type(ScalarTypeV1::I32),
            "position_ids",
            readonly_attributes(4),
        )?,
        parameter(6, i64_type, "position_ids_elements", vec![])?,
        parameter(
            7,
            pointer_value_type(ScalarTypeV1::F32),
            "cos_table_f32",
            readonly_attributes(4),
        )?,
        parameter(8, i64_type, "cos_table_f32_elements", vec![])?,
        parameter(
            9,
            pointer_value_type(ScalarTypeV1::F32),
            "sin_table_f32",
            readonly_attributes(4),
        )?,
        parameter(10, i64_type, "sin_table_f32_elements", vec![])?,
        parameter(
            11,
            pointer_value_type(ScalarTypeV1::Bf16),
            "rotated_query_bf16",
            writeonly_attributes(2),
        )?,
        parameter(12, i64_type, "rotated_query_bf16_elements", vec![])?,
        parameter(
            13,
            pointer_value_type(ScalarTypeV1::Bf16),
            "rotated_key_bf16",
            writeonly_attributes(2),
        )?,
        parameter(14, i64_type, "rotated_key_bf16_elements", vec![])?,
        parameter(15, i32_type, "active_tokens", vec![])?,
        parameter(16, i32_type, "sequences", vec![])?,
        parameter(17, i32_type, "query_heads", vec![])?,
        parameter(18, i32_type, "context_tokens", vec![])?,
    ];
    FunctionV2::new(
        FunctionIdV2::new(0),
        QWEN3_ROPE_KERNEL_SYMBOL_V1,
        FunctionKindV2::Kernel,
        CallingConventionV2::AmdGpuKernel,
        ReturnTypeV2::Void,
        parameters,
        executable_attributes(attributes),
        BlockIdV2::new(0),
        builder.blocks,
        evidence,
    )
    .map_err(PrepareQwen3RopeKvKernelErrorV1::HandoffV2)
}

struct KvValues {
    key: ValueIdV2,
    key_len: ValueIdV2,
    value: ValueIdV2,
    value_len: ValueIdV2,
    starts: ValueIdV2,
    starts_len: ValueIdV2,
    pages: ValueIdV2,
    pages_len: ValueIdV2,
    key_cache: ValueIdV2,
    key_cache_len: ValueIdV2,
    value_cache: ValueIdV2,
    value_cache_len: ValueIdV2,
    active_tokens: ValueIdV2,
    sequences: ValueIdV2,
    context: ValueIdV2,
}

impl KvValues {
    const fn fixed() -> Self {
        Self {
            key: ValueIdV2::new(1),
            key_len: ValueIdV2::new(2),
            value: ValueIdV2::new(3),
            value_len: ValueIdV2::new(4),
            starts: ValueIdV2::new(5),
            starts_len: ValueIdV2::new(6),
            pages: ValueIdV2::new(7),
            pages_len: ValueIdV2::new(8),
            key_cache: ValueIdV2::new(9),
            key_cache_len: ValueIdV2::new(10),
            value_cache: ValueIdV2::new(11),
            value_cache_len: ValueIdV2::new(12),
            active_tokens: ValueIdV2::new(13),
            sequences: ValueIdV2::new(14),
            context: ValueIdV2::new(15),
        }
    }
}

fn emit_kv_component(
    builder: &mut TypedFunctionBuilder,
    values: &KvValues,
    token_index: ValueIdV2,
    cache_token: ValueIdV2,
    head: ValueIdV2,
    component: ValueIdV2,
) {
    let width = builder.constant(ScalarTypeV1::I64, 128);
    let heads = builder.constant(ScalarTypeV1::I64, 8);
    let input_head = builder.integer(
        IntegerBinaryOperationV2::Multiply,
        token_index,
        heads,
        ScalarTypeV1::I64,
    );
    let input_head = builder.integer(
        IntegerBinaryOperationV2::Add,
        input_head,
        head,
        ScalarTypeV1::I64,
    );
    let input_base = builder.integer(
        IntegerBinaryOperationV2::Multiply,
        input_head,
        width,
        ScalarTypeV1::I64,
    );
    let input_index = builder.integer(
        IntegerBinaryOperationV2::Add,
        input_base,
        component,
        ScalarTypeV1::I64,
    );
    let cache_head = builder.integer(
        IntegerBinaryOperationV2::Multiply,
        cache_token,
        heads,
        ScalarTypeV1::I64,
    );
    let cache_head = builder.integer(
        IntegerBinaryOperationV2::Add,
        cache_head,
        head,
        ScalarTypeV1::I64,
    );
    let cache_base = builder.integer(
        IntegerBinaryOperationV2::Multiply,
        cache_head,
        width,
        ScalarTypeV1::I64,
    );
    let cache_index = builder.integer(
        IntegerBinaryOperationV2::Add,
        cache_base,
        component,
        ScalarTypeV1::I64,
    );
    let key = builder.bf16_load(values.key, input_index);
    let value = builder.bf16_load(values.value, input_index);
    builder.bf16_store(values.key_cache, cache_index, key);
    builder.bf16_store(values.value_cache, cache_index, value);
}

fn build_kv_kernel_function(
    attributes: Vec<FunctionAttributeV1>,
    evidence: EvidenceV2,
) -> Result<FunctionV2, PrepareQwen3RopeKvKernelErrorV1> {
    let values = KvValues::fixed();
    let mut builder = TypedFunctionBuilder::new(evidence.clone(), 16);
    let i32_type = ValueTypeV2::Scalar(ScalarTypeV1::I32);
    let i64_type = ValueTypeV2::Scalar(ScalarTypeV1::I64);
    let known_bucket = known_bucket_geometry(
        &mut builder,
        values.active_tokens,
        values.sequences,
        values.context,
    );
    let active64 = builder.cast(CastOperationV2::ZeroExtend, values.active_tokens, i64_type);
    let sequences64 = builder.cast(CastOperationV2::ZeroExtend, values.sequences, i64_type);
    let base_rows = builder.integer(
        IntegerBinaryOperationV2::Multiply,
        active64,
        sequences64,
        ScalarTypeV1::I64,
    );
    let kv_width = builder.constant(ScalarTypeV1::I64, 1_024);
    let kv_elements = builder.integer(
        IntegerBinaryOperationV2::Multiply,
        base_rows,
        kv_width,
        ScalarTypeV1::I64,
    );
    let pages_per_sequence =
        builder.constant(ScalarTypeV1::I64, u64::from(QWEN3_KV_PAGE_TABLE_ENTRIES_V1));
    let pages_elements = builder.integer(
        IntegerBinaryOperationV2::Multiply,
        sequences64,
        pages_per_sequence,
        ScalarTypeV1::I64,
    );
    let cache_elements = builder.constant(ScalarTypeV1::I64, QWEN3_KV_CACHE_ELEMENTS_V1);
    let lengths = [
        builder.compare(
            ComparePredicateV2::IntegerEqual,
            values.key_len,
            kv_elements,
        ),
        builder.compare(
            ComparePredicateV2::IntegerEqual,
            values.value_len,
            kv_elements,
        ),
        builder.compare(
            ComparePredicateV2::IntegerEqual,
            values.starts_len,
            sequences64,
        ),
        builder.compare(
            ComparePredicateV2::IntegerEqual,
            values.pages_len,
            pages_elements,
        ),
        builder.compare(
            ComparePredicateV2::IntegerEqual,
            values.key_cache_len,
            cache_elements,
        ),
        builder.compare(
            ComparePredicateV2::IntegerEqual,
            values.value_cache_len,
            cache_elements,
        ),
    ];
    let lengths = all(&mut builder, &lengths);
    let geometry = builder.and(known_bucket, lengths);
    trap_unless(&mut builder, geometry);

    let lane = builder.instruction(
        i32_type,
        InstructionKindV2::Call {
            target: fe2o3_llvm_handoff::CallTargetV2::Intrinsic(IntrinsicV2::AmdGpuWorkitemId(
                AxisV2::X,
            )),
            arguments: vec![],
        },
    );
    let local_token = builder.instruction(
        i32_type,
        InstructionKindV2::Call {
            target: fe2o3_llvm_handoff::CallTargetV2::Intrinsic(IntrinsicV2::AmdGpuWorkgroupId(
                AxisV2::X,
            )),
            arguments: vec![],
        },
    );
    let sequence = builder.instruction(
        i32_type,
        InstructionKindV2::Call {
            target: fe2o3_llvm_handoff::CallTargetV2::Intrinsic(IntrinsicV2::AmdGpuWorkgroupId(
                AxisV2::Y,
            )),
            arguments: vec![],
        },
    );
    let token_valid = builder.compare(
        ComparePredicateV2::UnsignedLessThan,
        local_token,
        values.active_tokens,
    );
    let sequence_valid = builder.compare(
        ComparePredicateV2::UnsignedLessThan,
        sequence,
        values.sequences,
    );
    let active = builder.and(token_valid, sequence_valid);
    let compute = builder.block();
    let complete = builder.block();
    builder.finish(TerminatorV2::ConditionalBranch {
        condition: active,
        then_block: compute,
        else_block: complete,
    });
    builder.start(complete);
    builder.finish(TerminatorV2::Return(None));
    builder.start(compute);
    let lane64 = builder.cast(CastOperationV2::ZeroExtend, lane, i64_type);
    let token64 = builder.cast(CastOperationV2::ZeroExtend, local_token, i64_type);
    let workgroup_y64 = builder.cast(CastOperationV2::ZeroExtend, sequence, i64_type);
    let token_index = global_index(&mut builder, workgroup_y64, active64, token64);
    let logical_start = builder.scalar_load(values.starts, workgroup_y64, ScalarTypeV1::I32, 4);
    let max_start = builder.constant(ScalarTypeV1::I32, 8_192);
    let start_valid = builder.compare(
        ComparePredicateV2::UnsignedLessThan,
        logical_start,
        max_start,
    );
    trap_unless(&mut builder, start_valid);
    let logical_position = builder.integer(
        IntegerBinaryOperationV2::Add,
        logical_start,
        local_token,
        ScalarTypeV1::I32,
    );
    let below_context = builder.compare(
        ComparePredicateV2::UnsignedLessThan,
        logical_position,
        values.context,
    );
    let below_max = builder.compare(
        ComparePredicateV2::UnsignedLessThan,
        logical_position,
        max_start,
    );
    let logical_valid = builder.and(below_context, below_max);
    trap_unless(&mut builder, logical_valid);
    let shift = builder.constant(ScalarTypeV1::I32, 4);
    let mask = builder.constant(ScalarTypeV1::I32, 15);
    let logical_page = builder.integer(
        IntegerBinaryOperationV2::LogicalShiftRight,
        logical_position,
        shift,
        ScalarTypeV1::I32,
    );
    let token_in_page = builder.integer(
        IntegerBinaryOperationV2::And,
        logical_position,
        mask,
        ScalarTypeV1::I32,
    );
    let logical_page_limit =
        builder.constant(ScalarTypeV1::I32, u64::from(QWEN3_KV_PAGE_TABLE_ENTRIES_V1));
    let logical_page_valid = builder.compare(
        ComparePredicateV2::UnsignedLessThan,
        logical_page,
        logical_page_limit,
    );
    let page_token_limit = builder.constant(ScalarTypeV1::I32, u64::from(QWEN3_KV_PAGE_TOKENS_V1));
    let page_offset_valid = builder.compare(
        ComparePredicateV2::UnsignedLessThan,
        token_in_page,
        page_token_limit,
    );
    let translation_valid = builder.and(logical_page_valid, page_offset_valid);
    trap_unless(&mut builder, translation_valid);
    let table_sequence = builder.integer(
        IntegerBinaryOperationV2::Multiply,
        workgroup_y64,
        pages_per_sequence,
        ScalarTypeV1::I64,
    );
    let logical_page64 = builder.cast(CastOperationV2::ZeroExtend, logical_page, i64_type);
    let table_index = builder.integer(
        IntegerBinaryOperationV2::Add,
        table_sequence,
        logical_page64,
        ScalarTypeV1::I64,
    );
    let physical_page = builder.scalar_load(values.pages, table_index, ScalarTypeV1::I32, 4);
    let physical_page_limit = builder.constant(
        ScalarTypeV1::I32,
        u64::from(QWEN3_KV_PHYSICAL_PAGE_SLOTS_V1),
    );
    let page_valid = builder.compare(
        ComparePredicateV2::UnsignedLessThan,
        physical_page,
        physical_page_limit,
    );
    trap_unless(&mut builder, page_valid);
    let physical64 = builder.cast(CastOperationV2::ZeroExtend, physical_page, i64_type);
    let page_tokens = builder.constant(ScalarTypeV1::I64, u64::from(QWEN3_KV_PAGE_TOKENS_V1));
    let cache_token = builder.integer(
        IntegerBinaryOperationV2::Multiply,
        physical64,
        page_tokens,
        ScalarTypeV1::I64,
    );
    let token_in_page64 = builder.cast(CastOperationV2::ZeroExtend, token_in_page, i64_type);
    let cache_token = builder.integer(
        IntegerBinaryOperationV2::Add,
        cache_token,
        token_in_page64,
        ScalarTypeV1::I64,
    );

    let zero = builder.constant(ScalarTypeV1::I64, 0);
    let one = builder.constant(ScalarTypeV1::I64, 1);
    let eight = builder.constant(ScalarTypeV1::I64, 8);
    let half = builder.constant(ScalarTypeV1::I64, 64);
    let initial = builder.current_id;
    let header = builder.block();
    let body = builder.block();
    let backedge = builder.block();
    let done = builder.block();
    let next_head = builder.reserve();
    builder.finish(TerminatorV2::Branch(header));
    builder.start(header);
    let head = builder.instruction(
        i64_type,
        InstructionKindV2::Phi {
            incoming: vec![(zero, initial), (next_head, backedge)],
        },
    );
    let head_active = builder.compare(ComparePredicateV2::UnsignedLessThan, head, eight);
    builder.finish(TerminatorV2::ConditionalBranch {
        condition: head_active,
        then_block: body,
        else_block: done,
    });
    builder.start(body);
    emit_kv_component(
        &mut builder,
        &values,
        token_index,
        cache_token,
        head,
        lane64,
    );
    let upper_component = builder.integer(
        IntegerBinaryOperationV2::Add,
        lane64,
        half,
        ScalarTypeV1::I64,
    );
    emit_kv_component(
        &mut builder,
        &values,
        token_index,
        cache_token,
        head,
        upper_component,
    );
    builder.instruction_with(
        next_head,
        i64_type,
        InstructionKindV2::Binary {
            operation: BinaryOperationV2::Integer(IntegerBinaryOperationV2::Add),
            left: head,
            right: one,
        },
    );
    builder.finish(TerminatorV2::Branch(backedge));
    builder.start(backedge);
    builder.finish(TerminatorV2::Branch(header));
    builder.start(done);
    builder.finish(TerminatorV2::Return(None));

    let parameters = vec![
        parameter(
            1,
            pointer_value_type(ScalarTypeV1::Bf16),
            "rotated_key_bf16",
            readonly_attributes(2),
        )?,
        parameter(2, i64_type, "rotated_key_bf16_elements", vec![])?,
        parameter(
            3,
            pointer_value_type(ScalarTypeV1::Bf16),
            "value_bf16",
            readonly_attributes(2),
        )?,
        parameter(4, i64_type, "value_bf16_elements", vec![])?,
        parameter(
            5,
            pointer_value_type(ScalarTypeV1::I32),
            "logical_starts",
            readonly_attributes(4),
        )?,
        parameter(6, i64_type, "logical_starts_elements", vec![])?,
        parameter(
            7,
            pointer_value_type(ScalarTypeV1::I32),
            "page_indices",
            readonly_attributes(4),
        )?,
        parameter(8, i64_type, "page_indices_elements", vec![])?,
        parameter(
            9,
            pointer_value_type(ScalarTypeV1::Bf16),
            "key_cache_bf16",
            writeonly_attributes(2),
        )?,
        parameter(10, i64_type, "key_cache_bf16_elements", vec![])?,
        parameter(
            11,
            pointer_value_type(ScalarTypeV1::Bf16),
            "value_cache_bf16",
            writeonly_attributes(2),
        )?,
        parameter(12, i64_type, "value_cache_bf16_elements", vec![])?,
        parameter(13, i32_type, "active_tokens", vec![])?,
        parameter(14, i32_type, "sequences", vec![])?,
        parameter(15, i32_type, "context_tokens", vec![])?,
    ];
    FunctionV2::new(
        FunctionIdV2::new(1),
        QWEN3_PAGED_KV_WRITE_KERNEL_SYMBOL_V1,
        FunctionKindV2::Kernel,
        CallingConventionV2::AmdGpuKernel,
        ReturnTypeV2::Void,
        parameters,
        executable_attributes(attributes),
        BlockIdV2::new(0),
        builder.blocks,
        evidence,
    )
    .map_err(PrepareQwen3RopeKvKernelErrorV1::HandoffV2)
}

/// Linear exact typed compiler handoff awaiting attempt-scoped Worker V2 execution.
pub struct InertQwen3RopeKvWorkerRequestV1 {
    prepared: PreparedQwen3RopeKvKernelV1,
}

impl fmt::Debug for InertQwen3RopeKvWorkerRequestV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InertQwen3RopeKvWorkerRequestV1")
            .field("catalog", &self.prepared.catalog.identity)
            .field("source", &self.prepared.source_identity)
            .field("handoff", &self.prepared.compiler_handoff_identity)
            .finish_non_exhaustive()
    }
}

impl InertQwen3RopeKvWorkerRequestV1 {
    /// Complete profile catalog retained by this request.
    #[must_use]
    pub const fn catalog(&self) -> &Qwen3RopeKvProfileCatalogV1 {
        &self.prepared.catalog
    }

    /// Exact compiler handoff for attempt-scoped transaction publication.
    #[must_use]
    pub const fn compiler_handoff(&self) -> &CompilerModuleHandoffV2 {
        &self.prepared.compiler_handoff
    }

    /// Typed graph identity retained by the compiler handoff.
    #[must_use]
    pub const fn source_identity(&self) -> HandoffIdentityV2 {
        self.prepared.source_identity
    }

    /// A request value does not establish Worker V2 execution or artifact existence.
    #[must_use]
    pub const fn authenticates_worker_execution(&self) -> bool {
        false
    }

    /// A compiler handoff grants no artifact, load, or launch authority.
    #[must_use]
    pub const fn grants_launch_authority(&self) -> bool {
        false
    }
}

/// Consumes a prepared owner into the exact Worker V2 request stage.
#[must_use]
pub const fn lower_qwen3_rope_kv_kernel_v1(
    prepared: PreparedQwen3RopeKvKernelV1,
) -> InertQwen3RopeKvWorkerRequestV1 {
    InertQwen3RopeKvWorkerRequestV1 { prepared }
}

/// Failure while executing the exact Handoff V2 module through Worker V2.
#[derive(Debug)]
pub enum ExecuteQwen3RopeKvWorkerErrorV1 {
    /// Consumed attempt bytes differ from the exact prepared handoff.
    HandoffSubstitution,
    /// A fixed link option could not be represented.
    FixedLinkOption,
    /// The fixed HSACO output ceiling could not be represented.
    OutputConstraint(WorkerProtocolError),
    /// Reproducible bootstrap and exact replay failed.
    FirstBuild(FirstBuildWorkerV2Error),
}

impl fmt::Display for ExecuteQwen3RopeKvWorkerErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Qwen3 RoPE/KV Worker V2 execution failed: {self:?}"
        )
    }
}

impl std::error::Error for ExecuteQwen3RopeKvWorkerErrorV1 {}

/// Linear exact Worker V2 bootstrap/replay evidence awaiting structural inspection.
pub struct InertQwen3RopeKvWorkerEvidenceV1 {
    prepared: PreparedQwen3RopeKvKernelV1,
    transaction_handoff: CompilerModuleHandoffIdentityV1,
    worker: InertFirstBuildWorkerV2EvidenceV1,
}

impl fmt::Debug for InertQwen3RopeKvWorkerEvidenceV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InertQwen3RopeKvWorkerEvidenceV1")
            .field("source", &self.prepared.source_identity)
            .field("transaction_handoff", &self.transaction_handoff)
            .field("worker", &self.worker.identity())
            .finish_non_exhaustive()
    }
}

impl InertQwen3RopeKvWorkerEvidenceV1 {
    /// Reproducible execution remains inert until exact structural inspection.
    #[must_use]
    pub const fn grants_artifact_authority(&self) -> bool {
        false
    }

    /// Worker execution alone does not prove numerical or hardware behavior.
    #[must_use]
    pub const fn proves_numerical_contract(&self) -> bool {
        false
    }

    /// Worker output does not establish `RoPE` operator refinement.
    #[must_use]
    pub const fn proves_operator_refinement(&self) -> bool {
        false
    }

    /// Worker output does not establish Ferric physical-KV refinement.
    #[must_use]
    pub const fn proves_kv_refinement(&self) -> bool {
        false
    }
}

/// Executes the exact transaction handoff through Worker V2 bootstrap and replay.
///
/// # Errors
///
/// Returns an error if linear handoff custody is substituted, fixed Worker
/// inputs cannot be formed, Worker V2 fails, or the returned transaction and
/// Worker identities do not match the request.
pub fn execute_qwen3_rope_kv_worker_v2_v1(
    request: InertQwen3RopeKvWorkerRequestV1,
    consumed: ConsumedCompilerModuleHandoffV1,
    worker: &PinnedWorkerV1,
    limits: WorkerExecutionLimitsV1,
) -> Result<InertQwen3RopeKvWorkerEvidenceV1, ExecuteQwen3RopeKvWorkerErrorV1> {
    let InertQwen3RopeKvWorkerRequestV1 { prepared } = request;
    if consumed.bytes() != prepared.compiler_handoff.canonical_bytes() {
        return Err(ExecuteQwen3RopeKvWorkerErrorV1::HandoffSubstitution);
    }
    let transaction_handoff = consumed.identity();
    let worker_evidence = execute_reproducible_first_build_worker_v2(
        consumed,
        worker,
        Vec::new(),
        fixed_link_options()?,
        WorkerOutputConstraintsV1::new(MAX_HSACO_BYTES as u64)
            .map_err(ExecuteQwen3RopeKvWorkerErrorV1::OutputConstraint)?,
        limits,
    )
    .map_err(ExecuteQwen3RopeKvWorkerErrorV1::FirstBuild)?;
    Ok(InertQwen3RopeKvWorkerEvidenceV1 {
        prepared,
        transaction_handoff,
        worker: worker_evidence,
    })
}

/// Exact post-worker structural rejection.
#[derive(Debug)]
pub enum InspectQwen3RopeKvKernelErrorV1 {
    /// Worker request or response canonical bytes failed decoding.
    Protocol(WorkerProtocolError),
    /// Transaction, compiler module, manifest, worker, or output lineage drifted.
    SourceLineage,
    /// AMDHSA metadata or descriptor binding failed.
    Hsaco(KernelBindingError),
    /// Kernel inventory, ABI, or resource facts differ from the exact profile.
    KernelProfile,
    /// Strict allocation-free COV6 loader validation failed.
    Loader(PlanError),
}

impl fmt::Display for InspectQwen3RopeKvKernelErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Qwen3 RoPE/KV structural inspection failed: {self:?}"
        )
    }
}

impl std::error::Error for InspectQwen3RopeKvKernelErrorV1 {}

/// Linear exact Worker V2 output after strict ABI/resource and loader inspection.
pub struct InspectedQwen3RopeKvKernelV1 {
    catalog: Qwen3RopeKvProfileCatalogV1,
    source_identity: HandoffIdentityV2,
    compiler_handoff_identity: CompilerModuleHandoffIdentityV2,
    transaction_handoff: CompilerModuleHandoffIdentityV1,
    loader_plan: LoadPlan,
    worker: InertFirstBuildWorkerV2EvidenceV1,
}

impl fmt::Debug for InspectedQwen3RopeKvKernelV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InspectedQwen3RopeKvKernelV1")
            .field("catalog", &self.catalog.identity)
            .field("source", &self.source_identity)
            .field("compiler_handoff", &self.compiler_handoff_identity)
            .field("transaction_handoff", &self.transaction_handoff)
            .field("worker", &self.worker.identity())
            .finish_non_exhaustive()
    }
}

impl InspectedQwen3RopeKvKernelV1 {
    /// Exact profile catalog retained with the inspected output owner.
    #[must_use]
    pub const fn catalog(&self) -> &Qwen3RopeKvProfileCatalogV1 {
        &self.catalog
    }

    /// Exact strict pure-Rust loader plan over the same Worker output bytes.
    #[must_use]
    pub const fn loader_plan(&self) -> &LoadPlan {
        &self.loader_plan
    }

    /// Exact bytes retained by sealed Worker V2 evidence.
    #[must_use]
    pub fn exact_worker_output_bytes(&self) -> &[u8] {
        self.worker.output_bytes()
    }

    /// Observed output bytes are not an independently approved deployment pin.
    #[must_use]
    pub const fn has_independent_deployment_pin(&self) -> bool {
        false
    }

    /// Structural inspection does not prove typed-source or machine refinement.
    #[must_use]
    pub const fn proves_machine_refinement(&self) -> bool {
        false
    }

    /// Structural inspection does not prove RoPE/KV numerical behavior.
    #[must_use]
    pub const fn proves_numerical_contract(&self) -> bool {
        false
    }

    /// Structural inspection does not establish `RoPE` operator refinement.
    #[must_use]
    pub const fn proves_operator_refinement(&self) -> bool {
        false
    }

    /// Structural inspection does not establish Ferric physical-KV refinement.
    #[must_use]
    pub const fn proves_kv_refinement(&self) -> bool {
        false
    }

    /// Structural inspection does not prove hardware execution.
    #[must_use]
    pub const fn proves_hardware_execution(&self) -> bool {
        false
    }

    /// No completion observation is represented by this inspected owner.
    #[must_use]
    pub const fn proves_completion(&self) -> bool {
        false
    }

    /// No performance measurement is represented by this inspected owner.
    #[must_use]
    pub const fn proves_performance(&self) -> bool {
        false
    }

    /// Structural inspection grants no load or launch authority.
    #[must_use]
    pub const fn grants_launch_authority(&self) -> bool {
        false
    }

    /// Binds one exact operation profile to its mode-specific numerical buffers.
    ///
    /// # Errors
    ///
    /// Returns an error if the profile is absent, any exact buffer range is
    /// invalid, or the host-only metadata labels do not match the operation.
    pub fn bind_checked_profile(
        &self,
        bucket: Qwen3RopeKvBucketV1,
        operation: Qwen3RopeKvOperationV1,
        addresses: [u64; 11],
        byte_lengths: [u64; 11],
        metadata: Qwen3RopeKvHostMetadataV1,
    ) -> Result<CheckedQwen3RopeKvLaunchV1, BindQwen3RopeKvLaunchErrorV1> {
        let profile = self
            .catalog
            .profile(bucket, operation)
            .ok_or(BindQwen3RopeKvLaunchErrorV1::Profile)?;
        let buffers = Qwen3RopeKvBufferContractV1::checked(profile, addresses, byte_lengths)
            .map_err(BindQwen3RopeKvLaunchErrorV1::Buffers)?;
        metadata
            .validate(operation)
            .map_err(BindQwen3RopeKvLaunchErrorV1::Metadata)?;
        Ok(CheckedQwen3RopeKvLaunchV1 {
            profile,
            buffers,
            metadata,
        })
    }
}

/// Consumes Worker V2 evidence through exact transcript, HSACO, ABI, and loader checks.
///
/// # Errors
///
/// Returns an error if Worker lineage, output identity, ELF/AMDHSA structure,
/// either explicit ABI, the hidden ABI, or the loader profile is not exact.
pub fn inspect_qwen3_rope_kv_kernel_v1(
    evidence: InertQwen3RopeKvWorkerEvidenceV1,
) -> Result<InspectedQwen3RopeKvKernelV1, InspectQwen3RopeKvKernelErrorV1> {
    let InertQwen3RopeKvWorkerEvidenceV1 {
        prepared,
        transaction_handoff,
        worker,
    } = evidence;
    validate_worker_lineage(&prepared, transaction_handoff, &worker)?;
    let bytes = worker.output_bytes();
    if !worker.output_identity().matches(bytes) {
        return Err(InspectQwen3RopeKvKernelErrorV1::SourceLineage);
    }
    let bound = inspect_and_bind_kernel_descriptors(bytes)
        .map_err(InspectQwen3RopeKvKernelErrorV1::Hsaco)?;
    let kernels = bound.inspection().kernels();
    let bindings = bound.bindings();
    if kernels.len() != 2 || bindings.len() != 2 {
        return Err(InspectQwen3RopeKvKernelErrorV1::KernelProfile);
    }
    let Some(rope_index) = kernels
        .iter()
        .position(|kernel| kernel.name() == QWEN3_ROPE_KERNEL_SYMBOL_V1)
    else {
        return Err(InspectQwen3RopeKvKernelErrorV1::KernelProfile);
    };
    let Some(kv_index) = kernels
        .iter()
        .position(|kernel| kernel.name() == QWEN3_PAGED_KV_WRITE_KERNEL_SYMBOL_V1)
    else {
        return Err(InspectQwen3RopeKvKernelErrorV1::KernelProfile);
    };
    if rope_index == kv_index {
        return Err(InspectQwen3RopeKvKernelErrorV1::KernelProfile);
    }
    let rope = &kernels[rope_index];
    let kv = &kernels[kv_index];
    let rope_binding = &bindings[rope_index];
    let kv_binding = &bindings[kv_index];
    if bound.inspection().code_object_version() != InspectedCodeObjectVersion::V6
        || bound.inspection().target().to_string() != QWEN3_ROPE_KV_TARGET_V1
        || bound.inspection().has_printf_metadata()
        || rope.name() != QWEN3_ROPE_KERNEL_SYMBOL_V1
        || rope.symbol() != QWEN3_ROPE_KERNEL_DESCRIPTOR_SYMBOL_V1
        || rope.kernarg_segment_size() != QWEN3_ROPE_TOTAL_KERNARG_BYTES_V1
        || rope.kernarg_segment_alignment() != QWEN3_ROPE_KV_KERNARG_ALIGNMENT_V1
        || rope.implicit_argument_offset() != Some(QWEN3_ROPE_EXPLICIT_KERNARG_BYTES_V1)
        || rope.implicit_argument_size() != 256
        || rope.required_workgroup_size() != Some(QWEN3_ROPE_KV_WORKGROUP_V1)
        || rope.max_flat_workgroup_size() != 64
        || rope.wavefront_size() != 64
        || rope.group_segment_fixed_size() != 0
        || rope.private_segment_fixed_size() != 0
        || rope.sgpr_spill_count().unwrap_or(0) != 0
        || rope.vgpr_spill_count().unwrap_or(0) != 0
        || rope.uses_dynamic_stack()
        || rope_binding.kernel_index() != rope_index
        || rope_binding.descriptor().group_segment_fixed_size() != 0
        || rope_binding.descriptor().private_segment_fixed_size() != 0
        || rope_binding.descriptor().wavefront_size() != 64
        || rope_binding.descriptor().uses_dynamic_stack()
        || !exact_rope_explicit_arguments(rope.explicit_arguments())
        || !exact_hidden_arguments(
            rope.hidden_arguments(),
            QWEN3_ROPE_EXPLICIT_KERNARG_BYTES_V1,
        )
        || kv.name() != QWEN3_PAGED_KV_WRITE_KERNEL_SYMBOL_V1
        || kv.symbol() != QWEN3_PAGED_KV_WRITE_KERNEL_DESCRIPTOR_SYMBOL_V1
        || kv.kernarg_segment_size() != QWEN3_KV_WRITE_TOTAL_KERNARG_BYTES_V1
        || kv.kernarg_segment_alignment() != QWEN3_ROPE_KV_KERNARG_ALIGNMENT_V1
        || kv.implicit_argument_offset() != Some(QWEN3_KV_WRITE_EXPLICIT_KERNARG_BYTES_V1)
        || kv.implicit_argument_size() != 256
        || kv.required_workgroup_size() != Some(QWEN3_ROPE_KV_WORKGROUP_V1)
        || kv.max_flat_workgroup_size() != 64
        || kv.wavefront_size() != 64
        || kv.group_segment_fixed_size() != 0
        || kv.private_segment_fixed_size() != 0
        || kv.sgpr_spill_count().unwrap_or(0) != 0
        || kv.vgpr_spill_count().unwrap_or(0) != 0
        || kv.uses_dynamic_stack()
        || kv_binding.kernel_index() != kv_index
        || kv_binding.descriptor().group_segment_fixed_size() != 0
        || kv_binding.descriptor().private_segment_fixed_size() != 0
        || kv_binding.descriptor().wavefront_size() != 64
        || kv_binding.descriptor().uses_dynamic_stack()
        || !exact_kv_explicit_arguments(kv.explicit_arguments())
        || !exact_hidden_arguments(
            kv.hidden_arguments(),
            QWEN3_KV_WRITE_EXPLICIT_KERNARG_BYTES_V1,
        )
    {
        return Err(InspectQwen3RopeKvKernelErrorV1::KernelProfile);
    }
    let loader = fe2o3_amdhsa_loader::validate(bytes, AdmittedProfile::Gfx942XnackOffCov6)
        .map_err(InspectQwen3RopeKvKernelErrorV1::Loader)?;
    let loader_plan = *loader.plan();
    Ok(InspectedQwen3RopeKvKernelV1 {
        catalog: prepared.catalog,
        source_identity: prepared.source_identity,
        compiler_handoff_identity: prepared.compiler_handoff_identity,
        transaction_handoff,
        loader_plan,
        worker,
    })
}

fn validate_worker_lineage(
    prepared: &PreparedQwen3RopeKvKernelV1,
    transaction_handoff: CompilerModuleHandoffIdentityV1,
    worker: &InertFirstBuildWorkerV2EvidenceV1,
) -> Result<(), InspectQwen3RopeKvKernelErrorV1> {
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
        return Err(InspectQwen3RopeKvKernelErrorV1::SourceLineage);
    }
    let bootstrap = InertDecodedWorkerExchangeV2::decode(
        worker.bootstrap_request_bytes(),
        worker.bootstrap().response().canonical_bytes(),
    )
    .map_err(InspectQwen3RopeKvKernelErrorV1::Protocol)?;
    let replay = InertDecodedWorkerExchangeV2::decode(
        worker.authorized_request_bytes(),
        worker.authorized().response().canonical_bytes(),
    )
    .map_err(InspectQwen3RopeKvKernelErrorV1::Protocol)?;
    for exchange in [&bootstrap, &replay] {
        let request = exchange.request();
        if request.target() != exact_target()
            || request.code_object_version() != CodeObjectVersion::V6
            || request.compiler_module().bytes() != prepared.compiler_handoff.module_bytes()
            || !request.external_providers().is_empty()
            || !request.import_symbols().is_empty()
            || !request.export_symbols().is_empty()
            || !request.final_symbols().iter().map(String::as_str).eq([
                QWEN3_PAGED_KV_WRITE_KERNEL_SYMBOL_V1,
                QWEN3_ROPE_KERNEL_SYMBOL_V1,
                QWEN3_PAGED_KV_WRITE_KERNEL_DESCRIPTOR_SYMBOL_V1,
                QWEN3_ROPE_KERNEL_DESCRIPTOR_SYMBOL_V1,
            ])
            || exchange.response().request_identity() != request.identity()
        {
            return Err(InspectQwen3RopeKvKernelErrorV1::SourceLineage);
        }
    }
    Ok(())
}

/// Failure while binding an inspected output to one finite runtime profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindQwen3RopeKvLaunchErrorV1 {
    /// The requested role/bucket/operation tuple is absent from the finite catalog.
    Profile,
    /// Numerical buffer address or extent validation failed.
    Buffers(Qwen3RopeKvBufferContractErrorV1),
    /// Host-only table or deployment-table metadata was absent or crossed modes.
    Metadata(Qwen3RopeKvHostMetadataErrorV1),
}

impl fmt::Display for BindQwen3RopeKvLaunchErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Qwen3 RoPE/KV launch binding failed: {self:?}")
    }
}

impl std::error::Error for BindQwen3RopeKvLaunchErrorV1 {}

/// Host-side identities retained across checked binding but absent from both machine ABIs.
///
/// These values are untrusted labels. For KV write they record the role-scoped
/// generation, exclusive owner, page-table identity, and selected global cache
/// pool checked by a future protected runner. For `RoPE` they record the fixed
/// deployment trig-table identity. They do not authenticate content or
/// authority and do not establish that the page table belongs to the pool.
#[derive(Debug, Eq, PartialEq)]
pub struct Qwen3RopeKvHostMetadataV1 {
    trig_table_identity: [u8; 32],
    page_table_identity: [u8; 32],
    cache_pool_identity: [u8; 32],
    exclusive_owner_identity: [u8; 32],
    page_generation: u64,
}

impl Qwen3RopeKvHostMetadataV1 {
    /// Constructs RoPE-only metadata for one fixed deployment table label.
    #[must_use]
    pub const fn for_rope(trig_table_identity: [u8; 32]) -> Self {
        Self {
            trig_table_identity,
            page_table_identity: [0; 32],
            cache_pool_identity: [0; 32],
            exclusive_owner_identity: [0; 32],
            page_generation: 0,
        }
    }

    /// Constructs KV-only role/generation/ownership/page-table/cache-pool labels.
    #[must_use]
    pub const fn for_kv_write(
        page_table_identity: [u8; 32],
        cache_pool_identity: [u8; 32],
        exclusive_owner_identity: [u8; 32],
        page_generation: u64,
    ) -> Self {
        Self {
            trig_table_identity: [0; 32],
            page_table_identity,
            cache_pool_identity,
            exclusive_owner_identity,
            page_generation,
        }
    }

    fn validate(
        &self,
        operation: Qwen3RopeKvOperationV1,
    ) -> Result<(), Qwen3RopeKvHostMetadataErrorV1> {
        match operation {
            Qwen3RopeKvOperationV1::Rope
                if self.trig_table_identity != [0; 32]
                    && self.page_table_identity == [0; 32]
                    && self.cache_pool_identity == [0; 32]
                    && self.exclusive_owner_identity == [0; 32]
                    && self.page_generation == 0 =>
            {
                Ok(())
            }
            Qwen3RopeKvOperationV1::PagedKvWrite
                if self.trig_table_identity == [0; 32]
                    && self.page_table_identity != [0; 32]
                    && self.cache_pool_identity != [0; 32]
                    && self.exclusive_owner_identity != [0; 32]
                    && self.page_table_identity != self.cache_pool_identity
                    && self.page_table_identity != self.exclusive_owner_identity
                    && self.cache_pool_identity != self.exclusive_owner_identity
                    && self.page_generation != 0 =>
            {
                Ok(())
            }
            _ => Err(Qwen3RopeKvHostMetadataErrorV1::ModeOrIdentity),
        }
    }

    /// These labels do not authenticate table content, provenance, or ownership.
    #[must_use]
    pub const fn authenticates_content(&self) -> bool {
        false
    }
}

/// Host metadata admission failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Qwen3RopeKvHostMetadataErrorV1 {
    /// Required identity/generation was absent, aliased, or supplied for the wrong operation.
    ModeOrIdentity,
}

/// Inert exact profile and numerical-buffer binding for a future protected launcher.
#[derive(Debug)]
pub struct CheckedQwen3RopeKvLaunchV1 {
    profile: Qwen3RopeKvProfileV1,
    buffers: Qwen3RopeKvBufferContractV1,
    metadata: Qwen3RopeKvHostMetadataV1,
}

impl CheckedQwen3RopeKvLaunchV1 {
    /// Exact finite profile.
    #[must_use]
    pub const fn profile(&self) -> Qwen3RopeKvProfileV1 {
        self.profile
    }

    /// Exact checked numerical buffer ranges.
    #[must_use]
    pub const fn buffers(&self) -> &Qwen3RopeKvBufferContractV1 {
        &self.buffers
    }

    /// Host-only metadata retained outside the machine ABI.
    #[must_use]
    pub const fn metadata(&self) -> &Qwen3RopeKvHostMetadataV1 {
        &self.metadata
    }

    /// This binding grants no allocation, load, or launch authority.
    #[must_use]
    pub const fn grants_launch_authority(&self) -> bool {
        false
    }
}

fn exact_pointer_argument(
    argument: &ExplicitArgument,
    name: &str,
    offset: u64,
    access: ArgumentAccess,
    alignment: u64,
    accepted_type: fn(ExplicitValueType) -> bool,
) -> bool {
    argument.name() == Some(name)
        && argument.offset() == offset
        && argument.size() == 8
        && argument.alignment().is_none_or(|actual| actual == 8)
        && argument
            .pointee_alignment()
            .is_none_or(|actual| actual == alignment)
        && argument.value_kind() == ExplicitValueKind::GlobalBuffer
        && argument.value_type().is_none_or(accepted_type)
        && argument.address_space() == Some(ArgumentAddressSpace::Global)
        && argument.access() == Some(access)
}

fn exact_length_argument(argument: &ExplicitArgument, name: &str, offset: u64) -> bool {
    argument.name() == Some(name)
        && argument.offset() == offset
        && argument.size() == 8
        && argument.value_kind() == ExplicitValueKind::ByValue
        && argument
            .value_type()
            .is_none_or(|value_type| value_type == ExplicitValueType::U64)
        && argument.address_space().is_none()
        && argument.access().is_none()
}

fn exact_rope_explicit_arguments(arguments: &[ExplicitArgument]) -> bool {
    if arguments.len() != 18 {
        return false;
    }
    let pointers = [
        (
            0,
            "query_bf16",
            ArgumentAccess::ReadOnly,
            2,
            is_bf16_metadata_carrier as fn(ExplicitValueType) -> bool,
        ),
        (
            2,
            "key_bf16",
            ArgumentAccess::ReadOnly,
            2,
            is_bf16_metadata_carrier,
        ),
        (
            4,
            "position_ids",
            ArgumentAccess::ReadOnly,
            4,
            is_i32_metadata_carrier,
        ),
        (
            6,
            "cos_table_f32",
            ArgumentAccess::ReadOnly,
            4,
            is_f32_metadata_carrier,
        ),
        (
            8,
            "sin_table_f32",
            ArgumentAccess::ReadOnly,
            4,
            is_f32_metadata_carrier,
        ),
        (
            10,
            "rotated_query_bf16",
            ArgumentAccess::WriteOnly,
            2,
            is_bf16_metadata_carrier,
        ),
        (
            12,
            "rotated_key_bf16",
            ArgumentAccess::WriteOnly,
            2,
            is_bf16_metadata_carrier,
        ),
    ];
    for (index, name, access, alignment, accepted_type) in pointers {
        if !exact_pointer_argument(
            &arguments[index],
            name,
            (index as u64 / 2) * 16,
            access,
            alignment,
            accepted_type,
        ) {
            return false;
        }
    }
    let lengths = [
        (1, "query_bf16_elements"),
        (3, "key_bf16_elements"),
        (5, "position_ids_elements"),
        (7, "cos_table_f32_elements"),
        (9, "sin_table_f32_elements"),
        (11, "rotated_query_bf16_elements"),
        (13, "rotated_key_bf16_elements"),
    ];
    for (index, name) in lengths {
        if !exact_length_argument(&arguments[index], name, ((index - 1) as u64 / 2) * 16 + 8) {
            return false;
        }
    }
    exact_scalar_argument(&arguments[14], "active_tokens", 112, ExplicitValueType::U32)
        && exact_scalar_argument(&arguments[15], "sequences", 116, ExplicitValueType::U32)
        && exact_scalar_argument(&arguments[16], "query_heads", 120, ExplicitValueType::U32)
        && exact_scalar_argument(
            &arguments[17],
            "context_tokens",
            124,
            ExplicitValueType::U32,
        )
}

fn exact_kv_explicit_arguments(arguments: &[ExplicitArgument]) -> bool {
    if arguments.len() != 15 {
        return false;
    }
    let pointers = [
        (
            0,
            "rotated_key_bf16",
            ArgumentAccess::ReadOnly,
            2,
            is_bf16_metadata_carrier as fn(ExplicitValueType) -> bool,
        ),
        (
            2,
            "value_bf16",
            ArgumentAccess::ReadOnly,
            2,
            is_bf16_metadata_carrier,
        ),
        (
            4,
            "logical_starts",
            ArgumentAccess::ReadOnly,
            4,
            is_i32_metadata_carrier,
        ),
        (
            6,
            "page_indices",
            ArgumentAccess::ReadOnly,
            4,
            is_i32_metadata_carrier,
        ),
        (
            8,
            "key_cache_bf16",
            ArgumentAccess::WriteOnly,
            2,
            is_bf16_metadata_carrier,
        ),
        (
            10,
            "value_cache_bf16",
            ArgumentAccess::WriteOnly,
            2,
            is_bf16_metadata_carrier,
        ),
    ];
    for (index, name, access, alignment, accepted_type) in pointers {
        if !exact_pointer_argument(
            &arguments[index],
            name,
            (index as u64 / 2) * 16,
            access,
            alignment,
            accepted_type,
        ) {
            return false;
        }
    }
    for (index, name) in [
        (1, "rotated_key_bf16_elements"),
        (3, "value_bf16_elements"),
        (5, "logical_starts_elements"),
        (7, "page_indices_elements"),
        (9, "key_cache_bf16_elements"),
        (11, "value_cache_bf16_elements"),
    ] {
        if !exact_length_argument(&arguments[index], name, ((index - 1) as u64 / 2) * 16 + 8) {
            return false;
        }
    }
    exact_scalar_argument(&arguments[12], "active_tokens", 96, ExplicitValueType::U32)
        && exact_scalar_argument(&arguments[13], "sequences", 100, ExplicitValueType::U32)
        && exact_scalar_argument(
            &arguments[14],
            "context_tokens",
            104,
            ExplicitValueType::U32,
        )
}

fn exact_scalar_argument(
    argument: &ExplicitArgument,
    name: &str,
    offset: u64,
    value_type: ExplicitValueType,
) -> bool {
    argument.name() == Some(name)
        && argument.offset() == offset
        && argument.size() == 4
        && argument.value_kind() == ExplicitValueKind::ByValue
        && argument
            .value_type()
            .is_none_or(|actual| actual == value_type)
        && argument.address_space().is_none()
        && argument.access().is_none()
}

const fn is_bf16_metadata_carrier(value_type: ExplicitValueType) -> bool {
    matches!(
        value_type,
        ExplicitValueType::I16 | ExplicitValueType::U16 | ExplicitValueType::F16
    )
}

const fn is_i32_metadata_carrier(value_type: ExplicitValueType) -> bool {
    matches!(value_type, ExplicitValueType::I32 | ExplicitValueType::U32)
}

const fn is_f32_metadata_carrier(value_type: ExplicitValueType) -> bool {
    matches!(value_type, ExplicitValueType::F32)
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

fn fixed_link_options() -> Result<Vec<LinkOptionV1>, ExecuteQwen3RopeKvWorkerErrorV1> {
    [
        ("code-object-version", "6"),
        ("opt-level", "2"),
        ("strip-debug", "true"),
        ("verify-each", "true"),
    ]
    .into_iter()
    .map(|(name, value)| {
        LinkOptionV1::new(name, value).map_err(|_| ExecuteQwen3RopeKvWorkerErrorV1::FixedLinkOption)
    })
    .collect()
}

fn bound_stage_identity(
    domain: &[u8],
    caller_label: [u8; 32],
    catalog: Qwen3RopeKvProfileCatalogIdentityV1,
) -> [u8; 32] {
    let mut bytes = Vec::with_capacity(64);
    bytes.extend_from_slice(&caller_label);
    bytes.extend_from_slice(catalog.as_bytes());
    hash(domain, &bytes)
}

fn exact_target() -> DeviceTargetV1 {
    DeviceTargetV1::parse(QWEN3_ROPE_KV_TARGET_V1)
        .expect("the fixed Qwen3 RoPE/KV target is canonical")
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
    use std::collections::{BTreeMap, BTreeSet};

    fn bindings(seed: u8) -> Qwen3RopeKvSourceBindingsV1 {
        Qwen3RopeKvSourceBindingsV1::new(
            [seed; 32],
            [seed.wrapping_add(1); 32],
            [seed.wrapping_add(2); 32],
            [seed.wrapping_add(3); 32],
        )
    }

    fn bucket(role: Qwen3RopeKvModelRoleV1, kind: Qwen3RopeKvBucketKindV1) -> Qwen3RopeKvBucketV1 {
        Qwen3RopeKvBucketV1::new(role, kind)
    }

    fn rope_layout(profile: Qwen3RopeKvProfileV1) -> ([u64; 11], [u64; 11]) {
        let query = profile.query_elements() * 2;
        let kv = profile.kv_elements() * 2;
        let positions = u64::from(profile.base_rows()) * 4;
        let trig = QWEN3_ROPE_TRIG_TABLE_ELEMENTS_V1 * 4;
        (
            [
                0x1_0000_0000,
                0x2_0000_0000,
                0,
                0x3_0000_0000,
                0x4_0000_0000,
                0x5_0000_0000,
                0,
                0,
                0x6_0000_0000,
                0x7_0000_0000,
                0,
            ],
            [query, kv, 0, positions, trig, trig, 0, 0, query, kv, 0],
        )
    }

    fn kv_layout(profile: Qwen3RopeKvProfileV1) -> ([u64; 11], [u64; 11]) {
        let sequences = u64::from(profile.bucket().sequence_and_active_tokens()[0]);
        let kv = profile.kv_elements() * 2;
        let starts = sequences * 4;
        let pages = sequences * u64::from(QWEN3_KV_PAGE_TABLE_ENTRIES_V1) * 4;
        let cache = QWEN3_KV_CACHE_BYTES_V1;
        (
            [
                0,
                0x1_0000_0000,
                0x2_0000_0000,
                0,
                0,
                0,
                0x3_0000_0000,
                0x4_0000_0000,
                0,
                0x5_0000_0000,
                0x6_0000_0000,
            ],
            [0, kv, kv, 0, 0, 0, starts, pages, 0, cache, cache],
        )
    }

    #[test]
    fn exact_44_profile_catalog_is_complete_and_unique() {
        let catalog = Qwen3RopeKvProfileCatalogV1::canonical().unwrap();
        assert_eq!(catalog.profiles().len(), 44);
        let identities = catalog
            .profiles()
            .iter()
            .map(|profile| *profile.identity().as_bytes())
            .collect::<BTreeSet<_>>();
        assert_eq!(identities.len(), 44);
        for role in QWEN3_ROPE_KV_ROLES_V1 {
            for kind in QWEN3_ROPE_KV_BUCKET_KINDS_V1 {
                for operation in QWEN3_ROPE_KV_OPERATIONS_V1 {
                    let profile = catalog.profile(bucket(role, kind), operation).unwrap();
                    let [sequences, active] = profile.bucket().sequence_and_active_tokens();
                    assert_eq!(profile.base_rows(), sequences * active);
                    assert_eq!(
                        profile.query_elements(),
                        u64::from(sequences * active * role.query_heads() * 128)
                    );
                    assert_eq!(
                        profile.kv_elements(),
                        u64::from(sequences * active * 8 * 128)
                    );
                    assert_eq!(profile.hsa_adapter_block_counts(), [active, sequences, 1]);
                    assert_eq!(profile.aql_grid_work_items(), [active * 64, sequences, 1]);
                }
            }
        }
        assert_eq!(Qwen3RopeKvModelRoleV1::Target8B.query_heads(), 32);
        assert_eq!(Qwen3RopeKvModelRoleV1::Draft06B.query_heads(), 16);
        assert_eq!(Qwen3RopeKvModelRoleV1::Target8B.layers(), 36);
        assert_eq!(Qwen3RopeKvModelRoleV1::Draft06B.layers(), 28);
        for (role, expected_query_elements) in [
            (
                Qwen3RopeKvModelRoleV1::Target8B,
                u64::from(2_048_u32 * 32 * 128),
            ),
            (
                Qwen3RopeKvModelRoleV1::Draft06B,
                u64::from(2_048_u32 * 16 * 128),
            ),
        ] {
            let profiles = catalog
                .profiles()
                .iter()
                .filter(|profile| profile.bucket().role() == role);
            assert_eq!(
                profiles.clone().map(|profile| profile.base_rows()).max(),
                Some(2_048)
            );
            assert_eq!(
                profiles.map(|profile| profile.query_elements()).max(),
                Some(expected_query_elements)
            );
        }
    }

    #[test]
    fn exact_bucket_context_and_speculative_role_extents_match_ferric() {
        let target = Qwen3RopeKvModelRoleV1::Target8B;
        let draft = Qwen3RopeKvModelRoleV1::Draft06B;
        let expected = [
            (
                Qwen3RopeKvBucketKindV1::PrefillS1T128,
                [1, 128],
                [1, 128],
                128,
            ),
            (
                Qwen3RopeKvBucketKindV1::PrefillS8T128,
                [8, 128],
                [8, 128],
                128,
            ),
            (
                Qwen3RopeKvBucketKindV1::PrefillS1T512,
                [1, 512],
                [1, 512],
                512,
            ),
            (
                Qwen3RopeKvBucketKindV1::PrefillS1T2048,
                [1, 2_048],
                [1, 2_048],
                2_048,
            ),
            (
                Qwen3RopeKvBucketKindV1::DecodeS1C8192,
                [1, 1],
                [1, 1],
                8_192,
            ),
            (
                Qwen3RopeKvBucketKindV1::DecodeS8C8192,
                [8, 1],
                [8, 1],
                8_192,
            ),
            (
                Qwen3RopeKvBucketKindV1::DecodeS32C8192,
                [32, 1],
                [32, 1],
                8_192,
            ),
            (
                Qwen3RopeKvBucketKindV1::SpeculativeS1K4C8192,
                [1, 5],
                [1, 4],
                8_192,
            ),
            (
                Qwen3RopeKvBucketKindV1::SpeculativeS8K4C8192,
                [8, 5],
                [8, 4],
                8_192,
            ),
            (
                Qwen3RopeKvBucketKindV1::SpeculativeS1K8C8192,
                [1, 9],
                [1, 8],
                8_192,
            ),
            (
                Qwen3RopeKvBucketKindV1::SpeculativeS1K16C8192,
                [1, 17],
                [1, 16],
                8_192,
            ),
        ];
        for (kind, target_shape, draft_shape, context) in expected {
            assert_eq!(
                bucket(target, kind).sequence_and_active_tokens(),
                target_shape
            );
            assert_eq!(
                bucket(draft, kind).sequence_and_active_tokens(),
                draft_shape
            );
            assert_eq!(bucket(target, kind).context_tokens(), context);
            assert_eq!(bucket(draft, kind).context_tokens(), context);
        }
    }

    #[test]
    fn rope_machine_geometry_rejects_cross_role_speculative_extents() {
        for (active, sequences) in [(5, 1), (5, 8), (9, 1), (17, 1)] {
            assert!(rope_machine_geometry_is_known(active, sequences, 32, 8_192));
            assert!(!rope_machine_geometry_is_known(
                active, sequences, 16, 8_192
            ));
        }
        for (active, sequences) in [(4, 1), (4, 8), (8, 1), (16, 1)] {
            assert!(rope_machine_geometry_is_known(active, sequences, 16, 8_192));
            assert!(!rope_machine_geometry_is_known(
                active, sequences, 32, 8_192
            ));
        }
        assert!(rope_machine_geometry_is_known(128, 8, 16, 128));
        assert!(rope_machine_geometry_is_known(128, 8, 32, 128));
        assert!(!rope_machine_geometry_is_known(5, 32, 32, 8_192));
    }

    #[test]
    fn kv_machine_graph_uses_global_16384_page_pool_without_sequence_prefix() {
        let catalog = Qwen3RopeKvProfileCatalogV1::canonical().unwrap();
        let handoff = construct_typed_handoff(&catalog, bindings(0x29)).unwrap();
        let function = handoff
            .module()
            .functions()
            .iter()
            .find(|function| function.symbol() == QWEN3_PAGED_KV_WRITE_KERNEL_SYMBOL_V1)
            .unwrap();
        let definitions = function
            .blocks()
            .iter()
            .flat_map(fe2o3_llvm_handoff::BasicBlockV2::instructions)
            .filter_map(|instruction| {
                instruction
                    .result()
                    .map(|result| (result.id(), instruction.kind()))
            })
            .collect::<BTreeMap<_, _>>();
        let constant_bits = |id: ValueIdV2| match definitions.get(&id) {
            Some(InstructionKindV2::Constant(value)) => Some(value.bits()),
            _ => None,
        };
        let physical_page = definitions
            .iter()
            .find_map(|(result, kind)| match kind {
                InstructionKindV2::Load {
                    pointer,
                    value_type: ScalarTypeV1::I32,
                    ..
                } if matches!(
                    definitions.get(pointer),
                    Some(InstructionKindV2::GetElementPtr { base, .. })
                        if *base == ValueIdV2::new(7)
                ) =>
                {
                    Some(*result)
                }
                _ => None,
            })
            .expect("the physical page must be loaded from page_indices");
        let physical64 = definitions
            .iter()
            .find_map(|(result, kind)| match kind {
                InstructionKindV2::Cast {
                    operation: CastOperationV2::ZeroExtend,
                    value,
                    to: ValueTypeV2::Scalar(ScalarTypeV1::I64),
                } if *value == physical_page => Some(*result),
                _ => None,
            })
            .expect("the admitted physical page must be extended for cache indexing");

        assert!(definitions.values().any(|kind| matches!(
            kind,
            InstructionKindV2::Compare {
                predicate: ComparePredicateV2::UnsignedLessThan,
                left,
                right,
            } if *left == physical_page
                && constant_bits(*right) == Some(u64::from(QWEN3_KV_PHYSICAL_PAGE_SLOTS_V1))
        )));
        assert!(!definitions.values().any(|kind| matches!(
            kind,
            InstructionKindV2::Compare {
                predicate: ComparePredicateV2::UnsignedLessThan,
                left,
                right,
            } if *left == physical_page
                && constant_bits(*right) == Some(u64::from(QWEN3_KV_PAGE_TABLE_ENTRIES_V1))
        )));
        assert!(definitions.values().any(|kind| matches!(
            kind,
            InstructionKindV2::Binary {
                operation: BinaryOperationV2::Integer(IntegerBinaryOperationV2::Multiply),
                left,
                right,
            } if (*left == physical64
                && constant_bits(*right) == Some(u64::from(QWEN3_KV_PAGE_TOKENS_V1)))
                || (*right == physical64
                    && constant_bits(*left) == Some(u64::from(QWEN3_KV_PAGE_TOKENS_V1)))
        )));
        assert!(!definitions.values().any(|kind| matches!(
            kind,
            InstructionKindV2::Binary {
                operation: BinaryOperationV2::Integer(IntegerBinaryOperationV2::Add),
                left,
                right,
            } if *left == physical64 || *right == physical64
        )));

        let cache_elements = definitions
            .iter()
            .find_map(|(result, kind)| match kind {
                InstructionKindV2::Constant(value)
                    if value.bits() == QWEN3_KV_CACHE_ELEMENTS_V1 =>
                {
                    Some(*result)
                }
                _ => None,
            })
            .expect("the fixed global cache extent must be in the machine graph");
        for length_parameter in [ValueIdV2::new(10), ValueIdV2::new(12)] {
            assert!(definitions.values().any(|kind| matches!(
                kind,
                InstructionKindV2::Compare {
                    predicate: ComparePredicateV2::IntegerEqual,
                    left,
                    right,
                } if (*left == length_parameter && *right == cache_elements)
                    || (*right == length_parameter && *left == cache_elements)
            )));
        }

        let profile = catalog.profiles()[0];
        assert_eq!(
            &profile.encode()[profile.encode().len() - 4..],
            &QWEN3_KV_PHYSICAL_PAGE_SLOTS_V1.to_le_bytes()
        );
    }

    #[test]
    fn buffer_contracts_are_exact_disjoint_and_mode_separated() {
        let catalog = Qwen3RopeKvProfileCatalogV1::canonical().unwrap();
        let selected = bucket(
            Qwen3RopeKvModelRoleV1::Target8B,
            Qwen3RopeKvBucketKindV1::DecodeS8C8192,
        );
        let rope = catalog
            .profile(selected, Qwen3RopeKvOperationV1::Rope)
            .unwrap();
        let kv = catalog
            .profile(selected, Qwen3RopeKvOperationV1::PagedKvWrite)
            .unwrap();
        let (rope_addresses, rope_lengths) = rope_layout(rope);
        let rope_checked =
            Qwen3RopeKvBufferContractV1::checked(rope, rope_addresses, rope_lengths).unwrap();
        assert_eq!(rope_checked.operation(), Qwen3RopeKvOperationV1::Rope);
        let (kv_addresses, kv_lengths) = kv_layout(kv);
        let kv_checked =
            Qwen3RopeKvBufferContractV1::checked(kv, kv_addresses, kv_lengths).unwrap();
        assert_eq!(kv_checked.operation(), Qwen3RopeKvOperationV1::PagedKvWrite);
        assert!(Qwen3RopeKvBufferContractV1::checked(rope, kv_addresses, kv_lengths).is_err());
        assert!(Qwen3RopeKvBufferContractV1::checked(kv, rope_addresses, rope_lengths).is_err());

        assert_eq!(kv_lengths[9], QWEN3_KV_CACHE_BYTES_V1);
        assert_eq!(kv_lengths[10], QWEN3_KV_CACHE_BYTES_V1);
        let mut per_sequence_cache_substitution = kv_lengths;
        let sequences = u64::from(kv.bucket().sequence_and_active_tokens()[0]);
        per_sequence_cache_substitution[9] = sequences
            * u64::from(QWEN3_KV_PAGE_TABLE_ENTRIES_V1)
            * u64::from(QWEN3_KV_PAGE_TOKENS_V1)
            * 8
            * u64::from(QWEN3_ROPE_KV_HEAD_DIMENSION_V1)
            * 2;
        per_sequence_cache_substitution[10] = per_sequence_cache_substitution[9];
        assert_ne!(per_sequence_cache_substitution[9], QWEN3_KV_CACHE_BYTES_V1);
        assert_eq!(
            Qwen3RopeKvBufferContractV1::checked(kv, kv_addresses, per_sequence_cache_substitution),
            Err(Qwen3RopeKvBufferContractErrorV1::ByteLength(
                Qwen3RopeKvBufferV1::KeyOutputOrCache
            ))
        );

        let mut short = rope_lengths;
        short[4] -= 4;
        assert_eq!(
            Qwen3RopeKvBufferContractV1::checked(rope, rope_addresses, short),
            Err(Qwen3RopeKvBufferContractErrorV1::ByteLength(
                Qwen3RopeKvBufferV1::CosTable
            ))
        );
        let mut alias = kv_addresses;
        alias[10] = alias[9];
        assert_eq!(
            Qwen3RopeKvBufferContractV1::checked(kv, alias, kv_lengths),
            Err(Qwen3RopeKvBufferContractErrorV1::Aliasing)
        );
        let mut overflow = kv_addresses;
        overflow[9] = u64::MAX - 1;
        assert_eq!(
            Qwen3RopeKvBufferContractV1::checked(kv, overflow, kv_lengths),
            Err(Qwen3RopeKvBufferContractErrorV1::RangeOverflow(
                Qwen3RopeKvBufferV1::KeyOutputOrCache
            ))
        );
    }

    #[test]
    fn host_metadata_rejects_missing_aliased_and_cross_operation_labels() {
        let rope = Qwen3RopeKvHostMetadataV1::for_rope([1; 32]);
        assert_eq!(rope.validate(Qwen3RopeKvOperationV1::Rope), Ok(()));
        assert!(rope.validate(Qwen3RopeKvOperationV1::PagedKvWrite).is_err());
        let kv = Qwen3RopeKvHostMetadataV1::for_kv_write([2; 32], [3; 32], [4; 32], 7);
        assert_eq!(kv.validate(Qwen3RopeKvOperationV1::PagedKvWrite), Ok(()));
        assert!(kv.validate(Qwen3RopeKvOperationV1::Rope).is_err());
        assert!(
            Qwen3RopeKvHostMetadataV1::for_kv_write([2; 32], [2; 32], [4; 32], 7)
                .validate(Qwen3RopeKvOperationV1::PagedKvWrite)
                .is_err()
        );
        assert!(
            Qwen3RopeKvHostMetadataV1::for_kv_write([2; 32], [3; 32], [2; 32], 7)
                .validate(Qwen3RopeKvOperationV1::PagedKvWrite)
                .is_err()
        );
        assert!(
            Qwen3RopeKvHostMetadataV1::for_kv_write([2; 32], [3; 32], [4; 32], 0)
                .validate(Qwen3RopeKvOperationV1::PagedKvWrite)
                .is_err()
        );
        assert!(!kv.authenticates_content());
    }

    #[test]
    fn source_bindings_reject_zero_and_repeated_labels() {
        assert!(prepare_qwen3_rope_kv_kernel_v1(bindings(0x31)).is_ok());
        let mut identities = [[0x41; 32], [0x42; 32], [0x43; 32], [0x44; 32]];
        for left in 0..4 {
            for right in left + 1..4 {
                identities[right] = identities[left];
                assert!(matches!(
                    prepare_qwen3_rope_kv_kernel_v1(Qwen3RopeKvSourceBindingsV1::new(
                        identities[0],
                        identities[1],
                        identities[2],
                        identities[3]
                    )),
                    Err(PrepareQwen3RopeKvKernelErrorV1::SourceBindings)
                ));
                identities = [[0x41; 32], [0x42; 32], [0x43; 32], [0x44; 32]];
            }
        }
        assert!(matches!(
            prepare_qwen3_rope_kv_kernel_v1(Qwen3RopeKvSourceBindingsV1::new(
                [0; 32], [2; 32], [3; 32], [4; 32]
            )),
            Err(PrepareQwen3RopeKvKernelErrorV1::SourceBindings)
        ));
    }

    #[test]
    fn typed_two_kernel_graph_is_deterministic_and_structurally_exact() {
        let first = prepare_qwen3_rope_kv_kernel_v1(bindings(0x61)).unwrap();
        let second = prepare_qwen3_rope_kv_kernel_v1(bindings(0x61)).unwrap();
        assert_eq!(first.source_identity(), second.source_identity());
        assert_eq!(first.assembly_sha256(), second.assembly_sha256());
        assert_eq!(
            first.compiler_handoff_identity(),
            second.compiler_handoff_identity()
        );
        let llvm = std::str::from_utf8(first.compiler_handoff().module_bytes()).unwrap();
        for required in [
            "define amdgpu_kernel void @qwen3_rope_v1",
            "define amdgpu_kernel void @qwen3_paged_kv_write_v1",
            "%position_ids",
            "%cos_table_f32",
            "%sin_table_f32",
            "%logical_starts",
            "%page_indices",
            "%key_cache_bf16",
            "%value_cache_bf16",
            "load bfloat",
            "load float",
            "load i32",
            "fpext bfloat",
            "fptrunc float",
            "fmul float",
            "fsub float",
            "fadd float",
            "lshr i32",
            "and i32",
            "icmp ult",
            "@llvm.trap",
            "!reqd_work_group_size",
            "!{i32 64, i32 1, i32 1}",
        ] {
            assert!(
                llvm.contains(required),
                "missing LLVM fragment: {required}\n{llvm}"
            );
        }
        assert!(!llvm.contains("@llvm.sin"));
        assert!(!llvm.contains("@llvm.cos"));
        assert!(!first.uses_pliron_lowering());
        assert!(!first.proves_machine_refinement());
        assert!(!first.proves_numerical_contract());
        assert!(!first.authenticates_worker_execution());
        assert!(!first.grants_launch_authority());
    }

    #[test]
    fn malformed_bytes_never_reach_hsaco_or_loader_profile() {
        assert!(matches!(
            inspect_and_bind_kernel_descriptors(b"not an ELF"),
            Err(KernelBindingError::Inspection(_))
        ));
        assert!(
            fe2o3_amdhsa_loader::validate(b"not an ELF", AdmittedProfile::Gfx942XnackOffCov6)
                .is_err()
        );
    }
}
