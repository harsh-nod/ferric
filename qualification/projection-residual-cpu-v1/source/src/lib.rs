#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)]

//! Candidate TP2 projection materialization followed by residual addition.

#[cfg(not(feature = "gfx950"))]
compile_error!("the projection-residual TP2 profile requires gfx950");

#[cfg(not(target_arch = "amdgpu"))]
extern crate alloc;
#[cfg(test)]
extern crate std;

pub mod collective;

pub const ROOTS_V1: [&str; 1] = ["ferric_qwen3_tp_peer_tp2_projection_residual_bf16_v1"];

#[cfg(all(not(target_arch = "amdgpu"), not(test)))]
pub fn compiler_expectation_roster_v1()
-> alloc::vec::Vec<fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1> {
    use fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1 as Entry;
    alloc::vec![Entry::for_marker::<
        collective::ferric_qwen3_tp_peer_tp2_projection_residual_bf16_v1_gpu::Marker,
    >()]
}
