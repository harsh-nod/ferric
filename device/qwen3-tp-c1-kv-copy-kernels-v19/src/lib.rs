#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)]

//! Independent TP1/C1 copy candidate; not a replacement for multi-row scatter.

#[cfg(not(feature = "gfx950"))]
compile_error!("the v19 C1 KV copy profile requires gfx950");

#[cfg(not(target_arch = "amdgpu"))]
extern crate alloc;

pub mod copy;

pub const ROOTS_V19: [&str; 1] = ["ferric_qwen3_tp_c1_kv_copy_bf16_v19"];

#[cfg(all(not(target_arch = "amdgpu"), not(test)))]
pub fn compiler_expectation_roster_v19()
-> alloc::vec::Vec<fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1> {
    use fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1 as Entry;
    alloc::vec![Entry::for_marker::<
        copy::ferric_qwen3_tp_c1_kv_copy_bf16_v19_gpu::Marker,
    >()]
}
