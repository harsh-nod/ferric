//! Exact finite Qwen3 SwiGLU compiler profiles.
//!
//! Gate, up, and activated output are BF16 `[rows, intermediate]`, with target
//! intermediate width 12,288 and draft width 3,072. Rows are the exact
//! sequence count times the role-specific active-token width of one of the
//! eleven Ferric graph buckets.
//!
//! The machine declaration widens gate/up BF16 values to FP32 and evaluates a
//! deterministic sign-selected sigmoid using the unresolved
//! `__ocml_exp_f32` provider boundary, then gate*sigmoid*up in fixed order and
//! BF16 RNE output. It assigns eight contiguous elements to each workitem in
//! fixed 256-workitem groups and masks only the final tail.
//!
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

/// Exact kernel entry shared by all twenty-two runtime profiles.
pub const QWEN3_SWIGLU_KERNEL_SYMBOL_V1: &str = "qwen3_swiglu_bf16_f32_v1";
/// Exact AMDHSA descriptor symbol.
pub const QWEN3_SWIGLU_KERNEL_DESCRIPTOR_SYMBOL_V1: &str = "qwen3_swiglu_bf16_f32_v1.kd";
/// Exact gfx942 feature profile.
pub const QWEN3_SWIGLU_TARGET_V1: &str = "gfx942:xnack-";
/// Exact code-object version.
pub const QWEN3_SWIGLU_CODE_OBJECT_VERSION_V1: u8 = 6;
/// Exact workgroup measured in workitems.
pub const QWEN3_SWIGLU_WORKGROUP_V1: [u32; 3] = [256, 1, 1];
/// Exact contiguous elements owned by one workitem.
pub const QWEN3_SWIGLU_ELEMENTS_PER_WORKITEM_V1: u32 = 8;
/// Exact elements covered by one workgroup before tail masking.
pub const QWEN3_SWIGLU_ELEMENTS_PER_WORKGROUP_V1: u32 = 2_048;
/// Largest admitted flattened row count.
pub const QWEN3_SWIGLU_MAX_ROWS_V1: u32 = 2_048;
/// Largest admitted intermediate width.
pub const QWEN3_SWIGLU_MAX_INTERMEDIATE_V1: u32 = 12_288;
/// Largest admitted element count per buffer.
pub const QWEN3_SWIGLU_MAX_ELEMENTS_V1: u64 = 25_165_824;
/// Three pointer-plus-`u64`-length slice records.
pub const QWEN3_SWIGLU_EXPLICIT_KERNARG_BYTES_V1: u64 = 48;
/// Exact explicit plus COV6 hidden kernarg bytes.
pub const QWEN3_SWIGLU_TOTAL_KERNARG_BYTES_V1: u64 = 304;
/// Exact kernarg alignment.
pub const QWEN3_SWIGLU_KERNARG_ALIGNMENT_V1: u64 = 8;
/// Number of finite target/draft `SwiGLU` profiles.
pub const QWEN3_SWIGLU_PROFILE_COUNT_V1: usize = 22;
/// Exact byte length of the final canonical direct-LLVM source.
pub const QWEN3_SWIGLU_LLVM_BYTES_V1: usize = 34_885;
/// SHA-256 of the final canonical direct-LLVM source bytes.
pub const QWEN3_SWIGLU_LLVM_SHA256_V1: [u8; 32] = [
    0xc2, 0xf5, 0x26, 0xe8, 0x32, 0xb9, 0x0b, 0xa5, 0xc7, 0x81, 0xcf, 0x30, 0x1f, 0x53, 0x52, 0x9b,
    0x65, 0x25, 0x3b, 0xf4, 0xea, 0x80, 0x56, 0xdf, 0xcc, 0x8e, 0x4e, 0x25, 0xc6, 0xcd, 0x2a, 0xca,
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
const PROFILE_DOMAIN: &[u8] = b"FERRIC/QWEN3/SWIGLU/PROFILE/V1\0";
const CATALOG_DOMAIN: &[u8] = b"FERRIC/QWEN3/SWIGLU/CATALOG/V1\0";
const KERNEL_IR_DOMAIN: &[u8] = b"FERRIC/QWEN3/SWIGLU/KERNEL-IR/V1\0";
const SOURCE_BINDING_DOMAIN: &[u8] = b"FERRIC/QWEN3/SWIGLU/SOURCE-BINDING/V1\0";

/// Target or speculative-draft Qwen3 model role.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Qwen3SwiGluModelRoleV1 {
    /// Qwen3-8B target with intermediate width 12,288.
    Target8B = 1,
    /// Qwen3-0.6B draft with intermediate width 3,072.
    Draft06B = 2,
}

impl Qwen3SwiGluModelRoleV1 {
    /// Exact model hidden width feeding gate/up projections.
    #[must_use]
    pub const fn hidden_size(self) -> u32 {
        match self {
            Self::Target8B => 4_096,
            Self::Draft06B => 1_024,
        }
    }

    /// Exact gate/up/output intermediate width.
    #[must_use]
    pub const fn intermediate_size(self) -> u32 {
        match self {
            Self::Target8B => 12_288,
            Self::Draft06B => 3_072,
        }
    }
}

/// Closed Ferric `SwiGLU` bucket set.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Qwen3SwiGluBucketV1 {
    /// Prefill: one sequence with 128 active tokens.
    PrefillS1T128 = 1,
    /// Prefill: eight sequences with 128 active tokens each.
    PrefillS8T128 = 2,
    /// Prefill: one sequence with 512 active tokens.
    PrefillS1T512 = 3,
    /// Prefill: one sequence with 2,048 active tokens.
    PrefillS1T2048 = 4,
    /// Ordinary decode, one sequence and one active token.
    DecodeS1C8192 = 5,
    /// Ordinary decode, eight sequences and one active token each.
    DecodeS8C8192 = 6,
    /// Ordinary decode, 32 sequences and one active token each.
    DecodeS32C8192 = 7,
    /// Speculative S1K4: target width five, draft width four.
    SpecS1K4C8192 = 8,
    /// Speculative S8K4: target width five, draft width four.
    SpecS8K4C8192 = 9,
    /// Speculative S1K8: target width nine, draft width eight.
    SpecS1K8C8192 = 10,
    /// Speculative S1K16: target width 17, draft width 16.
    SpecS1K16C8192 = 11,
}

impl Qwen3SwiGluBucketV1 {
    /// Exact independent sequence count.
    #[must_use]
    pub const fn sequences(self) -> u32 {
        match self {
            Self::PrefillS8T128 | Self::DecodeS8C8192 | Self::SpecS8K4C8192 => 8,
            Self::DecodeS32C8192 => 32,
            Self::PrefillS1T128
            | Self::PrefillS1T512
            | Self::PrefillS1T2048
            | Self::DecodeS1C8192
            | Self::SpecS1K4C8192
            | Self::SpecS1K8C8192
            | Self::SpecS1K16C8192 => 1,
        }
    }

    /// Exact active-token count per sequence for one model role.
    #[must_use]
    pub const fn active_tokens(self, role: Qwen3SwiGluModelRoleV1) -> u32 {
        match self {
            Self::PrefillS1T128 | Self::PrefillS8T128 => 128,
            Self::PrefillS1T512 => 512,
            Self::PrefillS1T2048 => 2_048,
            Self::DecodeS1C8192 | Self::DecodeS8C8192 | Self::DecodeS32C8192 => 1,
            Self::SpecS1K4C8192 | Self::SpecS8K4C8192 => match role {
                Qwen3SwiGluModelRoleV1::Target8B => 5,
                Qwen3SwiGluModelRoleV1::Draft06B => 4,
            },
            Self::SpecS1K8C8192 => match role {
                Qwen3SwiGluModelRoleV1::Target8B => 9,
                Qwen3SwiGluModelRoleV1::Draft06B => 8,
            },
            Self::SpecS1K16C8192 => match role {
                Qwen3SwiGluModelRoleV1::Target8B => 17,
                Qwen3SwiGluModelRoleV1::Draft06B => 16,
            },
        }
    }
}

/// Exact machine arithmetic declaration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Qwen3SwiGluNumericalPolicyV1 {
    /// BF16 widening, sign-selected stable sigmoid through exact OCML exp,
    /// fixed FP32 multiply/divide order, and BF16 RNE output.
    StableSigmoidFp32OcmlExpBf16RneOutput = 1,
}

/// SHA-256 identity of one exact profile record.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Qwen3SwiGluProfileIdentityV1([u8; 32]);

impl Qwen3SwiGluProfileIdentityV1 {
    /// Returns the domain-separated identity bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// One exact checked target/draft `SwiGLU` profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Qwen3SwiGluProfileV1 {
    role: Qwen3SwiGluModelRoleV1,
    bucket: Qwen3SwiGluBucketV1,
    sequences: u32,
    active_tokens: u32,
    rows: u32,
    hidden_size: u32,
    intermediate_size: u32,
    elements: u64,
    bytes_per_buffer: u64,
    launch_workitems: [u32; 3],
    grid_workgroups: [u32; 3],
    numerical_policy: Qwen3SwiGluNumericalPolicyV1,
    identity: Qwen3SwiGluProfileIdentityV1,
}

impl Qwen3SwiGluProfileV1 {
    fn checked(
        role: Qwen3SwiGluModelRoleV1,
        bucket: Qwen3SwiGluBucketV1,
    ) -> Result<Self, Qwen3SwiGluCatalogErrorV1> {
        let sequences = bucket.sequences();
        let active_tokens = bucket.active_tokens(role);
        let rows = sequences
            .checked_mul(active_tokens)
            .ok_or(Qwen3SwiGluCatalogErrorV1::ExtentOverflow)?;
        let hidden_size = role.hidden_size();
        let intermediate_size = role.intermediate_size();
        let elements = u64::from(rows)
            .checked_mul(u64::from(intermediate_size))
            .ok_or(Qwen3SwiGluCatalogErrorV1::ExtentOverflow)?;
        if rows > QWEN3_SWIGLU_MAX_ROWS_V1
            || intermediate_size > QWEN3_SWIGLU_MAX_INTERMEDIATE_V1
            || elements > QWEN3_SWIGLU_MAX_ELEMENTS_V1
        {
            return Err(Qwen3SwiGluCatalogErrorV1::ResourceLimit);
        }
        let bytes_per_buffer = elements
            .checked_mul(2)
            .ok_or(Qwen3SwiGluCatalogErrorV1::ExtentOverflow)?;
        let workgroups = elements
            .checked_add(u64::from(QWEN3_SWIGLU_ELEMENTS_PER_WORKGROUP_V1 - 1))
            .ok_or(Qwen3SwiGluCatalogErrorV1::GridOverflow)?
            / u64::from(QWEN3_SWIGLU_ELEMENTS_PER_WORKGROUP_V1);
        let workgroups =
            u32::try_from(workgroups).map_err(|_| Qwen3SwiGluCatalogErrorV1::GridOverflow)?;
        let workitems = workgroups
            .checked_mul(QWEN3_SWIGLU_WORKGROUP_V1[0])
            .ok_or(Qwen3SwiGluCatalogErrorV1::GridOverflow)?;
        let covered_elements = u64::from(workitems)
            .checked_mul(u64::from(QWEN3_SWIGLU_ELEMENTS_PER_WORKITEM_V1))
            .ok_or(Qwen3SwiGluCatalogErrorV1::GridOverflow)?;
        if covered_elements < elements {
            return Err(Qwen3SwiGluCatalogErrorV1::GridOverflow);
        }
        let mut profile = Self {
            role,
            bucket,
            sequences,
            active_tokens,
            rows,
            hidden_size,
            intermediate_size,
            elements,
            bytes_per_buffer,
            launch_workitems: [workitems, 1, 1],
            grid_workgroups: [workgroups, 1, 1],
            numerical_policy: Qwen3SwiGluNumericalPolicyV1::StableSigmoidFp32OcmlExpBf16RneOutput,
            identity: Qwen3SwiGluProfileIdentityV1([0; 32]),
        };
        profile.identity = Qwen3SwiGluProfileIdentityV1(hash(PROFILE_DOMAIN, &profile.encode()));
        Ok(profile)
    }

    fn encode(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(128);
        bytes.push(self.role as u8);
        bytes.push(self.bucket as u8);
        for value in [
            self.sequences,
            self.active_tokens,
            self.rows,
            self.hidden_size,
            self.intermediate_size,
            QWEN3_SWIGLU_ELEMENTS_PER_WORKITEM_V1,
            QWEN3_SWIGLU_ELEMENTS_PER_WORKGROUP_V1,
        ] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in [self.elements, self.bytes_per_buffer] {
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
    pub const fn role(self) -> Qwen3SwiGluModelRoleV1 {
        self.role
    }

    /// Exact `SwiGLU` bucket.
    #[must_use]
    pub const fn bucket(self) -> Qwen3SwiGluBucketV1 {
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

    /// Exact flattened row count.
    #[must_use]
    pub const fn rows(self) -> u32 {
        self.rows
    }

    /// Exact role-bound hidden width feeding gate/up projections.
    #[must_use]
    pub const fn hidden_size(self) -> u32 {
        self.hidden_size
    }

    /// Exact role-bound gate/up/output width.
    #[must_use]
    pub const fn intermediate_size(self) -> u32 {
        self.intermediate_size
    }

    /// Exact gate/up/output element count.
    #[must_use]
    pub const fn elements(self) -> u64 {
        self.elements
    }

    /// Exact BF16 bytes in each of the three buffers.
    #[must_use]
    pub const fn bytes_per_buffer(self) -> u64 {
        self.bytes_per_buffer
    }

    /// Exact global extent measured in workitems.
    #[must_use]
    pub const fn launch_workitems(self) -> [u32; 3] {
        self.launch_workitems
    }

    /// Exact grid measured in 256-workitem workgroups.
    #[must_use]
    pub const fn grid_workgroups(self) -> [u32; 3] {
        self.grid_workgroups
    }

    /// Exact declared arithmetic policy.
    #[must_use]
    pub const fn numerical_policy(self) -> Qwen3SwiGluNumericalPolicyV1 {
        self.numerical_policy
    }

    /// Exact domain-separated profile identity.
    #[must_use]
    pub const fn identity(self) -> Qwen3SwiGluProfileIdentityV1 {
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
pub enum Qwen3SwiGluCatalogErrorV1 {
    /// Tensor extent arithmetic overflowed.
    ExtentOverflow,
    /// A derived row, width, or element count exceeded the reviewed ceiling.
    ResourceLimit,
    /// Workitem or workgroup arithmetic overflowed.
    GridOverflow,
    /// The catalog did not contain exactly twenty-two distinct records.
    CatalogClosure,
}

impl fmt::Display for Qwen3SwiGluCatalogErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Qwen3 SwiGLU catalog failed: {self:?}")
    }
}

impl std::error::Error for Qwen3SwiGluCatalogErrorV1 {}

/// Identity of the exact twenty-two-profile catalog.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Qwen3SwiGluProfileCatalogIdentityV1([u8; 32]);

impl Qwen3SwiGluProfileCatalogIdentityV1 {
    /// Returns the exact catalog identity bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Complete finite target/draft Qwen3 `SwiGLU` catalog.
#[derive(Debug, Eq, PartialEq)]
pub struct Qwen3SwiGluProfileCatalogV1 {
    profiles: Box<[Qwen3SwiGluProfileV1]>,
    canonical_bytes: Box<[u8]>,
    identity: Qwen3SwiGluProfileCatalogIdentityV1,
}

impl Qwen3SwiGluProfileCatalogV1 {
    /// Constructs the exact role-major, bucket-major catalog.
    ///
    /// # Errors
    ///
    /// Returns an error if checked extent/grid arithmetic fails, a profile
    /// exceeds the reviewed resource ceilings, or the finite catalog is not
    /// exactly twenty-two distinct records.
    pub fn canonical() -> Result<Self, Qwen3SwiGluCatalogErrorV1> {
        let mut profiles = Vec::with_capacity(QWEN3_SWIGLU_PROFILE_COUNT_V1);
        for role in [
            Qwen3SwiGluModelRoleV1::Target8B,
            Qwen3SwiGluModelRoleV1::Draft06B,
        ] {
            for bucket in [
                Qwen3SwiGluBucketV1::PrefillS1T128,
                Qwen3SwiGluBucketV1::PrefillS8T128,
                Qwen3SwiGluBucketV1::PrefillS1T512,
                Qwen3SwiGluBucketV1::PrefillS1T2048,
                Qwen3SwiGluBucketV1::DecodeS1C8192,
                Qwen3SwiGluBucketV1::DecodeS8C8192,
                Qwen3SwiGluBucketV1::DecodeS32C8192,
                Qwen3SwiGluBucketV1::SpecS1K4C8192,
                Qwen3SwiGluBucketV1::SpecS8K4C8192,
                Qwen3SwiGluBucketV1::SpecS1K8C8192,
                Qwen3SwiGluBucketV1::SpecS1K16C8192,
            ] {
                profiles.push(Qwen3SwiGluProfileV1::checked(role, bucket)?);
            }
        }
        if profiles.len() != QWEN3_SWIGLU_PROFILE_COUNT_V1
            || profiles.iter().enumerate().any(|(index, profile)| {
                profiles[index + 1..]
                    .iter()
                    .any(|other| profile.identity == other.identity)
            })
        {
            return Err(Qwen3SwiGluCatalogErrorV1::CatalogClosure);
        }
        let mut canonical_bytes = Vec::with_capacity(512);
        let profile_count =
            u32::try_from(profiles.len()).map_err(|_| Qwen3SwiGluCatalogErrorV1::CatalogClosure)?;
        canonical_bytes.extend_from_slice(&profile_count.to_le_bytes());
        canonical_bytes.extend_from_slice(QWEN3_SWIGLU_TARGET_V1.as_bytes());
        canonical_bytes.push(QWEN3_SWIGLU_CODE_OBJECT_VERSION_V1);
        for dimension in QWEN3_SWIGLU_WORKGROUP_V1 {
            canonical_bytes.extend_from_slice(&dimension.to_le_bytes());
        }
        for profile in &profiles {
            let encoded = profile.encode();
            let encoded_len = u32::try_from(encoded.len())
                .map_err(|_| Qwen3SwiGluCatalogErrorV1::CatalogClosure)?;
            canonical_bytes.extend_from_slice(&encoded_len.to_le_bytes());
            canonical_bytes.extend_from_slice(&encoded);
            canonical_bytes.extend_from_slice(profile.identity.as_bytes());
        }
        let identity = Qwen3SwiGluProfileCatalogIdentityV1(hash(CATALOG_DOMAIN, &canonical_bytes));
        Ok(Self {
            profiles: profiles.into_boxed_slice(),
            canonical_bytes: canonical_bytes.into_boxed_slice(),
            identity,
        })
    }

    /// Exact stable-order profile slice.
    #[must_use]
    pub fn profiles(&self) -> &[Qwen3SwiGluProfileV1] {
        &self.profiles
    }

    /// Looks up one exact role/bucket pair.
    #[must_use]
    pub fn profile(
        &self,
        role: Qwen3SwiGluModelRoleV1,
        bucket: Qwen3SwiGluBucketV1,
    ) -> Option<Qwen3SwiGluProfileV1> {
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
    pub const fn identity(&self) -> Qwen3SwiGluProfileCatalogIdentityV1 {
        self.identity
    }

    /// This structural roster grants no source, artifact, or launch authority.
    #[must_use]
    pub const fn grants_authority(&self) -> bool {
        false
    }
}

/// Semantic role of one three-slice ABI argument.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Qwen3SwiGluArgumentRoleV1 {
    /// BF16 gate-projection input.
    Gate = 1,
    /// BF16 up-projection input.
    Up = 2,
    /// BF16 activated output.
    Output = 3,
}

/// Tensor storage and logical shape for one ABI argument.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Qwen3SwiGluArgumentShapeV1 {
    /// BF16 `[rows, intermediate]`.
    RowsIntermediateBf16Bits = 1,
}

/// Scalar storage interpretation for one ABI argument.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Qwen3SwiGluScalarV1 {
    /// BF16 represented as `u16` storage bits.
    Bf16 = 1,
}

/// One exact pointer-plus-length ABI record.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Qwen3SwiGluArgumentV1 {
    /// Semantic tensor role.
    pub role: Qwen3SwiGluArgumentRoleV1,
    /// Exact logical storage shape.
    pub shape: Qwen3SwiGluArgumentShapeV1,
    /// Semantic scalar type.
    pub scalar: Qwen3SwiGluScalarV1,
    /// Explicit kernarg byte offset.
    pub offset: u32,
    /// Pointer-plus-length record size.
    pub size: u32,
    /// Record alignment.
    pub alignment: u32,
}

/// Exact ordered machine step retained by the semantic KIR.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Qwen3SwiGluRecurrenceStepV1 {
    /// Each workitem owns eight consecutive logical elements.
    EightContiguousElementsPerWorkitem,
    /// Gate and up BF16 storage widen exactly to FP32.
    WidenGateAndUpBf16ToFp32,
    /// Select `-gate` for nonnegative gate and `gate` otherwise.
    SelectStableSigmoidExpArgument,
    /// Evaluate the exact unresolved OCML exponential boundary once.
    OneOcmlExpF32,
    /// Form `1 + exp` then select `1` or `exp` as numerator.
    StableSigmoidNumeratorAndDenominator,
    /// Divide the selected numerator by the denominator.
    DivideSigmoidFp32,
    /// Evaluate gate times sigmoid, then multiply by up.
    GateSigmoidThenUpFp32,
    /// Trap on non-finite input/intermediate before the owned store.
    FiniteChecksBeforeStore,
    /// Narrow the output to BF16 with round-to-nearest, ties-to-even.
    NarrowOutputBf16Rne,
}

/// Per-workitem failure behavior.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Qwen3SwiGluExceptionalPolicyV1 {
    /// A workitem traps before its current element store. Earlier elements of
    /// that workitem or other workgroups may already have stored output, so no
    /// whole-dispatch atomicity is claimed.
    PerWorkitemTrapBeforeCurrentStoreNoGlobalAtomicity,
}

/// Exact Ferric semantic KIR for one role/bucket profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Qwen3SwiGluKernelIrV1 {
    module_id: String,
    kernel_id: String,
    arguments: [Qwen3SwiGluArgumentV1; 3],
    profile_identity: Qwen3SwiGluProfileIdentityV1,
    recurrence: [Qwen3SwiGluRecurrenceStepV1; 9],
    exceptional_policy: Qwen3SwiGluExceptionalPolicyV1,
    identity: [u8; 32],
}

impl Qwen3SwiGluKernelIrV1 {
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

    /// Exact three-slice gate/up/output ABI.
    #[must_use]
    pub const fn arguments(&self) -> &[Qwen3SwiGluArgumentV1; 3] {
        &self.arguments
    }

    /// Profile identity whose geometry this KIR retains.
    #[must_use]
    pub const fn profile_identity(&self) -> Qwen3SwiGluProfileIdentityV1 {
        self.profile_identity
    }

    /// Exact ordered schedule steps.
    #[must_use]
    pub const fn recurrence(&self) -> &[Qwen3SwiGluRecurrenceStepV1; 9] {
        &self.recurrence
    }

    /// Per-workitem exceptional behavior.
    #[must_use]
    pub const fn exceptional_policy(&self) -> Qwen3SwiGluExceptionalPolicyV1 {
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
pub fn qwen3_swiglu_kernel_ir_v1(profile: Qwen3SwiGluProfileV1) -> Qwen3SwiGluKernelIrV1 {
    let arguments = [
        argument(
            Qwen3SwiGluArgumentRoleV1::Gate,
            Qwen3SwiGluArgumentShapeV1::RowsIntermediateBf16Bits,
            Qwen3SwiGluScalarV1::Bf16,
            0,
        ),
        argument(
            Qwen3SwiGluArgumentRoleV1::Up,
            Qwen3SwiGluArgumentShapeV1::RowsIntermediateBf16Bits,
            Qwen3SwiGluScalarV1::Bf16,
            16,
        ),
        argument(
            Qwen3SwiGluArgumentRoleV1::Output,
            Qwen3SwiGluArgumentShapeV1::RowsIntermediateBf16Bits,
            Qwen3SwiGluScalarV1::Bf16,
            32,
        ),
    ];
    let recurrence = [
        Qwen3SwiGluRecurrenceStepV1::EightContiguousElementsPerWorkitem,
        Qwen3SwiGluRecurrenceStepV1::WidenGateAndUpBf16ToFp32,
        Qwen3SwiGluRecurrenceStepV1::SelectStableSigmoidExpArgument,
        Qwen3SwiGluRecurrenceStepV1::OneOcmlExpF32,
        Qwen3SwiGluRecurrenceStepV1::StableSigmoidNumeratorAndDenominator,
        Qwen3SwiGluRecurrenceStepV1::DivideSigmoidFp32,
        Qwen3SwiGluRecurrenceStepV1::GateSigmoidThenUpFp32,
        Qwen3SwiGluRecurrenceStepV1::FiniteChecksBeforeStore,
        Qwen3SwiGluRecurrenceStepV1::NarrowOutputBf16Rne,
    ];
    let exceptional_policy =
        Qwen3SwiGluExceptionalPolicyV1::PerWorkitemTrapBeforeCurrentStoreNoGlobalAtomicity;
    let mut encoded = Vec::with_capacity(256);
    encoded.extend_from_slice(b"ferric::qwen3::swiglu_v1");
    encoded.extend_from_slice(QWEN3_SWIGLU_KERNEL_SYMBOL_V1.as_bytes());
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
    Qwen3SwiGluKernelIrV1 {
        module_id: "ferric::qwen3::swiglu_v1".to_owned(),
        kernel_id: QWEN3_SWIGLU_KERNEL_SYMBOL_V1.to_owned(),
        arguments,
        profile_identity: profile.identity,
        recurrence,
        exceptional_policy,
        identity: hash(KERNEL_IR_DOMAIN, &encoded),
    }
}

const fn argument(
    role: Qwen3SwiGluArgumentRoleV1,
    shape: Qwen3SwiGluArgumentShapeV1,
    scalar: Qwen3SwiGluScalarV1,
    offset: u32,
) -> Qwen3SwiGluArgumentV1 {
    Qwen3SwiGluArgumentV1 {
        role,
        shape,
        scalar,
        offset,
        size: 16,
        alignment: 8,
    }
}

fn encode_recurrence_step(step: Qwen3SwiGluRecurrenceStepV1, bytes: &mut Vec<u8>) {
    match step {
        Qwen3SwiGluRecurrenceStepV1::EightContiguousElementsPerWorkitem => bytes.push(1),
        Qwen3SwiGluRecurrenceStepV1::WidenGateAndUpBf16ToFp32 => bytes.push(2),
        Qwen3SwiGluRecurrenceStepV1::SelectStableSigmoidExpArgument => bytes.push(3),
        Qwen3SwiGluRecurrenceStepV1::OneOcmlExpF32 => bytes.push(4),
        Qwen3SwiGluRecurrenceStepV1::StableSigmoidNumeratorAndDenominator => bytes.push(5),
        Qwen3SwiGluRecurrenceStepV1::DivideSigmoidFp32 => bytes.push(6),
        Qwen3SwiGluRecurrenceStepV1::GateSigmoidThenUpFp32 => bytes.push(7),
        Qwen3SwiGluRecurrenceStepV1::FiniteChecksBeforeStore => bytes.push(8),
        Qwen3SwiGluRecurrenceStepV1::NarrowOutputBf16Rne => bytes.push(9),
    }
}

/// One of the three exact ABI memory regions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Qwen3SwiGluBufferV1 {
    /// BF16 gate-projection input.
    Gate = 1,
    /// BF16 up-projection input.
    Up = 2,
    /// BF16 activated output.
    Output = 3,
}

/// Exact numerical-span admission failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Qwen3SwiGluBufferContractErrorV1 {
    /// A required address was zero.
    ZeroAddress(Qwen3SwiGluBufferV1),
    /// Byte length differed from the finite profile.
    ByteLength(Qwen3SwiGluBufferV1),
    /// Start address violated scalar alignment.
    Alignment(Qwen3SwiGluBufferV1),
    /// Exclusive end overflowed `u64`.
    RangeOverflow(Qwen3SwiGluBufferV1),
    /// Two exact graph regions overlapped.
    Aliasing,
}

impl fmt::Display for Qwen3SwiGluBufferContractErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Qwen3 SwiGLU buffer contract failed: {self:?}")
    }
}

impl std::error::Error for Qwen3SwiGluBufferContractErrorV1 {}

/// Exact checked spans in gate/up/output ABI order.
#[derive(Debug, Eq, PartialEq)]
pub struct Qwen3SwiGluBufferContractV1 {
    addresses: [u64; 3],
    ends: [u64; 3],
    byte_lengths: [u64; 3],
}

impl Qwen3SwiGluBufferContractV1 {
    /// Checks exact byte lengths, alignment, range overflow, and pairwise
    /// disjointness. It does not inspect or authenticate buffer content.
    ///
    /// # Errors
    ///
    /// Returns an error for a zero or misaligned address, a profile-length
    /// mismatch, exclusive-end overflow, or overlap between any two roles.
    pub fn checked(
        profile: Qwen3SwiGluProfileV1,
        addresses: [u64; 3],
        byte_lengths: [u64; 3],
    ) -> Result<Self, Qwen3SwiGluBufferContractErrorV1> {
        let expected = [profile.bytes_per_buffer; 3];
        let roles = [
            Qwen3SwiGluBufferV1::Gate,
            Qwen3SwiGluBufferV1::Up,
            Qwen3SwiGluBufferV1::Output,
        ];
        let mut ends = [0_u64; 3];
        for index in 0..3 {
            if addresses[index] == 0 {
                return Err(Qwen3SwiGluBufferContractErrorV1::ZeroAddress(roles[index]));
            }
            if byte_lengths[index] != expected[index] {
                return Err(Qwen3SwiGluBufferContractErrorV1::ByteLength(roles[index]));
            }
            if !addresses[index].is_multiple_of(2) {
                return Err(Qwen3SwiGluBufferContractErrorV1::Alignment(roles[index]));
            }
            ends[index] = addresses[index].checked_add(byte_lengths[index]).ok_or(
                Qwen3SwiGluBufferContractErrorV1::RangeOverflow(roles[index]),
            )?;
        }
        for left in 0..3 {
            for right in left + 1..3 {
                if addresses[left] < ends[right] && addresses[right] < ends[left] {
                    return Err(Qwen3SwiGluBufferContractErrorV1::Aliasing);
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
    pub const fn addresses(&self) -> [u64; 3] {
        self.addresses
    }

    /// Exact exclusive ends in ABI role order.
    #[must_use]
    pub const fn ends(&self) -> [u64; 3] {
        self.ends
    }

    /// Exact byte lengths in ABI role order.
    #[must_use]
    pub const fn byte_lengths(&self) -> [u64; 3] {
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
pub struct Qwen3SwiGluSourceBindingsV1 {
    source: [u8; 32],
    kernel_ir: [u8; 32],
    schedule: [u8; 32],
    target_plan: [u8; 32],
}

impl Qwen3SwiGluSourceBindingsV1 {
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
pub enum PrepareQwen3SwiGluKernelErrorV1 {
    /// A source label was zero or reused for another role.
    SourceBindings,
    /// The finite profile catalog failed closed.
    Catalog(Qwen3SwiGluCatalogErrorV1),
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

impl fmt::Display for PrepareQwen3SwiGluKernelErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Qwen3 SwiGLU preparation failed: {self:?}")
    }
}

impl std::error::Error for PrepareQwen3SwiGluKernelErrorV1 {}

/// Linear Ferric-owned source/KIR catalog and generic compiler handoff.
pub struct PreparedQwen3SwiGluKernelV1 {
    catalog: Qwen3SwiGluProfileCatalogV1,
    source_binding_identity: [u8; 32],
    llvm_sha256: [u8; 32],
    compiler_handoff_identity: CompilerModuleHandoffIdentityV2,
    manifest_identity: CompilerModuleSymbolManifestIdentityV1,
    compiler_handoff: CompilerModuleHandoffV2,
}

impl fmt::Debug for PreparedQwen3SwiGluKernelV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedQwen3SwiGluKernelV1")
            .field("catalog", &self.catalog.identity)
            .field("source_binding", &self.source_binding_identity)
            .field("llvm_sha256", &self.llvm_sha256)
            .field("compiler_handoff", &self.compiler_handoff_identity)
            .finish_non_exhaustive()
    }
}

impl PreparedQwen3SwiGluKernelV1 {
    /// Complete finite profile catalog retained by this owner.
    #[must_use]
    pub const fn catalog(&self) -> &Qwen3SwiGluProfileCatalogV1 {
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

    /// The declared arithmetic remains unproved against an operator reference.
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
/// Returns an error if caller labels alias or are zero, finite profile/KIR or
/// canonical LLVM validation fails, or the exact compiler envelope, symbol
/// manifest, or generic handoff cannot be constructed.
pub fn prepare_qwen3_swiglu_kernel_v1(
    bindings: Qwen3SwiGluSourceBindingsV1,
) -> Result<PreparedQwen3SwiGluKernelV1, PrepareQwen3SwiGluKernelErrorV1> {
    validate_source_bindings(bindings)?;
    let catalog = Qwen3SwiGluProfileCatalogV1::canonical()
        .map_err(PrepareQwen3SwiGluKernelErrorV1::Catalog)?;
    let mut kir_identities = Vec::with_capacity(QWEN3_SWIGLU_PROFILE_COUNT_V1 * 32);
    for profile in catalog.profiles() {
        let kir = qwen3_swiglu_kernel_ir_v1(*profile);
        if kir.profile_identity() != profile.identity()
            || kir.arguments()[0].role != Qwen3SwiGluArgumentRoleV1::Gate
            || kir.arguments()[1].role != Qwen3SwiGluArgumentRoleV1::Up
            || kir.arguments()[2].role != Qwen3SwiGluArgumentRoleV1::Output
            || kir.recurrence()[3] != Qwen3SwiGluRecurrenceStepV1::OneOcmlExpF32
        {
            return Err(PrepareQwen3SwiGluKernelErrorV1::KernelIr);
        }
        kir_identities.extend_from_slice(kir.identity());
    }
    let llvm = canonical_qwen3_swiglu_llvm();
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
            QWEN3_SWIGLU_KERNEL_SYMBOL_V1,
        ),
        (
            CompilerModuleSymbolRoleV1::KernelDescriptor,
            QWEN3_SWIGLU_KERNEL_DESCRIPTOR_SYMBOL_V1,
        ),
        (
            CompilerModuleSymbolRoleV1::UnresolvedExternalImport,
            OCML_EXP_F32,
        ),
    ])
    .map_err(PrepareQwen3SwiGluKernelErrorV1::SymbolManifest)?;
    let manifest_identity = manifest.identity();
    let compiler_handoff = CompilerModuleHandoffV2::new(
        CompilerModuleKindV1::LlvmTextIr,
        target,
        CodeObjectVersion::V6,
        envelope,
        manifest,
        llvm.as_bytes(),
    )
    .map_err(PrepareQwen3SwiGluKernelErrorV1::CompilerHandoff)?;
    let compiler_handoff_identity = compiler_handoff.identity();
    Ok(PreparedQwen3SwiGluKernelV1 {
        catalog,
        source_binding_identity,
        llvm_sha256,
        compiler_handoff_identity,
        manifest_identity,
        compiler_handoff,
    })
}

fn validate_source_bindings(
    bindings: Qwen3SwiGluSourceBindingsV1,
) -> Result<(), PrepareQwen3SwiGluKernelErrorV1> {
    let identities = [
        bindings.source,
        bindings.kernel_ir,
        bindings.schedule,
        bindings.target_plan,
    ];
    for (index, identity) in identities.iter().enumerate() {
        if identity == &[0; 32] || identities[index + 1..].contains(identity) {
            return Err(PrepareQwen3SwiGluKernelErrorV1::SourceBindings);
        }
    }
    Ok(())
}

fn exact_ocml_envelope(
    target: DeviceTargetV1,
) -> Result<fe2o3_compiler_ffi::CompilerFfiEnvelopeV1, PrepareQwen3SwiGluKernelErrorV1> {
    let semantic_text = lower_hex(&OCML_EXP_BOUNDARY);
    let fields = DeviceFfiContractFieldsV1 {
        direction: DEVICE_FFI_DIRECTION_IMPORT_V1,
        symbol: OCML_EXP_F32,
        calling_convention: "C",
        code_object_version: u16::from(QWEN3_SWIGLU_CODE_OBJECT_VERSION_V1),
        target: QWEN3_SWIGLU_TARGET_V1,
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
            "ferric_qwen_kernels::swiglu::__ocml_exp_f32",
            [0x50; 16],
            "__ferric_qwen3_swiglu_ocml_exp_f32_v1",
        )
        .map_err(PrepareQwen3SwiGluKernelErrorV1::CompilerEnvelope)?,
        OCML_EXP_F32,
        OCML_EXP_ABI,
        OCML_EXP_EFFECTS,
        OCML_EXP_BOUNDARY,
    )
    .map_err(PrepareQwen3SwiGluKernelErrorV1::CompilerEnvelope)?;
    let mut builder = CompilerFfiEnvelopeBuilderV1::new(target, CodeObjectVersion::V6, 1)
        .map_err(PrepareQwen3SwiGluKernelErrorV1::CompilerEnvelope)?;
    builder
        .push(contract)
        .map_err(PrepareQwen3SwiGluKernelErrorV1::CompilerEnvelope)?;
    builder
        .finish()
        .map_err(PrepareQwen3SwiGluKernelErrorV1::CompilerEnvelope)
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

fn canonical_qwen3_swiglu_llvm() -> String {
    let mut output = String::with_capacity(48 * 1024);
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

define amdgpu_kernel void @qwen3_swiglu_bf16_f32_v1(ptr addrspace(1) nocapture readonly align 2 %gate.data, i64 %gate.len, ptr addrspace(1) nocapture readonly align 2 %up.data, i64 %up.len, ptr addrspace(1) noalias nocapture writeonly align 2 %output.data, i64 %output.len) #0 !reqd_work_group_size !0 !kernel_arg_access_qual !1 !kernel_arg_type !2 !kernel_arg_base_type !2 !kernel_arg_type_qual !3 {
entry:
  %local.i32 = call i32 @llvm.amdgcn.workitem.id.x()
  %group.i32 = call i32 @llvm.amdgcn.workgroup.id.x()
  %local = zext i32 %local.i32 to i64
  %group = zext i32 %group.i32 to i64
  %group.base = shl nuw i64 %group, 11
  %thread.offset = shl nuw i64 %local, 3
  %thread.base = add nuw i64 %group.base, %thread.offset
  %local.ok = icmp ult i64 %local, 256

  %extent.3072 = icmp eq i64 %gate.len, 3072
  %extent.12288 = icmp eq i64 %gate.len, 12288
  %extent.24576 = icmp eq i64 %gate.len, 24576
  %extent.49152 = icmp eq i64 %gate.len, 49152
  %extent.61440 = icmp eq i64 %gate.len, 61440
  %extent.98304 = icmp eq i64 %gate.len, 98304
  %extent.110592 = icmp eq i64 %gate.len, 110592
  %extent.208896 = icmp eq i64 %gate.len, 208896
  %extent.393216 = icmp eq i64 %gate.len, 393216
  %extent.491520 = icmp eq i64 %gate.len, 491520
  %extent.1572864 = icmp eq i64 %gate.len, 1572864
  %extent.3145728 = icmp eq i64 %gate.len, 3145728
  %extent.6291456 = icmp eq i64 %gate.len, 6291456
  %extent.12582912 = icmp eq i64 %gate.len, 12582912
  %extent.25165824 = icmp eq i64 %gate.len, 25165824
  %extent.0 = or i1 %extent.3072, %extent.12288
  %extent.1 = or i1 %extent.24576, %extent.49152
  %extent.2 = or i1 %extent.61440, %extent.98304
  %extent.3 = or i1 %extent.110592, %extent.208896
  %extent.4 = or i1 %extent.393216, %extent.491520
  %extent.5 = or i1 %extent.1572864, %extent.3145728
  %extent.6 = or i1 %extent.6291456, %extent.12582912
  %extent.7 = or i1 %extent.0, %extent.1
  %extent.8 = or i1 %extent.2, %extent.3
  %extent.9 = or i1 %extent.4, %extent.5
  %extent.10 = or i1 %extent.7, %extent.8
  %extent.11 = or i1 %extent.9, %extent.6
  %extent.12 = or i1 %extent.10, %extent.11
  %known.extent = or i1 %extent.12, %extent.25165824
  %up.extent = icmp eq i64 %up.len, %gate.len
  %output.extent = icmp eq i64 %output.len, %gate.len
  %matching.inputs = and i1 %known.extent, %up.extent
  %matching.buffers = and i1 %matching.inputs, %output.extent
  %shape.ok = and i1 %matching.buffers, %local.ok
  br i1 %shape.ok, label %e0.check, label %trap

",
    );
    for slot in 0..QWEN3_SWIGLU_ELEMENTS_PER_WORKITEM_V1 {
        emit_swiglu_element(&mut output, slot);
    }
    output.push_str(
        r#"done:
  ret void

trap:
  call void @llvm.trap()
  ret void
}

attributes #0 = { nounwind "amdgpu-no-completion-action" "amdgpu-no-default-queue" "amdgpu-no-heap-ptr" "amdgpu-no-hostcall-ptr" "amdgpu-no-multigrid-sync-arg" "amdgpu-no-queue-ptr" "amdgpu-flat-work-group-size"="256,256" "target-cpu"="gfx942" "target-features"="-wavefrontsize32,+wavefrontsize64,-xnack" "denormal-fp-math-f32"="ieee,ieee" "unsafe-fp-math"="false" "no-infs-fp-math"="false" "no-nans-fp-math"="false" "no-signed-zeros-fp-math"="false" "approx-func-fp-math"="false" "fp-contract"="off" }
attributes #1 = { nounwind readnone speculatable willreturn }

!0 = !{i32 256, i32 1, i32 1}
!1 = !{!"read_only", !"none", !"read_only", !"none", !"write_only", !"none"}
!2 = !{!"ushort*", !"ulong", !"ushort*", !"ulong", !"ushort*", !"ulong"}
!3 = !{!"const", !"", !"const", !"", !"restrict", !""}
"#,
    );
    output
}

fn emit_swiglu_element(output: &mut String, slot: u32) {
    let next = if slot + 1 == QWEN3_SWIGLU_ELEMENTS_PER_WORKITEM_V1 {
        "done".to_owned()
    } else {
        format!("e{}.check", slot + 1)
    };
    writeln!(
        output,
        "e{slot}.check:\n\
         \x20 %e{slot}.index = add nuw i64 %thread.base, {slot}\n\
         \x20 %e{slot}.in.bounds = icmp ult i64 %e{slot}.index, %gate.len\n\
         \x20 br i1 %e{slot}.in.bounds, label %e{slot}.load, label %{next}\n\n\
         e{slot}.load:\n\
         \x20 %e{slot}.gate.ptr = getelementptr inbounds i16, ptr addrspace(1) %gate.data, i64 %e{slot}.index\n\
         \x20 %e{slot}.up.ptr = getelementptr inbounds i16, ptr addrspace(1) %up.data, i64 %e{slot}.index\n\
         \x20 %e{slot}.gate.bf16 = load i16, ptr addrspace(1) %e{slot}.gate.ptr, align 2\n\
         \x20 %e{slot}.up.bf16 = load i16, ptr addrspace(1) %e{slot}.up.ptr, align 2\n\
         \x20 %e{slot}.gate.wide = zext i16 %e{slot}.gate.bf16 to i32\n\
         \x20 %e{slot}.up.wide = zext i16 %e{slot}.up.bf16 to i32\n\
         \x20 %e{slot}.gate.bits = shl nuw i32 %e{slot}.gate.wide, 16\n\
         \x20 %e{slot}.up.bits = shl nuw i32 %e{slot}.up.wide, 16\n\
         \x20 %e{slot}.gate = bitcast i32 %e{slot}.gate.bits to float\n\
         \x20 %e{slot}.up = bitcast i32 %e{slot}.up.bits to float\n\
         \x20 %e{slot}.gate.exponent = and i32 %e{slot}.gate.bits, 2139095040\n\
         \x20 %e{slot}.up.exponent = and i32 %e{slot}.up.bits, 2139095040\n\
         \x20 %e{slot}.gate.finite = icmp ne i32 %e{slot}.gate.exponent, 2139095040\n\
         \x20 %e{slot}.up.finite = icmp ne i32 %e{slot}.up.exponent, 2139095040\n\
         \x20 %e{slot}.inputs.finite = and i1 %e{slot}.gate.finite, %e{slot}.up.finite\n\
         \x20 br i1 %e{slot}.inputs.finite, label %e{slot}.sigmoid, label %trap\n\n\
         e{slot}.sigmoid:\n\
         \x20 %e{slot}.gate.nonnegative = fcmp oge float %e{slot}.gate, 0.000000e+00\n\
         \x20 %e{slot}.negative.gate = fsub float -0.000000e+00, %e{slot}.gate\n\
         \x20 %e{slot}.exp.argument = select i1 %e{slot}.gate.nonnegative, float %e{slot}.negative.gate, float %e{slot}.gate\n\
         \x20 %e{slot}.exp = call float @__ocml_exp_f32(float %e{slot}.exp.argument)\n\
         \x20 %e{slot}.exp.bits = bitcast float %e{slot}.exp to i32\n\
         \x20 %e{slot}.exp.exponent = and i32 %e{slot}.exp.bits, 2139095040\n\
         \x20 %e{slot}.exp.finite = icmp ne i32 %e{slot}.exp.exponent, 2139095040\n\
         \x20 %e{slot}.exp.nonnegative = fcmp oge float %e{slot}.exp, 0.000000e+00\n\
         \x20 %e{slot}.exp.valid = and i1 %e{slot}.exp.finite, %e{slot}.exp.nonnegative\n\
         \x20 %e{slot}.numerator = select i1 %e{slot}.gate.nonnegative, float 1.000000e+00, float %e{slot}.exp\n\
         \x20 %e{slot}.denominator = fadd float 1.000000e+00, %e{slot}.exp\n\
         \x20 %e{slot}.denominator.bits = bitcast float %e{slot}.denominator to i32\n\
         \x20 %e{slot}.denominator.exponent = and i32 %e{slot}.denominator.bits, 2139095040\n\
         \x20 %e{slot}.denominator.finite = icmp ne i32 %e{slot}.denominator.exponent, 2139095040\n\
         \x20 %e{slot}.denominator.positive = fcmp ogt float %e{slot}.denominator, 0.000000e+00\n\
         \x20 %e{slot}.denominator.valid = and i1 %e{slot}.denominator.finite, %e{slot}.denominator.positive\n\
         \x20 %e{slot}.sigmoid.inputs.valid = and i1 %e{slot}.exp.valid, %e{slot}.denominator.valid\n\
         \x20 br i1 %e{slot}.sigmoid.inputs.valid, label %e{slot}.arithmetic, label %trap\n\n\
         e{slot}.arithmetic:\n\
         \x20 %e{slot}.sigmoid.value = fdiv float %e{slot}.numerator, %e{slot}.denominator\n\
         \x20 %e{slot}.silu = fmul float %e{slot}.gate, %e{slot}.sigmoid.value\n\
         \x20 %e{slot}.product = fmul float %e{slot}.silu, %e{slot}.up\n\
         \x20 %e{slot}.sigmoid.bits = bitcast float %e{slot}.sigmoid.value to i32\n\
         \x20 %e{slot}.silu.bits = bitcast float %e{slot}.silu to i32\n\
         \x20 %e{slot}.product.bits = bitcast float %e{slot}.product to i32\n\
         \x20 %e{slot}.sigmoid.exponent = and i32 %e{slot}.sigmoid.bits, 2139095040\n\
         \x20 %e{slot}.silu.exponent = and i32 %e{slot}.silu.bits, 2139095040\n\
         \x20 %e{slot}.product.exponent = and i32 %e{slot}.product.bits, 2139095040\n\
         \x20 %e{slot}.sigmoid.finite = icmp ne i32 %e{slot}.sigmoid.exponent, 2139095040\n\
         \x20 %e{slot}.silu.finite = icmp ne i32 %e{slot}.silu.exponent, 2139095040\n\
         \x20 %e{slot}.product.finite = icmp ne i32 %e{slot}.product.exponent, 2139095040\n\
         \x20 %e{slot}.arithmetic.finite.0 = and i1 %e{slot}.sigmoid.finite, %e{slot}.silu.finite\n\
         \x20 %e{slot}.arithmetic.finite = and i1 %e{slot}.arithmetic.finite.0, %e{slot}.product.finite\n\
         \x20 br i1 %e{slot}.arithmetic.finite, label %e{slot}.narrow, label %trap\n\n\
         e{slot}.narrow:\n\
         \x20 %e{slot}.lsb.shift = lshr i32 %e{slot}.product.bits, 16\n\
         \x20 %e{slot}.lsb = and i32 %e{slot}.lsb.shift, 1\n\
         \x20 %e{slot}.bias = add nuw nsw i32 32767, %e{slot}.lsb\n\
         \x20 %e{slot}.rounded = add i32 %e{slot}.product.bits, %e{slot}.bias\n\
         \x20 %e{slot}.output.wide = lshr i32 %e{slot}.rounded, 16\n\
         \x20 %e{slot}.output.exponent = and i32 %e{slot}.output.wide, 32640\n\
         \x20 %e{slot}.output.finite = icmp ne i32 %e{slot}.output.exponent, 32640\n\
         \x20 %e{slot}.output.bf16 = trunc i32 %e{slot}.output.wide to i16\n\
         \x20 br i1 %e{slot}.output.finite, label %e{slot}.store, label %trap\n\n\
         e{slot}.store:\n\
         \x20 %e{slot}.output.ptr = getelementptr inbounds i16, ptr addrspace(1) %output.data, i64 %e{slot}.index\n\
         \x20 store i16 %e{slot}.output.bf16, ptr addrspace(1) %e{slot}.output.ptr, align 2\n\
         \x20 br label %{next}\n"
    )
    .expect("writing to a String cannot fail");
}

fn validate_canonical_llvm(module: &str) -> Result<(), PrepareQwen3SwiGluKernelErrorV1> {
    let module_sha256: [u8; 32] = Sha256::digest(module.as_bytes()).into();
    let exact = module.len() == QWEN3_SWIGLU_LLVM_BYTES_V1
        && module_sha256 == QWEN3_SWIGLU_LLVM_SHA256_V1
        && crate::COV6_NO_RUNTIME_SERVICE_ATTRIBUTES_V1
            .iter()
            .all(|attribute| module.matches(attribute).count() == 1)
        && module.matches("define amdgpu_kernel").count() == 1
        && module
            .matches("declare float @__ocml_exp_f32(float)")
            .count()
            == 1
        && module.matches("call float @__ocml_exp_f32(float ").count() == 8
        && module.matches("call void @llvm.trap()").count() == 1
        && module.matches("store i16").count() == 8
        && module.contains("@llvm.amdgcn.workitem.id.x")
        && module.contains("@llvm.amdgcn.workgroup.id.x")
        && module.contains("%group.base = shl nuw i64 %group, 11")
        && module.contains("%thread.offset = shl nuw i64 %local, 3")
        && module.contains("%local.ok = icmp ult i64 %local, 256")
        && module.contains("%extent.3072 = icmp eq i64 %gate.len, 3072")
        && module.contains("%extent.25165824 = icmp eq i64 %gate.len, 25165824")
        && module.contains("%up.extent = icmp eq i64 %up.len, %gate.len")
        && module.contains("%output.extent = icmp eq i64 %output.len, %gate.len")
        && module.contains("%e0.index = add nuw i64 %thread.base, 0")
        && module.contains("%e7.index = add nuw i64 %thread.base, 7")
        && module.contains(
            "%e0.exp.argument = select i1 %e0.gate.nonnegative, float %e0.negative.gate, float %e0.gate",
        )
        && module.contains(
            "%e7.exp.argument = select i1 %e7.gate.nonnegative, float %e7.negative.gate, float %e7.gate",
        )
        && module.contains("%e0.sigmoid.value = fdiv float %e0.numerator, %e0.denominator")
        && module.contains("%e7.product = fmul float %e7.silu, %e7.up")
        && module.contains("%e0.bias = add nuw nsw i32 32767, %e0.lsb")
        && module.contains("%e7.output.exponent = and i32 %e7.output.wide, 32640")
        && module.contains("!0 = !{i32 256, i32 1, i32 1}")
        && module.contains("\"amdgpu-flat-work-group-size\"=\"256,256\"")
        && module.contains("\"fp-contract\"=\"off\"")
        && !module.contains(" fast ")
        && !module.contains("contract ")
        && !module.contains("reassoc ")
        && !module.contains("cache.sequence")
        && !module.contains("sequence.cache")
        && !module.contains("comgr")
        && !module.contains("COMGR");
    if !exact {
        return Err(PrepareQwen3SwiGluKernelErrorV1::CompilerModule);
    }
    Ok(())
}

/// Linear exact compiler handoff awaiting attempt-scoped Worker V2 execution.
pub struct InertQwen3SwiGluWorkerRequestV1 {
    prepared: PreparedQwen3SwiGluKernelV1,
}

impl fmt::Debug for InertQwen3SwiGluWorkerRequestV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InertQwen3SwiGluWorkerRequestV1")
            .field("catalog", &self.prepared.catalog.identity)
            .field("source_binding", &self.prepared.source_binding_identity)
            .field("handoff", &self.prepared.compiler_handoff_identity)
            .finish_non_exhaustive()
    }
}

impl InertQwen3SwiGluWorkerRequestV1 {
    /// Complete profile catalog retained by this request.
    #[must_use]
    pub const fn catalog(&self) -> &Qwen3SwiGluProfileCatalogV1 {
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
pub const fn lower_qwen3_swiglu_kernel_v1(
    prepared: PreparedQwen3SwiGluKernelV1,
) -> InertQwen3SwiGluWorkerRequestV1 {
    InertQwen3SwiGluWorkerRequestV1 { prepared }
}

/// Failure while executing the exact module through Worker V2.
#[derive(Debug)]
pub enum ExecuteQwen3SwiGluWorkerErrorV1 {
    /// Consumed attempt bytes differ from the exact prepared handoff.
    HandoffSubstitution,
    /// A fixed link option could not be represented.
    FixedLinkOption,
    /// The fixed HSACO output ceiling could not be represented.
    OutputConstraint(WorkerProtocolError),
    /// Reproducible bootstrap and exact replay failed.
    FirstBuild(FirstBuildWorkerV2Error),
}

impl fmt::Display for ExecuteQwen3SwiGluWorkerErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Qwen3 SwiGLU Worker V2 execution failed: {self:?}"
        )
    }
}

impl std::error::Error for ExecuteQwen3SwiGluWorkerErrorV1 {}

/// Linear Worker V2 bootstrap/replay evidence awaiting structural inspection.
pub struct InertQwen3SwiGluWorkerEvidenceV1 {
    prepared: PreparedQwen3SwiGluKernelV1,
    transaction_handoff: CompilerModuleHandoffIdentityV1,
    worker: InertFirstBuildWorkerV2EvidenceV1,
}

impl fmt::Debug for InertQwen3SwiGluWorkerEvidenceV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InertQwen3SwiGluWorkerEvidenceV1")
            .field("source_binding", &self.prepared.source_binding_identity)
            .field("transaction_handoff", &self.transaction_handoff)
            .field("worker", &self.worker.identity())
            .finish_non_exhaustive()
    }
}

impl InertQwen3SwiGluWorkerEvidenceV1 {
    /// Reproducible execution remains inert until exact structural inspection.
    #[must_use]
    pub const fn grants_artifact_authority(&self) -> bool {
        false
    }

    /// Worker output does not prove the declared numerical contract.
    #[must_use]
    pub const fn proves_numerical_contract(&self) -> bool {
        false
    }

    /// Worker output does not prove that the declared `SwiGLU` operation was
    /// implemented correctly.
    #[must_use]
    pub const fn proves_operator_refinement(&self) -> bool {
        false
    }

    /// Worker output establishes no memory or race refinement.
    #[must_use]
    pub const fn proves_memory_or_race_refinement(&self) -> bool {
        false
    }
}

/// Executes exact attempt bytes through Worker V2 bootstrap and replay.
///
/// # Errors
///
/// Returns an error if consumed attempt bytes drift from the prepared handoff,
/// fixed execution parameters cannot be represented, or bounded Worker V2
/// bootstrap/replay fails.
pub fn execute_qwen3_swiglu_worker_v2_v1(
    request: InertQwen3SwiGluWorkerRequestV1,
    consumed: ConsumedCompilerModuleHandoffV1,
    worker: &PinnedWorkerV1,
    limits: WorkerExecutionLimitsV1,
) -> Result<InertQwen3SwiGluWorkerEvidenceV1, ExecuteQwen3SwiGluWorkerErrorV1> {
    let InertQwen3SwiGluWorkerRequestV1 { prepared } = request;
    if consumed.bytes() != prepared.compiler_handoff.canonical_bytes() {
        return Err(ExecuteQwen3SwiGluWorkerErrorV1::HandoffSubstitution);
    }
    let transaction_handoff = consumed.identity();
    let worker_evidence = execute_reproducible_first_build_worker_v2(
        consumed,
        worker,
        Vec::new(),
        fixed_link_options()?,
        WorkerOutputConstraintsV1::new(MAX_HSACO_BYTES as u64)
            .map_err(ExecuteQwen3SwiGluWorkerErrorV1::OutputConstraint)?,
        limits,
    )
    .map_err(ExecuteQwen3SwiGluWorkerErrorV1::FirstBuild)?;
    Ok(InertQwen3SwiGluWorkerEvidenceV1 {
        prepared,
        transaction_handoff,
        worker: worker_evidence,
    })
}

/// Exact post-worker structural rejection.
#[derive(Debug)]
pub enum InspectQwen3SwiGluKernelErrorV1 {
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

impl fmt::Display for InspectQwen3SwiGluKernelErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Qwen3 SwiGLU structural inspection failed: {self:?}"
        )
    }
}

impl std::error::Error for InspectQwen3SwiGluKernelErrorV1 {}

/// Linear Worker output after strict transcript, provider, ABI, resource, and
/// loader inspection.
pub struct InspectedQwen3SwiGluKernelV1 {
    catalog: Qwen3SwiGluProfileCatalogV1,
    source_binding_identity: [u8; 32],
    compiler_handoff_identity: CompilerModuleHandoffIdentityV2,
    transaction_handoff: CompilerModuleHandoffIdentityV1,
    loader_plan: LoadPlan,
    worker: InertFirstBuildWorkerV2EvidenceV1,
}

impl fmt::Debug for InspectedQwen3SwiGluKernelV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InspectedQwen3SwiGluKernelV1")
            .field("catalog", &self.catalog.identity)
            .field("source_binding", &self.source_binding_identity)
            .field("compiler_handoff", &self.compiler_handoff_identity)
            .field("transaction_handoff", &self.transaction_handoff)
            .field("worker", &self.worker.identity())
            .finish_non_exhaustive()
    }
}

impl InspectedQwen3SwiGluKernelV1 {
    /// Exact profile catalog retained with the inspected output owner.
    #[must_use]
    pub const fn catalog(&self) -> &Qwen3SwiGluProfileCatalogV1 {
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

    /// Structural inspection does not prove memory or race refinement.
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
    /// Returns an error if the role/bucket is outside the finite catalog, any
    /// buffer span fails the exact bounds contract, or the inert host labels
    /// alias, are zero, or carry generation zero.
    pub fn bind_checked_profile(
        &self,
        role: Qwen3SwiGluModelRoleV1,
        bucket: Qwen3SwiGluBucketV1,
        addresses: [u64; 3],
        byte_lengths: [u64; 3],
        metadata: Qwen3SwiGluHostMetadataV1,
    ) -> Result<CheckedQwen3SwiGluLaunchV1, BindQwen3SwiGluLaunchErrorV1> {
        let profile = self
            .catalog
            .profile(role, bucket)
            .ok_or(BindQwen3SwiGluLaunchErrorV1::Profile)?;
        let buffers = Qwen3SwiGluBufferContractV1::checked(profile, addresses, byte_lengths)
            .map_err(BindQwen3SwiGluLaunchErrorV1::Buffers)?;
        metadata
            .validate()
            .map_err(BindQwen3SwiGluLaunchErrorV1::Metadata)?;
        Ok(CheckedQwen3SwiGluLaunchV1 {
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
/// Returns an error if transcript decoding or lineage validation fails, HSACO
/// inspection rejects the bytes, the exact kernel ABI/resource profile drifts,
/// or the strict COV6 loader rejects the same output.
pub fn inspect_qwen3_swiglu_kernel_v1(
    evidence: InertQwen3SwiGluWorkerEvidenceV1,
) -> Result<InspectedQwen3SwiGluKernelV1, InspectQwen3SwiGluKernelErrorV1> {
    let InertQwen3SwiGluWorkerEvidenceV1 {
        prepared,
        transaction_handoff,
        worker,
    } = evidence;
    validate_worker_lineage(&prepared, transaction_handoff, &worker)?;
    let bytes = worker.output_bytes();
    if !worker.output_identity().matches(bytes) {
        return Err(InspectQwen3SwiGluKernelErrorV1::SourceLineage);
    }
    let bound = inspect_and_bind_kernel_descriptors(bytes)
        .map_err(InspectQwen3SwiGluKernelErrorV1::Hsaco)?;
    let [kernel] = bound.inspection().kernels() else {
        return Err(InspectQwen3SwiGluKernelErrorV1::KernelProfile);
    };
    let [binding] = bound.bindings() else {
        return Err(InspectQwen3SwiGluKernelErrorV1::KernelProfile);
    };
    let exact = bound.inspection().code_object_version() == InspectedCodeObjectVersion::V6
        && bound.inspection().target().to_string() == QWEN3_SWIGLU_TARGET_V1
        && !bound.inspection().has_printf_metadata()
        && kernel.name() == QWEN3_SWIGLU_KERNEL_SYMBOL_V1
        && kernel.symbol() == QWEN3_SWIGLU_KERNEL_DESCRIPTOR_SYMBOL_V1
        && kernel.kernarg_segment_size() == QWEN3_SWIGLU_TOTAL_KERNARG_BYTES_V1
        && kernel.kernarg_segment_alignment() == QWEN3_SWIGLU_KERNARG_ALIGNMENT_V1
        && kernel.implicit_argument_offset() == Some(QWEN3_SWIGLU_EXPLICIT_KERNARG_BYTES_V1)
        && kernel.implicit_argument_size() == 256
        && kernel.required_workgroup_size() == Some(QWEN3_SWIGLU_WORKGROUP_V1)
        && kernel.max_flat_workgroup_size() == 256
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
        && exact_swiglu_explicit_arguments(kernel.explicit_arguments())
        && exact_hidden_arguments(
            kernel.hidden_arguments(),
            QWEN3_SWIGLU_EXPLICIT_KERNARG_BYTES_V1,
        );
    if !exact {
        return Err(InspectQwen3SwiGluKernelErrorV1::KernelProfile);
    }
    let loader = fe2o3_amdhsa_loader::validate(bytes, AdmittedProfile::Gfx942XnackOffCov6)
        .map_err(InspectQwen3SwiGluKernelErrorV1::Loader)?;
    let loader_plan = *loader.plan();
    Ok(InspectedQwen3SwiGluKernelV1 {
        catalog: prepared.catalog,
        source_binding_identity: prepared.source_binding_identity,
        compiler_handoff_identity: prepared.compiler_handoff_identity,
        transaction_handoff,
        loader_plan,
        worker,
    })
}

fn validate_worker_lineage(
    prepared: &PreparedQwen3SwiGluKernelV1,
    transaction_handoff: CompilerModuleHandoffIdentityV1,
    worker: &InertFirstBuildWorkerV2EvidenceV1,
) -> Result<(), InspectQwen3SwiGluKernelErrorV1> {
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
        return Err(InspectQwen3SwiGluKernelErrorV1::SourceLineage);
    }
    let bootstrap = InertDecodedWorkerExchangeV2::decode(
        worker.bootstrap_request_bytes(),
        worker.bootstrap().response().canonical_bytes(),
    )
    .map_err(InspectQwen3SwiGluKernelErrorV1::Protocol)?;
    let replay = InertDecodedWorkerExchangeV2::decode(
        worker.authorized_request_bytes(),
        worker.authorized().response().canonical_bytes(),
    )
    .map_err(InspectQwen3SwiGluKernelErrorV1::Protocol)?;
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
                    QWEN3_SWIGLU_KERNEL_SYMBOL_V1,
                    QWEN3_SWIGLU_KERNEL_DESCRIPTOR_SYMBOL_V1,
                ]
            || exchange.response().request_identity() != request.identity()
            || !exact_ocml_provider(exchange.response())
        {
            return Err(InspectQwen3SwiGluKernelErrorV1::SourceLineage);
        }
    }
    Ok(())
}

fn exact_ocml_provider(response: &fe2o3_hsaco_finalize::WorkerResponseV2) -> bool {
    let Some(provider) = response.device_library_provider() else {
        return false;
    };
    exact_ocml_provider_header(
        provider.provider_identity(),
        &provider.target().to_string(),
        provider.code_object_version(),
        provider.import_symbols(),
    ) && provider.manifest_identity() != &[0; 32]
        && provider.files().len() == OCML_PROVIDER_BASENAMES.len()
        && provider
            .files()
            .iter()
            .zip(OCML_PROVIDER_BASENAMES)
            .all(|(file, basename)| file.basename() == basename && file.sha256() != &[0; 32])
}

fn exact_ocml_provider_header(
    provider_identity: &str,
    target: &str,
    code_object_version: CodeObjectVersion,
    import_symbols: &[String],
) -> bool {
    provider_identity == OCML_PROVIDER_IDENTITY
        && target == QWEN3_SWIGLU_TARGET_V1
        && code_object_version == CodeObjectVersion::V6
        && import_symbols == [OCML_EXP_F32]
}

fn exact_swiglu_explicit_arguments(arguments: &[ExplicitArgument]) -> bool {
    if arguments.len() != 6 {
        return false;
    }
    for (index, name, access, alignment, accepted_type) in [
        (
            0,
            "gate.data",
            ArgumentAccess::ReadOnly,
            2,
            is_bf16_metadata_carrier as fn(ExplicitValueType) -> bool,
        ),
        (
            2,
            "up.data",
            ArgumentAccess::ReadOnly,
            2,
            is_bf16_metadata_carrier,
        ),
        (
            4,
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
    for (index, name) in [(1, "gate.len"), (3, "up.len"), (5, "output.len")] {
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

fn exact_hidden_arguments(arguments: &[HiddenArgument], offset: u64) -> bool {
    const RELATIVE: [(u64, u64, HiddenValueKind); 13] = [
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
    ];
    arguments.len() == RELATIVE.len()
        && arguments.iter().zip(RELATIVE).all(|(actual, expected)| {
            actual.offset() == offset + expected.0
                && actual.size() == expected.1
                && actual.value_kind() == expected.2
        })
}

fn fixed_link_options() -> Result<Vec<LinkOptionV1>, ExecuteQwen3SwiGluWorkerErrorV1> {
    [
        ("code-object-version", "6"),
        ("opt-level", "2"),
        ("strip-debug", "true"),
        ("verify-each", "true"),
    ]
    .into_iter()
    .map(|(name, value)| {
        LinkOptionV1::new(name, value).map_err(|_| ExecuteQwen3SwiGluWorkerErrorV1::FixedLinkOption)
    })
    .collect()
}

/// Failure while binding an inspected output to a finite runtime profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindQwen3SwiGluLaunchErrorV1 {
    /// The requested role/bucket tuple is absent from the finite catalog.
    Profile,
    /// Numerical buffer validation failed.
    Buffers(Qwen3SwiGluBufferContractErrorV1),
    /// Required host-only identity/generation labels failed closed.
    Metadata(Qwen3SwiGluHostMetadataErrorV1),
}

impl fmt::Display for BindQwen3SwiGluLaunchErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Qwen3 SwiGLU launch binding failed: {self:?}")
    }
}

impl std::error::Error for BindQwen3SwiGluLaunchErrorV1 {}

/// Untrusted host-side labels for the three-buffer allocation snapshot.
#[derive(Debug, Eq, PartialEq)]
pub struct Qwen3SwiGluHostMetadataV1 {
    gate_identity: [u8; 32],
    up_identity: [u8; 32],
    output_identity: [u8; 32],
    allocation_owner_identity: [u8; 32],
    allocation_generation: u64,
}

impl Qwen3SwiGluHostMetadataV1 {
    /// Constructs inert labels for the exact gate/up/output snapshot.
    #[must_use]
    pub const fn new(
        gate_identity: [u8; 32],
        up_identity: [u8; 32],
        output_identity: [u8; 32],
        allocation_owner_identity: [u8; 32],
        allocation_generation: u64,
    ) -> Self {
        Self {
            gate_identity,
            up_identity,
            output_identity,
            allocation_owner_identity,
            allocation_generation,
        }
    }

    fn validate(&self) -> Result<(), Qwen3SwiGluHostMetadataErrorV1> {
        let identities = [
            self.gate_identity,
            self.up_identity,
            self.output_identity,
            self.allocation_owner_identity,
        ];
        for (index, identity) in identities.iter().enumerate() {
            if identity == &[0; 32] || identities[index + 1..].contains(identity) {
                return Err(Qwen3SwiGluHostMetadataErrorV1::IdentityOrGeneration);
            }
        }
        if self.allocation_generation == 0 {
            return Err(Qwen3SwiGluHostMetadataErrorV1::IdentityOrGeneration);
        }
        Ok(())
    }

    /// Exact allocation generation label.
    #[must_use]
    pub const fn allocation_generation(&self) -> u64 {
        self.allocation_generation
    }

    /// These labels do not authenticate content, ownership, or generation.
    #[must_use]
    pub const fn authenticates_content_or_ownership(&self) -> bool {
        false
    }
}

/// Host metadata admission failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Qwen3SwiGluHostMetadataErrorV1 {
    /// A required identity/generation was absent or identities were aliased.
    IdentityOrGeneration,
}

/// Inert exact profile and numerical-buffer binding for a future protected launcher.
#[derive(Debug)]
pub struct CheckedQwen3SwiGluLaunchV1 {
    profile: Qwen3SwiGluProfileV1,
    buffers: Qwen3SwiGluBufferContractV1,
    metadata: Qwen3SwiGluHostMetadataV1,
}

impl CheckedQwen3SwiGluLaunchV1 {
    /// Exact finite profile.
    #[must_use]
    pub const fn profile(&self) -> Qwen3SwiGluProfileV1 {
        self.profile
    }

    /// Exact checked numerical buffer ranges.
    #[must_use]
    pub const fn buffers(&self) -> &Qwen3SwiGluBufferContractV1 {
        &self.buffers
    }

    /// Host-only labels retained outside the machine ABI.
    #[must_use]
    pub const fn metadata(&self) -> &Qwen3SwiGluHostMetadataV1 {
        &self.metadata
    }

    /// This binding grants no allocation, load, or launch authority.
    #[must_use]
    pub const fn grants_launch_authority(&self) -> bool {
        false
    }
}

fn exact_target() -> DeviceTargetV1 {
    DeviceTargetV1::parse(QWEN3_SWIGLU_TARGET_V1)
        .expect("the fixed Qwen3 SwiGLU target is canonical")
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

    fn bindings(seed: u8) -> Qwen3SwiGluSourceBindingsV1 {
        Qwen3SwiGluSourceBindingsV1::new(
            [seed; 32],
            [seed.wrapping_add(1); 32],
            [seed.wrapping_add(2); 32],
            [seed.wrapping_add(3); 32],
        )
    }

    fn profile(role: Qwen3SwiGluModelRoleV1, bucket: Qwen3SwiGluBucketV1) -> Qwen3SwiGluProfileV1 {
        Qwen3SwiGluProfileCatalogV1::canonical()
            .unwrap()
            .profile(role, bucket)
            .unwrap()
    }

    fn layout(profile: Qwen3SwiGluProfileV1) -> ([u64; 3], [u64; 3]) {
        (
            [0x1_0000_0000, 0x2_0000_0000, 0x3_0000_0000],
            [profile.bytes_per_buffer(); 3],
        )
    }

    fn metadata(generation: u64) -> Qwen3SwiGluHostMetadataV1 {
        Qwen3SwiGluHostMetadataV1::new([1; 32], [2; 32], [3; 32], [4; 32], generation)
    }

    fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
        haystack
            .windows(needle.len())
            .any(|window| window == needle)
    }

    #[test]
    fn exact_twenty_two_profile_catalog_is_complete_unique_and_bounded() {
        let catalog = Qwen3SwiGluProfileCatalogV1::canonical().unwrap();
        assert_eq!(catalog.profiles().len(), QWEN3_SWIGLU_PROFILE_COUNT_V1);
        assert_eq!(
            catalog
                .profiles()
                .iter()
                .copied()
                .map(Qwen3SwiGluProfileV1::identity)
                .collect::<BTreeSet<_>>()
                .len(),
            QWEN3_SWIGLU_PROFILE_COUNT_V1
        );
        assert_eq!(
            catalog
                .profiles()
                .iter()
                .map(|profile| profile.elements())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>(),
            vec![
                3_072, 12_288, 24_576, 49_152, 61_440, 98_304, 110_592, 208_896, 393_216, 491_520,
                1_572_864, 3_145_728, 6_291_456, 12_582_912, 25_165_824,
            ]
        );
        for profile in catalog.profiles() {
            assert_eq!(
                profile.rows(),
                profile
                    .sequences()
                    .checked_mul(profile.active_tokens())
                    .unwrap()
            );
            assert_eq!(
                profile.elements(),
                u64::from(profile.rows()) * u64::from(profile.intermediate_size())
            );
            assert_eq!(profile.bytes_per_buffer(), profile.elements() * 2);
            assert!(profile.rows() <= QWEN3_SWIGLU_MAX_ROWS_V1);
            assert!(profile.intermediate_size() <= QWEN3_SWIGLU_MAX_INTERMEDIATE_V1);
            assert!(profile.elements() <= QWEN3_SWIGLU_MAX_ELEMENTS_V1);
            let groups = profile
                .elements()
                .div_ceil(u64::from(QWEN3_SWIGLU_ELEMENTS_PER_WORKGROUP_V1));
            let groups = u32::try_from(groups).unwrap();
            assert_eq!(profile.grid_workgroups(), [groups, 1, 1]);
            assert_eq!(
                profile.launch_workitems(),
                [groups.checked_mul(256).unwrap(), 1, 1]
            );
            assert!(
                u64::from(profile.launch_workitems()[0])
                    * u64::from(QWEN3_SWIGLU_ELEMENTS_PER_WORKITEM_V1)
                    >= profile.elements()
            );
        }
        assert!(!catalog.grants_authority());
    }

    #[test]
    fn role_widths_and_bucket_rows_are_exact() {
        for role in [
            Qwen3SwiGluModelRoleV1::Target8B,
            Qwen3SwiGluModelRoleV1::Draft06B,
        ] {
            let expected_intermediate = match role {
                Qwen3SwiGluModelRoleV1::Target8B => 12_288,
                Qwen3SwiGluModelRoleV1::Draft06B => 3_072,
            };
            let expected_hidden = match role {
                Qwen3SwiGluModelRoleV1::Target8B => 4_096,
                Qwen3SwiGluModelRoleV1::Draft06B => 1_024,
            };
            for bucket in [
                Qwen3SwiGluBucketV1::PrefillS1T128,
                Qwen3SwiGluBucketV1::PrefillS8T128,
                Qwen3SwiGluBucketV1::PrefillS1T512,
                Qwen3SwiGluBucketV1::PrefillS1T2048,
                Qwen3SwiGluBucketV1::DecodeS1C8192,
                Qwen3SwiGluBucketV1::DecodeS8C8192,
                Qwen3SwiGluBucketV1::DecodeS32C8192,
                Qwen3SwiGluBucketV1::SpecS1K4C8192,
                Qwen3SwiGluBucketV1::SpecS8K4C8192,
                Qwen3SwiGluBucketV1::SpecS1K8C8192,
                Qwen3SwiGluBucketV1::SpecS1K16C8192,
            ] {
                let profile = profile(role, bucket);
                assert_eq!(profile.hidden_size(), expected_hidden);
                assert_eq!(profile.intermediate_size(), expected_intermediate);
            }
        }
        for (bucket, sequences, target_active, draft_active) in [
            (Qwen3SwiGluBucketV1::PrefillS1T128, 1, 128, 128),
            (Qwen3SwiGluBucketV1::PrefillS8T128, 8, 128, 128),
            (Qwen3SwiGluBucketV1::PrefillS1T512, 1, 512, 512),
            (Qwen3SwiGluBucketV1::PrefillS1T2048, 1, 2_048, 2_048),
            (Qwen3SwiGluBucketV1::DecodeS1C8192, 1, 1, 1),
            (Qwen3SwiGluBucketV1::DecodeS8C8192, 8, 1, 1),
            (Qwen3SwiGluBucketV1::DecodeS32C8192, 32, 1, 1),
            (Qwen3SwiGluBucketV1::SpecS1K4C8192, 1, 5, 4),
            (Qwen3SwiGluBucketV1::SpecS8K4C8192, 8, 5, 4),
            (Qwen3SwiGluBucketV1::SpecS1K8C8192, 1, 9, 8),
            (Qwen3SwiGluBucketV1::SpecS1K16C8192, 1, 17, 16),
        ] {
            let target = profile(Qwen3SwiGluModelRoleV1::Target8B, bucket);
            let draft = profile(Qwen3SwiGluModelRoleV1::Draft06B, bucket);
            assert_eq!(target.sequences(), sequences);
            assert_eq!(draft.sequences(), sequences);
            assert_eq!(target.active_tokens(), target_active);
            assert_eq!(draft.active_tokens(), draft_active);
            assert_eq!(target.rows(), sequences * target_active);
            assert_eq!(draft.rows(), sequences * draft_active);
        }
    }

    #[test]
    fn width_substitution_is_outside_the_catalog_identity() {
        let catalog = Qwen3SwiGluProfileCatalogV1::canonical().unwrap();
        let exact = profile(
            Qwen3SwiGluModelRoleV1::Draft06B,
            Qwen3SwiGluBucketV1::DecodeS1C8192,
        );
        let mut substituted = exact;
        substituted.intermediate_size = exact.hidden_size();
        substituted.elements =
            u64::from(substituted.rows) * u64::from(substituted.intermediate_size);
        substituted.bytes_per_buffer = substituted.elements * 2;
        assert!(!catalog.profiles().contains(&substituted));
        assert_ne!(exact.intermediate_size(), exact.hidden_size());
    }

    #[test]
    fn semantic_kir_fixes_three_bf16_slices_and_stable_silu_order() {
        let profile = profile(
            Qwen3SwiGluModelRoleV1::Target8B,
            Qwen3SwiGluBucketV1::SpecS1K16C8192,
        );
        let kir = qwen3_swiglu_kernel_ir_v1(profile);
        assert_eq!(
            kir.arguments().map(|argument| argument.role),
            [
                Qwen3SwiGluArgumentRoleV1::Gate,
                Qwen3SwiGluArgumentRoleV1::Up,
                Qwen3SwiGluArgumentRoleV1::Output,
            ]
        );
        assert!(kir
            .arguments()
            .iter()
            .all(|argument| argument.scalar == Qwen3SwiGluScalarV1::Bf16));
        assert_eq!(
            kir.recurrence(),
            &[
                Qwen3SwiGluRecurrenceStepV1::EightContiguousElementsPerWorkitem,
                Qwen3SwiGluRecurrenceStepV1::WidenGateAndUpBf16ToFp32,
                Qwen3SwiGluRecurrenceStepV1::SelectStableSigmoidExpArgument,
                Qwen3SwiGluRecurrenceStepV1::OneOcmlExpF32,
                Qwen3SwiGluRecurrenceStepV1::StableSigmoidNumeratorAndDenominator,
                Qwen3SwiGluRecurrenceStepV1::DivideSigmoidFp32,
                Qwen3SwiGluRecurrenceStepV1::GateSigmoidThenUpFp32,
                Qwen3SwiGluRecurrenceStepV1::FiniteChecksBeforeStore,
                Qwen3SwiGluRecurrenceStepV1::NarrowOutputBf16Rne,
            ]
        );
        assert_eq!(
            kir.exceptional_policy(),
            Qwen3SwiGluExceptionalPolicyV1::PerWorkitemTrapBeforeCurrentStoreNoGlobalAtomicity
        );
        assert!(!kir.proves_machine_refinement());
    }

    #[test]
    fn canonical_source_is_fully_pinned_and_structurally_closed() {
        let exact = canonical_qwen3_swiglu_llvm();
        assert_eq!(exact.len(), QWEN3_SWIGLU_LLVM_BYTES_V1);
        assert_eq!(
            <[u8; 32]>::from(Sha256::digest(exact.as_bytes())),
            QWEN3_SWIGLU_LLVM_SHA256_V1
        );
        validate_canonical_llvm(&exact).unwrap();
        for attribute in crate::COV6_NO_RUNTIME_SERVICE_ATTRIBUTES_V1 {
            let missing = exact.replacen(attribute, "", 1);
            assert_ne!(exact, missing);
            assert!(validate_canonical_llvm(&missing).is_err());
        }
        assert_eq!(
            exact.matches("call float @__ocml_exp_f32(float ").count(),
            8
        );
        assert_eq!(exact.matches("store i16").count(), 8);
        assert!(exact.contains("%e0.negative.gate = fsub float -0.000000e+00, %e0.gate"));
        assert!(exact.contains("%e7.product = fmul float %e7.silu, %e7.up"));
        assert!(!exact.contains(" fast "));
        assert!(!exact.contains("cache.sequence"));
    }

    #[test]
    fn source_classifier_rejects_extent_provider_helper_store_and_fast_math_substitutions() {
        let exact = canonical_qwen3_swiglu_llvm();
        for hostile in [
            exact.replacen(
                "%extent.25165824 = icmp eq i64 %gate.len, 25165824",
                "%extent.25165824 = icmp eq i64 %gate.len, 25165823",
                1,
            ),
            exact.replacen(
                "declare float @__ocml_exp_f32(float)",
                "declare float @host_exp_f32(float)",
                1,
            ),
            exact.replacen(
                "float %e0.negative.gate, float %e0.gate",
                "float %e0.gate, float %e0.negative.gate",
                1,
            ),
            exact.replacen("store i16 %e7.output.bf16", "store i16 %e0.output.bf16", 1),
            exact.replacen(
                "%e0.product = fmul float",
                "%e0.product = fmul fast float",
                1,
            ),
        ] {
            assert_ne!(hostile, exact);
            assert!(validate_canonical_llvm(&hostile).is_err());
        }
    }

    #[test]
    fn ocml_envelope_pins_exact_ferric_swiglu_semantic_owner() {
        let envelope = exact_ocml_envelope(exact_target()).unwrap();
        let bytes = envelope.canonical_bytes();
        assert!(contains_bytes(
            bytes,
            b"ferric_qwen_kernels::swiglu::__ocml_exp_f32"
        ));
        assert!(!contains_bytes(bytes, b"ferric_qwen_kernels::paged_decode"));
        let exact_owner = CompilerFfiSourceOwnerV1::new(
            "ferric_qwen_kernels",
            "ferric_qwen_kernels::swiglu::__ocml_exp_f32",
            [0x50; 16],
            "__ferric_qwen3_swiglu_ocml_exp_f32_v1",
        )
        .unwrap();
        let hostile_owner = CompilerFfiSourceOwnerV1::new(
            "ferric_qwen_kernels",
            "ferric_qwen_kernels::swiglu::host_exp_f32",
            [0x50; 16],
            "__ferric_qwen3_swiglu_ocml_exp_f32_v1",
        )
        .unwrap();
        assert_ne!(exact_owner.identity(), hostile_owner.identity());
    }

    #[test]
    fn ocml_provider_header_rejects_identity_target_version_and_import_substitutions() {
        let exact_imports = vec![OCML_EXP_F32.to_owned()];
        assert!(exact_ocml_provider_header(
            OCML_PROVIDER_IDENTITY,
            QWEN3_SWIGLU_TARGET_V1,
            CodeObjectVersion::V6,
            &exact_imports,
        ));
        assert!(!exact_ocml_provider_header(
            "gfx942-ocml-hostile",
            QWEN3_SWIGLU_TARGET_V1,
            CodeObjectVersion::V6,
            &exact_imports,
        ));
        assert!(!exact_ocml_provider_header(
            OCML_PROVIDER_IDENTITY,
            "gfx942:xnack+",
            CodeObjectVersion::V6,
            &exact_imports,
        ));
        assert!(!exact_ocml_provider_header(
            OCML_PROVIDER_IDENTITY,
            QWEN3_SWIGLU_TARGET_V1,
            CodeObjectVersion::V5,
            &exact_imports,
        ));
        assert!(!exact_ocml_provider_header(
            OCML_PROVIDER_IDENTITY,
            QWEN3_SWIGLU_TARGET_V1,
            CodeObjectVersion::V6,
            &["host_exp_f32".to_owned()],
        ));
    }

    #[test]
    fn source_bindings_fail_closed_on_zero_alias_and_role_drift() {
        assert!(prepare_qwen3_swiglu_kernel_v1(bindings(1)).is_ok());
        assert!(
            prepare_qwen3_swiglu_kernel_v1(Qwen3SwiGluSourceBindingsV1::new(
                [0; 32], [2; 32], [3; 32], [4; 32]
            ))
            .is_err()
        );
        assert!(
            prepare_qwen3_swiglu_kernel_v1(Qwen3SwiGluSourceBindingsV1::new(
                [1; 32], [2; 32], [1; 32], [4; 32]
            ))
            .is_err()
        );
        let first = prepare_qwen3_swiglu_kernel_v1(bindings(8)).unwrap();
        let second = prepare_qwen3_swiglu_kernel_v1(Qwen3SwiGluSourceBindingsV1::new(
            [8; 32], [9; 32], [11; 32], [10; 32],
        ))
        .unwrap();
        assert_ne!(
            first.source_binding_identity(),
            second.source_binding_identity()
        );
    }

    #[test]
    fn buffer_contract_rejects_extent_alignment_alias_and_overflow() {
        let profile = profile(
            Qwen3SwiGluModelRoleV1::Target8B,
            Qwen3SwiGluBucketV1::DecodeS1C8192,
        );
        let (addresses, lengths) = layout(profile);
        let exact = Qwen3SwiGluBufferContractV1::checked(profile, addresses, lengths).unwrap();
        assert_eq!(exact.byte_lengths(), lengths);
        assert!(!exact.authenticates_device_memory());

        let mut wrong_length = lengths;
        wrong_length[1] -= 2;
        assert!(matches!(
            Qwen3SwiGluBufferContractV1::checked(profile, addresses, wrong_length),
            Err(Qwen3SwiGluBufferContractErrorV1::ByteLength(
                Qwen3SwiGluBufferV1::Up
            ))
        ));
        let mut unaligned = addresses;
        unaligned[2] += 1;
        assert!(matches!(
            Qwen3SwiGluBufferContractV1::checked(profile, unaligned, lengths),
            Err(Qwen3SwiGluBufferContractErrorV1::Alignment(
                Qwen3SwiGluBufferV1::Output
            ))
        ));
        let mut alias = addresses;
        alias[2] = alias[0];
        assert_eq!(
            Qwen3SwiGluBufferContractV1::checked(profile, alias, lengths),
            Err(Qwen3SwiGluBufferContractErrorV1::Aliasing)
        );
        let mut overflowing = addresses;
        overflowing[0] = u64::MAX - lengths[0] + 1;
        assert!(matches!(
            Qwen3SwiGluBufferContractV1::checked(profile, overflowing, lengths),
            Err(Qwen3SwiGluBufferContractErrorV1::RangeOverflow(
                Qwen3SwiGluBufferV1::Gate
            ))
        ));
    }

    #[test]
    fn host_metadata_is_inert_and_rejects_alias_or_zero_generation() {
        let valid = metadata(9);
        valid.validate().unwrap();
        assert_eq!(valid.allocation_generation(), 9);
        assert!(!valid.authenticates_content_or_ownership());
        assert!(
            Qwen3SwiGluHostMetadataV1::new([1; 32], [2; 32], [1; 32], [4; 32], 9)
                .validate()
                .is_err()
        );
        assert!(metadata(0).validate().is_err());
    }

    #[test]
    fn prepared_and_request_stages_remain_inert_and_linear() {
        let prepared = prepare_qwen3_swiglu_kernel_v1(bindings(20)).unwrap();
        assert!(!prepared.uses_typed_handoff_v2_source());
        assert!(!prepared.authenticates_compiler_origin());
        assert!(!prepared.proves_operator_or_numerical_refinement());
        assert!(!prepared.has_ferric_plan_identity_join());
        assert!(!prepared.has_kernel_schedule_catalog_join());
        assert!(!prepared.grants_launch_authority());
        let request = lower_qwen3_swiglu_kernel_v1(prepared);
        assert_eq!(request.catalog().profiles().len(), 22);
        assert!(!request.authenticates_worker_execution());
        assert!(!request.grants_launch_authority());
    }
}
