use core::sync::atomic::{AtomicU32, Ordering};
use fe2o3_device::{Bf16, WriteOnlyDisjointSlice, kernel, memory, thread};

macro_rules! load_atomic_word_v1 {
    ($words:expr, $index:expr) => {
        $words[$index].load(Ordering::Acquire)
    };
}

macro_rules! store_atomic_word_v1 {
    ($words:expr, $index:expr, $value:expr, $ordering:expr) => {
        $words[$index].store($value, $ordering)
    };
}

#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [1, 1, 1]))]
pub fn ferric_qwen3_mlp_state_guard_v1(
    state: &[AtomicU32],
    guard: &[AtomicU32],
    generation_lo: u32,
    generation_hi: u32,
) {
    if state.len() != 548
        || guard.len() != 4
        || (generation_lo | generation_hi) == 0
        || thread::launch_extent_1d() != 64
    {
        fe2o3_device::trap();
    }
    if thread::index_1d().get() == 0 {
        let valid = mlp_state_valid_v1!(state, load_atomic_word_v1);
        publish_guard_v1!(
            guard,
            generation_lo,
            generation_hi,
            valid,
            store_atomic_word_v1
        );
    }
}

#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [64, 1, 1]))]
#[allow(clippy::too_many_arguments)]
pub fn ferric_qwen3_tp2_guarded_projection_residual_bf16_v1(
    p0: &[f32],
    p1: &[f32],
    residual: &[u16],
    mut output: WriteOnlyDisjointSlice<u16>,
    guard0: &[AtomicU32],
    guard1: &[AtomicU32],
    generation_lo: u32,
    generation_hi: u32,
) {
    let elements = 4096_usize;
    if p0.len() != elements
        || p1.len() != elements
        || residual.len() != elements
        || output.len() != elements
        || guard0.len() != 4
        || guard1.len() != 4
        || (generation_lo | generation_hi) == 0
        || thread::launch_extent_1d() != elements
    {
        fe2o3_device::trap();
    }
    let invocation = thread::index_1d();
    let index = invocation.get();
    if index >= elements {
        fe2o3_device::trap();
    }
    with_current_guards_v1!(
        guard0, guard1, generation_lo, generation_hi, load_atomic_word_v1;
        {
            let value = projection_residual_v1!(
                memory::volatile_load(p0, index),
                memory::volatile_load(p1, index),
                memory::volatile_load(residual, index)
            );
            if !output.write(invocation, value) {
                fe2o3_device::trap();
            }
        }
    );
}
