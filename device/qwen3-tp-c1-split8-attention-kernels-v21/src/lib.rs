#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)]

//! Isolated TP1/C1 eight-partition attention proposal with reassociated softmax.
//! Neither these sources nor their host models qualify a numerical profile.

#[cfg(not(feature = "gfx950"))]
compile_error!("the v21 split-context attention proposal requires gfx950");

#[cfg(not(target_arch = "amdgpu"))]
extern crate alloc;

pub mod attention;

pub const ROOTS_V21: [&str; 2] = [
    "ferric_qwen3_tp_c1_split8_attention_partial_f32_v21",
    "ferric_qwen3_tp_c1_split8_attention_merge_bf16_v21",
];

pub const STATS_ELEMENTS_V21: usize = 32 * 8 * 2;
pub const NUMERATOR_ELEMENTS_V21: usize = 32 * 8 * 128;
pub const SCRATCH_BYTES_V21: usize = (STATS_ELEMENTS_V21 + NUMERATOR_ELEMENTS_V21) * 4;
pub const PARTIAL_IMPLICIT_OFFSET_V21: usize = 136;
pub const PARTIAL_KERNARG_BYTES_V21: usize = PARTIAL_IMPLICIT_OFFSET_V21 + 256;
pub const MERGE_IMPLICIT_OFFSET_V21: usize = 48;
pub const MERGE_KERNARG_BYTES_V21: usize = MERGE_IMPLICIT_OFFSET_V21 + 256;

#[cfg(all(not(target_arch = "amdgpu"), not(test)))]
pub fn compiler_expectation_roster_v21()
-> alloc::vec::Vec<fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1> {
    use fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1 as Entry;
    alloc::vec![
        Entry::for_marker::<
            attention::ferric_qwen3_tp_c1_split8_attention_partial_f32_v21_gpu::Marker,
        >(),
        Entry::for_marker::<
            attention::ferric_qwen3_tp_c1_split8_attention_merge_bf16_v21_gpu::Marker,
        >(),
    ]
}
