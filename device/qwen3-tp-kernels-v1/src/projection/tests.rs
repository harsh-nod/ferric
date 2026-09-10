use super::*;
use crate::contract::geometry;
use std::vec;
use std::vec::Vec;

fn bf(value: f32) -> u16 {
    Bf16::from_f32(value).to_bits()
}

#[test]
fn exact_binary_projections_reconstruct_all_model_rank_partitions() {
    for role in [1, 2] {
        let full = geometry(role, 1).unwrap();
        for k in [full.query_heads * 128, full.intermediate] {
            let k = k as usize;
            let input: Vec<_> = (0..k).map(|i| bf((i % 17) as f32 / 16.0 - 0.5)).collect();
            let weights: Vec<_> = (0..k).map(|i| bf((i % 13) as f32 / 32.0 - 0.25)).collect();
            let full_sum = tp_gemv_sum_v1!(&input, &weights, 0, k);
            for world in [1, 2, 8] {
                let local_k = k / world;
                let mut reduced = 0.0_f32;
                for rank in 0..world {
                    let start = rank * local_k;
                    let end = start + local_k;
                    reduced +=
                        tp_gemv_sum_v1!(&input[start..end], &weights[start..end], 0, local_k);
                }
                assert_eq!(full_sum.to_bits(), reduced.to_bits());
                let residual = bf(0.375);
                assert_eq!(
                    bf(full_sum + Bf16::from_bits(residual).to_f32()),
                    bf(reduced + Bf16::from_bits(residual).to_f32())
                );
            }
        }
        let k = full.hidden as usize;
        let input: Vec<_> = (0..k).map(|i| bf((i % 7) as f32 / 8.0)).collect();
        let weights: Vec<_> = (0..k * 2).map(|i| bf((i % 11) as f32 / 16.0)).collect();
        let expected = tp_gemv_sum_v1!(&input, &weights, 1, k);
        let compact = &weights[k..2 * k];
        assert_eq!(
            expected.to_bits(),
            tp_gemv_sum_v1!(&input, compact, 0, k).to_bits()
        );
    }
}

#[test]
fn fp32_partials_are_not_bf16_rounded_before_reduction() {
    let input = [bf(1.0), bf(1.0)];
    let weights = [bf(1.0), bf(0.00390625)];
    let partial = tp_gemv_sum_v1!(&input, &weights, 0, 2);
    assert_eq!(partial, 1.00390625);
    assert_ne!(
        partial.to_bits(),
        Bf16::from_f32(partial).to_f32().to_bits()
    );
}

#[test]
fn regrouping_is_explicitly_not_a_bitwise_full_dot_product_claim() {
    let mut input = vec![bf(0.0); 1_024];
    input[0] = bf(100_000_000.0);
    input[1] = bf(1.0);
    input[512] = bf(-100_000_000.0);
    input[513] = bf(1.0);
    let weights = vec![bf(1.0); 1_024];
    let full = tp_gemv_sum_v1!(&input, &weights, 0, 1_024);
    let first = tp_gemv_sum_v1!(&input[..512], &weights[..512], 0, 512);
    let second = tp_gemv_sum_v1!(&input[512..], &weights[512..], 0, 512);
    assert_eq!(full, 1.0);
    assert_eq!(first + second, 0.0);
}

#[test]
#[should_panic]
fn nonfinite_projection_input_traps() {
    let _ = tp_gemv_sum_v1!(&[bf(f32::NAN)], &[bf(1.0)], 0, 1);
}
