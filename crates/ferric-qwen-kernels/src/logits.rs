//! Exact finite Qwen3 lowest-ID argmax and compact-completion kernels.
//!
//! K1 owns logits projection. This module starts from BF16 logits. Every
//! target and draft plan profile produces one lowest-ID argmax choice per
//! active row. Only target profiles may additionally encode publication
//! records: direct modes emit the final active-row choice, while speculative
//! modes emit the maximal accepted draft prefix plus target correction or
//! bonus. Draft profiles remain proposal-only.
//!
//! The catalog, source pin, Worker transcript, structural inspection, and
//! buffer binding do not establish numerical, operator, race, source-to-
//! machine, content, allocation, generation, load, launch, completion,
//! hardware, performance, or plan-publication refinement.

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

/// Exact lowest-ID argmax kernel entry.
pub const QWEN3_LOGITS_ARGMAX_KERNEL_SYMBOL_V1: &str = "ferric_qwen3_lowest_id_argmax_bf16_v1";
/// Exact lowest-ID argmax descriptor.
pub const QWEN3_LOGITS_ARGMAX_DESCRIPTOR_SYMBOL_V1: &str =
    "ferric_qwen3_lowest_id_argmax_bf16_v1.kd";
/// Exact target-only compact encoder entry.
pub const QWEN3_LOGITS_COMPACT_KERNEL_SYMBOL_V1: &str = "ferric_qwen3_compact_completion_v1";
/// Exact target-only compact encoder descriptor.
pub const QWEN3_LOGITS_COMPACT_DESCRIPTOR_SYMBOL_V1: &str = "ferric_qwen3_compact_completion_v1.kd";
/// Exact device target.
pub const QWEN3_LOGITS_TARGET_V1: &str = "gfx942:xnack-";
/// Exact code-object version.
pub const QWEN3_LOGITS_CODE_OBJECT_VERSION_V1: u8 = 6;
/// Exact workgroup for both entries.
pub const QWEN3_LOGITS_WORKGROUP_V1: [u32; 3] = [64, 1, 1];
/// Exact Qwen3 vocabulary.
pub const QWEN3_LOGITS_VOCABULARY_V1: u32 = 151_936;
/// Maximum speculative K.
pub const QWEN3_LOGITS_MAX_SPECULATIVE_K_V1: u32 = 16;
/// Fixed live-token capacity in one publication record.
pub const QWEN3_LOGITS_MAX_EMITTED_TOKENS_V1: u32 = 17;
/// Exact canonical compact-record byte count.
pub const QWEN3_LOGITS_COMPACT_RECORD_BYTES_V1: u64 = 120;
/// Exact number of target/draft profiles.
pub const QWEN3_LOGITS_PROFILE_COUNT_V1: usize = 22;
/// Exact argmax explicit kernarg bytes.
pub const QWEN3_LOGITS_ARGMAX_EXPLICIT_KERNARG_BYTES_V1: u64 = 40;
/// Exact argmax explicit plus COV6 hidden kernarg bytes.
pub const QWEN3_LOGITS_ARGMAX_TOTAL_KERNARG_BYTES_V1: u64 = 296;
/// Exact compact explicit kernarg bytes, rounded to eight-byte alignment.
pub const QWEN3_LOGITS_COMPACT_EXPLICIT_KERNARG_BYTES_V1: u64 = 128;
/// Exact compact explicit plus COV6 hidden kernarg bytes.
pub const QWEN3_LOGITS_COMPACT_TOTAL_KERNARG_BYTES_V1: u64 = 384;
/// Exact kernarg alignment.
pub const QWEN3_LOGITS_KERNARG_ALIGNMENT_V1: u64 = 8;
/// Exact final LLVM byte length.
pub const QWEN3_LOGITS_LLVM_BYTES_V1: usize = 18_437;
/// Exact final LLVM SHA-256.
pub const QWEN3_LOGITS_LLVM_SHA256_V1: [u8; 32] = [
    0x6c, 0x51, 0x0f, 0x86, 0x45, 0x55, 0xab, 0xb4, 0x73, 0x94, 0x29, 0x44, 0xcc, 0x8f, 0x86, 0x88,
    0x8d, 0x5c, 0x21, 0xc9, 0xaf, 0x6e, 0xe8, 0x14, 0x02, 0xf6, 0x13, 0x04, 0x5b, 0xce, 0x62, 0x68,
];

const PROFILE_DOMAIN: &[u8] = b"FERRIC/QWEN3/LOGITS/PROFILE/V1\0";
const CATALOG_DOMAIN: &[u8] = b"FERRIC/QWEN3/LOGITS/CATALOG/V1\0";
const KERNEL_IR_DOMAIN: &[u8] = b"FERRIC/QWEN3/LOGITS/KERNEL-IR/V1\0";
const SOURCE_BINDING_DOMAIN: &[u8] = b"FERRIC/QWEN3/LOGITS/SOURCE-BINDING/V1\0";

/// Target or speculative-draft model role.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Qwen3LogitsModelRoleV1 {
    /// Qwen3-8B target.
    Target8B = 1,
    /// Qwen3-0.6B draft.
    Draft06B = 2,
}

/// Ferric execution mode.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Qwen3LogitsModeV1 {
    /// Prompt prefill.
    Prefill = 1,
    /// Ordinary decode.
    Decode = 2,
    /// Speculative proposal or verification.
    Speculative = 3,
}

/// Closed Ferric plan bucket vocabulary.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Qwen3LogitsBucketKindV1 {
    /// S1T128 prefill.
    PrefillS1T128 = 1,
    /// S8T128 prefill.
    PrefillS8T128 = 2,
    /// S1T512 prefill.
    PrefillS1T512 = 3,
    /// S1T2048 prefill.
    PrefillS1T2048 = 4,
    /// S1 decode at context 8192.
    DecodeS1C8192 = 5,
    /// S8 decode at context 8192.
    DecodeS8C8192 = 6,
    /// S32 decode at context 8192.
    DecodeS32C8192 = 7,
    /// S1K4 speculation.
    SpeculativeS1K4C8192 = 8,
    /// S8K4 speculation.
    SpeculativeS8K4C8192 = 9,
    /// S1K8 speculation.
    SpeculativeS1K8C8192 = 10,
    /// S1K16 speculation.
    SpeculativeS1K16C8192 = 11,
}

impl Qwen3LogitsBucketKindV1 {
    const fn mode(self) -> Qwen3LogitsModeV1 {
        match self {
            Self::PrefillS1T128
            | Self::PrefillS8T128
            | Self::PrefillS1T512
            | Self::PrefillS1T2048 => Qwen3LogitsModeV1::Prefill,
            Self::DecodeS1C8192 | Self::DecodeS8C8192 | Self::DecodeS32C8192 => {
                Qwen3LogitsModeV1::Decode
            }
            _ => Qwen3LogitsModeV1::Speculative,
        }
    }

    const fn sequences(self) -> u32 {
        match self {
            Self::PrefillS8T128 | Self::DecodeS8C8192 | Self::SpeculativeS8K4C8192 => 8,
            Self::DecodeS32C8192 => 32,
            _ => 1,
        }
    }

    const fn active_tokens(self, role: Qwen3LogitsModelRoleV1) -> u32 {
        match self {
            Self::PrefillS1T128 | Self::PrefillS8T128 => 128,
            Self::PrefillS1T512 => 512,
            Self::PrefillS1T2048 => 2_048,
            Self::DecodeS1C8192 | Self::DecodeS8C8192 | Self::DecodeS32C8192 => 1,
            Self::SpeculativeS1K4C8192 | Self::SpeculativeS8K4C8192 => match role {
                Qwen3LogitsModelRoleV1::Target8B => 5,
                Qwen3LogitsModelRoleV1::Draft06B => 4,
            },
            Self::SpeculativeS1K8C8192 => match role {
                Qwen3LogitsModelRoleV1::Target8B => 9,
                Qwen3LogitsModelRoleV1::Draft06B => 8,
            },
            Self::SpeculativeS1K16C8192 => match role {
                Qwen3LogitsModelRoleV1::Target8B => 17,
                Qwen3LogitsModelRoleV1::Draft06B => 16,
            },
        }
    }

    const fn speculative_k(self) -> u32 {
        match self {
            Self::SpeculativeS1K4C8192 | Self::SpeculativeS8K4C8192 => 4,
            Self::SpeculativeS1K8C8192 => 8,
            Self::SpeculativeS1K16C8192 => 16,
            _ => 0,
        }
    }
}

/// Role-bound bucket identity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Qwen3LogitsBucketV1 {
    role: Qwen3LogitsModelRoleV1,
    kind: Qwen3LogitsBucketKindV1,
}

impl Qwen3LogitsBucketV1 {
    /// Constructs one exact role-bound bucket.
    #[must_use]
    pub const fn new(role: Qwen3LogitsModelRoleV1, kind: Qwen3LogitsBucketKindV1) -> Self {
        Self { role, kind }
    }

    /// Model role.
    #[must_use]
    pub const fn role(self) -> Qwen3LogitsModelRoleV1 {
        self.role
    }

    /// Plan bucket.
    #[must_use]
    pub const fn kind(self) -> Qwen3LogitsBucketKindV1 {
        self.kind
    }

    /// Execution mode.
    #[must_use]
    pub const fn mode(self) -> Qwen3LogitsModeV1 {
        self.kind.mode()
    }

    /// Exact [sequences, active tokens].
    #[must_use]
    pub const fn shape(self) -> [u32; 2] {
        [self.kind.sequences(), self.kind.active_tokens(self.role)]
    }

    /// Exact flattened row count.
    #[must_use]
    pub const fn rows(self) -> u32 {
        let [sequences, active] = self.shape();
        sequences * active
    }

    /// Speculative K, excluding the target bonus row.
    #[must_use]
    pub const fn speculative_k(self) -> u32 {
        self.kind.speculative_k()
    }
}

/// Physical output behavior admitted for one role.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Qwen3LogitsCompletionKindV1 {
    /// Draft output is proposal choices only.
    DraftChoices = 1,
    /// Target direct mode emits one final-row token.
    TargetDirect = 2,
    /// Target speculative mode emits accepted prefix plus correction or bonus.
    TargetSpeculative = 3,
}

/// Stable profile identity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Qwen3LogitsProfileIdentityV1([u8; 32]);

impl Qwen3LogitsProfileIdentityV1 {
    /// Identity bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Exact finite argmax and optional compact profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Qwen3LogitsProfileV1 {
    bucket: Qwen3LogitsBucketV1,
    completion: Qwen3LogitsCompletionKindV1,
    logits_elements: u64,
    choices_elements: u64,
    draft_elements: u64,
    compact_record_bytes: u64,
    identity: Qwen3LogitsProfileIdentityV1,
}

impl Qwen3LogitsProfileV1 {
    fn canonical(bucket: Qwen3LogitsBucketV1) -> Option<Self> {
        let [sequences, active] = bucket.shape();
        let rows = u64::from(sequences).checked_mul(u64::from(active))?;
        let logits_elements = rows.checked_mul(u64::from(QWEN3_LOGITS_VOCABULARY_V1))?;
        let completion = match (bucket.role(), bucket.mode()) {
            (Qwen3LogitsModelRoleV1::Draft06B, _) => Qwen3LogitsCompletionKindV1::DraftChoices,
            (Qwen3LogitsModelRoleV1::Target8B, Qwen3LogitsModeV1::Speculative) => {
                Qwen3LogitsCompletionKindV1::TargetSpeculative
            }
            (Qwen3LogitsModelRoleV1::Target8B, _) => Qwen3LogitsCompletionKindV1::TargetDirect,
        };
        let draft_elements = if completion == Qwen3LogitsCompletionKindV1::TargetSpeculative {
            u64::from(sequences).checked_mul(u64::from(bucket.speculative_k()))?
        } else {
            0
        };
        let compact_record_bytes = if completion == Qwen3LogitsCompletionKindV1::DraftChoices {
            0
        } else {
            u64::from(sequences).checked_mul(QWEN3_LOGITS_COMPACT_RECORD_BYTES_V1)?
        };
        let mut encoded = Vec::with_capacity(64);
        encoded.push(bucket.role() as u8);
        encoded.push(bucket.kind() as u8);
        encoded.push(bucket.mode() as u8);
        encoded.push(completion as u8);
        encoded.extend_from_slice(&sequences.to_le_bytes());
        encoded.extend_from_slice(&active.to_le_bytes());
        encoded.extend_from_slice(&bucket.rows().to_le_bytes());
        encoded.extend_from_slice(&bucket.speculative_k().to_le_bytes());
        encoded.extend_from_slice(&QWEN3_LOGITS_VOCABULARY_V1.to_le_bytes());
        encoded.extend_from_slice(&logits_elements.to_le_bytes());
        encoded.extend_from_slice(&rows.to_le_bytes());
        encoded.extend_from_slice(&draft_elements.to_le_bytes());
        encoded.extend_from_slice(&compact_record_bytes.to_le_bytes());
        Some(Self {
            bucket,
            completion,
            logits_elements,
            choices_elements: rows,
            draft_elements,
            compact_record_bytes,
            identity: Qwen3LogitsProfileIdentityV1(hash(PROFILE_DOMAIN, &encoded)),
        })
    }

    /// Role-bound plan bucket.
    #[must_use]
    pub const fn bucket(self) -> Qwen3LogitsBucketV1 {
        self.bucket
    }

    /// Output behavior.
    #[must_use]
    pub const fn completion(self) -> Qwen3LogitsCompletionKindV1 {
        self.completion
    }

    /// Exact [sequences, active tokens, vocabulary].
    #[must_use]
    pub const fn logits_shape(self) -> [u32; 3] {
        let [sequences, active] = self.bucket.shape();
        [sequences, active, QWEN3_LOGITS_VOCABULARY_V1]
    }

    /// Exact [sequences, active tokens] choice shape.
    #[must_use]
    pub const fn choice_shape(self) -> [u32; 2] {
        self.bucket.shape()
    }

    /// Exact speculative K.
    #[must_use]
    pub const fn speculative_k(self) -> u32 {
        self.bucket.speculative_k()
    }

    /// Exact [logits, choices, draft tokens, record bytes] extents.
    #[must_use]
    pub const fn storage_extents(self) -> [u64; 4] {
        [
            self.logits_elements,
            self.choices_elements,
            self.draft_elements,
            self.compact_record_bytes,
        ]
    }

    /// Argmax AQL workitems.
    #[must_use]
    pub const fn argmax_grid_workitems(self) -> [u32; 3] {
        [self.bucket.rows() * QWEN3_LOGITS_WORKGROUP_V1[0], 1, 1]
    }

    /// Compact AQL workitems, absent for draft profiles.
    #[must_use]
    pub const fn compact_grid_workitems(self) -> Option<[u32; 3]> {
        match self.completion {
            Qwen3LogitsCompletionKindV1::DraftChoices => None,
            Qwen3LogitsCompletionKindV1::TargetDirect
            | Qwen3LogitsCompletionKindV1::TargetSpeculative => {
                Some([self.bucket.shape()[0] * QWEN3_LOGITS_WORKGROUP_V1[0], 1, 1])
            }
        }
    }

    /// Stable Ferric profile identity.
    #[must_use]
    pub const fn identity(self) -> Qwen3LogitsProfileIdentityV1 {
        self.identity
    }

    /// Catalog structure is not numerical or machine refinement.
    #[must_use]
    pub const fn proves_refinement(self) -> bool {
        false
    }
}

/// Catalog construction failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Qwen3LogitsCatalogErrorV1 {
    /// Checked profile arithmetic failed.
    Arithmetic,
    /// Profile count or a canonical field drifted.
    ProfileSet,
    /// A stable host identity was repeated.
    DuplicateIdentity,
}

impl fmt::Display for Qwen3LogitsCatalogErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Qwen3 logits catalog failed: {self:?}")
    }
}

impl std::error::Error for Qwen3LogitsCatalogErrorV1 {}

/// Stable catalog identity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Qwen3LogitsProfileCatalogIdentityV1([u8; 32]);

impl Qwen3LogitsProfileCatalogIdentityV1 {
    /// Identity bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Complete 22-profile catalog.
#[derive(Debug, Eq, PartialEq)]
pub struct Qwen3LogitsProfileCatalogV1 {
    profiles: Vec<Qwen3LogitsProfileV1>,
    identity: Qwen3LogitsProfileCatalogIdentityV1,
}

impl Qwen3LogitsProfileCatalogV1 {
    /// Constructs and validates all target/draft profiles.
    ///
    /// # Errors
    ///
    /// Returns an error if checked extent construction, the finite profile set,
    /// or profile identity uniqueness differs from the canonical catalog.
    pub fn canonical() -> Result<Self, Qwen3LogitsCatalogErrorV1> {
        let mut profiles = Vec::with_capacity(QWEN3_LOGITS_PROFILE_COUNT_V1);
        for role in QWEN3_LOGITS_ROLES_V1 {
            for kind in QWEN3_LOGITS_BUCKETS_V1 {
                profiles.push(
                    Qwen3LogitsProfileV1::canonical(Qwen3LogitsBucketV1::new(role, kind))
                        .ok_or(Qwen3LogitsCatalogErrorV1::Arithmetic)?,
                );
            }
        }
        if profiles.len() != QWEN3_LOGITS_PROFILE_COUNT_V1 {
            return Err(Qwen3LogitsCatalogErrorV1::ProfileSet);
        }
        for index in 0..profiles.len() {
            if profiles[index + 1..]
                .iter()
                .any(|profile| profile.identity == profiles[index].identity)
            {
                return Err(Qwen3LogitsCatalogErrorV1::DuplicateIdentity);
            }
        }
        let mut encoded = Vec::with_capacity(4 + profiles.len() * 32);
        encoded.extend_from_slice(
            &u32::try_from(profiles.len())
                .map_err(|_| Qwen3LogitsCatalogErrorV1::Arithmetic)?
                .to_le_bytes(),
        );
        for profile in &profiles {
            encoded.extend_from_slice(profile.identity.as_bytes());
        }
        Ok(Self {
            profiles,
            identity: Qwen3LogitsProfileCatalogIdentityV1(hash(CATALOG_DOMAIN, &encoded)),
        })
    }

    /// Profiles in canonical role then bucket order.
    #[must_use]
    pub fn profiles(&self) -> &[Qwen3LogitsProfileV1] {
        &self.profiles
    }

    /// Catalog identity.
    #[must_use]
    pub const fn identity(&self) -> Qwen3LogitsProfileCatalogIdentityV1 {
        self.identity
    }

    /// Finds one exact role-bound profile.
    #[must_use]
    pub fn profile(&self, bucket: Qwen3LogitsBucketV1) -> Option<Qwen3LogitsProfileV1> {
        self.profiles
            .iter()
            .copied()
            .find(|profile| profile.bucket == bucket)
    }
}

const QWEN3_LOGITS_ROLES_V1: [Qwen3LogitsModelRoleV1; 2] = [
    Qwen3LogitsModelRoleV1::Target8B,
    Qwen3LogitsModelRoleV1::Draft06B,
];

const QWEN3_LOGITS_BUCKETS_V1: [Qwen3LogitsBucketKindV1; 11] = [
    Qwen3LogitsBucketKindV1::PrefillS1T128,
    Qwen3LogitsBucketKindV1::PrefillS8T128,
    Qwen3LogitsBucketKindV1::PrefillS1T512,
    Qwen3LogitsBucketKindV1::PrefillS1T2048,
    Qwen3LogitsBucketKindV1::DecodeS1C8192,
    Qwen3LogitsBucketKindV1::DecodeS8C8192,
    Qwen3LogitsBucketKindV1::DecodeS32C8192,
    Qwen3LogitsBucketKindV1::SpeculativeS1K4C8192,
    Qwen3LogitsBucketKindV1::SpeculativeS8K4C8192,
    Qwen3LogitsBucketKindV1::SpeculativeS1K8C8192,
    Qwen3LogitsBucketKindV1::SpeculativeS1K16C8192,
];

/// One physical buffer role.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Qwen3LogitsBufferV1 {
    /// BF16 logits.
    Logits = 1,
    /// U32 argmax choices.
    Choices = 2,
    /// U32 speculative draft tokens.
    DraftTokens = 3,
    /// U32 request slots.
    RequestSlots = 4,
    /// U32 request generations.
    RequestGenerations = 5,
    /// U64 completion epochs.
    Epochs = 6,
    /// Per-request 32-byte plan identities.
    PlanIdentities = 7,
    /// Canonical 120-byte records.
    Records = 8,
}

/// Checked binding rejection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Qwen3LogitsBufferContractErrorV1 {
    /// A byte length differed.
    Length(Qwen3LogitsBufferV1),
    /// A pointer was zero or insufficiently aligned.
    Address(Qwen3LogitsBufferV1),
    /// A byte interval overflowed.
    Overflow(Qwen3LogitsBufferV1),
    /// Two buffers overlap.
    Aliasing(Qwen3LogitsBufferV1, Qwen3LogitsBufferV1),
    /// Draft and target compact behavior were confused.
    CompletionMode,
    /// The compact stage did not consume the exact argmax output.
    ChoiceJoin,
}

impl fmt::Display for Qwen3LogitsBufferContractErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Qwen3 logits buffer binding failed: {self:?}")
    }
}

impl std::error::Error for Qwen3LogitsBufferContractErrorV1 {}

/// Exact argmax slice binding.
#[derive(Debug, Eq, PartialEq)]
pub struct Qwen3ArgmaxBufferContractV1 {
    addresses: [u64; 2],
    byte_lengths: [u64; 2],
}

impl Qwen3ArgmaxBufferContractV1 {
    /// Addresses in [logits, choices] order.
    #[must_use]
    pub const fn addresses(&self) -> [u64; 2] {
        self.addresses
    }

    /// Byte lengths in [logits, choices] order.
    #[must_use]
    pub const fn byte_lengths(&self) -> [u64; 2] {
        self.byte_lengths
    }
}

/// Exact target-only compact slice binding.
#[derive(Debug, Eq, PartialEq)]
pub struct Qwen3CompactBufferContractV1 {
    addresses: [u64; 7],
    byte_lengths: [u64; 7],
}

impl Qwen3CompactBufferContractV1 {
    /// Addresses in choices, draft, slot, generation, epoch, plan, record order.
    #[must_use]
    pub const fn addresses(&self) -> [u64; 7] {
        self.addresses
    }

    /// Exact byte lengths in the same order.
    #[must_use]
    pub const fn byte_lengths(&self) -> [u64; 7] {
        self.byte_lengths
    }
}

/// Joined exact argmax and optional compact binding.
#[derive(Debug, Eq, PartialEq)]
pub struct Qwen3LogitsBufferContractV1 {
    argmax: Qwen3ArgmaxBufferContractV1,
    compact: Option<Qwen3CompactBufferContractV1>,
}

impl Qwen3LogitsBufferContractV1 {
    fn checked(
        profile: Qwen3LogitsProfileV1,
        argmax_addresses: [u64; 2],
        argmax_lengths: [u64; 2],
        compact: Option<([u64; 7], [u64; 7])>,
    ) -> Result<Self, Qwen3LogitsBufferContractErrorV1> {
        let [logits_elements, choices_elements, draft_elements, record_bytes] =
            profile.storage_extents();
        let expected_argmax =
            [
                logits_elements.checked_mul(2).ok_or(
                    Qwen3LogitsBufferContractErrorV1::Overflow(Qwen3LogitsBufferV1::Logits),
                )?,
                choices_elements.checked_mul(4).ok_or(
                    Qwen3LogitsBufferContractErrorV1::Overflow(Qwen3LogitsBufferV1::Choices),
                )?,
            ];
        check_slice(
            Qwen3LogitsBufferV1::Logits,
            argmax_addresses[0],
            argmax_lengths[0],
            expected_argmax[0],
            2,
        )?;
        check_slice(
            Qwen3LogitsBufferV1::Choices,
            argmax_addresses[1],
            argmax_lengths[1],
            expected_argmax[1],
            4,
        )?;
        check_disjoint(
            Qwen3LogitsBufferV1::Logits,
            argmax_addresses[0],
            argmax_lengths[0],
            Qwen3LogitsBufferV1::Choices,
            argmax_addresses[1],
            argmax_lengths[1],
        )?;
        let argmax = Qwen3ArgmaxBufferContractV1 {
            addresses: argmax_addresses,
            byte_lengths: argmax_lengths,
        };
        if profile.completion() == Qwen3LogitsCompletionKindV1::DraftChoices {
            if compact.is_some() {
                return Err(Qwen3LogitsBufferContractErrorV1::CompletionMode);
            }
            return Ok(Self {
                argmax,
                compact: None,
            });
        }
        let (addresses, lengths) =
            compact.ok_or(Qwen3LogitsBufferContractErrorV1::CompletionMode)?;
        if addresses[0] != argmax_addresses[1] || lengths[0] != argmax_lengths[1] {
            return Err(Qwen3LogitsBufferContractErrorV1::ChoiceJoin);
        }
        let sequences = u64::from(profile.choice_shape()[0]);
        let expected = [
            expected_argmax[1],
            draft_elements
                .checked_mul(4)
                .ok_or(Qwen3LogitsBufferContractErrorV1::Overflow(
                    Qwen3LogitsBufferV1::DraftTokens,
                ))?,
            sequences * 4,
            sequences * 4,
            sequences * 8,
            sequences * 32,
            record_bytes,
        ];
        let roles = [
            Qwen3LogitsBufferV1::Choices,
            Qwen3LogitsBufferV1::DraftTokens,
            Qwen3LogitsBufferV1::RequestSlots,
            Qwen3LogitsBufferV1::RequestGenerations,
            Qwen3LogitsBufferV1::Epochs,
            Qwen3LogitsBufferV1::PlanIdentities,
            Qwen3LogitsBufferV1::Records,
        ];
        let alignments = [4, 4, 4, 4, 8, 1, 4];
        for index in 0..7 {
            check_slice(
                roles[index],
                addresses[index],
                lengths[index],
                expected[index],
                alignments[index],
            )?;
        }
        for first in 0..7 {
            for second in first + 1..7 {
                check_disjoint(
                    roles[first],
                    addresses[first],
                    lengths[first],
                    roles[second],
                    addresses[second],
                    lengths[second],
                )?;
            }
        }
        for index in 1..7 {
            check_disjoint(
                Qwen3LogitsBufferV1::Logits,
                argmax_addresses[0],
                argmax_lengths[0],
                roles[index],
                addresses[index],
                lengths[index],
            )?;
        }
        Ok(Self {
            argmax,
            compact: Some(Qwen3CompactBufferContractV1 {
                addresses,
                byte_lengths: lengths,
            }),
        })
    }

    /// Exact argmax binding.
    #[must_use]
    pub const fn argmax(&self) -> &Qwen3ArgmaxBufferContractV1 {
        &self.argmax
    }

    /// Exact target compact binding, absent for draft.
    #[must_use]
    pub const fn compact(&self) -> Option<&Qwen3CompactBufferContractV1> {
        self.compact.as_ref()
    }
}

fn check_slice(
    role: Qwen3LogitsBufferV1,
    address: u64,
    actual: u64,
    expected: u64,
    alignment: u64,
) -> Result<(), Qwen3LogitsBufferContractErrorV1> {
    if actual != expected {
        return Err(Qwen3LogitsBufferContractErrorV1::Length(role));
    }
    if address == 0 || !address.is_multiple_of(alignment) {
        return Err(Qwen3LogitsBufferContractErrorV1::Address(role));
    }
    address
        .checked_add(actual)
        .ok_or(Qwen3LogitsBufferContractErrorV1::Overflow(role))?;
    Ok(())
}

fn check_disjoint(
    first_role: Qwen3LogitsBufferV1,
    first_address: u64,
    first_length: u64,
    second_role: Qwen3LogitsBufferV1,
    second_address: u64,
    second_length: u64,
) -> Result<(), Qwen3LogitsBufferContractErrorV1> {
    if first_length == 0 || second_length == 0 {
        return Ok(());
    }
    let first_end = first_address
        .checked_add(first_length)
        .ok_or(Qwen3LogitsBufferContractErrorV1::Overflow(first_role))?;
    let second_end = second_address
        .checked_add(second_length)
        .ok_or(Qwen3LogitsBufferContractErrorV1::Overflow(second_role))?;
    if first_address < second_end && second_address < first_end {
        return Err(Qwen3LogitsBufferContractErrorV1::Aliasing(
            first_role,
            second_role,
        ));
    }
    Ok(())
}

/// Semantic KIR argument access.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Qwen3LogitsArgumentAccessV1 {
    /// Read-only slice.
    ReadOnly,
    /// Write-only slice.
    WriteOnly,
}

/// One semantic KIR slice.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Qwen3LogitsArgumentV1 {
    /// Buffer role.
    pub role: Qwen3LogitsBufferV1,
    /// Access mode.
    pub access: Qwen3LogitsArgumentAccessV1,
    /// Exact elements or bytes for byte storage.
    pub extent: u64,
    /// Storage bytes per element.
    pub element_bytes: u8,
}

/// Ferric-owned semantic sidecar for one exact profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Qwen3LogitsKernelIrV1 {
    profile_identity: Qwen3LogitsProfileIdentityV1,
    completion: Qwen3LogitsCompletionKindV1,
    argmax_arguments: [Qwen3LogitsArgumentV1; 2],
    compact_arguments: Option<[Qwen3LogitsArgumentV1; 7]>,
    identity: [u8; 32],
}

impl Qwen3LogitsKernelIrV1 {
    /// Profile identity retained by the sidecar.
    #[must_use]
    pub const fn profile_identity(&self) -> Qwen3LogitsProfileIdentityV1 {
        self.profile_identity
    }

    /// Exact output behavior.
    #[must_use]
    pub const fn completion(&self) -> Qwen3LogitsCompletionKindV1 {
        self.completion
    }

    /// Exact two-slice argmax boundary.
    #[must_use]
    pub const fn argmax_arguments(&self) -> &[Qwen3LogitsArgumentV1; 2] {
        &self.argmax_arguments
    }

    /// Exact seven-slice target compact boundary.
    #[must_use]
    pub const fn compact_arguments(&self) -> Option<&[Qwen3LogitsArgumentV1; 7]> {
        self.compact_arguments.as_ref()
    }

    /// Stable KIR identity.
    #[must_use]
    pub const fn identity(&self) -> &[u8; 32] {
        &self.identity
    }
}

/// Lowers one profile to the Ferric semantic KIR sidecar.
#[must_use]
pub fn qwen3_logits_kernel_ir_v1(profile: Qwen3LogitsProfileV1) -> Qwen3LogitsKernelIrV1 {
    let [logits, choices, draft, records] = profile.storage_extents();
    let argmax_arguments = [
        Qwen3LogitsArgumentV1 {
            role: Qwen3LogitsBufferV1::Logits,
            access: Qwen3LogitsArgumentAccessV1::ReadOnly,
            extent: logits,
            element_bytes: 2,
        },
        Qwen3LogitsArgumentV1 {
            role: Qwen3LogitsBufferV1::Choices,
            access: Qwen3LogitsArgumentAccessV1::WriteOnly,
            extent: choices,
            element_bytes: 4,
        },
    ];
    let compact_arguments = if profile.completion() == Qwen3LogitsCompletionKindV1::DraftChoices {
        None
    } else {
        let sequences = u64::from(profile.choice_shape()[0]);
        Some([
            Qwen3LogitsArgumentV1 {
                role: Qwen3LogitsBufferV1::Choices,
                access: Qwen3LogitsArgumentAccessV1::ReadOnly,
                extent: choices,
                element_bytes: 4,
            },
            Qwen3LogitsArgumentV1 {
                role: Qwen3LogitsBufferV1::DraftTokens,
                access: Qwen3LogitsArgumentAccessV1::ReadOnly,
                extent: draft,
                element_bytes: 4,
            },
            Qwen3LogitsArgumentV1 {
                role: Qwen3LogitsBufferV1::RequestSlots,
                access: Qwen3LogitsArgumentAccessV1::ReadOnly,
                extent: sequences,
                element_bytes: 4,
            },
            Qwen3LogitsArgumentV1 {
                role: Qwen3LogitsBufferV1::RequestGenerations,
                access: Qwen3LogitsArgumentAccessV1::ReadOnly,
                extent: sequences,
                element_bytes: 4,
            },
            Qwen3LogitsArgumentV1 {
                role: Qwen3LogitsBufferV1::Epochs,
                access: Qwen3LogitsArgumentAccessV1::ReadOnly,
                extent: sequences,
                element_bytes: 8,
            },
            Qwen3LogitsArgumentV1 {
                role: Qwen3LogitsBufferV1::PlanIdentities,
                access: Qwen3LogitsArgumentAccessV1::ReadOnly,
                extent: sequences * 32,
                element_bytes: 1,
            },
            Qwen3LogitsArgumentV1 {
                role: Qwen3LogitsBufferV1::Records,
                access: Qwen3LogitsArgumentAccessV1::WriteOnly,
                extent: records,
                element_bytes: 1,
            },
        ])
    };
    let mut encoded = Vec::with_capacity(256);
    encoded.extend_from_slice(profile.identity().as_bytes());
    encoded.push(profile.completion() as u8);
    for argument in argmax_arguments
        .iter()
        .chain(compact_arguments.iter().flatten())
    {
        encoded.push(argument.role as u8);
        encoded.push(argument.access as u8);
        encoded.push(argument.element_bytes);
        encoded.extend_from_slice(&argument.extent.to_le_bytes());
    }
    Qwen3LogitsKernelIrV1 {
        profile_identity: profile.identity(),
        completion: profile.completion(),
        argmax_arguments,
        compact_arguments,
        identity: hash(KERNEL_IR_DOMAIN, &encoded),
    }
}

/// Ferric-domain labels binding source, KIR, schedule, and target plan.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Qwen3LogitsSourceBindingsV1 {
    /// Complete source closure label.
    pub source: [u8; 32],
    /// Semantic KIR label.
    pub kernel_ir: [u8; 32],
    /// Finite schedule label.
    pub schedule: [u8; 32],
    /// Ferric target-plan label.
    pub target_plan: [u8; 32],
}

impl Qwen3LogitsSourceBindingsV1 {
    /// Constructs the exact four-label binding.
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
}

/// Failure while preparing the complete Ferric logits compiler owner.
#[derive(Debug)]
pub enum PrepareQwen3LogitsKernelErrorV1 {
    /// A source-stage label was zero or repeated.
    SourceBindings,
    /// The finite profile catalog failed.
    Catalog(Qwen3LogitsCatalogErrorV1),
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

impl fmt::Display for PrepareQwen3LogitsKernelErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Qwen3 logits preparation failed: {self:?}")
    }
}

impl std::error::Error for PrepareQwen3LogitsKernelErrorV1 {}

/// Linear prepared compiler owner awaiting Worker request construction.
pub struct PreparedQwen3LogitsKernelV1 {
    catalog: Qwen3LogitsProfileCatalogV1,
    source_binding_identity: [u8; 32],
    llvm_sha256: [u8; 32],
    compiler_handoff_identity: CompilerModuleHandoffIdentityV2,
    manifest_identity: CompilerModuleSymbolManifestIdentityV1,
    compiler_handoff: CompilerModuleHandoffV2,
}

impl fmt::Debug for PreparedQwen3LogitsKernelV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedQwen3LogitsKernelV1")
            .field("catalog", &self.catalog.identity)
            .field("source_binding", &self.source_binding_identity)
            .field("llvm_sha256", &self.llvm_sha256)
            .field("compiler_handoff", &self.compiler_handoff_identity)
            .finish_non_exhaustive()
    }
}

impl PreparedQwen3LogitsKernelV1 {
    /// Complete finite profile catalog.
    #[must_use]
    pub const fn catalog(&self) -> &Qwen3LogitsProfileCatalogV1 {
        &self.catalog
    }

    /// Ferric-domain source binding.
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

    /// Closed two-entry/two-descriptor manifest identity.
    #[must_use]
    pub const fn manifest_identity(&self) -> CompilerModuleSymbolManifestIdentityV1 {
        self.manifest_identity
    }

    /// Exact generic compiler handoff.
    #[must_use]
    pub const fn compiler_handoff(&self) -> &CompilerModuleHandoffV2 {
        &self.compiler_handoff
    }

    /// The machine classifier does not distinguish machine-equivalent profiles.
    #[must_use]
    pub const fn classifier_distinguishes_duplicate_profiles(&self) -> bool {
        false
    }

    /// Direct LLVM does not authenticate compiler origin.
    #[must_use]
    pub const fn authenticates_compiler_origin(&self) -> bool {
        false
    }

    /// Structural source does not prove operator or numerical refinement.
    #[must_use]
    pub const fn proves_operator_or_numerical_refinement(&self) -> bool {
        false
    }

    /// Preparation does not close the generated-plan join.
    #[must_use]
    pub const fn has_ferric_plan_identity_join(&self) -> bool {
        false
    }

    /// Preparation grants no artifact, load, publication, or launch authority.
    #[must_use]
    pub const fn grants_launch_authority(&self) -> bool {
        false
    }
}

/// Constructs the finite catalog, KIR family, pinned LLVM, and handoff.
///
/// # Errors
///
/// Returns an error when source labels, the finite catalog or KIR family, the
/// pinned LLVM module, compiler envelope, symbol manifest, or handoff differ
/// from the closed Ferric boundary.
pub fn prepare_qwen3_logits_kernel_v1(
    bindings: Qwen3LogitsSourceBindingsV1,
) -> Result<PreparedQwen3LogitsKernelV1, PrepareQwen3LogitsKernelErrorV1> {
    validate_source_bindings(bindings)?;
    let catalog = Qwen3LogitsProfileCatalogV1::canonical()
        .map_err(PrepareQwen3LogitsKernelErrorV1::Catalog)?;
    let mut kir_identities = Vec::with_capacity(QWEN3_LOGITS_PROFILE_COUNT_V1 * 32);
    for profile in catalog.profiles() {
        let kir = qwen3_logits_kernel_ir_v1(*profile);
        if kir.profile_identity() != profile.identity()
            || kir.completion() != profile.completion()
            || kir.argmax_arguments()[0].access != Qwen3LogitsArgumentAccessV1::ReadOnly
            || kir.argmax_arguments()[1].access != Qwen3LogitsArgumentAccessV1::WriteOnly
            || kir.compact_arguments().is_some()
                != (profile.completion() != Qwen3LogitsCompletionKindV1::DraftChoices)
        {
            return Err(PrepareQwen3LogitsKernelErrorV1::KernelIr);
        }
        kir_identities.extend_from_slice(kir.identity());
    }
    let llvm = canonical_qwen3_logits_llvm();
    validate_canonical_llvm(&llvm)?;
    let llvm_sha256: [u8; 32] = Sha256::digest(llvm.as_bytes()).into();
    let mut source_preimage = Vec::with_capacity(32 * (6 + QWEN3_LOGITS_PROFILE_COUNT_V1));
    source_preimage.extend_from_slice(&bindings.source);
    source_preimage.extend_from_slice(&bindings.kernel_ir);
    source_preimage.extend_from_slice(&bindings.schedule);
    source_preimage.extend_from_slice(&bindings.target_plan);
    source_preimage.extend_from_slice(catalog.identity.as_bytes());
    source_preimage.extend_from_slice(&kir_identities);
    source_preimage.extend_from_slice(&llvm_sha256);
    let source_binding_identity = hash(SOURCE_BINDING_DOMAIN, &source_preimage);
    let target = exact_target();
    let envelope =
        CompilerFfiEnvelopeV1::for_module_without_device_ffi(target, CodeObjectVersion::V6)
            .map_err(PrepareQwen3LogitsKernelErrorV1::CompilerEnvelope)?;
    let manifest = CompilerModuleSymbolManifestV1::new([
        (
            CompilerModuleSymbolRoleV1::KernelEntry,
            QWEN3_LOGITS_COMPACT_KERNEL_SYMBOL_V1,
        ),
        (
            CompilerModuleSymbolRoleV1::KernelEntry,
            QWEN3_LOGITS_ARGMAX_KERNEL_SYMBOL_V1,
        ),
        (
            CompilerModuleSymbolRoleV1::KernelDescriptor,
            QWEN3_LOGITS_COMPACT_DESCRIPTOR_SYMBOL_V1,
        ),
        (
            CompilerModuleSymbolRoleV1::KernelDescriptor,
            QWEN3_LOGITS_ARGMAX_DESCRIPTOR_SYMBOL_V1,
        ),
    ])
    .map_err(PrepareQwen3LogitsKernelErrorV1::SymbolManifest)?;
    let manifest_identity = manifest.identity();
    let compiler_handoff = CompilerModuleHandoffV2::new(
        CompilerModuleKindV1::LlvmTextIr,
        target,
        CodeObjectVersion::V6,
        envelope,
        manifest,
        llvm.as_bytes(),
    )
    .map_err(PrepareQwen3LogitsKernelErrorV1::CompilerHandoff)?;
    let compiler_handoff_identity = compiler_handoff.identity();
    Ok(PreparedQwen3LogitsKernelV1 {
        catalog,
        source_binding_identity,
        llvm_sha256,
        compiler_handoff_identity,
        manifest_identity,
        compiler_handoff,
    })
}

fn validate_source_bindings(
    bindings: Qwen3LogitsSourceBindingsV1,
) -> Result<(), PrepareQwen3LogitsKernelErrorV1> {
    let identities = [
        bindings.source,
        bindings.kernel_ir,
        bindings.schedule,
        bindings.target_plan,
    ];
    for (index, identity) in identities.iter().enumerate() {
        if identity == &[0; 32] || identities[index + 1..].contains(identity) {
            return Err(PrepareQwen3LogitsKernelErrorV1::SourceBindings);
        }
    }
    Ok(())
}

fn canonical_qwen3_logits_llvm() -> String {
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

",
    );
    emit_argmax_kernel(&mut output);
    output.push('\n');
    emit_compact_kernel(&mut output);
    output.push_str(
        r#"
attributes #0 = { nounwind "amdgpu-flat-work-group-size"="64,64" "target-cpu"="gfx942" "target-features"="-wavefrontsize32,+wavefrontsize64,-xnack" "denormal-fp-math-f32"="ieee,ieee" "unsafe-fp-math"="false" "no-infs-fp-math"="false" "no-nans-fp-math"="false" "no-signed-zeros-fp-math"="false" "approx-func-fp-math"="false" "fp-contract"="off" }
attributes #1 = { nounwind readnone speculatable willreturn }

!0 = !{i32 64, i32 1, i32 1}
!1 = !{!"read_only", !"none", !"write_only", !"none", !"none", !"none"}
!2 = !{!"ushort*", !"ulong", !"uint*", !"ulong", !"uint", !"uint"}
!3 = !{!"const restrict", !"", !"restrict", !"", !"", !""}
!4 = !{!"read_only", !"none", !"read_only", !"none", !"read_only", !"none", !"read_only", !"none", !"read_only", !"none", !"read_only", !"none", !"write_only", !"none", !"none", !"none", !"none"}
!5 = !{!"uint*", !"ulong", !"uint*", !"ulong", !"uint*", !"ulong", !"uint*", !"ulong", !"ulong*", !"ulong", !"uchar*", !"ulong", !"uchar*", !"ulong", !"uint", !"uint", !"uint"}
!6 = !{!"const restrict", !"", !"const restrict", !"", !"const restrict", !"", !"const restrict", !"", !"const restrict", !"", !"const restrict", !"", !"restrict", !"", !"", !"", !""}
"#,
    );
    output
}

fn emit_argmax_kernel(output: &mut String) {
    let symbol = QWEN3_LOGITS_ARGMAX_KERNEL_SYMBOL_V1;
    writeln!(
        output,
        "define amdgpu_kernel void @{symbol}(ptr addrspace(1) noalias nocapture readonly align 2 %logits.data, i64 %logits.len, ptr addrspace(1) noalias nocapture writeonly align 4 %choices.data, i64 %choices.len, i32 %rows, i32 %vocabulary) #0 !reqd_work_group_size !0 !kernel_arg_access_qual !1 !kernel_arg_type !2 !kernel_arg_base_type !2 !kernel_arg_type_qual !3 {{"
    )
    .expect("writing to a String cannot fail");
    output.push_str(
        r"entry:
  %vocabulary.ok = icmp eq i32 %vocabulary, 151936
  %row.1 = icmp eq i32 %rows, 1
  %row.4 = icmp eq i32 %rows, 4
  %row.5 = icmp eq i32 %rows, 5
  %row.8 = icmp eq i32 %rows, 8
  %row.9 = icmp eq i32 %rows, 9
  %row.16 = icmp eq i32 %rows, 16
  %row.17 = icmp eq i32 %rows, 17
  %row.32 = icmp eq i32 %rows, 32
  %row.40 = icmp eq i32 %rows, 40
  %row.128 = icmp eq i32 %rows, 128
  %row.512 = icmp eq i32 %rows, 512
  %row.1024 = icmp eq i32 %rows, 1024
  %row.2048 = icmp eq i32 %rows, 2048
  %rows.4.5 = or i1 %row.4, %row.5
  %rows.8.9 = or i1 %row.8, %row.9
  %rows.16.17 = or i1 %row.16, %row.17
  %rows.32.40 = or i1 %row.32, %row.40
  %rows.128.512 = or i1 %row.128, %row.512
  %rows.1024.2048 = or i1 %row.1024, %row.2048
  %rows.small.0 = or i1 %row.1, %rows.4.5
  %rows.small.1 = or i1 %rows.8.9, %rows.16.17
  %rows.small.2 = or i1 %rows.32.40, %rows.small.0
  %rows.small = or i1 %rows.small.1, %rows.small.2
  %rows.large.0 = or i1 %rows.128.512, %rows.1024.2048
  %known.rows = or i1 %rows.small, %rows.large.0
  %known.profile = and i1 %vocabulary.ok, %known.rows
  %rows64 = zext i32 %rows to i64
  %vocabulary64 = zext i32 %vocabulary to i64
  %logits.expected = mul nuw i64 %rows64, %vocabulary64
  %logits.length.ok = icmp eq i64 %logits.len, %logits.expected
  %choices.length.ok = icmp eq i64 %choices.len, %rows64
  %lengths.ok = and i1 %logits.length.ok, %choices.length.ok
  %shape.ok = and i1 %known.profile, %lengths.ok
  br i1 %shape.ok, label %coordinates, label %trap

coordinates:
  %local = call i32 @llvm.amdgcn.workitem.id.x()
  %row.i32 = call i32 @llvm.amdgcn.workgroup.id.x()
  %lane.zero = icmp eq i32 %local, 0
  %row.ok = icmp ult i32 %row.i32, %rows
  %active = and i1 %lane.zero, %row.ok
  br i1 %active, label %first.load, label %return

first.load:
  %row = zext i32 %row.i32 to i64
  %row.base = mul nuw i64 %row, %vocabulary64
  %first.ptr = getelementptr inbounds i16, ptr addrspace(1) %logits.data, i64 %row.base
  %first.bf16 = load i16, ptr addrspace(1) %first.ptr, align 2
  %first.exp = and i16 %first.bf16, 32640
  %first.finite = icmp ne i16 %first.exp, 32640
  br i1 %first.finite, label %scan, label %trap

scan:
  %first.wide = zext i16 %first.bf16 to i32
  %first.bits = shl nuw i32 %first.wide, 16
  %first.value = bitcast i32 %first.bits to float
  br label %scan.cond

scan.cond:
  %token = phi i32 [ 1, %scan ], [ %token.next, %select ]
  %winner.token = phi i32 [ 0, %scan ], [ %selected.token, %select ]
  %winner.value = phi float [ %first.value, %scan ], [ %selected.value, %select ]
  %more = icmp ult i32 %token, %vocabulary
  br i1 %more, label %scan.load, label %store

scan.load:
  %token64 = zext i32 %token to i64
  %logit.index = add nuw i64 %row.base, %token64
  %logit.ptr = getelementptr inbounds i16, ptr addrspace(1) %logits.data, i64 %logit.index
  %logit.bf16 = load i16, ptr addrspace(1) %logit.ptr, align 2
  %logit.exp = and i16 %logit.bf16, 32640
  %logit.finite = icmp ne i16 %logit.exp, 32640
  br i1 %logit.finite, label %select, label %trap

select:
  %logit.wide = zext i16 %logit.bf16 to i32
  %logit.bits = shl nuw i32 %logit.wide, 16
  %logit.value = bitcast i32 %logit.bits to float
  %strictly.greater = fcmp ogt float %logit.value, %winner.value
  %selected.token = select i1 %strictly.greater, i32 %token, i32 %winner.token
  %selected.value = select i1 %strictly.greater, float %logit.value, float %winner.value
  %token.next = add nuw i32 %token, 1
  br label %scan.cond

store:
  %choice.ptr = getelementptr inbounds i32, ptr addrspace(1) %choices.data, i64 %row
  store i32 %winner.token, ptr addrspace(1) %choice.ptr, align 4
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

fn emit_compact_kernel(output: &mut String) {
    let symbol = QWEN3_LOGITS_COMPACT_KERNEL_SYMBOL_V1;
    writeln!(
        output,
        "define amdgpu_kernel void @{symbol}(ptr addrspace(1) noalias nocapture readonly align 4 %choices.data, i64 %choices.len, ptr addrspace(1) noalias nocapture readonly align 4 %draft.data, i64 %draft.len, ptr addrspace(1) noalias nocapture readonly align 4 %slots.data, i64 %slots.len, ptr addrspace(1) noalias nocapture readonly align 4 %generations.data, i64 %generations.len, ptr addrspace(1) noalias nocapture readonly align 8 %epochs.data, i64 %epochs.len, ptr addrspace(1) noalias nocapture readonly align 1 %plans.data, i64 %plans.len, ptr addrspace(1) noalias nocapture writeonly align 4 %records.data, i64 %records.len, i32 %sequences, i32 %active.tokens, i32 %speculative.k) #0 !reqd_work_group_size !0 !kernel_arg_access_qual !4 !kernel_arg_type !5 !kernel_arg_base_type !5 !kernel_arg_type_qual !6 {{"
    )
    .expect("writing to a String cannot fail");
    output.push_str(
        r"entry:
  %s1 = icmp eq i32 %sequences, 1
  %s8 = icmp eq i32 %sequences, 8
  %s32 = icmp eq i32 %sequences, 32
  %a1 = icmp eq i32 %active.tokens, 1
  %a5 = icmp eq i32 %active.tokens, 5
  %a9 = icmp eq i32 %active.tokens, 9
  %a17 = icmp eq i32 %active.tokens, 17
  %a128 = icmp eq i32 %active.tokens, 128
  %a512 = icmp eq i32 %active.tokens, 512
  %a2048 = icmp eq i32 %active.tokens, 2048
  %k0 = icmp eq i32 %speculative.k, 0
  %k4 = icmp eq i32 %speculative.k, 4
  %k8 = icmp eq i32 %speculative.k, 8
  %k16 = icmp eq i32 %speculative.k, 16
  %direct.s1.a1 = and i1 %s1, %a1
  %direct.s8.a1 = and i1 %s8, %a1
  %direct.s32.a1 = and i1 %s32, %a1
  %direct.s1.a128 = and i1 %s1, %a128
  %direct.s8.a128 = and i1 %s8, %a128
  %direct.s1.a512 = and i1 %s1, %a512
  %direct.s1.a2048 = and i1 %s1, %a2048
  %direct.0 = or i1 %direct.s1.a1, %direct.s8.a1
  %direct.1 = or i1 %direct.s32.a1, %direct.s1.a128
  %direct.2 = or i1 %direct.s8.a128, %direct.s1.a512
  %direct.3 = or i1 %direct.0, %direct.1
  %direct.4 = or i1 %direct.2, %direct.s1.a2048
  %direct.shape = or i1 %direct.3, %direct.4
  %direct.profile = and i1 %k0, %direct.shape
  %spec.s1.k4.shape = and i1 %s1, %a5
  %spec.s8.k4.shape = and i1 %s8, %a5
  %spec.k4.shape = or i1 %spec.s1.k4.shape, %spec.s8.k4.shape
  %spec.k4 = and i1 %k4, %spec.k4.shape
  %spec.s1.k8.shape = and i1 %s1, %a9
  %spec.k8 = and i1 %k8, %spec.s1.k8.shape
  %spec.s1.k16.shape = and i1 %s1, %a17
  %spec.k16 = and i1 %k16, %spec.s1.k16.shape
  %spec.0 = or i1 %spec.k4, %spec.k8
  %spec.profile = or i1 %spec.0, %spec.k16
  %known.profile = or i1 %direct.profile, %spec.profile
  %sequences64 = zext i32 %sequences to i64
  %active64 = zext i32 %active.tokens to i64
  %k64 = zext i32 %speculative.k to i64
  %choices.expected = mul nuw i64 %sequences64, %active64
  %draft.expected = mul nuw i64 %sequences64, %k64
  %plans.expected = mul nuw i64 %sequences64, 32
  %records.expected = mul nuw i64 %sequences64, 120
  %choices.ok = icmp eq i64 %choices.len, %choices.expected
  %draft.ok = icmp eq i64 %draft.len, %draft.expected
  %slots.ok = icmp eq i64 %slots.len, %sequences64
  %generations.ok = icmp eq i64 %generations.len, %sequences64
  %epochs.ok = icmp eq i64 %epochs.len, %sequences64
  %plans.ok = icmp eq i64 %plans.len, %plans.expected
  %records.ok = icmp eq i64 %records.len, %records.expected
  %length.0 = and i1 %choices.ok, %draft.ok
  %length.1 = and i1 %slots.ok, %generations.ok
  %length.2 = and i1 %epochs.ok, %plans.ok
  %length.3 = and i1 %length.0, %length.1
  %length.4 = and i1 %length.2, %records.ok
  %lengths.ok = and i1 %length.3, %length.4
  %entry.ok = and i1 %known.profile, %lengths.ok
  br i1 %entry.ok, label %coordinates, label %trap

coordinates:
  %local = call i32 @llvm.amdgcn.workitem.id.x()
  %sequence.i32 = call i32 @llvm.amdgcn.workgroup.id.x()
  %lane.zero = icmp eq i32 %local, 0
  %sequence.ok = icmp ult i32 %sequence.i32, %sequences
  %active = and i1 %lane.zero, %sequence.ok
  br i1 %active, label %authority.load, label %return

authority.load:
  %sequence = zext i32 %sequence.i32 to i64
  %slot.ptr = getelementptr inbounds i32, ptr addrspace(1) %slots.data, i64 %sequence
  %slot = load i32, ptr addrspace(1) %slot.ptr, align 4
  %slot.ok = icmp ult i32 %slot, 32
  %generation.ptr = getelementptr inbounds i32, ptr addrspace(1) %generations.data, i64 %sequence
  %generation = load i32, ptr addrspace(1) %generation.ptr, align 4
  %generation.ok = icmp ne i32 %generation, 0
  %authority.ok = and i1 %slot.ok, %generation.ok
  br i1 %authority.ok, label %plan.scan, label %trap

plan.scan:
  %plan.base = mul nuw i64 %sequence, 32
  br label %plan.cond

plan.cond:
  %plan.index = phi i64 [ 0, %plan.scan ], [ %plan.next, %plan.body ]
  %plan.present = phi i1 [ false, %plan.scan ], [ %plan.present.next, %plan.body ]
  %plan.more = icmp ult i64 %plan.index, 32
  br i1 %plan.more, label %plan.body, label %plan.done

plan.body:
  %plan.offset = add nuw i64 %plan.base, %plan.index
  %plan.byte.ptr = getelementptr inbounds i8, ptr addrspace(1) %plans.data, i64 %plan.offset
  %plan.byte = load i8, ptr addrspace(1) %plan.byte.ptr, align 1
  %plan.byte.nonzero = icmp ne i8 %plan.byte, 0
  %plan.present.next = or i1 %plan.present, %plan.byte.nonzero
  %plan.next = add nuw i64 %plan.index, 1
  br label %plan.cond

plan.done:
  br i1 %plan.present, label %choice.base, label %trap

choice.base:
  %choice.base.index = mul nuw i64 %sequence, %active64
  %is.direct = icmp eq i32 %speculative.k, 0
  br i1 %is.direct, label %direct.choice, label %accept.cond

direct.choice:
  %active.minus.one = sub nuw i64 %active64, 1
  %direct.choice.index = add nuw i64 %choice.base.index, %active.minus.one
  %direct.choice.ptr = getelementptr inbounds i32, ptr addrspace(1) %choices.data, i64 %direct.choice.index
  %direct.choice.token = load i32, ptr addrspace(1) %direct.choice.ptr, align 4
  %direct.choice.ok = icmp ult i32 %direct.choice.token, 151936
  br i1 %direct.choice.ok, label %record.start.direct, label %trap

accept.cond:
  %accepted = phi i32 [ 0, %choice.base ], [ %accepted.next, %accept.equal ]
  %accepted.less.k = icmp ult i32 %accepted, %speculative.k
  br i1 %accepted.less.k, label %accept.load, label %correction

accept.load:
  %accepted64 = zext i32 %accepted to i64
  %draft.base = mul nuw i64 %sequence, %k64
  %draft.index = add nuw i64 %draft.base, %accepted64
  %draft.ptr = getelementptr inbounds i32, ptr addrspace(1) %draft.data, i64 %draft.index
  %draft.token = load i32, ptr addrspace(1) %draft.ptr, align 4
  %target.index = add nuw i64 %choice.base.index, %accepted64
  %target.ptr = getelementptr inbounds i32, ptr addrspace(1) %choices.data, i64 %target.index
  %target.token = load i32, ptr addrspace(1) %target.ptr, align 4
  %draft.in.vocabulary = icmp ult i32 %draft.token, 151936
  %target.in.vocabulary = icmp ult i32 %target.token, 151936
  %tokens.in.vocabulary = and i1 %draft.in.vocabulary, %target.in.vocabulary
  %tokens.equal = icmp eq i32 %draft.token, %target.token
  br i1 %tokens.in.vocabulary, label %accept.compare, label %trap

accept.compare:
  br i1 %tokens.equal, label %accept.equal, label %correction

accept.equal:
  %accepted.next = add nuw i32 %accepted, 1
  br label %accept.cond

correction:
  %accepted.final = phi i32 [ %accepted, %accept.cond ], [ %accepted, %accept.compare ]
  %accepted.final64 = zext i32 %accepted.final to i64
  %correction.index = add nuw i64 %choice.base.index, %accepted.final64
  %correction.ptr = getelementptr inbounds i32, ptr addrspace(1) %choices.data, i64 %correction.index
  %correction.token = load i32, ptr addrspace(1) %correction.ptr, align 4
  %correction.ok = icmp ult i32 %correction.token, 151936
  br i1 %correction.ok, label %record.start.speculative, label %trap

record.start.direct:
  br label %record.start

record.start.speculative:
  br label %record.start

record.start:
  %record.accepted = phi i32 [ 0, %record.start.direct ], [ %accepted.final, %record.start.speculative ]
  %record.correction = phi i32 [ %direct.choice.token, %record.start.direct ], [ %correction.token, %record.start.speculative ]
  %record.base = mul nuw i64 %sequence, 120
  %record.slot.ptr = getelementptr inbounds i8, ptr addrspace(1) %records.data, i64 %record.base
  store i32 %slot, ptr addrspace(1) %record.slot.ptr, align 4
  %record.generation.offset = add nuw i64 %record.base, 4
  %record.generation.ptr = getelementptr inbounds i8, ptr addrspace(1) %records.data, i64 %record.generation.offset
  store i32 %generation, ptr addrspace(1) %record.generation.ptr, align 4
  %epoch.ptr = getelementptr inbounds i64, ptr addrspace(1) %epochs.data, i64 %sequence
  %epoch = load i64, ptr addrspace(1) %epoch.ptr, align 8
  %record.epoch.offset = add nuw i64 %record.base, 8
  %record.epoch.ptr = getelementptr inbounds i8, ptr addrspace(1) %records.data, i64 %record.epoch.offset
  store i64 %epoch, ptr addrspace(1) %record.epoch.ptr, align 8
  br label %plan.copy.cond

plan.copy.cond:
  %copy.index = phi i64 [ 0, %record.start ], [ %copy.next, %plan.copy.body ]
  %copy.more = icmp ult i64 %copy.index, 32
  br i1 %copy.more, label %plan.copy.body, label %header.finish

plan.copy.body:
  %copy.plan.offset = add nuw i64 %plan.base, %copy.index
  %copy.plan.ptr = getelementptr inbounds i8, ptr addrspace(1) %plans.data, i64 %copy.plan.offset
  %copy.byte = load i8, ptr addrspace(1) %copy.plan.ptr, align 1
  %record.plan.base = add nuw i64 %record.base, 16
  %copy.record.offset = add nuw i64 %record.plan.base, %copy.index
  %copy.record.ptr = getelementptr inbounds i8, ptr addrspace(1) %records.data, i64 %copy.record.offset
  store i8 %copy.byte, ptr addrspace(1) %copy.record.ptr, align 1
  %copy.next = add nuw i64 %copy.index, 1
  br label %plan.copy.cond

header.finish:
  %accepted.i8 = trunc i32 %record.accepted to i8
  %record.accepted.offset = add nuw i64 %record.base, 48
  %record.accepted.ptr = getelementptr inbounds i8, ptr addrspace(1) %records.data, i64 %record.accepted.offset
  store i8 %accepted.i8, ptr addrspace(1) %record.accepted.ptr, align 1
  %emitted.count = add nuw i32 %record.accepted, 1
  %emitted.i8 = trunc i32 %emitted.count to i8
  %record.emitted.offset = add nuw i64 %record.base, 49
  %record.emitted.ptr = getelementptr inbounds i8, ptr addrspace(1) %records.data, i64 %record.emitted.offset
  store i8 %emitted.i8, ptr addrspace(1) %record.emitted.ptr, align 1
  %record.reserved.offset = add nuw i64 %record.base, 50
  %record.reserved.ptr = getelementptr inbounds i8, ptr addrspace(1) %records.data, i64 %record.reserved.offset
  store i16 0, ptr addrspace(1) %record.reserved.ptr, align 2
  br label %tokens.zero.cond

tokens.zero.cond:
  %zero.index = phi i64 [ 0, %header.finish ], [ %zero.next, %tokens.zero.body ]
  %zero.more = icmp ult i64 %zero.index, 17
  br i1 %zero.more, label %tokens.zero.body, label %tokens.emit.cond

tokens.zero.body:
  %zero.byte.offset = mul nuw i64 %zero.index, 4
  %record.tokens.base = add nuw i64 %record.base, 52
  %zero.record.offset = add nuw i64 %record.tokens.base, %zero.byte.offset
  %zero.record.ptr = getelementptr inbounds i8, ptr addrspace(1) %records.data, i64 %zero.record.offset
  store i32 0, ptr addrspace(1) %zero.record.ptr, align 4
  %zero.next = add nuw i64 %zero.index, 1
  br label %tokens.zero.cond

tokens.emit.cond:
  %emit.index = phi i32 [ 0, %tokens.zero.cond ], [ %emit.next, %tokens.emit.body ]
  %emit.more = icmp ult i32 %emit.index, %record.accepted
  br i1 %emit.more, label %tokens.emit.body, label %correction.store

tokens.emit.body:
  %emit.index64 = zext i32 %emit.index to i64
  %emit.draft.base = mul nuw i64 %sequence, %k64
  %emit.draft.index = add nuw i64 %emit.draft.base, %emit.index64
  %emit.draft.ptr = getelementptr inbounds i32, ptr addrspace(1) %draft.data, i64 %emit.draft.index
  %emit.draft.token = load i32, ptr addrspace(1) %emit.draft.ptr, align 4
  %emit.byte.offset = mul nuw i64 %emit.index64, 4
  %emit.tokens.base = add nuw i64 %record.base, 52
  %emit.record.offset = add nuw i64 %emit.tokens.base, %emit.byte.offset
  %emit.record.ptr = getelementptr inbounds i8, ptr addrspace(1) %records.data, i64 %emit.record.offset
  store i32 %emit.draft.token, ptr addrspace(1) %emit.record.ptr, align 4
  %emit.next = add nuw i32 %emit.index, 1
  br label %tokens.emit.cond

correction.store:
  %accepted.store64 = zext i32 %record.accepted to i64
  %correction.byte.offset = mul nuw i64 %accepted.store64, 4
  %correction.tokens.base = add nuw i64 %record.base, 52
  %correction.record.offset = add nuw i64 %correction.tokens.base, %correction.byte.offset
  %correction.record.ptr = getelementptr inbounds i8, ptr addrspace(1) %records.data, i64 %correction.record.offset
  store i32 %record.correction, ptr addrspace(1) %correction.record.ptr, align 4
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

fn validate_canonical_llvm(module: &str) -> Result<(), PrepareQwen3LogitsKernelErrorV1> {
    let hash: [u8; 32] = Sha256::digest(module.as_bytes()).into();
    let argmax_symbol = QWEN3_LOGITS_ARGMAX_KERNEL_SYMBOL_V1;
    let compact_symbol = QWEN3_LOGITS_COMPACT_KERNEL_SYMBOL_V1;
    let exact = module.len() == QWEN3_LOGITS_LLVM_BYTES_V1
        && hash == QWEN3_LOGITS_LLVM_SHA256_V1
        && module
            .matches(&format!("define amdgpu_kernel void @{argmax_symbol}"))
            .count()
            == 1
        && module
            .matches(&format!("define amdgpu_kernel void @{compact_symbol}"))
            .count()
            == 1
        && module.contains("%strictly.greater = fcmp ogt float")
        && !module.contains("fcmp oge")
        && module
            .contains("%direct.choice.index = add nuw i64 %choice.base.index, %active.minus.one")
        && module.contains("%accepted.less.k = icmp ult i32 %accepted, %speculative.k")
        && module.contains("store i16 0, ptr addrspace(1) %record.reserved.ptr")
        && module.contains("%records.expected = mul nuw i64 %sequences64, 120")
        && module.contains("%vocabulary.ok = icmp eq i32 %vocabulary, 151936")
        && !module.contains("atomic")
        && !module.contains("volatile")
        && !module.contains("llvm.fma")
        && !module.contains(" fmul contract ")
        && !module.contains(" fadd contract ")
        && !module.contains("fast ")
        && !module.contains("fma")
        && !module.contains("mfma")
        && !module.contains("comgr")
        && !module.contains("COMGR");
    if !exact {
        return Err(PrepareQwen3LogitsKernelErrorV1::CompilerModule);
    }
    Ok(())
}

/// Linear exact compiler handoff awaiting Worker V2 execution.
pub struct InertQwen3LogitsWorkerRequestV1 {
    prepared: PreparedQwen3LogitsKernelV1,
}

impl fmt::Debug for InertQwen3LogitsWorkerRequestV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InertQwen3LogitsWorkerRequestV1")
            .field("catalog", &self.prepared.catalog.identity)
            .field("source_binding", &self.prepared.source_binding_identity)
            .field("handoff", &self.prepared.compiler_handoff_identity)
            .finish_non_exhaustive()
    }
}

impl InertQwen3LogitsWorkerRequestV1 {
    /// Complete finite catalog retained by the request.
    #[must_use]
    pub const fn catalog(&self) -> &Qwen3LogitsProfileCatalogV1 {
        &self.prepared.catalog
    }

    /// Exact compiler handoff for transaction publication.
    #[must_use]
    pub const fn compiler_handoff(&self) -> &CompilerModuleHandoffV2 {
        &self.prepared.compiler_handoff
    }

    /// Ferric-domain source binding.
    #[must_use]
    pub const fn source_binding_identity(&self) -> &[u8; 32] {
        &self.prepared.source_binding_identity
    }

    /// A request does not establish Worker execution.
    #[must_use]
    pub const fn authenticates_worker_execution(&self) -> bool {
        false
    }

    /// A request grants no artifact, publication, or launch authority.
    #[must_use]
    pub const fn grants_launch_authority(&self) -> bool {
        false
    }
}

/// Consumes a prepared owner into the Worker V2 request stage.
#[must_use]
pub const fn lower_qwen3_logits_kernel_v1(
    prepared: PreparedQwen3LogitsKernelV1,
) -> InertQwen3LogitsWorkerRequestV1 {
    InertQwen3LogitsWorkerRequestV1 { prepared }
}

/// Failure while executing source through Worker V2.
#[derive(Debug)]
pub enum ExecuteQwen3LogitsWorkerErrorV1 {
    /// Consumed transaction bytes differed.
    HandoffSubstitution,
    /// A fixed link option could not be represented.
    FixedLinkOption,
    /// The fixed HSACO ceiling could not be represented.
    OutputConstraint(WorkerProtocolError),
    /// Reproducible bootstrap and replay failed.
    FirstBuild(FirstBuildWorkerV2Error),
}

impl fmt::Display for ExecuteQwen3LogitsWorkerErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Qwen3 logits Worker V2 execution failed: {self:?}"
        )
    }
}

impl std::error::Error for ExecuteQwen3LogitsWorkerErrorV1 {}

/// Linear Worker bootstrap/replay evidence awaiting inspection.
pub struct InertQwen3LogitsWorkerEvidenceV1 {
    prepared: PreparedQwen3LogitsKernelV1,
    transaction_handoff: CompilerModuleHandoffIdentityV1,
    worker: InertFirstBuildWorkerV2EvidenceV1,
}

impl fmt::Debug for InertQwen3LogitsWorkerEvidenceV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InertQwen3LogitsWorkerEvidenceV1")
            .field("source_binding", &self.prepared.source_binding_identity)
            .field("transaction_handoff", &self.transaction_handoff)
            .field("worker", &self.worker.identity())
            .finish_non_exhaustive()
    }
}

impl InertQwen3LogitsWorkerEvidenceV1 {
    /// Evidence remains inert until strict inspection.
    #[must_use]
    pub const fn grants_artifact_authority(&self) -> bool {
        false
    }

    /// Worker output does not prove numerical behavior.
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

/// Executes the exact transaction through Worker V2 bootstrap/replay.
///
/// # Errors
///
/// Returns an error if the consumed handoff is substituted, a fixed Worker
/// option cannot be represented, or reproducible Worker execution fails.
pub fn execute_qwen3_logits_worker_v2_v1(
    request: InertQwen3LogitsWorkerRequestV1,
    consumed: ConsumedCompilerModuleHandoffV1,
    worker: &PinnedWorkerV1,
    limits: WorkerExecutionLimitsV1,
) -> Result<InertQwen3LogitsWorkerEvidenceV1, ExecuteQwen3LogitsWorkerErrorV1> {
    let InertQwen3LogitsWorkerRequestV1 { prepared } = request;
    if consumed.bytes() != prepared.compiler_handoff.canonical_bytes() {
        return Err(ExecuteQwen3LogitsWorkerErrorV1::HandoffSubstitution);
    }
    let transaction_handoff = consumed.identity();
    let worker_evidence = execute_reproducible_first_build_worker_v2(
        consumed,
        worker,
        Vec::new(),
        fixed_link_options()?,
        WorkerOutputConstraintsV1::new(MAX_HSACO_BYTES as u64)
            .map_err(ExecuteQwen3LogitsWorkerErrorV1::OutputConstraint)?,
        limits,
    )
    .map_err(ExecuteQwen3LogitsWorkerErrorV1::FirstBuild)?;
    Ok(InertQwen3LogitsWorkerEvidenceV1 {
        prepared,
        transaction_handoff,
        worker: worker_evidence,
    })
}

/// Exact post-worker structural rejection.
#[derive(Debug)]
pub enum InspectQwen3LogitsKernelErrorV1 {
    /// Worker request or response failed decoding.
    Protocol(WorkerProtocolError),
    /// Compiler, transaction, Worker, or output lineage drifted.
    SourceLineage,
    /// AMDHSA metadata or descriptor binding failed.
    Hsaco(KernelBindingError),
    /// Kernel inventory, ABI, or resources differed.
    KernelProfile,
    /// Strict COV6 loader validation failed.
    Loader(PlanError),
}

impl fmt::Display for InspectQwen3LogitsKernelErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Qwen3 logits structural inspection failed: {self:?}"
        )
    }
}

impl std::error::Error for InspectQwen3LogitsKernelErrorV1 {}

/// Linear Worker output after exact ABI/resource and loader inspection.
pub struct InspectedQwen3LogitsKernelV1 {
    catalog: Qwen3LogitsProfileCatalogV1,
    source_binding_identity: [u8; 32],
    compiler_handoff_identity: CompilerModuleHandoffIdentityV2,
    transaction_handoff: CompilerModuleHandoffIdentityV1,
    loader_plan: LoadPlan,
    worker: InertFirstBuildWorkerV2EvidenceV1,
}

impl fmt::Debug for InspectedQwen3LogitsKernelV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InspectedQwen3LogitsKernelV1")
            .field("catalog", &self.catalog.identity)
            .field("source_binding", &self.source_binding_identity)
            .field("compiler_handoff", &self.compiler_handoff_identity)
            .field("transaction_handoff", &self.transaction_handoff)
            .field("worker", &self.worker.identity())
            .finish_non_exhaustive()
    }
}

impl InspectedQwen3LogitsKernelV1 {
    /// Complete finite catalog.
    #[must_use]
    pub const fn catalog(&self) -> &Qwen3LogitsProfileCatalogV1 {
        &self.catalog
    }

    /// Exact strict loader plan over the same bytes.
    #[must_use]
    pub const fn loader_plan(&self) -> &LoadPlan {
        &self.loader_plan
    }

    /// Exact bytes retained by sealed Worker evidence.
    #[must_use]
    pub fn exact_worker_output_bytes(&self) -> &[u8] {
        self.worker.output_bytes()
    }

    /// Observed bytes are not an independent deployment pin.
    #[must_use]
    pub const fn has_independent_deployment_pin(&self) -> bool {
        false
    }

    /// Inspection does not prove source-to-machine refinement.
    #[must_use]
    pub const fn proves_machine_refinement(&self) -> bool {
        false
    }

    /// Inspection does not prove numerical or operator refinement.
    #[must_use]
    pub const fn proves_operator_or_numerical_refinement(&self) -> bool {
        false
    }

    /// Inspection does not prove race refinement.
    #[must_use]
    pub const fn proves_race_refinement(&self) -> bool {
        false
    }

    /// Inspection does not authenticate logits, tokens, or authority content.
    #[must_use]
    pub const fn authenticates_content(&self) -> bool {
        false
    }

    /// Inspection does not authenticate allocation ownership.
    #[must_use]
    pub const fn authenticates_allocation_ownership(&self) -> bool {
        false
    }

    /// Inspection does not authenticate generation values.
    #[must_use]
    pub const fn authenticates_generation(&self) -> bool {
        false
    }

    /// Inspection does not prove hardware execution.
    #[must_use]
    pub const fn proves_hardware_execution(&self) -> bool {
        false
    }

    /// Inspection does not prove completion or publication.
    #[must_use]
    pub const fn proves_completion(&self) -> bool {
        false
    }

    /// Inspection does not prove performance.
    #[must_use]
    pub const fn proves_performance(&self) -> bool {
        false
    }

    /// Inspection grants no load, launch, or publication authority.
    #[must_use]
    pub const fn grants_launch_authority(&self) -> bool {
        false
    }

    /// Binds one profile to exact checked slices.
    ///
    /// # Errors
    ///
    /// Returns an error if the bucket is absent or any address, length,
    /// alignment, alias, compact-mode, or argmax-to-compact join check fails.
    pub fn bind_checked_profile(
        &self,
        bucket: Qwen3LogitsBucketV1,
        argmax_addresses: [u64; 2],
        argmax_lengths: [u64; 2],
        compact: Option<([u64; 7], [u64; 7])>,
    ) -> Result<CheckedQwen3LogitsLaunchV1, BindQwen3LogitsLaunchErrorV1> {
        let profile = self
            .catalog
            .profile(bucket)
            .ok_or(BindQwen3LogitsLaunchErrorV1::Profile)?;
        let buffers = Qwen3LogitsBufferContractV1::checked(
            profile,
            argmax_addresses,
            argmax_lengths,
            compact,
        )
        .map_err(BindQwen3LogitsLaunchErrorV1::Buffers)?;
        Ok(CheckedQwen3LogitsLaunchV1 { profile, buffers })
    }
}

/// Consumes Worker evidence through transcript, ABI, resource, and loader checks.
///
/// # Errors
///
/// Returns an error if Worker lineage, output identity, HSACO structure,
/// kernel ABI/resources, or the strict loader profile differs from the exact
/// Ferric boundary.
pub fn inspect_qwen3_logits_kernel_v1(
    evidence: InertQwen3LogitsWorkerEvidenceV1,
) -> Result<InspectedQwen3LogitsKernelV1, InspectQwen3LogitsKernelErrorV1> {
    let InertQwen3LogitsWorkerEvidenceV1 {
        prepared,
        transaction_handoff,
        worker,
    } = evidence;
    validate_worker_lineage(&prepared, transaction_handoff, &worker)?;
    let bytes = worker.output_bytes();
    if !worker.output_identity().matches(bytes) {
        return Err(InspectQwen3LogitsKernelErrorV1::SourceLineage);
    }
    let bound = inspect_and_bind_kernel_descriptors(bytes)
        .map_err(InspectQwen3LogitsKernelErrorV1::Hsaco)?;
    let [argmax, compact] = bound.inspection().kernels() else {
        return Err(InspectQwen3LogitsKernelErrorV1::KernelProfile);
    };
    let [argmax_binding, compact_binding] = bound.bindings() else {
        return Err(InspectQwen3LogitsKernelErrorV1::KernelProfile);
    };
    let exact = bound.inspection().code_object_version() == InspectedCodeObjectVersion::V6
        && bound.inspection().target().to_string() == QWEN3_LOGITS_TARGET_V1
        && !bound.inspection().has_printf_metadata()
        && exact_kernel_profile(
            argmax,
            argmax_binding,
            ExactKernelProfileV1 {
                index: 0,
                symbol: QWEN3_LOGITS_ARGMAX_KERNEL_SYMBOL_V1,
                descriptor: QWEN3_LOGITS_ARGMAX_DESCRIPTOR_SYMBOL_V1,
                explicit_bytes: QWEN3_LOGITS_ARGMAX_EXPLICIT_KERNARG_BYTES_V1,
                total_bytes: QWEN3_LOGITS_ARGMAX_TOTAL_KERNARG_BYTES_V1,
                explicit_arguments: exact_argmax_explicit_arguments,
            },
        )
        && exact_kernel_profile(
            compact,
            compact_binding,
            ExactKernelProfileV1 {
                index: 1,
                symbol: QWEN3_LOGITS_COMPACT_KERNEL_SYMBOL_V1,
                descriptor: QWEN3_LOGITS_COMPACT_DESCRIPTOR_SYMBOL_V1,
                explicit_bytes: QWEN3_LOGITS_COMPACT_EXPLICIT_KERNARG_BYTES_V1,
                total_bytes: QWEN3_LOGITS_COMPACT_TOTAL_KERNARG_BYTES_V1,
                explicit_arguments: exact_compact_explicit_arguments,
            },
        );
    if !exact {
        return Err(InspectQwen3LogitsKernelErrorV1::KernelProfile);
    }
    let loader = fe2o3_amdhsa_loader::validate(bytes, AdmittedProfile::Gfx942XnackOffCov6)
        .map_err(InspectQwen3LogitsKernelErrorV1::Loader)?;
    let loader_plan = *loader.plan();
    Ok(InspectedQwen3LogitsKernelV1 {
        catalog: prepared.catalog,
        source_binding_identity: prepared.source_binding_identity,
        compiler_handoff_identity: prepared.compiler_handoff_identity,
        transaction_handoff,
        loader_plan,
        worker,
    })
}

fn validate_worker_lineage(
    prepared: &PreparedQwen3LogitsKernelV1,
    transaction_handoff: CompilerModuleHandoffIdentityV1,
    worker: &InertFirstBuildWorkerV2EvidenceV1,
) -> Result<(), InspectQwen3LogitsKernelErrorV1> {
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
        return Err(InspectQwen3LogitsKernelErrorV1::SourceLineage);
    }
    let bootstrap = InertDecodedWorkerExchangeV2::decode(
        worker.bootstrap_request_bytes(),
        worker.bootstrap().response().canonical_bytes(),
    )
    .map_err(InspectQwen3LogitsKernelErrorV1::Protocol)?;
    let replay = InertDecodedWorkerExchangeV2::decode(
        worker.authorized_request_bytes(),
        worker.authorized().response().canonical_bytes(),
    )
    .map_err(InspectQwen3LogitsKernelErrorV1::Protocol)?;
    for exchange in [&bootstrap, &replay] {
        let request = exchange.request();
        if request.target() != exact_target()
            || request.code_object_version() != CodeObjectVersion::V6
            || request.compiler_module().bytes() != prepared.compiler_handoff.module_bytes()
            || !request.external_providers().is_empty()
            || !request.import_symbols().is_empty()
            || !request.export_symbols().is_empty()
            || !request.final_symbols().iter().map(String::as_str).eq([
                QWEN3_LOGITS_COMPACT_KERNEL_SYMBOL_V1,
                QWEN3_LOGITS_ARGMAX_KERNEL_SYMBOL_V1,
                QWEN3_LOGITS_COMPACT_DESCRIPTOR_SYMBOL_V1,
                QWEN3_LOGITS_ARGMAX_DESCRIPTOR_SYMBOL_V1,
            ])
            || exchange.response().request_identity() != request.identity()
            || exchange.response().device_library_provider().is_some()
        {
            return Err(InspectQwen3LogitsKernelErrorV1::SourceLineage);
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
        && kernel.kernarg_segment_alignment() == QWEN3_LOGITS_KERNARG_ALIGNMENT_V1
        && kernel.implicit_argument_offset() == Some(profile.explicit_bytes)
        && kernel.implicit_argument_size() == 256
        && kernel.required_workgroup_size() == Some(QWEN3_LOGITS_WORKGROUP_V1)
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

fn exact_argmax_explicit_arguments(arguments: &[ExplicitArgument]) -> bool {
    arguments.len() == 6
        && exact_pointer_argument(
            &arguments[0],
            "logits.data",
            0,
            2,
            ArgumentAccess::ReadOnly,
            is_u16_metadata_carrier,
        )
        && exact_integer_argument(&arguments[1], "logits.len", 8, 8, is_u64_metadata_carrier)
        && exact_pointer_argument(
            &arguments[2],
            "choices.data",
            16,
            4,
            ArgumentAccess::WriteOnly,
            is_u32_metadata_carrier,
        )
        && exact_integer_argument(&arguments[3], "choices.len", 24, 8, is_u64_metadata_carrier)
        && exact_integer_argument(&arguments[4], "rows", 32, 4, is_u32_metadata_carrier)
        && exact_integer_argument(&arguments[5], "vocabulary", 36, 4, is_u32_metadata_carrier)
}

fn exact_compact_explicit_arguments(arguments: &[ExplicitArgument]) -> bool {
    if arguments.len() != 17 {
        return false;
    }
    let pointers = [
        (
            0,
            "choices.data",
            0,
            4,
            ArgumentAccess::ReadOnly,
            is_u32_metadata_carrier as fn(_) -> _,
        ),
        (
            2,
            "draft.data",
            16,
            4,
            ArgumentAccess::ReadOnly,
            is_u32_metadata_carrier,
        ),
        (
            4,
            "slots.data",
            32,
            4,
            ArgumentAccess::ReadOnly,
            is_u32_metadata_carrier,
        ),
        (
            6,
            "generations.data",
            48,
            4,
            ArgumentAccess::ReadOnly,
            is_u32_metadata_carrier,
        ),
        (
            8,
            "epochs.data",
            64,
            8,
            ArgumentAccess::ReadOnly,
            is_u64_metadata_carrier,
        ),
        (
            10,
            "plans.data",
            80,
            1,
            ArgumentAccess::ReadOnly,
            is_u8_metadata_carrier,
        ),
        (
            12,
            "records.data",
            96,
            4,
            ArgumentAccess::WriteOnly,
            is_u8_metadata_carrier,
        ),
    ];
    for (index, name, offset, alignment, access, accepted) in pointers {
        if !exact_pointer_argument(&arguments[index], name, offset, alignment, access, accepted) {
            return false;
        }
        if !exact_integer_argument(
            &arguments[index + 1],
            match index {
                0 => "choices.len",
                2 => "draft.len",
                4 => "slots.len",
                6 => "generations.len",
                8 => "epochs.len",
                10 => "plans.len",
                12 => "records.len",
                _ => return false,
            },
            offset + 8,
            8,
            is_u64_metadata_carrier,
        ) {
            return false;
        }
    }
    exact_integer_argument(&arguments[14], "sequences", 112, 4, is_u32_metadata_carrier)
        && exact_integer_argument(
            &arguments[15],
            "active.tokens",
            116,
            4,
            is_u32_metadata_carrier,
        )
        && exact_integer_argument(
            &arguments[16],
            "speculative.k",
            120,
            4,
            is_u32_metadata_carrier,
        )
}

fn exact_pointer_argument(
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

const fn is_u8_metadata_carrier(value_type: ExplicitValueType) -> bool {
    matches!(value_type, ExplicitValueType::I8 | ExplicitValueType::U8)
}

const fn is_u16_metadata_carrier(value_type: ExplicitValueType) -> bool {
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

fn fixed_link_options() -> Result<Vec<LinkOptionV1>, ExecuteQwen3LogitsWorkerErrorV1> {
    [
        ("code-object-version", "6"),
        ("opt-level", "2"),
        ("strip-debug", "true"),
        ("verify-each", "true"),
    ]
    .into_iter()
    .map(|(name, value)| {
        LinkOptionV1::new(name, value).map_err(|_| ExecuteQwen3LogitsWorkerErrorV1::FixedLinkOption)
    })
    .collect()
}

/// Failure while binding inspected output to a runtime profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindQwen3LogitsLaunchErrorV1 {
    /// The requested role/bucket was absent.
    Profile,
    /// Buffer validation failed.
    Buffers(Qwen3LogitsBufferContractErrorV1),
}

impl fmt::Display for BindQwen3LogitsLaunchErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Qwen3 logits launch binding failed: {self:?}")
    }
}

impl std::error::Error for BindQwen3LogitsLaunchErrorV1 {}

/// Exact inert runtime binding retained for a future protected launcher.
#[derive(Debug)]
pub struct CheckedQwen3LogitsLaunchV1 {
    profile: Qwen3LogitsProfileV1,
    buffers: Qwen3LogitsBufferContractV1,
}

impl CheckedQwen3LogitsLaunchV1 {
    /// Exact finite profile.
    #[must_use]
    pub const fn profile(&self) -> Qwen3LogitsProfileV1 {
        self.profile
    }

    /// Exact checked buffer ranges.
    #[must_use]
    pub const fn buffers(&self) -> &Qwen3LogitsBufferContractV1 {
        &self.buffers
    }

    /// Exact argmax kernel symbol.
    #[must_use]
    pub const fn argmax_kernel_symbol(&self) -> &'static str {
        QWEN3_LOGITS_ARGMAX_KERNEL_SYMBOL_V1
    }

    /// Target-only compact symbol.
    #[must_use]
    pub const fn compact_kernel_symbol(&self) -> Option<&'static str> {
        match self.profile.completion() {
            Qwen3LogitsCompletionKindV1::DraftChoices => None,
            Qwen3LogitsCompletionKindV1::TargetDirect
            | Qwen3LogitsCompletionKindV1::TargetSpeculative => {
                Some(QWEN3_LOGITS_COMPACT_KERNEL_SYMBOL_V1)
            }
        }
    }

    /// This binding grants no allocation, publication, load, or launch authority.
    #[must_use]
    pub const fn grants_launch_authority(&self) -> bool {
        false
    }
}

fn exact_target() -> DeviceTargetV1 {
    DeviceTargetV1::parse(QWEN3_LOGITS_TARGET_V1)
        .expect("the fixed Ferric Qwen3 logits target is canonical")
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

    fn bindings() -> Qwen3LogitsSourceBindingsV1 {
        Qwen3LogitsSourceBindingsV1::new([1; 32], [2; 32], [3; 32], [4; 32])
    }

    fn profile(
        role: Qwen3LogitsModelRoleV1,
        kind: Qwen3LogitsBucketKindV1,
    ) -> Qwen3LogitsProfileV1 {
        Qwen3LogitsProfileV1::canonical(Qwen3LogitsBucketV1::new(role, kind)).unwrap()
    }

    fn exact_lengths(profile: Qwen3LogitsProfileV1) -> ([u64; 2], Option<[u64; 7]>) {
        let [logits, choices, draft, records] = profile.storage_extents();
        let argmax = [logits * 2, choices * 4];
        let sequences = u64::from(profile.choice_shape()[0]);
        let compact = if profile.completion() == Qwen3LogitsCompletionKindV1::DraftChoices {
            None
        } else {
            Some([
                choices * 4,
                draft * 4,
                sequences * 4,
                sequences * 4,
                sequences * 8,
                sequences * 32,
                records,
            ])
        };
        (argmax, compact)
    }

    fn compact_addresses(choice: u64) -> [u64; 7] {
        [
            choice,
            0x2000_0000,
            0x3000_0000,
            0x4000_0000,
            0x5000_0000,
            0x6000_0000,
            0x7000_0000,
        ]
    }

    #[test]
    fn exact_22_profile_catalog_is_complete_unique_and_role_ordered() {
        let catalog = Qwen3LogitsProfileCatalogV1::canonical().unwrap();
        assert_eq!(catalog.profiles().len(), 22);
        let identities: BTreeSet<_> = catalog
            .profiles()
            .iter()
            .map(|profile| profile.identity())
            .collect();
        assert_eq!(identities.len(), 22);
        assert_eq!(
            catalog.profiles()[0].bucket(),
            Qwen3LogitsBucketV1::new(
                Qwen3LogitsModelRoleV1::Target8B,
                Qwen3LogitsBucketKindV1::PrefillS1T128,
            )
        );
        assert_eq!(
            catalog.profiles()[21].bucket(),
            Qwen3LogitsBucketV1::new(
                Qwen3LogitsModelRoleV1::Draft06B,
                Qwen3LogitsBucketKindV1::SpeculativeS1K16C8192,
            )
        );
        assert!(catalog.identity().as_bytes().iter().any(|byte| *byte != 0));
    }

    #[test]
    fn every_bucket_shape_and_target_draft_speculative_width_is_exact() {
        let expected = [
            (Qwen3LogitsBucketKindV1::PrefillS1T128, 1, 128, 0),
            (Qwen3LogitsBucketKindV1::PrefillS8T128, 8, 128, 0),
            (Qwen3LogitsBucketKindV1::PrefillS1T512, 1, 512, 0),
            (Qwen3LogitsBucketKindV1::PrefillS1T2048, 1, 2_048, 0),
            (Qwen3LogitsBucketKindV1::DecodeS1C8192, 1, 1, 0),
            (Qwen3LogitsBucketKindV1::DecodeS8C8192, 8, 1, 0),
            (Qwen3LogitsBucketKindV1::DecodeS32C8192, 32, 1, 0),
        ];
        for (kind, sequences, active, k) in expected {
            for role in QWEN3_LOGITS_ROLES_V1 {
                let profile = profile(role, kind);
                assert_eq!(profile.choice_shape(), [sequences, active]);
                assert_eq!(profile.logits_shape(), [sequences, active, 151_936]);
                assert_eq!(profile.speculative_k(), k);
            }
        }
        for (kind, sequences, k) in [
            (Qwen3LogitsBucketKindV1::SpeculativeS1K4C8192, 1, 4),
            (Qwen3LogitsBucketKindV1::SpeculativeS8K4C8192, 8, 4),
            (Qwen3LogitsBucketKindV1::SpeculativeS1K8C8192, 1, 8),
            (Qwen3LogitsBucketKindV1::SpeculativeS1K16C8192, 1, 16),
        ] {
            assert_eq!(
                profile(Qwen3LogitsModelRoleV1::Target8B, kind).choice_shape(),
                [sequences, k + 1]
            );
            assert_eq!(
                profile(Qwen3LogitsModelRoleV1::Draft06B, kind).choice_shape(),
                [sequences, k]
            );
        }
    }

    #[test]
    fn compact_publication_is_target_only_and_direct_uses_final_active_row() {
        let catalog = Qwen3LogitsProfileCatalogV1::canonical().unwrap();
        let mut draft = 0;
        let mut direct = 0;
        let mut speculative = 0;
        for profile in catalog.profiles() {
            match profile.completion() {
                Qwen3LogitsCompletionKindV1::DraftChoices => {
                    draft += 1;
                    assert!(profile.compact_grid_workitems().is_none());
                    assert_eq!(profile.storage_extents()[3], 0);
                }
                Qwen3LogitsCompletionKindV1::TargetDirect => {
                    direct += 1;
                    assert_eq!(profile.speculative_k(), 0);
                    assert!(profile.compact_grid_workitems().is_some());
                }
                Qwen3LogitsCompletionKindV1::TargetSpeculative => {
                    speculative += 1;
                    assert_eq!(profile.choice_shape()[1], profile.speculative_k() + 1);
                    assert!(profile.compact_grid_workitems().is_some());
                }
            }
        }
        assert_eq!((draft, direct, speculative), (11, 7, 4));
        let source = canonical_qwen3_logits_llvm();
        assert!(source
            .contains("%direct.choice.index = add nuw i64 %choice.base.index, %active.minus.one"));
    }

    #[test]
    fn lowest_id_ties_require_ascending_strict_greater_comparison() {
        fn argmax(values: &[f32]) -> usize {
            let mut winner = 0;
            for index in 1..values.len() {
                if values[index] > values[winner] {
                    winner = index;
                }
            }
            winner
        }
        assert_eq!(argmax(&[-3.0, 9.0, 9.0, 8.0]), 1);
        assert_eq!(argmax(&[-0.0, 0.0]), 0);
        let source = canonical_qwen3_logits_llvm();
        assert_eq!(source.matches("fcmp ogt float").count(), 1);
        assert!(!source.contains("fcmp oge"));
        assert!(source.contains("%token.next = add nuw i32 %token, 1"));
        assert!(source.contains("%first.exp = and i16 %first.bf16, 32640"));
        assert!(source.contains("%logit.exp = and i16 %logit.bf16, 32640"));
    }

    #[test]
    fn compact_record_layout_is_exact_120_bytes_and_reserved_bytes_are_zeroed() {
        let source = canonical_qwen3_logits_llvm();
        for (field, offset) in [
            ("record.generation.offset", 4),
            ("record.epoch.offset", 8),
            ("record.plan.base", 16),
            ("record.accepted.offset", 48),
            ("record.emitted.offset", 49),
            ("record.reserved.offset", 50),
            ("record.tokens.base", 52),
        ] {
            assert!(source.contains(&format!("%{field} = add nuw i64 %record.base, {offset}")));
        }
        assert!(source.contains("store i16 0, ptr addrspace(1) %record.reserved.ptr"));
        assert!(source.contains("%zero.more = icmp ult i64 %zero.index, 17"));
        assert!(source.contains("%records.expected = mul nuw i64 %sequences64, 120"));
        assert!(!source.contains("%records.expected = mul nuw i64 %sequences64, 96"));
    }

    #[test]
    fn maximal_accepted_prefix_and_correction_or_bonus_are_structurally_bound() {
        fn compact(draft: &[u32], target: &[u32]) -> (usize, Vec<u32>) {
            let mut accepted = 0;
            while accepted < draft.len() && draft[accepted] == target[accepted] {
                accepted += 1;
            }
            let mut emitted = draft[..accepted].to_vec();
            emitted.push(target[accepted]);
            (accepted, emitted)
        }
        assert_eq!(compact(&[3, 4, 5], &[9, 4, 5, 6]), (0, vec![9]));
        assert_eq!(compact(&[3, 4, 5], &[3, 4, 9, 6]), (2, vec![3, 4, 9]));
        assert_eq!(compact(&[3, 4, 5], &[3, 4, 5, 6]), (3, vec![3, 4, 5, 6]));
        let source = canonical_qwen3_logits_llvm();
        assert!(source.contains("%accepted.less.k = icmp ult i32 %accepted, %speculative.k"));
        assert!(source.contains("%tokens.equal = icmp eq i32 %draft.token, %target.token"));
        assert!(source
            .contains("%correction.index = add nuw i64 %choice.base.index, %accepted.final64"));
    }

    #[test]
    fn exact_buffer_lengths_aliases_overflow_and_draft_authority_fail_closed() {
        let target = profile(
            Qwen3LogitsModelRoleV1::Target8B,
            Qwen3LogitsBucketKindV1::SpeculativeS8K4C8192,
        );
        let (argmax_lengths, compact_lengths) = exact_lengths(target);
        let argmax_addresses = [0x1000, 0x1000_0000];
        let compact_addresses = compact_addresses(argmax_addresses[1]);
        assert!(Qwen3LogitsBufferContractV1::checked(
            target,
            argmax_addresses,
            argmax_lengths,
            Some((compact_addresses, compact_lengths.unwrap())),
        )
        .is_ok());

        let mut wrong = compact_lengths.unwrap();
        wrong[6] -= 1;
        assert_eq!(
            Qwen3LogitsBufferContractV1::checked(
                target,
                argmax_addresses,
                argmax_lengths,
                Some((compact_addresses, wrong)),
            ),
            Err(Qwen3LogitsBufferContractErrorV1::Length(
                Qwen3LogitsBufferV1::Records
            ))
        );
        let mut aliased = compact_addresses;
        aliased[6] = aliased[2];
        assert!(matches!(
            Qwen3LogitsBufferContractV1::checked(
                target,
                argmax_addresses,
                argmax_lengths,
                Some((aliased, compact_lengths.unwrap())),
            ),
            Err(Qwen3LogitsBufferContractErrorV1::Aliasing(_, _))
        ));
        assert!(matches!(
            Qwen3LogitsBufferContractV1::checked(
                target,
                [u64::MAX - 1, argmax_addresses[1]],
                argmax_lengths,
                Some((compact_addresses, compact_lengths.unwrap())),
            ),
            Err(Qwen3LogitsBufferContractErrorV1::Overflow(
                Qwen3LogitsBufferV1::Logits
            ))
        ));

        let draft = profile(
            Qwen3LogitsModelRoleV1::Draft06B,
            Qwen3LogitsBucketKindV1::SpeculativeS1K16C8192,
        );
        let (draft_lengths, _) = exact_lengths(draft);
        assert!(Qwen3LogitsBufferContractV1::checked(
            draft,
            [0x1000, 0x1000_0000],
            draft_lengths,
            None,
        )
        .is_ok());
        assert_eq!(
            Qwen3LogitsBufferContractV1::checked(
                draft,
                [0x1000, 0x1000_0000],
                draft_lengths,
                Some((compact_addresses, [0; 7])),
            ),
            Err(Qwen3LogitsBufferContractErrorV1::CompletionMode)
        );
    }

    #[test]
    fn every_extent_index_and_workitem_product_is_bounded() {
        let catalog = Qwen3LogitsProfileCatalogV1::canonical().unwrap();
        for profile in catalog.profiles() {
            let [logits, choices, draft, records] = profile.storage_extents();
            assert!(i64::try_from(logits).is_ok());
            assert!(i64::try_from(choices).is_ok());
            assert!(i64::try_from(draft).is_ok());
            assert!(i64::try_from(records).is_ok());
            assert_eq!(
                logits,
                choices
                    .checked_mul(u64::from(QWEN3_LOGITS_VOCABULARY_V1))
                    .unwrap()
            );
            assert_eq!(
                profile.argmax_grid_workitems()[0],
                profile.bucket().rows().checked_mul(64).unwrap()
            );
            if let Some([grid_x, ..]) = profile.compact_grid_workitems() {
                assert_eq!(grid_x, profile.choice_shape()[0].checked_mul(64).unwrap());
            }
            let last_logit = logits.checked_sub(1).unwrap();
            assert!(last_logit < logits);
        }
    }

    #[test]
    fn hostile_profile_vocab_record_tie_and_maximality_substitutions_are_rejected() {
        let original = profile(
            Qwen3LogitsModelRoleV1::Draft06B,
            Qwen3LogitsBucketKindV1::SpeculativeS1K16C8192,
        );
        let mut changed = original;
        changed.logits_elements = changed.logits_elements.checked_sub(1).unwrap();
        assert_ne!(
            qwen3_logits_kernel_ir_v1(original).identity(),
            qwen3_logits_kernel_ir_v1(changed).identity()
        );

        let source = canonical_qwen3_logits_llvm();
        for changed in [
            source.replacen(
                "%vocabulary.ok = icmp eq i32 %vocabulary, 151936",
                "%vocabulary.ok = icmp eq i32 %vocabulary, 151935",
                1,
            ),
            source.replacen("fcmp ogt float", "fcmp oge float", 1),
            source.replacen(
                "%records.expected = mul nuw i64 %sequences64, 120",
                "%records.expected = mul nuw i64 %sequences64, 96",
                1,
            ),
            source.replacen(
                "%accepted.less.k = icmp ult i32 %accepted, %speculative.k",
                "%accepted.less.k = icmp ule i32 %accepted, %speculative.k",
                1,
            ),
            source.replacen(
                "%active.minus.one = sub nuw i64 %active64, 1",
                "%active.minus.one = sub nuw i64 %active64, %active64",
                1,
            ),
            source.replacen(
                "store i16 0, ptr addrspace(1) %record.reserved.ptr",
                "store i16 1, ptr addrspace(1) %record.reserved.ptr",
                1,
            ),
        ] {
            assert!(matches!(
                validate_canonical_llvm(&changed),
                Err(PrepareQwen3LogitsKernelErrorV1::CompilerModule)
            ));
        }
    }

    #[test]
    fn complete_source_pin_bindings_and_nonclaims_fail_closed() {
        let source = canonical_qwen3_logits_llvm();
        assert_eq!(source.len(), QWEN3_LOGITS_LLVM_BYTES_V1);
        let digest: [u8; 32] = Sha256::digest(source.as_bytes()).into();
        assert_eq!(digest, QWEN3_LOGITS_LLVM_SHA256_V1);
        assert!(validate_canonical_llvm(&source).is_ok());
        let prepared = prepare_qwen3_logits_kernel_v1(bindings()).unwrap();
        assert!(!prepared.classifier_distinguishes_duplicate_profiles());
        assert!(!prepared.authenticates_compiler_origin());
        assert!(!prepared.proves_operator_or_numerical_refinement());
        assert!(!prepared.has_ferric_plan_identity_join());
        assert!(!prepared.grants_launch_authority());
        for invalid in [
            Qwen3LogitsSourceBindingsV1::new([0; 32], [2; 32], [3; 32], [4; 32]),
            Qwen3LogitsSourceBindingsV1::new([1; 32], [1; 32], [3; 32], [4; 32]),
        ] {
            assert!(matches!(
                prepare_qwen3_logits_kernel_v1(invalid),
                Err(PrepareQwen3LogitsKernelErrorV1::SourceBindings)
            ));
        }
    }

    #[test]
    fn machine_equivalent_profiles_keep_distinct_host_identities() {
        let first = profile(
            Qwen3LogitsModelRoleV1::Target8B,
            Qwen3LogitsBucketKindV1::DecodeS8C8192,
        );
        let second = profile(
            Qwen3LogitsModelRoleV1::Draft06B,
            Qwen3LogitsBucketKindV1::SpeculativeS1K8C8192,
        );
        assert_ne!(first.choice_shape(), second.choice_shape());
        assert_eq!(first.storage_extents()[0], second.storage_extents()[0]);
        assert_eq!(first.storage_extents()[1], second.storage_extents()[1]);
        assert_eq!(
            first.argmax_grid_workitems(),
            second.argmax_grid_workitems()
        );
        assert_ne!(first.identity(), second.identity());
        assert_ne!(
            qwen3_logits_kernel_ir_v1(first).identity(),
            qwen3_logits_kernel_ir_v1(second).identity()
        );
    }
}
