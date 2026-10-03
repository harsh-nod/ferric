#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)]

//! Closed row-one TP2 arithmetic; peer allocation authority remains host-owned.

#[cfg(not(feature = "gfx950"))]
compile_error!("the v18 TP2 peer profile requires gfx950");

#[cfg(not(target_arch = "amdgpu"))]
extern crate alloc;
#[cfg(test)]
extern crate std;

pub mod collective;
pub mod copy;

pub const ROOTS_V18: [&str; 2] = [
    "ferric_qwen3_tp_peer_copy_bf16_v4",
    "ferric_qwen3_tp_peer_tp2_ordered_residual_bf16_v18",
];

#[cfg(all(not(target_arch = "amdgpu"), not(test)))]
pub fn compiler_expectation_roster_v18()
-> alloc::vec::Vec<fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1> {
    use fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1 as Entry;
    let mut entries = alloc::vec![
        Entry::for_marker::<copy::ferric_qwen3_tp_peer_copy_bf16_v4_gpu::Marker>(),
        Entry::for_marker::<
            collective::ferric_qwen3_tp_peer_tp2_ordered_residual_bf16_v18_gpu::Marker,
        >(),
    ];
    entries.sort_by_key(Entry::kernel_binding_id);
    entries
}
