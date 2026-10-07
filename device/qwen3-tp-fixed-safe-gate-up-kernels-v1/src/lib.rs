#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)]

//! Default-off fixed-shape experiment; not an emitted image or model route.

#[cfg(all(target_arch = "amdgpu", not(feature = "fixed-safe-gate-up-r1")))]
compile_error!("device emission requires explicit fixed-safe-gate-up-r1 selection");

#[cfg(not(target_arch = "amdgpu"))]
extern crate alloc;

#[cfg(feature = "fixed-safe-gate-up-r1")]
pub mod activation_pack;
#[cfg(feature = "fixed-safe-gate-up-r1")]
pub mod projection;
#[cfg(not(target_arch = "amdgpu"))]
pub mod fixed_host;
#[cfg(not(target_arch = "amdgpu"))]
pub mod host;
#[cfg(not(target_arch = "amdgpu"))]
pub mod reference_bf16;

pub const EXPERIMENT_ENABLED: bool = cfg!(feature = "fixed-safe-gate-up-r1");
pub const ROOT: &str = "ferric_qwen3_c1_gate_up_fixed_safe_u32_bf16_r1";
pub const ACTIVATION_PACK_ROOT: &str = "ferric_qwen3_c1_activation_pack_u32_r1";

#[cfg(all(feature = "fixed-safe-gate-up-r1", not(target_arch = "amdgpu"), not(test)))]
pub fn compiler_expectation_roster()
-> alloc::vec::Vec<fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1> {
    use fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1 as Entry;
    alloc::vec![
        Entry::for_marker::<
            projection::ferric_qwen3_c1_gate_up_fixed_safe_u32_bf16_r1_gpu::Marker,
        >(),
        Entry::for_marker::<
            activation_pack::ferric_qwen3_c1_activation_pack_u32_r1_gpu::Marker,
        >(),
    ]
}
