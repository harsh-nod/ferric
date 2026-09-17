#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)] // The kernel macro emits an internal helper module.

//! One finite consumer-clear publication attempt, never a progress guarantee.

use core::sync::atomic::AtomicU32;
use fe2o3_device::{DisjointSlice, WriteOnlyDisjointSlice, kernel, publish_once_128, thread};

pub const KERNEL_SYMBOL: &str = "ferric_gfx950_static_publication_v1";
pub const CELLS: usize = 128;
pub const INVOCATIONS: usize = 256;
pub const EXPLICIT_KERNARG_BYTES: usize = 80;

/// The two groups write distinct status/value slots, including NotReady.
/// Payload/flags retain the consuming primitive's full custody requirements.
#[kernel(
    typed,
    launch(required = [128, 1, 1], max = [128, 1, 1], max_grid = [2, 1, 1])
)]
pub fn ferric_gfx950_static_publication_v1(
    payload: DisjointSlice<f32>,
    flags: &[AtomicU32],
    input: DisjointSlice<f32>,
    mut statuses: WriteOnlyDisjointSlice<u32>,
    mut values: WriteOnlyDisjointSlice<f32>,
) {
    if input.len() != CELLS || statuses.len() != INVOCATIONS || values.len() != INVOCATIONS {
        fe2o3_device::trap();
    }
    let input = input.into_read_only();
    let position = thread::index_1d().get();
    let producer_value = input.load_or(position % CELLS, 0.0);
    // The facade owns shape rejection and the exact WG0/WG1 protocol. Do not
    // pre-reject payload/flag lengths: the finite invalid-shape cases observe it.
    let attempt = publish_once_128(payload, flags, producer_value);
    if !statuses.write(thread::index_1d(), attempt.status) {
        fe2o3_device::trap();
    }
    if !values.write(thread::index_1d(), attempt.value) {
        fe2o3_device::trap();
    }
}
