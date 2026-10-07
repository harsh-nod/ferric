#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)]

//! Separate query-load hoisting hypothesis; no existing kernel or route changes.

#[cfg(not(feature = "gfx950"))]
compile_error!("the v14 query-hoist attention profile requires gfx950");

#[cfg(not(target_arch = "amdgpu"))]
extern crate alloc;

pub mod attention;

pub const ROOTS_V14: [&str; 1] = ["ferric_qwen3_tp_batch32_wave_paged_gqa_query_hoist_bf16_v14"];

#[cfg(all(not(target_arch = "amdgpu"), not(test)))]
pub fn compiler_expectation_roster_v14()
-> alloc::vec::Vec<fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1> {
    use fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1 as Entry;
    alloc::vec![Entry::for_marker::<
        attention::ferric_qwen3_tp_batch32_wave_paged_gqa_query_hoist_bf16_v14_gpu::Marker,
    >()]
}
