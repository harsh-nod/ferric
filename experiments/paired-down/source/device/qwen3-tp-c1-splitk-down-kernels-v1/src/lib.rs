#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)]

//! Default-off TP1/C1 down projection experiment, not an admitted model route.

#[cfg(all(target_arch = "amdgpu", not(feature = "splitk8-down-r1")))]
compile_error!("device emission requires explicit splitk8-down-r1 selection");

#[cfg(not(target_arch = "amdgpu"))]
extern crate alloc;

#[cfg(feature = "splitk8-down-r1")]
pub mod projection;

pub const EXPERIMENT_ENABLED: bool = cfg!(feature = "splitk8-down-r1");
pub const PARTIAL_ROOT: &str = "ferric_qwen3_c1_down_splitk8_mfma_partial_f32_r1";
pub const MERGE_ROOT: &str = "ferric_qwen3_c1_down_splitk8_merge_f32_r1";
pub const PARTIAL_ELEMENTS: usize = 8 * 4096;
pub const PARTIAL_BYTES: usize = PARTIAL_ELEMENTS * 4;

#[cfg(all(feature = "splitk8-down-r1", not(target_arch = "amdgpu"), not(test)))]
pub fn compiler_expectation_roster()
-> alloc::vec::Vec<fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1> {
    use fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1 as Entry;
    alloc::vec![
        Entry::for_marker::<projection::ferric_qwen3_c1_down_splitk8_mfma_partial_f32_r1_gpu::Marker>(
        ),
        Entry::for_marker::<projection::ferric_qwen3_c1_down_splitk8_merge_f32_r1_gpu::Marker>(),
    ]
}
