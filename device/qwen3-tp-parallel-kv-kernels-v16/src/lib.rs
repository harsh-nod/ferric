#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)]

//! Separate exact-copy parallel KV append hypothesis; existing roots stay unchanged.

#[cfg(not(feature = "gfx950"))]
compile_error!("the v16 parallel KV append profile requires gfx950");

#[cfg(not(target_arch = "amdgpu"))]
extern crate alloc;

pub mod append;

pub const ROOTS_V16: [&str; 1] = ["ferric_qwen3_tp_batch_parallel_kv_append_v16"];

#[cfg(all(not(target_arch = "amdgpu"), not(test)))]
pub fn compiler_expectation_roster_v16()
-> alloc::vec::Vec<fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1> {
    use fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1 as Entry;
    alloc::vec![Entry::for_marker::<
        append::ferric_qwen3_tp_batch_parallel_kv_append_v16_gpu::Marker,
    >()]
}
