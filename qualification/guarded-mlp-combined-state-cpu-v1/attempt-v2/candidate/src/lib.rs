#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)]

//! Additive candidate: one atomic state/guard allocation and generation-gated TP2 R2.

#[cfg(not(feature = "gfx950"))]
compile_error!("the guarded MLP segment profile requires gfx950");

#[cfg(not(target_arch = "amdgpu"))]
extern crate alloc;
#[cfg(test)]
extern crate std;

#[macro_use]
mod arithmetic;
#[macro_use]
mod guard;
pub mod kernels;

pub const MLP_STATE_WORDS_V2: usize = 548;
pub const MLP_GUARD_WORDS_V2: usize = 4;
pub const MLP_GUARD_OFFSET_WORDS_V2: usize = 548;
pub const MLP_COMBINED_WORDS_V2: usize = 552;

pub const ROOTS_V2: [&str; 2] = [
    "ferric_qwen3_mlp_state_guard_v2",
    "ferric_qwen3_tp2_guarded_projection_residual_bf16_v2",
];

#[cfg(all(not(target_arch = "amdgpu"), not(test)))]
pub fn compiler_expectation_roster_v2()
-> alloc::vec::Vec<fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1> {
    use fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1 as Entry;
    alloc::vec![
        Entry::for_marker::<kernels::ferric_qwen3_mlp_state_guard_v2_gpu::Marker>(),
        Entry::for_marker::<
            kernels::ferric_qwen3_tp2_guarded_projection_residual_bf16_v2_gpu::Marker,
        >(),
    ]
}

#[cfg(test)]
mod guard_tests;

#[cfg(test)]
mod combined_state_tests;
