#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)]

//! Non-authoritative host-test wrapper for the aggregate-owned RMSNorm source.

mod target {
    pub const QWEN3_DEVICE_TARGET_V1: &str = "gfx942:xnack-";
}

#[path = "../../qwen3-all-kernels-v1/src/rmsnorm.rs"]
mod kernels;

pub use kernels::*;
