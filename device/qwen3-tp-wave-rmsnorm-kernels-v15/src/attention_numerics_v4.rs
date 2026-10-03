// Shared claimed-task handler; host tests replace only checked storage, subgroup
// operations, and exp. The online recurrence remains the queued V3 source.
macro_rules! qwen_claimed_attention_v4 {
    ($task:ident, $subgroup:ident, $math:ident) => {{
        let position = $task.position();
        let mut head = 0_usize;
        while head < 16 {
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
                    let key_0 = fe2o3_device::Bf16::from_bits(
                        match $task.key(head, token, physical_page, 0) {
                            Some(bits) => bits,
                            None => 0x7fc0,
                        },
                    );
                    let key_1 = fe2o3_device::Bf16::from_bits(
                        match $task.key(head, token, physical_page, 1) {
                            Some(bits) => bits,
                            None => 0x7fc0,
                        },
                    );
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
            // Rejection never skips a later head/token collective. The worker
            // gathers rejection and incomplete coverage in its round finish.
            if !finite {
                $task.reject();
            } else if !$task.write_head(head, first, second) {
                $task.reject();
            }
            head += 1;
        }
    }};
}
