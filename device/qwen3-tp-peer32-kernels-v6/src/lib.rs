#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)]

//! Ordered TP arithmetic only. Peer allocation authority remains host-owned.

#[cfg(not(feature = "gfx950"))]
compile_error!("the v6 peer32 profile requires gfx950");

#[cfg(not(target_arch = "amdgpu"))]
extern crate alloc;
#[cfg(test)]
extern crate std;

pub mod collective;

pub const ROOTS_V6: [&str; 2] = [
    "ferric_qwen3_tp_batch32_peer_ordered_residual_bf16_v6",
    "ferric_qwen3_tp_batch32_peer_copy_bf16_v6",
];

#[cfg(all(not(target_arch = "amdgpu"), not(test)))]
pub fn compiler_expectation_roster_v6()
-> alloc::vec::Vec<fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1> {
    use fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1 as Entry;
    let mut entries = alloc::vec![
        Entry::for_marker::<
            collective::ferric_qwen3_tp_batch32_peer_ordered_residual_bf16_v6_gpu::Marker,
        >(),
        Entry::for_marker::<collective::ferric_qwen3_tp_batch32_peer_copy_bf16_v6_gpu::Marker>(),
    ];
    entries.sort_by_key(Entry::kernel_binding_id);
    entries
}
