#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)]

//! Non-authoritative host-test wrapper for the aggregate-owned RMSNorm source.

#[path = "../../qwen3-all-kernels-v1/src/rmsnorm.rs"]
mod kernels;

pub use kernels::*;
