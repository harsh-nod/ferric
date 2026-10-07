#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)]

//! Isolated matched prefill32 down experiment, not an admitted serving route.

#[cfg(all(target_arch = "amdgpu", not(feature = "paired-prefill32-down-k16-r1")))]
compile_error!("device emission requires explicit paired-prefill32-down-k16-r1 selection");

#[cfg(not(target_arch = "amdgpu"))]
extern crate alloc;

pub mod contract;

#[cfg(feature = "paired-prefill32-down-k16-r1")]
pub mod projection;

pub const EXPERIMENT_ENABLED: bool = cfg!(feature = "paired-prefill32-down-k16-r1");
pub const CONTROL_ROOT: &str = "ferric_qwen3_prefill32_down_k16_control_f32_r1";
pub const PAIRED_ROOT: &str = "ferric_qwen3_prefill32_down_k16_paired_f32_r1";

#[cfg(all(
    feature = "paired-prefill32-down-k16-r1",
    not(target_arch = "amdgpu"),
    not(test)
))]
pub fn compiler_expectation_roster()
-> alloc::vec::Vec<fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1> {
    use fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1 as Entry;
    let mut entries = alloc::vec![
        Entry::for_marker::<projection::ferric_qwen3_prefill32_down_k16_control_f32_r1_gpu::Marker>(),
        Entry::for_marker::<projection::ferric_qwen3_prefill32_down_k16_paired_f32_r1_gpu::Marker>(),
    ];
    entries.sort_by_key(Entry::kernel_binding_id);
    entries
}
