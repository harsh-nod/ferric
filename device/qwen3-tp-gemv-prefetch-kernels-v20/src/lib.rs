#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)]

//! Source-only four-pair load scheduling proposal; no emitted artifact or gain.

#[cfg(not(feature = "gfx950"))]
compile_error!("the v20 GEMV prefetch proposal requires gfx950");

#[cfg(not(target_arch = "amdgpu"))]
extern crate alloc;

pub mod projection;

pub const ROOTS_V20: [&str; 2] = [
    "ferric_qwen3_tp_wave_gemv_prefetch4_bf16_v20",
    "ferric_qwen3_tp_wave_gemv_prefetch4_partial_f32_v20",
];

#[cfg(all(not(target_arch = "amdgpu"), not(test)))]
pub fn compiler_expectation_roster_v20()
-> alloc::vec::Vec<fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1> {
    use fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1 as Entry;
    alloc::vec![
        Entry::for_marker::<projection::ferric_qwen3_tp_wave_gemv_prefetch4_bf16_v20_gpu::Marker>(),
        Entry::for_marker::<
            projection::ferric_qwen3_tp_wave_gemv_prefetch4_partial_f32_v20_gpu::Marker,
        >(),
    ]
}
