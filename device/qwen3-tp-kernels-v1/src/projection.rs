use fe2o3_device::{Bf16, WriteOnlyDisjointSlice, kernel, memory, thread};

// Both output formats and the host numerical checks execute this accumulation.
#[cfg(test)]
macro_rules! tp_gemv_sum_v1 {
    ($a:expr, $weights:expr, $column:expr, $k:expr) => {{
        let mut sum = 0.0_f32;
        let mut inner = 0_usize;
        while inner < $k {
            let left = Bf16::from_bits(memory::volatile_load($a, inner)).to_f32();
            let right =
                Bf16::from_bits(memory::volatile_load($weights, $column * $k + inner)).to_f32();
            let product = left * right;
            sum += product;
            if !product.is_finite() || !sum.is_finite() {
                fe2o3_device::trap();
            }
            inner += 1;
        }
        sum
    }};
}

/// One compact column-parallel projection. Projection tags: Q1/K2/V3/Gate4/Up5.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [192, 1, 1]), control_flow(loop_bounds(12288)))]
#[allow(clippy::too_many_arguments)]
pub fn ferric_qwen3_tp_gemv_bf16_f32_bf16_v1(
    a: &[u16],
    weights: &[u16],
    mut output: WriteOnlyDisjointSlice<u16>,
    n: u32,
    k: u32,
    model_role: u32,
    world_size: u32,
    projection: u32,
) {
    if !((model_role == 1 || model_role == 2)
        && (world_size == 1 || world_size == 2 || world_size == 8))
    {
        fe2o3_device::trap();
    }
    let hidden = if model_role == 1 { 4_096 } else { 1_024 };
    let queries = if model_role == 1 && world_size == 1 {
        4_096
    } else if model_role == 1 && world_size == 2 {
        2_048
    } else if model_role == 1 && world_size == 8 {
        512
    } else if model_role == 2 && world_size == 1 {
        2_048
    } else if model_role == 2 && world_size == 2 {
        1_024
    } else {
        256
    };
    let keys = if world_size == 1 {
        1_024
    } else if world_size == 2 {
        512
    } else {
        128
    };
    let intermediate = if model_role == 1 && world_size == 1 {
        12_288
    } else if model_role == 1 && world_size == 2 {
        6_144
    } else if model_role == 1 && world_size == 8 {
        1_536
    } else if model_role == 2 && world_size == 1 {
        3_072
    } else if model_role == 2 && world_size == 2 {
        1_536
    } else {
        384
    };
    if !(k == hidden
        && ((projection == 1 && n == queries)
            || ((projection == 2 || projection == 3) && n == keys)
            || ((projection == 4 || projection == 5) && n == intermediate)))
        || n == 0
        || n > 12_288
        || k == 0
        || k > 12_288
        || n & 63 != 0
    {
        fe2o3_device::trap();
    }
    let n = n as usize;
    let k = k as usize;
    if n < 12_289 {
    } else {
        fe2o3_device::trap();
    }
    if k < 12_289 {
    } else {
        fe2o3_device::trap();
    }
    if a.len() != k
        || weights.len() != n * k
        || output.len() != n
        || thread::launch_extent_1d() != n
    {
        fe2o3_device::trap();
    }
    let invocation = thread::index_1d();
    let column = invocation.get();
    if column < n {
    } else {
        fe2o3_device::trap();
    }
    // BEGIN tp_gemv_sum_v1
    let mut sum = 0.0_f32;
    let mut inner = 0_usize;
    while inner < k {
        let left = Bf16::from_bits(memory::volatile_load(a, inner)).to_f32();
        let right = Bf16::from_bits(memory::volatile_load(weights, column * k + inner)).to_f32();
        let product = left * right;
        sum += product;
        if !product.is_finite() || !sum.is_finite() {
            fe2o3_device::trap();
        }
        inner += 1;
    }
    // END tp_gemv_sum_v1
    let narrowed = Bf16::from_f32(sum);
    if !narrowed.is_finite() || !output.write(invocation, narrowed.to_bits()) {
        fe2o3_device::trap();
    }
}

/// FP32 row-parallel partial, without residual or BF16 rounding. O1/Down2.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [64, 1, 1]), control_flow(loop_bounds(12288)))]
#[allow(clippy::too_many_arguments)]
pub fn ferric_qwen3_tp_gemv_partial_bf16_f32_v1(
    a: &[u16],
    weights: &[u16],
    mut output: WriteOnlyDisjointSlice<f32>,
    n: u32,
    k: u32,
    model_role: u32,
    world_size: u32,
    projection: u32,
) {
    if !((model_role == 1 || model_role == 2)
        && (world_size == 1 || world_size == 2 || world_size == 8))
    {
        fe2o3_device::trap();
    }
    let hidden = if model_role == 1 { 4_096 } else { 1_024 };
    let queries = if model_role == 1 && world_size == 1 {
        4_096
    } else if model_role == 1 && world_size == 2 {
        2_048
    } else if model_role == 1 && world_size == 8 {
        512
    } else if model_role == 2 && world_size == 1 {
        2_048
    } else if model_role == 2 && world_size == 2 {
        1_024
    } else {
        256
    };
    let intermediate = if model_role == 1 && world_size == 1 {
        12_288
    } else if model_role == 1 && world_size == 2 {
        6_144
    } else if model_role == 1 && world_size == 8 {
        1_536
    } else if model_role == 2 && world_size == 1 {
        3_072
    } else if model_role == 2 && world_size == 2 {
        1_536
    } else {
        384
    };
    if !(n == hidden
        && ((projection == 1 && k == queries) || (projection == 2 && k == intermediate)))
        || n == 0
        || n > 4_096
        || k == 0
        || k > 12_288
        || n & 63 != 0
    {
        fe2o3_device::trap();
    }
    let n = n as usize;
    let k = k as usize;
    if n < 4_097 {
    } else {
        fe2o3_device::trap();
    }
    if k < 12_289 {
    } else {
        fe2o3_device::trap();
    }
    if a.len() != k
        || weights.len() != n * k
        || output.len() != n
        || thread::launch_extent_1d() != n
    {
        fe2o3_device::trap();
    }
    let invocation = thread::index_1d();
    let column = invocation.get();
    if column < n {
    } else {
        fe2o3_device::trap();
    }
    // BEGIN tp_gemv_sum_v1
    let mut sum = 0.0_f32;
    let mut inner = 0_usize;
    while inner < k {
        let left = Bf16::from_bits(memory::volatile_load(a, inner)).to_f32();
        let right = Bf16::from_bits(memory::volatile_load(weights, column * k + inner)).to_f32();
        let product = left * right;
        sum += product;
        if !product.is_finite() || !sum.is_finite() {
            fe2o3_device::trap();
        }
        inner += 1;
    }
    // END tp_gemv_sum_v1
    if !output.write(invocation, sum) {
        fe2o3_device::trap();
    }
}

#[cfg(test)]
mod tests;
