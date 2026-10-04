// Only the outer row range changes. V1 inner products/reductions are reused
// literally; private task views add the tile's row base to weight/output roots.
#[allow(unused_macros)]
macro_rules! qwen_claimed_mlp_projection_tile_v2 {
    ($task:ident, $subgroup:ident) => {{
        let lane = $task.lane();
        let mut row = 0;
        while row < 64 {
            // Rejecting a lane never skips a later row's subgroup operations.
            let (sum, narrowed, finite) = qwen_wave_dot_v1!(
                lane,
                inner,
                fe2o3_device::Bf16::from_bits(match $task.input(inner) {
                    Some(value) => value,
                    None => 0x7fc0,
                })
                .to_f32(),
                fe2o3_device::Bf16::from_bits(match $task.weight(row, inner) {
                    Some(value) => value,
                    None => 0x7fc0,
                })
                .to_f32(),
                $subgroup
            );
            if !finite || !sum.is_finite() || !narrowed.is_finite() {
                $task.reject();
            } else if lane == 0 && !$task.write_column(row, narrowed.to_bits()) {
                $task.reject();
            }
            row += 1;
        }
    }};
}

#[allow(unused_macros)]
macro_rules! qwen_claimed_mlp_down_tile_v2 {
    ($task:ident, $subgroup:ident) => {{
        let lane = $task.lane();
        let mut row = 0;
        while row < 64 {
            let (sum, finite) = qwen_wave_mlp_down_partial_v1!(
                lane,
                inner,
                fe2o3_device::Bf16::from_bits(match $task.input(inner) {
                    Some(value) => value,
                    None => 0x7fc0,
                })
                .to_f32(),
                fe2o3_device::Bf16::from_bits(match $task.weight(row, inner) {
                    Some(value) => value,
                    None => 0x7fc0,
                })
                .to_f32(),
                $subgroup
            );
            if !finite || !sum.is_finite() {
                $task.reject();
            } else if lane == 0 && !$task.write_output(row, sum) {
                $task.reject();
            }
            row += 1;
        }
    }};
}

// Two independent row accumulators share an acquired activation load. Each
// row keeps its original 96-step multiply/add order and Wave64 reduction.
#[allow(unused_macros)]
macro_rules! qwen_claimed_mlp_down_tile2_v1 {
    ($task:ident, $subgroup:ident) => {{
        let lane = $task.lane();
        let mut pair = 0;
        while pair < 32 {
            let row0 = pair * 2;
            let row1 = row0 + 1;
            let mut partial0 = 0.0_f32;
            let mut partial1 = 0.0_f32;
            let mut finite0 = true;
            let mut finite1 = true;
            let mut step = 0_usize;
            while step < 96 {
                let inner = step * 64 + lane;
                let input = fe2o3_device::Bf16::from_bits(match $task.input(inner) {
                    Some(value) => value,
                    None => 0x7fc0,
                })
                .to_f32();
                let weight0 = fe2o3_device::Bf16::from_bits(match $task.weight(row0, inner) {
                    Some(value) => value,
                    None => 0x7fc0,
                })
                .to_f32();
                let product0 = input * weight0;
                partial0 += product0;
                finite0 &= product0.is_finite() & partial0.is_finite();
                let weight1 = fe2o3_device::Bf16::from_bits(match $task.weight(row1, inner) {
                    Some(value) => value,
                    None => 0x7fc0,
                })
                .to_f32();
                let product1 = input * weight1;
                partial1 += product1;
                finite1 &= product1.is_finite() & partial1.is_finite();
                step += 1;
            }
            let sum0 = $subgroup.reduce_sum_f32::<64>(partial0);
            let mut rejected0 = !finite0 || !sum0.is_finite();
            if rejected0 {
                $task.reject();
            } else if lane == 0 && !$task.write_output(row0, sum0) {
                $task.reject();
                rejected0 = true;
            }
            // A rejected row makes subsequent input() reads return None in
            // the original provider. Invalidate the speculative second row
            // too, but never skip its collective or reject the first early.
            if rejected0 {
                partial1 = f32::from_bits(0x7fc0_0000);
                finite1 = false;
            }
            let sum1 = $subgroup.reduce_sum_f32::<64>(partial1);
            if !finite1 || !sum1.is_finite() {
                $task.reject();
            } else if lane == 0 && !$task.write_output(row1, sum1) {
                $task.reject();
            }
            pair += 1;
        }
    }};
}
