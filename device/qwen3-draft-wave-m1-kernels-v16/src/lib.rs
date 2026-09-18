#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)]

//! Opt-in draft M=1 engineering candidate, not a qualified M1 catalog extension.
//! Wave reduction changes association; native numerical and token parity are pending.

#[cfg(not(feature = "gfx950"))]
compile_error!("the v16 draft wave M1 profile requires gfx950");

#[cfg(not(target_arch = "amdgpu"))]
extern crate alloc;

pub mod contract;
pub mod projection;

pub use contract::ROOTS_V16;

#[cfg(all(not(target_arch = "amdgpu"), not(test)))]
pub fn compiler_expectation_roster_v16()
-> alloc::vec::Vec<fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1> {
    use fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1 as Entry;
    let mut entries = alloc::vec![
        Entry::for_marker::<projection::ferric_qwen3_draft_wave_m1_gemv_bf16_v16_gpu::Marker>(),
        Entry::for_marker::<projection::ferric_qwen3_draft_wave_m1_gemv_f32_v16_gpu::Marker>(),
    ];
    entries.sort_by_key(Entry::kernel_binding_id);
    entries
}
