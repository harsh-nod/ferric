#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)]

//! Additive, opt-in Wave64 finite FP32 argmax. No projection arithmetic changes.

#[cfg(not(feature = "gfx950"))]
compile_error!("the v11 parallel FP32 argmax profile requires gfx950");

#[cfg(not(target_arch = "amdgpu"))]
extern crate alloc;
#[cfg(test)]
extern crate std;

pub mod logits;

pub const ROOTS_V11: [&str; 1] = ["ferric_qwen3_tp_batch32_wave_argmax_f32_v11"];

#[cfg(all(not(target_arch = "amdgpu"), not(test)))]
pub fn compiler_expectation_roster_v11()
-> alloc::vec::Vec<fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1> {
    use fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1 as Entry;
    alloc::vec![Entry::for_marker::<
        logits::ferric_qwen3_tp_batch32_wave_argmax_f32_v11_gpu::Marker,
    >(),]
}
