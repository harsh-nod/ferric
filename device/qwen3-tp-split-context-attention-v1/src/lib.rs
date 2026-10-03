#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)]

//! Two-root sidecar; independent std-only CPU tests are outside this crate.

#[cfg(not(feature = "gfx950"))]
compile_error!("split-context attention requires gfx950");

#[cfg(not(target_arch = "amdgpu"))]
extern crate alloc;

pub mod attention;

pub const ROOTS_SPLIT_CONTEXT_V1: [&str; 2] = [
    "ferric_qwen3_tp_split_context_partial_bf16_v1",
    "ferric_qwen3_tp_split_context_merge_bf16_v1",
];

#[cfg(all(not(target_arch = "amdgpu"), not(test)))]
pub fn compiler_expectation_roster_split_context_v1()
-> alloc::vec::Vec<fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1> {
    use fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1 as Entry;
    let mut entries = alloc::vec![
        Entry::for_marker::<attention::ferric_qwen3_tp_split_context_partial_bf16_v1_gpu::Marker>(),
        Entry::for_marker::<attention::ferric_qwen3_tp_split_context_merge_bf16_v1_gpu::Marker>(),
    ];
    entries.sort_by_key(Entry::kernel_binding_id);
    entries
}
