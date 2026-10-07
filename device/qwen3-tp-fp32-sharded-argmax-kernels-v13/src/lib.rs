#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)]

//! Separate two-stage FP32 argmax prototype, with no adapter route or default change.

#[cfg(not(feature = "gfx950"))]
compile_error!("the v13 sharded FP32 argmax profile requires gfx950");

#[cfg(not(target_arch = "amdgpu"))]
extern crate alloc;
#[cfg(test)]
extern crate std;

pub mod logits;

pub const VOCABULARY_V13: usize = 151936;
pub const MAX_ROWS_V13: usize = 32;
pub const SHARDS_V13: usize = 64;
pub const SCRATCH_BYTES_V13: usize = 2 * MAX_ROWS_V13 * SHARDS_V13 * size_of::<f32>();
pub const ROOTS_V13: [&str; 2] = [
    "ferric_qwen3_tp_batch32_sharded_argmax_produce_f32_v13",
    "ferric_qwen3_tp_batch32_sharded_argmax_finalize_f32_v13",
];

#[cfg(all(not(target_arch = "amdgpu"), not(test)))]
pub fn compiler_expectation_roster_v13()
-> alloc::vec::Vec<fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1> {
    use fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1 as Entry;
    alloc::vec![
        Entry::for_marker::<
            logits::ferric_qwen3_tp_batch32_sharded_argmax_produce_f32_v13_gpu::Marker,
        >(),
        Entry::for_marker::<
            logits::ferric_qwen3_tp_batch32_sharded_argmax_finalize_f32_v13_gpu::Marker,
        >(),
    ]
}
