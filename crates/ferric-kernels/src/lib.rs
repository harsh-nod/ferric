#![forbid(unsafe_code)]

//! Finite kernel-profile catalog foundations for the exact Qwen3 M1 envelope.
//!
//! The catalog is structural. It binds exact graph operations, Ferric-owned
//! source declarations, and caller-supplied future compiler/runtime authority
//! requirements. It does not prove that a declared path exists or implements
//! its family, authenticate those authorities, or grant proof, artifact,
//! compilation, load, launch, dispatch,
//! completion, hardware, performance, or qualification authority.

mod catalog;
pub mod m1_kernel_safety;
mod validation;

pub use catalog::{
    build_structural_kernel_catalog, validate_kernel_profile, validate_structural_kernel_catalog,
    KernelAuthorityComponent, KernelAuthorityRequirements, KernelCatalogError, KernelFamily,
    KernelOperationBinding, KernelProfileDescriptor, KernelProfileDisposition,
    KernelSourceDeclaration, StructuralKernelCatalog, FERRIC_KERNEL_SOURCE_DECLARATIONS,
    GFX942_PROCESSOR, GFX942_TARGET_FEATURES, M1_B3_PLAN_BUCKETS, M1_KERNEL_CATALOG_VERSION,
    M1_KERNEL_OPERATION_BINDINGS, M1_KERNEL_PLAN_COUNT,
};
pub use m1_kernel_safety::{
    validate_m1_kernel_memory_safety, validate_m1_kernel_race_freedom,
    validate_m1_kernel_resource_bounds, validate_m1_kernel_safety_certificate,
    M1KernelInitializedRangeV1, M1KernelModeledAccessModeV1, M1KernelModeledAccessV1,
    M1KernelSafetyCertificateErrorV1, M1KernelSafetyCertificateInputV1,
    ValidatedM1KernelSafetyCertificateV1, M1_KERNEL_MAX_INITIALIZED_RANGES_V1,
    M1_KERNEL_MAX_MODELED_ACCESSES_V1,
};
pub use validation::{
    validate_kernel_catalog_input, KernelCatalogAuthorityInputs, KernelCatalogInput,
    KernelCatalogValidationError, ValidatedKernelCatalogInput, VERIFIED_GFX942_PROCESSOR_BYTES,
    VERIFIED_GFX942_TARGET_FEATURE_BYTES,
};

#[allow(unused_imports)]
use vstd::prelude::*;

verus! {

/// Cross-crate verifier view of the finite M1 kernel-profile envelope.
pub open spec fn m1_kernel_profile_is_finite(
    profile: KernelProfileDescriptor,
) -> bool {
    catalog::m1_kernel_profile_is_finite(profile)
}

} // verus!
