//! Fully initialized service allocation for one exact model-memory plan.
//!
//! Ferric owns the semantic content-role namespace, model-specific image
//! checks, zero-filled KV construction, and the join back to its addressless
//! model-memory plan. Generic fe2o3 remains responsible for allocating,
//! verifying, mapping, and retaining device memory without exposing an address.
//!
//! A successful return retains only typed addressless keys. Native allocation,
//! mapping, initialized-content, and teardown authority remains in the caller's
//! [`ServiceAllocationSessionV1`]. An allocation failure can leave allocations
//! completed earlier in the fixed target-weight, draft-weight, target-KV,
//! draft-KV order retained by that session. This function does not claim an
//! atomic four-allocation rollback and does not return individual release
//! authority.

use core::fmt;
use std::collections::TryReserveError;

use fe2o3_kfd::{
    Gfx942DeviceContentDescriptorErrorV1, Gfx942DeviceContentDescriptorV1,
    Gfx942DeviceContentRoleV1,
};
use fe2o3_service_host::{
    DeviceInputRoleV1, DeviceStateRoleV1, ServiceAllocationErrorV1, ServiceAllocationSessionV1,
};
use ferric_build::{
    qwen3_kv_arena_bytes, AddresslessModelMemoryPlan, ModelMemoryAllocationKind,
    QWEN3_MODEL_MEMORY_ALLOCATION_ALIGNMENT_V1,
};
use ferric_spec::Qwen3ModelRole;

use crate::model_memory_allocations::preflight_addressless_model_memory_plan_v1;
use crate::{
    bind_addressless_model_memory_allocations_v1, BoundModelMemoryAllocationsV1,
    ModelMemoryAllocationBindingErrorV1,
};

/// SHA-256 of `ferric-m1-initialized-model-memory-content-role-v1\0`.
///
/// This Ferric-owned namespace is deliberately independent of caller-selected
/// model allocation identities and generic fe2o3 allocation generations.
pub const M1_INITIALIZED_MODEL_MEMORY_CONTENT_ROLE_IDENTITY_V1: [u8; 32] = [
    250, 69, 108, 231, 92, 218, 32, 95, 159, 200, 205, 87, 56, 144, 155, 233, 204, 148, 216, 17,
    231, 253, 219, 136, 61, 101, 199, 250, 83, 130, 234, 175,
];

/// Host-only validation or exact zero-image preparation failure.
#[derive(Debug)]
pub enum InitializedModelMemoryPreflightErrorV1 {
    /// The retained addressless plan failed exact identity, extent, alignment,
    /// or non-aliasing validation.
    Plan(ModelMemoryAllocationBindingErrorV1),
    /// A supplied prepacked weight image does not have its role's exact length.
    WeightImageLength {
        /// Selected model role.
        role: Qwen3ModelRole,
        /// Required complete prepacked image length.
        expected: u64,
        /// Supplied image length.
        actual: u64,
    },
    /// A required host byte image cannot be indexed on this host architecture.
    HostImageExtent {
        /// Selected model role.
        role: Qwen3ModelRole,
        /// Allocation purpose.
        kind: ModelMemoryAllocationKind,
        /// Required image length.
        byte_len: u64,
    },
    /// Host allocation for a complete zero-filled KV image failed.
    HostImageAllocation {
        /// Selected model role.
        role: Qwen3ModelRole,
        /// Allocation purpose.
        kind: ModelMemoryAllocationKind,
        /// Required image length.
        byte_len: u64,
        /// Standard allocator reservation failure.
        source: TryReserveError,
    },
}

impl fmt::Display for InitializedModelMemoryPreflightErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "initialized model-memory preflight rejected: {self:?}"
        )
    }
}

impl std::error::Error for InitializedModelMemoryPreflightErrorV1 {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::HostImageAllocation { source, .. } => Some(source),
            Self::Plan(source) => Some(source),
            Self::WeightImageLength { .. } | Self::HostImageExtent { .. } => None,
        }
    }
}

/// Fail-closed initialized model-memory allocation error.
#[derive(Debug)]
pub enum InitializedModelMemoryAllocationErrorV1 {
    /// A host-only plan, weight-image, or KV-image check failed before KFD use.
    Preflight(InitializedModelMemoryPreflightErrorV1),
    /// A deterministic content descriptor could not be constructed.
    Descriptor {
        /// Selected model role.
        role: Qwen3ModelRole,
        /// Allocation purpose.
        kind: ModelMemoryAllocationKind,
        /// Generic descriptor rejection.
        source: Gfx942DeviceContentDescriptorErrorV1,
    },
    /// The generic service allocation session rejected an initialized image.
    Allocation {
        /// Selected model role.
        role: Qwen3ModelRole,
        /// Allocation purpose.
        kind: ModelMemoryAllocationKind,
        /// Generic owner or KFD failure.
        source: ServiceAllocationErrorV1,
    },
    /// Exact keys produced by allocation could not be rebound to the plan.
    Binding(ModelMemoryAllocationBindingErrorV1),
}

impl fmt::Display for InitializedModelMemoryAllocationErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "initialized model-memory allocation failed: {self:?}"
        )
    }
}

impl std::error::Error for InitializedModelMemoryAllocationErrorV1 {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Preflight(source) => Some(source),
            Self::Descriptor { source, .. } => Some(source),
            Self::Allocation { source, .. } => Some(source),
            Self::Binding(source) => Some(source),
        }
    }
}

/// Rejected allocation attempt retaining the exact addressless plan.
///
/// Supplied byte images are consumed by the attempt. On an allocation error,
/// the caller must inspect and eventually release or quarantine the supplied
/// service session; earlier successful allocations remain owned there.
#[must_use = "the rejected addressless model-memory plan remains recoverable"]
#[derive(Debug)]
pub struct InitializedModelMemoryAllocationFailureV1 {
    error: InitializedModelMemoryAllocationErrorV1,
    plan: Box<AddresslessModelMemoryPlan>,
}

impl InitializedModelMemoryAllocationFailureV1 {
    /// Returns the fail-closed diagnostic.
    #[must_use]
    pub const fn error(&self) -> &InitializedModelMemoryAllocationErrorV1 {
        &self.error
    }

    /// Recovers the diagnostic and exact unchanged addressless plan.
    #[must_use]
    pub fn into_parts(
        self,
    ) -> (
        InitializedModelMemoryAllocationErrorV1,
        AddresslessModelMemoryPlan,
    ) {
        (self.error, *self.plan)
    }
}

/// Returns the deterministic Ferric content role for one model-memory slot.
///
/// The four exact ordinals are target weights `0`, draft weights `1`, target
/// KV `2`, and draft KV `3`. The returned value is descriptor data only; it is
/// not allocation, address, mapping, or initialized-content authority.
///
/// # Errors
///
/// Returns the generic descriptor error if the fixed Ferric role namespace is
/// invalid.
pub fn m1_model_memory_content_role_v1(
    role: Qwen3ModelRole,
    kind: ModelMemoryAllocationKind,
) -> Result<Gfx942DeviceContentRoleV1, Gfx942DeviceContentDescriptorErrorV1> {
    Gfx942DeviceContentRoleV1::new(
        M1_INITIALIZED_MODEL_MEMORY_CONTENT_ROLE_IDENTITY_V1,
        model_memory_content_ordinal(role, kind),
    )
}

/// Describes one exact byte image under Ferric's deterministic content role.
///
/// This helper is pure descriptor construction. It neither allocates nor maps
/// memory and cannot mint initialized-device authority.
///
/// # Errors
///
/// Returns the generic descriptor error for an invalid role or byte image.
pub fn m1_model_memory_content_descriptor_v1(
    role: Qwen3ModelRole,
    kind: ModelMemoryAllocationKind,
    bytes: &[u8],
) -> Result<Gfx942DeviceContentDescriptorV1, Gfx942DeviceContentDescriptorErrorV1> {
    let content_role = m1_model_memory_content_role_v1(role, kind)?;
    Gfx942DeviceContentDescriptorV1::from_bytes(content_role, bytes)
}

/// Allocates and binds exact initialized target/draft model memory.
///
/// All Ferric-owned fallible host preflight runs before the first service
/// allocation: the plan's four allocation declarations, both supplied weight
/// lengths, both complete zero-filled KV host images, and all four content
/// descriptors. The caller supplies the sole generic allocation/KFD owner.
/// Weights use `DeviceInputRoleV1`; mutable KV arenas use
/// `DeviceStateRoleV1`. Every allocation uses the exact 4096-byte model-memory
/// alignment and complete image extent.
///
/// The returned value owns no native address or allocation lease. The caller
/// must keep `allocations` alive for dispatch-range resolution and must use its
/// generic teardown path after all queue custody is returned.
///
/// # Errors
///
/// Returns [`InitializedModelMemoryAllocationFailureV1`] for explicit
/// preflight, descriptor, service-allocation, or final binding rejection. The
/// failure retains the unchanged addressless plan but not consumed byte images.
pub fn allocate_initialized_model_memory_v1(
    allocations: &mut ServiceAllocationSessionV1,
    plan: AddresslessModelMemoryPlan,
    target_prepacked_weights: Box<[u8]>,
    draft_prepacked_weights: Box<[u8]>,
) -> Result<BoundModelMemoryAllocationsV1, InitializedModelMemoryAllocationFailureV1> {
    let selected = match preflight_addressless_model_memory_plan_v1(&plan) {
        Ok(selected) => selected,
        Err(source) => {
            return Err(allocation_failure(
                InitializedModelMemoryAllocationErrorV1::Preflight(
                    InitializedModelMemoryPreflightErrorV1::Plan(source),
                ),
                plan,
            ));
        }
    };

    if let Err(error) =
        validate_weight_image_length(Qwen3ModelRole::Target8B, target_prepacked_weights.len())
    {
        return Err(allocation_failure(
            InitializedModelMemoryAllocationErrorV1::Preflight(error),
            plan,
        ));
    }
    if let Err(error) =
        validate_weight_image_length(Qwen3ModelRole::Draft06B, draft_prepacked_weights.len())
    {
        return Err(allocation_failure(
            InitializedModelMemoryAllocationErrorV1::Preflight(error),
            plan,
        ));
    }

    let target_kv = match zeroed_kv_image(Qwen3ModelRole::Target8B) {
        Ok(image) => image,
        Err(error) => {
            return Err(allocation_failure(
                InitializedModelMemoryAllocationErrorV1::Preflight(error),
                plan,
            ));
        }
    };
    let draft_kv = match zeroed_kv_image(Qwen3ModelRole::Draft06B) {
        Ok(image) => image,
        Err(error) => {
            return Err(allocation_failure(
                InitializedModelMemoryAllocationErrorV1::Preflight(error),
                plan,
            ));
        }
    };

    let target_weight_descriptor = match m1_model_memory_content_descriptor_v1(
        Qwen3ModelRole::Target8B,
        ModelMemoryAllocationKind::Weights,
        &target_prepacked_weights,
    ) {
        Ok(descriptor) => descriptor,
        Err(source) => {
            return Err(descriptor_failure(
                source,
                Qwen3ModelRole::Target8B,
                ModelMemoryAllocationKind::Weights,
                plan,
            ));
        }
    };
    let draft_weight_descriptor = match m1_model_memory_content_descriptor_v1(
        Qwen3ModelRole::Draft06B,
        ModelMemoryAllocationKind::Weights,
        &draft_prepacked_weights,
    ) {
        Ok(descriptor) => descriptor,
        Err(source) => {
            return Err(descriptor_failure(
                source,
                Qwen3ModelRole::Draft06B,
                ModelMemoryAllocationKind::Weights,
                plan,
            ));
        }
    };
    let target_kv_descriptor = match m1_model_memory_content_descriptor_v1(
        Qwen3ModelRole::Target8B,
        ModelMemoryAllocationKind::KvArena,
        &target_kv,
    ) {
        Ok(descriptor) => descriptor,
        Err(source) => {
            return Err(descriptor_failure(
                source,
                Qwen3ModelRole::Target8B,
                ModelMemoryAllocationKind::KvArena,
                plan,
            ));
        }
    };
    let draft_kv_descriptor = match m1_model_memory_content_descriptor_v1(
        Qwen3ModelRole::Draft06B,
        ModelMemoryAllocationKind::KvArena,
        &draft_kv,
    ) {
        Ok(descriptor) => descriptor,
        Err(source) => {
            return Err(descriptor_failure(
                source,
                Qwen3ModelRole::Draft06B,
                ModelMemoryAllocationKind::KvArena,
                plan,
            ));
        }
    };

    let target_weights = match allocations.allocate_initialized_device_local::<DeviceInputRoleV1>(
        target_prepacked_weights,
        QWEN3_MODEL_MEMORY_ALLOCATION_ALIGNMENT_V1,
        target_weight_descriptor,
    ) {
        Ok(key) => key,
        Err(source) => {
            return Err(service_allocation_failure(
                source,
                Qwen3ModelRole::Target8B,
                ModelMemoryAllocationKind::Weights,
                plan,
            ));
        }
    };
    let draft_weights = match allocations.allocate_initialized_device_local::<DeviceInputRoleV1>(
        draft_prepacked_weights,
        QWEN3_MODEL_MEMORY_ALLOCATION_ALIGNMENT_V1,
        draft_weight_descriptor,
    ) {
        Ok(key) => key,
        Err(source) => {
            return Err(service_allocation_failure(
                source,
                Qwen3ModelRole::Draft06B,
                ModelMemoryAllocationKind::Weights,
                plan,
            ));
        }
    };
    let target_kv = match allocations.allocate_initialized_device_local::<DeviceStateRoleV1>(
        target_kv,
        QWEN3_MODEL_MEMORY_ALLOCATION_ALIGNMENT_V1,
        target_kv_descriptor,
    ) {
        Ok(key) => key,
        Err(source) => {
            return Err(service_allocation_failure(
                source,
                Qwen3ModelRole::Target8B,
                ModelMemoryAllocationKind::KvArena,
                plan,
            ));
        }
    };
    let draft_kv = match allocations.allocate_initialized_device_local::<DeviceStateRoleV1>(
        draft_kv,
        QWEN3_MODEL_MEMORY_ALLOCATION_ALIGNMENT_V1,
        draft_kv_descriptor,
    ) {
        Ok(key) => key,
        Err(source) => {
            return Err(service_allocation_failure(
                source,
                Qwen3ModelRole::Draft06B,
                ModelMemoryAllocationKind::KvArena,
                plan,
            ));
        }
    };

    bind_addressless_model_memory_allocations_v1(
        plan,
        selected,
        target_weights,
        draft_weights,
        target_kv,
        draft_kv,
    )
    .map_err(|failure| {
        let (source, plan) = failure.into_parts();
        allocation_failure(
            InitializedModelMemoryAllocationErrorV1::Binding(source),
            plan,
        )
    })
}

const fn model_memory_content_ordinal(
    role: Qwen3ModelRole,
    kind: ModelMemoryAllocationKind,
) -> u32 {
    match (role, kind) {
        (Qwen3ModelRole::Target8B, ModelMemoryAllocationKind::Weights) => 0,
        (Qwen3ModelRole::Draft06B, ModelMemoryAllocationKind::Weights) => 1,
        (Qwen3ModelRole::Target8B, ModelMemoryAllocationKind::KvArena) => 2,
        (Qwen3ModelRole::Draft06B, ModelMemoryAllocationKind::KvArena) => 3,
    }
}

fn validate_weight_image_length(
    role: Qwen3ModelRole,
    actual: usize,
) -> Result<(), InitializedModelMemoryPreflightErrorV1> {
    let actual = u64::try_from(actual).map_err(|_| {
        InitializedModelMemoryPreflightErrorV1::HostImageExtent {
            role,
            kind: ModelMemoryAllocationKind::Weights,
            byte_len: u64::MAX,
        }
    })?;
    let expected = role.tensor_data_bytes();
    if actual != expected {
        return Err(InitializedModelMemoryPreflightErrorV1::WeightImageLength {
            role,
            expected,
            actual,
        });
    }
    Ok(())
}

fn zeroed_kv_image(
    role: Qwen3ModelRole,
) -> Result<Box<[u8]>, InitializedModelMemoryPreflightErrorV1> {
    let byte_len = qwen3_kv_arena_bytes(role);
    let length = usize::try_from(byte_len).map_err(|_| {
        InitializedModelMemoryPreflightErrorV1::HostImageExtent {
            role,
            kind: ModelMemoryAllocationKind::KvArena,
            byte_len,
        }
    })?;
    zeroed_image(length).map_err(|source| {
        InitializedModelMemoryPreflightErrorV1::HostImageAllocation {
            role,
            kind: ModelMemoryAllocationKind::KvArena,
            byte_len,
            source,
        }
    })
}

fn zeroed_image(length: usize) -> Result<Box<[u8]>, TryReserveError> {
    let mut image = Vec::new();
    image.try_reserve_exact(length)?;
    image.resize(length, 0);
    Ok(image.into_boxed_slice())
}

fn descriptor_failure(
    source: Gfx942DeviceContentDescriptorErrorV1,
    role: Qwen3ModelRole,
    kind: ModelMemoryAllocationKind,
    plan: AddresslessModelMemoryPlan,
) -> InitializedModelMemoryAllocationFailureV1 {
    allocation_failure(
        InitializedModelMemoryAllocationErrorV1::Descriptor { role, kind, source },
        plan,
    )
}

fn service_allocation_failure(
    source: ServiceAllocationErrorV1,
    role: Qwen3ModelRole,
    kind: ModelMemoryAllocationKind,
    plan: AddresslessModelMemoryPlan,
) -> InitializedModelMemoryAllocationFailureV1 {
    allocation_failure(
        InitializedModelMemoryAllocationErrorV1::Allocation { role, kind, source },
        plan,
    )
}

fn allocation_failure(
    error: InitializedModelMemoryAllocationErrorV1,
    plan: AddresslessModelMemoryPlan,
) -> InitializedModelMemoryAllocationFailureV1 {
    InitializedModelMemoryAllocationFailureV1 {
        error,
        plan: Box::new(plan),
    }
}

#[cfg(test)]
mod tests {
    use fe2o3_kfd::Gfx942DeviceContentDescriptorErrorV1;
    use ferric_build::{qwen3_model_memory_plan_test_fixture, ModelMemoryAllocationKind};
    use ferric_spec::Qwen3ModelRole;
    use sha2::{Digest, Sha256};

    use super::{
        m1_model_memory_content_descriptor_v1, m1_model_memory_content_role_v1,
        preflight_addressless_model_memory_plan_v1, validate_weight_image_length, zeroed_image,
        InitializedModelMemoryPreflightErrorV1,
        M1_INITIALIZED_MODEL_MEMORY_CONTENT_ROLE_IDENTITY_V1,
    };

    #[test]
    fn ferric_content_namespace_is_the_frozen_domain_digest() {
        assert_eq!(
            M1_INITIALIZED_MODEL_MEMORY_CONTENT_ROLE_IDENTITY_V1,
            Sha256::digest(b"ferric-m1-initialized-model-memory-content-role-v1\0").as_slice()
        );
    }

    #[test]
    fn roles_are_deterministic_and_separate_all_four_exact_slots() {
        let coordinates = [
            (Qwen3ModelRole::Target8B, ModelMemoryAllocationKind::Weights),
            (Qwen3ModelRole::Draft06B, ModelMemoryAllocationKind::Weights),
            (Qwen3ModelRole::Target8B, ModelMemoryAllocationKind::KvArena),
            (Qwen3ModelRole::Draft06B, ModelMemoryAllocationKind::KvArena),
        ];
        for (ordinal, (role, kind)) in coordinates.into_iter().enumerate() {
            let first = m1_model_memory_content_role_v1(role, kind).unwrap();
            let second = m1_model_memory_content_role_v1(role, kind).unwrap();
            assert_eq!(first, second);
            assert_eq!(
                first.identity(),
                M1_INITIALIZED_MODEL_MEMORY_CONTENT_ROLE_IDENTITY_V1
            );
            assert_eq!(first.ordinal(), u32::try_from(ordinal).unwrap());
        }
    }

    #[test]
    fn descriptors_bind_role_length_digest_and_bytes_deterministically() {
        let bytes = b"ferric-model-image";
        let first = m1_model_memory_content_descriptor_v1(
            Qwen3ModelRole::Target8B,
            ModelMemoryAllocationKind::Weights,
            bytes,
        )
        .unwrap();
        let second = m1_model_memory_content_descriptor_v1(
            Qwen3ModelRole::Target8B,
            ModelMemoryAllocationKind::Weights,
            bytes,
        )
        .unwrap();
        let other_slot = m1_model_memory_content_descriptor_v1(
            Qwen3ModelRole::Draft06B,
            ModelMemoryAllocationKind::Weights,
            bytes,
        )
        .unwrap();
        let other_bytes = m1_model_memory_content_descriptor_v1(
            Qwen3ModelRole::Target8B,
            ModelMemoryAllocationKind::Weights,
            b"ferric-model-imagf",
        )
        .unwrap();

        assert_eq!(first, second);
        assert_eq!(first.byte_len(), u64::try_from(bytes.len()).unwrap());
        assert_eq!(first.sha256(), Sha256::digest(bytes).as_slice());
        assert_ne!(first.identity(), other_slot.identity());
        assert_ne!(first.identity(), other_bytes.identity());
    }

    #[test]
    fn descriptor_rejects_an_empty_image_without_kfd() {
        assert_eq!(
            m1_model_memory_content_descriptor_v1(
                Qwen3ModelRole::Target8B,
                ModelMemoryAllocationKind::Weights,
                &[],
            ),
            Err(Gfx942DeviceContentDescriptorErrorV1::ZeroByteExtent)
        );
    }

    #[test]
    fn exact_plan_and_weight_lengths_preflight_without_kfd() {
        let plan = qwen3_model_memory_plan_test_fixture();
        let selected = preflight_addressless_model_memory_plan_v1(&plan).unwrap();
        for (role, kind) in [
            (Qwen3ModelRole::Target8B, ModelMemoryAllocationKind::Weights),
            (Qwen3ModelRole::Draft06B, ModelMemoryAllocationKind::Weights),
            (Qwen3ModelRole::Target8B, ModelMemoryAllocationKind::KvArena),
            (Qwen3ModelRole::Draft06B, ModelMemoryAllocationKind::KvArena),
        ] {
            assert_eq!(
                selected.get(role, kind),
                plan.allocation(role, kind).allocation_id()
            );
        }
        validate_weight_image_length(
            Qwen3ModelRole::Target8B,
            usize::try_from(Qwen3ModelRole::Target8B.tensor_data_bytes()).unwrap(),
        )
        .unwrap();
        validate_weight_image_length(
            Qwen3ModelRole::Draft06B,
            usize::try_from(Qwen3ModelRole::Draft06B.tensor_data_bytes()).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn weight_length_drift_fails_before_kfd() {
        assert!(matches!(
            validate_weight_image_length(Qwen3ModelRole::Draft06B, 7),
            Err(InitializedModelMemoryPreflightErrorV1::WeightImageLength {
                role: Qwen3ModelRole::Draft06B,
                expected,
                actual: 7,
            }) if expected == Qwen3ModelRole::Draft06B.tensor_data_bytes()
        ));
    }

    #[test]
    fn zero_image_builder_returns_an_exact_zero_extent_without_kfd() {
        let image = zeroed_image(4_097).unwrap();
        assert_eq!(image.len(), 4_097);
        assert!(image.iter().all(|byte| *byte == 0));
    }
}
