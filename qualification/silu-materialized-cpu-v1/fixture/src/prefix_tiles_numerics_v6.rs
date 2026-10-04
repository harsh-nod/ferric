// Additive P227 handler slicing only. Existing dot, FP32 O and online
// recurrence macros remain unchanged; no storage, launch or numerical authority.
#[allow(unused_macros)]
macro_rules! qwen_claimed_projection_tile_v6 {
    ($task:ident, $subgroup:ident) => {{
        let lane = $task.lane();
        let mut column = 0;
        while column < 64 {
            // Invalid lanes continue all64 collectives; rejection is gathered
            // uniformly afterward, so no lane exits around a subgroup operation.
            let (sum, narrowed, finite) = qwen_wave_dot_v1!(
                lane,
                inner,
                fe2o3_device::Bf16::from_bits(match $task.input(inner) {
                    Some(value) => value,
                    None => 0x7fc0,
                })
                .to_f32(),
                fe2o3_device::Bf16::from_bits(match $task.weight(column, inner) {
                    Some(value) => value,
                    None => 0x7fc0,
                })
                .to_f32(),
                $subgroup
            );
            if !finite || !sum.is_finite() || !narrowed.is_finite() {
                $task.reject();
            } else if lane == 0 && !$task.write_column(column, narrowed.to_bits()) {
                $task.reject();
            }
            column += 1;
        }
    }};
}

#[allow(unused_macros)]
macro_rules! qwen_claimed_output_projection_tile_v6 {
    ($task:ident, $subgroup:ident) => {{
        let lane = $task.lane();
        let mut column = 0;
        while column < 64 {
            // Rejected lanes still execute every reduction. The outer worker
            // gathers failure before publishing completion of this task.
            let (sum, finite) = qwen_wave_output_partial_f32_v1!(
                lane,
                inner,
                fe2o3_device::Bf16::from_bits(match $task.input(inner) {
                    Some(value) => value,
                    None => 0x7fc0,
                })
                .to_f32(),
                fe2o3_device::Bf16::from_bits(match $task.weight(column, inner) {
                    Some(value) => value,
                    None => 0x7fc0,
                })
                .to_f32(),
                $subgroup
            );
            if !finite || !sum.is_finite() {
                $task.reject();
            } else if lane == 0 && !$task.write_output(column, sum) {
                $task.reject();
            }
            column += 1;
        }
    }};
}

// Shared claimed-task handler; host tests replace only checked storage, subgroup
// operations, and exp. The online recurrence remains the queued V3 source.
macro_rules! qwen_claimed_attention_head_v6 {
    ($task:ident, $subgroup:ident, $math:ident) => {{
        let position = $task.position();
        let head = $task.head();
        let (first, second, finite) = qwen_attention_online_pair_v1!(
            qwen_attention_emit_pair_v1,
            (),
            2304_usize,
            position,
            token,
            {
                // Every lane enters this bit-preserving broadcast, including
                // invalid pages. Metadata is not assumed uniform merely
                // because it is immutable during the dispatch.
                let raw_page = match $task.page(token) {
                    Some(page) => page,
                    None => u32::MAX,
                };
                let physical_page = $subgroup
                    .broadcast_f32::<64>(f32::from_bits(raw_page), 0)
                    .to_bits();
                let query_0 = fe2o3_device::Bf16::from_bits(match $task.query(head, 0) {
                    Some(bits) => bits,
                    None => 0x7fc0,
                });
                let query_1 = fe2o3_device::Bf16::from_bits(match $task.query(head, 1) {
                    Some(bits) => bits,
                    None => 0x7fc0,
                });
                let key_0 =
                    fe2o3_device::Bf16::from_bits(match $task.key(head, token, physical_page, 0) {
                        Some(bits) => bits,
                        None => 0x7fc0,
                    });
                let key_1 =
                    fe2o3_device::Bf16::from_bits(match $task.key(head, token, physical_page, 1) {
                        Some(bits) => bits,
                        None => 0x7fc0,
                    });
                let product_0 = query_0.to_f32() * key_0.to_f32();
                let product_1 = query_1.to_f32() * key_1.to_f32();
                let partial = product_0 + product_1;
                let dot = $subgroup.reduce_sum_f32::<64>(partial);
                let score = dot * f32::from_bits(0x3db5_04f3);
                let value_0 = fe2o3_device::Bf16::from_bits(
                    match $task.value(head, token, physical_page, 0) {
                        Some(bits) => bits,
                        None => 0x7fc0,
                    },
                )
                .to_f32();
                let value_1 = fe2o3_device::Bf16::from_bits(
                    match $task.value(head, token, physical_page, 1) {
                        Some(bits) => bits,
                        None => 0x7fc0,
                    },
                )
                .to_f32();
                let product_finite = (physical_page < 144)
                    & query_0.is_finite()
                    & query_1.is_finite()
                    & key_0.is_finite()
                    & key_1.is_finite()
                    & product_0.is_finite()
                    & product_1.is_finite()
                    & partial.is_finite()
                    & dot.is_finite();
                (score, value_0, value_1, product_finite)
            },
            $math
        );
        // Rejection never skips a later causal-token collective in this head. The worker
        // gathers rejection and incomplete coverage in its round finish.
        if !finite {
            $task.reject();
        } else if !$task.write_head(head, first, second) {
            $task.reject();
        }
    }};
}
