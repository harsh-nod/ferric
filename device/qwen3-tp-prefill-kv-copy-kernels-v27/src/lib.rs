#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)]

//! Isolated aligned sixteen-row TP1 prefill copy candidate; no execution grant.

#[cfg(not(feature = "gfx950"))]
compile_error!("the v27 prefill KV copy profile requires gfx950");

#[cfg(not(target_arch = "amdgpu"))]
extern crate alloc;

pub mod copy;

pub const ROOTS_V27: [&str; 1] = ["ferric_qwen3_tp_prefill16_kv_copy_bf16_v27"];

#[cfg(all(not(target_arch = "amdgpu"), not(test)))]
pub fn compiler_expectation_roster_v27()
-> alloc::vec::Vec<fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1> {
    use fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1 as Entry;
    alloc::vec![Entry::for_marker::<
        copy::ferric_qwen3_tp_prefill16_kv_copy_bf16_v27_gpu::Marker,
    >()]
}
