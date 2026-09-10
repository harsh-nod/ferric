use fe2o3_device::{Bf16, Blocked, Index1D, Math, WriteOnlyDisjointSlice, kernel, memory, thread};

macro_rules! tp_swiglu_element_v1 {
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

#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [192, 1, 1]))]
pub fn ferric_qwen3_tp_swiglu_bf16_f32_v1(
    gate: &[u16],
    up: &[u16],
    mut output: WriteOnlyDisjointSlice<u16, Blocked<Index1D, 1, 1>>,
    model_role: u32,
    world_size: u32,
) {
    if !((model_role == 1 || model_role == 2)
        && (world_size == 1 || world_size == 2 || world_size == 8))
    {
        fe2o3_device::trap();
    }
    let elements = if model_role == 1 {
        12_288 / world_size
    } else {
        3_072 / world_size
    };
    if elements == 0 || elements > 12_288 {
        fe2o3_device::trap();
    }
    let elements = elements as usize;
    if gate.len() != elements
        || up.len() != elements
        || output.len() != elements
        || thread::launch_extent_1d() != elements
    {
        fe2o3_device::trap();
    }
    let Some(block) = thread::index_1d().checked_block::<1, 1>() else {
        fe2o3_device::trap();
    };
    let Some(index) = block.component_index(0) else {
        fe2o3_device::trap();
    };
    if index >= elements {
        fe2o3_device::trap();
    }
    let gate_value = memory::volatile_load(gate, index);
    let up_value = memory::volatile_load(up, index);
    let math = Math::current();
    let value = tp_swiglu_element_v1!(gate_value, up_value, math);
    if !output.write_block(&block, 0, value) {
        fe2o3_device::trap();
    }
}

#[cfg(test)]
mod tests;
