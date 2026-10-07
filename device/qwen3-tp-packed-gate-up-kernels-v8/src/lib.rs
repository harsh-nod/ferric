#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)]

//! Default-off, fixed-shape fused gate/up experiment. No emitted image or gain.

#[cfg(all(target_arch = "amdgpu", not(feature = "fused-gate-up-r1")))]
compile_error!("device emission requires explicit fused-gate-up-r1 selection");

#[cfg(not(target_arch = "amdgpu"))]
extern crate alloc;

#[cfg(feature = "fused-gate-up-r1")]
pub mod activation_pack;
#[cfg(feature = "fused-gate-up-r1")]
pub mod fused_gate_up;
#[cfg(not(target_arch = "amdgpu"))]
pub mod host;

pub const EXPERIMENT_ENABLED: bool = cfg!(feature = "fused-gate-up-r1");
pub const FUSED_ROOT: &str = "ferric_qwen3_c1_gate_up_packed_u32_bf16_v8";
pub const PACK_ROOT: &str = "ferric_qwen3_c1_gate_up_activation_pack_u32_v8";

#[cfg(all(feature = "fused-gate-up-r1", not(target_arch = "amdgpu"), not(test)))]
pub fn compiler_expectation_roster()
-> alloc::vec::Vec<fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1> {
    use fe2o3_host::CompilerGeneratedKernelExpectationRosterEntryV1 as Entry;
    alloc::vec![
        Entry::for_marker::<fused_gate_up::ferric_qwen3_c1_gate_up_packed_u32_bf16_v8_gpu::Marker>(
        ),
        Entry::for_marker::<
            activation_pack::ferric_qwen3_c1_gate_up_activation_pack_u32_v8_gpu::Marker,
        >(),
    ]
}
