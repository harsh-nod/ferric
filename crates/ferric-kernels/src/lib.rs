#![forbid(unsafe_code)]

//! Finite kernel-profile catalog foundations for the exact Qwen3 M1 envelope.
//!
//! The catalog is structural. It binds exact graph operations, reviewed
//! upstream fixture/model source identities, and caller-supplied future
//! compiler/runtime authority requirements. It does not authenticate those
//! authorities or grant proof, artifact, compilation, load, launch, dispatch,
//! completion, hardware, performance, or qualification authority.

mod catalog;

pub use catalog::{
    build_structural_kernel_catalog, validate_kernel_profile, validate_structural_kernel_catalog,
    KernelAuthorityComponent, KernelAuthorityRequirements, KernelCatalogError, KernelFamily,
    KernelOperationBinding, KernelProfileDescriptor, KernelProfileDisposition,
    ReviewedKernelSource, StructuralKernelCatalog, GFX942_PROCESSOR, GFX942_TARGET_FEATURES,
    M1_B3_PLAN_BUCKETS, M1_KERNEL_CATALOG_VERSION, M1_KERNEL_OPERATION_BINDINGS,
    M1_KERNEL_PLAN_COUNT, REVIEWED_KERNEL_SOURCES,
};
