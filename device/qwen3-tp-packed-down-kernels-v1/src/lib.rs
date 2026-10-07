#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)]

//! Isolated, unverified TP1/C1 down FP32-output typed-word GEMV proposal. No emitted image or gain.

#[cfg(not(feature = "gfx950"))]
compile_error!("the packed-u32 proposal requires gfx950");

#[cfg(not(target_arch = "amdgpu"))]
extern crate alloc;

pub mod activation_pack;
#[cfg(not(target_arch = "amdgpu"))]
pub mod activation_pack_host;
#[cfg(not(target_arch = "amdgpu"))]
pub mod host;
pub mod projection;

pub const ROOT: &str = "ferric_qwen3_c1_down_wave_gemv_packed_u32_f32_r1";
pub const ACTIVATION_PACK_ROOT: &str = "ferric_qwen3_c1_down_activation_pack_u32_r1";

#[cfg(all(not(target_arch = "amdgpu"), not(test)))]
pub fn compiler_expectation_roster()
-> alloc::vec::Vec<fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1> {
    use fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1 as Entry;
    alloc::vec![
        Entry::for_marker::<projection::ferric_qwen3_c1_down_wave_gemv_packed_u32_f32_r1_gpu::Marker>(
        ),
        Entry::for_marker::<activation_pack::ferric_qwen3_c1_down_activation_pack_u32_r1_gpu::Marker>(
        ),
    ]
}
