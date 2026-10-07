#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)]

//! Separate cooperative RMSNorm hypothesis; no existing kernel or route changes.

#[cfg(not(feature = "gfx950"))]
compile_error!("the v15 wave RMSNorm profile requires gfx950");

#[cfg(not(target_arch = "amdgpu"))]
extern crate alloc;

pub mod rmsnorm;

pub const ROOTS_V15: [&str; 1] = ["ferric_qwen3_tp_batch32_wave_rmsnorm_bf16_v15"];

#[cfg(all(not(target_arch = "amdgpu"), not(test)))]
pub fn compiler_expectation_roster_v15()
-> alloc::vec::Vec<fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1> {
    use fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1 as Entry;
    alloc::vec![Entry::for_marker::<
        rmsnorm::ferric_qwen3_tp_batch32_wave_rmsnorm_bf16_v15_gpu::Marker,
    >()]
}
