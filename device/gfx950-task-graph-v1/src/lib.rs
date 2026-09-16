#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)] // The kernel attribute emits an internal helper module.

//! Finite engineering scheduler, not a model backend or a qualified runtime.
//!
//! Two workgroups share a seven-bit ready queue. Each task uses all 128 lanes
//! of its claiming workgroup and two unconditional LDS publication phases.
//! Payloads are atomic integers: this does not establish ordinary tensor
//! read-from authority or a general cross-workgroup memory proof.

use fe2o3_device::atomic::Ordering;
use fe2o3_device::{
    DeviceGlobalMutPtr, StridedReadView2D, WorkgroupLdsScope, WorkgroupPipeline, kernel, thread,
};

pub const KERNEL_SYMBOL: &str = "ferric_gfx950_task_graph_v1";
pub const TASKS: usize = 7;
pub const LANES: usize = 128;
pub const ITERATIONS: usize = 16;
pub const EMPTY_PROBES: usize = 8;
pub const INPUT_WORDS: usize = TASKS * LANES;
pub const MAX_INPUT: u32 = 1024;
pub const ALL_TASKS: u32 = 127;
pub const EXPLICIT_KERNARG_BYTES: usize = 136;
pub const LDS_BYTES: usize = 1024;
pub const WORKGROUP: [u32; 3] = [128, 1, 1];
pub const AQL_GRID: [u32; 3] = [256, 1, 1];
pub const ERROR_STALE_EPOCH: u32 = 1;
pub const ERROR_DUPLICATE: u32 = 2;
pub const ERROR_INVALID: u32 = 4;
/// Execute `0 -> {1,2} -> 3 -> {4,5} -> 6` with a bounded ready-bit queue.
///
/// Inputs contain seven rows of 128 values in `0..=1024`. Task payload is
/// its row sum plus its immediate predecessor payloads. The largest valid
/// result is 13*128*1024, so all arithmetic is exact in u32.
///
/// All thirteen pointer arguments name distinct, aligned, coherent u32 atomic
/// objects. The host initializes epoch to config[0], ready to1, and every
/// other object to0. The nonzero epoch is immutable during a dispatch and its
/// storage cannot be reused until all workgroups have quiesced. Stale config
/// records ERROR_STALE_EPOCH without queue or payload mutation.
///
/// The ready mask is an unordered bounded queue. A successful fetch_and claim
/// removes one bit exactly once. An unsuccessful attempt witnesses another
/// claimant of a previously unobserved task. Each worker has at most seven
/// nonempty attempts and eight empty probes, fitting sixteen fixed rounds.
/// Empty probes permit another resident worker to publish work; they never
/// wait for a particular task or assume that another workgroup is resident.
/// An active owner publishes successors before its next attempt. One resident
/// workgroup can finish the entire graph without waiting for another one.
#[kernel(
    typed,
    launch(
        required = [128, 1, 1],
        max = [128, 1, 1],
        max_grid = [2, 1, 1],
        static_shared_memory_bytes = 1024
    ),
    control_flow(loop_bounds(16, 7, 128))
)]
pub fn ferric_gfx950_task_graph_v1(
    inputs: &[u32],
    config: &[u32],
    epoch: DeviceGlobalMutPtr<u32>,
    ready: DeviceGlobalMutPtr<u32>,
    done: DeviceGlobalMutPtr<u32>,
    claimed: DeviceGlobalMutPtr<u32>,
    owners: DeviceGlobalMutPtr<u32>,
    errors: DeviceGlobalMutPtr<u32>,
    payload0: DeviceGlobalMutPtr<u32>,
    payload1: DeviceGlobalMutPtr<u32>,
    payload2: DeviceGlobalMutPtr<u32>,
    payload3: DeviceGlobalMutPtr<u32>,
    payload4: DeviceGlobalMutPtr<u32>,
    payload5: DeviceGlobalMutPtr<u32>,
    payload6: DeviceGlobalMutPtr<u32>,
) {
    if inputs.len() != INPUT_WORDS || config.len() != 1 {
        fe2o3_device::trap();
    }
    let input_view = if let Ok(view) =
        StridedReadView2D::from_shared_slice(inputs, 0, 1, INPUT_WORDS, INPUT_WORDS)
    {
        view
    } else {
        fe2o3_device::trap()
    };
    // The typed linear index retains invocation evidence for LDS projection.
    // With the exact 128-lane launch, quotient/remainder are WG/local IDs.
    let global_index = thread::index_1d().get();
    let lane = global_index % LANES;
    let worker = (global_index / LANES) as u32;
    if lane >= LANES || worker >= 2 {
        fe2o3_device::trap();
    }
    let expected_epoch = config[0];
    let owner = if worker == 0 { 1u32 } else { 2u32 };
    let mut scope = WorkgroupLdsScope::current();
    let mut pipeline = WorkgroupPipeline::<u32, 2, 128, 1>::current(&mut scope);
    let mut retired = false;
    let mut empty_probes = 0usize;
    let mut iteration = 0usize;
    while iteration < ITERATIONS {
        // Only the WG leader arbitrates the ready queue. Zero encodes idle;
        // 1..=7 encodes a claimed task. Every lane still enters both phases.
        let mut selected = 0u32;
        if lane == 0 && !retired {
            let current_epoch = epoch.as_atomic().fetch_or(0, Ordering::Acquire);
            if expected_epoch == 0 || current_epoch != expected_epoch {
                errors
                    .as_atomic()
                    .fetch_or(ERROR_STALE_EPOCH, Ordering::Relaxed);
                retired = true;
            } else {
                let snapshot = ready.as_atomic().fetch_or(0, Ordering::Acquire);
                if snapshot & !ALL_TASKS != 0 {
                    errors
                        .as_atomic()
                        .fetch_or(ERROR_INVALID, Ordering::Relaxed);
                    retired = true;
                } else if snapshot == 0 {
                    if empty_probes < EMPTY_PROBES {
                        empty_probes += 1;
                    }
                    if empty_probes == EMPTY_PROBES {
                        retired = true;
                    }
                } else {
                    let mut task = 0u32;
                    let mut scan = 0u32;
                    let mut found = false;
                    let mut bit = 1u32;
                    // Fixed seven-entry priority scan: source cttz lowering is
                    // not yet available, and this has one canonical loop exit.
                    while scan < 7 {
                        if !found && snapshot & bit != 0 {
                            task = scan;
                            found = true;
                        }
                        scan += 1;
                        bit <<= 1;
                    }
                    if task >= 7 {
                        errors
                            .as_atomic()
                            .fetch_or(ERROR_INVALID, Ordering::Relaxed);
                        retired = true;
                    } else {
                        let bit = 1u32 << task;
                        let previous = ready.as_atomic().fetch_and(!bit, Ordering::AcqRel);
                        if previous & bit != 0 {
                            let prior_claims = claimed.as_atomic().fetch_or(bit, Ordering::AcqRel);
                            if prior_claims & bit != 0 {
                                errors
                                    .as_atomic()
                                    .fetch_or(ERROR_DUPLICATE, Ordering::Relaxed);
                                retired = true;
                            } else {
                                owners
                                    .as_atomic()
                                    .fetch_or(owner << (2 * task), Ordering::Relaxed);
                                selected = task + 1;
                            }
                        }
                    }
                }
            }
        }
        let dispatch_phase = 2 * iteration;
        pipeline.stage(dispatch_phase);
        pipeline.write(dispatch_phase, lane, selected);
        pipeline.commit(dispatch_phase);
        pipeline.wait(dispatch_phase);
        pipeline.consume(dispatch_phase);
        let task_token = pipeline.read(dispatch_phase, 0);
        pipeline.release(dispatch_phase);

        // An invalid lane publishes a sentinel so every lane's reduction
        // rejects this task, without skipping any collective phase.
        let mut contribution = 0u32;
        if task_token > 0 && task_token <= 7 {
            let row_base = if task_token == 1 {
                0usize
            } else if task_token == 2 {
                128
            } else if task_token == 3 {
                256
            } else if task_token == 4 {
                384
            } else if task_token == 5 {
                512
            } else if task_token == 6 {
                640
            } else {
                768
            };
            let offset = row_base + lane;
            // A total checked load cannot introduce a lane-dependent panic
            // before the next collective. Invalid coordinates use the same
            // sentinel as invalid input values and cannot publish success.
            let value = input_view.load_or(0, offset, MAX_INPUT + 1);
            if value <= MAX_INPUT {
                contribution = value;
            } else {
                contribution = MAX_INPUT + 1;
                errors
                    .as_atomic()
                    .fetch_or(ERROR_INVALID, Ordering::Relaxed);
            }
        }
        let reduction_phase = dispatch_phase + 1;
        pipeline.stage(reduction_phase);
        pipeline.write(reduction_phase, lane, contribution);
        pipeline.commit(reduction_phase);
        pipeline.wait(reduction_phase);
        pipeline.consume(reduction_phase);
        let mut sum = 0u32;
        let mut arithmetic_valid = true;
        let mut column = 0usize;
        while column < LANES {
            let value = pipeline.read(reduction_phase, column);
            if value <= MAX_INPUT {
                // Two u32 values fit in u64; narrow only after checking the
                // complete sum, including malformed externally supplied state.
                let next = sum as u64 + value as u64;
                if next <= u32::MAX as u64 {
                    sum = next as u32;
                } else {
                    arithmetic_valid = false;
                }
            } else {
                arithmetic_valid = false;
            }
            column += 1;
        }
        pipeline.release(reduction_phase);

        if lane == 0 && task_token > 0 && task_token <= 7 {
            // Completion uses the broadcast token directly, with one exact
            // payload slot and completion bit for each of its seven values.
            // RMW reads and writes keep the entire cross-WG payload atomic.
            // They are intentionally not a substitute for normal tensor HB.
            if task_token == 2 || task_token == 3 {
                let next = sum as u64 + payload0.as_atomic().fetch_or(0, Ordering::Acquire) as u64;
                if next <= u32::MAX as u64 {
                    sum = next as u32;
                } else {
                    arithmetic_valid = false;
                }
            } else if task_token == 4 {
                let next = sum as u64 + payload1.as_atomic().fetch_or(0, Ordering::Acquire) as u64;
                if next <= u32::MAX as u64 {
                    sum = next as u32;
                } else {
                    arithmetic_valid = false;
                }
                let next = sum as u64 + payload2.as_atomic().fetch_or(0, Ordering::Acquire) as u64;
                if next <= u32::MAX as u64 {
                    sum = next as u32;
                } else {
                    arithmetic_valid = false;
                }
            } else if task_token == 5 || task_token == 6 {
                let next = sum as u64 + payload3.as_atomic().fetch_or(0, Ordering::Acquire) as u64;
                if next <= u32::MAX as u64 {
                    sum = next as u32;
                } else {
                    arithmetic_valid = false;
                }
            } else if task_token == 7 {
                let next = sum as u64 + payload4.as_atomic().fetch_or(0, Ordering::Acquire) as u64;
                if next <= u32::MAX as u64 {
                    sum = next as u32;
                } else {
                    arithmetic_valid = false;
                }
                let next = sum as u64 + payload5.as_atomic().fetch_or(0, Ordering::Acquire) as u64;
                if next <= u32::MAX as u64 {
                    sum = next as u32;
                } else {
                    arithmetic_valid = false;
                }
            }
            if arithmetic_valid {
                let bit;
                if task_token == 1 {
                    payload0.as_atomic().swap(sum, Ordering::Release);
                    bit = 1;
                } else if task_token == 2 {
                    payload1.as_atomic().swap(sum, Ordering::Release);
                    bit = 2;
                } else if task_token == 3 {
                    payload2.as_atomic().swap(sum, Ordering::Release);
                    bit = 4;
                } else if task_token == 4 {
                    payload3.as_atomic().swap(sum, Ordering::Release);
                    bit = 8;
                } else if task_token == 5 {
                    payload4.as_atomic().swap(sum, Ordering::Release);
                    bit = 16;
                } else if task_token == 6 {
                    payload5.as_atomic().swap(sum, Ordering::Release);
                    bit = 32;
                } else {
                    payload6.as_atomic().swap(sum, Ordering::Release);
                    bit = 64;
                }

                let old_done = done.as_atomic().fetch_or(bit, Ordering::AcqRel);
                let completed = old_done | bit;
                let mut successors = 0u32;
                if task_token == 1 {
                    successors = 6;
                } else if (task_token == 2 || task_token == 3) && completed & 6 == 6 {
                    successors = 8;
                } else if task_token == 4 {
                    successors = 48;
                } else if (task_token == 5 || task_token == 6) && completed & 48 == 48 {
                    successors = 64;
                }
                // The exact old value identifies the unique last predecessor.
                // A later done read would allow duplicate fan-in publication.
                if old_done & bit != 0 {
                    errors
                        .as_atomic()
                        .fetch_or(ERROR_DUPLICATE, Ordering::Relaxed);
                } else if successors != 0 {
                    ready.as_atomic().fetch_or(successors, Ordering::Release);
                }
            } else {
                errors
                    .as_atomic()
                    .fetch_or(ERROR_INVALID, Ordering::Relaxed);
            }
        }
        iteration += 1;
    }
}
