//! Exact finite Qwen3 paged-GQA causal decode compiler profiles.
//!
//! Q and O are BF16 `[S,T,QH,128]`. K and V are BF16 P16 caches in the
//! fixed global pool `[16384,16,8,128]`, addressed through `u32` page indices
//! `[S,512]`.
//! Every page index reached by the kernel is checked `<16384`; neither the
//! host binding nor the kernel requires the complete table to cover, permute,
//! or exhaust the global pool. There is no sequence-local cache base.
//! Every selected query head maps to `query_head / gqa_group_size`, with
//! target QH32/GQA4 and draft QH16/GQA2 sharing KVH8.
//!
//! The machine declaration uses an ascending D128 FP32 dot and an online FP32
//! max/sum/numerator recurrence with the existing unresolved
//! `__ocml_exp_f32` provider boundary, FP32 division, and BF16 RNE output.
//! A separate two-pass host reference is intended for future reconciliation.
//! Numerical, operator, memory, race, content, plan, schedule, hardware, and
//! performance claims remain false/Open. The module also makes no
//! compiler-origin, provider-content, artifact, load, launch, or completion
//! claim.

use core::fmt;
use std::fmt::Write as _;

use fe2o3_amdhsa_loader::{AdmittedProfile, LoadPlan, PlanError};
use fe2o3_artifact_transaction::{
    CompilerModuleHandoffIdentityV1, ConsumedCompilerModuleHandoffV1,
};
use fe2o3_compiler_ffi::{
    CodeObjectVersion, CompilerFfiContractV1, CompilerFfiEnvelopeBuilderV1,
    CompilerFfiEnvelopeError, CompilerFfiLinkRoleV1, CompilerFfiSourceOwnerV1,
    CompilerModuleHandoffErrorV2, CompilerModuleHandoffIdentityV2, CompilerModuleHandoffV2,
    CompilerModuleKindV1, CompilerModuleSymbolManifestErrorV1,
    CompilerModuleSymbolManifestIdentityV1, CompilerModuleSymbolManifestV1,
    CompilerModuleSymbolRoleV1, DeviceTargetV1, EXTERNAL_DEVICE_LIBRARY_GFX942_DATA_LAYOUT_V1,
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
use reserved_fe2o3_symbols::{
    derive_device_ffi_contract_id_v1, DeviceFfiContractFieldsV1, DeviceFfiDirectionV1,
    DEVICE_FFI_DIRECTION_IMPORT_V1,
};
use sha2::{Digest as _, Sha256};

/// Exact kernel entry shared by all fourteen runtime profiles.
pub const QWEN3_PAGED_DECODE_KERNEL_SYMBOL_V1: &str = "qwen3_paged_gqa_decode_bf16_f32_v1";
/// Exact AMDHSA descriptor symbol.
pub const QWEN3_PAGED_DECODE_KERNEL_DESCRIPTOR_SYMBOL_V1: &str =
    "qwen3_paged_gqa_decode_bf16_f32_v1.kd";
/// Exact gfx942 feature profile.
pub const QWEN3_PAGED_DECODE_TARGET_V1: &str = "gfx942:xnack-";
/// Exact code-object version.
pub const QWEN3_PAGED_DECODE_CODE_OBJECT_VERSION_V1: u8 = 6;
/// Exact Wave64 workgroup, measured in workitems.
pub const QWEN3_PAGED_DECODE_WORKGROUP_V1: [u32; 3] = [64, 1, 1];
/// Exact Qwen3 attention feature count per head.
pub const QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1: u32 = 128;
/// Exact key/value head count for both model roles.
pub const QWEN3_PAGED_DECODE_KV_HEADS_V1: u32 = 8;
/// Exact tokens per physical cache page.
pub const QWEN3_PAGED_DECODE_PAGE_TOKENS_V1: u32 = 16;
/// Exact logical-page entries per sequence.
pub const QWEN3_PAGED_DECODE_PAGE_TABLE_ENTRIES_V1: u32 = 512;
/// Exact physical-page slots in the fixed global cache pool.
pub const QWEN3_PAGED_DECODE_CACHE_POOL_PAGES_V1: u32 = 16_384;
/// Exact FP32 bits for `1 / sqrt(128)`.
pub const QWEN3_PAGED_DECODE_ATTENTION_SCALE_BITS_V1: u32 = 0x3db5_04f3;
/// Six pointer-plus-`u64`-length slice records.
pub const QWEN3_PAGED_DECODE_EXPLICIT_KERNARG_BYTES_V1: u64 = 96;
/// Exact explicit plus COV6 hidden kernarg bytes.
pub const QWEN3_PAGED_DECODE_TOTAL_KERNARG_BYTES_V1: u64 = 352;
/// Exact kernarg alignment.
pub const QWEN3_PAGED_DECODE_KERNARG_ALIGNMENT_V1: u64 = 8;
/// Number of finite target/draft paged decode profiles.
pub const QWEN3_PAGED_DECODE_PROFILE_COUNT_V1: usize = 14;
/// Exact byte length of the final canonical direct-LLVM source.
pub const QWEN3_PAGED_DECODE_LLVM_BYTES_V1: usize = 23_685;
/// SHA-256 of the final canonical direct-LLVM source bytes.
pub const QWEN3_PAGED_DECODE_LLVM_SHA256_V1: [u8; 32] = [
    0xa9, 0x12, 0x31, 0xb0, 0x22, 0x71, 0x51, 0x97, 0x7b, 0x6c, 0xe0, 0xb3, 0x84, 0x12, 0x60, 0x52,
    0x35, 0xf9, 0x74, 0x01, 0x27, 0x59, 0x0b, 0x47, 0x9b, 0x70, 0x98, 0x05, 0x12, 0xb3, 0x6e, 0x28,
];

const OCML_EXP_F32: &str = "__ocml_exp_f32";
const OCML_EXP_ABI: &str = "C(f32[size=4,align=4])->f32[size=4,align=4]";
const OCML_EXP_EFFECTS: &str = "none";
const OCML_PROVIDER_IDENTITY: &str = "gfx942-ocml-v1";
const OCML_PROVIDER_BASENAMES: [&str; 4] = [
    "ocml.bc",
    "oclc_isa_version_942.bc",
    "oclc_unsafe_math_off.bc",
    "oclc_finite_only_off.bc",
];
const OCML_EXP_BOUNDARY: [u8; 32] = [
    0xdb, 0x91, 0x96, 0x57, 0x5c, 0xcc, 0xcc, 0xd8, 0x03, 0x53, 0xf5, 0xed, 0x04, 0xbc, 0x42, 0x5b,
    0x64, 0x34, 0x4a, 0x42, 0x07, 0x09, 0x79, 0x3e, 0xe8, 0x37, 0x79, 0xad, 0xd2, 0x1e, 0x47, 0x60,
];
const PROFILE_DOMAIN: &[u8] = b"FERRIC/QWEN3/PAGED-GQA-DECODE/PROFILE/V1\0";
const CATALOG_DOMAIN: &[u8] = b"FERRIC/QWEN3/PAGED-GQA-DECODE/CATALOG/V1\0";
const KERNEL_IR_DOMAIN: &[u8] = b"FERRIC/QWEN3/PAGED-GQA-DECODE/KERNEL-IR/V1\0";
const SOURCE_BINDING_DOMAIN: &[u8] = b"FERRIC/QWEN3/PAGED-GQA-DECODE/SOURCE-BINDING/V1\0";

/// Target or speculative-draft Qwen3 model role.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Qwen3PagedDecodeModelRoleV1 {
    /// Qwen3-8B target with QH32, KVH8, D128, and GQA4.
    Target8B = 1,
    /// Qwen3-0.6B draft with QH16, KVH8, D128, and GQA2.
    Draft06B = 2,
}

impl Qwen3PagedDecodeModelRoleV1 {
    /// Exact query-head count.
    #[must_use]
    pub const fn query_heads(self) -> u32 {
        match self {
            Self::Target8B => 32,
            Self::Draft06B => 16,
        }
    }

    /// Exact consecutive query heads sharing one KV head.
    #[must_use]
    pub const fn gqa_group_size(self) -> u32 {
        match self {
            Self::Target8B => 4,
            Self::Draft06B => 2,
        }
    }

    /// Exact pre-output-projection attention width.
    #[must_use]
    pub const fn query_width(self) -> u32 {
        self.query_heads() * QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1
    }
}

/// Closed Ferric paged decode bucket set.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Qwen3PagedDecodeBucketV1 {
    /// Ordinary decode, one sequence and one active token.
    DecodeS1C8192 = 1,
    /// Ordinary decode, eight sequences and one active token each.
    DecodeS8C8192 = 2,
    /// Ordinary decode, 32 sequences and one active token each.
    DecodeS32C8192 = 3,
    /// Speculative S1K4: target width five, draft width four.
    SpecS1K4C8192 = 4,
    /// Speculative S8K4: target width five, draft width four.
    SpecS8K4C8192 = 5,
    /// Speculative S1K8: target width nine, draft width eight.
    SpecS1K8C8192 = 6,
    /// Speculative S1K16: target width 17, draft width 16.
    SpecS1K16C8192 = 7,
}

impl Qwen3PagedDecodeBucketV1 {
    /// Exact independent sequence count.
    #[must_use]
    pub const fn sequences(self) -> u32 {
        match self {
            Self::DecodeS8C8192 | Self::SpecS8K4C8192 => 8,
            Self::DecodeS32C8192 => 32,
            Self::DecodeS1C8192
            | Self::SpecS1K4C8192
            | Self::SpecS1K8C8192
            | Self::SpecS1K16C8192 => 1,
        }
    }

    /// Exact active-token count per sequence for one model role.
    #[must_use]
    pub const fn active_tokens(self, role: Qwen3PagedDecodeModelRoleV1) -> u32 {
        match self {
            Self::DecodeS1C8192 | Self::DecodeS8C8192 | Self::DecodeS32C8192 => 1,
            Self::SpecS1K4C8192 | Self::SpecS8K4C8192 => match role {
                Qwen3PagedDecodeModelRoleV1::Target8B => 5,
                Qwen3PagedDecodeModelRoleV1::Draft06B => 4,
            },
            Self::SpecS1K8C8192 => match role {
                Qwen3PagedDecodeModelRoleV1::Target8B => 9,
                Qwen3PagedDecodeModelRoleV1::Draft06B => 8,
            },
            Self::SpecS1K16C8192 => match role {
                Qwen3PagedDecodeModelRoleV1::Target8B => 17,
                Qwen3PagedDecodeModelRoleV1::Draft06B => 16,
            },
        }
    }
}

/// Exact machine arithmetic declaration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Qwen3PagedDecodeNumericalPolicyV1 {
    /// BF16 widening, ascending D128 dot, online FP32 recurrence, existing OCML
    /// exp boundary, FP32 division, and BF16 RNE output.
    OnlineFp32OcmlExpBf16RneOutput = 1,
}

/// SHA-256 identity of one exact profile record.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Qwen3PagedDecodeProfileIdentityV1([u8; 32]);

impl Qwen3PagedDecodeProfileIdentityV1 {
    /// Returns the domain-separated identity bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// One exact checked target/draft paged-GQA paged decode profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Qwen3PagedDecodeProfileV1 {
    role: Qwen3PagedDecodeModelRoleV1,
    bucket: Qwen3PagedDecodeBucketV1,
    sequences: u32,
    active_tokens: u32,
    context_capacity: u32,
    query_heads: u32,
    gqa_group_size: u32,
    query_width: u32,
    query_elements: u64,
    cache_elements_each: u64,
    page_table_elements: u64,
    context_elements: u64,
    launch_workitems: [u32; 3],
    grid_workgroups: [u32; 3],
    numerical_policy: Qwen3PagedDecodeNumericalPolicyV1,
    identity: Qwen3PagedDecodeProfileIdentityV1,
}

impl Qwen3PagedDecodeProfileV1 {
    fn checked(
        role: Qwen3PagedDecodeModelRoleV1,
        bucket: Qwen3PagedDecodeBucketV1,
    ) -> Result<Self, Qwen3PagedDecodeCatalogErrorV1> {
        let sequences = bucket.sequences();
        let active_tokens = bucket.active_tokens(role);
        let context_capacity = 8_192;
        let query_heads = role.query_heads();
        let gqa_group_size = role.gqa_group_size();
        if query_heads.checked_div(gqa_group_size) != Some(QWEN3_PAGED_DECODE_KV_HEADS_V1) {
            return Err(Qwen3PagedDecodeCatalogErrorV1::GqaGeometry);
        }
        let query_width = query_heads
            .checked_mul(QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1)
            .ok_or(Qwen3PagedDecodeCatalogErrorV1::ExtentOverflow)?;
        let positions = u64::from(sequences)
            .checked_mul(u64::from(active_tokens))
            .ok_or(Qwen3PagedDecodeCatalogErrorV1::ExtentOverflow)?;
        let query_elements = positions
            .checked_mul(u64::from(query_width))
            .ok_or(Qwen3PagedDecodeCatalogErrorV1::ExtentOverflow)?;
        let cache_elements_each = u64::from(QWEN3_PAGED_DECODE_CACHE_POOL_PAGES_V1)
            .checked_mul(u64::from(QWEN3_PAGED_DECODE_PAGE_TOKENS_V1))
            .and_then(|value| value.checked_mul(u64::from(QWEN3_PAGED_DECODE_KV_HEADS_V1)))
            .and_then(|value| value.checked_mul(u64::from(QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1)))
            .ok_or(Qwen3PagedDecodeCatalogErrorV1::ExtentOverflow)?;
        let page_table_elements = u64::from(sequences)
            .checked_mul(u64::from(QWEN3_PAGED_DECODE_PAGE_TABLE_ENTRIES_V1))
            .ok_or(Qwen3PagedDecodeCatalogErrorV1::ExtentOverflow)?;
        let context_elements = u64::from(sequences);
        let vectors = u32::try_from(
            positions
                .checked_mul(u64::from(query_heads))
                .ok_or(Qwen3PagedDecodeCatalogErrorV1::GridOverflow)?,
        )
        .map_err(|_| Qwen3PagedDecodeCatalogErrorV1::GridOverflow)?;
        let workitems = vectors
            .checked_mul(QWEN3_PAGED_DECODE_WORKGROUP_V1[0])
            .ok_or(Qwen3PagedDecodeCatalogErrorV1::GridOverflow)?;
        let mut profile = Self {
            role,
            bucket,
            sequences,
            active_tokens,
            context_capacity,
            query_heads,
            gqa_group_size,
            query_width,
            query_elements,
            cache_elements_each,
            page_table_elements,
            context_elements,
            launch_workitems: [workitems, 1, 1],
            grid_workgroups: [vectors, 1, 1],
            numerical_policy: Qwen3PagedDecodeNumericalPolicyV1::OnlineFp32OcmlExpBf16RneOutput,
            identity: Qwen3PagedDecodeProfileIdentityV1([0; 32]),
        };
        profile.identity =
            Qwen3PagedDecodeProfileIdentityV1(hash(PROFILE_DOMAIN, &profile.encode()));
        Ok(profile)
    }

    fn encode(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(128);
        bytes.push(self.role as u8);
        bytes.push(self.bucket as u8);
        for value in [
            self.sequences,
            self.active_tokens,
            self.context_capacity,
            self.query_heads,
            QWEN3_PAGED_DECODE_KV_HEADS_V1,
            QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1,
            self.gqa_group_size,
            self.query_width,
            QWEN3_PAGED_DECODE_PAGE_TOKENS_V1,
            QWEN3_PAGED_DECODE_PAGE_TABLE_ENTRIES_V1,
            QWEN3_PAGED_DECODE_CACHE_POOL_PAGES_V1,
            QWEN3_PAGED_DECODE_ATTENTION_SCALE_BITS_V1,
        ] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in [
            self.query_elements,
            self.cache_elements_each,
            self.page_table_elements,
            self.context_elements,
        ] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in self
            .launch_workitems
            .into_iter()
            .chain(self.grid_workgroups)
        {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.push(self.numerical_policy as u8);
        bytes
    }

    /// Exact model role.
    #[must_use]
    pub const fn role(self) -> Qwen3PagedDecodeModelRoleV1 {
        self.role
    }

    /// Exact paged decode bucket.
    #[must_use]
    pub const fn bucket(self) -> Qwen3PagedDecodeBucketV1 {
        self.bucket
    }

    /// Exact sequence count.
    #[must_use]
    pub const fn sequences(self) -> u32 {
        self.sequences
    }

    /// Exact active tokens per sequence.
    #[must_use]
    pub const fn active_tokens(self) -> u32 {
        self.active_tokens
    }

    /// Exact maximum logical context per sequence.
    #[must_use]
    pub const fn context_capacity(self) -> u32 {
        self.context_capacity
    }

    /// Exact query-head count.
    #[must_use]
    pub const fn query_heads(self) -> u32 {
        self.query_heads
    }

    /// Exact GQA group size.
    #[must_use]
    pub const fn gqa_group_size(self) -> u32 {
        self.gqa_group_size
    }

    /// Exact attention width before O projection.
    #[must_use]
    pub const fn query_width(self) -> u32 {
        self.query_width
    }

    /// Exact BF16 query and, separately, output element count.
    #[must_use]
    pub const fn query_elements(self) -> u64 {
        self.query_elements
    }

    /// Exact BF16 key-cache and, separately, value-cache element count.
    #[must_use]
    pub const fn cache_elements_each(self) -> u64 {
        self.cache_elements_each
    }

    /// Exact `u32` page-index element count.
    #[must_use]
    pub const fn page_table_elements(self) -> u64 {
        self.page_table_elements
    }

    /// Exact `u32` committed-token count element count.
    #[must_use]
    pub const fn context_elements(self) -> u64 {
        self.context_elements
    }

    /// Exact global extent measured in workitems: `[S*A*QH*64,1,1]`.
    #[must_use]
    pub const fn launch_workitems(self) -> [u32; 3] {
        self.launch_workitems
    }

    /// Exact grid measured in Wave64 workgroups: `[S*A*QH,1,1]`.
    #[must_use]
    pub const fn grid_workgroups(self) -> [u32; 3] {
        self.grid_workgroups
    }

    /// Exact declared online-recurrence policy.
    #[must_use]
    pub const fn numerical_policy(self) -> Qwen3PagedDecodeNumericalPolicyV1 {
        self.numerical_policy
    }

    /// Exact domain-separated profile identity.
    #[must_use]
    pub const fn identity(self) -> Qwen3PagedDecodeProfileIdentityV1 {
        self.identity
    }

    /// A profile declaration is not numerical or operator-refinement evidence.
    #[must_use]
    pub const fn proves_operator_refinement(self) -> bool {
        false
    }
}

/// Finite catalog construction failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Qwen3PagedDecodeCatalogErrorV1 {
    /// Query-head quotient did not equal exact KVH8.
    GqaGeometry,
    /// Tensor extent arithmetic overflowed.
    ExtentOverflow,
    /// Workitem or workgroup arithmetic overflowed.
    GridOverflow,
    /// The catalog did not contain exactly fourteen distinct records.
    CatalogClosure,
}

impl fmt::Display for Qwen3PagedDecodeCatalogErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Qwen3 paged-GQA paged decode catalog failed: {self:?}"
        )
    }
}

impl std::error::Error for Qwen3PagedDecodeCatalogErrorV1 {}

/// Identity of the exact fourteen-profile catalog.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Qwen3PagedDecodeProfileCatalogIdentityV1([u8; 32]);

impl Qwen3PagedDecodeProfileCatalogIdentityV1 {
    /// Returns the exact catalog identity bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Complete finite target/draft B3 paged decode catalog.
#[derive(Debug, Eq, PartialEq)]
pub struct Qwen3PagedDecodeProfileCatalogV1 {
    profiles: Box<[Qwen3PagedDecodeProfileV1]>,
    canonical_bytes: Box<[u8]>,
    identity: Qwen3PagedDecodeProfileCatalogIdentityV1,
}

impl Qwen3PagedDecodeProfileCatalogV1 {
    /// Constructs the exact role-major, bucket-major catalog.
    ///
    /// # Errors
    ///
    /// Returns an error if any exact profile geometry or catalog extent is invalid.
    pub fn canonical() -> Result<Self, Qwen3PagedDecodeCatalogErrorV1> {
        let mut profiles = Vec::with_capacity(QWEN3_PAGED_DECODE_PROFILE_COUNT_V1);
        for role in [
            Qwen3PagedDecodeModelRoleV1::Target8B,
            Qwen3PagedDecodeModelRoleV1::Draft06B,
        ] {
            for bucket in [
                Qwen3PagedDecodeBucketV1::DecodeS1C8192,
                Qwen3PagedDecodeBucketV1::DecodeS8C8192,
                Qwen3PagedDecodeBucketV1::DecodeS32C8192,
                Qwen3PagedDecodeBucketV1::SpecS1K4C8192,
                Qwen3PagedDecodeBucketV1::SpecS8K4C8192,
                Qwen3PagedDecodeBucketV1::SpecS1K8C8192,
                Qwen3PagedDecodeBucketV1::SpecS1K16C8192,
            ] {
                profiles.push(Qwen3PagedDecodeProfileV1::checked(role, bucket)?);
            }
        }
        if profiles.len() != QWEN3_PAGED_DECODE_PROFILE_COUNT_V1
            || profiles.iter().enumerate().any(|(index, profile)| {
                profiles[index + 1..]
                    .iter()
                    .any(|other| profile.identity == other.identity)
            })
        {
            return Err(Qwen3PagedDecodeCatalogErrorV1::CatalogClosure);
        }
        let mut canonical_bytes = Vec::with_capacity(512);
        let profile_count = u32::try_from(profiles.len())
            .map_err(|_| Qwen3PagedDecodeCatalogErrorV1::CatalogClosure)?;
        canonical_bytes.extend_from_slice(&profile_count.to_le_bytes());
        canonical_bytes.extend_from_slice(QWEN3_PAGED_DECODE_TARGET_V1.as_bytes());
        canonical_bytes.push(QWEN3_PAGED_DECODE_CODE_OBJECT_VERSION_V1);
        for dimension in QWEN3_PAGED_DECODE_WORKGROUP_V1 {
            canonical_bytes.extend_from_slice(&dimension.to_le_bytes());
        }
        for profile in &profiles {
            let encoded = profile.encode();
            let encoded_len = u32::try_from(encoded.len())
                .map_err(|_| Qwen3PagedDecodeCatalogErrorV1::CatalogClosure)?;
            canonical_bytes.extend_from_slice(&encoded_len.to_le_bytes());
            canonical_bytes.extend_from_slice(&encoded);
            canonical_bytes.extend_from_slice(profile.identity.as_bytes());
        }
        let identity =
            Qwen3PagedDecodeProfileCatalogIdentityV1(hash(CATALOG_DOMAIN, &canonical_bytes));
        Ok(Self {
            profiles: profiles.into_boxed_slice(),
            canonical_bytes: canonical_bytes.into_boxed_slice(),
            identity,
        })
    }

    /// Exact stable-order profile slice.
    #[must_use]
    pub fn profiles(&self) -> &[Qwen3PagedDecodeProfileV1] {
        &self.profiles
    }

    /// Looks up one exact role/bucket pair.
    #[must_use]
    pub fn profile(
        &self,
        role: Qwen3PagedDecodeModelRoleV1,
        bucket: Qwen3PagedDecodeBucketV1,
    ) -> Option<Qwen3PagedDecodeProfileV1> {
        self.profiles
            .iter()
            .copied()
            .find(|profile| profile.role == role && profile.bucket == bucket)
    }

    /// Canonical bytes retaining every checked shape and launch unit.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    /// Exact catalog identity.
    #[must_use]
    pub const fn identity(&self) -> Qwen3PagedDecodeProfileCatalogIdentityV1 {
        self.identity
    }

    /// This structural roster grants no source, artifact, or launch authority.
    #[must_use]
    pub const fn grants_authority(&self) -> bool {
        false
    }
}

/// Semantic role of one six-slice ABI argument.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Qwen3PagedDecodeArgumentRoleV1 {
    /// Contiguous rotated query input.
    Query = 1,
    /// Paged rotated-key cache input.
    KeyCache = 2,
    /// Paged value-cache input.
    ValueCache = 3,
    /// Logical-to-physical page indices.
    PageIndices = 4,
    /// Committed-token count per sequence.
    CommittedTokens = 5,
    /// Contiguous attention output.
    Output = 6,
}

/// Tensor storage and logical shape for one ABI argument.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Qwen3PagedDecodeArgumentShapeV1 {
    /// BF16 `[S,T,QH,128]`.
    QueryBf16Bits = 1,
    /// BF16 `[16384,16,8,128]` fixed global physical-page pool.
    PagedKvCacheBf16Bits = 2,
    /// `u32 [S,512]`.
    PageIndicesU32 = 3,
    /// `u32 [S]`.
    CommittedTokensU32 = 4,
    /// BF16 `[S,A,QH,128]`.
    OutputBf16Bits = 5,
}

/// Scalar storage interpretation for one ABI argument.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Qwen3PagedDecodeScalarV1 {
    /// BF16 represented as `u16` storage bits.
    Bf16 = 1,
    /// Unsigned 32-bit physical page index.
    U32 = 2,
}

/// One exact pointer-plus-length ABI record.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Qwen3PagedDecodeArgumentV1 {
    /// Semantic tensor role.
    pub role: Qwen3PagedDecodeArgumentRoleV1,
    /// Exact logical storage shape.
    pub shape: Qwen3PagedDecodeArgumentShapeV1,
    /// Semantic scalar type.
    pub scalar: Qwen3PagedDecodeScalarV1,
    /// Explicit kernarg byte offset.
    pub offset: u32,
    /// Pointer-plus-length record size.
    pub size: u32,
    /// Record alignment.
    pub alignment: u32,
}

/// Exact online-recurrence step retained by the semantic KIR.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Qwen3PagedDecodeRecurrenceStepV1 {
    /// Ascending-feature D128 BF16-to-FP32 dot product.
    SequentialDotD128,
    /// Multiply by an exact FP32 bit pattern.
    ScaleByExactF32Bits(u32),
    /// Query head maps to KV head by exact quotient.
    QuotientGqaHeadMapping,
    /// Query position is committed count plus the active-token index.
    CommittedPlusActiveQueryPosition,
    /// Keys range from zero through the query position within one sequence.
    SameSequenceCausalPrefixInclusive,
    /// Map `key/16` through `[S,512]` into the global 16,384-page P16 pool.
    P16LogicalToPhysicalPageMapping,
    /// The first key initializes max, denominator, and numerator pair.
    FirstKeyInitializesState,
    /// Later keys update max and evaluate both OCML exponential weights.
    OnlineMaxAndTwoOcmlExpWeights,
    /// Denominator and adjacent numerator pair are rescaled sequentially.
    RescaleDenominatorAndNumeratorPair,
    /// Divide the numerator pair by the denominator.
    DivideNumeratorPairByDenominator,
    /// Narrow the adjacent FP32 pair to BF16 with RNE.
    NarrowOutputPairBf16Rne,
}

/// Per-workitem failure behavior.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Qwen3PagedDecodeExceptionalPolicyV1 {
    /// A workitem traps before its two owned stores. Other workgroups may
    /// already have stored output, so no whole-dispatch atomicity is claimed.
    PerLaneTrapBeforeOwnedPairNoGlobalAtomicity,
}

/// Exact Ferric semantic KIR for one role/bucket profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Qwen3PagedDecodeKernelIrV1 {
    module_id: String,
    kernel_id: String,
    arguments: [Qwen3PagedDecodeArgumentV1; 6],
    profile_identity: Qwen3PagedDecodeProfileIdentityV1,
    recurrence: [Qwen3PagedDecodeRecurrenceStepV1; 11],
    exceptional_policy: Qwen3PagedDecodeExceptionalPolicyV1,
    identity: [u8; 32],
}

impl Qwen3PagedDecodeKernelIrV1 {
    /// Ferric-owned semantic module identity.
    #[must_use]
    pub fn module_id(&self) -> &str {
        &self.module_id
    }

    /// Exact exported kernel identity.
    #[must_use]
    pub fn kernel_id(&self) -> &str {
        &self.kernel_id
    }

    /// Exact six-slice Q/paged-K/paged-V/page-index/committed/O ABI.
    #[must_use]
    pub const fn arguments(&self) -> &[Qwen3PagedDecodeArgumentV1; 6] {
        &self.arguments
    }

    /// Profile identity whose geometry this KIR retains.
    #[must_use]
    pub const fn profile_identity(&self) -> Qwen3PagedDecodeProfileIdentityV1 {
        self.profile_identity
    }

    /// Exact ordered online recurrence.
    #[must_use]
    pub const fn recurrence(&self) -> &[Qwen3PagedDecodeRecurrenceStepV1; 11] {
        &self.recurrence
    }

    /// Per-workitem exceptional behavior.
    #[must_use]
    pub const fn exceptional_policy(&self) -> Qwen3PagedDecodeExceptionalPolicyV1 {
        self.exceptional_policy
    }

    /// Domain-separated identity of every retained KIR field.
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

/// Constructs the canonical semantic KIR for one exact profile.
#[must_use]
pub fn qwen3_paged_decode_kernel_ir_v1(
    profile: Qwen3PagedDecodeProfileV1,
) -> Qwen3PagedDecodeKernelIrV1 {
    let arguments = [
        argument(
            Qwen3PagedDecodeArgumentRoleV1::Query,
            Qwen3PagedDecodeArgumentShapeV1::QueryBf16Bits,
            Qwen3PagedDecodeScalarV1::Bf16,
            0,
        ),
        argument(
            Qwen3PagedDecodeArgumentRoleV1::KeyCache,
            Qwen3PagedDecodeArgumentShapeV1::PagedKvCacheBf16Bits,
            Qwen3PagedDecodeScalarV1::Bf16,
            16,
        ),
        argument(
            Qwen3PagedDecodeArgumentRoleV1::ValueCache,
            Qwen3PagedDecodeArgumentShapeV1::PagedKvCacheBf16Bits,
            Qwen3PagedDecodeScalarV1::Bf16,
            32,
        ),
        argument(
            Qwen3PagedDecodeArgumentRoleV1::PageIndices,
            Qwen3PagedDecodeArgumentShapeV1::PageIndicesU32,
            Qwen3PagedDecodeScalarV1::U32,
            48,
        ),
        argument(
            Qwen3PagedDecodeArgumentRoleV1::CommittedTokens,
            Qwen3PagedDecodeArgumentShapeV1::CommittedTokensU32,
            Qwen3PagedDecodeScalarV1::U32,
            64,
        ),
        argument(
            Qwen3PagedDecodeArgumentRoleV1::Output,
            Qwen3PagedDecodeArgumentShapeV1::OutputBf16Bits,
            Qwen3PagedDecodeScalarV1::Bf16,
            80,
        ),
    ];
    let recurrence = [
        Qwen3PagedDecodeRecurrenceStepV1::SequentialDotD128,
        Qwen3PagedDecodeRecurrenceStepV1::ScaleByExactF32Bits(
            QWEN3_PAGED_DECODE_ATTENTION_SCALE_BITS_V1,
        ),
        Qwen3PagedDecodeRecurrenceStepV1::QuotientGqaHeadMapping,
        Qwen3PagedDecodeRecurrenceStepV1::CommittedPlusActiveQueryPosition,
        Qwen3PagedDecodeRecurrenceStepV1::SameSequenceCausalPrefixInclusive,
        Qwen3PagedDecodeRecurrenceStepV1::P16LogicalToPhysicalPageMapping,
        Qwen3PagedDecodeRecurrenceStepV1::FirstKeyInitializesState,
        Qwen3PagedDecodeRecurrenceStepV1::OnlineMaxAndTwoOcmlExpWeights,
        Qwen3PagedDecodeRecurrenceStepV1::RescaleDenominatorAndNumeratorPair,
        Qwen3PagedDecodeRecurrenceStepV1::DivideNumeratorPairByDenominator,
        Qwen3PagedDecodeRecurrenceStepV1::NarrowOutputPairBf16Rne,
    ];
    let exceptional_policy =
        Qwen3PagedDecodeExceptionalPolicyV1::PerLaneTrapBeforeOwnedPairNoGlobalAtomicity;
    let mut encoded = Vec::with_capacity(256);
    encoded.extend_from_slice(b"ferric::qwen3::paged_gqa_decode_v1");
    encoded.extend_from_slice(QWEN3_PAGED_DECODE_KERNEL_SYMBOL_V1.as_bytes());
    encoded.extend_from_slice(profile.identity.as_bytes());
    for value in arguments {
        encoded.extend_from_slice(&[value.role as u8, value.shape as u8, value.scalar as u8]);
        encoded.extend_from_slice(&value.offset.to_le_bytes());
        encoded.extend_from_slice(&value.size.to_le_bytes());
        encoded.extend_from_slice(&value.alignment.to_le_bytes());
    }
    for step in recurrence {
        encode_recurrence_step(step, &mut encoded);
    }
    encoded.push(1);
    Qwen3PagedDecodeKernelIrV1 {
        module_id: "ferric::qwen3::paged_gqa_decode_v1".to_owned(),
        kernel_id: QWEN3_PAGED_DECODE_KERNEL_SYMBOL_V1.to_owned(),
        arguments,
        profile_identity: profile.identity,
        recurrence,
        exceptional_policy,
        identity: hash(KERNEL_IR_DOMAIN, &encoded),
    }
}

const fn argument(
    role: Qwen3PagedDecodeArgumentRoleV1,
    shape: Qwen3PagedDecodeArgumentShapeV1,
    scalar: Qwen3PagedDecodeScalarV1,
    offset: u32,
) -> Qwen3PagedDecodeArgumentV1 {
    Qwen3PagedDecodeArgumentV1 {
        role,
        shape,
        scalar,
        offset,
        size: 16,
        alignment: 8,
    }
}

fn encode_recurrence_step(step: Qwen3PagedDecodeRecurrenceStepV1, bytes: &mut Vec<u8>) {
    match step {
        Qwen3PagedDecodeRecurrenceStepV1::SequentialDotD128 => bytes.push(1),
        Qwen3PagedDecodeRecurrenceStepV1::ScaleByExactF32Bits(bits) => {
            bytes.push(2);
            bytes.extend_from_slice(&bits.to_le_bytes());
        }
        Qwen3PagedDecodeRecurrenceStepV1::QuotientGqaHeadMapping => bytes.push(3),
        Qwen3PagedDecodeRecurrenceStepV1::CommittedPlusActiveQueryPosition => bytes.push(4),
        Qwen3PagedDecodeRecurrenceStepV1::SameSequenceCausalPrefixInclusive => bytes.push(5),
        Qwen3PagedDecodeRecurrenceStepV1::P16LogicalToPhysicalPageMapping => bytes.push(6),
        Qwen3PagedDecodeRecurrenceStepV1::FirstKeyInitializesState => bytes.push(7),
        Qwen3PagedDecodeRecurrenceStepV1::OnlineMaxAndTwoOcmlExpWeights => bytes.push(8),
        Qwen3PagedDecodeRecurrenceStepV1::RescaleDenominatorAndNumeratorPair => bytes.push(9),
        Qwen3PagedDecodeRecurrenceStepV1::DivideNumeratorPairByDenominator => bytes.push(10),
        Qwen3PagedDecodeRecurrenceStepV1::NarrowOutputPairBf16Rne => bytes.push(11),
    }
}

/// One of the six exact ABI memory regions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Qwen3PagedDecodeBufferV1 {
    /// Contiguous BF16 rotated query `[S,A,QH,128]`.
    Query = 1,
    /// BF16 key cache in the fixed global pool `[16384,16,8,128]`.
    KeyCache = 2,
    /// BF16 value cache in the fixed global pool `[16384,16,8,128]`.
    ValueCache = 3,
    /// `u32` physical-page indices `[S,512]`.
    PageIndices = 4,
    /// `u32` committed-token counts `[S]`.
    CommittedTokens = 5,
    /// Contiguous BF16 attention output `[S,A,QH,128]`.
    Output = 6,
}

/// Exact numerical-span admission failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Qwen3PagedDecodeBufferContractErrorV1 {
    /// A required address was zero.
    ZeroAddress(Qwen3PagedDecodeBufferV1),
    /// Byte length differed from the finite profile.
    ByteLength(Qwen3PagedDecodeBufferV1),
    /// Start address violated scalar alignment.
    Alignment(Qwen3PagedDecodeBufferV1),
    /// Exclusive end overflowed `u64`.
    RangeOverflow(Qwen3PagedDecodeBufferV1),
    /// Two exact graph regions overlapped.
    Aliasing,
}

impl fmt::Display for Qwen3PagedDecodeBufferContractErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Qwen3 paged decode buffer contract failed: {self:?}"
        )
    }
}

impl std::error::Error for Qwen3PagedDecodeBufferContractErrorV1 {}

/// Exact checked spans in Q/K/V/page-index/committed/O ABI order.
#[derive(Debug, Eq, PartialEq)]
pub struct Qwen3PagedDecodeBufferContractV1 {
    addresses: [u64; 6],
    ends: [u64; 6],
    byte_lengths: [u64; 6],
}

impl Qwen3PagedDecodeBufferContractV1 {
    /// Checks exact byte lengths, alignment, range overflow, and pairwise
    /// disjointness. It does not inspect page-index or cache content.
    ///
    /// # Errors
    ///
    /// Returns an error for a zero, misaligned, overflowing, incorrectly sized,
    /// or overlapping buffer span.
    pub fn checked(
        profile: Qwen3PagedDecodeProfileV1,
        addresses: [u64; 6],
        byte_lengths: [u64; 6],
    ) -> Result<Self, Qwen3PagedDecodeBufferContractErrorV1> {
        let query_bytes = profile.query_elements.checked_mul(2).ok_or(
            Qwen3PagedDecodeBufferContractErrorV1::ByteLength(Qwen3PagedDecodeBufferV1::Query),
        )?;
        let cache_bytes = profile.cache_elements_each.checked_mul(2).ok_or(
            Qwen3PagedDecodeBufferContractErrorV1::ByteLength(Qwen3PagedDecodeBufferV1::KeyCache),
        )?;
        let page_bytes = profile.page_table_elements.checked_mul(4).ok_or(
            Qwen3PagedDecodeBufferContractErrorV1::ByteLength(
                Qwen3PagedDecodeBufferV1::PageIndices,
            ),
        )?;
        let context_bytes = profile.context_elements.checked_mul(4).ok_or(
            Qwen3PagedDecodeBufferContractErrorV1::ByteLength(
                Qwen3PagedDecodeBufferV1::CommittedTokens,
            ),
        )?;
        let expected = [
            query_bytes,
            cache_bytes,
            cache_bytes,
            page_bytes,
            context_bytes,
            query_bytes,
        ];
        let roles = [
            Qwen3PagedDecodeBufferV1::Query,
            Qwen3PagedDecodeBufferV1::KeyCache,
            Qwen3PagedDecodeBufferV1::ValueCache,
            Qwen3PagedDecodeBufferV1::PageIndices,
            Qwen3PagedDecodeBufferV1::CommittedTokens,
            Qwen3PagedDecodeBufferV1::Output,
        ];
        let alignments = [2_u64, 2, 2, 4, 4, 2];
        let mut ends = [0_u64; 6];
        for index in 0..6 {
            if addresses[index] == 0 {
                return Err(Qwen3PagedDecodeBufferContractErrorV1::ZeroAddress(
                    roles[index],
                ));
            }
            if byte_lengths[index] != expected[index] {
                return Err(Qwen3PagedDecodeBufferContractErrorV1::ByteLength(
                    roles[index],
                ));
            }
            if !addresses[index].is_multiple_of(alignments[index]) {
                return Err(Qwen3PagedDecodeBufferContractErrorV1::Alignment(
                    roles[index],
                ));
            }
            ends[index] = addresses[index].checked_add(byte_lengths[index]).ok_or(
                Qwen3PagedDecodeBufferContractErrorV1::RangeOverflow(roles[index]),
            )?;
        }
        for left in 0..6 {
            for right in left + 1..6 {
                if addresses[left] < ends[right] && addresses[right] < ends[left] {
                    return Err(Qwen3PagedDecodeBufferContractErrorV1::Aliasing);
                }
            }
        }
        Ok(Self {
            addresses,
            ends,
            byte_lengths,
        })
    }

    /// Exact starts in ABI role order.
    #[must_use]
    pub const fn addresses(&self) -> [u64; 6] {
        self.addresses
    }

    /// Exact exclusive ends in ABI role order.
    #[must_use]
    pub const fn ends(&self) -> [u64; 6] {
        self.ends
    }

    /// Exact byte lengths in ABI role order.
    #[must_use]
    pub const fn byte_lengths(&self) -> [u64; 6] {
        self.byte_lengths
    }

    /// Integer spans do not authenticate mappings, leases, or content.
    #[must_use]
    pub const fn authenticates_device_memory(&self) -> bool {
        false
    }
}

/// Four inert identities labeling source, KIR, schedule, and target-plan stages.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Qwen3PagedDecodeSourceBindingsV1 {
    source: [u8; 32],
    kernel_ir: [u8; 32],
    schedule: [u8; 32],
    target_plan: [u8; 32],
}

impl Qwen3PagedDecodeSourceBindingsV1 {
    /// Constructs inert labels. Preparation requires all four to be nonzero
    /// and distinct.
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

    /// Caller labels authenticate no source, producer, compiler, or plan.
    #[must_use]
    pub const fn authenticates_provenance(self) -> bool {
        false
    }
}

/// Failure while preparing the Ferric-owned direct-LLVM handoff.
#[derive(Debug)]
pub enum PrepareQwen3PagedDecodeKernelErrorV1 {
    /// A source label was zero or reused for another role.
    SourceBindings,
    /// The finite profile catalog failed closed.
    Catalog(Qwen3PagedDecodeCatalogErrorV1),
    /// A canonical semantic KIR record drifted.
    KernelIr,
    /// The exact direct-LLVM body failed its closed structural classifier.
    CompilerModule,
    /// The exact unresolved OCML contract envelope failed closed.
    CompilerEnvelope(CompilerFfiEnvelopeError),
    /// The exact entry/descriptor/import manifest failed closed.
    SymbolManifest(CompilerModuleSymbolManifestErrorV1),
    /// The generic Handoff V2 compiler-module container failed closed.
    CompilerHandoff(CompilerModuleHandoffErrorV2),
}

impl fmt::Display for PrepareQwen3PagedDecodeKernelErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Qwen3 paged decode preparation failed: {self:?}")
    }
}

impl std::error::Error for PrepareQwen3PagedDecodeKernelErrorV1 {}

/// Linear Ferric-owned source/KIR catalog and generic compiler handoff.
pub struct PreparedQwen3PagedDecodeKernelV1 {
    catalog: Qwen3PagedDecodeProfileCatalogV1,
    source_binding_identity: [u8; 32],
    llvm_sha256: [u8; 32],
    compiler_handoff_identity: CompilerModuleHandoffIdentityV2,
    manifest_identity: CompilerModuleSymbolManifestIdentityV1,
    compiler_handoff: CompilerModuleHandoffV2,
}

impl fmt::Debug for PreparedQwen3PagedDecodeKernelV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedQwen3PagedDecodeKernelV1")
            .field("catalog", &self.catalog.identity)
            .field("source_binding", &self.source_binding_identity)
            .field("llvm_sha256", &self.llvm_sha256)
            .field("compiler_handoff", &self.compiler_handoff_identity)
            .finish_non_exhaustive()
    }
}

impl PreparedQwen3PagedDecodeKernelV1 {
    /// Complete finite profile catalog retained by this owner.
    #[must_use]
    pub const fn catalog(&self) -> &Qwen3PagedDecodeProfileCatalogV1 {
        &self.catalog
    }

    /// Identity binding inert stage labels, the exact catalog, every KIR, and
    /// the canonical LLVM body. It authenticates no external producer.
    #[must_use]
    pub const fn source_binding_identity(&self) -> &[u8; 32] {
        &self.source_binding_identity
    }

    /// SHA-256 of the exact canonical direct-LLVM body.
    #[must_use]
    pub const fn llvm_sha256(&self) -> &[u8; 32] {
        &self.llvm_sha256
    }

    /// Complete canonical compiler-handoff identity.
    #[must_use]
    pub const fn compiler_handoff_identity(&self) -> CompilerModuleHandoffIdentityV2 {
        self.compiler_handoff_identity
    }

    /// Closed entry/descriptor/import manifest identity.
    #[must_use]
    pub const fn manifest_identity(&self) -> CompilerModuleSymbolManifestIdentityV1 {
        self.manifest_identity
    }

    /// Borrows the exact Handoff V2 compiler module for attempt publication.
    #[must_use]
    pub const fn compiler_handoff(&self) -> &CompilerModuleHandoffV2 {
        &self.compiler_handoff
    }

    /// Handoff V2 cannot represent the OCML exp intrinsic, so this lane uses
    /// the public bounded direct-LLVM/OCML Worker route. This does not inherit
    /// any prior source or compiler authority.
    #[must_use]
    pub const fn uses_typed_handoff_v2_source(&self) -> bool {
        false
    }

    /// The source binding does not authenticate compiler origin.
    #[must_use]
    pub const fn authenticates_compiler_origin(&self) -> bool {
        false
    }

    /// Online recurrence remains unreconciled with the separate two-pass host reference.
    #[must_use]
    pub const fn proves_operator_or_numerical_refinement(&self) -> bool {
        false
    }

    /// Exact profile selection is not yet joined to Ferric plan identity.
    #[must_use]
    pub const fn has_ferric_plan_identity_join(&self) -> bool {
        false
    }

    /// This compiler slice does not close the kernel schedule catalog.
    #[must_use]
    pub const fn has_kernel_schedule_catalog_join(&self) -> bool {
        false
    }

    /// Exact source/profile structure grants no artifact or launch authority.
    #[must_use]
    pub const fn grants_launch_authority(&self) -> bool {
        false
    }
}

/// Constructs the exact catalog, KIR family, LLVM source, OCML import envelope,
/// and generic compiler handoff.
///
/// # Errors
///
/// Returns an error if source labels, profile construction, KIR construction,
/// the OCML FFI boundary, symbol manifest, or compiler handoff is invalid.
pub fn prepare_qwen3_paged_decode_kernel_v1(
    bindings: Qwen3PagedDecodeSourceBindingsV1,
) -> Result<PreparedQwen3PagedDecodeKernelV1, PrepareQwen3PagedDecodeKernelErrorV1> {
    validate_source_bindings(bindings)?;
    let catalog = Qwen3PagedDecodeProfileCatalogV1::canonical()
        .map_err(PrepareQwen3PagedDecodeKernelErrorV1::Catalog)?;
    let mut kir_identities = Vec::with_capacity(QWEN3_PAGED_DECODE_PROFILE_COUNT_V1 * 32);
    for profile in catalog.profiles() {
        let kir = qwen3_paged_decode_kernel_ir_v1(*profile);
        if kir.profile_identity() != profile.identity()
            || kir.arguments()[3].shape != Qwen3PagedDecodeArgumentShapeV1::PageIndicesU32
            || kir.arguments()[4].shape != Qwen3PagedDecodeArgumentShapeV1::CommittedTokensU32
            || kir.recurrence()[5]
                != Qwen3PagedDecodeRecurrenceStepV1::P16LogicalToPhysicalPageMapping
        {
            return Err(PrepareQwen3PagedDecodeKernelErrorV1::KernelIr);
        }
        kir_identities.extend_from_slice(kir.identity());
    }
    let llvm = canonical_qwen3_paged_decode_llvm();
    validate_canonical_llvm(&llvm)?;
    let llvm_sha256: [u8; 32] = Sha256::digest(llvm.as_bytes()).into();
    let mut source_preimage = Vec::with_capacity(32 * 7);
    source_preimage.extend_from_slice(&bindings.source);
    source_preimage.extend_from_slice(&bindings.kernel_ir);
    source_preimage.extend_from_slice(&bindings.schedule);
    source_preimage.extend_from_slice(&bindings.target_plan);
    source_preimage.extend_from_slice(catalog.identity.as_bytes());
    source_preimage.extend_from_slice(&kir_identities);
    source_preimage.extend_from_slice(&llvm_sha256);
    let source_binding_identity = hash(SOURCE_BINDING_DOMAIN, &source_preimage);
    let target = exact_target();
    let envelope = exact_ocml_envelope(target)?;
    let manifest = CompilerModuleSymbolManifestV1::new([
        (
            CompilerModuleSymbolRoleV1::KernelEntry,
            QWEN3_PAGED_DECODE_KERNEL_SYMBOL_V1,
        ),
        (
            CompilerModuleSymbolRoleV1::KernelDescriptor,
            QWEN3_PAGED_DECODE_KERNEL_DESCRIPTOR_SYMBOL_V1,
        ),
        (
            CompilerModuleSymbolRoleV1::UnresolvedExternalImport,
            OCML_EXP_F32,
        ),
    ])
    .map_err(PrepareQwen3PagedDecodeKernelErrorV1::SymbolManifest)?;
    let manifest_identity = manifest.identity();
    let compiler_handoff = CompilerModuleHandoffV2::new(
        CompilerModuleKindV1::LlvmTextIr,
        target,
        CodeObjectVersion::V6,
        envelope,
        manifest,
        llvm.as_bytes(),
    )
    .map_err(PrepareQwen3PagedDecodeKernelErrorV1::CompilerHandoff)?;
    let compiler_handoff_identity = compiler_handoff.identity();
    Ok(PreparedQwen3PagedDecodeKernelV1 {
        catalog,
        source_binding_identity,
        llvm_sha256,
        compiler_handoff_identity,
        manifest_identity,
        compiler_handoff,
    })
}

fn validate_source_bindings(
    bindings: Qwen3PagedDecodeSourceBindingsV1,
) -> Result<(), PrepareQwen3PagedDecodeKernelErrorV1> {
    let identities = [
        bindings.source,
        bindings.kernel_ir,
        bindings.schedule,
        bindings.target_plan,
    ];
    for (index, identity) in identities.iter().enumerate() {
        if identity == &[0; 32] || identities[index + 1..].contains(identity) {
            return Err(PrepareQwen3PagedDecodeKernelErrorV1::SourceBindings);
        }
    }
    Ok(())
}

fn exact_ocml_envelope(
    target: DeviceTargetV1,
) -> Result<fe2o3_compiler_ffi::CompilerFfiEnvelopeV1, PrepareQwen3PagedDecodeKernelErrorV1> {
    let semantic_text = lower_hex(&OCML_EXP_BOUNDARY);
    let fields = DeviceFfiContractFieldsV1 {
        direction: DEVICE_FFI_DIRECTION_IMPORT_V1,
        symbol: OCML_EXP_F32,
        calling_convention: "C",
        code_object_version: u16::from(QWEN3_PAGED_DECODE_CODE_OBJECT_VERSION_V1),
        target: QWEN3_PAGED_DECODE_TARGET_V1,
        physical_abi: OCML_EXP_ABI,
        effects: OCML_EXP_EFFECTS,
        semantic_identity: &semantic_text,
    };
    let contract = CompilerFfiContractV1::new(
        derive_device_ffi_contract_id_v1(fields),
        DeviceFfiDirectionV1::Import,
        CompilerFfiLinkRoleV1::RequiresExternalDefinition,
        target,
        CodeObjectVersion::V6,
        CompilerFfiSourceOwnerV1::new(
            "ferric_qwen_kernels",
            "ferric_qwen_kernels::paged_decode::__ocml_exp_f32",
            [0x50; 16],
            "__ferric_qwen3_paged_decode_ocml_exp_f32_v1",
        )
        .map_err(PrepareQwen3PagedDecodeKernelErrorV1::CompilerEnvelope)?,
        OCML_EXP_F32,
        OCML_EXP_ABI,
        OCML_EXP_EFFECTS,
        OCML_EXP_BOUNDARY,
    )
    .map_err(PrepareQwen3PagedDecodeKernelErrorV1::CompilerEnvelope)?;
    let mut builder = CompilerFfiEnvelopeBuilderV1::new(target, CodeObjectVersion::V6, 1)
        .map_err(PrepareQwen3PagedDecodeKernelErrorV1::CompilerEnvelope)?;
    builder
        .push(contract)
        .map_err(PrepareQwen3PagedDecodeKernelErrorV1::CompilerEnvelope)?;
    builder
        .finish()
        .map_err(PrepareQwen3PagedDecodeKernelErrorV1::CompilerEnvelope)
}

fn lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

fn canonical_qwen3_paged_decode_llvm() -> String {
    let mut output = String::with_capacity(96 * 1024);
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
declare void @llvm.trap()
declare float @__ocml_exp_f32(float)

define amdgpu_kernel void @qwen3_paged_gqa_decode_bf16_f32_v1(ptr addrspace(1) nocapture readonly align 2 %q.data, i64 %q.len, ptr addrspace(1) nocapture readonly align 2 %k.data, i64 %k.len, ptr addrspace(1) nocapture readonly align 2 %v.data, i64 %v.len, ptr addrspace(1) nocapture readonly align 4 %pages.data, i64 %pages.len, ptr addrspace(1) nocapture readonly align 4 %committed.data, i64 %committed.len, ptr addrspace(1) noalias nocapture writeonly align 2 %output.data, i64 %output.len) #0 !reqd_work_group_size !0 !kernel_arg_access_qual !1 !kernel_arg_type !2 !kernel_arg_base_type !2 !kernel_arg_type_qual !3 {
entry:
  %local.i32 = call i32 @llvm.amdgcn.workitem.id.x()
  %group.i32 = call i32 @llvm.amdgcn.workgroup.id.x()
  %local = zext i32 %local.i32 to i64
  %group = zext i32 %group.i32 to i64
  %group.base = mul nuw i64 %group, 64
  %global = add nuw i64 %group.base, %local
  %local.ok = icmp ult i64 %local, 64

  %q.t.decode.s1 = icmp eq i64 %q.len, 4096
  %q.t.decode.s8 = icmp eq i64 %q.len, 32768
  %q.t.decode.s32 = icmp eq i64 %q.len, 131072
  %q.t.spec.s1k4 = icmp eq i64 %q.len, 20480
  %q.t.spec.s8k4 = icmp eq i64 %q.len, 163840
  %q.t.spec.s1k8 = icmp eq i64 %q.len, 36864
  %q.t.spec.s1k16 = icmp eq i64 %q.len, 69632
  %q.d.decode.s1 = icmp eq i64 %q.len, 2048
  %q.d.decode.s8 = icmp eq i64 %q.len, 16384
  %q.d.decode.s32 = icmp eq i64 %q.len, 65536
  %q.d.spec.s1k4 = icmp eq i64 %q.len, 8192
  %q.d.spec.s8k4 = icmp eq i64 %q.len, 65536
  %q.d.spec.s1k8 = icmp eq i64 %q.len, 16384
  %q.d.spec.s1k16 = icmp eq i64 %q.len, 32768
  %cache.global = icmp eq i64 %k.len, 268435456
  %value.cache.eq = icmp eq i64 %v.len, %k.len
  %pages.s1 = icmp eq i64 %pages.len, 512
  %pages.s8 = icmp eq i64 %pages.len, 4096
  %pages.s32 = icmp eq i64 %pages.len, 16384
  %committed.s1 = icmp eq i64 %committed.len, 1
  %committed.s8 = icmp eq i64 %committed.len, 8
  %committed.s32 = icmp eq i64 %committed.len, 32
  %output.eq = icmp eq i64 %output.len, %q.len

  %case.t.decode.s1.0 = and i1 %q.t.decode.s1, %pages.s1
  %case.t.decode.s1 = and i1 %case.t.decode.s1.0, %committed.s1
  %case.t.decode.s8.0 = and i1 %q.t.decode.s8, %pages.s8
  %case.t.decode.s8 = and i1 %case.t.decode.s8.0, %committed.s8
  %case.t.decode.s32.0 = and i1 %q.t.decode.s32, %pages.s32
  %case.t.decode.s32 = and i1 %case.t.decode.s32.0, %committed.s32
  %case.t.spec.s1k4.0 = and i1 %q.t.spec.s1k4, %pages.s1
  %case.t.spec.s1k4 = and i1 %case.t.spec.s1k4.0, %committed.s1
  %case.t.spec.s8k4.0 = and i1 %q.t.spec.s8k4, %pages.s8
  %case.t.spec.s8k4 = and i1 %case.t.spec.s8k4.0, %committed.s8
  %case.t.spec.s1k8.0 = and i1 %q.t.spec.s1k8, %pages.s1
  %case.t.spec.s1k8 = and i1 %case.t.spec.s1k8.0, %committed.s1
  %case.t.spec.s1k16.0 = and i1 %q.t.spec.s1k16, %pages.s1
  %case.t.spec.s1k16 = and i1 %case.t.spec.s1k16.0, %committed.s1
  %case.d.decode.s1.0 = and i1 %q.d.decode.s1, %pages.s1
  %case.d.decode.s1 = and i1 %case.d.decode.s1.0, %committed.s1
  %case.d.decode.s8.0 = and i1 %q.d.decode.s8, %pages.s8
  %case.d.decode.s8 = and i1 %case.d.decode.s8.0, %committed.s8
  %case.d.decode.s32.0 = and i1 %q.d.decode.s32, %pages.s32
  %case.d.decode.s32 = and i1 %case.d.decode.s32.0, %committed.s32
  %case.d.spec.s1k4.0 = and i1 %q.d.spec.s1k4, %pages.s1
  %case.d.spec.s1k4 = and i1 %case.d.spec.s1k4.0, %committed.s1
  %case.d.spec.s8k4.0 = and i1 %q.d.spec.s8k4, %pages.s8
  %case.d.spec.s8k4 = and i1 %case.d.spec.s8k4.0, %committed.s8
  %case.d.spec.s1k8.0 = and i1 %q.d.spec.s1k8, %pages.s1
  %case.d.spec.s1k8 = and i1 %case.d.spec.s1k8.0, %committed.s1
  %case.d.spec.s1k16.0 = and i1 %q.d.spec.s1k16, %pages.s1
  %case.d.spec.s1k16 = and i1 %case.d.spec.s1k16.0, %committed.s1

  %target.0 = or i1 %case.t.decode.s1, %case.t.decode.s8
  %target.1 = or i1 %case.t.decode.s32, %case.t.spec.s1k4
  %target.2 = or i1 %case.t.spec.s8k4, %case.t.spec.s1k8
  %target.3 = or i1 %target.0, %target.1
  %target.4 = or i1 %target.2, %case.t.spec.s1k16
  %target = or i1 %target.3, %target.4
  %draft.0 = or i1 %case.d.decode.s1, %case.d.decode.s8
  %draft.1 = or i1 %case.d.decode.s32, %case.d.spec.s1k4
  %draft.2 = or i1 %case.d.spec.s8k4, %case.d.spec.s1k8
  %draft.3 = or i1 %draft.0, %draft.1
  %draft.4 = or i1 %draft.2, %case.d.spec.s1k16
  %draft = or i1 %draft.3, %draft.4
  %known.profile = or i1 %target, %draft
  %known.cache = and i1 %cache.global, %value.cache.eq
  %known.output = and i1 %known.profile, %output.eq
  %known.shape = and i1 %known.output, %known.cache
  %shape.ok = and i1 %known.shape, %local.ok
  br i1 %shape.ok, label %shape.selected, label %trap

shape.selected:
  %sequences.short = select i1 %pages.s8, i64 8, i64 1
  %sequences = select i1 %pages.s32, i64 32, i64 %sequences.short
  %heads = select i1 %target, i64 32, i64 16
  %gqa = select i1 %target, i64 4, i64 2
  %active.one.t0 = or i1 %case.t.decode.s1, %case.t.decode.s8
  %active.one.t = or i1 %active.one.t0, %case.t.decode.s32
  %active.one.d0 = or i1 %case.d.decode.s1, %case.d.decode.s8
  %active.one.d = or i1 %active.one.d0, %case.d.decode.s32
  %active.one = or i1 %active.one.t, %active.one.d
  %active.four = or i1 %case.d.spec.s1k4, %case.d.spec.s8k4
  %active.five = or i1 %case.t.spec.s1k4, %case.t.spec.s8k4
  %active.short = select i1 %active.four, i64 4, i64 5
  %active.eight = select i1 %case.d.spec.s1k8, i64 8, i64 9
  %active.long = select i1 %case.d.spec.s1k16, i64 16, i64 17
  %active.k4 = or i1 %active.four, %active.five
  %active.not.one = select i1 %active.k4, i64 %active.short, i64 %active.eight
  %active.k16 = or i1 %case.t.spec.s1k16, %case.d.spec.s1k16
  %active.spec = select i1 %active.k16, i64 %active.long, i64 %active.not.one
  %active = select i1 %active.one, i64 1, i64 %active.spec
  %workitems = lshr exact i64 %q.len, 1
  %global.ok = icmp ult i64 %global, %workitems
  br i1 %global.ok, label %indices, label %trap

indices:
  %vector = lshr exact i64 %global, 6
  %query.head = urem i64 %vector, %heads
  %position = udiv i64 %vector, %heads
  %query.token = urem i64 %position, %active
  %sequence = udiv i64 %position, %active
  %sequence.ok = icmp ult i64 %sequence, %sequences
  %kv.head = udiv i64 %query.head, %gqa
  %kv.head.ok = icmp ult i64 %kv.head, 8
  %indices.ok = and i1 %sequence.ok, %kv.head.ok
  %q.base = mul nuw i64 %vector, 128
  %column = shl nuw nsw i64 %local, 1
  br i1 %indices.ok, label %context, label %trap

context:
  %committed.ptr = getelementptr inbounds i32, ptr addrspace(1) %committed.data, i64 %sequence
  %committed.i32 = load i32, ptr addrspace(1) %committed.ptr, align 4
  %committed = zext i32 %committed.i32 to i64
  %resident = add nuw i64 %committed, %active
  %query.absolute = add nuw i64 %committed, %query.token
  %committed.ok = icmp ult i64 %committed, 8192
  %resident.ok = icmp ule i64 %resident, 8192
  %context.ok = and i1 %committed.ok, %resident.ok
  br i1 %context.ok, label %initial.page, label %trap

initial.page:
",
    );
    emit_page_mapping(&mut output, "initial", "0");
    output.push_str("  br i1 %initial.page.ok, label %initial.score.entry, label %trap\n\n");
    emit_score(&mut output, "initial");
    output.push_str(
        r"  br i1 %initial.score.finite, label %initial.value, label %trap

initial.value:
  %initial.value0.index = add nuw i64 %initial.cache.base, %column
  %initial.value1.index = add nuw i64 %initial.value0.index, 1
  %initial.value0.ptr = getelementptr inbounds i16, ptr addrspace(1) %v.data, i64 %initial.value0.index
  %initial.value1.ptr = getelementptr inbounds i16, ptr addrspace(1) %v.data, i64 %initial.value1.index
  %initial.value0.bf16 = load i16, ptr addrspace(1) %initial.value0.ptr, align 2
  %initial.value1.bf16 = load i16, ptr addrspace(1) %initial.value1.ptr, align 2
  %initial.value0.wide = zext i16 %initial.value0.bf16 to i32
  %initial.value1.wide = zext i16 %initial.value1.bf16 to i32
  %initial.value0.bits = shl nuw i32 %initial.value0.wide, 16
  %initial.value1.bits = shl nuw i32 %initial.value1.wide, 16
  %initial.value0 = bitcast i32 %initial.value0.bits to float
  %initial.value1 = bitcast i32 %initial.value1.bits to float
  %initial.value0.exp = and i32 %initial.value0.bits, 2139095040
  %initial.value1.exp = and i32 %initial.value1.bits, 2139095040
  %initial.value0.finite = icmp ne i32 %initial.value0.exp, 2139095040
  %initial.value1.finite = icmp ne i32 %initial.value1.exp, 2139095040
  %initial.values.finite = and i1 %initial.value0.finite, %initial.value1.finite
  br i1 %initial.values.finite, label %recur.cond, label %trap

recur.cond:
  %key = phi i64 [ 1, %initial.value ], [ %next.key, %recur.ok ]
  %running.max = phi float [ %initial.score, %initial.value ], [ %next.max, %recur.ok ]
  %running.sum = phi float [ 1.000000e+00, %initial.value ], [ %next.sum, %recur.ok ]
  %numerator0 = phi float [ %initial.value0, %initial.value ], [ %next.numerator0, %recur.ok ]
  %numerator1 = phi float [ %initial.value1, %initial.value ], [ %next.numerator1, %recur.ok ]
  %recur.more = icmp ule i64 %key, %query.absolute
  br i1 %recur.more, label %next.page.entry, label %finish

next.page.entry:
",
    );
    emit_page_mapping(&mut output, "next", "%key");
    output.push_str("  br i1 %next.page.ok, label %next.score.entry, label %trap\n\n");
    emit_score(&mut output, "next");
    output.push_str(
        r#"  br i1 %next.score.finite, label %recur.score.ok, label %trap

recur.score.ok:
  %score.greater = fcmp ogt float %next.score, %running.max
  %next.max = select i1 %score.greater, float %next.score, float %running.max
  %previous.delta = fsub float %running.max, %next.max
  %current.delta = fsub float %next.score, %next.max
  %previous.weight = call float @__ocml_exp_f32(float %previous.delta)
  %current.weight = call float @__ocml_exp_f32(float %current.delta)
  %previous.weight.bits = bitcast float %previous.weight to i32
  %current.weight.bits = bitcast float %current.weight to i32
  %previous.weight.exp = and i32 %previous.weight.bits, 2139095040
  %current.weight.exp = and i32 %current.weight.bits, 2139095040
  %previous.weight.finite = icmp ne i32 %previous.weight.exp, 2139095040
  %current.weight.finite = icmp ne i32 %current.weight.exp, 2139095040
  %weights.finite = and i1 %previous.weight.finite, %current.weight.finite
  %next.value0.index = add nuw i64 %next.cache.base, %column
  %next.value1.index = add nuw i64 %next.value0.index, 1
  %next.value0.ptr = getelementptr inbounds i16, ptr addrspace(1) %v.data, i64 %next.value0.index
  %next.value1.ptr = getelementptr inbounds i16, ptr addrspace(1) %v.data, i64 %next.value1.index
  %next.value0.bf16 = load i16, ptr addrspace(1) %next.value0.ptr, align 2
  %next.value1.bf16 = load i16, ptr addrspace(1) %next.value1.ptr, align 2
  %next.value0.wide = zext i16 %next.value0.bf16 to i32
  %next.value1.wide = zext i16 %next.value1.bf16 to i32
  %next.value0.bits = shl nuw i32 %next.value0.wide, 16
  %next.value1.bits = shl nuw i32 %next.value1.wide, 16
  %next.value0 = bitcast i32 %next.value0.bits to float
  %next.value1 = bitcast i32 %next.value1.bits to float
  %next.value0.exp = and i32 %next.value0.bits, 2139095040
  %next.value1.exp = and i32 %next.value1.bits, 2139095040
  %next.value0.finite = icmp ne i32 %next.value0.exp, 2139095040
  %next.value1.finite = icmp ne i32 %next.value1.exp, 2139095040
  %next.values.finite = and i1 %next.value0.finite, %next.value1.finite
  %weighted.sum = fmul float %running.sum, %previous.weight
  %next.sum = fadd float %weighted.sum, %current.weight
  %weighted.numerator0 = fmul float %numerator0, %previous.weight
  %weighted.current0 = fmul float %next.value0, %current.weight
  %next.numerator0 = fadd float %weighted.numerator0, %weighted.current0
  %weighted.numerator1 = fmul float %numerator1, %previous.weight
  %weighted.current1 = fmul float %next.value1, %current.weight
  %next.numerator1 = fadd float %weighted.numerator1, %weighted.current1
  %next.sum.bits = bitcast float %next.sum to i32
  %next.numerator0.bits = bitcast float %next.numerator0 to i32
  %next.numerator1.bits = bitcast float %next.numerator1 to i32
  %next.sum.exp = and i32 %next.sum.bits, 2139095040
  %next.numerator0.exp = and i32 %next.numerator0.bits, 2139095040
  %next.numerator1.exp = and i32 %next.numerator1.bits, 2139095040
  %next.sum.finite = icmp ne i32 %next.sum.exp, 2139095040
  %next.numerator0.finite = icmp ne i32 %next.numerator0.exp, 2139095040
  %next.numerator1.finite = icmp ne i32 %next.numerator1.exp, 2139095040
  %next.sum.positive = fcmp ogt float %next.sum, 0.000000e+00
  %recur.valid.0 = and i1 %weights.finite, %next.values.finite
  %recur.valid.1 = and i1 %recur.valid.0, %next.sum.finite
  %recur.valid.2 = and i1 %recur.valid.1, %next.sum.positive
  %recur.valid.3 = and i1 %recur.valid.2, %next.numerator0.finite
  %recur.valid = and i1 %recur.valid.3, %next.numerator1.finite
  br i1 %recur.valid, label %recur.ok, label %trap

recur.ok:
  %next.key = add nuw i64 %key, 1
  br label %recur.cond

finish:
  %output0 = fdiv float %numerator0, %running.sum
  %output1 = fdiv float %numerator1, %running.sum
  %output0.bits = bitcast float %output0 to i32
  %output1.bits = bitcast float %output1 to i32
  %output0.exp = and i32 %output0.bits, 2139095040
  %output1.exp = and i32 %output1.bits, 2139095040
  %output0.finite = icmp ne i32 %output0.exp, 2139095040
  %output1.finite = icmp ne i32 %output1.exp, 2139095040
  %outputs.finite = and i1 %output0.finite, %output1.finite
  br i1 %outputs.finite, label %narrow, label %trap

narrow:
  %output0.lsb.shift = lshr i32 %output0.bits, 16
  %output1.lsb.shift = lshr i32 %output1.bits, 16
  %output0.lsb = and i32 %output0.lsb.shift, 1
  %output1.lsb = and i32 %output1.lsb.shift, 1
  %output0.bias = add nuw nsw i32 32767, %output0.lsb
  %output1.bias = add nuw nsw i32 32767, %output1.lsb
  %output0.rounded = add i32 %output0.bits, %output0.bias
  %output1.rounded = add i32 %output1.bits, %output1.bias
  %output0.bf16.wide = lshr i32 %output0.rounded, 16
  %output1.bf16.wide = lshr i32 %output1.rounded, 16
  %output0.bf16 = trunc i32 %output0.bf16.wide to i16
  %output1.bf16 = trunc i32 %output1.bf16.wide to i16
  %first.output = shl nuw i64 %global, 1
  %second.output = add nuw i64 %first.output, 1
  %first.output.ok = icmp ult i64 %first.output, %output.len
  %second.output.ok = icmp ult i64 %second.output, %output.len
  %pair.output.ok = and i1 %first.output.ok, %second.output.ok
  br i1 %pair.output.ok, label %store, label %trap

store:
  %output0.ptr = getelementptr inbounds i16, ptr addrspace(1) %output.data, i64 %first.output
  %output1.ptr = getelementptr inbounds i16, ptr addrspace(1) %output.data, i64 %second.output
  store i16 %output0.bf16, ptr addrspace(1) %output0.ptr, align 2
  store i16 %output1.bf16, ptr addrspace(1) %output1.ptr, align 2
  ret void

trap:
  call void @llvm.trap()
  ret void
}

attributes #0 = { nounwind "amdgpu-flat-work-group-size"="64,64" "target-cpu"="gfx942" "target-features"="-wavefrontsize32,+wavefrontsize64,-xnack" "denormal-fp-math-f32"="ieee,ieee" "unsafe-fp-math"="false" "no-infs-fp-math"="false" "no-nans-fp-math"="false" "no-signed-zeros-fp-math"="false" "approx-func-fp-math"="false" "fp-contract"="off" }
attributes #1 = { nounwind readnone speculatable willreturn }

!0 = !{i32 64, i32 1, i32 1}
!1 = !{!"read_only", !"none", !"read_only", !"none", !"read_only", !"none", !"read_only", !"none", !"read_only", !"none", !"write_only", !"none"}
!2 = !{!"ushort*", !"ulong", !"ushort*", !"ulong", !"ushort*", !"ulong", !"uint*", !"ulong", !"uint*", !"ulong", !"ushort*", !"ulong"}
!3 = !{!"const", !"", !"const", !"", !"const", !"", !"const", !"", !"const", !"", !"restrict", !""}
"#,
    );
    output
}

fn emit_page_mapping(output: &mut String, prefix: &str, key: &str) {
    writeln!(
        output,
        "  %{prefix}.logical.page = lshr i64 {key}, 4\n\
         \x20 %{prefix}.token.in.page = and i64 {key}, 15\n\
         \x20 %{prefix}.logical.page.ok = icmp ult i64 %{prefix}.logical.page, 512\n\
         \x20 %{prefix}.table.sequence = mul nuw i64 %sequence, 512\n\
         \x20 %{prefix}.table.index = add nuw i64 %{prefix}.table.sequence, %{prefix}.logical.page\n\
         \x20 %{prefix}.table.index.ok = icmp ult i64 %{prefix}.table.index, %pages.len\n\
         \x20 %{prefix}.page.lookup.ok = and i1 %{prefix}.logical.page.ok, %{prefix}.table.index.ok\n\
         \x20 br i1 %{prefix}.page.lookup.ok, label %{prefix}.page.load, label %trap\n\n\
         {prefix}.page.load:\n\
         \x20 %{prefix}.page.ptr = getelementptr inbounds i32, ptr addrspace(1) %pages.data, i64 %{prefix}.table.index\n\
         \x20 %{prefix}.physical.page.i32 = load i32, ptr addrspace(1) %{prefix}.page.ptr, align 4\n\
         \x20 %{prefix}.physical.page.ok = icmp ult i32 %{prefix}.physical.page.i32, 16384\n\
         \x20 %{prefix}.physical.page = zext i32 %{prefix}.physical.page.i32 to i64\n\
         \x20 %{prefix}.cache.page.tokens = mul nuw i64 %{prefix}.physical.page, 16\n\
         \x20 %{prefix}.cache.token = add nuw i64 %{prefix}.cache.page.tokens, %{prefix}.token.in.page\n\
         \x20 %{prefix}.cache.token.heads = mul nuw i64 %{prefix}.cache.token, 8\n\
         \x20 %{prefix}.cache.head = add nuw i64 %{prefix}.cache.token.heads, %kv.head\n\
         \x20 %{prefix}.cache.base = mul nuw i64 %{prefix}.cache.head, 128\n\
         \x20 %{prefix}.cache.end = add nuw i64 %{prefix}.cache.base, 128\n\
         \x20 %{prefix}.cache.extent.ok = icmp ule i64 %{prefix}.cache.end, %k.len\n\
         \x20 %{prefix}.page.ok = and i1 %{prefix}.physical.page.ok, %{prefix}.cache.extent.ok"
    )
    .expect("writing to a String cannot fail");
}

fn emit_score(output: &mut String, prefix: &str) {
    writeln!(
        output,
        "{prefix}.score.entry:\n\
         \x20 br label %{prefix}.dot.cond\n\n\
         {prefix}.dot.cond:\n\
         \x20 %{prefix}.feature = phi i64 [ 0, %{prefix}.score.entry ], [ %{prefix}.feature.next, %{prefix}.dot.step ]\n\
         \x20 %{prefix}.dot = phi float [ 0.000000e+00, %{prefix}.score.entry ], [ %{prefix}.dot.next, %{prefix}.dot.step ]\n\
         \x20 %{prefix}.dot.more = icmp ult i64 %{prefix}.feature, 128\n\
         \x20 br i1 %{prefix}.dot.more, label %{prefix}.dot.body, label %{prefix}.dot.done\n\n\
         {prefix}.dot.body:\n\
         \x20 %{prefix}.q.index = add nuw i64 %q.base, %{prefix}.feature\n\
         \x20 %{prefix}.k.index = add nuw i64 %{prefix}.cache.base, %{prefix}.feature\n\
         \x20 %{prefix}.q.ptr = getelementptr inbounds i16, ptr addrspace(1) %q.data, i64 %{prefix}.q.index\n\
         \x20 %{prefix}.k.ptr = getelementptr inbounds i16, ptr addrspace(1) %k.data, i64 %{prefix}.k.index\n\
         \x20 %{prefix}.q.bf16 = load i16, ptr addrspace(1) %{prefix}.q.ptr, align 2\n\
         \x20 %{prefix}.k.bf16 = load i16, ptr addrspace(1) %{prefix}.k.ptr, align 2\n\
         \x20 %{prefix}.q.wide = zext i16 %{prefix}.q.bf16 to i32\n\
         \x20 %{prefix}.k.wide = zext i16 %{prefix}.k.bf16 to i32\n\
         \x20 %{prefix}.q.bits = shl nuw i32 %{prefix}.q.wide, 16\n\
         \x20 %{prefix}.k.bits = shl nuw i32 %{prefix}.k.wide, 16\n\
         \x20 %{prefix}.q = bitcast i32 %{prefix}.q.bits to float\n\
         \x20 %{prefix}.k = bitcast i32 %{prefix}.k.bits to float\n\
         \x20 %{prefix}.q.exp = and i32 %{prefix}.q.bits, 2139095040\n\
         \x20 %{prefix}.k.exp = and i32 %{prefix}.k.bits, 2139095040\n\
         \x20 %{prefix}.q.finite = icmp ne i32 %{prefix}.q.exp, 2139095040\n\
         \x20 %{prefix}.k.finite = icmp ne i32 %{prefix}.k.exp, 2139095040\n\
         \x20 %{prefix}.inputs.finite = and i1 %{prefix}.q.finite, %{prefix}.k.finite\n\
         \x20 %{prefix}.product = fmul float %{prefix}.q, %{prefix}.k\n\
         \x20 %{prefix}.dot.next = fadd float %{prefix}.dot, %{prefix}.product\n\
         \x20 %{prefix}.product.bits = bitcast float %{prefix}.product to i32\n\
         \x20 %{prefix}.dot.next.bits = bitcast float %{prefix}.dot.next to i32\n\
         \x20 %{prefix}.product.exp = and i32 %{prefix}.product.bits, 2139095040\n\
         \x20 %{prefix}.dot.next.exp = and i32 %{prefix}.dot.next.bits, 2139095040\n\
         \x20 %{prefix}.product.finite = icmp ne i32 %{prefix}.product.exp, 2139095040\n\
         \x20 %{prefix}.dot.next.finite = icmp ne i32 %{prefix}.dot.next.exp, 2139095040\n\
         \x20 %{prefix}.arithmetic.finite = and i1 %{prefix}.product.finite, %{prefix}.dot.next.finite\n\
         \x20 %{prefix}.dot.valid = and i1 %{prefix}.inputs.finite, %{prefix}.arithmetic.finite\n\
         \x20 br i1 %{prefix}.dot.valid, label %{prefix}.dot.step, label %trap\n\n\
         {prefix}.dot.step:\n\
         \x20 %{prefix}.feature.next = add nuw i64 %{prefix}.feature, 1\n\
         \x20 br label %{prefix}.dot.cond\n\n\
         {prefix}.dot.done:\n\
         \x20 %{prefix}.scale = bitcast i32 1035273459 to float\n\
         \x20 %{prefix}.score = fmul float %{prefix}.dot, %{prefix}.scale\n\
         \x20 %{prefix}.score.bits = bitcast float %{prefix}.score to i32\n\
         \x20 %{prefix}.score.exp = and i32 %{prefix}.score.bits, 2139095040\n\
         \x20 %{prefix}.score.finite = icmp ne i32 %{prefix}.score.exp, 2139095040"
    )
    .expect("writing to a String cannot fail");
}

fn validate_canonical_llvm(module: &str) -> Result<(), PrepareQwen3PagedDecodeKernelErrorV1> {
    let module_sha256: [u8; 32] = Sha256::digest(module.as_bytes()).into();
    let exact = module.len() == QWEN3_PAGED_DECODE_LLVM_BYTES_V1
        && module_sha256 == QWEN3_PAGED_DECODE_LLVM_SHA256_V1
        && module.matches("define amdgpu_kernel").count() == 1
        && module
            .matches("declare float @__ocml_exp_f32(float)")
            .count()
            == 1
        && module.matches("call float @__ocml_exp_f32(float ").count() == 2
        && module.matches("call void @llvm.trap()").count() == 1
        && module.matches("store i16").count() == 2
        && module.contains("@llvm.amdgcn.workitem.id.x")
        && module.contains("@llvm.amdgcn.workgroup.id.x")
        && module.contains("%workitems = lshr exact i64 %q.len, 1")
        && module.contains("%committed = zext i32 %committed.i32 to i64")
        && module.contains("%query.absolute = add nuw i64 %committed, %query.token")
        && module.contains("%recur.more = icmp ule i64 %key, %query.absolute")
        && module.contains("%initial.logical.page = lshr i64 0, 4")
        && module.contains("%next.logical.page = lshr i64 %key, 4")
        && module
            .contains("%initial.physical.page.ok = icmp ult i32 %initial.physical.page.i32, 16384")
        && module.contains("%next.physical.page.ok = icmp ult i32 %next.physical.page.i32, 16384")
        && module.contains("%initial.cache.page.tokens = mul nuw i64 %initial.physical.page, 16")
        && module.contains("%next.cache.page.tokens = mul nuw i64 %next.physical.page, 16")
        && module.contains("%heads = select i1 %target, i64 32, i64 16")
        && module.contains("%gqa = select i1 %target, i64 4, i64 2")
        && module.contains("bitcast i32 1035273459 to float")
        && module.contains("%pair.output.ok = and i1")
        && module.contains("\"fp-contract\"=\"off\"")
        && !module.contains(" fast ")
        && !module.contains("contract ")
        && !module.contains("reassoc ")
        && !module.contains("cache.sequence")
        && !module.contains("sequence.cache")
        && !module.contains("comgr")
        && !module.contains("COMGR");
    if !exact {
        return Err(PrepareQwen3PagedDecodeKernelErrorV1::CompilerModule);
    }
    Ok(())
}

/// Linear exact compiler handoff awaiting attempt-scoped Worker V2 execution.
pub struct InertQwen3PagedDecodeWorkerRequestV1 {
    prepared: PreparedQwen3PagedDecodeKernelV1,
}

impl fmt::Debug for InertQwen3PagedDecodeWorkerRequestV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InertQwen3PagedDecodeWorkerRequestV1")
            .field("catalog", &self.prepared.catalog.identity)
            .field("source_binding", &self.prepared.source_binding_identity)
            .field("handoff", &self.prepared.compiler_handoff_identity)
            .finish_non_exhaustive()
    }
}

impl InertQwen3PagedDecodeWorkerRequestV1 {
    /// Complete profile catalog retained by this request.
    #[must_use]
    pub const fn catalog(&self) -> &Qwen3PagedDecodeProfileCatalogV1 {
        &self.prepared.catalog
    }

    /// Exact compiler handoff for attempt-scoped transaction publication.
    #[must_use]
    pub const fn compiler_handoff(&self) -> &CompilerModuleHandoffV2 {
        &self.prepared.compiler_handoff
    }

    /// Ferric-domain source binding retained by the compiler handoff.
    #[must_use]
    pub const fn source_binding_identity(&self) -> &[u8; 32] {
        &self.prepared.source_binding_identity
    }

    /// A request value does not establish Worker execution or artifact existence.
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
pub const fn lower_qwen3_paged_decode_kernel_v1(
    prepared: PreparedQwen3PagedDecodeKernelV1,
) -> InertQwen3PagedDecodeWorkerRequestV1 {
    InertQwen3PagedDecodeWorkerRequestV1 { prepared }
}

/// Failure while executing the exact module through Worker V2.
#[derive(Debug)]
pub enum ExecuteQwen3PagedDecodeWorkerErrorV1 {
    /// Consumed attempt bytes differ from the exact prepared handoff.
    HandoffSubstitution,
    /// A fixed link option could not be represented.
    FixedLinkOption,
    /// The fixed HSACO output ceiling could not be represented.
    OutputConstraint(WorkerProtocolError),
    /// Reproducible bootstrap and exact replay failed.
    FirstBuild(FirstBuildWorkerV2Error),
}

impl fmt::Display for ExecuteQwen3PagedDecodeWorkerErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Qwen3 paged decode Worker V2 execution failed: {self:?}"
        )
    }
}

impl std::error::Error for ExecuteQwen3PagedDecodeWorkerErrorV1 {}

/// Linear Worker V2 bootstrap/replay evidence awaiting structural inspection.
pub struct InertQwen3PagedDecodeWorkerEvidenceV1 {
    prepared: PreparedQwen3PagedDecodeKernelV1,
    transaction_handoff: CompilerModuleHandoffIdentityV1,
    worker: InertFirstBuildWorkerV2EvidenceV1,
}

impl fmt::Debug for InertQwen3PagedDecodeWorkerEvidenceV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InertQwen3PagedDecodeWorkerEvidenceV1")
            .field("source_binding", &self.prepared.source_binding_identity)
            .field("transaction_handoff", &self.transaction_handoff)
            .field("worker", &self.worker.identity())
            .finish_non_exhaustive()
    }
}

impl InertQwen3PagedDecodeWorkerEvidenceV1 {
    /// Reproducible execution remains inert until exact structural inspection.
    #[must_use]
    pub const fn grants_artifact_authority(&self) -> bool {
        false
    }

    /// Worker output does not prove the online numerical contract.
    #[must_use]
    pub const fn proves_numerical_contract(&self) -> bool {
        false
    }

    /// Worker output does not reconcile online and two-pass attention.
    #[must_use]
    pub const fn proves_operator_refinement(&self) -> bool {
        false
    }

    /// Worker output establishes no paged-memory or race refinement.
    #[must_use]
    pub const fn proves_memory_or_race_refinement(&self) -> bool {
        false
    }
}

/// Executes exact attempt bytes through Worker V2 bootstrap and replay.
///
/// # Errors
///
/// Returns an error for a substituted handoff, invalid fixed link options or
/// output constraints, or a Worker V2 execution failure.
pub fn execute_qwen3_paged_decode_worker_v2_v1(
    request: InertQwen3PagedDecodeWorkerRequestV1,
    consumed: ConsumedCompilerModuleHandoffV1,
    worker: &PinnedWorkerV1,
    limits: WorkerExecutionLimitsV1,
) -> Result<InertQwen3PagedDecodeWorkerEvidenceV1, ExecuteQwen3PagedDecodeWorkerErrorV1> {
    let InertQwen3PagedDecodeWorkerRequestV1 { prepared } = request;
    if consumed.bytes() != prepared.compiler_handoff.canonical_bytes() {
        return Err(ExecuteQwen3PagedDecodeWorkerErrorV1::HandoffSubstitution);
    }
    let transaction_handoff = consumed.identity();
    let worker_evidence = execute_reproducible_first_build_worker_v2(
        consumed,
        worker,
        Vec::new(),
        fixed_link_options()?,
        WorkerOutputConstraintsV1::new(MAX_HSACO_BYTES as u64)
            .map_err(ExecuteQwen3PagedDecodeWorkerErrorV1::OutputConstraint)?,
        limits,
    )
    .map_err(ExecuteQwen3PagedDecodeWorkerErrorV1::FirstBuild)?;
    Ok(InertQwen3PagedDecodeWorkerEvidenceV1 {
        prepared,
        transaction_handoff,
        worker: worker_evidence,
    })
}

/// Exact post-worker structural rejection.
#[derive(Debug)]
pub enum InspectQwen3PagedDecodeKernelErrorV1 {
    /// Worker request or response canonical bytes failed decoding.
    Protocol(WorkerProtocolError),
    /// Compiler, transaction, manifest, Worker, provider, or output lineage drifted.
    SourceLineage,
    /// AMDHSA metadata or descriptor binding failed.
    Hsaco(KernelBindingError),
    /// Kernel inventory, ABI, or resources differ from the exact profile.
    KernelProfile,
    /// Strict allocation-free COV6 loader validation failed.
    Loader(PlanError),
}

impl fmt::Display for InspectQwen3PagedDecodeKernelErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Qwen3 paged decode structural inspection failed: {self:?}"
        )
    }
}

impl std::error::Error for InspectQwen3PagedDecodeKernelErrorV1 {}

/// Linear Worker output after strict transcript, provider, ABI, resource, and
/// loader inspection.
pub struct InspectedQwen3PagedDecodeKernelV1 {
    catalog: Qwen3PagedDecodeProfileCatalogV1,
    source_binding_identity: [u8; 32],
    compiler_handoff_identity: CompilerModuleHandoffIdentityV2,
    transaction_handoff: CompilerModuleHandoffIdentityV1,
    loader_plan: LoadPlan,
    worker: InertFirstBuildWorkerV2EvidenceV1,
}

impl fmt::Debug for InspectedQwen3PagedDecodeKernelV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InspectedQwen3PagedDecodeKernelV1")
            .field("catalog", &self.catalog.identity)
            .field("source_binding", &self.source_binding_identity)
            .field("compiler_handoff", &self.compiler_handoff_identity)
            .field("transaction_handoff", &self.transaction_handoff)
            .field("worker", &self.worker.identity())
            .finish_non_exhaustive()
    }
}

impl InspectedQwen3PagedDecodeKernelV1 {
    /// Exact profile catalog retained with the inspected output owner.
    #[must_use]
    pub const fn catalog(&self) -> &Qwen3PagedDecodeProfileCatalogV1 {
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

    /// Structural inspection does not prove paged-memory or race refinement.
    #[must_use]
    pub const fn proves_memory_or_race_refinement(&self) -> bool {
        false
    }

    /// Provider evidence is measured structure, not independent content authentication.
    #[must_use]
    pub const fn authenticates_ocml_provider_content(&self) -> bool {
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

    /// Binds one exact profile to checked numerical spans and inert host labels.
    ///
    /// # Errors
    ///
    /// Returns an error if the profile is absent, any buffer span fails its
    /// exact contract, or the inert metadata labels are invalid.
    pub fn bind_checked_profile(
        &self,
        role: Qwen3PagedDecodeModelRoleV1,
        bucket: Qwen3PagedDecodeBucketV1,
        addresses: [u64; 6],
        byte_lengths: [u64; 6],
        metadata: Qwen3PagedDecodeHostMetadataV1,
    ) -> Result<CheckedQwen3PagedDecodeLaunchV1, BindQwen3PagedDecodeLaunchErrorV1> {
        let profile = self
            .catalog
            .profile(role, bucket)
            .ok_or(BindQwen3PagedDecodeLaunchErrorV1::Profile)?;
        let buffers = Qwen3PagedDecodeBufferContractV1::checked(profile, addresses, byte_lengths)
            .map_err(BindQwen3PagedDecodeLaunchErrorV1::Buffers)?;
        metadata
            .validate()
            .map_err(BindQwen3PagedDecodeLaunchErrorV1::Metadata)?;
        Ok(CheckedQwen3PagedDecodeLaunchV1 {
            profile,
            buffers,
            metadata,
        })
    }
}

/// Consumes Worker evidence through exact transcript, provider, HSACO, ABI,
/// resource, and loader checks.
///
/// # Errors
///
/// Returns an error if lineage, provider binding, output identity, HSACO
/// structure, kernel ABI, resource limits, or the loader profile fails closed.
pub fn inspect_qwen3_paged_decode_kernel_v1(
    evidence: InertQwen3PagedDecodeWorkerEvidenceV1,
) -> Result<InspectedQwen3PagedDecodeKernelV1, InspectQwen3PagedDecodeKernelErrorV1> {
    let InertQwen3PagedDecodeWorkerEvidenceV1 {
        prepared,
        transaction_handoff,
        worker,
    } = evidence;
    validate_worker_lineage(&prepared, transaction_handoff, &worker)?;
    let bytes = worker.output_bytes();
    if !worker.output_identity().matches(bytes) {
        return Err(InspectQwen3PagedDecodeKernelErrorV1::SourceLineage);
    }
    let bound = inspect_and_bind_kernel_descriptors(bytes)
        .map_err(InspectQwen3PagedDecodeKernelErrorV1::Hsaco)?;
    let [kernel] = bound.inspection().kernels() else {
        return Err(InspectQwen3PagedDecodeKernelErrorV1::KernelProfile);
    };
    let [binding] = bound.bindings() else {
        return Err(InspectQwen3PagedDecodeKernelErrorV1::KernelProfile);
    };
    let exact = bound.inspection().code_object_version() == InspectedCodeObjectVersion::V6
        && bound.inspection().target().to_string() == QWEN3_PAGED_DECODE_TARGET_V1
        && !bound.inspection().has_printf_metadata()
        && kernel.name() == QWEN3_PAGED_DECODE_KERNEL_SYMBOL_V1
        && kernel.symbol() == QWEN3_PAGED_DECODE_KERNEL_DESCRIPTOR_SYMBOL_V1
        && kernel.kernarg_segment_size() == QWEN3_PAGED_DECODE_TOTAL_KERNARG_BYTES_V1
        && kernel.kernarg_segment_alignment() == QWEN3_PAGED_DECODE_KERNARG_ALIGNMENT_V1
        && kernel.implicit_argument_offset() == Some(QWEN3_PAGED_DECODE_EXPLICIT_KERNARG_BYTES_V1)
        && kernel.implicit_argument_size() == 256
        && kernel.required_workgroup_size() == Some(QWEN3_PAGED_DECODE_WORKGROUP_V1)
        && kernel.max_flat_workgroup_size() == 64
        && kernel.wavefront_size() == 64
        && kernel.group_segment_fixed_size() == 0
        && kernel.private_segment_fixed_size() == 0
        && kernel.sgpr_spill_count().unwrap_or(0) == 0
        && kernel.vgpr_spill_count().unwrap_or(0) == 0
        && !kernel.uses_dynamic_stack()
        && binding.kernel_index() == 0
        && binding.descriptor().group_segment_fixed_size() == 0
        && binding.descriptor().private_segment_fixed_size() == 0
        && binding.descriptor().wavefront_size() == 64
        && !binding.descriptor().uses_dynamic_stack()
        && exact_paged_decode_explicit_arguments(kernel.explicit_arguments())
        && exact_hidden_arguments(
            kernel.hidden_arguments(),
            QWEN3_PAGED_DECODE_EXPLICIT_KERNARG_BYTES_V1,
        );
    if !exact {
        return Err(InspectQwen3PagedDecodeKernelErrorV1::KernelProfile);
    }
    let loader = fe2o3_amdhsa_loader::validate(bytes, AdmittedProfile::Gfx942XnackOffCov6)
        .map_err(InspectQwen3PagedDecodeKernelErrorV1::Loader)?;
    let loader_plan = *loader.plan();
    Ok(InspectedQwen3PagedDecodeKernelV1 {
        catalog: prepared.catalog,
        source_binding_identity: prepared.source_binding_identity,
        compiler_handoff_identity: prepared.compiler_handoff_identity,
        transaction_handoff,
        loader_plan,
        worker,
    })
}

fn validate_worker_lineage(
    prepared: &PreparedQwen3PagedDecodeKernelV1,
    transaction_handoff: CompilerModuleHandoffIdentityV1,
    worker: &InertFirstBuildWorkerV2EvidenceV1,
) -> Result<(), InspectQwen3PagedDecodeKernelErrorV1> {
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
        return Err(InspectQwen3PagedDecodeKernelErrorV1::SourceLineage);
    }
    let bootstrap = InertDecodedWorkerExchangeV2::decode(
        worker.bootstrap_request_bytes(),
        worker.bootstrap().response().canonical_bytes(),
    )
    .map_err(InspectQwen3PagedDecodeKernelErrorV1::Protocol)?;
    let replay = InertDecodedWorkerExchangeV2::decode(
        worker.authorized_request_bytes(),
        worker.authorized().response().canonical_bytes(),
    )
    .map_err(InspectQwen3PagedDecodeKernelErrorV1::Protocol)?;
    for exchange in [&bootstrap, &replay] {
        let request = exchange.request();
        if request.target() != exact_target()
            || request.code_object_version() != CodeObjectVersion::V6
            || request.compiler_module().bytes() != prepared.compiler_handoff.module_bytes()
            || !request.external_providers().is_empty()
            || request.import_symbols() != [OCML_EXP_F32]
            || !request.export_symbols().is_empty()
            || request.final_symbols()
                != [
                    OCML_EXP_F32,
                    QWEN3_PAGED_DECODE_KERNEL_SYMBOL_V1,
                    QWEN3_PAGED_DECODE_KERNEL_DESCRIPTOR_SYMBOL_V1,
                ]
            || exchange.response().request_identity() != request.identity()
            || !exact_ocml_provider(exchange.response())
        {
            return Err(InspectQwen3PagedDecodeKernelErrorV1::SourceLineage);
        }
    }
    Ok(())
}

fn exact_ocml_provider(response: &fe2o3_hsaco_finalize::WorkerResponseV2) -> bool {
    let Some(provider) = response.device_library_provider() else {
        return false;
    };
    provider.provider_identity() == OCML_PROVIDER_IDENTITY
        && provider.target().to_string() == QWEN3_PAGED_DECODE_TARGET_V1
        && provider.code_object_version() == CodeObjectVersion::V6
        && provider.import_symbols() == [OCML_EXP_F32]
        && provider.manifest_identity() != &[0; 32]
        && provider.files().len() == OCML_PROVIDER_BASENAMES.len()
        && provider
            .files()
            .iter()
            .zip(OCML_PROVIDER_BASENAMES)
            .all(|(file, basename)| file.basename() == basename && file.sha256() != &[0; 32])
}

fn exact_paged_decode_explicit_arguments(arguments: &[ExplicitArgument]) -> bool {
    if arguments.len() != 12 {
        return false;
    }
    for (index, name, access, alignment, accepted_type) in [
        (
            0,
            "q.data",
            ArgumentAccess::ReadOnly,
            2,
            is_bf16_metadata_carrier as fn(ExplicitValueType) -> bool,
        ),
        (
            2,
            "k.data",
            ArgumentAccess::ReadOnly,
            2,
            is_bf16_metadata_carrier,
        ),
        (
            4,
            "v.data",
            ArgumentAccess::ReadOnly,
            2,
            is_bf16_metadata_carrier,
        ),
        (
            6,
            "pages.data",
            ArgumentAccess::ReadOnly,
            4,
            is_i32_metadata_carrier,
        ),
        (
            8,
            "committed.data",
            ArgumentAccess::ReadOnly,
            4,
            is_i32_metadata_carrier,
        ),
        (
            10,
            "output.data",
            ArgumentAccess::WriteOnly,
            2,
            is_bf16_metadata_carrier,
        ),
    ] {
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
        (1, "q.len"),
        (3, "k.len"),
        (5, "v.len"),
        (7, "pages.len"),
        (9, "committed.len"),
        (11, "output.len"),
    ] {
        if !exact_length_argument(&arguments[index], name, ((index - 1) as u64 / 2) * 16 + 8) {
            return false;
        }
    }
    true
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

const fn is_bf16_metadata_carrier(value_type: ExplicitValueType) -> bool {
    matches!(
        value_type,
        ExplicitValueType::I16 | ExplicitValueType::U16 | ExplicitValueType::F16
    )
}

const fn is_i32_metadata_carrier(value_type: ExplicitValueType) -> bool {
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

fn fixed_link_options() -> Result<Vec<LinkOptionV1>, ExecuteQwen3PagedDecodeWorkerErrorV1> {
    [
        ("code-object-version", "6"),
        ("opt-level", "2"),
        ("strip-debug", "true"),
        ("verify-each", "true"),
    ]
    .into_iter()
    .map(|(name, value)| {
        LinkOptionV1::new(name, value)
            .map_err(|_| ExecuteQwen3PagedDecodeWorkerErrorV1::FixedLinkOption)
    })
    .collect()
}

/// Failure while binding an inspected output to a finite runtime profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindQwen3PagedDecodeLaunchErrorV1 {
    /// The requested role/bucket tuple is absent from the finite catalog.
    Profile,
    /// Numerical buffer validation failed.
    Buffers(Qwen3PagedDecodeBufferContractErrorV1),
    /// Required host-only identity/generation labels failed closed.
    Metadata(Qwen3PagedDecodeHostMetadataErrorV1),
}

impl fmt::Display for BindQwen3PagedDecodeLaunchErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Qwen3 paged decode launch binding failed: {self:?}"
        )
    }
}

impl std::error::Error for BindQwen3PagedDecodeLaunchErrorV1 {}

/// Untrusted host-side labels for the page/cache/context snapshot.
#[derive(Debug, Eq, PartialEq)]
pub struct Qwen3PagedDecodeHostMetadataV1 {
    page_table_identity: [u8; 32],
    key_cache_identity: [u8; 32],
    value_cache_identity: [u8; 32],
    committed_tokens_identity: [u8; 32],
    cache_owner_identity: [u8; 32],
    page_generation: u64,
}

impl Qwen3PagedDecodeHostMetadataV1 {
    /// Constructs inert labels for the exact page-table/cache snapshot.
    #[must_use]
    pub const fn new(
        page_table_identity: [u8; 32],
        key_cache_identity: [u8; 32],
        value_cache_identity: [u8; 32],
        committed_tokens_identity: [u8; 32],
        cache_owner_identity: [u8; 32],
        page_generation: u64,
    ) -> Self {
        Self {
            page_table_identity,
            key_cache_identity,
            value_cache_identity,
            committed_tokens_identity,
            cache_owner_identity,
            page_generation,
        }
    }

    fn validate(&self) -> Result<(), Qwen3PagedDecodeHostMetadataErrorV1> {
        let identities = [
            self.page_table_identity,
            self.key_cache_identity,
            self.value_cache_identity,
            self.committed_tokens_identity,
            self.cache_owner_identity,
        ];
        for (index, identity) in identities.iter().enumerate() {
            if identity == &[0; 32] || identities[index + 1..].contains(identity) {
                return Err(Qwen3PagedDecodeHostMetadataErrorV1::IdentityOrGeneration);
            }
        }
        if self.page_generation == 0 {
            return Err(Qwen3PagedDecodeHostMetadataErrorV1::IdentityOrGeneration);
        }
        Ok(())
    }

    /// Exact page generation label.
    #[must_use]
    pub const fn page_generation(&self) -> u64 {
        self.page_generation
    }

    /// These labels do not authenticate content, ownership, or generation.
    #[must_use]
    pub const fn authenticates_content_or_ownership(&self) -> bool {
        false
    }
}

/// Host metadata admission failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Qwen3PagedDecodeHostMetadataErrorV1 {
    /// A required identity/generation was absent or identities were aliased.
    IdentityOrGeneration,
}

/// Inert exact profile and numerical-buffer binding for a future protected launcher.
#[derive(Debug)]
pub struct CheckedQwen3PagedDecodeLaunchV1 {
    profile: Qwen3PagedDecodeProfileV1,
    buffers: Qwen3PagedDecodeBufferContractV1,
    metadata: Qwen3PagedDecodeHostMetadataV1,
}

impl CheckedQwen3PagedDecodeLaunchV1 {
    /// Exact finite profile.
    #[must_use]
    pub const fn profile(&self) -> Qwen3PagedDecodeProfileV1 {
        self.profile
    }

    /// Exact checked numerical buffer ranges.
    #[must_use]
    pub const fn buffers(&self) -> &Qwen3PagedDecodeBufferContractV1 {
        &self.buffers
    }

    /// Host-only labels retained outside the machine ABI.
    #[must_use]
    pub const fn metadata(&self) -> &Qwen3PagedDecodeHostMetadataV1 {
        &self.metadata
    }

    /// This binding grants no allocation, load, or launch authority.
    #[must_use]
    pub const fn grants_launch_authority(&self) -> bool {
        false
    }
}

fn exact_target() -> DeviceTargetV1 {
    DeviceTargetV1::parse(QWEN3_PAGED_DECODE_TARGET_V1)
        .expect("the fixed Qwen3 paged decode target is canonical")
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

    fn bindings(seed: u8) -> Qwen3PagedDecodeSourceBindingsV1 {
        Qwen3PagedDecodeSourceBindingsV1::new(
            [seed; 32],
            [seed.wrapping_add(1); 32],
            [seed.wrapping_add(2); 32],
            [seed.wrapping_add(3); 32],
        )
    }

    fn profile(
        role: Qwen3PagedDecodeModelRoleV1,
        bucket: Qwen3PagedDecodeBucketV1,
    ) -> Qwen3PagedDecodeProfileV1 {
        Qwen3PagedDecodeProfileCatalogV1::canonical()
            .unwrap()
            .profile(role, bucket)
            .unwrap()
    }

    fn layout(profile: Qwen3PagedDecodeProfileV1) -> ([u64; 6], [u64; 6]) {
        let query = profile.query_elements() * 2;
        let cache = profile.cache_elements_each() * 2;
        let pages = profile.page_table_elements() * 4;
        let committed = profile.context_elements() * 4;
        (
            [
                0x1_0000_0000,
                0x2_0000_0000,
                0x3_0000_0000,
                0x4_0000_0000,
                0x5_0000_0000,
                0x6_0000_0000,
            ],
            [query, cache, cache, pages, committed, query],
        )
    }

    #[test]
    fn exact_fourteen_profile_catalog_is_complete_unique_and_checked() {
        let catalog = Qwen3PagedDecodeProfileCatalogV1::canonical().unwrap();
        assert_eq!(
            catalog.profiles().len(),
            QWEN3_PAGED_DECODE_PROFILE_COUNT_V1
        );
        assert_eq!(
            catalog
                .profiles()
                .iter()
                .copied()
                .map(Qwen3PagedDecodeProfileV1::identity)
                .collect::<BTreeSet<_>>()
                .len(),
            QWEN3_PAGED_DECODE_PROFILE_COUNT_V1
        );
        for profile in catalog.profiles() {
            let vectors = profile
                .sequences()
                .checked_mul(profile.active_tokens())
                .and_then(|value| value.checked_mul(profile.query_heads()))
                .unwrap();
            assert_eq!(profile.grid_workgroups(), [vectors, 1, 1]);
            assert_eq!(
                profile.launch_workitems(),
                [vectors.checked_mul(64).unwrap(), 1, 1]
            );
            assert_eq!(
                profile.query_heads() / profile.gqa_group_size(),
                QWEN3_PAGED_DECODE_KV_HEADS_V1
            );
        }
        assert!(!catalog.grants_authority());
    }

    #[test]
    fn attention_width_is_qh_times_d_and_rejects_draft_hidden_substitution() {
        let catalog = Qwen3PagedDecodeProfileCatalogV1::canonical().unwrap();
        let target = catalog
            .profile(
                Qwen3PagedDecodeModelRoleV1::Target8B,
                Qwen3PagedDecodeBucketV1::DecodeS1C8192,
            )
            .unwrap();
        let draft = catalog
            .profile(
                Qwen3PagedDecodeModelRoleV1::Draft06B,
                Qwen3PagedDecodeBucketV1::DecodeS1C8192,
            )
            .unwrap();
        assert_eq!(target.query_width(), 4_096);
        assert_eq!(draft.query_width(), 2_048);
        let mut hostile_draft_hidden_width = draft;
        hostile_draft_hidden_width.query_width = 1_024;
        hostile_draft_hidden_width.query_elements /= 2;
        assert!(!catalog.profiles().contains(&hostile_draft_hidden_width));
        assert!(canonical_qwen3_paged_decode_llvm()
            .contains("%q.d.decode.s1 = icmp eq i64 %q.len, 2048"));
    }

    #[test]
    fn global_paged_cache_geometry_is_fixed_and_profiles_only_scale_tables() {
        let catalog = Qwen3PagedDecodeProfileCatalogV1::canonical().unwrap();
        for profile in catalog.profiles() {
            assert_eq!(
                profile.page_table_elements(),
                u64::from(profile.sequences()) * 512
            );
            assert_eq!(profile.cache_elements_each(), 16_384 * 16 * 8 * 128);
            assert_eq!(profile.context_elements(), u64::from(profile.sequences()));
            assert_eq!(profile.context_capacity(), 8_192);
            let ir = qwen3_paged_decode_kernel_ir_v1(*profile);
            assert_eq!(
                ir.recurrence()[5],
                Qwen3PagedDecodeRecurrenceStepV1::P16LogicalToPhysicalPageMapping
            );
            assert_eq!(
                ir.exceptional_policy(),
                Qwen3PagedDecodeExceptionalPolicyV1::PerLaneTrapBeforeOwnedPairNoGlobalAtomicity
            );
        }
        let s1 = catalog
            .profile(
                Qwen3PagedDecodeModelRoleV1::Target8B,
                Qwen3PagedDecodeBucketV1::DecodeS1C8192,
            )
            .unwrap();
        let s8 = catalog
            .profile(
                Qwen3PagedDecodeModelRoleV1::Target8B,
                Qwen3PagedDecodeBucketV1::DecodeS8C8192,
            )
            .unwrap();
        let s32 = catalog
            .profile(
                Qwen3PagedDecodeModelRoleV1::Target8B,
                Qwen3PagedDecodeBucketV1::DecodeS32C8192,
            )
            .unwrap();
        assert_eq!(s1.cache_elements_each(), s8.cache_elements_each());
        assert_eq!(s8.cache_elements_each(), s32.cache_elements_each());
        assert_eq!(
            [
                s1.page_table_elements(),
                s8.page_table_elements(),
                s32.page_table_elements(),
            ],
            [512, 4_096, 16_384]
        );
    }

    #[test]
    fn role_bucket_active_widths_are_exact() {
        let catalog = Qwen3PagedDecodeProfileCatalogV1::canonical().unwrap();
        for (bucket, target, draft, sequences) in [
            (Qwen3PagedDecodeBucketV1::DecodeS1C8192, 1, 1, 1),
            (Qwen3PagedDecodeBucketV1::DecodeS8C8192, 1, 1, 8),
            (Qwen3PagedDecodeBucketV1::DecodeS32C8192, 1, 1, 32),
            (Qwen3PagedDecodeBucketV1::SpecS1K4C8192, 5, 4, 1),
            (Qwen3PagedDecodeBucketV1::SpecS8K4C8192, 5, 4, 8),
            (Qwen3PagedDecodeBucketV1::SpecS1K8C8192, 9, 8, 1),
            (Qwen3PagedDecodeBucketV1::SpecS1K16C8192, 17, 16, 1),
        ] {
            let target_profile = catalog
                .profile(Qwen3PagedDecodeModelRoleV1::Target8B, bucket)
                .unwrap();
            let draft_profile = catalog
                .profile(Qwen3PagedDecodeModelRoleV1::Draft06B, bucket)
                .unwrap();
            assert_eq!(target_profile.sequences(), sequences);
            assert_eq!(draft_profile.sequences(), sequences);
            assert_eq!(target_profile.active_tokens(), target);
            assert_eq!(draft_profile.active_tokens(), draft);
        }
    }

    #[test]
    fn committed_plus_active_is_the_exact_causal_interval_without_content_authority() {
        let exact = canonical_qwen3_paged_decode_llvm();
        assert!(exact.contains("%resident = add nuw i64 %committed, %active"));
        assert!(exact.contains("%query.absolute = add nuw i64 %committed, %query.token"));
        assert!(exact.contains("%resident.ok = icmp ule i64 %resident, 8192"));
        assert!(exact.contains("%recur.more = icmp ule i64 %key, %query.absolute"));
        let target_k4 = profile(
            Qwen3PagedDecodeModelRoleV1::Target8B,
            Qwen3PagedDecodeBucketV1::SpecS1K4C8192,
        );
        let (addresses, lengths) = layout(target_k4);
        let buffers =
            Qwen3PagedDecodeBufferContractV1::checked(target_k4, addresses, lengths).unwrap();
        assert!(!buffers.authenticates_device_memory());
    }

    #[test]
    fn exact_llvm_classifier_rejects_contract_and_mapping_substitution() {
        let exact = canonical_qwen3_paged_decode_llvm();
        assert_eq!(exact.len(), QWEN3_PAGED_DECODE_LLVM_BYTES_V1);
        assert_eq!(
            <[u8; 32]>::from(Sha256::digest(exact.as_bytes())),
            QWEN3_PAGED_DECODE_LLVM_SHA256_V1
        );
        validate_canonical_llvm(&exact).unwrap();
        let contract_substitution = exact.replacen(
            "declare float @__ocml_exp_f32(float)",
            "declare float @hostile_exp_f32(float)",
            1,
        );
        assert!(validate_canonical_llvm(&contract_substitution).is_err());
        let helper_substitution = exact.replacen(
            "%next.logical.page = lshr i64 %key, 4",
            "%next.logical.page = lshr i64 %key, 5",
            1,
        );
        assert!(validate_canonical_llvm(&helper_substitution).is_err());
        let initial_pool_substitution = exact.replacen(
            "%initial.physical.page.ok = icmp ult i32 %initial.physical.page.i32, 16384",
            "%initial.physical.page.ok = icmp ult i32 %initial.physical.page.i32, 512",
            1,
        );
        assert!(validate_canonical_llvm(&initial_pool_substitution).is_err());
        let recurrent_pool_substitution = exact.replacen(
            "%next.physical.page.ok = icmp ult i32 %next.physical.page.i32, 16384",
            "%next.physical.page.ok = icmp ult i32 %next.physical.page.i32, 512",
            1,
        );
        assert!(validate_canonical_llvm(&recurrent_pool_substitution).is_err());
        assert!(!exact.contains("cache.sequence"));
        assert!(!exact.contains("sequence.cache.base"));
        let store_substitution =
            exact.replacen("store i16 %output1.bf16", "store i16 %output0.bf16", 1);
        assert_ne!(exact, store_substitution);
        assert!(validate_canonical_llvm(&store_substitution).is_err());
    }

    #[test]
    fn source_bindings_fail_closed_and_preparation_retains_nonclaims() {
        assert!(prepare_qwen3_paged_decode_kernel_v1(bindings(1)).is_ok());
        assert!(
            prepare_qwen3_paged_decode_kernel_v1(Qwen3PagedDecodeSourceBindingsV1::new(
                [0; 32], [2; 32], [3; 32], [4; 32]
            ))
            .is_err()
        );
        assert!(
            prepare_qwen3_paged_decode_kernel_v1(Qwen3PagedDecodeSourceBindingsV1::new(
                [1; 32], [1; 32], [3; 32], [4; 32]
            ))
            .is_err()
        );
        let prepared = prepare_qwen3_paged_decode_kernel_v1(bindings(7)).unwrap();
        assert!(!prepared.uses_typed_handoff_v2_source());
        assert!(!prepared.authenticates_compiler_origin());
        assert!(!prepared.proves_operator_or_numerical_refinement());
        assert!(!prepared.has_ferric_plan_identity_join());
        assert!(!prepared.has_kernel_schedule_catalog_join());
        assert!(!prepared.grants_launch_authority());
        assert_eq!(
            prepared
                .compiler_handoff()
                .envelope()
                .directional_symbols()
                .imports()
                .collect::<Vec<_>>(),
            [OCML_EXP_F32]
        );
    }

    #[test]
    fn buffer_contract_rejects_lengths_alignment_overflow_and_aliases() {
        let profile = profile(
            Qwen3PagedDecodeModelRoleV1::Target8B,
            Qwen3PagedDecodeBucketV1::DecodeS1C8192,
        );
        let (addresses, lengths) = layout(profile);
        let checked =
            Qwen3PagedDecodeBufferContractV1::checked(profile, addresses, lengths).unwrap();
        assert!(!checked.authenticates_device_memory());
        let mut short = lengths;
        short[2] -= 2;
        assert_eq!(
            Qwen3PagedDecodeBufferContractV1::checked(profile, addresses, short),
            Err(Qwen3PagedDecodeBufferContractErrorV1::ByteLength(
                Qwen3PagedDecodeBufferV1::ValueCache
            ))
        );
        let mut misaligned = addresses;
        misaligned[3] += 2;
        assert_eq!(
            Qwen3PagedDecodeBufferContractV1::checked(profile, misaligned, lengths),
            Err(Qwen3PagedDecodeBufferContractErrorV1::Alignment(
                Qwen3PagedDecodeBufferV1::PageIndices
            ))
        );
        let mut short_context = lengths;
        short_context[4] -= 4;
        assert_eq!(
            Qwen3PagedDecodeBufferContractV1::checked(profile, addresses, short_context),
            Err(Qwen3PagedDecodeBufferContractErrorV1::ByteLength(
                Qwen3PagedDecodeBufferV1::CommittedTokens
            ))
        );
        let mut misaligned_context = addresses;
        misaligned_context[4] += 2;
        assert_eq!(
            Qwen3PagedDecodeBufferContractV1::checked(profile, misaligned_context, lengths),
            Err(Qwen3PagedDecodeBufferContractErrorV1::Alignment(
                Qwen3PagedDecodeBufferV1::CommittedTokens
            ))
        );
        let mut aliased = addresses;
        aliased[5] = aliased[0];
        assert_eq!(
            Qwen3PagedDecodeBufferContractV1::checked(profile, aliased, lengths),
            Err(Qwen3PagedDecodeBufferContractErrorV1::Aliasing)
        );
        let mut overflowing = addresses;
        overflowing[0] = u64::MAX - lengths[0] + 1;
        assert_eq!(
            Qwen3PagedDecodeBufferContractV1::checked(profile, overflowing, lengths),
            Err(Qwen3PagedDecodeBufferContractErrorV1::RangeOverflow(
                Qwen3PagedDecodeBufferV1::Query
            ))
        );
    }

    #[test]
    fn host_metadata_is_inert_and_rejects_alias_or_zero_generation() {
        let valid =
            Qwen3PagedDecodeHostMetadataV1::new([1; 32], [2; 32], [3; 32], [4; 32], [5; 32], 9);
        assert_eq!(valid.validate(), Ok(()));
        assert!(!valid.authenticates_content_or_ownership());
        assert_eq!(valid.page_generation(), 9);
        let aliased =
            Qwen3PagedDecodeHostMetadataV1::new([1; 32], [2; 32], [2; 32], [4; 32], [5; 32], 9);
        assert_eq!(
            aliased.validate(),
            Err(Qwen3PagedDecodeHostMetadataErrorV1::IdentityOrGeneration)
        );
        let zero_generation =
            Qwen3PagedDecodeHostMetadataV1::new([1; 32], [2; 32], [3; 32], [4; 32], [5; 32], 0);
        assert_eq!(
            zero_generation.validate(),
            Err(Qwen3PagedDecodeHostMetadataErrorV1::IdentityOrGeneration)
        );
    }
}
