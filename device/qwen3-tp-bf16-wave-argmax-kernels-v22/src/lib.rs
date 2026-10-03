#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)]

//! Additive single-request BF16 Wave64 argmax. Projection arithmetic is unchanged.

#[cfg(not(feature = "gfx950"))]
compile_error!("the v22 BF16 Wave64 argmax profile requires gfx950");

#[cfg(not(target_arch = "amdgpu"))]
extern crate alloc;
#[cfg(test)]
extern crate std;

pub mod logits;

pub const ROOTS_V22: [&str; 1] = ["ferric_qwen3_tp_single_wave_argmax_bf16_v22"];

#[cfg(all(not(target_arch = "amdgpu"), not(test)))]
pub fn compiler_expectation_roster_v22()
-> alloc::vec::Vec<fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1> {
    use fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1 as Entry;
    alloc::vec![Entry::for_marker::<
        logits::ferric_qwen3_tp_single_wave_argmax_bf16_v22_gpu::Marker,
    >(),]
}
