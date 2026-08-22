//! Typed service-allocation binding for one addressless model-memory plan.
//!
//! This module retains Ferric's exact target/draft weight and KV declarations
//! beside generic fe2o3 allocation keys. It only produces owner-checked,
//! addressless dispatch ranges. It does not allocate or initialize memory,
//! expose a native address, construct a packet, publish a queue, launch work,
//! authenticate content, or claim execution, hardware, or performance results.

use core::fmt;

use fe2o3_service_host::{
    DeviceInputRoleV1, DeviceLocalAllocationV1, DeviceStateRoleV1, ServiceAllocationErrorV1,
    ServiceAllocationKeyV1, ServiceAllocationSessionV1, ServiceDeviceDispatchRangeV1,
};
use ferric_build::{
    qwen3_kv_arena_bytes, AddresslessModelMemoryPlan, DeclaredDeviceAllocation,
    DeclaredMemoryRange, KvCacheComponent, ModelMemoryAllocationKind, ModelMemoryPlanError,
    ModelWeightLayoutError, QWEN3_KV_ARENA_ALIGNMENT_V1,
    QWEN3_MODEL_MEMORY_ALLOCATION_ALIGNMENT_V1,
};
use ferric_spec::{Identity, Qwen3ModelRole, Qwen3TensorKind};

type WeightAllocationKeyV1 = ServiceAllocationKeyV1<DeviceInputRoleV1, DeviceLocalAllocationV1>;
type KvAllocationKeyV1 = ServiceAllocationKeyV1<DeviceStateRoleV1, DeviceLocalAllocationV1>;

const MODEL_MEMORY_SLOTS_V1: [(Qwen3ModelRole, ModelMemoryAllocationKind); 4] = [
    (Qwen3ModelRole::Target8B, ModelMemoryAllocationKind::Weights),
    (Qwen3ModelRole::Draft06B, ModelMemoryAllocationKind::Weights),
    (Qwen3ModelRole::Target8B, ModelMemoryAllocationKind::KvArena),
    (Qwen3ModelRole::Draft06B, ModelMemoryAllocationKind::KvArena),
];

/// Four caller-selected inert allocation identities in exact role/kind order.
///
/// These identities join Ferric declarations to service keys without treating
/// either value as native allocation identity or address authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelectedModelMemoryAllocationIdentitiesV1 {
    target_weights: Identity,
    draft_weights: Identity,
    target_kv: Identity,
    draft_kv: Identity,
}

impl SelectedModelMemoryAllocationIdentitiesV1 {
    /// Creates an unvalidated exact-slot selection.
    #[must_use]
    pub const fn new(
        target_weights: Identity,
        draft_weights: Identity,
        target_kv: Identity,
        draft_kv: Identity,
    ) -> Self {
        Self {
            target_weights,
            draft_weights,
            target_kv,
            draft_kv,
        }
    }

    /// Returns the identity selected for one exact model-memory coordinate.
    #[must_use]
    pub const fn get(self, role: Qwen3ModelRole, kind: ModelMemoryAllocationKind) -> Identity {
        match (role, kind) {
            (Qwen3ModelRole::Target8B, ModelMemoryAllocationKind::Weights) => self.target_weights,
            (Qwen3ModelRole::Draft06B, ModelMemoryAllocationKind::Weights) => self.draft_weights,
            (Qwen3ModelRole::Target8B, ModelMemoryAllocationKind::KvArena) => self.target_kv,
            (Qwen3ModelRole::Draft06B, ModelMemoryAllocationKind::KvArena) => self.draft_kv,
        }
    }

    const fn as_array(self) -> [Identity; 4] {
        [
            self.target_weights,
            self.draft_weights,
            self.target_kv,
            self.draft_kv,
        ]
    }
}

/// Fail-closed model-memory service binding error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelMemoryAllocationBindingErrorV1 {
    /// A retained plan allocation has an absent identity.
    MissingPlanAllocationIdentity {
        /// Model role owning the allocation.
        role: Qwen3ModelRole,
        /// Allocation purpose.
        kind: ModelMemoryAllocationKind,
    },
    /// A caller-selected allocation has an absent identity.
    MissingSelectedAllocationIdentity {
        /// Model role owning the allocation.
        role: Qwen3ModelRole,
        /// Allocation purpose.
        kind: ModelMemoryAllocationKind,
    },
    /// A caller-selected identity differs from the exact plan coordinate.
    SelectedAllocationIdentityDrift {
        /// Model role owning the allocation.
        role: Qwen3ModelRole,
        /// Allocation purpose.
        kind: ModelMemoryAllocationKind,
    },
    /// Two exact role/kind coordinates reuse one selected identity.
    AllocationIdentityAlias {
        /// Earlier exact coordinate.
        left: (Qwen3ModelRole, ModelMemoryAllocationKind),
        /// Later exact coordinate.
        right: (Qwen3ModelRole, ModelMemoryAllocationKind),
    },
    /// A retained plan allocation has a noncanonical extent.
    PlanAllocationExtentDrift {
        /// Model role owning the allocation.
        role: Qwen3ModelRole,
        /// Allocation purpose.
        kind: ModelMemoryAllocationKind,
        /// Canonical required bytes.
        expected: u64,
        /// Rejected retained bytes.
        actual: u64,
    },
    /// A retained plan allocation does not declare exact 4096-byte alignment.
    PlanAllocationAlignmentDrift {
        /// Model role owning the allocation.
        role: Qwen3ModelRole,
        /// Allocation purpose.
        kind: ModelMemoryAllocationKind,
        /// Exact required alignment.
        expected: u64,
        /// Rejected retained alignment.
        actual: u64,
    },
    /// A service key extent differs from its exact plan allocation.
    ServiceAllocationExtentDrift {
        /// Model role owning the allocation.
        role: Qwen3ModelRole,
        /// Allocation purpose.
        kind: ModelMemoryAllocationKind,
        /// Exact retained plan bytes.
        expected: u64,
        /// Rejected service-key bytes.
        actual: u64,
    },
    /// A service key does not declare exact 4096-byte base alignment.
    ServiceAllocationAlignmentDrift {
        /// Model role owning the allocation.
        role: Qwen3ModelRole,
        /// Allocation purpose.
        kind: ModelMemoryAllocationKind,
        /// Exact required alignment.
        expected: u64,
        /// Rejected service-key alignment.
        actual: u64,
    },
    /// A canonically resolved subrange names another selected allocation.
    ResolvedRangeIdentityDrift {
        /// Model role owning the allocation.
        role: Qwen3ModelRole,
        /// Allocation purpose.
        kind: ModelMemoryAllocationKind,
    },
}

impl fmt::Display for ModelMemoryAllocationBindingErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "model-memory service allocation binding rejected: {self:?}"
        )
    }
}

impl std::error::Error for ModelMemoryAllocationBindingErrorV1 {}

/// Rejected binding retaining the exact unchanged addressless plan.
#[must_use = "the rejected addressless model-memory plan remains recoverable"]
#[derive(Debug)]
pub struct ModelMemoryAllocationBindingFailureV1 {
    error: ModelMemoryAllocationBindingErrorV1,
    plan: Box<AddresslessModelMemoryPlan>,
}

impl ModelMemoryAllocationBindingFailureV1 {
    /// Returns the fail-closed diagnostic.
    #[must_use]
    pub const fn error(&self) -> ModelMemoryAllocationBindingErrorV1 {
        self.error
    }

    /// Recovers the diagnostic and exact unchanged addressless plan.
    #[must_use]
    pub fn into_parts(
        self,
    ) -> (
        ModelMemoryAllocationBindingErrorV1,
        AddresslessModelMemoryPlan,
    ) {
        (self.error, *self.plan)
    }
}

/// Failure while resolving one exact model-memory service dispatch range.
#[derive(Debug)]
pub enum ModelMemoryDispatchRangeErrorV1 {
    /// Retained allocation declarations or service-key geometry drifted.
    Binding(ModelMemoryAllocationBindingErrorV1),
    /// The requested weight coordinate is outside the authenticated layout.
    Weight(ModelWeightLayoutError),
    /// The requested KV coordinate is outside the addressless plan.
    Kv(ModelMemoryPlanError),
    /// The generic allocation owner rejected the key or range.
    Allocation(ServiceAllocationErrorV1),
}

impl fmt::Display for ModelMemoryDispatchRangeErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "model-memory dispatch range rejected: {self:?}")
    }
}

impl std::error::Error for ModelMemoryDispatchRangeErrorV1 {}

/// Move-only custody of an exact model-memory plan and four typed service keys.
///
/// This owner is deliberately not `Clone`. The service allocation session
/// retains native allocation ownership; the keys retained here remain inert.
///
/// ```compile_fail
/// use ferric_engine::BoundModelMemoryAllocationsV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<BoundModelMemoryAllocationsV1>();
/// ```
#[must_use = "the addressless model-memory plan and service keys must remain retained"]
#[derive(Debug)]
pub struct BoundModelMemoryAllocationsV1 {
    plan: AddresslessModelMemoryPlan,
    selected: SelectedModelMemoryAllocationIdentitiesV1,
    target_weights: WeightAllocationKeyV1,
    draft_weights: WeightAllocationKeyV1,
    target_kv: KvAllocationKeyV1,
    draft_kv: KvAllocationKeyV1,
}

impl BoundModelMemoryAllocationsV1 {
    /// Returns the retained exact addressless plan.
    #[must_use]
    pub const fn plan(&self) -> &AddresslessModelMemoryPlan {
        &self.plan
    }

    /// Returns the selected inert identity for one exact allocation coordinate.
    #[must_use]
    pub const fn selected_allocation_identity(
        &self,
        role: Qwen3ModelRole,
        kind: ModelMemoryAllocationKind,
    ) -> Identity {
        self.selected.get(role, kind)
    }

    pub(crate) const fn kv_allocation_key(&self, role: Qwen3ModelRole) -> KvAllocationKeyV1 {
        match role {
            Qwen3ModelRole::Target8B => self.target_kv,
            Qwen3ModelRole::Draft06B => self.draft_kv,
        }
    }

    pub(crate) fn revalidate_for_kv_partition(
        &self,
    ) -> Result<(), ModelMemoryAllocationBindingErrorV1> {
        self.revalidate()
    }

    /// Resolves one exact authenticated weight coordinate into an owner-checked,
    /// addressless device dispatch range.
    ///
    /// Global tensors require `ferric_spec::QWEN3_NO_LAYER`; per-layer tensors
    /// require an in-range layer. The retained service key preserves the
    /// `DeviceInputRoleV1` marker through both generic owner checks.
    ///
    /// # Errors
    ///
    /// Returns [`ModelMemoryDispatchRangeErrorV1`] for binding drift, an invalid
    /// canonical coordinate, or generic allocation-owner rejection.
    pub fn weight_dispatch_range(
        &self,
        allocations: &ServiceAllocationSessionV1,
        role: Qwen3ModelRole,
        kind: Qwen3TensorKind,
        layer: u32,
    ) -> Result<ServiceDeviceDispatchRangeV1, ModelMemoryDispatchRangeErrorV1> {
        self.revalidate()
            .map_err(ModelMemoryDispatchRangeErrorV1::Binding)?;
        let resolved = resolve_weight_plan_range(&self.plan, role, kind, layer)
            .map_err(ModelMemoryDispatchRangeErrorV1::Weight)?;
        self.revalidate_resolved(role, ModelMemoryAllocationKind::Weights, resolved)
            .map_err(ModelMemoryDispatchRangeErrorV1::Binding)?;
        let key = match role {
            Qwen3ModelRole::Target8B => self.target_weights,
            Qwen3ModelRole::Draft06B => self.draft_weights,
        };
        let range = allocations
            .range(key, resolved.offset, resolved.extent, resolved.alignment)
            .map_err(ModelMemoryDispatchRangeErrorV1::Allocation)?;
        allocations
            .device_dispatch_range(range)
            .map_err(ModelMemoryDispatchRangeErrorV1::Allocation)
    }

    /// Resolves one exact role/component/layer KV plane into an owner-checked,
    /// addressless device dispatch range.
    ///
    /// The retained service key preserves the `DeviceStateRoleV1` marker
    /// through both generic owner checks.
    ///
    /// # Errors
    ///
    /// Returns [`ModelMemoryDispatchRangeErrorV1`] for binding drift, an invalid
    /// canonical KV coordinate, or generic allocation-owner rejection.
    pub fn kv_dispatch_range(
        &self,
        allocations: &ServiceAllocationSessionV1,
        role: Qwen3ModelRole,
        component: KvCacheComponent,
        layer: u32,
    ) -> Result<ServiceDeviceDispatchRangeV1, ModelMemoryDispatchRangeErrorV1> {
        self.revalidate()
            .map_err(ModelMemoryDispatchRangeErrorV1::Binding)?;
        let resolved = resolve_kv_plan_range(&self.plan, role, component, layer)
            .map_err(ModelMemoryDispatchRangeErrorV1::Kv)?;
        self.revalidate_resolved(role, ModelMemoryAllocationKind::KvArena, resolved)
            .map_err(ModelMemoryDispatchRangeErrorV1::Binding)?;
        let key = match role {
            Qwen3ModelRole::Target8B => self.target_kv,
            Qwen3ModelRole::Draft06B => self.draft_kv,
        };
        let range = allocations
            .range(key, resolved.offset, resolved.extent, resolved.alignment)
            .map_err(ModelMemoryDispatchRangeErrorV1::Allocation)?;
        allocations
            .device_dispatch_range(range)
            .map_err(ModelMemoryDispatchRangeErrorV1::Allocation)
    }

    fn revalidate(&self) -> Result<(), ModelMemoryAllocationBindingErrorV1> {
        validate_model_memory_allocation_binding(
            plan_allocations(&self.plan),
            self.selected.as_array(),
            self.service_geometries(),
        )
    }

    fn service_geometries(&self) -> [ServiceAllocationGeometryV1; 4] {
        [
            ServiceAllocationGeometryV1::from_key(self.target_weights),
            ServiceAllocationGeometryV1::from_key(self.draft_weights),
            ServiceAllocationGeometryV1::from_key(self.target_kv),
            ServiceAllocationGeometryV1::from_key(self.draft_kv),
        ]
    }

    fn revalidate_resolved(
        &self,
        role: Qwen3ModelRole,
        kind: ModelMemoryAllocationKind,
        resolved: ResolvedModelMemoryRangeV1,
    ) -> Result<(), ModelMemoryAllocationBindingErrorV1> {
        if resolved.allocation_id != self.selected.get(role, kind) {
            return Err(
                ModelMemoryAllocationBindingErrorV1::ResolvedRangeIdentityDrift { role, kind },
            );
        }
        Ok(())
    }
}

/// Binds one exact addressless model-memory plan to four typed service keys.
///
/// The caller supplies target/draft weight keys with `DeviceInputRoleV1` and
/// target/draft KV keys with `DeviceStateRoleV1`, plus four inert identities
/// that explicitly join those otherwise opaque keys to the plan declarations.
/// Preflight checks every identity, exact extent, and exact 4096-byte base
/// alignment before constructing the move-only owner.
///
/// # Errors
///
/// Returns [`ModelMemoryAllocationBindingFailureV1`] for any identity,
/// aliasing, extent, or alignment drift. The failure retains the exact
/// unchanged addressless plan.
pub fn bind_addressless_model_memory_allocations_v1(
    plan: AddresslessModelMemoryPlan,
    selected: SelectedModelMemoryAllocationIdentitiesV1,
    target_weights: WeightAllocationKeyV1,
    draft_weights: WeightAllocationKeyV1,
    target_kv: KvAllocationKeyV1,
    draft_kv: KvAllocationKeyV1,
) -> Result<BoundModelMemoryAllocationsV1, ModelMemoryAllocationBindingFailureV1> {
    let geometries = [
        ServiceAllocationGeometryV1::from_key(target_weights),
        ServiceAllocationGeometryV1::from_key(draft_weights),
        ServiceAllocationGeometryV1::from_key(target_kv),
        ServiceAllocationGeometryV1::from_key(draft_kv),
    ];
    let plan = preflight_addressless_model_memory_allocations(plan, selected, geometries)?;
    Ok(BoundModelMemoryAllocationsV1 {
        plan,
        selected,
        target_weights,
        draft_weights,
        target_kv,
        draft_kv,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ServiceAllocationGeometryV1 {
    extent: u64,
    alignment: u64,
}

impl ServiceAllocationGeometryV1 {
    const fn new(extent: u64, alignment: u64) -> Self {
        Self { extent, alignment }
    }

    fn from_key<R>(key: ServiceAllocationKeyV1<R, DeviceLocalAllocationV1>) -> Self
    where
        R: fe2o3_service_host::ServiceAllocationRoleMarkerV1,
    {
        Self::new(key.extent_bytes(), key.alignment())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ResolvedModelMemoryRangeV1 {
    allocation_id: Identity,
    offset: u64,
    extent: u64,
    alignment: u64,
}

fn preflight_addressless_model_memory_allocations(
    plan: AddresslessModelMemoryPlan,
    selected: SelectedModelMemoryAllocationIdentitiesV1,
    geometries: [ServiceAllocationGeometryV1; 4],
) -> Result<AddresslessModelMemoryPlan, ModelMemoryAllocationBindingFailureV1> {
    match validate_model_memory_allocation_binding(
        plan_allocations(&plan),
        selected.as_array(),
        geometries,
    ) {
        Ok(()) => Ok(plan),
        Err(error) => Err(ModelMemoryAllocationBindingFailureV1 {
            error,
            plan: Box::new(plan),
        }),
    }
}

fn plan_allocations(plan: &AddresslessModelMemoryPlan) -> [DeclaredDeviceAllocation; 4] {
    MODEL_MEMORY_SLOTS_V1.map(|(role, kind)| plan.allocation(role, kind))
}

pub(crate) fn preflight_addressless_model_memory_plan_v1(
    plan: &AddresslessModelMemoryPlan,
) -> Result<SelectedModelMemoryAllocationIdentitiesV1, ModelMemoryAllocationBindingErrorV1> {
    let allocations = plan_allocations(plan);
    let selected = SelectedModelMemoryAllocationIdentitiesV1::new(
        allocations[0].allocation_id(),
        allocations[1].allocation_id(),
        allocations[2].allocation_id(),
        allocations[3].allocation_id(),
    );
    let geometries = allocations.map(|allocation| {
        ServiceAllocationGeometryV1::new(allocation.byte_len(), allocation.alignment())
    });
    validate_model_memory_allocation_binding(allocations, selected.as_array(), geometries)?;
    Ok(selected)
}

fn validate_model_memory_allocation_binding(
    plan: [DeclaredDeviceAllocation; 4],
    selected: [Identity; 4],
    service: [ServiceAllocationGeometryV1; 4],
) -> Result<(), ModelMemoryAllocationBindingErrorV1> {
    for (index, (role, kind)) in MODEL_MEMORY_SLOTS_V1.iter().copied().enumerate() {
        if !plan[index].allocation_id().is_present() {
            return Err(
                ModelMemoryAllocationBindingErrorV1::MissingPlanAllocationIdentity { role, kind },
            );
        }
        if !selected[index].is_present() {
            return Err(
                ModelMemoryAllocationBindingErrorV1::MissingSelectedAllocationIdentity {
                    role,
                    kind,
                },
            );
        }
        if selected[index] != plan[index].allocation_id() {
            return Err(
                ModelMemoryAllocationBindingErrorV1::SelectedAllocationIdentityDrift { role, kind },
            );
        }
    }

    for left in 0..selected.len() {
        for right in (left + 1)..selected.len() {
            if selected[left] == selected[right] {
                return Err(
                    ModelMemoryAllocationBindingErrorV1::AllocationIdentityAlias {
                        left: MODEL_MEMORY_SLOTS_V1[left],
                        right: MODEL_MEMORY_SLOTS_V1[right],
                    },
                );
            }
        }
    }

    for (index, (role, kind)) in MODEL_MEMORY_SLOTS_V1.iter().copied().enumerate() {
        let expected_extent = match kind {
            ModelMemoryAllocationKind::Weights => role.tensor_data_bytes(),
            ModelMemoryAllocationKind::KvArena => qwen3_kv_arena_bytes(role),
        };
        if plan[index].byte_len() != expected_extent {
            return Err(
                ModelMemoryAllocationBindingErrorV1::PlanAllocationExtentDrift {
                    role,
                    kind,
                    expected: expected_extent,
                    actual: plan[index].byte_len(),
                },
            );
        }
        if plan[index].alignment() != QWEN3_MODEL_MEMORY_ALLOCATION_ALIGNMENT_V1 {
            return Err(
                ModelMemoryAllocationBindingErrorV1::PlanAllocationAlignmentDrift {
                    role,
                    kind,
                    expected: QWEN3_MODEL_MEMORY_ALLOCATION_ALIGNMENT_V1,
                    actual: plan[index].alignment(),
                },
            );
        }
        if service[index].extent != plan[index].byte_len() {
            return Err(
                ModelMemoryAllocationBindingErrorV1::ServiceAllocationExtentDrift {
                    role,
                    kind,
                    expected: plan[index].byte_len(),
                    actual: service[index].extent,
                },
            );
        }
        if service[index].alignment != QWEN3_MODEL_MEMORY_ALLOCATION_ALIGNMENT_V1 {
            return Err(
                ModelMemoryAllocationBindingErrorV1::ServiceAllocationAlignmentDrift {
                    role,
                    kind,
                    expected: QWEN3_MODEL_MEMORY_ALLOCATION_ALIGNMENT_V1,
                    actual: service[index].alignment,
                },
            );
        }
    }
    Ok(())
}

fn resolve_weight_plan_range(
    plan: &AddresslessModelMemoryPlan,
    role: Qwen3ModelRole,
    kind: Qwen3TensorKind,
    layer: u32,
) -> Result<ResolvedModelMemoryRangeV1, ModelWeightLayoutError> {
    let coordinate = plan.weight_layout().lookup(role, kind, layer)?;
    let binding = plan.weight_by_ordinal(role, coordinate.ordinal())?;
    let range = binding.range();
    Ok(ResolvedModelMemoryRangeV1 {
        allocation_id: range.allocation_id(),
        offset: range.offset(),
        extent: range.byte_len(),
        alignment: binding.weight().section().alignment(),
    })
}

fn resolve_kv_plan_range(
    plan: &AddresslessModelMemoryPlan,
    role: Qwen3ModelRole,
    component: KvCacheComponent,
    layer: u32,
) -> Result<ResolvedModelMemoryRangeV1, ModelMemoryPlanError> {
    let range: DeclaredMemoryRange = plan.kv_layer(role, component, layer)?;
    Ok(ResolvedModelMemoryRangeV1 {
        allocation_id: range.allocation_id(),
        offset: range.offset(),
        extent: range.byte_len(),
        alignment: QWEN3_KV_ARENA_ALIGNMENT_V1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferric_build::{
        qwen3_model_memory_plan_test_fixture, QWEN3_KV_LAYER_BYTES_V1,
        QWEN3_MODEL_MEMORY_ALLOCATION_ALIGNMENT_V1,
    };
    use ferric_spec::QWEN3_NO_LAYER;

    const fn identity(seed: u8) -> Identity {
        Identity::new([seed; 32])
    }

    fn exact_plan() -> AddresslessModelMemoryPlan {
        qwen3_model_memory_plan_test_fixture()
    }

    fn exact_selected(
        plan: &AddresslessModelMemoryPlan,
    ) -> SelectedModelMemoryAllocationIdentitiesV1 {
        SelectedModelMemoryAllocationIdentitiesV1::new(
            plan.allocation(Qwen3ModelRole::Target8B, ModelMemoryAllocationKind::Weights)
                .allocation_id(),
            plan.allocation(Qwen3ModelRole::Draft06B, ModelMemoryAllocationKind::Weights)
                .allocation_id(),
            plan.allocation(Qwen3ModelRole::Target8B, ModelMemoryAllocationKind::KvArena)
                .allocation_id(),
            plan.allocation(Qwen3ModelRole::Draft06B, ModelMemoryAllocationKind::KvArena)
                .allocation_id(),
        )
    }

    fn exact_geometries(plan: &AddresslessModelMemoryPlan) -> [ServiceAllocationGeometryV1; 4] {
        plan_allocations(plan).map(|allocation| {
            ServiceAllocationGeometryV1::new(allocation.byte_len(), allocation.alignment())
        })
    }

    #[test]
    fn every_weight_and_kv_coordinate_resolves_to_the_exact_plan_row() {
        let plan = exact_plan();
        for role in [Qwen3ModelRole::Target8B, Qwen3ModelRole::Draft06B] {
            let weight_allocation = plan.allocation(role, ModelMemoryAllocationKind::Weights);
            for ordinal in 0..role.tensor_count() {
                let expected = plan.weight_by_ordinal(role, ordinal).unwrap();
                let metadata = expected.weight().metadata();
                let actual =
                    resolve_weight_plan_range(&plan, role, metadata.kind, metadata.layer).unwrap();
                assert_eq!(actual.allocation_id, weight_allocation.allocation_id());
                assert_eq!(
                    (actual.offset, actual.extent),
                    expected.weight().destination_range()
                );
                assert_eq!(actual.alignment, expected.weight().section().alignment());
            }

            let kv_allocation = plan.allocation(role, ModelMemoryAllocationKind::KvArena);
            for layer in 0..role.layers() {
                for component in [KvCacheComponent::Key, KvCacheComponent::Value] {
                    let expected = plan.kv_layer(role, component, layer).unwrap();
                    let actual = resolve_kv_plan_range(&plan, role, component, layer).unwrap();
                    assert_eq!(actual.allocation_id, kv_allocation.allocation_id());
                    assert_eq!(
                        (actual.offset, actual.extent),
                        (expected.offset(), expected.byte_len())
                    );
                    assert_eq!(actual.alignment, QWEN3_KV_ARENA_ALIGNMENT_V1);
                }
            }
        }
    }

    #[test]
    fn global_and_out_of_range_coordinates_fail_closed() {
        let plan = exact_plan();
        assert!(resolve_weight_plan_range(
            &plan,
            Qwen3ModelRole::Target8B,
            Qwen3TensorKind::TokenEmbedding,
            QWEN3_NO_LAYER,
        )
        .is_ok());
        assert!(matches!(
            resolve_weight_plan_range(
                &plan,
                Qwen3ModelRole::Target8B,
                Qwen3TensorKind::TokenEmbedding,
                0,
            ),
            Err(ModelWeightLayoutError::UnknownCoordinate { .. })
        ));
        assert_eq!(
            resolve_kv_plan_range(
                &plan,
                Qwen3ModelRole::Draft06B,
                KvCacheComponent::Value,
                Qwen3ModelRole::Draft06B.layers(),
            ),
            Err(ModelMemoryPlanError::LayerOutOfRange {
                role: Qwen3ModelRole::Draft06B,
                layer: Qwen3ModelRole::Draft06B.layers(),
            })
        );
    }

    #[test]
    fn exact_preflight_accepts_all_four_named_coordinates() {
        let plan = exact_plan();
        validate_model_memory_allocation_binding(
            plan_allocations(&plan),
            exact_selected(&plan).as_array(),
            exact_geometries(&plan),
        )
        .unwrap();
        assert_eq!(QWEN3_MODEL_MEMORY_ALLOCATION_ALIGNMENT_V1, 4_096);
        assert!(QWEN3_KV_LAYER_BYTES_V1.is_multiple_of(4_096));
    }

    #[test]
    fn rejected_preflight_recovers_the_exact_unchanged_plan() {
        let plan = exact_plan();
        let record_id = plan.weight_layout().admission().record().record_id();
        let declarations = plan_allocations(&plan);
        let mut geometries = exact_geometries(&plan);
        geometries[2].extent -= 1;
        let failure = preflight_addressless_model_memory_allocations(
            plan,
            SelectedModelMemoryAllocationIdentitiesV1::new(
                declarations[0].allocation_id(),
                declarations[1].allocation_id(),
                declarations[2].allocation_id(),
                declarations[3].allocation_id(),
            ),
            geometries,
        )
        .unwrap_err();
        assert!(matches!(
            failure.error(),
            ModelMemoryAllocationBindingErrorV1::ServiceAllocationExtentDrift {
                role: Qwen3ModelRole::Target8B,
                kind: ModelMemoryAllocationKind::KvArena,
                ..
            }
        ));
        let (_, recovered) = failure.into_parts();
        assert_eq!(
            recovered.weight_layout().admission().record().record_id(),
            record_id
        );
        assert_eq!(plan_allocations(&recovered), declarations);
    }

    #[test]
    fn missing_drifted_and_aliased_identities_are_rejected_by_exact_slot() {
        let plan = exact_plan();
        let declarations = plan_allocations(&plan);
        let exact = exact_selected(&plan).as_array();
        let geometries = exact_geometries(&plan);

        let mut missing_plan = declarations;
        let target_weights = missing_plan[0];
        missing_plan[0] = DeclaredDeviceAllocation::new(
            Identity::new([0; 32]),
            target_weights.byte_len(),
            target_weights.alignment(),
        );
        assert_eq!(
            validate_model_memory_allocation_binding(missing_plan, exact, geometries),
            Err(
                ModelMemoryAllocationBindingErrorV1::MissingPlanAllocationIdentity {
                    role: Qwen3ModelRole::Target8B,
                    kind: ModelMemoryAllocationKind::Weights,
                }
            )
        );

        let mut missing = exact;
        missing[1] = Identity::new([0; 32]);
        assert_eq!(
            validate_model_memory_allocation_binding(declarations, missing, geometries),
            Err(
                ModelMemoryAllocationBindingErrorV1::MissingSelectedAllocationIdentity {
                    role: Qwen3ModelRole::Draft06B,
                    kind: ModelMemoryAllocationKind::Weights,
                }
            )
        );

        let mut drifted = exact;
        drifted[3] = identity(99);
        assert_eq!(
            validate_model_memory_allocation_binding(declarations, drifted, geometries),
            Err(
                ModelMemoryAllocationBindingErrorV1::SelectedAllocationIdentityDrift {
                    role: Qwen3ModelRole::Draft06B,
                    kind: ModelMemoryAllocationKind::KvArena,
                }
            )
        );

        let mut swapped = exact;
        swapped.swap(0, 1);
        assert_eq!(
            validate_model_memory_allocation_binding(declarations, swapped, geometries),
            Err(
                ModelMemoryAllocationBindingErrorV1::SelectedAllocationIdentityDrift {
                    role: Qwen3ModelRole::Target8B,
                    kind: ModelMemoryAllocationKind::Weights,
                }
            )
        );

        for left in 0..4 {
            for right in (left + 1)..4 {
                let mut aliased_declarations = declarations;
                let right_allocation = aliased_declarations[right];
                aliased_declarations[right] = DeclaredDeviceAllocation::new(
                    aliased_declarations[left].allocation_id(),
                    right_allocation.byte_len(),
                    right_allocation.alignment(),
                );
                let mut aliased_selected = exact;
                aliased_selected[right] = aliased_selected[left];
                assert_eq!(
                    validate_model_memory_allocation_binding(
                        aliased_declarations,
                        aliased_selected,
                        geometries,
                    ),
                    Err(
                        ModelMemoryAllocationBindingErrorV1::AllocationIdentityAlias {
                            left: MODEL_MEMORY_SLOTS_V1[left],
                            right: MODEL_MEMORY_SLOTS_V1[right],
                        }
                    )
                );
            }
        }
    }

    #[test]
    fn every_plan_and_service_geometry_drift_is_rejected() {
        let plan = exact_plan();
        let declarations = plan_allocations(&plan);
        let selected = exact_selected(&plan).as_array();
        let geometries = exact_geometries(&plan);

        for index in 0..4 {
            let (role, kind) = MODEL_MEMORY_SLOTS_V1[index];
            let mut short_plan = declarations;
            short_plan[index] = DeclaredDeviceAllocation::new(
                short_plan[index].allocation_id(),
                short_plan[index].byte_len() - 1,
                short_plan[index].alignment(),
            );
            assert!(matches!(
                validate_model_memory_allocation_binding(short_plan, selected, geometries),
                Err(ModelMemoryAllocationBindingErrorV1::PlanAllocationExtentDrift {
                    role: actual_role,
                    kind: actual_kind,
                    ..
                }) if actual_role == role && actual_kind == kind
            ));

            let mut plan_alignment = declarations;
            plan_alignment[index] = DeclaredDeviceAllocation::new(
                plan_alignment[index].allocation_id(),
                plan_alignment[index].byte_len(),
                2_048,
            );
            assert!(matches!(
                validate_model_memory_allocation_binding(plan_alignment, selected, geometries),
                Err(ModelMemoryAllocationBindingErrorV1::PlanAllocationAlignmentDrift {
                    role: actual_role,
                    kind: actual_kind,
                    ..
                }) if actual_role == role && actual_kind == kind
            ));

            let mut service_extent = geometries;
            service_extent[index].extent -= 1;
            assert!(matches!(
                validate_model_memory_allocation_binding(declarations, selected, service_extent),
                Err(ModelMemoryAllocationBindingErrorV1::ServiceAllocationExtentDrift {
                    role: actual_role,
                    kind: actual_kind,
                    ..
                }) if actual_role == role && actual_kind == kind
            ));

            let mut service_alignment = geometries;
            service_alignment[index].alignment = 2_048;
            assert!(matches!(
                validate_model_memory_allocation_binding(
                    declarations,
                    selected,
                    service_alignment,
                ),
                Err(ModelMemoryAllocationBindingErrorV1::ServiceAllocationAlignmentDrift {
                    role: actual_role,
                    kind: actual_kind,
                    ..
                }) if actual_role == role && actual_kind == kind
            ));
        }
    }
}
