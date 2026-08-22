//! Addressless memory layout for admitted Qwen3 weights and global KV pools.
//!
//! This module consumes the authenticated model-weight layout and binds it to
//! four distinct caller-declared allocation identities and exact byte lengths.
//! It derives immutable weight, KV-layer, and KV-page byte ranges from the
//! Ferric-owned kernel geometry. It does not allocate memory, expose an
//! address, authenticate an allocation, initialize bytes, or grant load,
//! dispatch, completion, inference, hardware, or qualification authority.

use crate::{AuthenticatedModelWeightLayout, ModelWeightBinding, ModelWeightLayoutError};
use ferric_qwen_kernels::{paged_decode, prefill, rope_kv};
use ferric_spec::{
    Identity, PhysicalPageId, Qwen3ModelRole, RequestId, M1_KV_PAGE_TABLE_ENTRIES,
    M1_MAX_ACTIVE_SEQUENCES,
};
use std::fmt;

const BF16_BYTES: u64 = 2;

const _: () = {
    assert!(rope_kv::QWEN3_KV_PAGE_TOKENS_V1 == prefill::QWEN3_PREFILL_PAGE_TOKENS_V1);
    assert!(rope_kv::QWEN3_KV_PAGE_TOKENS_V1 == paged_decode::QWEN3_PAGED_DECODE_PAGE_TOKENS_V1);
    assert!(
        rope_kv::QWEN3_KV_PAGE_TABLE_ENTRIES_V1 == prefill::QWEN3_PREFILL_PAGE_TABLE_ENTRIES_V1
    );
    assert!(
        rope_kv::QWEN3_KV_PAGE_TABLE_ENTRIES_V1
            == paged_decode::QWEN3_PAGED_DECODE_PAGE_TABLE_ENTRIES_V1
    );
    assert!(rope_kv::QWEN3_KV_PHYSICAL_PAGE_SLOTS_V1 == prefill::QWEN3_PREFILL_CACHE_POOL_PAGES_V1);
    assert!(
        rope_kv::QWEN3_KV_PHYSICAL_PAGE_SLOTS_V1
            == paged_decode::QWEN3_PAGED_DECODE_CACHE_POOL_PAGES_V1
    );
    assert!(rope_kv::QWEN3_ROPE_KV_HEAD_DIMENSION_V1 == prefill::QWEN3_PREFILL_HEAD_DIMENSION_V1);
    assert!(
        rope_kv::QWEN3_ROPE_KV_HEAD_DIMENSION_V1
            == paged_decode::QWEN3_PAGED_DECODE_HEAD_DIMENSION_V1
    );
    assert!(
        rope_kv::QWEN3_KV_CACHE_BYTES_V1
            == rope_kv::QWEN3_KV_PHYSICAL_PAGE_SLOTS_V1 as u64 * QWEN3_KV_PAGE_BYTES_V1
    );
    assert!(
        rope_kv::QWEN3_KV_PHYSICAL_PAGE_SLOTS_V1 as usize
            == M1_MAX_ACTIVE_SEQUENCES as usize * M1_KV_PAGE_TABLE_ENTRIES
    );
};

/// Bytes in one P16, KVH8, D128 BF16 physical cache page.
pub const QWEN3_KV_PAGE_BYTES_V1: u64 = rope_kv::QWEN3_KV_PAGE_TOKENS_V1 as u64
    * 8
    * rope_kv::QWEN3_ROPE_KV_HEAD_DIMENSION_V1 as u64
    * BF16_BYTES;
/// Bytes in one layer's complete key and value cache planes.
pub const QWEN3_KV_LAYER_BYTES_V1: u64 = rope_kv::QWEN3_KV_CACHE_BYTES_V1 * 2;
/// Exact KFD-compatible base alignment for every model allocation.
pub const QWEN3_MODEL_MEMORY_ALLOCATION_ALIGNMENT_V1: u64 = 4_096;
/// Required base alignment for role-scoped KV arenas and all derived pages.
pub const QWEN3_KV_ARENA_ALIGNMENT_V1: u64 = QWEN3_MODEL_MEMORY_ALLOCATION_ALIGNMENT_V1;

/// Returns the exact byte length of one role-scoped, all-layer KV arena.
#[must_use]
pub const fn qwen3_kv_arena_bytes(role: Qwen3ModelRole) -> u64 {
    role.layers() as u64 * QWEN3_KV_LAYER_BYTES_V1
}

/// One caller-declared device allocation identity and exact byte length.
///
/// This value is copyable because it owns no allocation, mapping, address, or
/// initialized-memory authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeclaredDeviceAllocation {
    allocation_id: Identity,
    byte_len: u64,
    alignment: u64,
}

impl DeclaredDeviceAllocation {
    /// Creates inert allocation declaration data.
    #[must_use]
    pub const fn new(allocation_id: Identity, byte_len: u64, alignment: u64) -> Self {
        Self {
            allocation_id,
            byte_len,
            alignment,
        }
    }

    /// Returns the caller-declared allocation identity.
    #[must_use]
    pub const fn allocation_id(self) -> Identity {
        self.allocation_id
    }

    /// Returns the caller-declared allocation length.
    #[must_use]
    pub const fn byte_len(self) -> u64 {
        self.byte_len
    }

    /// Returns the caller-declared allocation-base alignment.
    #[must_use]
    pub const fn alignment(self) -> u64 {
        self.alignment
    }
}

/// Complete four-allocation declaration for target/draft weights and KV.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelMemoryAllocationSet {
    target_weights: DeclaredDeviceAllocation,
    draft_weights: DeclaredDeviceAllocation,
    target_kv: DeclaredDeviceAllocation,
    draft_kv: DeclaredDeviceAllocation,
}

impl ModelMemoryAllocationSet {
    /// Creates an unvalidated addressless declaration set.
    #[must_use]
    pub const fn new(
        target_weights: DeclaredDeviceAllocation,
        draft_weights: DeclaredDeviceAllocation,
        target_kv: DeclaredDeviceAllocation,
        draft_kv: DeclaredDeviceAllocation,
    ) -> Self {
        Self {
            target_weights,
            draft_weights,
            target_kv,
            draft_kv,
        }
    }

    /// Returns the declaration selected by role and memory kind.
    #[must_use]
    pub const fn get(
        self,
        role: Qwen3ModelRole,
        kind: ModelMemoryAllocationKind,
    ) -> DeclaredDeviceAllocation {
        match (role, kind) {
            (Qwen3ModelRole::Target8B, ModelMemoryAllocationKind::Weights) => self.target_weights,
            (Qwen3ModelRole::Draft06B, ModelMemoryAllocationKind::Weights) => self.draft_weights,
            (Qwen3ModelRole::Target8B, ModelMemoryAllocationKind::KvArena) => self.target_kv,
            (Qwen3ModelRole::Draft06B, ModelMemoryAllocationKind::KvArena) => self.draft_kv,
        }
    }

    const fn as_array(self) -> [DeclaredDeviceAllocation; 4] {
        [
            self.target_weights,
            self.draft_weights,
            self.target_kv,
            self.draft_kv,
        ]
    }
}

/// Weight or KV allocation role within the addressless model plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelMemoryAllocationKind {
    /// Complete prepacked BF16 weight image for one model role.
    Weights,
    /// Complete all-layer key and value cache arena for one model role.
    KvArena,
}

/// Key or value plane within a layer's global physical-page pool.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KvCacheComponent {
    /// Paged key-cache plane.
    Key,
    /// Paged value-cache plane.
    Value,
}

impl KvCacheComponent {
    const fn offset(self) -> u64 {
        match self {
            Self::Key => 0,
            Self::Value => rope_kv::QWEN3_KV_CACHE_BYTES_V1,
        }
    }
}

/// One checked subrange within caller-declared allocation data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeclaredMemoryRange {
    allocation_id: Identity,
    offset: u64,
    byte_len: u64,
}

impl DeclaredMemoryRange {
    /// Returns the containing caller-declared allocation identity.
    #[must_use]
    pub const fn allocation_id(self) -> Identity {
        self.allocation_id
    }

    /// Returns the byte offset from the declared allocation base.
    #[must_use]
    pub const fn offset(self) -> u64 {
        self.offset
    }

    /// Returns the exact range length.
    #[must_use]
    pub const fn byte_len(self) -> u64 {
        self.byte_len
    }

    /// Returns the exclusive byte end.
    #[must_use]
    pub const fn end(self) -> u64 {
        self.offset + self.byte_len
    }
}

/// Borrowed weight section joined to its exact declared-allocation subrange.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelWeightMemoryBinding<'a> {
    weight: ModelWeightBinding<'a>,
    range: DeclaredMemoryRange,
}

/// One request-local physical page bound to its global role-arena subrange.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelKvPageMemoryBinding {
    request: RequestId,
    page: PhysicalPageId,
    component: KvCacheComponent,
    layer: u32,
    global_page: u32,
    range: DeclaredMemoryRange,
}

impl ModelKvPageMemoryBinding {
    /// Returns the exact generational request owning the local page table.
    #[must_use]
    pub const fn request(self) -> RequestId {
        self.request
    }

    /// Returns the retained role, local page index, and page generation.
    #[must_use]
    pub const fn page(self) -> PhysicalPageId {
        self.page
    }

    /// Returns the selected key or value cache plane.
    #[must_use]
    pub const fn component(self) -> KvCacheComponent {
        self.component
    }

    /// Returns the selected transformer layer.
    #[must_use]
    pub const fn layer(self) -> u32 {
        self.layer
    }

    /// Returns the exact global pool slot derived from request slot and local page.
    #[must_use]
    pub const fn global_page(self) -> u32 {
        self.global_page
    }

    /// Returns the exact addressless role-arena page range.
    #[must_use]
    pub const fn range(self) -> DeclaredMemoryRange {
        self.range
    }
}

impl<'a> ModelWeightMemoryBinding<'a> {
    /// Returns the typed retained manifest binding.
    #[must_use]
    pub const fn weight(self) -> ModelWeightBinding<'a> {
        self.weight
    }

    /// Returns the exact addressless allocation subrange.
    #[must_use]
    pub const fn range(self) -> DeclaredMemoryRange {
        self.range
    }
}

/// Fail-closed model-memory declaration or lookup error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelMemoryPlanError {
    /// One required declaration has an absent identity.
    MissingAllocationIdentity {
        /// Model role owning the rejected declaration.
        role: Qwen3ModelRole,
        /// Allocation purpose.
        kind: ModelMemoryAllocationKind,
    },
    /// Two logically distinct allocations reuse one identity.
    AllocationAlias,
    /// A declared allocation length differs from the exact required length.
    AllocationLength {
        /// Model role owning the rejected declaration.
        role: Qwen3ModelRole,
        /// Allocation purpose.
        kind: ModelMemoryAllocationKind,
        /// Exact required bytes.
        expected: u64,
        /// Rejected caller-declared bytes.
        actual: u64,
    },
    /// A declared allocation has the wrong exact KFD-compatible alignment.
    AllocationAlignment {
        /// Model role owning the rejected declaration.
        role: Qwen3ModelRole,
        /// Allocation purpose.
        kind: ModelMemoryAllocationKind,
        /// Exact required alignment.
        expected: u64,
        /// Rejected caller-declared alignment.
        actual: u64,
    },
    /// A requested transformer layer is outside the selected role.
    LayerOutOfRange {
        /// Selected model role.
        role: Qwen3ModelRole,
        /// Rejected layer.
        layer: u32,
    },
    /// A requested global physical page slot is outside the kernel pool.
    PageOutOfRange {
        /// Rejected physical page slot.
        page: u32,
    },
    /// A generational request names a slot outside the fixed global pool partition.
    RequestSlotOutOfRange {
        /// Rejected request slot.
        slot: u32,
    },
    /// A request-local physical page exceeds its 512-entry page table.
    RequestPageOutOfRange {
        /// Rejected request-local page index.
        page: u32,
    },
    /// A request generation of zero cannot identify live scheduler custody.
    ZeroRequestGeneration,
    /// A physical-page generation of zero cannot identify live KV custody.
    ZeroPageGeneration,
    /// Checked range arithmetic exceeded the declared allocation.
    RangeOverflow,
}

impl fmt::Display for ModelMemoryPlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "addressless model memory plan rejected: {self:?}"
        )
    }
}

impl std::error::Error for ModelMemoryPlanError {}

/// Rejected planning attempt retaining the exact linear model layout.
#[derive(Debug, PartialEq, Eq)]
pub struct ModelMemoryPlanFailure {
    error: ModelMemoryPlanError,
    layout: AuthenticatedModelWeightLayout,
    declarations: ModelMemoryAllocationSet,
}

impl ModelMemoryPlanFailure {
    /// Returns the fail-closed diagnostic.
    #[must_use]
    pub const fn error(&self) -> ModelMemoryPlanError {
        self.error
    }

    /// Recovers the unchanged authenticated layout and declaration set.
    #[must_use]
    pub fn into_parts(
        self,
    ) -> (
        ModelMemoryPlanError,
        AuthenticatedModelWeightLayout,
        ModelMemoryAllocationSet,
    ) {
        (self.error, self.layout, self.declarations)
    }
}

/// Linear addressless plan for exact model weights and role-scoped KV arenas.
///
/// This value is intentionally not `Clone`. It retains the authenticated
/// model layout but no fe2o3 allocation lease or address authority.
#[derive(Debug, PartialEq, Eq)]
pub struct AddresslessModelMemoryPlan {
    layout: AuthenticatedModelWeightLayout,
    declarations: ModelMemoryAllocationSet,
}

impl AddresslessModelMemoryPlan {
    /// Returns the retained authenticated model-weight layout.
    #[must_use]
    pub const fn weight_layout(&self) -> &AuthenticatedModelWeightLayout {
        &self.layout
    }

    /// Returns one exact retained allocation declaration.
    #[must_use]
    pub const fn allocation(
        &self,
        role: Qwen3ModelRole,
        kind: ModelMemoryAllocationKind,
    ) -> DeclaredDeviceAllocation {
        self.declarations.get(role, kind)
    }

    /// Resolves one typed weight ordinal to its declared allocation subrange.
    ///
    /// # Errors
    ///
    /// Returns [`ModelWeightLayoutError`] if the retained ordinal map fails its
    /// exact bounds or immutable consistency checks.
    pub fn weight_by_ordinal(
        &self,
        role: Qwen3ModelRole,
        ordinal: u32,
    ) -> Result<ModelWeightMemoryBinding<'_>, ModelWeightLayoutError> {
        let weight = self.layout.by_ordinal(role, ordinal)?;
        let (offset, byte_len) = weight.destination_range();
        Ok(ModelWeightMemoryBinding {
            weight,
            range: DeclaredMemoryRange {
                allocation_id: self
                    .declarations
                    .get(role, ModelMemoryAllocationKind::Weights)
                    .allocation_id,
                offset,
                byte_len,
            },
        })
    }

    /// Resolves one complete layer key or value cache plane.
    ///
    /// # Errors
    ///
    /// Returns [`ModelMemoryPlanError`] for an out-of-range layer or checked
    /// arithmetic/range failure.
    pub fn kv_layer(
        &self,
        role: Qwen3ModelRole,
        component: KvCacheComponent,
        layer: u32,
    ) -> Result<DeclaredMemoryRange, ModelMemoryPlanError> {
        if layer >= role.layers() {
            return Err(ModelMemoryPlanError::LayerOutOfRange { role, layer });
        }
        let allocation = self
            .declarations
            .get(role, ModelMemoryAllocationKind::KvArena);
        let offset = u64::from(layer)
            .checked_mul(QWEN3_KV_LAYER_BYTES_V1)
            .and_then(|value| value.checked_add(component.offset()))
            .ok_or(ModelMemoryPlanError::RangeOverflow)?;
        checked_range(allocation, offset, rope_kv::QWEN3_KV_CACHE_BYTES_V1)
    }

    /// Resolves one fixed physical P16 page within a layer cache plane.
    ///
    /// # Errors
    ///
    /// Returns [`ModelMemoryPlanError`] for an invalid layer/page or checked
    /// arithmetic/range failure.
    pub fn kv_page(
        &self,
        role: Qwen3ModelRole,
        component: KvCacheComponent,
        layer: u32,
        page: u32,
    ) -> Result<DeclaredMemoryRange, ModelMemoryPlanError> {
        if page >= rope_kv::QWEN3_KV_PHYSICAL_PAGE_SLOTS_V1 {
            return Err(ModelMemoryPlanError::PageOutOfRange { page });
        }
        let layer_range = self.kv_layer(role, component, layer)?;
        let page_offset = u64::from(page)
            .checked_mul(QWEN3_KV_PAGE_BYTES_V1)
            .and_then(|value| layer_range.offset.checked_add(value))
            .ok_or(ModelMemoryPlanError::RangeOverflow)?;
        checked_range(
            self.declarations
                .get(role, ModelMemoryAllocationKind::KvArena),
            page_offset,
            QWEN3_KV_PAGE_BYTES_V1,
        )
    }

    /// Resolves one request-local physical page into its disjoint global pool slot.
    ///
    /// The mapping is `request.slot * 512 + page.index`. Request generation and
    /// physical-page generation remain retained in the returned binding but do
    /// not change the byte address. The engine must still enforce retirement
    /// before either generation can be reused.
    ///
    /// # Errors
    ///
    /// Returns [`ModelMemoryPlanError`] for a zero generation, out-of-range
    /// request slot, request-local page, layer, or checked range computation.
    pub fn kv_request_page(
        &self,
        request: RequestId,
        page: PhysicalPageId,
        component: KvCacheComponent,
        layer: u32,
    ) -> Result<ModelKvPageMemoryBinding, ModelMemoryPlanError> {
        if request.generation() == 0 {
            return Err(ModelMemoryPlanError::ZeroRequestGeneration);
        }
        if page.generation() == 0 {
            return Err(ModelMemoryPlanError::ZeroPageGeneration);
        }
        if request.slot() >= M1_MAX_ACTIVE_SEQUENCES {
            return Err(ModelMemoryPlanError::RequestSlotOutOfRange {
                slot: request.slot(),
            });
        }
        let local_page = page.index();
        let page_table_entries = u32::try_from(M1_KV_PAGE_TABLE_ENTRIES)
            .map_err(|_| ModelMemoryPlanError::RangeOverflow)?;
        if local_page >= page_table_entries {
            return Err(ModelMemoryPlanError::RequestPageOutOfRange { page: local_page });
        }
        let global_page = request
            .slot()
            .checked_mul(page_table_entries)
            .and_then(|base| base.checked_add(local_page))
            .ok_or(ModelMemoryPlanError::RangeOverflow)?;
        let range = self.kv_page(page.role(), component, layer, global_page)?;
        Ok(ModelKvPageMemoryBinding {
            request,
            page,
            component,
            layer,
            global_page,
            range,
        })
    }
}

/// Linear result of one model-memory planning attempt.
#[must_use]
#[derive(Debug, PartialEq, Eq)]
pub enum ModelMemoryPlanOutcome {
    /// Exact declarations were retained in an addressless model plan.
    Planned(AddresslessModelMemoryPlan),
    /// The unchanged linear layout and declarations remain recoverable.
    Rejected(ModelMemoryPlanFailure),
}

/// Consumes an authenticated weight layout into an exact addressless model and
/// KV memory plan.
///
/// Success validates declaration identities and lengths only. A future engine
/// stage must join each declaration to a distinct generic fe2o3 allocation
/// lease with exact initialized-range custody before any device address or
/// dispatch may be constructed.
pub fn plan_authenticated_model_memory(
    layout: AuthenticatedModelWeightLayout,
    declarations: ModelMemoryAllocationSet,
) -> ModelMemoryPlanOutcome {
    match validate_declarations(declarations) {
        Ok(()) => ModelMemoryPlanOutcome::Planned(AddresslessModelMemoryPlan {
            layout,
            declarations,
        }),
        Err(error) => ModelMemoryPlanOutcome::Rejected(ModelMemoryPlanFailure {
            error,
            layout,
            declarations,
        }),
    }
}

/// Builds the compact exact model-memory fixture for cross-crate tests.
///
/// This helper exists only under the `test-fixtures` feature. It grants no
/// deployed model-content, allocation, initialization, or dispatch authority.
#[cfg(feature = "test-fixtures")]
#[doc(hidden)]
#[must_use]
pub fn qwen3_model_memory_plan_test_fixture() -> AddresslessModelMemoryPlan {
    use crate::{
        build_authenticated_model_weight_layout, build_prepacked_deployment_bundle,
        seal_authenticated_bundle, tokenizer::test_fixtures::authenticated_assets,
        tokenizer::test_fixtures::test_tokenizer, weight_stream::test_fixtures::test_prepacked,
    };

    const fn fixture_identity(byte: u8) -> Identity {
        Identity::new([byte; 32])
    }

    let prepacked = build_prepacked_deployment_bundle(
        authenticated_assets(),
        test_tokenizer(Qwen3ModelRole::Target8B),
        test_tokenizer(Qwen3ModelRole::Draft06B),
        test_prepacked(Qwen3ModelRole::Target8B),
        test_prepacked(Qwen3ModelRole::Draft06B),
    )
    .expect("exact compact prepacked fixture");
    let admission = seal_authenticated_bundle(prepacked).expect("sealed compact fixture");
    let layout =
        build_authenticated_model_weight_layout(admission).expect("exact model layout fixture");
    let declarations = ModelMemoryAllocationSet::new(
        DeclaredDeviceAllocation::new(
            fixture_identity(1),
            Qwen3ModelRole::Target8B.tensor_data_bytes(),
            QWEN3_MODEL_MEMORY_ALLOCATION_ALIGNMENT_V1,
        ),
        DeclaredDeviceAllocation::new(
            fixture_identity(2),
            Qwen3ModelRole::Draft06B.tensor_data_bytes(),
            QWEN3_MODEL_MEMORY_ALLOCATION_ALIGNMENT_V1,
        ),
        DeclaredDeviceAllocation::new(
            fixture_identity(3),
            qwen3_kv_arena_bytes(Qwen3ModelRole::Target8B),
            QWEN3_MODEL_MEMORY_ALLOCATION_ALIGNMENT_V1,
        ),
        DeclaredDeviceAllocation::new(
            fixture_identity(4),
            qwen3_kv_arena_bytes(Qwen3ModelRole::Draft06B),
            QWEN3_MODEL_MEMORY_ALLOCATION_ALIGNMENT_V1,
        ),
    );
    match plan_authenticated_model_memory(layout, declarations) {
        ModelMemoryPlanOutcome::Planned(plan) => plan,
        ModelMemoryPlanOutcome::Rejected(failure) => {
            panic!("exact model-memory fixture rejected: {:?}", failure.error())
        }
    }
}

fn validate_declarations(
    declarations: ModelMemoryAllocationSet,
) -> Result<(), ModelMemoryPlanError> {
    for (role, kind) in [
        (Qwen3ModelRole::Target8B, ModelMemoryAllocationKind::Weights),
        (Qwen3ModelRole::Draft06B, ModelMemoryAllocationKind::Weights),
        (Qwen3ModelRole::Target8B, ModelMemoryAllocationKind::KvArena),
        (Qwen3ModelRole::Draft06B, ModelMemoryAllocationKind::KvArena),
    ] {
        let allocation = declarations.get(role, kind);
        if !allocation.allocation_id.is_present() {
            return Err(ModelMemoryPlanError::MissingAllocationIdentity { role, kind });
        }
        let expected = match kind {
            ModelMemoryAllocationKind::Weights => role.tensor_data_bytes(),
            ModelMemoryAllocationKind::KvArena => qwen3_kv_arena_bytes(role),
        };
        if allocation.byte_len != expected {
            return Err(ModelMemoryPlanError::AllocationLength {
                role,
                kind,
                expected,
                actual: allocation.byte_len,
            });
        }
        if allocation.alignment != QWEN3_MODEL_MEMORY_ALLOCATION_ALIGNMENT_V1 {
            return Err(ModelMemoryPlanError::AllocationAlignment {
                role,
                kind,
                expected: QWEN3_MODEL_MEMORY_ALLOCATION_ALIGNMENT_V1,
                actual: allocation.alignment,
            });
        }
    }

    let allocations = declarations.as_array();
    for (left_index, left) in allocations.iter().enumerate() {
        for right in &allocations[left_index + 1..] {
            if left.allocation_id.equals(&right.allocation_id) {
                return Err(ModelMemoryPlanError::AllocationAlias);
            }
        }
    }
    Ok(())
}

fn checked_range(
    allocation: DeclaredDeviceAllocation,
    offset: u64,
    byte_len: u64,
) -> Result<DeclaredMemoryRange, ModelMemoryPlanError> {
    let end = offset
        .checked_add(byte_len)
        .ok_or(ModelMemoryPlanError::RangeOverflow)?;
    if end > allocation.byte_len {
        return Err(ModelMemoryPlanError::RangeOverflow);
    }
    Ok(DeclaredMemoryRange {
        allocation_id: allocation.allocation_id,
        offset,
        byte_len,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        plan_authenticated_model_memory, qwen3_kv_arena_bytes, AddresslessModelMemoryPlan,
        DeclaredDeviceAllocation, KvCacheComponent, ModelMemoryAllocationKind,
        ModelMemoryAllocationSet, ModelMemoryPlanError, ModelMemoryPlanOutcome,
        QWEN3_KV_ARENA_ALIGNMENT_V1, QWEN3_KV_LAYER_BYTES_V1, QWEN3_KV_PAGE_BYTES_V1,
        QWEN3_MODEL_MEMORY_ALLOCATION_ALIGNMENT_V1,
    };
    use crate::{
        build_authenticated_model_weight_layout, build_prepacked_deployment_bundle,
        seal_authenticated_bundle,
        tokenizer::tests::{authenticated_assets, test_tokenizer},
        weight_stream::tests::test_prepacked,
    };
    use ferric_qwen_kernels::rope_kv::{QWEN3_KV_CACHE_BYTES_V1, QWEN3_KV_PHYSICAL_PAGE_SLOTS_V1};
    use ferric_spec::{
        Identity, PhysicalPageId, Qwen3ModelRole, RequestId, M1_KV_PAGE_TABLE_ENTRIES,
        M1_MAX_ACTIVE_SEQUENCES,
    };

    const fn identity(byte: u8) -> Identity {
        Identity::new([byte; 32])
    }

    fn layout() -> crate::AuthenticatedModelWeightLayout {
        let prepacked = build_prepacked_deployment_bundle(
            authenticated_assets(),
            test_tokenizer(Qwen3ModelRole::Target8B),
            test_tokenizer(Qwen3ModelRole::Draft06B),
            test_prepacked(Qwen3ModelRole::Target8B),
            test_prepacked(Qwen3ModelRole::Draft06B),
        )
        .expect("official fixture deployment");
        let admission = seal_authenticated_bundle(prepacked).expect("official admission");
        build_authenticated_model_weight_layout(admission).expect("official model layout")
    }

    fn declarations() -> ModelMemoryAllocationSet {
        ModelMemoryAllocationSet::new(
            DeclaredDeviceAllocation::new(
                identity(1),
                Qwen3ModelRole::Target8B.tensor_data_bytes(),
                QWEN3_MODEL_MEMORY_ALLOCATION_ALIGNMENT_V1,
            ),
            DeclaredDeviceAllocation::new(
                identity(2),
                Qwen3ModelRole::Draft06B.tensor_data_bytes(),
                QWEN3_MODEL_MEMORY_ALLOCATION_ALIGNMENT_V1,
            ),
            DeclaredDeviceAllocation::new(
                identity(3),
                qwen3_kv_arena_bytes(Qwen3ModelRole::Target8B),
                QWEN3_MODEL_MEMORY_ALLOCATION_ALIGNMENT_V1,
            ),
            DeclaredDeviceAllocation::new(
                identity(4),
                qwen3_kv_arena_bytes(Qwen3ModelRole::Draft06B),
                QWEN3_MODEL_MEMORY_ALLOCATION_ALIGNMENT_V1,
            ),
        )
    }

    fn exact_plan() -> AddresslessModelMemoryPlan {
        match plan_authenticated_model_memory(layout(), declarations()) {
            ModelMemoryPlanOutcome::Planned(plan) => plan,
            ModelMemoryPlanOutcome::Rejected(failure) => {
                panic!("exact declaration rejected: {:?}", failure.error())
            }
        }
    }

    #[test]
    fn exact_plan_covers_all_weights_layers_components_and_page_boundaries() {
        let plan = exact_plan();
        assert_eq!(
            qwen3_kv_arena_bytes(Qwen3ModelRole::Target8B),
            38_654_705_664
        );
        assert_eq!(
            qwen3_kv_arena_bytes(Qwen3ModelRole::Draft06B),
            30_064_771_072
        );
        assert_eq!(QWEN3_KV_PAGE_BYTES_V1, 32_768);
        assert_eq!(QWEN3_KV_ARENA_ALIGNMENT_V1, 4_096);

        for role in [Qwen3ModelRole::Target8B, Qwen3ModelRole::Draft06B] {
            let weight_allocation = plan.allocation(role, ModelMemoryAllocationKind::Weights);
            assert_eq!(weight_allocation.byte_len(), role.tensor_data_bytes());
            assert_eq!(
                weight_allocation.alignment(),
                QWEN3_MODEL_MEMORY_ALLOCATION_ALIGNMENT_V1
            );
            for ordinal in 0..role.tensor_count() {
                let binding = plan
                    .weight_by_ordinal(role, ordinal)
                    .expect("complete retained weight map");
                assert_eq!(binding.weight().ordinal(), ordinal);
                assert_eq!(
                    binding.range().allocation_id(),
                    weight_allocation.allocation_id()
                );
                assert_eq!(
                    (binding.range().offset(), binding.range().byte_len()),
                    binding.weight().destination_range()
                );
                assert!(binding.range().end() <= weight_allocation.byte_len());
            }

            let kv_allocation = plan.allocation(role, ModelMemoryAllocationKind::KvArena);
            assert_eq!(kv_allocation.byte_len(), qwen3_kv_arena_bytes(role));
            assert_eq!(kv_allocation.alignment(), QWEN3_KV_ARENA_ALIGNMENT_V1);
            for layer in 0..role.layers() {
                let key = plan
                    .kv_layer(role, KvCacheComponent::Key, layer)
                    .expect("key plane");
                let value = plan
                    .kv_layer(role, KvCacheComponent::Value, layer)
                    .expect("value plane");
                assert_eq!(key.offset(), u64::from(layer) * QWEN3_KV_LAYER_BYTES_V1);
                assert_eq!(key.end(), value.offset());
                assert_eq!(value.byte_len(), QWEN3_KV_CACHE_BYTES_V1);

                for page in [0, QWEN3_KV_PHYSICAL_PAGE_SLOTS_V1 - 1] {
                    let key_page = plan
                        .kv_page(role, KvCacheComponent::Key, layer, page)
                        .expect("key page");
                    let value_page = plan
                        .kv_page(role, KvCacheComponent::Value, layer, page)
                        .expect("value page");
                    assert_eq!(key_page.byte_len(), QWEN3_KV_PAGE_BYTES_V1);
                    assert_eq!(
                        key_page.offset(),
                        key.offset() + u64::from(page) * QWEN3_KV_PAGE_BYTES_V1
                    );
                    assert_eq!(
                        value_page.offset(),
                        value.offset() + u64::from(page) * QWEN3_KV_PAGE_BYTES_V1
                    );
                }
            }
            let final_value = plan
                .kv_layer(role, KvCacheComponent::Value, role.layers() - 1)
                .expect("final value plane");
            assert_eq!(final_value.end(), kv_allocation.byte_len());
        }
    }

    #[test]
    fn absent_alias_and_each_exact_length_drift_retain_linear_inputs() {
        let mut cases = Vec::new();
        let exact = declarations();
        let mut values = exact.as_array();
        values[0] = DeclaredDeviceAllocation::new(
            Identity::new([0; 32]),
            values[0].byte_len(),
            values[0].alignment(),
        );
        cases.push((
            ModelMemoryAllocationSet::new(values[0], values[1], values[2], values[3]),
            ModelMemoryPlanError::MissingAllocationIdentity {
                role: Qwen3ModelRole::Target8B,
                kind: ModelMemoryAllocationKind::Weights,
            },
        ));

        for left in 0..4 {
            for right in left + 1..4 {
                let mut aliased = exact.as_array();
                aliased[right] = DeclaredDeviceAllocation::new(
                    aliased[left].allocation_id(),
                    aliased[right].byte_len(),
                    aliased[right].alignment(),
                );
                cases.push((
                    ModelMemoryAllocationSet::new(aliased[0], aliased[1], aliased[2], aliased[3]),
                    ModelMemoryPlanError::AllocationAlias,
                ));
            }
        }

        let slots = [
            (Qwen3ModelRole::Target8B, ModelMemoryAllocationKind::Weights),
            (Qwen3ModelRole::Draft06B, ModelMemoryAllocationKind::Weights),
            (Qwen3ModelRole::Target8B, ModelMemoryAllocationKind::KvArena),
            (Qwen3ModelRole::Draft06B, ModelMemoryAllocationKind::KvArena),
        ];
        for (index, (role, kind)) in slots.iter().copied().enumerate() {
            let mut changed = exact.as_array();
            let expected = changed[index].byte_len();
            changed[index] = DeclaredDeviceAllocation::new(
                changed[index].allocation_id(),
                expected - 1,
                changed[index].alignment(),
            );
            cases.push((
                ModelMemoryAllocationSet::new(changed[0], changed[1], changed[2], changed[3]),
                ModelMemoryPlanError::AllocationLength {
                    role,
                    kind,
                    expected,
                    actual: expected - 1,
                },
            ));
        }
        for (index, (role, kind)) in slots.iter().copied().enumerate() {
            let mut changed = exact.as_array();
            changed[index] = DeclaredDeviceAllocation::new(
                changed[index].allocation_id(),
                changed[index].byte_len(),
                QWEN3_MODEL_MEMORY_ALLOCATION_ALIGNMENT_V1 / 2,
            );
            cases.push((
                ModelMemoryAllocationSet::new(changed[0], changed[1], changed[2], changed[3]),
                ModelMemoryPlanError::AllocationAlignment {
                    role,
                    kind,
                    expected: QWEN3_MODEL_MEMORY_ALLOCATION_ALIGNMENT_V1,
                    actual: QWEN3_MODEL_MEMORY_ALLOCATION_ALIGNMENT_V1 / 2,
                },
            ));
        }

        for (declarations, expected_error) in cases {
            let source_layout = layout();
            let record_id = source_layout.admission().record().record_id();
            let failure = match plan_authenticated_model_memory(source_layout, declarations) {
                ModelMemoryPlanOutcome::Rejected(failure) => failure,
                ModelMemoryPlanOutcome::Planned(_) => panic!("invalid declaration accepted"),
            };
            assert_eq!(failure.error(), expected_error);
            let (actual_error, recovered, recovered_declarations) = failure.into_parts();
            assert_eq!(actual_error, expected_error);
            assert_eq!(recovered.admission().record().record_id(), record_id);
            assert_eq!(recovered_declarations, declarations);
        }
    }

    #[test]
    fn layer_and_page_bounds_fail_closed() {
        let plan = exact_plan();
        for role in [Qwen3ModelRole::Target8B, Qwen3ModelRole::Draft06B] {
            assert_eq!(
                plan.kv_layer(role, KvCacheComponent::Key, role.layers()),
                Err(ModelMemoryPlanError::LayerOutOfRange {
                    role,
                    layer: role.layers(),
                })
            );
            assert_eq!(
                plan.kv_page(
                    role,
                    KvCacheComponent::Value,
                    0,
                    QWEN3_KV_PHYSICAL_PAGE_SLOTS_V1,
                ),
                Err(ModelMemoryPlanError::PageOutOfRange {
                    page: QWEN3_KV_PHYSICAL_PAGE_SLOTS_V1,
                })
            );
        }
    }

    #[test]
    fn request_local_pages_partition_the_complete_global_pool_without_aliasing() {
        let plan = exact_plan();
        let local_last = u32::try_from(M1_KV_PAGE_TABLE_ENTRIES - 1).unwrap();
        for role in [Qwen3ModelRole::Target8B, Qwen3ModelRole::Draft06B] {
            for slot in 0..M1_MAX_ACTIVE_SEQUENCES {
                let request = RequestId::new(slot, slot + 1);
                for local_page in [0, local_last] {
                    let page = PhysicalPageId::new(role, local_page, slot + 2);
                    let binding = plan
                        .kv_request_page(request, page, KvCacheComponent::Key, role.layers() - 1)
                        .expect("request-local page maps into the global role pool");
                    let expected_global =
                        slot * u32::try_from(M1_KV_PAGE_TABLE_ENTRIES).unwrap() + local_page;
                    assert_eq!(binding.request(), request);
                    assert_eq!(binding.page(), page);
                    assert_eq!(binding.component(), KvCacheComponent::Key);
                    assert_eq!(binding.layer(), role.layers() - 1);
                    assert_eq!(binding.global_page(), expected_global);
                    assert_eq!(
                        binding.range(),
                        plan.kv_page(
                            role,
                            KvCacheComponent::Key,
                            role.layers() - 1,
                            expected_global,
                        )
                        .unwrap()
                    );
                }
            }
        }

        let final_binding = plan
            .kv_request_page(
                RequestId::new(M1_MAX_ACTIVE_SEQUENCES - 1, 9),
                PhysicalPageId::new(Qwen3ModelRole::Target8B, local_last, 11),
                KvCacheComponent::Value,
                Qwen3ModelRole::Target8B.layers() - 1,
            )
            .unwrap();
        assert_eq!(
            final_binding.global_page(),
            QWEN3_KV_PHYSICAL_PAGE_SLOTS_V1 - 1
        );
        assert_eq!(
            final_binding.range().end(),
            qwen3_kv_arena_bytes(Qwen3ModelRole::Target8B)
        );
    }

    #[test]
    fn request_and_page_generation_and_bounds_fail_closed() {
        let plan = exact_plan();
        let role = Qwen3ModelRole::Draft06B;
        assert_eq!(
            plan.kv_request_page(
                RequestId::new(M1_MAX_ACTIVE_SEQUENCES, 1),
                PhysicalPageId::new(role, 0, 1),
                KvCacheComponent::Key,
                0,
            ),
            Err(ModelMemoryPlanError::RequestSlotOutOfRange {
                slot: M1_MAX_ACTIVE_SEQUENCES,
            })
        );
        let local_out_of_range = u32::try_from(M1_KV_PAGE_TABLE_ENTRIES).unwrap();
        assert_eq!(
            plan.kv_request_page(
                RequestId::new(0, 1),
                PhysicalPageId::new(role, local_out_of_range, 1),
                KvCacheComponent::Value,
                0,
            ),
            Err(ModelMemoryPlanError::RequestPageOutOfRange {
                page: local_out_of_range,
            })
        );
        assert_eq!(
            plan.kv_request_page(
                RequestId::new(0, 0),
                PhysicalPageId::new(role, 0, 1),
                KvCacheComponent::Key,
                0,
            ),
            Err(ModelMemoryPlanError::ZeroRequestGeneration)
        );
        assert_eq!(
            plan.kv_request_page(
                RequestId::new(0, 1),
                PhysicalPageId::new(role, 0, 0),
                KvCacheComponent::Key,
                0,
            ),
            Err(ModelMemoryPlanError::ZeroPageGeneration)
        );
    }
}
