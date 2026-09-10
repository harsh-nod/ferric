use fe2o3_device::{
    Bf16, Index1D, Math, RowStriped2D, WriteOnlyDisjointSlice, kernel, memory, thread,
};

const ATTENTION_SCALE: f32 = f32::from_bits(0x3db5_04f3);

// A lane retains two outputs while replaying the same dot/online-softmax order.
#[cfg(test)]
macro_rules! tp_attention_pair_v1 {
    ($q:expr, $keys:expr, $values:expr, $count:expr, $columns:expr,
     $query_head:expr, $kv_head:expr, $lane:expr, $math:expr) => {{
        let mut maximum = 0.0_f32;
        let mut denominator = 0.0_f32;
        let mut numerator_0 = 0.0_f32;
        let mut numerator_1 = 0.0_f32;
        let mut token = 0_usize;
        while token < $count {
            let cache_base = token * $columns + $kv_head * 128;
            let query_base = $query_head * 128;
            let mut dot = 0.0_f32;
            let mut dimension = 0_usize;
            while dimension < 128 {
                let query =
                    Bf16::from_bits(memory::volatile_load($q, query_base + dimension)).to_f32();
                let key =
                    Bf16::from_bits(memory::volatile_load($keys, cache_base + dimension)).to_f32();
                let product = query * key;
                dot += product;
                if !product.is_finite() || !dot.is_finite() {
                    fe2o3_device::trap();
                }
                dimension += 1;
            }
            let score = dot * ATTENTION_SCALE;
            let value_0 =
                Bf16::from_bits(memory::volatile_load($values, cache_base + $lane)).to_f32();
            let value_1 =
                Bf16::from_bits(memory::volatile_load($values, cache_base + $lane + 64)).to_f32();
            if !score.is_finite() || !value_0.is_finite() || !value_1.is_finite() {
                fe2o3_device::trap();
            }
            if token == 0 {
                maximum = score;
                denominator = 1.0;
                numerator_0 = value_0;
                numerator_1 = value_1;
            } else {
                let next_maximum = if score > maximum { score } else { maximum };
                let previous_weight = $math.exp_f32(maximum - next_maximum);
                let current_weight = $math.exp_f32(score - next_maximum);
                denominator = denominator * previous_weight + current_weight;
                numerator_0 = numerator_0 * previous_weight + value_0 * current_weight;
                numerator_1 = numerator_1 * previous_weight + value_1 * current_weight;
                if !previous_weight.is_finite()
                    || previous_weight < 0.0
                    || !current_weight.is_finite()
                    || current_weight < 0.0
                    || !denominator.is_finite()
                    || denominator <= 0.0
                    || !numerator_0.is_finite()
                    || !numerator_1.is_finite()
                {
                    fe2o3_device::trap();
                }
                maximum = next_maximum;
            }
            token += 1;
        }
        let output_0 = numerator_0 / denominator;
        let output_1 = numerator_1 / denominator;
        let narrowed_0 = Bf16::from_f32(output_0);
        let narrowed_1 = Bf16::from_f32(output_1);
        if !output_0.is_finite()
            || !output_1.is_finite()
            || !narrowed_0.is_finite()
            || !narrowed_1.is_finite()
        {
            fe2o3_device::trap();
        }
        (narrowed_0.to_bits(), narrowed_1.to_bits())
    }};
}

/// Single-token causal attention over the initialized contiguous local prefix.
#[kernel(typed, launch(required = [64, 1, 1], max = [64, 1, 1], max_grid = [32, 1, 1]), control_flow(loop_bounds(8192, 128)))]
#[allow(clippy::too_many_arguments)]
pub fn ferric_qwen3_tp_gqa_decode_bf16_f32_v1(
    query: &[u16],
    key_cache: &[u16],
    value_cache: &[u16],
    mut output: WriteOnlyDisjointSlice<u16, RowStriped2D<Index1D, 64, 2>>,
    count: u32,
    capacity: u32,
    model_role: u32,
    world_size: u32,
) {
    if !((model_role == 1 || model_role == 2)
        && (world_size == 1 || world_size == 2 || world_size == 8))
        || !(capacity > 0 && capacity <= 8_192)
        || count == 0
        || count > capacity
    {
        fe2o3_device::trap();
    }
    let query_heads = if model_role == 1 && world_size == 1 {
        32_u32
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
        8_u32
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
    let capacity = capacity as usize;
    let count = count as usize;
    if query_heads < 33 {
    } else {
        fe2o3_device::trap();
    }
    if kv_heads < 9 {
    } else {
        fe2o3_device::trap();
    }
    if capacity < 8_193 {
    } else {
        fe2o3_device::trap();
    }
    if count < 8_193 {
    } else {
        fe2o3_device::trap();
    }
    let columns = kv_heads * 128;
    if query.len() != query_heads * 128
        || output.len() != query_heads * 128
        || key_cache.len() != capacity * columns
        || value_cache.len() != capacity * columns
        || thread::launch_extent_1d() != query_heads * 64
    {
        fe2o3_device::trap();
    }
    let raw = thread::index_1d().get();
    let query_head = raw / 64;
    let lane = raw % 64;
    if query_head < query_heads {
    } else {
        fe2o3_device::trap();
    }
    let kv_head = if model_role == 1 {
        query_head / 4
    } else {
        query_head / 2
    };
    if kv_head < kv_heads {
    } else {
        fe2o3_device::trap();
    }
    let math = Math::current();
    // BEGIN tp_attention_pair_v1
    let mut maximum = 0.0_f32;
    let mut denominator = 0.0_f32;
    let mut numerator_0 = 0.0_f32;
    let mut numerator_1 = 0.0_f32;
    let mut token = 0_usize;
    while token < count {
        let cache_base = token * columns + kv_head * 128;
        let query_base = query_head * 128;
        let mut dot = 0.0_f32;
        let mut dimension = 0_usize;
        while dimension < 128 {
            let query =
                Bf16::from_bits(memory::volatile_load(query, query_base + dimension)).to_f32();
            let key =
                Bf16::from_bits(memory::volatile_load(key_cache, cache_base + dimension)).to_f32();
            let product = query * key;
            dot += product;
            if !product.is_finite() || !dot.is_finite() {
                fe2o3_device::trap();
            }
            dimension += 1;
        }
        let score = dot * ATTENTION_SCALE;
        let value_0 =
            Bf16::from_bits(memory::volatile_load(value_cache, cache_base + lane)).to_f32();
        let value_1 =
            Bf16::from_bits(memory::volatile_load(value_cache, cache_base + lane + 64)).to_f32();
        if !score.is_finite() || !value_0.is_finite() || !value_1.is_finite() {
            fe2o3_device::trap();
        }
        if token == 0 {
            maximum = score;
            denominator = 1.0;
            numerator_0 = value_0;
            numerator_1 = value_1;
        } else {
            let next_maximum = if score > maximum { score } else { maximum };
            let previous_weight = math.exp_f32(maximum - next_maximum);
            let current_weight = math.exp_f32(score - next_maximum);
            denominator = denominator * previous_weight + current_weight;
            numerator_0 = numerator_0 * previous_weight + value_0 * current_weight;
            numerator_1 = numerator_1 * previous_weight + value_1 * current_weight;
            if !previous_weight.is_finite()
                || previous_weight < 0.0
                || !current_weight.is_finite()
                || current_weight < 0.0
                || !denominator.is_finite()
                || denominator <= 0.0
                || !numerator_0.is_finite()
                || !numerator_1.is_finite()
            {
                fe2o3_device::trap();
            }
            maximum = next_maximum;
        }
        token += 1;
    }
    let output_0 = numerator_0 / denominator;
    let output_1 = numerator_1 / denominator;
    let narrowed_0 = Bf16::from_f32(output_0);
    let narrowed_1 = Bf16::from_f32(output_1);
    if !output_0.is_finite()
        || !output_1.is_finite()
        || !narrowed_0.is_finite()
        || !narrowed_1.is_finite()
    {
        fe2o3_device::trap();
    }
    let first = narrowed_0.to_bits();
    let second = narrowed_1.to_bits();
    // END tp_attention_pair_v1
    let Some(stripe) = thread::index_1d().checked_row_striped_2d::<64, 2>() else {
        fe2o3_device::trap();
    };
    if !output.write_row_striped_2d(&stripe, 0, query_heads, 128, 128, first)
        || !output.write_row_striped_2d(&stripe, 1, query_heads, 128, 128, second)
    {
        fe2o3_device::trap();
    }
}

#[cfg(test)]
mod tests;
