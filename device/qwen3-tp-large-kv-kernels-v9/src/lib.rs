#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)]

//! Additive TP1 physical KV capacity; logical context and arithmetic are unchanged.

#[cfg(not(feature = "gfx950"))]
compile_error!("the v9 large-KV profile requires gfx950");

#[cfg(not(target_arch = "amdgpu"))]
extern crate alloc;
#[cfg(test)]
extern crate std;

pub mod append;
pub mod attention;
pub mod contract;

pub const ROOTS_V9: [&str; 2] = [
    "ferric_qwen3_tp_batch32_large_kv_append_v9",
    "ferric_qwen3_tp_batch32_large_kv_paged_gqa_bf16_f32_v9",
];

#[cfg(all(not(target_arch = "amdgpu"), not(test)))]
pub fn compiler_expectation_roster_v9()
-> alloc::vec::Vec<fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1> {
    use fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1 as Entry;
    let mut entries = alloc::vec![
        Entry::for_marker::<append::ferric_qwen3_tp_batch32_large_kv_append_v9_gpu::Marker>(),
        Entry::for_marker::<attention::ferric_qwen3_tp_batch32_large_kv_paged_gqa_bf16_f32_v9_gpu::Marker>(),
    ];
    entries.sort_by_key(Entry::kernel_binding_id);
    entries
}
