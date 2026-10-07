#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)]

//! Separate, opt-in TP1 final-head precision. No other model arithmetic changes.

#[cfg(not(feature = "gfx950"))]
compile_error!("the v8 FP32 head32 profile requires gfx950");

#[cfg(not(target_arch = "amdgpu"))]
extern crate alloc;
#[cfg(test)]
extern crate std;

pub mod logits;
pub mod projection;

pub const ROOTS_V8: [&str; 3] = [
    "ferric_qwen3_tp_batch32_head_bf16_f32_v8",
    "ferric_qwen3_tp_batch32_mfma_head_f32_v8",
    "ferric_qwen3_tp_batch32_argmax_f32_v8",
];

#[cfg(all(not(target_arch = "amdgpu"), not(test)))]
pub fn compiler_expectation_roster_v8()
-> alloc::vec::Vec<fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1> {
    use fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1 as Entry;
    let mut entries = alloc::vec![
        Entry::for_marker::<projection::ferric_qwen3_tp_batch32_head_bf16_f32_v8_gpu::Marker>(),
        Entry::for_marker::<projection::ferric_qwen3_tp_batch32_mfma_head_f32_v8_gpu::Marker>(),
        Entry::for_marker::<logits::ferric_qwen3_tp_batch32_argmax_f32_v8_gpu::Marker>(),
    ];
    entries.sort_by_key(Entry::kernel_binding_id);
    entries
}
