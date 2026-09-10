use fe2o3_device::{
    Bf16, GridExclusive, Index1D, RowStriped2D, WriteOnlyDisjointSlice, kernel, memory, thread,
};

#[cfg(test)]
macro_rules! tp_rope_pair_v1 {
    ($first:expr, $second:expr, $cos:expr, $sin:expr) => {{
        let first = Bf16::from_bits($first).to_f32();
        let second = Bf16::from_bits($second).to_f32();
        let cos = $cos;
        let sin = $sin;
        let rotated_first = first * cos - second * sin;
        let rotated_second = second * cos + first * sin;
        let first_out = Bf16::from_f32(rotated_first);
        let second_out = Bf16::from_f32(rotated_second);
        if !first.is_finite()
            || !second.is_finite()
            || !cos.is_finite()
            || !sin.is_finite()
            || !rotated_first.is_finite()
            || !rotated_second.is_finite()
            || !first_out.is_finite()
            || !second_out.is_finite()
        {
            fe2o3_device::trap();
        }
        (first_out.to_bits(), second_out.to_bits())
    }};
}

/// The trig slices contain exactly the 64 pairs for the supplied position.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [1, 1, 1]), control_flow(loop_bounds(32, 8)))]
#[allow(clippy::too_many_arguments)]
pub fn ferric_qwen3_tp_rope_v1(
    query: &[u16],
    key: &[u16],
    cos: &[f32],
    sin: &[f32],
    mut rotated_query: WriteOnlyDisjointSlice<u16, RowStriped2D<Index1D, 64, 64>>,
    mut rotated_key: WriteOnlyDisjointSlice<u16, RowStriped2D<Index1D, 64, 64>>,
    position: u32,
    model_role: u32,
    world_size: u32,
) {
    if !((model_role == 1 || model_role == 2)
        && (world_size == 1 || world_size == 2 || world_size == 8))
        || position >= 8_192
    {
        fe2o3_device::trap();
    }
    let query_heads = if model_role == 1 && world_size == 1 {
        32
    } else if model_role == 1 && world_size == 2 {
        16
    } else if model_role == 1 && world_size == 8 {
        4
    } else if model_role == 2 && world_size == 1 {
        16
    } else if model_role == 2 && world_size == 2 {
        8
    } else {
        2
    };
    let kv_heads = if world_size == 1 {
        8
    } else if world_size == 2 {
        4
    } else {
        1
    };
    if query_heads == 0 || query_heads > 32 || kv_heads == 0 || kv_heads > 8 {
        fe2o3_device::trap();
    }
    let query_heads = query_heads as usize;
    let kv_heads = kv_heads as usize;
    if query_heads < 33 {
    } else {
        fe2o3_device::trap();
    }
    if kv_heads < 9 {
    } else {
        fe2o3_device::trap();
    }
    let query_columns = query_heads * 128;
    let key_columns = kv_heads * 128;
    if query.len() != query_columns
        || key.len() != key_columns
        || cos.len() != 64
        || sin.len() != 64
        || rotated_query.len() != query_columns
        || rotated_key.len() != key_columns
        || thread::launch_extent_1d() != 64
    {
        fe2o3_device::trap();
    }
    let lane = thread::index_1d().get();
    if lane >= 64 {
        fe2o3_device::trap();
    }
    let cosine = memory::volatile_load(cos, lane);
    let sine = memory::volatile_load(sin, lane);
    let Some(stripe) = thread::index_1d().checked_row_striped_2d::<64, 64>() else {
        fe2o3_device::trap();
    };
    let mut head = 0_usize;
    while head < 32 {
        if head < query_heads {
            let index = head * 128 + lane;
            let first = memory::volatile_load(query, index);
            let second = memory::volatile_load(query, index + 64);
            // BEGIN tp_rope_pair_v1
            let first = Bf16::from_bits(first).to_f32();
            let second = Bf16::from_bits(second).to_f32();
            let cos = cosine;
            let sin = sine;
            let rotated_first = first * cos - second * sin;
            let rotated_second = second * cos + first * sin;
            let first_out = Bf16::from_f32(rotated_first);
            let second_out = Bf16::from_f32(rotated_second);
            if !first.is_finite()
                || !second.is_finite()
                || !cos.is_finite()
                || !sin.is_finite()
                || !rotated_first.is_finite()
                || !rotated_second.is_finite()
                || !first_out.is_finite()
                || !second_out.is_finite()
            {
                fe2o3_device::trap();
            }
            let first_out = first_out.to_bits();
            let second_out = second_out.to_bits();
            // END tp_rope_pair_v1
            if !rotated_query.write_row_striped_2d(
                &stripe,
                head * 2,
                1,
                query_columns,
                query_columns,
                first_out,
            ) || !rotated_query.write_row_striped_2d(
                &stripe,
                head * 2 + 1,
                1,
                query_columns,
                query_columns,
                second_out,
            ) {
                fe2o3_device::trap();
            }
        }
        head += 1;
    }
    let mut head = 0_usize;
    while head < 8 {
        if head < kv_heads {
            let index = head * 128 + lane;
            let first = memory::volatile_load(key, index);
            let second = memory::volatile_load(key, index + 64);
            // BEGIN tp_rope_pair_v1
            let first = Bf16::from_bits(first).to_f32();
            let second = Bf16::from_bits(second).to_f32();
            let cos = cosine;
            let sin = sine;
            let rotated_first = first * cos - second * sin;
            let rotated_second = second * cos + first * sin;
            let first_out = Bf16::from_f32(rotated_first);
            let second_out = Bf16::from_f32(rotated_second);
            if !first.is_finite()
                || !second.is_finite()
                || !cos.is_finite()
                || !sin.is_finite()
                || !rotated_first.is_finite()
                || !rotated_second.is_finite()
                || !first_out.is_finite()
                || !second_out.is_finite()
            {
                fe2o3_device::trap();
            }
            let first_out = first_out.to_bits();
            let second_out = second_out.to_bits();
            // END tp_rope_pair_v1
            if !rotated_key.write_row_striped_2d(
                &stripe,
                head * 2,
                1,
                key_columns,
                key_columns,
                first_out,
            ) || !rotated_key.write_row_striped_2d(
                &stripe,
                head * 2 + 1,
                1,
                key_columns,
                key_columns,
                second_out,
            ) {
                fe2o3_device::trap();
            }
        }
        head += 1;
    }
}

/// Overwrites exactly one initialized cache row; trailing rows are untouched.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [1, 1, 1]), control_flow(loop_bounds(1024)))]
#[allow(clippy::too_many_arguments)]
pub fn ferric_qwen3_tp_kv_append_v1(
    key: &[u16],
    value: &[u16],
    mut key_cache: WriteOnlyDisjointSlice<u16, GridExclusive>,
    mut value_cache: WriteOnlyDisjointSlice<u16, GridExclusive>,
    position: u32,
    capacity: u32,
    model_role: u32,
    world_size: u32,
) {
    if !((model_role == 1 || model_role == 2)
        && (world_size == 1 || world_size == 2 || world_size == 8))
        || !(capacity > 0 && capacity <= 8_192)
        || position >= capacity
    {
        fe2o3_device::trap();
    }
    let kv_heads = if world_size == 1 {
        8
    } else if world_size == 2 {
        4
    } else {
        1
    };
    if kv_heads == 0 || kv_heads > 8 {
        fe2o3_device::trap();
    }
    let kv_heads = kv_heads as usize;
    if kv_heads < 9 {
    } else {
        fe2o3_device::trap();
    }
    let columns = kv_heads * 128;
    let capacity = capacity as usize;
    let position = position as usize;
    if capacity < 8_193 {
    } else {
        fe2o3_device::trap();
    }
    if position < capacity {
    } else {
        fe2o3_device::trap();
    }
    if key.len() != columns
        || value.len() != columns
        || key_cache.len() != capacity * columns
        || value_cache.len() != capacity * columns
        || thread::launch_extent_1d() != 64
    {
        fe2o3_device::trap();
    }
    let Some(leader) = thread::grid_leader() else {
        return;
    };
    let base = position * columns;
    let mut component = 0_usize;
    while component < 1024 {
        if component < columns {
            let key_value = memory::volatile_load(key, component);
            let value_value = memory::volatile_load(value, component);
            if !key_cache.write_exclusive(&leader, base + component, key_value)
                || !value_cache.write_exclusive(&leader, base + component, value_value)
            {
                fe2o3_device::trap();
            }
        }
        component += 1;
    }
}

#[cfg(test)]
mod tests;
