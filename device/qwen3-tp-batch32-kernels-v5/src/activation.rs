use fe2o3_device::{Bf16, Math, WriteOnlyDisjointSlice, kernel, memory, thread};

macro_rules! batch_swiglu_v5 {
    ($gate:expr, $up:expr, $math:expr) => {{
        let gate = Bf16::from_bits($gate).to_f32();
        let up = Bf16::from_bits($up).to_f32();
        if !gate.is_finite() || !up.is_finite() {
            fe2o3_device::trap();
        }
        let nonnegative = gate >= 0.0;
        let argument = if nonnegative { -gate } else { gate };
        let exponent = $math.exp_f32(argument);
        let numerator = if nonnegative { 1.0 } else { exponent };
        let sigmoid = numerator / (1.0 + exponent);
        let silu = gate * sigmoid;
        let product = silu * up;
        let result = Bf16::from_f32(product);
        if !exponent.is_finite()
            || exponent < 0.0
            || !sigmoid.is_finite()
            || !silu.is_finite()
            || !product.is_finite()
            || !result.is_finite()
        {
            fe2o3_device::trap();
        }
        result.to_bits()
    }};
}

#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [6144, 1, 1]))]
pub fn ferric_qwen3_tp_batch32_swiglu_bf16_f32_v5(
    gate: &[u16],
    up: &[u16],
    mut output: WriteOnlyDisjointSlice<u16>,
    rows: u32,
    world_size: u32,
) {
    if rows == 0 || rows > 32 || !(world_size == 1 || world_size == 2 || world_size == 8) {
        fe2o3_device::trap();
    }
    let columns = if world_size == 1 {
        12288_u32
    } else if world_size == 2 {
        6144
    } else {
        1536
    };
    let rows = rows as usize;
    let columns = columns as usize;
    if rows < 33 {
    } else {
        fe2o3_device::trap();
    }
    if columns < 12289 {
    } else {
        fe2o3_device::trap();
    }
    if gate.len() < rows * columns
        || gate.len() > 32 * columns
        || up.len() < rows * columns
        || up.len() > 32 * columns
        || output.len() < rows * columns
        || output.len() > 32 * columns
        || thread::launch_extent_1d() != rows * columns
    {
        fe2o3_device::trap();
    }
    let invocation = thread::index_1d();
    let index = invocation.get();
    if index < rows * columns {
    } else {
        fe2o3_device::trap();
    }
    let math = Math::current();
    let value = batch_swiglu_v5!(
        memory::volatile_load(gate, index),
        memory::volatile_load(up, index),
        math
    );
    if !output.write(invocation, value) {
        fe2o3_device::trap();
    }
}
