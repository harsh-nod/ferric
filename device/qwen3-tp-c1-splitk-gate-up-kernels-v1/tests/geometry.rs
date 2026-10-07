use fe2o3_device::Bf16;
use ferric_qwen3_tp_c1_splitk_gate_up_kernels_device_v1::{PARTIAL_BYTES, PARTIAL_ELEMENTS};

#[test]
fn compact_row_stripes_cover_every_scratch_element_exactly_once() {
    let mut writes = vec![0_u8; PARTIAL_ELEMENTS];
    for raw in 0..3072 * 64 {
        let group = raw / 64;
        let lane = raw % 64;
        if lane < 16 {
            let partition = group / 768;
            let tile_column = group % 768;
            let physical = group * 16 + lane;
            let logical = partition * 12288 + tile_column * 16 + lane;
            assert_eq!(physical, logical);
            writes[physical] += 1;
        }
    }
    assert!(writes.iter().all(|count| *count == 1));
    assert_eq!(PARTIAL_BYTES, 196_608);
}

#[test]
fn k_partitions_cover_all_indices_once_per_fragment_column() {
    for column_lane in 0..16 {
        let mut reads = vec![0_u8; 4096];
        for partition in 0..4 {
            for step in 0..64 {
                for lane_group in 0..4 {
                    let lane = lane_group * 16 + column_lane;
                    for component in 0..4 {
                        let reduction = partition * 1024 + step * 16 + (lane / 16) * 4 + component;
                        reads[reduction] += 1;
                    }
                }
            }
        }
        assert!(reads.iter().all(|count| *count == 1));
    }
}

#[test]
fn last_partition_and_column_tile_reach_exact_extents() {
    assert_eq!(3 * 1024 + 63 * 16 + 3 * 4 + 3, 4095);
    assert_eq!(767 * 16 + 15, 12287);
    for column in 0_u32..768 {
        assert_eq!(usize::from(u16::try_from(column).unwrap()), column as usize);
    }
    assert!(u8::try_from(256).is_err());
    assert!(u8::try_from(767).is_err());
}

#[test]
fn active_mfma_row_matches_compact_store() {
    for lane in 0..64 {
        for component in 0..4 {
            assert_eq!(lane / 16 * 4 + component == 0, lane < 16 && component == 0);
        }
    }
}

#[test]
fn merge_reads_initialized_partials_without_touching_output_tail() {
    let mut reads = vec![0_u8; PARTIAL_ELEMENTS];
    let mut output = vec![0_u8; 32 * 12288];
    for (column, writes) in output.iter_mut().enumerate().take(192 * 64) {
        for partition in 0..4 {
            reads[partition * 12288 + column] += 1;
        }
        *writes += 1;
    }
    assert!(reads.iter().all(|count| *count == 1));
    assert!(output[..12288].iter().all(|count| *count == 1));
    assert!(output[12288..].iter().all(|count| *count == 0));
}

fn merge_model(partials: [f32; 4]) -> Option<u16> {
    let mut sum = 0.0_f32;
    let mut finite = true;
    for value in partials {
        sum += value;
        finite &= value.is_finite() & sum.is_finite();
    }
    let value = Bf16::from_f32(sum);
    (finite && value.is_finite()).then_some(value.to_bits())
}

#[test]
fn merge_is_ordered_fp32_not_exact_real_arithmetic() {
    let partials = [16_777_216.0, 1.0, -16_777_216.0, 0.0];
    assert_eq!(merge_model(partials), Some(0));
    assert_eq!(partials.into_iter().map(f64::from).sum::<f64>(), 1.0);
    assert_eq!(merge_model([1.0; 4]), Some(0x4080));
}

#[test]
fn bf16_rounding_happens_once_after_merging_all_partitions() {
    let values = [1.0, 1.0 / 256.0, 1.0 / 256.0, 0.0];
    assert_eq!(merge_model(values), Some(0x3f81));
    let mut sum = Bf16::from_f32(0.0);
    for value in values {
        sum = Bf16::from_f32(sum.to_f32() + value);
    }
    assert_eq!(sum.to_bits(), 0x3f80);
}

#[test]
fn bf16_ties_round_to_even_for_both_signs() {
    for (value, expected) in [
        (0.0, 0x0000),
        (-0.0, 0x0000),
        (1.0 + 1.0 / 256.0, 0x3f80),
        (1.0 + 3.0 / 256.0, 0x3f82),
        (-1.0 - 1.0 / 256.0, 0xbf80),
        (-1.0 - 3.0 / 256.0, 0xbf82),
    ] {
        assert_eq!(merge_model([value, 0.0, 0.0, 0.0]), Some(expected));
    }
}

#[test]
fn merge_rejects_nonfinite_inputs_and_intermediate_fp32_overflow() {
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        for position in 0..4 {
            let mut partials = [0.0; 4];
            partials[position] = value;
            assert!(merge_model(partials).is_none());
        }
    }
    assert!(merge_model([f32::MAX, f32::MAX, -f32::MAX, 0.0]).is_none());
}

#[test]
fn merge_rejects_finite_fp32_that_overflows_bf16() {
    assert!(f32::MAX.is_finite());
    assert!(merge_model([f32::MAX, 0.0, 0.0, 0.0]).is_none());
    assert_eq!(
        merge_model([f32::from_bits(0x7f7f0000), 0.0, 0.0, 0.0]),
        Some(0x7f7f)
    );
}

#[test]
fn dyadic_partition_reference_is_exact_before_final_bf16_rounding() {
    for column in [0_usize, 15, 16, 4095, 4096, 8191, 8192, 12287] {
        let mut partials = [0.0_f32; 4];
        let mut exact = 0_i64;
        for k in 0..4096 {
            let a = i64::try_from(k % 17).unwrap() - 8;
            let b = i64::try_from((3 * k + 5 * column) % 19).unwrap() - 9;
            partials[k / 1024] += (a * b) as f32 / 256.0;
            exact += a * b;
        }
        let merged: f32 = partials.into_iter().sum();
        assert_eq!(f64::from(merged), exact as f64 / 256.0);
        assert_eq!(
            merge_model(partials),
            Some(Bf16::from_f32(exact as f32 / 256.0).to_bits())
        );
    }
}
