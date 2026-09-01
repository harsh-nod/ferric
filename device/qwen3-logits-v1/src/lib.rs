#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)]

//! Non-authoritative host-test wrapper for the aggregate-owned K7 source.

#[path = "../../qwen3-all-kernels-v1/src/logits.rs"]
mod kernels;

pub use kernels::*;
