#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)]

//! Default-off prefill Q/K normalization candidate, not an admitted image.

#[cfg(not(target_arch = "amdgpu"))]
extern crate alloc;

#[cfg(feature = "prefill32-qk")]
pub mod rmsnorm;

#[cfg(feature = "prefill32-qk")]
pub const ROOTS_R1: [&str; 1] = ["ferric_qwen3_prefill32_qk_wave_rmsnorm_bf16_r1"];

#[cfg(all(feature = "prefill32-qk", not(target_arch = "amdgpu"), not(test)))]
pub fn compiler_expectation_roster_r1()
-> alloc::vec::Vec<fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1> {
    use fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1 as Entry;
    alloc::vec![Entry::for_marker::<
        rmsnorm::ferric_qwen3_prefill32_qk_wave_rmsnorm_bf16_r1_gpu::Marker,
    >()]
}
