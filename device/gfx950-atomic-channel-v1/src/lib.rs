#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)] // The kernel attribute emits an internal helper module.

//! Fixed indexed atomic storage fixture, not a tensor-publication protocol.
//!
//! Each invocation accesses only its own cell. A passing execution establishes
//! the tested slice ABI, guarded indexing and exact u32 data path; it does not
//! establish inter-workgroup read-from relationships or model inference.

use core::sync::atomic::{AtomicU32, Ordering};
use fe2o3_device::{DisjointSlice, WriteOnlyDisjointSlice, kernel, thread};

pub const KERNEL_SYMBOL: &str = "ferric_gfx950_atomic_channel_v1";
pub const WORDS: usize = 256;
pub const WORKGROUP: [u32; 3] = [128, 1, 1];
pub const AQL_GRID: [u32; 3] = [256, 1, 1];
pub const EXPLICIT_KERNARG_BYTES: usize = 48;

/// Preserve every input bit through an indexed Release-store/Acquire-load pair.
///
/// All three allocations are disjoint and remain live through exact completion.
/// The atomic slice requires its own authenticated extent and memory contract;
/// the signature must never be represented as an ordinary readonly u32 slice.
#[kernel(
    typed,
    launch(required = [128, 1, 1], max = [128, 1, 1], max_grid = [2, 1, 1])
)]
pub fn ferric_gfx950_atomic_channel_v1(
    channels: &[AtomicU32],
    mut inputs: DisjointSlice<u32>,
    mut output: WriteOnlyDisjointSlice<u32>,
) {
    if channels.len() != WORDS || inputs.len() != WORDS || output.len() != WORDS {
        fe2o3_device::trap();
    }
    let index = thread::index_1d();
    let position = index.get();
    if position >= WORDS {
        return;
    }
    // An explicit exclusive input capability proves separation from the shared
    // atomic allocation. This kernel only reads its owned input element.
    let Some(input) = inputs.get_mut(thread::index_1d()) else {
        fe2o3_device::trap();
    };
    let value = *input;
    // Ordinary Rust slice guards must remain attached to the atomic address.
    channels[position].store(value, Ordering::Release);
    let observed = channels[position].load(Ordering::Acquire);
    if !output.write(index, observed) {
        fe2o3_device::trap();
    }
}
