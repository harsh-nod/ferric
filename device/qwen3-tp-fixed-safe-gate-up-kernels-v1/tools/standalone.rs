//! Dependency-free host/source regression entry point, not SDK qualification.

extern crate alloc;
extern crate self as ferric_qwen3_tp_fixed_safe_gate_up_kernels_v1;

#[path = "../src/fixed_host.rs"]
pub mod fixed_host;
#[path = "../src/host.rs"]
pub mod host;
#[path = "../src/reference_bf16.rs"]
pub mod reference_bf16;

#[path = "../tests/contract.rs"]
mod contract;
#[path = "../tests/build_route.rs"]
mod build_route;
#[path = "../tests/fixed_shape.rs"]
mod fixed_shape;
