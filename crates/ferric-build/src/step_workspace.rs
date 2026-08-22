//! Addressless, bucket-exact workspace planning for one M1 model step.
//!
//! The ranges in this module cover request patch arrays, per-step numerical
//! tensors, the shared physical-page table, `RoPE` tables, K7 choices and the
//! target-only compact-completion inputs and outputs. Model weights and global
//! KV cache planes remain owned by [`crate::AddresslessModelMemoryPlan`].
//!
//! This module validates only inert identities and integer byte intervals. It
//! does not allocate memory, construct a native address, authenticate bytes,
//! bind an executable, lower packets, read results, or grant launch authority.

use crate::{hash_field, sha256::Sha256};
use ferric_qwen_kernels::{logits, rope_kv};
use ferric_spec::{
    geometry, Identity, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanDimensions,
    Qwen3PlanSelection, QWEN3_VOCABULARY_SIZE,
};
use std::fmt;

const BF16_BYTES: u64 = 2;
const U32_BYTES: u64 = 4;
const U64_BYTES: u64 = 8;
const IDENTITY_BYTES: u64 = 32;
const WORKSPACE_IDENTITY_DOMAIN: &[u8] = b"ferric.m1.step-workspace.v1";

/// Canonical format version of the addressless M1 step-workspace manifest.
pub const M1_STEP_WORKSPACE_LAYOUT_VERSION: u32 = 1;
/// Required alignment of a future allocation base containing the workspace.
pub const M1_STEP_WORKSPACE_ALLOCATION_ALIGNMENT_V1: u64 = 64;

/// One exact step-local or immutable-support range role.
///
/// Weights and global key/value cache planes are intentionally absent because
/// their ranges are resolved by [`crate::AddresslessModelMemoryPlan`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1StepWorkspaceRangeRole {
    /// Generated-runner `u32` input token IDs `[S,A]`.
    TokenIds,
    /// Generated-runner and K3 `u32` absolute position IDs `[S,A]`.
    PositionIds,
    /// Generated-runner and K7 `u32` live token counts `[S]`.
    ActiveLengths,
    /// Generated-runner context lengths, reused as K3 logical starts and K5 committed counts.
    ContextLengths,
    /// K3-K5 `u32` physical-page indices `[S,512]`.
    KvPageIndices,
    /// K3 immutable FP32 cosine table `[8192,64]`.
    RopeCosTable,
    /// K3 immutable FP32 sine table `[8192,64]`.
    RopeSinTable,
    /// BF16 in-place K1 residual stream, covering graph `Hidden` and `HiddenAfterAttention`.
    ResidualHidden,
    /// BF16 input-normalized stream `[S,A,H]`.
    NormalizedHidden,
    /// BF16 projected query `[S,A,QH,128]`.
    Query,
    /// BF16 projected key `[S,A,8,128]`.
    Key,
    /// BF16 projected value `[S,A,8,128]`.
    Value,
    /// BF16 normalized query `[S,A,QH,128]`.
    NormalizedQuery,
    /// BF16 normalized key `[S,A,8,128]`.
    NormalizedKey,
    /// BF16 rotated query `[S,A,QH,128]`.
    RotatedQuery,
    /// BF16 rotated key `[S,A,8,128]`.
    RotatedKey,
    /// BF16 attention output `[S,A,QH,128]`.
    AttentionOutput,
    /// BF16 post-attention normalized stream `[S,A,H]`.
    PostAttentionNormalized,
    /// BF16 gate projection `[S,A,I]`.
    Gate,
    /// BF16 up projection `[S,A,I]`.
    Up,
    /// BF16 `SwiGLU` result `[S,A,I]`.
    Activated,
    /// BF16 final normalized stream `[S,A,H]`.
    FinalNormalized,
    /// BF16 logits `[S,A,151936]`.
    Logits,
    /// K7 `u32` lowest-ID argmax choices `[S,A]`.
    Choices,
    /// Target speculative K7 `u32` draft choices `[K,S]`, one contiguous slice per iteration.
    DraftChoices,
    /// Target K7 `u32` request slots `[S]`.
    RequestSlots,
    /// Target K7 `u32` request generations `[S]`.
    RequestGenerations,
    /// Target K7 `u64` completion epochs `[S]`.
    CompletionEpochs,
    /// Target K7 32-byte plan identities `[S]`.
    PlanIdentities,
    /// Target K7 canonical 120-byte compact records `[S]`.
    CompactCompletionRecords,
    /// Target speculative draft-decode `u32` position IDs `[K,S]`, iteration-major.
    DraftPositionIds,
    /// Target speculative draft-decode `u32` context lengths `[K,S]`, iteration-major.
    DraftContextLengths,
}

impl M1StepWorkspaceRangeRole {
    const fn alignment(self) -> u64 {
        match self {
            Self::CompletionEpochs => U64_BYTES,
            Self::ResidualHidden
            | Self::NormalizedHidden
            | Self::Query
            | Self::Key
            | Self::Value
            | Self::NormalizedQuery
            | Self::NormalizedKey
            | Self::RotatedQuery
            | Self::RotatedKey
            | Self::AttentionOutput
            | Self::PostAttentionNormalized
            | Self::Gate
            | Self::Up
            | Self::Activated
            | Self::FinalNormalized
            | Self::Logits => BF16_BYTES,
            Self::PlanIdentities => 1,
            Self::TokenIds
            | Self::PositionIds
            | Self::ActiveLengths
            | Self::ContextLengths
            | Self::KvPageIndices
            | Self::RopeCosTable
            | Self::RopeSinTable
            | Self::Choices
            | Self::DraftChoices
            | Self::DraftPositionIds
            | Self::DraftContextLengths
            | Self::RequestSlots
            | Self::RequestGenerations
            | Self::CompactCompletionRecords => U32_BYTES,
        }
    }

    const fn tag(self) -> u8 {
        match self {
            Self::TokenIds => 1,
            Self::PositionIds => 2,
            Self::ActiveLengths => 3,
            Self::ContextLengths => 4,
            Self::KvPageIndices => 5,
            Self::RopeCosTable => 6,
            Self::RopeSinTable => 7,
            Self::ResidualHidden => 8,
            Self::NormalizedHidden => 9,
            Self::Query => 10,
            Self::Key => 11,
            Self::Value => 12,
            Self::NormalizedQuery => 13,
            Self::NormalizedKey => 14,
            Self::RotatedQuery => 15,
            Self::RotatedKey => 16,
            Self::AttentionOutput => 17,
            Self::PostAttentionNormalized => 18,
            Self::Gate => 19,
            Self::Up => 20,
            Self::Activated => 21,
            Self::FinalNormalized => 22,
            Self::Logits => 23,
            Self::Choices => 24,
            Self::DraftChoices => 25,
            Self::RequestSlots => 26,
            Self::RequestGenerations => 27,
            Self::CompletionEpochs => 28,
            Self::PlanIdentities => 29,
            Self::CompactCompletionRecords => 30,
            Self::DraftPositionIds => 31,
            Self::DraftContextLengths => 32,
        }
    }
}

/// One addressless half-open byte interval within the declared workspace.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1StepWorkspaceRange {
    role: M1StepWorkspaceRangeRole,
    offset: u64,
    byte_len: u64,
    alignment: u64,
}

/// One checked workspace range joined to its containing allocation identity.
///
/// This copyable value carries no native address, lease, mapping, or content
/// authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1StepWorkspaceMemoryBinding {
    allocation_id: Identity,
    range: M1StepWorkspaceRange,
}

impl M1StepWorkspaceMemoryBinding {
    /// Returns the inert identity of the containing future allocation.
    #[must_use]
    pub const fn allocation_id(self) -> Identity {
        self.allocation_id
    }

    /// Returns the exact checked addressless range.
    #[must_use]
    pub const fn range(self) -> M1StepWorkspaceRange {
        self.range
    }

    /// The binding carries no native address or mapped-memory authority.
    #[must_use]
    pub const fn authenticates_device_memory(self) -> bool {
        false
    }
}

impl M1StepWorkspaceRange {
    /// Constructs inert, unvalidated range declaration data.
    #[must_use]
    pub const fn new(
        role: M1StepWorkspaceRangeRole,
        offset: u64,
        byte_len: u64,
        alignment: u64,
    ) -> Self {
        Self {
            role,
            offset,
            byte_len,
            alignment,
        }
    }

    /// Returns the semantic workspace role.
    #[must_use]
    pub const fn role(self) -> M1StepWorkspaceRangeRole {
        self.role
    }

    /// Returns the byte offset from the future allocation base.
    #[must_use]
    pub const fn offset(self) -> u64 {
        self.offset
    }

    /// Returns the exact byte length.
    #[must_use]
    pub const fn byte_len(self) -> u64 {
        self.byte_len
    }

    /// Returns the exact pointee alignment required by the current ABI.
    #[must_use]
    pub const fn alignment(self) -> u64 {
        self.alignment
    }

    /// Returns the exclusive end when it is representable.
    #[must_use]
    pub const fn checked_end(self) -> Option<u64> {
        self.offset.checked_add(self.byte_len)
    }
}

/// Deterministically generated requirements for one exact finite selection.
///
/// This copyable-by-cloning metadata owns no allocation or initialized bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct M1StepWorkspaceRequirements {
    selection: Qwen3PlanSelection,
    allocation_byte_len: u64,
    allocation_alignment: u64,
    ranges: Box<[M1StepWorkspaceRange]>,
}

impl M1StepWorkspaceRequirements {
    /// Returns the exact role, mode, and bucket.
    #[must_use]
    pub const fn selection(&self) -> Qwen3PlanSelection {
        self.selection
    }

    /// Returns the exact containing allocation length.
    #[must_use]
    pub const fn allocation_byte_len(&self) -> u64 {
        self.allocation_byte_len
    }

    /// Returns the required future allocation-base alignment.
    #[must_use]
    pub const fn allocation_alignment(&self) -> u64 {
        self.allocation_alignment
    }

    /// Returns the exact ordered, pairwise-disjoint range roster.
    #[must_use]
    pub fn ranges(&self) -> &[M1StepWorkspaceRange] {
        &self.ranges
    }

    /// Returns one range by semantic role when that role is active.
    #[must_use]
    pub fn range(&self, role: M1StepWorkspaceRangeRole) -> Option<M1StepWorkspaceRange> {
        self.ranges.iter().copied().find(|range| range.role == role)
    }

    /// Requirements are addressless and grant no native allocation authority.
    #[must_use]
    pub const fn grants_allocation_authority(&self) -> bool {
        false
    }
}

/// One caller-declared future device allocation identity and exact geometry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeclaredM1StepWorkspaceAllocation {
    allocation_id: Identity,
    byte_len: u64,
    alignment: u64,
}

impl DeclaredM1StepWorkspaceAllocation {
    /// Creates inert allocation declaration data.
    #[must_use]
    pub const fn new(allocation_id: Identity, byte_len: u64, alignment: u64) -> Self {
        Self {
            allocation_id,
            byte_len,
            alignment,
        }
    }

    /// Returns the caller-supplied allocation identity.
    #[must_use]
    pub const fn allocation_id(self) -> Identity {
        self.allocation_id
    }

    /// Returns the caller-supplied exact byte length.
    #[must_use]
    pub const fn byte_len(self) -> u64 {
        self.byte_len
    }

    /// Returns the caller-supplied base alignment.
    #[must_use]
    pub const fn alignment(self) -> u64 {
        self.alignment
    }
}

/// Complete caller declaration for an available M1 step workspace.
#[derive(Debug, Eq, PartialEq)]
pub struct M1StepWorkspaceDeclaration {
    selection: Qwen3PlanSelection,
    allocation: DeclaredM1StepWorkspaceAllocation,
    ranges: Box<[M1StepWorkspaceRange]>,
}

impl M1StepWorkspaceDeclaration {
    /// Constructs an unvalidated declaration without granting allocation custody.
    #[must_use]
    pub fn new(
        selection: Qwen3PlanSelection,
        allocation: DeclaredM1StepWorkspaceAllocation,
        ranges: Box<[M1StepWorkspaceRange]>,
    ) -> Self {
        Self {
            selection,
            allocation,
            ranges,
        }
    }

    /// Returns the declared selection.
    #[must_use]
    pub const fn selection(&self) -> Qwen3PlanSelection {
        self.selection
    }

    /// Returns the inert future-allocation declaration.
    #[must_use]
    pub const fn allocation(&self) -> DeclaredM1StepWorkspaceAllocation {
        self.allocation
    }

    /// Returns the declared ordered range roster.
    #[must_use]
    pub fn ranges(&self) -> &[M1StepWorkspaceRange] {
        &self.ranges
    }
}

/// Linear available-workspace token containing only declaration data.
///
/// This type intentionally does not implement `Clone`.
///
/// ```compile_fail
/// use ferric_build::AvailableM1StepWorkspace;
/// fn require_clone<T: Clone>() {}
/// require_clone::<AvailableM1StepWorkspace>();
/// ```
#[derive(Debug, Eq, PartialEq)]
pub struct AvailableM1StepWorkspace {
    declaration: M1StepWorkspaceDeclaration,
}

impl AvailableM1StepWorkspace {
    /// Wraps one caller declaration as linearly available metadata.
    #[must_use]
    pub const fn new(declaration: M1StepWorkspaceDeclaration) -> Self {
        Self { declaration }
    }

    /// Borrows the unchanged declaration.
    #[must_use]
    pub const fn declaration(&self) -> &M1StepWorkspaceDeclaration {
        &self.declaration
    }

    /// Availability is not a native allocation lease.
    #[must_use]
    pub const fn authenticates_allocation(&self) -> bool {
        false
    }
}

/// Fail-closed workspace declaration or selection error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1StepWorkspacePlanError {
    /// The declared model role differs from the expected step role.
    SelectionRoleDrift,
    /// The declared execution mode differs from the expected step mode.
    SelectionModeDrift,
    /// The declared finite bucket differs from the expected step bucket.
    SelectionBucketDrift,
    /// The bucket is not valid for its declared execution mode.
    InvalidBucketMode,
    /// The future allocation identity is absent.
    MissingAllocationIdentity,
    /// The declared allocation-base alignment differs from the exact requirement.
    AllocationAlignment {
        /// Required alignment.
        expected: u64,
        /// Rejected declared alignment.
        actual: u64,
    },
    /// The declared allocation length differs from the deterministic layout.
    AllocationLength {
        /// Required bytes.
        expected: u64,
        /// Rejected declared bytes.
        actual: u64,
    },
    /// The range roster has the wrong length.
    RangeCount {
        /// Exact required count.
        expected: usize,
        /// Rejected count.
        actual: usize,
    },
    /// A declared range has zero, wrong, or unsatisfied alignment.
    RangeAlignment {
        /// Rejected range position.
        index: usize,
        /// Rejected semantic role.
        role: M1StepWorkspaceRangeRole,
    },
    /// A declared range exclusive end overflowed `u64`.
    RangeOverflow {
        /// Rejected range position.
        index: usize,
        /// Rejected semantic role.
        role: M1StepWorkspaceRangeRole,
    },
    /// A declared range extends beyond the containing allocation.
    RangeOutOfBounds {
        /// Rejected range position.
        index: usize,
        /// Rejected semantic role.
        role: M1StepWorkspaceRangeRole,
    },
    /// Two declared ranges overlap.
    RangeAlias {
        /// Earlier range position.
        left: usize,
        /// Later range position.
        right: usize,
    },
    /// A range role is absent, repeated, or out of canonical order.
    RangeRole {
        /// Rejected range position.
        index: usize,
        /// Required role.
        expected: M1StepWorkspaceRangeRole,
        /// Rejected role.
        actual: M1StepWorkspaceRangeRole,
    },
    /// A range starts at the wrong deterministic offset.
    RangeOffset {
        /// Rejected range position.
        index: usize,
        /// Required offset.
        expected: u64,
        /// Rejected offset.
        actual: u64,
    },
    /// A range has the wrong exact byte length.
    RangeLength {
        /// Rejected range position.
        index: usize,
        /// Required bytes.
        expected: u64,
        /// Rejected bytes.
        actual: u64,
    },
    /// Checked extent or layout arithmetic overflowed.
    ArithmeticOverflow,
}

impl fmt::Display for M1StepWorkspacePlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "M1 step workspace rejected: {self:?}")
    }
}

impl std::error::Error for M1StepWorkspacePlanError {}

/// Rejected planning attempt retaining the exact available token.
#[derive(Debug, Eq, PartialEq)]
pub struct M1StepWorkspacePlanFailure {
    error: M1StepWorkspacePlanError,
    available: AvailableM1StepWorkspace,
}

impl M1StepWorkspacePlanFailure {
    /// Returns the fail-closed diagnostic.
    #[must_use]
    pub const fn error(&self) -> M1StepWorkspacePlanError {
        self.error
    }

    /// Recovers the diagnostic and unchanged available token.
    #[must_use]
    pub fn into_parts(self) -> (M1StepWorkspacePlanError, AvailableM1StepWorkspace) {
        (self.error, self.available)
    }
}

/// Linear addressless plan for one exact model step workspace.
///
/// This type intentionally does not implement `Clone`. Its identity binds
/// declaration data only and authenticates no allocation, mapping, contents,
/// executable, launch, completion, readback, or hardware result.
///
/// ```compile_fail
/// use ferric_build::AddresslessM1StepWorkspacePlan;
/// fn require_clone<T: Clone>() {}
/// require_clone::<AddresslessM1StepWorkspacePlan>();
/// ```
#[derive(Debug, Eq, PartialEq)]
pub struct AddresslessM1StepWorkspacePlan {
    workspace_id: Identity,
    requirements: M1StepWorkspaceRequirements,
    available: AvailableM1StepWorkspace,
}

impl AddresslessM1StepWorkspacePlan {
    /// Returns the domain-separated identity of selection and exact declaration data.
    #[must_use]
    pub const fn workspace_id(&self) -> Identity {
        self.workspace_id
    }

    /// Returns the exact finite selection.
    #[must_use]
    pub const fn selection(&self) -> Qwen3PlanSelection {
        self.requirements.selection
    }

    /// Returns the exact checked future-allocation declaration.
    #[must_use]
    pub const fn allocation(&self) -> DeclaredM1StepWorkspaceAllocation {
        self.available.declaration.allocation
    }

    /// Returns the deterministic checked range roster.
    #[must_use]
    pub fn ranges(&self) -> &[M1StepWorkspaceRange] {
        &self.requirements.ranges
    }

    /// Returns one checked range by semantic role when active for this selection.
    #[must_use]
    pub fn range(&self, role: M1StepWorkspaceRangeRole) -> Option<M1StepWorkspaceRange> {
        self.requirements.range(role)
    }

    /// Joins one active range to the checked future-allocation identity.
    #[must_use]
    pub fn memory_binding(
        &self,
        role: M1StepWorkspaceRangeRole,
    ) -> Option<M1StepWorkspaceMemoryBinding> {
        self.range(role).map(|range| M1StepWorkspaceMemoryBinding {
            allocation_id: self.available.declaration.allocation.allocation_id,
            range,
        })
    }

    /// Aborts addressless planning and recovers the exact available declaration.
    #[must_use]
    pub fn abort(self) -> AvailableM1StepWorkspace {
        self.available
    }

    /// The plan grants no native address, allocation, or execution authority.
    #[must_use]
    pub const fn grants_runtime_authority(&self) -> bool {
        false
    }
}

/// Linear result of one addressless workspace planning attempt.
#[must_use]
#[derive(Debug, Eq, PartialEq)]
pub enum M1StepWorkspacePlanOutcome {
    /// The exact declaration was retained in an addressless plan.
    Planned(AddresslessM1StepWorkspacePlan),
    /// The unchanged available token remains recoverable.
    Rejected(M1StepWorkspacePlanFailure),
}

/// Generates the exact ordered range requirements for one finite selection.
///
/// # Errors
///
/// Returns [`M1StepWorkspacePlanError::InvalidBucketMode`] for a mode/bucket
/// mismatch, or [`M1StepWorkspacePlanError::ArithmeticOverflow`] if checked
/// extent construction cannot be represented.
pub fn m1_step_workspace_requirements(
    selection: Qwen3PlanSelection,
) -> Result<M1StepWorkspaceRequirements, M1StepWorkspacePlanError> {
    let dimensions = selection
        .bucket
        .dimensions(selection.role, selection.mode)
        .ok_or(M1StepWorkspacePlanError::InvalidBucketMode)?;
    build_requirements(selection, dimensions)
}

/// Consumes a linear available declaration into a checked addressless plan.
///
/// Rejection retains the unchanged available token. Success still grants no
/// allocation, address, initialization, load, dispatch, completion, or
/// readback authority.
pub fn plan_addressless_m1_step_workspace(
    expected_selection: Qwen3PlanSelection,
    available: AvailableM1StepWorkspace,
) -> M1StepWorkspacePlanOutcome {
    match validate_available(expected_selection, &available) {
        Ok(requirements) => {
            let workspace_id = workspace_identity(&available.declaration);
            M1StepWorkspacePlanOutcome::Planned(AddresslessM1StepWorkspacePlan {
                workspace_id,
                requirements,
                available,
            })
        }
        Err(error) => {
            M1StepWorkspacePlanOutcome::Rejected(M1StepWorkspacePlanFailure { error, available })
        }
    }
}

fn build_requirements(
    selection: Qwen3PlanSelection,
    dimensions: Qwen3PlanDimensions,
) -> Result<M1StepWorkspaceRequirements, M1StepWorkspacePlanError> {
    let rows = checked_mul(
        u64::from(dimensions.sequences),
        u64::from(dimensions.active_tokens),
    )?;
    let geometry = geometry(selection.role);
    let hidden = bf16_extent(rows, u64::from(geometry.hidden_size))?;
    let query = bf16_extent(
        checked_mul(rows, u64::from(geometry.query_heads))?,
        u64::from(geometry.head_dim),
    )?;
    let kv = bf16_extent(
        checked_mul(rows, u64::from(geometry.kv_heads))?,
        u64::from(geometry.head_dim),
    )?;
    let intermediate = bf16_extent(rows, u64::from(geometry.intermediate_size))?;
    let logits = bf16_extent(rows, u64::from(QWEN3_VOCABULARY_SIZE))?;
    let row_u32 = checked_mul(rows, U32_BYTES)?;
    let sequence_u32 = checked_mul(u64::from(dimensions.sequences), U32_BYTES)?;
    let pages = checked_mul(
        checked_mul(
            u64::from(dimensions.sequences),
            u64::from(rope_kv::QWEN3_KV_PAGE_TABLE_ENTRIES_V1),
        )?,
        U32_BYTES,
    )?;
    let trig = checked_mul(rope_kv::QWEN3_ROPE_TRIG_TABLE_ELEMENTS_V1, U32_BYTES)?;

    let mut builder = RangeBuilder::new();
    for (role, byte_len) in [
        (M1StepWorkspaceRangeRole::TokenIds, row_u32),
        (M1StepWorkspaceRangeRole::PositionIds, row_u32),
        (M1StepWorkspaceRangeRole::ActiveLengths, sequence_u32),
        (M1StepWorkspaceRangeRole::ContextLengths, sequence_u32),
        (M1StepWorkspaceRangeRole::KvPageIndices, pages),
        (M1StepWorkspaceRangeRole::RopeCosTable, trig),
        (M1StepWorkspaceRangeRole::RopeSinTable, trig),
        (M1StepWorkspaceRangeRole::ResidualHidden, hidden),
        (M1StepWorkspaceRangeRole::NormalizedHidden, hidden),
        (M1StepWorkspaceRangeRole::Query, query),
        (M1StepWorkspaceRangeRole::Key, kv),
        (M1StepWorkspaceRangeRole::Value, kv),
        (M1StepWorkspaceRangeRole::NormalizedQuery, query),
        (M1StepWorkspaceRangeRole::NormalizedKey, kv),
        (M1StepWorkspaceRangeRole::RotatedQuery, query),
        (M1StepWorkspaceRangeRole::RotatedKey, kv),
        (M1StepWorkspaceRangeRole::AttentionOutput, query),
        (M1StepWorkspaceRangeRole::PostAttentionNormalized, hidden),
        (M1StepWorkspaceRangeRole::Gate, intermediate),
        (M1StepWorkspaceRangeRole::Up, intermediate),
        (M1StepWorkspaceRangeRole::Activated, intermediate),
        (M1StepWorkspaceRangeRole::FinalNormalized, hidden),
        (M1StepWorkspaceRangeRole::Logits, logits),
        (M1StepWorkspaceRangeRole::Choices, row_u32),
    ] {
        builder.push(role, byte_len)?;
    }

    if matches!(selection.role, Qwen3ModelRole::Target8B) {
        let draft_metadata_bytes = if matches!(selection.mode, Qwen3ExecutionMode::Speculative) {
            let draft_tokens = dimensions
                .active_tokens
                .checked_sub(1)
                .ok_or(M1StepWorkspacePlanError::ArithmeticOverflow)?;
            let bytes = checked_mul(
                checked_mul(u64::from(dimensions.sequences), u64::from(draft_tokens))?,
                U32_BYTES,
            )?;
            builder.push(M1StepWorkspaceRangeRole::DraftChoices, bytes)?;
            Some(bytes)
        } else {
            None
        };
        builder.push(M1StepWorkspaceRangeRole::RequestSlots, sequence_u32)?;
        builder.push(M1StepWorkspaceRangeRole::RequestGenerations, sequence_u32)?;
        builder.push(
            M1StepWorkspaceRangeRole::CompletionEpochs,
            checked_mul(u64::from(dimensions.sequences), U64_BYTES)?,
        )?;
        builder.push(
            M1StepWorkspaceRangeRole::PlanIdentities,
            checked_mul(u64::from(dimensions.sequences), IDENTITY_BYTES)?,
        )?;
        builder.push(
            M1StepWorkspaceRangeRole::CompactCompletionRecords,
            checked_mul(
                u64::from(dimensions.sequences),
                logits::QWEN3_LOGITS_COMPACT_RECORD_BYTES_V1,
            )?,
        )?;
        if let Some(bytes) = draft_metadata_bytes {
            builder.push(M1StepWorkspaceRangeRole::DraftPositionIds, bytes)?;
            builder.push(M1StepWorkspaceRangeRole::DraftContextLengths, bytes)?;
        }
    }

    let (ranges, allocation_byte_len) = builder.finish()?;
    Ok(M1StepWorkspaceRequirements {
        selection,
        allocation_byte_len,
        allocation_alignment: M1_STEP_WORKSPACE_ALLOCATION_ALIGNMENT_V1,
        ranges,
    })
}

struct RangeBuilder {
    cursor: u64,
    ranges: Vec<M1StepWorkspaceRange>,
}

impl RangeBuilder {
    const fn new() -> Self {
        Self {
            cursor: 0,
            ranges: Vec::new(),
        }
    }

    fn push(
        &mut self,
        role: M1StepWorkspaceRangeRole,
        byte_len: u64,
    ) -> Result<(), M1StepWorkspacePlanError> {
        if byte_len == 0 {
            return Err(M1StepWorkspacePlanError::ArithmeticOverflow);
        }
        let offset = align_up(self.cursor, M1_STEP_WORKSPACE_ALLOCATION_ALIGNMENT_V1)?;
        self.cursor = offset
            .checked_add(byte_len)
            .ok_or(M1StepWorkspacePlanError::ArithmeticOverflow)?;
        self.ranges.push(M1StepWorkspaceRange {
            role,
            offset,
            byte_len,
            alignment: role.alignment(),
        });
        Ok(())
    }

    fn finish(self) -> Result<(Box<[M1StepWorkspaceRange]>, u64), M1StepWorkspacePlanError> {
        let byte_len = align_up(self.cursor, M1_STEP_WORKSPACE_ALLOCATION_ALIGNMENT_V1)?;
        Ok((self.ranges.into_boxed_slice(), byte_len))
    }
}

fn validate_available(
    expected_selection: Qwen3PlanSelection,
    available: &AvailableM1StepWorkspace,
) -> Result<M1StepWorkspaceRequirements, M1StepWorkspacePlanError> {
    let declaration = &available.declaration;
    if declaration.selection.role != expected_selection.role {
        return Err(M1StepWorkspacePlanError::SelectionRoleDrift);
    }
    if declaration.selection.mode != expected_selection.mode {
        return Err(M1StepWorkspacePlanError::SelectionModeDrift);
    }
    if declaration.selection.bucket != expected_selection.bucket {
        return Err(M1StepWorkspacePlanError::SelectionBucketDrift);
    }
    let requirements = m1_step_workspace_requirements(declaration.selection)?;
    if !declaration.allocation.allocation_id.is_present() {
        return Err(M1StepWorkspacePlanError::MissingAllocationIdentity);
    }
    if declaration.allocation.alignment != requirements.allocation_alignment {
        return Err(M1StepWorkspacePlanError::AllocationAlignment {
            expected: requirements.allocation_alignment,
            actual: declaration.allocation.alignment,
        });
    }
    if declaration.allocation.byte_len != requirements.allocation_byte_len {
        return Err(M1StepWorkspacePlanError::AllocationLength {
            expected: requirements.allocation_byte_len,
            actual: declaration.allocation.byte_len,
        });
    }
    if declaration.ranges.len() != requirements.ranges.len() {
        return Err(M1StepWorkspacePlanError::RangeCount {
            expected: requirements.ranges.len(),
            actual: declaration.ranges.len(),
        });
    }

    let mut ends = Vec::with_capacity(declaration.ranges.len());
    for (index, range) in declaration.ranges.iter().copied().enumerate() {
        if range.alignment == 0
            || !range.alignment.is_power_of_two()
            || range.alignment != range.role.alignment()
            || !range.offset.is_multiple_of(range.alignment)
        {
            return Err(M1StepWorkspacePlanError::RangeAlignment {
                index,
                role: range.role,
            });
        }
        let end = range.offset.checked_add(range.byte_len).ok_or(
            M1StepWorkspacePlanError::RangeOverflow {
                index,
                role: range.role,
            },
        )?;
        if end > declaration.allocation.byte_len {
            return Err(M1StepWorkspacePlanError::RangeOutOfBounds {
                index,
                role: range.role,
            });
        }
        ends.push(end);
    }
    for left in 0..declaration.ranges.len() {
        for right in left + 1..declaration.ranges.len() {
            if declaration.ranges[left].offset < ends[right]
                && declaration.ranges[right].offset < ends[left]
            {
                return Err(M1StepWorkspacePlanError::RangeAlias { left, right });
            }
        }
    }
    for (index, (actual, expected)) in declaration
        .ranges
        .iter()
        .copied()
        .zip(requirements.ranges.iter().copied())
        .enumerate()
    {
        if actual.role != expected.role {
            return Err(M1StepWorkspacePlanError::RangeRole {
                index,
                expected: expected.role,
                actual: actual.role,
            });
        }
        if actual.offset != expected.offset {
            return Err(M1StepWorkspacePlanError::RangeOffset {
                index,
                expected: expected.offset,
                actual: actual.offset,
            });
        }
        if actual.byte_len != expected.byte_len {
            return Err(M1StepWorkspacePlanError::RangeLength {
                index,
                expected: expected.byte_len,
                actual: actual.byte_len,
            });
        }
    }
    Ok(requirements)
}

fn workspace_identity(declaration: &M1StepWorkspaceDeclaration) -> Identity {
    let mut record = Vec::with_capacity(96 + declaration.ranges.len() * 32);
    record.extend_from_slice(&M1_STEP_WORKSPACE_LAYOUT_VERSION.to_le_bytes());
    record.push(role_tag(declaration.selection.role));
    record.push(mode_tag(declaration.selection.mode));
    record.push(bucket_tag(declaration.selection));
    record.extend_from_slice(declaration.allocation.allocation_id.as_bytes());
    record.extend_from_slice(&declaration.allocation.byte_len.to_le_bytes());
    record.extend_from_slice(&declaration.allocation.alignment.to_le_bytes());
    record.extend_from_slice(&(declaration.ranges.len() as u64).to_le_bytes());
    for range in &declaration.ranges {
        record.push(range.role.tag());
        record.extend_from_slice(&range.offset.to_le_bytes());
        record.extend_from_slice(&range.byte_len.to_le_bytes());
        record.extend_from_slice(&range.alignment.to_le_bytes());
    }
    let mut hasher = Sha256::new();
    hash_field(&mut hasher, WORKSPACE_IDENTITY_DOMAIN);
    hash_field(&mut hasher, &record);
    Identity::new(hasher.finish())
}

const fn role_tag(role: Qwen3ModelRole) -> u8 {
    match role {
        Qwen3ModelRole::Target8B => 1,
        Qwen3ModelRole::Draft06B => 2,
    }
}

const fn mode_tag(mode: Qwen3ExecutionMode) -> u8 {
    match mode {
        Qwen3ExecutionMode::Prefill => 1,
        Qwen3ExecutionMode::Decode => 2,
        Qwen3ExecutionMode::Speculative => 3,
    }
}

const fn bucket_tag(selection: Qwen3PlanSelection) -> u8 {
    use ferric_spec::Qwen3PlanBucket;
    match selection.bucket {
        Qwen3PlanBucket::PrefillS1T128 => 1,
        Qwen3PlanBucket::PrefillS8T128 => 2,
        Qwen3PlanBucket::PrefillS1T512 => 3,
        Qwen3PlanBucket::PrefillS1T2048 => 4,
        Qwen3PlanBucket::DecodeS1C8192 => 5,
        Qwen3PlanBucket::DecodeS8C8192 => 6,
        Qwen3PlanBucket::DecodeS32C8192 => 7,
        Qwen3PlanBucket::SpeculativeS1K4C8192 => 8,
        Qwen3PlanBucket::SpeculativeS8K4C8192 => 9,
        Qwen3PlanBucket::SpeculativeS1K8C8192 => 10,
        Qwen3PlanBucket::SpeculativeS1K16C8192 => 11,
    }
}

fn checked_mul(left: u64, right: u64) -> Result<u64, M1StepWorkspacePlanError> {
    left.checked_mul(right)
        .ok_or(M1StepWorkspacePlanError::ArithmeticOverflow)
}

fn bf16_extent(left: u64, right: u64) -> Result<u64, M1StepWorkspacePlanError> {
    checked_mul(checked_mul(left, right)?, BF16_BYTES)
}

fn align_up(value: u64, alignment: u64) -> Result<u64, M1StepWorkspacePlanError> {
    let mask = alignment
        .checked_sub(1)
        .ok_or(M1StepWorkspacePlanError::ArithmeticOverflow)?;
    value
        .checked_add(mask)
        .map(|sum| sum & !mask)
        .ok_or(M1StepWorkspacePlanError::ArithmeticOverflow)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferric_qwen_kernels::{gemm, paged_decode, prefill, rmsnorm, swiglu};
    use ferric_spec::Qwen3PlanBucket;

    const BUCKETS: [(Qwen3ExecutionMode, Qwen3PlanBucket); 11] = [
        (Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128),
        (Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS8T128),
        (Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T512),
        (Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T2048),
        (Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS1C8192),
        (Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS8C8192),
        (Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS32C8192),
        (
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
        ),
        (
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS8K4C8192,
        ),
        (
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K8C8192,
        ),
        (
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K16C8192,
        ),
    ];

    fn identity(seed: u8) -> Identity {
        Identity::new([seed; 32])
    }

    const fn selection(
        role: Qwen3ModelRole,
        mode: Qwen3ExecutionMode,
        bucket: Qwen3PlanBucket,
    ) -> Qwen3PlanSelection {
        Qwen3PlanSelection { role, mode, bucket }
    }

    fn exact_available(selection: Qwen3PlanSelection) -> AvailableM1StepWorkspace {
        let requirements = m1_step_workspace_requirements(selection).unwrap();
        AvailableM1StepWorkspace::new(M1StepWorkspaceDeclaration::new(
            selection,
            DeclaredM1StepWorkspaceAllocation::new(
                identity(9),
                requirements.allocation_byte_len(),
                requirements.allocation_alignment(),
            ),
            requirements.ranges().to_vec().into_boxed_slice(),
        ))
    }

    fn rejection(
        expected: Qwen3PlanSelection,
        available: AvailableM1StepWorkspace,
    ) -> M1StepWorkspacePlanFailure {
        let M1StepWorkspacePlanOutcome::Rejected(failure) =
            plan_addressless_m1_step_workspace(expected, available)
        else {
            panic!("expected rejection")
        };
        failure
    }

    #[test]
    fn all_22_finite_layouts_are_exact_aligned_and_disjoint() {
        for role in [Qwen3ModelRole::Target8B, Qwen3ModelRole::Draft06B] {
            for (mode, bucket) in BUCKETS {
                let selection = selection(role, mode, bucket);
                let requirements = m1_step_workspace_requirements(selection).unwrap();
                assert_eq!(requirements.selection(), selection);
                assert_eq!(requirements.allocation_alignment(), 64);
                assert!(requirements.allocation_byte_len().is_multiple_of(64));
                let expected_count = match (role, mode) {
                    (Qwen3ModelRole::Target8B, Qwen3ExecutionMode::Speculative) => 32,
                    (Qwen3ModelRole::Target8B, _) => 29,
                    (Qwen3ModelRole::Draft06B, _) => 24,
                };
                assert_eq!(requirements.ranges().len(), expected_count);
                let has_draft_metadata =
                    role == Qwen3ModelRole::Target8B && mode == Qwen3ExecutionMode::Speculative;
                for metadata_role in [
                    M1StepWorkspaceRangeRole::DraftPositionIds,
                    M1StepWorkspaceRangeRole::DraftContextLengths,
                ] {
                    assert_eq!(
                        requirements.range(metadata_role).is_some(),
                        has_draft_metadata
                    );
                }
                if has_draft_metadata {
                    assert_eq!(
                        requirements.ranges()[expected_count - 2].role(),
                        M1StepWorkspaceRangeRole::DraftPositionIds
                    );
                    assert_eq!(
                        requirements.ranges()[expected_count - 1].role(),
                        M1StepWorkspaceRangeRole::DraftContextLengths
                    );
                }
                for (index, range) in requirements.ranges().iter().copied().enumerate() {
                    assert!(range.byte_len() > 0);
                    assert!(range.offset().is_multiple_of(64));
                    assert!(range.offset().is_multiple_of(range.alignment()));
                    assert!(range.checked_end().unwrap() <= requirements.allocation_byte_len());
                    for other in &requirements.ranges()[index + 1..] {
                        assert!(range.checked_end().unwrap() <= other.offset());
                    }
                }
                let M1StepWorkspacePlanOutcome::Planned(plan) =
                    plan_addressless_m1_step_workspace(selection, exact_available(selection))
                else {
                    panic!("exact declaration rejected")
                };
                assert!(plan.workspace_id().is_present());
                assert_eq!(plan.selection(), selection);
                assert!(!plan.grants_runtime_authority());
                let token_ids = plan
                    .memory_binding(M1StepWorkspaceRangeRole::TokenIds)
                    .unwrap();
                assert_eq!(token_ids.allocation_id(), identity(9));
                assert_eq!(
                    token_ids.range(),
                    requirements
                        .range(M1StepWorkspaceRangeRole::TokenIds)
                        .unwrap()
                );
                assert!(!token_ids.authenticates_device_memory());
                assert_eq!(plan.abort(), exact_available(selection));
            }
        }
    }

    #[test]
    fn appended_speculative_metadata_role_tags_do_not_renumber_existing_roles() {
        assert_eq!(M1StepWorkspaceRangeRole::DraftChoices.tag(), 25);
        assert_eq!(M1StepWorkspaceRangeRole::CompactCompletionRecords.tag(), 30);
        assert_eq!(M1StepWorkspaceRangeRole::DraftPositionIds.tag(), 31);
        assert_eq!(M1StepWorkspaceRangeRole::DraftContextLengths.tag(), 32);
    }

    #[test]
    fn exact_extents_cover_patch_activation_kv_rope_and_k7_contracts() {
        let selection = selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS8K4C8192,
        );
        let requirements = m1_step_workspace_requirements(selection).unwrap();
        let length = |role| requirements.range(role).unwrap().byte_len();
        let rows = 8 * 5;
        assert_eq!(length(M1StepWorkspaceRangeRole::TokenIds), rows * 4);
        assert_eq!(length(M1StepWorkspaceRangeRole::PositionIds), rows * 4);
        assert_eq!(length(M1StepWorkspaceRangeRole::ActiveLengths), 8 * 4);
        assert_eq!(length(M1StepWorkspaceRangeRole::ContextLengths), 8 * 4);
        assert_eq!(
            length(M1StepWorkspaceRangeRole::KvPageIndices),
            8 * u64::from(rope_kv::QWEN3_KV_PAGE_TABLE_ENTRIES_V1) * 4
        );
        assert_eq!(
            length(M1StepWorkspaceRangeRole::RopeCosTable),
            rope_kv::QWEN3_ROPE_TRIG_TABLE_ELEMENTS_V1 * 4
        );
        assert_eq!(
            length(M1StepWorkspaceRangeRole::ResidualHidden),
            rows * 4_096 * 2
        );
        assert_eq!(length(M1StepWorkspaceRangeRole::Query), rows * 32 * 128 * 2);
        assert_eq!(length(M1StepWorkspaceRangeRole::Key), rows * 8 * 128 * 2);
        assert_eq!(length(M1StepWorkspaceRangeRole::Gate), rows * 12_288 * 2);
        assert_eq!(
            length(M1StepWorkspaceRangeRole::Logits),
            rows * u64::from(QWEN3_VOCABULARY_SIZE) * 2
        );
        assert_eq!(length(M1StepWorkspaceRangeRole::Choices), rows * 4);
        assert_eq!(length(M1StepWorkspaceRangeRole::DraftChoices), 8 * 4 * 4);
        assert_eq!(
            length(M1StepWorkspaceRangeRole::DraftPositionIds),
            8 * 4 * 4
        );
        assert_eq!(
            length(M1StepWorkspaceRangeRole::DraftContextLengths),
            8 * 4 * 4
        );
        assert_eq!(length(M1StepWorkspaceRangeRole::RequestSlots), 8 * 4);
        assert_eq!(length(M1StepWorkspaceRangeRole::RequestGenerations), 8 * 4);
        assert_eq!(length(M1StepWorkspaceRangeRole::CompletionEpochs), 8 * 8);
        assert_eq!(length(M1StepWorkspaceRangeRole::PlanIdentities), 8 * 32);
        assert_eq!(
            length(M1StepWorkspaceRangeRole::CompactCompletionRecords),
            8 * logits::QWEN3_LOGITS_COMPACT_RECORD_BYTES_V1
        );
    }

    #[test]
    fn draft_and_target_direct_omit_inactive_k7_ranges() {
        let draft = m1_step_workspace_requirements(selection(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K16C8192,
        ))
        .unwrap();
        assert!(draft.range(M1StepWorkspaceRangeRole::Choices).is_some());
        for role in [
            M1StepWorkspaceRangeRole::DraftChoices,
            M1StepWorkspaceRangeRole::DraftPositionIds,
            M1StepWorkspaceRangeRole::DraftContextLengths,
            M1StepWorkspaceRangeRole::RequestSlots,
            M1StepWorkspaceRangeRole::RequestGenerations,
            M1StepWorkspaceRangeRole::CompletionEpochs,
            M1StepWorkspaceRangeRole::PlanIdentities,
            M1StepWorkspaceRangeRole::CompactCompletionRecords,
        ] {
            assert!(draft.range(role).is_none());
        }
        let direct = m1_step_workspace_requirements(selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS32C8192,
        ))
        .unwrap();
        assert!(direct
            .range(M1StepWorkspaceRangeRole::DraftChoices)
            .is_none());
        assert!(direct
            .range(M1StepWorkspaceRangeRole::DraftPositionIds)
            .is_none());
        assert!(direct
            .range(M1StepWorkspaceRangeRole::DraftContextLengths)
            .is_none());
        assert!(direct
            .range(M1StepWorkspaceRangeRole::CompactCompletionRecords)
            .is_some());
    }

    #[test]
    fn all_workspace_k7_extents_match_the_canonical_kernel_catalog() {
        let catalog = logits::Qwen3LogitsProfileCatalogV1::canonical().unwrap();
        assert_eq!(catalog.profiles().len(), 22);
        for (index, profile) in catalog.profiles().iter().copied().enumerate() {
            let role = if index < BUCKETS.len() {
                Qwen3ModelRole::Target8B
            } else {
                Qwen3ModelRole::Draft06B
            };
            let (mode, bucket) = BUCKETS[index % BUCKETS.len()];
            let requirements =
                m1_step_workspace_requirements(selection(role, mode, bucket)).unwrap();
            let [logits_elements, choices_elements, draft_elements, record_bytes] =
                profile.storage_extents();
            assert_eq!(
                requirements
                    .range(M1StepWorkspaceRangeRole::Logits)
                    .unwrap()
                    .byte_len(),
                logits_elements * BF16_BYTES
            );
            assert_eq!(
                requirements
                    .range(M1StepWorkspaceRangeRole::Choices)
                    .unwrap()
                    .byte_len(),
                choices_elements * U32_BYTES
            );
            assert_eq!(
                requirements
                    .range(M1StepWorkspaceRangeRole::DraftChoices)
                    .map(M1StepWorkspaceRange::byte_len),
                (draft_elements != 0).then_some(draft_elements * U32_BYTES)
            );
            for role in [
                M1StepWorkspaceRangeRole::DraftPositionIds,
                M1StepWorkspaceRangeRole::DraftContextLengths,
            ] {
                assert_eq!(
                    requirements.range(role).map(M1StepWorkspaceRange::byte_len),
                    (draft_elements != 0).then_some(draft_elements * U32_BYTES)
                );
            }
            assert_eq!(
                requirements
                    .range(M1StepWorkspaceRangeRole::CompactCompletionRecords)
                    .map(M1StepWorkspaceRange::byte_len),
                (record_bytes != 0).then_some(record_bytes)
            );
        }
    }

    #[test]
    fn all_workspace_k1_extents_match_embedding_and_projection_catalogs() {
        let embeddings = gemm::Qwen3TokenEmbeddingProfileCatalogV1::canonical().unwrap();
        for (index, profile) in embeddings.profiles().iter().copied().enumerate() {
            let role = if index < BUCKETS.len() {
                Qwen3ModelRole::Target8B
            } else {
                Qwen3ModelRole::Draft06B
            };
            let (mode, bucket) = BUCKETS[index % BUCKETS.len()];
            let requirements =
                m1_step_workspace_requirements(selection(role, mode, bucket)).unwrap();
            let [tokens, _weight, output] = profile.storage_elements();
            assert_eq!(
                requirements
                    .range(M1StepWorkspaceRangeRole::TokenIds)
                    .unwrap()
                    .byte_len(),
                tokens * U32_BYTES
            );
            assert_eq!(
                requirements
                    .range(M1StepWorkspaceRangeRole::ResidualHidden)
                    .unwrap()
                    .byte_len(),
                output * BF16_BYTES
            );
        }

        let projections = gemm::Qwen3GemmProfileCatalogV1::canonical().unwrap();
        for (index, profile) in projections.profiles().iter().copied().enumerate() {
            let role = if index < BUCKETS.len() * 8 {
                Qwen3ModelRole::Target8B
            } else {
                Qwen3ModelRole::Draft06B
            };
            let bucket_index = (index / 8) % BUCKETS.len();
            let (mode, bucket) = BUCKETS[bucket_index];
            let requirements =
                m1_step_workspace_requirements(selection(role, mode, bucket)).unwrap();
            let (input_role, output_role) = match profile.operation() {
                gemm::Qwen3GemmOperationV1::QueryProjection => (
                    M1StepWorkspaceRangeRole::NormalizedHidden,
                    M1StepWorkspaceRangeRole::Query,
                ),
                gemm::Qwen3GemmOperationV1::KeyProjection => (
                    M1StepWorkspaceRangeRole::NormalizedHidden,
                    M1StepWorkspaceRangeRole::Key,
                ),
                gemm::Qwen3GemmOperationV1::ValueProjection => (
                    M1StepWorkspaceRangeRole::NormalizedHidden,
                    M1StepWorkspaceRangeRole::Value,
                ),
                gemm::Qwen3GemmOperationV1::AttentionOutputResidual => (
                    M1StepWorkspaceRangeRole::AttentionOutput,
                    M1StepWorkspaceRangeRole::ResidualHidden,
                ),
                gemm::Qwen3GemmOperationV1::GateProjection => (
                    M1StepWorkspaceRangeRole::PostAttentionNormalized,
                    M1StepWorkspaceRangeRole::Gate,
                ),
                gemm::Qwen3GemmOperationV1::UpProjection => (
                    M1StepWorkspaceRangeRole::PostAttentionNormalized,
                    M1StepWorkspaceRangeRole::Up,
                ),
                gemm::Qwen3GemmOperationV1::DownResidual => (
                    M1StepWorkspaceRangeRole::Activated,
                    M1StepWorkspaceRangeRole::ResidualHidden,
                ),
                gemm::Qwen3GemmOperationV1::LogitsProjection => (
                    M1StepWorkspaceRangeRole::FinalNormalized,
                    M1StepWorkspaceRangeRole::Logits,
                ),
            };
            let [input, _weight, output] = profile.storage_elements();
            assert_eq!(
                requirements.range(input_role).unwrap().byte_len(),
                input * BF16_BYTES
            );
            assert_eq!(
                requirements.range(output_role).unwrap().byte_len(),
                output * BF16_BYTES
            );
        }
    }

    #[test]
    fn all_workspace_k2_through_k6_extents_match_canonical_kernel_catalogs() {
        let norms = rmsnorm::Qwen3RmsNormProfileCatalogV1::canonical().unwrap();
        for (index, profile) in norms.profiles().iter().copied().enumerate() {
            if matches!(
                profile.operation(),
                rmsnorm::Qwen3RmsNormOperationV1::ResidualFusedHidden
            ) {
                continue;
            }
            let role = if index < BUCKETS.len() * 6 {
                Qwen3ModelRole::Target8B
            } else {
                Qwen3ModelRole::Draft06B
            };
            let (mode, bucket) = BUCKETS[(index / 6) % BUCKETS.len()];
            let requirements =
                m1_step_workspace_requirements(selection(role, mode, bucket)).unwrap();
            let (input, output) = match profile.operation() {
                rmsnorm::Qwen3RmsNormOperationV1::InputRmsNorm => (
                    M1StepWorkspaceRangeRole::ResidualHidden,
                    M1StepWorkspaceRangeRole::NormalizedHidden,
                ),
                rmsnorm::Qwen3RmsNormOperationV1::QueryRmsNorm => (
                    M1StepWorkspaceRangeRole::Query,
                    M1StepWorkspaceRangeRole::NormalizedQuery,
                ),
                rmsnorm::Qwen3RmsNormOperationV1::KeyRmsNorm => (
                    M1StepWorkspaceRangeRole::Key,
                    M1StepWorkspaceRangeRole::NormalizedKey,
                ),
                rmsnorm::Qwen3RmsNormOperationV1::PostAttentionRmsNorm => (
                    M1StepWorkspaceRangeRole::ResidualHidden,
                    M1StepWorkspaceRangeRole::PostAttentionNormalized,
                ),
                rmsnorm::Qwen3RmsNormOperationV1::FinalRmsNorm => (
                    M1StepWorkspaceRangeRole::ResidualHidden,
                    M1StepWorkspaceRangeRole::FinalNormalized,
                ),
                rmsnorm::Qwen3RmsNormOperationV1::ResidualFusedHidden => unreachable!(),
            };
            let row_bytes = profile.row_elements() * BF16_BYTES;
            assert_eq!(requirements.range(input).unwrap().byte_len(), row_bytes);
            assert_eq!(requirements.range(output).unwrap().byte_len(), row_bytes);
        }

        let rope_catalog = rope_kv::Qwen3RopeKvProfileCatalogV1::canonical().unwrap();
        for (index, profile) in rope_catalog.profiles().iter().copied().enumerate() {
            let role = if index < BUCKETS.len() * 2 {
                Qwen3ModelRole::Target8B
            } else {
                Qwen3ModelRole::Draft06B
            };
            let (mode, bucket) = BUCKETS[(index / 2) % BUCKETS.len()];
            let requirements =
                m1_step_workspace_requirements(selection(role, mode, bucket)).unwrap();
            let query_bytes = profile.query_elements() * BF16_BYTES;
            let kv_bytes = profile.kv_elements() * BF16_BYTES;
            match profile.operation() {
                rope_kv::Qwen3RopeKvOperationV1::Rope => {
                    for range_role in [
                        M1StepWorkspaceRangeRole::NormalizedQuery,
                        M1StepWorkspaceRangeRole::RotatedQuery,
                    ] {
                        assert_eq!(
                            requirements.range(range_role).unwrap().byte_len(),
                            query_bytes
                        );
                    }
                    for range_role in [
                        M1StepWorkspaceRangeRole::NormalizedKey,
                        M1StepWorkspaceRangeRole::RotatedKey,
                    ] {
                        assert_eq!(requirements.range(range_role).unwrap().byte_len(), kv_bytes);
                    }
                }
                rope_kv::Qwen3RopeKvOperationV1::PagedKvWrite => {
                    assert_eq!(
                        requirements
                            .range(M1StepWorkspaceRangeRole::RotatedKey)
                            .unwrap()
                            .byte_len(),
                        kv_bytes
                    );
                    assert_eq!(
                        requirements
                            .range(M1StepWorkspaceRangeRole::Value)
                            .unwrap()
                            .byte_len(),
                        kv_bytes
                    );
                }
            }
        }

        let prefill = prefill::Qwen3PrefillProfileCatalogV1::canonical().unwrap();
        for (index, profile) in prefill.profiles().iter().copied().enumerate() {
            let role = if index < 4 {
                Qwen3ModelRole::Target8B
            } else {
                Qwen3ModelRole::Draft06B
            };
            let (mode, bucket) = BUCKETS[index % 4];
            let requirements =
                m1_step_workspace_requirements(selection(role, mode, bucket)).unwrap();
            let query_bytes = profile.query_elements() * BF16_BYTES;
            assert_eq!(
                requirements
                    .range(M1StepWorkspaceRangeRole::RotatedQuery)
                    .unwrap()
                    .byte_len(),
                query_bytes
            );
            assert_eq!(
                requirements
                    .range(M1StepWorkspaceRangeRole::AttentionOutput)
                    .unwrap()
                    .byte_len(),
                query_bytes
            );
            assert_eq!(
                requirements
                    .range(M1StepWorkspaceRangeRole::KvPageIndices)
                    .unwrap()
                    .byte_len(),
                profile.page_table_elements() * U32_BYTES
            );
        }

        let decode = paged_decode::Qwen3PagedDecodeProfileCatalogV1::canonical().unwrap();
        for (index, profile) in decode.profiles().iter().copied().enumerate() {
            let role = if index < 7 {
                Qwen3ModelRole::Target8B
            } else {
                Qwen3ModelRole::Draft06B
            };
            let (mode, bucket) = BUCKETS[4 + index % 7];
            let requirements =
                m1_step_workspace_requirements(selection(role, mode, bucket)).unwrap();
            let query_bytes = profile.query_elements() * BF16_BYTES;
            assert_eq!(
                requirements
                    .range(M1StepWorkspaceRangeRole::RotatedQuery)
                    .unwrap()
                    .byte_len(),
                query_bytes
            );
            assert_eq!(
                requirements
                    .range(M1StepWorkspaceRangeRole::AttentionOutput)
                    .unwrap()
                    .byte_len(),
                query_bytes
            );
            assert_eq!(
                requirements
                    .range(M1StepWorkspaceRangeRole::KvPageIndices)
                    .unwrap()
                    .byte_len(),
                profile.page_table_elements() * U32_BYTES
            );
            assert_eq!(
                requirements
                    .range(M1StepWorkspaceRangeRole::ContextLengths)
                    .unwrap()
                    .byte_len(),
                profile.context_elements() * U32_BYTES
            );
        }

        let swiglu = swiglu::Qwen3SwiGluProfileCatalogV1::canonical().unwrap();
        for (index, profile) in swiglu.profiles().iter().copied().enumerate() {
            let role = if index < BUCKETS.len() {
                Qwen3ModelRole::Target8B
            } else {
                Qwen3ModelRole::Draft06B
            };
            let (mode, bucket) = BUCKETS[index % BUCKETS.len()];
            let requirements =
                m1_step_workspace_requirements(selection(role, mode, bucket)).unwrap();
            for range_role in [
                M1StepWorkspaceRangeRole::Gate,
                M1StepWorkspaceRangeRole::Up,
                M1StepWorkspaceRangeRole::Activated,
            ] {
                assert_eq!(
                    requirements.range(range_role).unwrap().byte_len(),
                    profile.bytes_per_buffer()
                );
            }
        }
    }

    #[test]
    fn invalid_mode_bucket_and_selection_drift_fail_closed() {
        let exact = selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        );
        assert_eq!(
            m1_step_workspace_requirements(selection(
                Qwen3ModelRole::Target8B,
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::DecodeS1C8192,
            )),
            Err(M1StepWorkspacePlanError::InvalidBucketMode)
        );
        let wrong_role = selection(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        );
        assert_eq!(
            rejection(exact, exact_available(wrong_role)).error(),
            M1StepWorkspacePlanError::SelectionRoleDrift
        );
        let wrong_mode = selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T128,
        );
        assert_eq!(
            rejection(exact, exact_available(wrong_mode)).error(),
            M1StepWorkspacePlanError::SelectionModeDrift
        );
        let wrong_bucket = selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS8C8192,
        );
        assert_eq!(
            rejection(exact, exact_available(wrong_bucket)).error(),
            M1StepWorkspacePlanError::SelectionBucketDrift
        );
    }

    #[test]
    fn allocation_failures_recover_the_exact_available_token() {
        let selection = selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        );
        let requirements = m1_step_workspace_requirements(selection).unwrap();
        for (allocation, error) in [
            (
                DeclaredM1StepWorkspaceAllocation::new(
                    Identity::new([0; 32]),
                    requirements.allocation_byte_len(),
                    64,
                ),
                M1StepWorkspacePlanError::MissingAllocationIdentity,
            ),
            (
                DeclaredM1StepWorkspaceAllocation::new(
                    identity(7),
                    requirements.allocation_byte_len(),
                    32,
                ),
                M1StepWorkspacePlanError::AllocationAlignment {
                    expected: 64,
                    actual: 32,
                },
            ),
            (
                DeclaredM1StepWorkspaceAllocation::new(
                    identity(7),
                    requirements.allocation_byte_len() - 64,
                    64,
                ),
                M1StepWorkspacePlanError::AllocationLength {
                    expected: requirements.allocation_byte_len(),
                    actual: requirements.allocation_byte_len() - 64,
                },
            ),
        ] {
            let available = AvailableM1StepWorkspace::new(M1StepWorkspaceDeclaration::new(
                selection,
                allocation,
                requirements.ranges().to_vec().into_boxed_slice(),
            ));
            let declaration = format!("{:?}", available.declaration());
            let failure = rejection(selection, available);
            assert_eq!(failure.error(), error);
            let (recovered_error, recovered) = failure.into_parts();
            assert_eq!(recovered_error, error);
            assert_eq!(format!("{:?}", recovered.declaration()), declaration);
        }
    }

    #[test]
    fn hostile_range_rosters_reject_alignment_overflow_bounds_alias_and_drift() {
        let selection = selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
        );
        let requirements = m1_step_workspace_requirements(selection).unwrap();
        let reject = |ranges: Vec<M1StepWorkspaceRange>| {
            rejection(
                selection,
                AvailableM1StepWorkspace::new(M1StepWorkspaceDeclaration::new(
                    selection,
                    DeclaredM1StepWorkspaceAllocation::new(
                        identity(3),
                        requirements.allocation_byte_len(),
                        64,
                    ),
                    ranges.into_boxed_slice(),
                )),
            )
            .error()
        };

        let mut short = requirements.ranges().to_vec();
        short.pop();
        assert_eq!(
            reject(short),
            M1StepWorkspacePlanError::RangeCount {
                expected: 32,
                actual: 31
            }
        );

        let mut alignment = requirements.ranges().to_vec();
        alignment[0] = M1StepWorkspaceRange::new(
            alignment[0].role(),
            alignment[0].offset(),
            alignment[0].byte_len(),
            2,
        );
        assert_eq!(
            reject(alignment),
            M1StepWorkspacePlanError::RangeAlignment {
                index: 0,
                role: M1StepWorkspaceRangeRole::TokenIds
            }
        );

        let mut overflow = requirements.ranges().to_vec();
        overflow[0] =
            M1StepWorkspaceRange::new(overflow[0].role(), u64::MAX - 3, 8, overflow[0].alignment());
        assert_eq!(
            reject(overflow),
            M1StepWorkspacePlanError::RangeOverflow {
                index: 0,
                role: M1StepWorkspaceRangeRole::TokenIds
            }
        );

        let mut out_of_bounds = requirements.ranges().to_vec();
        out_of_bounds[0] = M1StepWorkspaceRange::new(
            out_of_bounds[0].role(),
            requirements.allocation_byte_len(),
            out_of_bounds[0].byte_len(),
            out_of_bounds[0].alignment(),
        );
        assert_eq!(
            reject(out_of_bounds),
            M1StepWorkspacePlanError::RangeOutOfBounds {
                index: 0,
                role: M1StepWorkspaceRangeRole::TokenIds
            }
        );

        let mut alias = requirements.ranges().to_vec();
        alias[1] = M1StepWorkspaceRange::new(
            alias[1].role(),
            alias[0].offset(),
            alias[1].byte_len(),
            alias[1].alignment(),
        );
        assert_eq!(
            reject(alias),
            M1StepWorkspacePlanError::RangeAlias { left: 0, right: 1 }
        );

        let mut role = requirements.ranges().to_vec();
        role.swap(0, 1);
        assert_eq!(
            reject(role),
            M1StepWorkspacePlanError::RangeRole {
                index: 0,
                expected: M1StepWorkspaceRangeRole::TokenIds,
                actual: M1StepWorkspaceRangeRole::PositionIds
            }
        );

        let mut offset = requirements.ranges().to_vec();
        let last = offset.len() - 1;
        offset[last] = M1StepWorkspaceRange::new(
            offset[last].role(),
            offset[last].offset() + 64,
            offset[last].byte_len(),
            offset[last].alignment(),
        );
        let offset_error = reject(offset);
        assert!(
            matches!(
                offset_error,
                M1StepWorkspacePlanError::RangeOutOfBounds { .. }
            ) || matches!(
                offset_error,
                M1StepWorkspacePlanError::RangeOffset { index, .. } if index == last
            )
        );

        let mut length = requirements.ranges().to_vec();
        length[0] = M1StepWorkspaceRange::new(
            length[0].role(),
            length[0].offset(),
            length[0].byte_len() - 4,
            length[0].alignment(),
        );
        assert_eq!(
            reject(length),
            M1StepWorkspacePlanError::RangeLength {
                index: 0,
                expected: requirements.ranges()[0].byte_len(),
                actual: requirements.ranges()[0].byte_len() - 4
            }
        );
    }

    #[test]
    fn allocation_identity_changes_workspace_identity_without_changing_layout() {
        let selection = selection(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS8C8192,
        );
        let plan = |seed| {
            let requirements = m1_step_workspace_requirements(selection).unwrap();
            let available = AvailableM1StepWorkspace::new(M1StepWorkspaceDeclaration::new(
                selection,
                DeclaredM1StepWorkspaceAllocation::new(
                    identity(seed),
                    requirements.allocation_byte_len(),
                    64,
                ),
                requirements.ranges().to_vec().into_boxed_slice(),
            ));
            let M1StepWorkspacePlanOutcome::Planned(plan) =
                plan_addressless_m1_step_workspace(selection, available)
            else {
                panic!("exact layout rejected")
            };
            plan
        };
        let first = plan(10);
        let second = plan(11);
        assert_eq!(first.ranges(), second.ranges());
        assert_ne!(first.workspace_id(), second.workspace_id());
    }
}
