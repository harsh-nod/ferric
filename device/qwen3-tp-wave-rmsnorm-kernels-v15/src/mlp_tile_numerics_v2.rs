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
